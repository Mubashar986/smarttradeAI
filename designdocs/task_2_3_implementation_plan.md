# Task 2.3 — Persistent Session & Task Management: Implementation Plan

> Stage: 3 — Implementation Planning
> Inputs: [task_2_3_understanding.md](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/designdocs/task_2_3_understanding.md), [task_2_3_design.md](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/designdocs/task_2_3_design.md)
> RequestFeedback = true

## 1. Change Summary Card

| Property | Value |
|---|---|
| Files Modified | 5 (`state.rs`, `sessions.rs`, `turns.rs`, `lib.rs`, `main.rs`) |
| Files Created | 1 (`persistence.rs`) |
| Files Deleted | 0 |
| Lines Added | ~330 |
| Lines Removed | ~20 |
| New Dependencies | **None** (timestamps via `EXTRACT(EPOCH …)` SQL, no `chrono` feature needed) |
| Schema Changes | **None** (Task 2.1/2.2 schema already sufficient; no new migration) |
| Estimated Complexity | Medium (one new module + surgical edits at 12 call sites) |
| Estimated Time | ~45 minutes |
| Risk Level | 🟡 Medium (mitigations for every 🔴 in design Section 5) |

## 2. Dependency Check

#### New Crate Dependencies
- **None required.** Verified against [rust/Cargo.toml:21](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/Cargo.toml:21): workspace `sqlx` features `["runtime-tokio", "postgres", "json"]` cover `PgPool`, `JSONB` binding via `serde_json::Value`, and `BIGINT` decoding. TIMESTAMPTZ is converted to epoch millis in SQL (`(EXTRACT(EPOCH FROM created_at) * 1000)::BIGINT`), avoiding the `chrono` feature.

## 3. Execution Order

1. **`persistence.rs`** (NEW) — owns all SQL; everything else imports from it, so it must exist first.
2. **`state.rs`** — adds `as_str` helpers, `Session::rehydrated`, `AppState::live_session`, and DB write-through in task mutators. Downstream route files call these, so `state.rs` must compile before routes.
3. **`lib.rs`** — `mod persistence;` + re-export (small; placed here so steps 1-2 resolve module paths).
4. **`sessions.rs`** — accept-path persist + DB-backed reads + lazy rehydrate (depends on 1+2).
5. **`turns.rs`** — enqueue/get_task/worker persistence (depends on 1+2).
6. **`main.rs`** — boot reconcile call (depends on `lib.rs` re-export; must be last so the binary compiles).

**Why this order:** the dependency graph is `main.rs → lib.rs → {routes → state.rs → persistence.rs}`. Building bottom-up (persistence → state → routes → binary) means every file compiles against symbols that already exist; a top-down order would leave unresolvable imports mid-implementation.

## 4. Step-by-Step Code Changes

### Step 1: [NEW] [persistence.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/persistence.rs)

**What:** All session/task SQL and row mapping in one module.
**Why:** Quarantines query text (single-responsibility); routes/state never contain SQL.

