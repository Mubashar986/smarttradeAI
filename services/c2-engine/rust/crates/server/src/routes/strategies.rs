use std::fs;
use std::path::{Path as FsPath, PathBuf};

use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::Json;
use runtime::SmartTradeToolConfig;
use serde_json::Value as JsonValue;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::Row;

use crate::middleware::auth::AuthClaims;
use crate::state::{
    ApiError, ApiResult, DeleteStrategyResponse, ErrorResponse, ListStrategiesResponse,
    StrategyDetailsResponse, StrategyRecord, UpdateStrategyRequest, internal_error, not_found,
    unix_timestamp_millis,
};

pub(crate) async fn list_strategies(
    claims: Option<Extension<AuthClaims>>,
) -> ApiResult<Json<ListStrategiesResponse>> {
    let user_id = resolved_user_id(claims);
    let strategies = load_strategies_for_user(&user_id)
        .await
        .map_err(internal_error)?;
    Ok(Json(ListStrategiesResponse {
        strategies: strategies.into_iter().map(StrategyRecord::summary).collect(),
    }))
}

pub(crate) async fn get_strategy(
    claims: Option<Extension<AuthClaims>>,
    Path(id): Path<String>,
) -> ApiResult<Json<StrategyDetailsResponse>> {
    let user_id = resolved_user_id(claims);
    let strategy = load_strategy_record(&user_id, &id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("strategy `{id}` not found")))?;
    Ok(Json(StrategyDetailsResponse { strategy }))
}

pub(crate) async fn patch_strategy(
    claims: Option<Extension<AuthClaims>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateStrategyRequest>,
) -> ApiResult<Json<StrategyDetailsResponse>> {
    if payload.name.is_none()
        && payload.code.is_none()
        && payload.explanation.is_none()
        && payload.status.is_none()
        && payload.pair.is_none()
        && payload.timeframe.is_none()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "at least one strategy field must be provided".to_string(),
            }),
        ));
    }

    let user_id = resolved_user_id(claims);
    let strategy = update_strategy_record(&user_id, &id, &payload)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("strategy `{id}` not found")))?;
    Ok(Json(StrategyDetailsResponse { strategy }))
}

pub(crate) async fn delete_strategy(
    claims: Option<Extension<AuthClaims>>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteStrategyResponse>> {
    let user_id = resolved_user_id(claims);
    let deleted = soft_delete_strategy_record(&user_id, &id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("strategy `{id}` not found")))?;
    Ok(Json(deleted))
}

fn resolved_user_id(claims: Option<Extension<AuthClaims>>) -> String {
    claims
        .as_ref()
        .and_then(|claims| claims.0.principal_id())
        .unwrap_or("local-dev-user")
        .to_string()
}

async fn load_strategies_for_user(user_id: &str) -> Result<Vec<StrategyRecord>, String> {
    let storage = SmartTradeToolConfig::from_env();
    match storage.database_url {
        Some(database_url) => load_db_strategies(&database_url, user_id).await,
        None => load_local_strategies(&storage.strategies_dir, user_id),
    }
}

