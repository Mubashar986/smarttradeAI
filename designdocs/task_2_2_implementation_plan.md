---
RequestFeedback: true
Stage: 3
Task: 2.2 Secure Entity Identifiers
---

# Task 2.2 — Implementation Plan

## 1. Change Summary Card

| Property | Value |
|---|---|
| Files modified | 8 existing files |
| Files created | 1 migration |
| Files deleted | 0 |
| Estimated code/schema change | ~180 lines |
| New dependencies | None — uuid is already in the workspace with v4 and serde |
| Estimated complexity | High: identifier rotation plus route/runtime changes |
| Estimated time | 45–90 minutes plus database verification |
| Risk level | High because primary keys and foreign-key references are migrated |
| Wire compatibility | Field names remain unchanged; values become canonical UUID strings |

## 2. Dependency Check

No new crate dependency is required. Existing declarations are present in workspace Cargo.toml, server/Cargo.toml, and runtime/Cargo.toml. The application already has Uuid::new_v4 available in runtime and uuid available in server.

The migration will use PostgreSQL 16 gen_random_uuid from pgcrypto; the SQL must explicitly create the extension if absent. The final schema will store canonical UUID strings in the existing string-compatible entity columns to limit API and runtime churn. The strategy primary key and audit foreign key must change from integer to string-compatible UUID storage.

## 3. Execution Order

1. 0002_secure_entity_ids.sql — add transactional backfill and final constraints first, because application code must not emit UUIDs into a schema that still requires numeric strategy IDs.
2. init.sql — align fresh Docker bootstrap with the post-migration schema.
3. state.rs — replace process-local counters with UUID v4 allocation.
4. sessions.rs and turns.rs — update focused API tests and preserve existing route flow.
5. runtime/src/smarttrade_tools.rs — generate and insert a UUID strategy ID.
6. server/src/routes/strategies.rs — remove i64 parsing and bind opaque IDs directly.
7. server/src/lib.rs — update fixtures and add end-to-end UUID path coverage.
8. SmartTradeAI_SRS.md — update examples and data-model documentation.

The migration precedes code because deployment order must not allow new UUID writes to hit the old SERIAL strategy column. Route code follows the shared allocator and schema so the compiler exposes mismatches in dependency order.

## 4. Step-by-Step Code and Schema Changes

### Step 1 — NEW 0002_secure_entity_ids.sql

Where: new SQLx migration after 0001_smarttrade_core.sql.  
Why: rotate legacy values and all dependent references atomically.

~~~diff
+++ b/services/c2-engine/plugins/smarttrade-mql5/db/migrations/0002_secure_entity_ids.sql
+BEGIN;
+CREATE EXTENSION IF NOT EXISTS pgcrypto;
+
+CREATE TEMP TABLE session_id_map ON COMMIT DROP AS
+SELECT id AS old_id, gen_random_uuid()::text AS new_id FROM sessions;
+CREATE UNIQUE INDEX ON session_id_map(old_id);
+CREATE TEMP TABLE task_id_map ON COMMIT DROP AS
+SELECT id AS old_id, gen_random_uuid()::text AS new_id FROM tasks;
+CREATE UNIQUE INDEX ON task_id_map(old_id);
+CREATE TEMP TABLE strategy_id_map ON COMMIT DROP AS
+SELECT id::text AS old_id, gen_random_uuid()::text AS new_id FROM strategies;
+CREATE UNIQUE INDEX ON strategy_id_map(old_id);
+
+-- Drop affected foreign keys, update dependent columns from the maps,
+-- swap primary-key values, recreate the same foreign keys, and add
+-- canonical UUID checks before committing.
+COMMIT;
~~~

The implementation must fill in the FK-safe swap for sessions.id, tasks.id, strategies.id, tasks.session_id, audit_logs.session_id, audit_logs.task_id, and strategy_audit_log.strategy_id. It must verify that no old ID remains before commit and must not update IDs in separate transactions.

### Step 2 — MODIFY init.sql

Where: existing users, sessions, tasks, strategies, and audit definitions.  
Why: a fresh database must have the same entity-ID contract as an upgraded database.

~~~diff
 CREATE TABLE IF NOT EXISTS strategies (
-    id SERIAL PRIMARY KEY,
+    id VARCHAR(255) PRIMARY KEY,
     name VARCHAR(255) NOT NULL,
...
 );

 CREATE TABLE IF NOT EXISTS strategy_audit_log (
-    id SERIAL PRIMARY KEY,
-    strategy_id INTEGER REFERENCES strategies(id),
+    id VARCHAR(255) PRIMARY KEY,
+    strategy_id VARCHAR(255) REFERENCES strategies(id),
...
 );
~~~

Also align audit_logs entity references and add UUID-format checks after confirming how legacy bootstrap rows are handled. Compare init.sql and the final migration schema by applying both to disposable PostgreSQL 16 databases.

### Step 3 — MODIFY state.rs

Where: imports, AppState fields, constructor, and allocation methods around lines 1–4 and 147–195.  
Why: remove the process-local sequence source.

~~~diff
-use std::sync::atomic::{AtomicU64, Ordering};
 use std::sync::Arc;
...
 use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
+use uuid::Uuid;
...
     pub sessions: SessionStore,
     pub tasks: TaskStore,