```diff
+use runtime::Session as RuntimeSession;
+use serde_json::Value as JsonValue;
+use sqlx::PgPool;
+
+use crate::state::{
+    SessionSummary, TaskResultType, TaskStatus, TaskStatusResponse, TurnTask,
+};
+
+/// Row shape returned by `load_session`: creation time plus the durable conversation.
+pub(crate) struct SessionRow {
+    pub created_at: u64,
+    pub conversation: RuntimeSession,
+}
+
+pub(crate) async fn insert_session(pool: &PgPool, session_id: &str) -> Result<(), sqlx::Error> {
+    sqlx::query("INSERT INTO sessions (id) VALUES ($1)")
+        .bind(session_id)
+        .execute(pool)
+        .await?;
+    Ok(())
+}
+
+pub(crate) async fn update_session_conversation(
+    pool: &PgPool,
+    session_id: &str,
+    conversation: &RuntimeSession,
+) -> Result<(), sqlx::Error> {
+    let payload = serde_json::to_value(conversation)
+        .map_err(|error| sqlx::Error::Encode(Box::new(error)))?;
+    sqlx::query("UPDATE sessions SET conversation = $2, updated_at = NOW() WHERE id = $1")
+        .bind(session_id)
+        .bind(payload)
+        .execute(pool)
+        .await?;
+    Ok(())
+}
+
+pub(crate) async fn list_sessions(pool: &PgPool) -> Result<Vec<SessionSummary>, sqlx::Error> {
+    let rows = sqlx::query_as::<_, (String, i64, i32)>(
+        "SELECT id, (EXTRACT(EPOCH FROM created_at) * 1000)::BIGINT AS created_ms, \
+                COALESCE(jsonb_array_length(conversation->'messages'), 0) AS message_count \
+         FROM sessions ORDER BY id",
+    )
+    .fetch_all(pool)
+    .await?;
+    Ok(rows
+        .into_iter()
+        .map(|(id, created_ms, message_count)| SessionSummary {
+            id,
+            created_at: created_ms.max(0) as u64,
+            message_count: message_count.max(0) as usize,
+        })
+        .collect())
+}
+
+pub(crate) async fn load_session(
+    pool: &PgPool,
+    session_id: &str,
+) -> Result<Option<SessionRow>, sqlx::Error> {
+    let row = sqlx::query_as::<_, (i64, JsonValue)>(
+        "SELECT (EXTRACT(EPOCH FROM created_at) * 1000)::BIGINT AS created_ms, conversation \
+         FROM sessions WHERE id = $1",
+    )
+    .bind(session_id)
+    .fetch_optional(pool)
+    .await?;
+    Ok(row.map(|(created_ms, conversation)| {
+        let conversation = serde_json::from_value::<RuntimeSession>(conversation)
+            .unwrap_or_else(|error| {
+                tracing::warn!(
+                    session_id = %session_id,
+                    error = %error,
+                    "stored conversation did not deserialize; starting from empty history"
+                );
+                RuntimeSession::new()
+            });
+        SessionRow {
+            created_at: created_ms.max(0) as u64,
+            conversation,
+        }
+    }))
+}
+
+pub(crate) async fn insert_task(pool: &PgPool, task: &TurnTask) -> Result<(), sqlx::Error> {
+    sqlx::query(
+        "INSERT INTO tasks (id, session_id, user_id, status, message_type, context, payload) \
+         VALUES ($1, $2, $3, $4, $5, $6, $7)",
+    )
+    .bind(&task.id)
+    .bind(&task.session_id)
+    .bind(task.context.user_id.as_deref())
+    .bind(task.status.as_str())
+    .bind(task.message_type.as_str())
+    .bind(serde_json::to_value(&task.context).map_err(|e| sqlx::Error::Encode(Box::new(e)))?)
+    .bind(&task.payload)
+    .execute(pool)
+    .await?;
+    Ok(())
+}
+
+pub(crate) async fn delete_task(pool: &PgPool, task_id: &str) -> Result<(), sqlx::Error> {
+    sqlx::query("DELETE FROM tasks WHERE id = $1")
+        .bind(task_id)
+        .execute(pool)
+        .await?;
+    Ok(())
+}
+
+pub(crate) async fn mark_task_running(pool: &PgPool, task_id: &str) -> Result<(), sqlx::Error> {
+    sqlx::query(
+        "UPDATE tasks SET status = 'running', payload = '{\"phase\":\"running\"}'::jsonb, \
+                error = NULL, updated_at = NOW() WHERE id = $1",
+    )
+    .bind(task_id)
+    .execute(pool)
+    .await?;
+    Ok(())
+}
+
+pub(crate) async fn complete_task(
+    pool: &PgPool,
+    task_id: &str,
+    result_type: TaskResultType,
+    payload: &JsonValue,
+) -> Result<(), sqlx::Error> {
+    sqlx::query(
+        "UPDATE tasks SET status = 'completed', result_type = $2, payload = $3, error = NULL, \
+                updated_at = NOW() WHERE id = $1",
+    )
+    .bind(task_id)
+    .bind(result_type.as_str())
+    .bind(payload)
+    .execute(pool)
+    .await?;
+    Ok(())
+}
+
+pub(crate) async fn fail_task(pool: &PgPool, task_id: &str, error: &str) -> Result<(), sqlx::Error> {
+    sqlx::query(
+        "UPDATE tasks SET status = 'failed', result_type = 'error', \
+                payload = jsonb_build_object('error', $2::text), error = $2, \
+                updated_at = NOW() WHERE id = $1",
+    )
+    .bind(task_id)
+    .bind(error)
+    .execute(pool)
+    .await?;
+    Ok(())
+}
+
+pub(crate) async fn load_task(
+    pool: &PgPool,
+    task_id: &str,
+) -> Result<Option<TaskStatusResponse>, sqlx::Error> {
+    let row = sqlx::query_as::<_, (String, String, Option<String>, JsonValue)>(
+        "SELECT id, status, result_type, payload FROM tasks WHERE id = $1",
+    )
+    .bind(task_id)
+    .fetch_optional(pool)
+    .await?;
+    Ok(row.map(|(id, status, result_type, payload)| TaskStatusResponse {
+        task_id: id,
+        status: parse_task_status(&status),
+        result_type: result_type.as_deref().map(parse_task_result_type),
+        payload,
+    }))
+}
+
+fn parse_task_status(status: &str) -> TaskStatus {
+    match status {
+        "running" => TaskStatus::Running,
+        "completed" => TaskStatus::Completed,
+        "failed" => TaskStatus::Failed,
+        _ => TaskStatus::Queued,
+    }
+}
+
+fn parse_task_result_type(result_type: &str) -> TaskResultType {
+    match result_type {
+        "clarification" => TaskResultType::Clarification,
+        "explanation" => TaskResultType::Explanation,
+        "error" => TaskResultType::Error,
+        _ => TaskResultType::Generation,
+    }
+}
+
+/// Mark every task left in `queued`/`running` by a dead process as failed.
+/// Idempotent; runs once at boot before the turn worker starts. Multi-node
+/// safety (not marking a *live* peer's work) is Task 3.1 advisory-lock scope.
+pub async fn reconcile_interrupted_tasks(pool: &PgPool) -> Result<u64, sqlx::Error> {
+    let result = sqlx::query(
+        "UPDATE tasks SET status = 'failed', result_type = 'error', \
+                error = 'interrupted by server restart', \
+                payload = jsonb_build_object('error', 'interrupted by server restart'), \
+                updated_at = NOW() \
+         WHERE status IN ('queued', 'running')",
+    )
+    .execute(pool)
+    .await?;
+    Ok(result.rows_affected())
+}
```