async fn load_strategy_record(
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<StrategyRecord>, String> {
    let storage = SmartTradeToolConfig::from_env();
    match storage.database_url {
        Some(database_url) => load_db_strategy(&database_url, user_id, strategy_id).await,
        None => load_local_strategy(&storage.strategies_dir, user_id, strategy_id),
    }
}

async fn update_strategy_record(
    user_id: &str,
    strategy_id: &str,
    update: &UpdateStrategyRequest,
) -> Result<Option<StrategyRecord>, String> {
    let storage = SmartTradeToolConfig::from_env();
    match storage.database_url {
        Some(database_url) => {
            update_db_strategy(&database_url, user_id, strategy_id, update).await
        }
        None => update_local_strategy(&storage.strategies_dir, user_id, strategy_id, update),
    }
}

async fn soft_delete_strategy_record(
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<DeleteStrategyResponse>, String> {
    let storage = SmartTradeToolConfig::from_env();
    match storage.database_url {
        Some(database_url) => soft_delete_db_strategy(&database_url, user_id, strategy_id).await,
        None => soft_delete_local_strategy(&storage.strategies_dir, user_id, strategy_id),
    }
}

async fn load_db_strategies(
    database_url: &str,
    user_id: &str,
) -> Result<Vec<StrategyRecord>, String> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|error| error.to_string())?;
    let rows = sqlx::query(
        r#"
        SELECT
            id::text AS id,
            name,
            code,
            explanation,
            status,
            session_id,
            user_id,
            pair,
            timeframe,
            created_at::text AS created_at,
            updated_at::text AS updated_at
        FROM strategies
        WHERE user_id = $1 AND status <> 'DELETED'
        ORDER BY updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .map_err(|error| error.to_string())?;

    rows.into_iter().map(strategy_record_from_row).collect()
}

async fn load_db_strategy(
    database_url: &str,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<StrategyRecord>, String> {
    let Ok(strategy_id) = strategy_id.parse::<i64>() else {
        return Ok(None);
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|error| error.to_string())?;
    let row = sqlx::query(
        r#"
        SELECT
            id::text AS id,
            name,
            code,
            explanation,
            status,
            session_id,
            user_id,
            pair,
            timeframe,
            created_at::text AS created_at,
            updated_at::text AS updated_at
        FROM strategies
        WHERE user_id = $1 AND id = $2 AND status <> 'DELETED'
        "#,
    )
    .bind(user_id)
    .bind(strategy_id)
    .fetch_optional(&pool)
    .await
    .map_err(|error| error.to_string())?;

    row.map(strategy_record_from_row).transpose()
}

async fn update_db_strategy(
    database_url: &str,
    user_id: &str,
    strategy_id: &str,
    update: &UpdateStrategyRequest,
) -> Result<Option<StrategyRecord>, String> {
    let Ok(strategy_id) = strategy_id.parse::<i64>() else {
        return Ok(None);
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|error| error.to_string())?;
    let row = sqlx::query(
        r#"
        UPDATE strategies
        SET
            name = COALESCE($3, name),
            code = COALESCE($4, code),
            explanation = COALESCE($5, explanation),
            status = COALESCE($6, status),
            pair = COALESCE($7, pair),
            timeframe = COALESCE($8, timeframe),
            updated_at = NOW()
        WHERE user_id = $1 AND id = $2 AND status <> 'DELETED'
        RETURNING
            id::text AS id,
            name,
            code,
            explanation,
            status,
            session_id,
            user_id,
            pair,
            timeframe,
            created_at::text AS created_at,
            updated_at::text AS updated_at
        "#,
    )
    .bind(user_id)
    .bind(strategy_id)
    .bind(update.name.clone())
    .bind(update.code.clone())
    .bind(update.explanation.clone())
    .bind(update.status.clone())
    .bind(update.pair.clone())
    .bind(update.timeframe.clone())
    .fetch_optional(&pool)
    .await
    .map_err(|error| error.to_string())?;

    row.map(strategy_record_from_row).transpose()
}

async fn soft_delete_db_strategy(
    database_url: &str,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<DeleteStrategyResponse>, String> {
    let Ok(strategy_id_num) = strategy_id.parse::<i64>() else {
        return Ok(None);
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .map_err(|error| error.to_string())?;
    let row = sqlx::query(
        r#"
        UPDATE strategies
        SET status = 'DELETED', updated_at = NOW()
        WHERE user_id = $1 AND id = $2 AND status <> 'DELETED'
        RETURNING id::text AS id, status
        "#,
    )
    .bind(user_id)
    .bind(strategy_id_num)
    .fetch_optional(&pool)
    .await
    .map_err(|error| error.to_string())?;

    Ok(row.map(|row| DeleteStrategyResponse {
        strategy_id: row.try_get("id").unwrap_or_default(),
        status: row
            .try_get::<String, _>("status")
            .unwrap_or_else(|_| "DELETED".to_string()),
    }))
}

fn strategy_record_from_row(row: PgRow) -> Result<StrategyRecord, String> {
    Ok(StrategyRecord {
        id: row.try_get("id").map_err(|error| error.to_string())?,
        name: row.try_get("name").map_err(|error| error.to_string())?,
        code: row.try_get("code").map_err(|error| error.to_string())?,
        explanation: row
            .try_get("explanation")
            .map_err(|error| error.to_string())?,
        status: row.try_get("status").map_err(|error| error.to_string())?,
        session_id: row
            .try_get("session_id")
            .map_err(|error| error.to_string())?,
        user_id: row.try_get("user_id").map_err(|error| error.to_string())?,
        pair: row.try_get("pair").map_err(|error| error.to_string())?,
        timeframe: row
            .try_get("timeframe")
            .map_err(|error| error.to_string())?,
        created_at: row
            .try_get("created_at")
            .map_err(|error| error.to_string())?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|error| error.to_string())?,
    })
}

fn load_local_strategies(
    strategies_dir: &FsPath,
    user_id: &str,
) -> Result<Vec<StrategyRecord>, String> {
    let mut strategies = local_strategy_paths(strategies_dir)?
        .into_iter()
        .filter_map(|path| read_local_strategy_record(&path).ok())
        .filter(|strategy| strategy.user_id == user_id && strategy.status != "DELETED")
        .collect::<Vec<_>>();
    strategies.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(strategies)
}

fn load_local_strategy(
    strategies_dir: &FsPath,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<StrategyRecord>, String> {
    Ok(load_local_strategies(strategies_dir, user_id)?
        .into_iter()
        .find(|strategy| strategy.id == strategy_id))
}

fn update_local_strategy(
    strategies_dir: &FsPath,
    user_id: &str,
    strategy_id: &str,
    update: &UpdateStrategyRequest,
) -> Result<Option<StrategyRecord>, String> {
    let Some(metadata_path) = find_local_strategy_metadata_path(strategies_dir, user_id, strategy_id)?
    else {
        return Ok(None);
    };

    let mut metadata = read_local_strategy_metadata(&metadata_path)?;
    if let Some(name) = &update.name {
        metadata.insert("strategy_name".to_string(), JsonValue::String(name.clone()));
    }
    if let Some(explanation) = &update.explanation {
        metadata.insert("explanation".to_string(), JsonValue::String(explanation.clone()));
    }
    if let Some(status) = &update.status {
        metadata.insert("status".to_string(), JsonValue::String(status.clone()));
    }
    if let Some(pair) = &update.pair {
        metadata.insert("pair".to_string(), JsonValue::String(pair.clone()));
    }
    if let Some(timeframe) = &update.timeframe {
        metadata.insert("timeframe".to_string(), JsonValue::String(timeframe.clone()));
    }
    metadata.insert(
        "updated_at".to_string(),
        JsonValue::String(current_iso8601_like_timestamp()),
    );

    if let Some(code) = &update.code {
        fs::write(metadata_path.with_extension("mq5"), code).map_err(|error| error.to_string())?;
    }

    write_local_strategy_metadata(&metadata_path, &metadata)?;
    read_local_strategy_record(&metadata_path).map(Some)
}

fn soft_delete_local_strategy(
    strategies_dir: &FsPath,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<DeleteStrategyResponse>, String> {
    let Some(metadata_path) = find_local_strategy_metadata_path(strategies_dir, user_id, strategy_id)?
    else {
        return Ok(None);
    };
    let mut metadata = read_local_strategy_metadata(&metadata_path)?;
    metadata.insert("status".to_string(), JsonValue::String("DELETED".to_string()));
    metadata.insert(
        "updated_at".to_string(),
        JsonValue::String(current_iso8601_like_timestamp()),
    );
    write_local_strategy_metadata(&metadata_path, &metadata)?;
    Ok(Some(DeleteStrategyResponse {
        strategy_id: strategy_id.to_string(),
        status: "DELETED".to_string(),
    }))
}

fn local_strategy_paths(strategies_dir: &FsPath) -> Result<Vec<PathBuf>, String> {
    let entries = match fs::read_dir(strategies_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn find_local_strategy_metadata_path(
    strategies_dir: &FsPath,
    user_id: &str,
    strategy_id: &str,
) -> Result<Option<PathBuf>, String> {
    for path in local_strategy_paths(strategies_dir)? {
        let metadata = read_local_strategy_metadata(&path)?;
        let record_user_id = metadata
            .get("user_id")
            .and_then(JsonValue::as_str)
            .unwrap_or_default();
        if record_user_id != user_id {
            continue;
        }
        let record_id = metadata
            .get("strategy_id")
            .and_then(JsonValue::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default()
                    .to_string()
            });
        if record_id == strategy_id {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn read_local_strategy_record(metadata_path: &FsPath) -> Result<StrategyRecord, String> {
    let metadata = read_local_strategy_metadata(metadata_path)?;
    let code_path = metadata_path.with_extension("mq5");
    let code = fs::read_to_string(&code_path).unwrap_or_default();
    let id = metadata
        .get("strategy_id")
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            metadata_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_string()
        });
    Ok(StrategyRecord {
        id,
        name: metadata
            .get("strategy_name")
            .and_then(JsonValue::as_str)
            .or_else(|| metadata.get("name").and_then(JsonValue::as_str))
            .unwrap_or("Unnamed")
            .to_string(),
        code,
        explanation: metadata
            .get("explanation")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        status: metadata
            .get("status")
            .and_then(JsonValue::as_str)
            .unwrap_or("DRAFT")
            .to_string(),
        session_id: metadata
            .get("session_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        user_id: metadata
            .get("user_id")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        pair: metadata
            .get("pair")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        timeframe: metadata
            .get("timeframe")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        created_at: metadata
            .get("created_at")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_string(),
        updated_at: metadata
            .get("updated_at")
            .and_then(JsonValue::as_str)
            .or_else(|| metadata.get("created_at").and_then(JsonValue::as_str))
            .unwrap_or("")
            .to_string(),
    })
}

fn read_local_strategy_metadata(metadata_path: &FsPath) -> Result<serde_json::Map<String, JsonValue>, String> {
    let value = serde_json::from_str::<JsonValue>(
        &fs::read_to_string(metadata_path).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| "local strategy metadata must be a JSON object".to_string())
}

fn write_local_strategy_metadata(
    metadata_path: &FsPath,
    metadata: &serde_json::Map<String, JsonValue>,
) -> Result<(), String> {
    fs::write(
        metadata_path,
        serde_json::to_string_pretty(&JsonValue::Object(metadata.clone()))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn current_iso8601_like_timestamp() -> String {
    format!("unix:{}", unix_timestamp_millis())
}
