use runtime::Session as RuntimeSession;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::state::{SessionSummary, TaskResultType, TaskStatus, TaskStatusResponse, TurnTask};

pub(crate) struct SessionRow {
    pub created_at: u64,
    pub conversation: RuntimeSession,
}

pub(crate) async fn insert_session(pool: &PgPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO sessions (id) VALUES ($1)")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn update_session_conversation(
    pool: &PgPool,
    session_id: &str,
    conversation: &RuntimeSession,
) -> Result<(), sqlx::Error> {
    let payload = serde_json::to_value(conversation)
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    sqlx::query("UPDATE sessions SET conversation = $2, updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(payload)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn list_sessions(pool: &PgPool) -> Result<Vec<SessionSummary>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, i64, i32)>(
        "SELECT id, (EXTRACT(EPOCH FROM created_at) * 1000)::BIGINT AS created_ms, \
                COALESCE(jsonb_array_length(conversation->'messages'), 0) AS message_count \
         FROM sessions ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, created_ms, message_count)| SessionSummary {
            id,
            created_at: created_ms.max(0) as u64,
            message_count: message_count.max(0) as usize,
        })
        .collect())
}

pub(crate) async fn load_session(
    pool: &PgPool,
    session_id: &str,
) -> Result<Option<SessionRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, (i64, JsonValue)>(
        "SELECT (EXTRACT(EPOCH FROM created_at) * 1000)::BIGINT AS created_ms, conversation \
         FROM sessions WHERE id = $1",
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(created_ms, conversation)| {
        let conversation = serde_json::from_value::<RuntimeSession>(conversation)
            .unwrap_or_else(|error| {
                tracing::warn!(
                    session_id = %session_id,
                    error = %error,
                    "stored conversation did not deserialize; starting from empty history"
                );
                RuntimeSession::new()
            });
        SessionRow { created_at: created_ms.max(0) as u64, conversation }
    }))
}

// NOTE: user_id is deliberately NOT bound here. tasks.user_id has a foreign
// key to users(id) which has not been populated yet (Phase 4.1 not shipped).
// Binding an unrecognized user_id would fail every authenticated insert with
// a foreign-key violation. Leave it NULL until a later task wires real users.
pub(crate) async fn insert_task(pool: &PgPool, task: &TurnTask) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tasks (id, session_id, status, message_type, context, payload) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&task.id)
    .bind(&task.session_id)
    .bind(task.status.as_str())
    .bind(task.message_type.as_str())
    .bind(serde_json::to_value(&task.context).map_err(|e| sqlx::Error::Protocol(e.to_string()))?)
    .bind(&task.payload)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn delete_task(pool: &PgPool, task_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn mark_task_running(pool: &PgPool, task_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tasks SET status = 'running', payload = '{\"phase\":\"running\"}'::jsonb, \
                error = NULL, updated_at = NOW() WHERE id = $1",
    )
    .bind(task_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn complete_task(
    pool: &PgPool,
    task_id: &str,
    result_type: TaskResultType,
    payload: &JsonValue,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tasks SET status = 'completed', result_type = $2, payload = $3, error = NULL, \
                updated_at = NOW() WHERE id = $1",
    )
    .bind(task_id)
    .bind(result_type.as_str())
    .bind(payload)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn fail_task(pool: &PgPool, task_id: &str, error: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tasks SET status = 'failed', result_type = 'error', \
                payload = jsonb_build_object('error', $2::text), error = $2, \
                updated_at = NOW() WHERE id = $1",
    )
    .bind(task_id)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn load_task(
    pool: &PgPool,
    task_id: &str,
) -> Result<Option<TaskStatusResponse>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, Option<String>, JsonValue)>(
        "SELECT id, status, result_type, payload FROM tasks WHERE id = $1",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id, status, result_type, payload)| TaskStatusResponse {
        task_id: id,
        status: parse_task_status(&status),
        result_type: result_type.as_deref().map(parse_task_result_type),
        payload,
    }))
}

fn parse_task_status(status: &str) -> TaskStatus {
    match status {
        "running" => TaskStatus::Running,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        _ => TaskStatus::Queued,
    }
}

fn parse_task_result_type(result_type: &str) -> TaskResultType {
    match result_type {
        "clarification" => TaskResultType::Clarification,
        "explanation" => TaskResultType::Explanation,
        "error" => TaskResultType::Error,
        _ => TaskResultType::Generation,
    }
}

/// Mark every task left in `queued`/`running` by a dead process as failed.
/// Idempotent; runs once at boot before the turn worker starts. Multi-node
/// safety (not marking a *live* peer's work) is out of scope for this task.
pub async fn reconcile_interrupted_tasks(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE tasks SET status = 'failed', result_type = 'error', \
                error = 'interrupted by server restart', \
                payload = jsonb_build_object('error', 'interrupted by server restart'), \
                updated_at = NOW() \
         WHERE status IN ('queued', 'running')",
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