---

### Step 2: [MODIFY] [state.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs)

**2a. What:** `as_str` for `TaskStatus` / `TaskResultType` (needed for SQL binds).
**Where:** after the `TaskResultType` enum (line ~79), mirroring `TurnMessageType::as_str`.

```diff
 pub enum TaskResultType {
     Clarification,
     Generation,
     Explanation,
     Error,
 }
+
+impl TaskStatus {
+    pub(crate) const fn as_str(self) -> &'static str {
+        match self {
+            Self::Queued => "queued",
+            Self::Running => "running",
+            Self::Completed => "completed",
+            Self::Failed => "failed",
+        }
+    }
+}
+
+impl TaskResultType {
+    pub(crate) const fn as_str(self) -> &'static str {
+        match self {
+            Self::Clarification => "clarification",
+            Self::Generation => "generation",
+            Self::Explanation => "explanation",
+            Self::Error => "error",
+        }
+    }
+}
```

**2b. What:** Write-through DB updates in the three task mutators (worker-path policy: log, don't fail).
**Where:** lines 203-224.

```diff
     pub async fn mark_task_running(&self, task_id: &str) {
         if let Some(task) = self.tasks.write().await.get_mut(task_id) {
             task.mark_running();
         }
+        if let Some(pool) = &self.pool {
+            if let Err(error) = crate::persistence::mark_task_running(pool, task_id).await {
+                tracing::error!(task_id = %task_id, error = %error, "failed to persist task running state");
+            }
+        }
     }
 
     pub async fn complete_task(
         &self,
         task_id: &str,
         result_type: TaskResultType,
         payload: JsonValue,
     ) {
         if let Some(task) = self.tasks.write().await.get_mut(task_id) {
-            task.complete(result_type, payload);
+            task.complete(result_type, payload.clone());
+        }
+        if let Some(pool) = &self.pool {
+            if let Err(error) = crate::persistence::complete_task(pool, task_id, result_type, &payload).await {
+                tracing::error!(task_id = %task_id, error = %error, "failed to persist task completion");
+            }
         }
     }
 
     pub async fn fail_task(&self, task_id: &str, error: String) {
         if let Some(task) = self.tasks.write().await.get_mut(task_id) {
-            task.fail(error);
+            task.fail(error.clone());
+        }
+        if let Some(pool) = &self.pool {
+            if let Err(db_error) = crate::persistence::fail_task(pool, task_id, &error).await {
+                tracing::error!(task_id = %task_id, error = %db_error, "failed to persist task failure");
+            }
         }
     }
```

**2c. What:** Lazy rehydration funnel + rehydration constructor.
**Where:** new method after `turn_lock_for` (~line 201); new constructor after `Session::new` (~line 261).

```diff
+    /// Return the live in-memory session, rehydrating it from PostgreSQL when
+    /// this process has not seen it yet (e.g. after a restart). Double-checked
+    /// insertion mirrors `turn_lock_for`; `None` means the ID exists nowhere.
+    pub async fn live_session(&self, session_id: &str) -> Option<Session> {
+        if let Some(session) = self.sessions.read().await.get(session_id) {
+            return Some(session.clone());
+        }
+        let pool = self.pool.as_ref()?;
+        let row = match crate::persistence::load_session(pool, session_id).await {
+            Ok(Some(row)) => row,
+            Ok(None) => return None,
+            Err(error) => {
+                tracing::error!(session_id = %session_id, error = %error, "failed to load session from database");
+                return None;
+            }
+        };
+        let mut sessions = self.sessions.write().await;
+        let session = sessions
+            .entry(session_id.to_string())
+            .or_insert_with(|| {
+                Session::rehydrated(session_id.to_string(), row.created_at, row.conversation)
+            })
+            .clone();
+        Some(session)
+    }
+
     pub async fn turn_lock_for(&self, session_id: &str) -> Arc<Mutex<()>> {
```

```diff
     pub(crate) fn new(id: SessionId) -> Self {
         let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
         Self {
             id,
             created_at: unix_timestamp_millis(),
             conversation: RuntimeSession::new(),
             events,
         }
     }
+
+    /// Rebuild a live handle from a persisted row: restored identity and
+    /// conversation, plus a fresh broadcast channel (live layer is never
+    /// serialized — the compiler enforces this via missing `Serialize`).
+    pub(crate) fn rehydrated(
+        id: SessionId,
+        created_at: u64,
+        conversation: RuntimeSession,
+    ) -> Self {
+        let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
+        Self {
+            id,
+            created_at,
+            conversation,
+            events,
+        }
+    }
```

---

### Step 3: [MODIFY] [lib.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/lib.rs)

**What:** Register the module and re-export the boot-time reconcile function.
**Where:** lines 1-9.

```diff
 mod llm_bridge;
 mod middleware;
 mod mql5_extractor;
+mod persistence;
 mod routes;
 mod state;
 
 // Re-export the public API consumed by the c2-engine binary crate.
+pub use persistence::reconcile_interrupted_tasks;
 pub use routes::turns::run_turn_worker;
 pub use state::AppState;
```

---

### Step 4: [MODIFY] [routes/sessions.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/sessions.rs)

**4a. What:** `create_session` persists first (accept-path policy: DB error → 500, memory untouched).
**Where:** lines 19-36.

```diff
 pub(crate) async fn create_session(
     State(state): State<AppState>,
-) -> (StatusCode, Json<CreateSessionResponse>) {
+) -> ApiResult<(StatusCode, Json<CreateSessionResponse>)> {
     let session_id = state.allocate_session_id();
+    if let Some(pool) = &state.pool {
+        crate::persistence::insert_session(pool, &session_id)
+            .await
+            .map_err(|error| {
+                tracing::error!(session_id = %session_id, error = %error, "failed to persist new session");
+                crate::state::internal_error("failed to persist session".to_string())
+            })?;
+    }
     let session = Session::new(session_id.clone());
 
     state
         .sessions
         .write()
         .await
         .insert(session_id.clone(), session);
     let _ = state.turn_lock_for(&session_id).await;
 
-    (
+    Ok((
         StatusCode::CREATED,
         Json(CreateSessionResponse { session_id }),
-    )
+    ))
 }
```

**4b. What:** `list_sessions` reads the ledger when a pool exists (sees pre-restart sessions).
**Where:** lines 38-53.

```diff
-pub(crate) async fn list_sessions(State(state): State<AppState>) -> Json<ListSessionsResponse> {
+pub(crate) async fn list_sessions(
+    State(state): State<AppState>,
+) -> ApiResult<Json<ListSessionsResponse>> {
+    if let Some(pool) = &state.pool {
+        let summaries = crate::persistence::list_sessions(pool).await.map_err(|error| {
+            tracing::error!(error = %error, "failed to list sessions from database");
+            crate::state::internal_error("failed to list sessions".to_string())
+        })?;
+        return Ok(Json(ListSessionsResponse { sessions: summaries }));
+    }
     let sessions = state.sessions.read().await;
     let mut summaries = sessions
         .values()
         .map(|session| SessionSummary {
             id: session.id.clone(),
             created_at: session.created_at,
             message_count: session.conversation.messages.len(),
         })
         .collect::<Vec<_>>();
     summaries.sort_by(|left, right| left.id.cmp(&right.id));
 
-    Json(ListSessionsResponse {
+    Ok(Json(ListSessionsResponse {
         sessions: summaries,
-    })
+    }))
 }
```

**4c. What:** `get_session`, `stream_session_events`, `stream_session_websocket` go through `live_session` (rehydrate-on-miss) instead of raw map lookups.
**Where:** lines 55-69, 75-87, 115-127. Same pattern three times; shown once:

```diff
 pub(crate) async fn get_session(
     State(state): State<AppState>,
     Path(id): Path<SessionId>,
 ) -> ApiResult<Json<SessionDetailsResponse>> {
-    let sessions = state.sessions.read().await;
-    let session = sessions
-        .get(&id)
+    let session = state
+        .live_session(&id)
+        .await
         .ok_or_else(|| not_found(format!("session `{id}` not found")))?;
 
     Ok(Json(SessionDetailsResponse {
         id: session.id.clone(),
         created_at: session.created_at,
         session: session.conversation.clone(),
     }))
 }
```
(SSE and WS handlers: identical replacement of their `sessions.read().await … get(&id)` blocks with `state.live_session(&id).await`, keeping the snapshot + `subscribe()` construction unchanged.)

---

### Step 5: [MODIFY] [routes/turns.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs)

**5a. What:** `enqueue_turn` — rehydrate-aware session access, persist message + task, full rollback on DB error, DB cleanup on channel failure.
**Where:** lines 65-131.

```diff
 async fn enqueue_turn(
     state: &AppState,
     session_id: &str,
     payload: SubmitTurnRequest,
 ) -> ApiResult<TaskId> {
     let task_id = state.allocate_task_id();
     let mut context = payload.context;
     context.task_id = Some(task_id.clone());
     let message = ConversationMessage::user_text(payload.text.clone());
     let broadcaster = {
+        let session = state
+            .live_session(session_id)
+            .await
+            .ok_or_else(|| not_found(format!("session `{session_id}` not found")))?;
         let mut sessions = state.sessions.write().await;
         let session = sessions
             .get_mut(session_id)
             .ok_or_else(|| not_found(format!("session `{session_id}` not found")))?;
         session.conversation.messages.push(message.clone());
         session.events.clone()
     };
 
+    let task = TurnTask::queued(
+        task_id.clone(),
+        session_id.to_string(),
+        payload.message_type,
+        context.clone(),
+    );
+    if let Some(pool) = &state.pool {
+        let persisted = async {
+            let conversation = {
+                let sessions = state.sessions.read().await;
+                sessions.get(session_id).map(|s| s.conversation.clone())
+            };
+            if let Some(conversation) = conversation {
+                crate::persistence::update_session_conversation(pool, session_id, &conversation)
+                    .await?;
+            }
+            crate::persistence::insert_task(pool, &task).await
+        };
+        if let Err(error) = persisted.await {
+            tracing::error!(task_id = %task_id, error = %error, "failed to persist enqueued turn");
+            // Accept-path policy: never accept unrecorded work — roll back memory.
+            state.tasks.write().await.remove(&task_id);
+            if let Some(session) = state.sessions.write().await.get_mut(session_id) {
+                session.conversation.messages.pop();
+            }
+            return Err(crate::state::internal_error(
+                "failed to persist turn".to_string(),
+            ));
+        }
+    }
+
     state.tasks.write().await.insert(
         task_id.clone(),
-        TurnTask::queued(
-            task_id.clone(),
-            session_id.to_string(),
-            payload.message_type,
-            context.clone(),
-        ),
+        task,
     );
```
And the channel-failure cleanup gains the DB delete:

```diff
         .is_err()
     {
         state.tasks.write().await.remove(&task_id);
+        if let Some(pool) = &state.pool {
+            if let Err(error) = crate::persistence::delete_task(pool, &task_id).await {
+                tracing::error!(task_id = %task_id, error = %error, "failed to delete unpersisted task row");
+            }
+        }
         return Err((
             StatusCode::SERVICE_UNAVAILABLE,
```

**5b. What:** `get_task` — memory-first, DB fallback so status survives restart.
**Where:** lines 133-147.

```diff
 pub(crate) async fn get_task(
     State(state): State<AppState>,
     Path(task_id): Path<TaskId>,
 ) -> ApiResult<Json<TaskStatusResponse>> {
-    let tasks = state.tasks.read().await;
-    let task = tasks
-        .get(&task_id)
-        .ok_or_else(|| not_found(format!("task `{task_id}` not found")))?;
-    Ok(Json(TaskStatusResponse {
-        task_id: task.id.clone(),
-        status: task.status,
-        result_type: task.result_type,
-        payload: task.payload.clone(),
-    }))
+    {
+        let tasks = state.tasks.read().await;
+        if let Some(task) = tasks.get(&task_id) {
+            return Ok(Json(TaskStatusResponse {
+                task_id: task.id.clone(),
+                status: task.status,
+                result_type: task.result_type,
+                payload: task.payload.clone(),
+            }));
+        }
+    }
+    if let Some(pool) = &state.pool {
+        if let Some(task) = crate::persistence::load_task(pool, &task_id)
+            .await
+            .map_err(|error| {
+                tracing::error!(task_id = %task_id, error = %error, "failed to load task from database");
+                crate::state::internal_error("failed to load task".to_string())
+            })?
+        {
+            return Ok(Json(task));
+        }
+    }
+    Err(not_found(format!("task `{task_id}` not found")))
 }
```

**5c. What:** Worker conversation write-back also persists (worker-path policy: log, don't fail).
**Where:** lines 550-557.

```diff
     // 9. Write the updated session back to AppState.
     let updated_session = runtime.into_session();
     {
         let mut sessions = state.sessions.write().await;
         if let Some(session) = sessions.get_mut(&request.session_id) {
-            session.conversation = updated_session;
+            session.conversation = updated_session.clone();
         }
     }
+    if let Some(pool) = &state.pool {
+        if let Err(error) = crate::persistence::update_session_conversation(
+            pool,
+            &request.session_id,
+            &updated_session,
+        )
+        .await
+        {
+            tracing::error!(session_id = %request.session_id, error = %error, "failed to persist conversation");
+        }
+    }
```

**5d. What:** Worker snapshot uses `live_session` so turns on rehydrated sessions work.
**Where:** lines 261-268.

```diff
     // 1. Snapshot the current session conversation.
     let session_snapshot = {
-        let sessions = state.sessions.read().await;
-        let session = sessions
-            .get(&request.session_id)
+        let session = state
+            .live_session(&request.session_id)
+            .await
             .ok_or_else(|| format!("session `{}` not found", request.session_id))?;
         session.conversation.clone()
     };
```

---

### Step 6: [MODIFY] [main.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/c2-engine/src/main.rs)

**What:** Reconcile orphaned tasks after migrations, before the worker starts.
**Where:** lines 54-60.

```diff
     let (state, turn_rx) = AppState::new(pool);
 
+    if let Some(pool) = &state.pool {
+        match server::reconcile_interrupted_tasks(pool).await {
+            Ok(count) if count > 0 => {
+                tracing::warn!(count, "marked interrupted tasks as failed after restart")
+            }
+            Ok(_) => tracing::info!("no interrupted tasks to reconcile"),
+            Err(error) => tracing::error!(error = %error, "failed to reconcile interrupted tasks"),
+        }
+    }
+
     tracing::info!(
         model = %state.llm_model,
         "starting turn worker"
     );
```
**Why here:** reconcile must run after migrations (table exists) and before `run_turn_worker` accepts new work; it is idempotent, so every boot is safe.

## 5. Verification Commands

#### Compilation & Lint
```powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine\rust"
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

#### Unit & Integration Tests (no DB required — pool = None path)
```powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine\rust"
cargo test --workspace
```

#### Smoke Test (with PostgreSQL)
```powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine"
docker compose up -d --build postgres redis c2-engine
$session = Invoke-RestMethod http://localhost:3000/v1/sessions -Method Post
docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT id FROM sessions WHERE id = '$($session.session_id)';"
$turn = Invoke-RestMethod "http://localhost:3000/v1/sessions/$($session.session_id)/turn" -Method Post -ContentType application/json -Body '{"text":"Explain RSI"}'
docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT id, status FROM tasks WHERE id = '$($turn.task_id)';"
# Restart survival check:
docker compose restart c2-engine; Start-Sleep 5
Invoke-RestMethod "http://localhost:3000/v1/sessions/$($session.session_id)" -Method Get
Invoke-RestMethod "http://localhost:3000/v1/tasks/$($turn.task_id)" -Method Get
```

## 6. Rollback Instructions

```powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI"
git diff --stat HEAD          # review what changed
git checkout -- .             # revert uncommitted changes
# OR, if already committed:
git revert HEAD
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine"
docker compose up c2-engine --build -d
Invoke-RestMethod http://localhost:3000/health
```
No database rollback needed — this task adds no migration; rows written by the new code are inert to the old code. (Optional dev-only cleanup: `TRUNCATE tasks; TRUNCATE sessions CASCADE;`)

---

## ⛔ STOP! USER REVIEW REQUIRED

This plan modifies **5 files** and creates **1 new file** (~330 lines added, ~20 removed, zero new dependencies, zero schema changes).

Please review the diff previews above carefully. If everything looks correct:
- Reply with **"Approve"** or **"Proceed"** to begin implementation.
- Reply with feedback if you want changes to the plan.

**No code will be written until you approve.**

---

## Workflow Checklist

- [x] Summary card filled in (files, lines, complexity, risk)
- [x] New crate dependencies checked — none required (Section 2)
- [x] Execution order specified with dependency reasoning (Section 3)
- [x] Every change shown in diff format with context lines (Section 4)
- [x] "Why" explanation for non-obvious changes (persist-first, rollback, reconcile ordering)
- [x] Verification commands are copy-pasteable PowerShell (Section 5)
- [x] Rollback instructions provided (Section 6)
- [x] STOP gate present at the end
- [x] RequestFeedback = true set in artifact metadata