-    next_session_id: Arc<AtomicU64>,
-    next_task_id: Arc<AtomicU64>,
...
-                next_session_id: Arc::new(AtomicU64::new(1)),
-                next_task_id: Arc::new(AtomicU64::new(1)),
...
     pub(crate) fn allocate_session_id(&self) -> SessionId {
-        let id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
-        format!("session-{id}")
+        Uuid::new_v4().to_string()
     }
     pub(crate) fn allocate_task_id(&self) -> TaskId {
-        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
-        format!("task-{id}")
+        Uuid::new_v4().to_string()
     }
~~~

Add tests for canonical UUID parsing, uniqueness over a bounded sample, and the absence of the old prefix contract. Keep the aliases string-compatible in this first slice to avoid changing event serialization.

### Step 4 — MODIFY sessions.rs and turns.rs

Where: creation/enqueue paths and existing tests.  
Why: lock the public contract to opaque IDs while retaining the existing handler flow.

~~~diff
 let session_id = state.allocate_session_id();
 let session = Session::new(session_id.clone());
...
 let task_id = state.allocate_task_id();
 let mut context = payload.context;
 context.task_id = Some(task_id.clone());
~~~

The production lines remain structurally the same because allocation is centralized. Update assertions and add UUID parse checks rather than asserting session-1 or task-1.

### Step 5 — MODIFY runtime/src/smarttrade_tools.rs

Where: persist_strategy_postgres around lines 1006–1031.  
Why: PostgreSQL must no longer allocate a numeric strategy key.

~~~diff
 async fn persist_strategy_postgres(
     request: &SaveStrategyRequest,
     pool: &sqlx::PgPool,
 ) -> Result<String, String> {
-    let strategy_id: i32 = sqlx::query_scalar(
+    let strategy_id = Uuid::new_v4().to_string();
+    sqlx::query(
         r#"
         INSERT INTO strategies
-            (name, code, explanation, status, session_id, user_id, pair, timeframe, created_at, updated_at)
+            (id, name, code, explanation, status, session_id, user_id, pair, timeframe, created_at, updated_at)
         VALUES
-            ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
-        RETURNING id
+            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
         "#,
     )
+    .bind(&strategy_id)
     .bind(&request.strategy_name)
...
-    Ok(strategy_id.to_string())
+    Ok(strategy_id)
 }
~~~

The actual diff must keep placeholder count and bind order exact. If the final schema uses native uuid rather than string-compatible columns, bind Uuid and convert only at the return boundary.

### Step 6 — MODIFY routes/strategies.rs

Where: load_db_strategy, update_db_strategy, and soft_delete_db_strategy around lines 175–287.  
Why: UUID paths must reach SQL instead of being rejected by numeric parsing.

~~~diff
-    let Ok(strategy_id) = strategy_id.parse::<i64>() else {
-        return Ok(None);
-    };
     let row = sqlx::query(
...
     )
     .bind(user_id)
     .bind(strategy_id)
~~~

Apply the same removal to all three helpers. Preserve WHERE user_id = $1 and status <> 'DELETED'; changing identifier secrecy must not weaken ownership filtering.

### Step 7 — MODIFY server tests and docs

Where: server/src/lib.rs fixtures and SmartTradeAI_SRS.md examples.  
Why: tests and operational examples must prove and teach the new contract.

~~~diff
-    let created = "session-1";
+    let created = create_session(&client, &server).await;
+    assert!(uuid::Uuid::parse_str(&created.session_id).is_ok());
~~~

Keep local strategy fixture IDs only where they intentionally test local-file behavior; add at least one UUID fixture for parity. Replace stale SRS examples with a placeholder UUID rather than a sequential session.

## 5. Verification Commands

These commands are for execution after approval; they are not run during the design gate.

### Static Rust checks

~~~powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine\rust"
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
~~~

### Unit and integration checks

~~~powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine\rust"
cargo test --workspace
~~~

### Fresh and upgrade database checks

~~~powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine"
docker compose down -v
docker compose up -d postgres
docker compose logs postgres --tail 100
~~~

Run the migration path against a disposable PostgreSQL 16 instance. The upgrade fixture must contain session-1, task-1, and numeric strategy rows, then assert that all IDs and references are UUID-shaped and row counts are unchanged. The fresh bootstrap check must assert that strategies.id is no longer an integer sequence.

### API smoke check

~~~powershell
$session = Invoke-RestMethod -Uri "http://localhost:3000/v1/sessions" -Method Post
[Guid]::Parse($session.session_id)
$turn = Invoke-RestMethod -Uri "http://localhost:3000/v1/sessions/$($session.session_id)/turn" -Method Post -ContentType "application/json" -Body '{"text":"explain the current strategy"}'
[Guid]::Parse($turn.task_id)
~~~

## 6. Rollback Instructions

~~~powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI"
git diff --stat HEAD
git revert <secure-id-commit>
~~~

For an already-applied primary-key migration, restore the pre-migration PostgreSQL snapshot first; a source revert cannot reconstruct the old IDs. Do not use destructive worktree commands because unrelated user changes are present.

---

## ⛔ STOP! USER REVIEW REQUIRED

This plan changes 8 existing files, creates 1 migration, and includes a high-risk primary-key rotation. Review the migration strategy and diff previews carefully.

Reply with **“Approve”** or **“Proceed”** to begin implementation. No production code or schema will be written until approval.

