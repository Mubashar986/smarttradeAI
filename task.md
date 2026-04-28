# SmartTradeAI C2 Engine - Surgery Task Tracker

## Step 1: Delete Dead Crates
- [x] Delete `crates/claw-cli/`
- [x] Delete `crates/commands/`
- [x] Delete `crates/compat-harness/`
- [x] Delete `crates/lsp/`
- [x] Delete `crates/tools/`
- [x] Delete `crates/plugins/`
- [x] Update workspace `Cargo.toml` (remove `lsp-types` dep)

## Step 2: Strip Runtime
- [x] Delete 12 dead files from `runtime/src/`
- [x] Update `runtime/src/lib.rs` - remove dead module declarations
- [x] Update `runtime/Cargo.toml` - remove unused deps
- [x] Fix compilation errors

## Step 3: Add New Dependencies
- [x] Add `redis`, `sqlx`, `reqwest`, `regex`, `jsonwebtoken`, `argon2`, `uuid` to Cargo files
- [x] Enable `ws` feature on `axum`

## Step 4: Create `smarttrade_tools.rs`
- [x] Convert `classify_intent.py` -> Rust
- [x] Convert `detect_ambiguity.py` -> Rust
- [x] Convert `inject_skeleton.py` -> Rust
- [x] Convert `run_static_analysis.py` -> Rust
- [x] Convert `save_strategy.py` -> Rust
- [x] Convert `search_knowledge_base.py` -> Rust
- [x] Convert `compile_mql5.py` -> Rust
- [x] Implement `SmartTradeToolExecutor`

## Step 5: Convert Hook Logic to Rust
- [x] Convert `gate_enforcer.sh` -> Rust in `hooks.rs`

## Step 6: Add JWT Auth Middleware
- [x] Create `server/src/auth.rs`
- [x] Add Tower middleware layer

## Step 7: Add WebSocket Support
- [x] Add `/v1/ws/{session_id}` route

## Step 8: Add Strategy CRUD Routes
- [x] `GET /v1/strategies`
- [x] `GET /v1/strategies/{id}`
- [x] `PATCH /v1/strategies/{id}`
- [x] `DELETE /v1/strategies/{id}`

## Step 9: Create `c2-engine` Binary Crate
- [x] Create `crates/c2-engine/` with `main.rs`

## Step 10: Adapt System Prompt
- [x] Rewrite `runtime/src/prompt.rs`

## Step 11: Verify
- [ ] `cargo build`
- [ ] `cargo test`
