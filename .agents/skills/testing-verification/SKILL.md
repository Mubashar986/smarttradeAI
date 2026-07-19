---
name: testing-verification
description: Creates a Stage 4 Testing & Completion Artifact. Defines a 10-point edge case matrix with expected outputs, provides copy-pasteable PowerShell/Docker commands for every test case, analyzes user-reported test results, and produces a final completion report.
---

# Testing & Verification Skill

This is Stage 4 of the SmartTradeAI task lifecycle. After the code is written and compiled, this skill produces the most exhaustive testing protocol possible — covering unit, integration, stress, failover, and security test categories with at least 10 cases each.

---

## Core Principles (Non-Negotiable)

1. **User runs all tests.** Never execute test commands directly. Provide exact, copy-pasteable PowerShell commands.
2. **Pre-test checklist first.** Before any test, verify the environment is in a known-good state (containers running, database healthy, no leftover test data).
3. **Categorize every test.** Each test case must be tagged with its category (Unit / Integration / Stress / Failover / Security).
4. **Clean up after testing.** Provide explicit commands to remove test data, stop containers, and restore the environment to its pre-test state.
5. **Watch the logs.** Every test section must tell the user to open a log tail in a second terminal.
6. **Code quality review.** After all tests pass, perform a code quality audit on the new code.

---

## Required Document Structure

Save as `task_X_Y_testing.md` in the artifact directory.

### Section 1: Pre-Test Environment Checklist
Before running any tests, the user must verify:

```markdown
## Pre-Test Checklist
Run these commands to verify your environment is ready:

` ` `powershell
# 1. Are all containers running?
docker compose ps

# 2. Is the database healthy?
docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT 1;"

# 3. Is the web server responding?
Invoke-RestMethod -Uri "http://localhost:3000/v1/strategies" -Method Get

# 4. Open a log tail in a SECOND terminal window (keep this open during all tests):
docker logs -f smarttrade-c2-engine
` ` `

If any of these fail, run:
` ` `powershell
docker compose up postgres redis c2-engine --build -d
` ` `
```

### Section 2: Test Categories & Edge Case Matrices

Each category must contain **at least 10 test cases** with specific inputs, expected outputs, and PowerShell commands.

---

#### Category A: Unit Tests (Compilation & Static Analysis)
These verify that the code compiles correctly and passes all existing unit tests.

```markdown
| ID | Test Case | Command | Expected Output |
|----|-----------|---------|-----------------|
| U-01 | Clean compilation (no warnings) | `cargo check` | 0 errors, 0 warnings |
| U-02 | All unit tests pass | `cargo test` | All tests pass |
| U-03 | No unused imports | `cargo check` | No "unused import" warnings |
| U-04 | No unused variables | `cargo check` | No "unused variable" warnings |
| U-05 | No dead code warnings | `cargo check` | No "never used" warnings |
| U-06 | Clippy lint pass | `cargo clippy` | No lint violations |
| U-07 | Format compliance | `cargo fmt --check` | No formatting diffs |
| U-08 | Test target compiles | `cargo test --no-run` | Compiles without errors |
| U-09 | Release build compiles | `cargo build --release` | Compiles in release mode |
| U-10 | Doc comments valid | `cargo doc --no-deps` | No doc warning errors |
```

---

#### Category B: Integration Tests (API & Database)
These verify that the REST API endpoints interact correctly with the database.

```markdown
| ID | Test Case | Trigger | Expected Output |
|----|-----------|---------|-----------------|
| I-01 | List strategies (empty DB) | GET /v1/strategies | `{ "strategies": [] }` |
| I-02 | Insert a strategy via SQL | INSERT INTO strategies | Row appears in DB |
| I-03 | List strategies (with data) | GET /v1/strategies | Returns inserted row |
| I-04 | Fetch single strategy by ID | GET /v1/strategies/:id | Returns specific row |
| I-05 | Update strategy fields | PUT /v1/strategies/:id | Fields updated in DB |
| I-06 | Soft-delete a strategy | DELETE /v1/strategies/:id | Status set to DELETED |
| I-07 | Deleted strategy hidden from list | GET /v1/strategies | Deleted row not in list |
| I-08 | Invalid strategy ID (404) | GET /v1/strategies/99999 | 404 or empty response |
| I-09 | Malformed JSON body | POST with bad JSON | 400 Bad Request |
| I-10 | Query scoped to user_id | GET as different user | Empty array (isolation) |
```

---

#### Category C: Stress Tests (Concurrency & Pool Saturation)
These verify the connection pool handles high load without leaking sockets or crashing.

```markdown
| ID | Test Case | Trigger | Expected Output |
|----|-----------|---------|-----------------|
| S-01 | 15 sequential requests | Loop 15 GETs | Connection count stable |
| S-02 | 50 sequential requests | Loop 50 GETs | Connection count stable |
| S-03 | 5 parallel workers x 50 requests | 250 concurrent GETs | No socket leaks |
| S-04 | 10 parallel workers x 100 requests | 1000 concurrent GETs | Pool queues correctly |
| S-05 | Rapid insert-then-read cycle | INSERT then immediate GET | Data consistent |
| S-06 | Pool ceiling test (max_connections) | Exceed pool max | Requests queue, no crash |
| S-07 | Mixed read/write under load | Parallel GETs and POSTs | No deadlocks |
| S-08 | Long query simulation | pg_sleep(2) query | Other requests not blocked |
| S-09 | Connection count before stress | pg_stat_activity count | Baseline established |
| S-10 | Connection count after stress | pg_stat_activity count | Count returns to baseline |
```

---

#### Category D: Failover & Recovery Tests
These verify the system recovers gracefully from infrastructure failures.

```markdown
| ID | Test Case | Trigger | Expected Output |
|----|-----------|---------|-----------------|
| F-01 | Stop Postgres, query API | docker compose stop postgres | 500 Internal Server Error |
| F-02 | Restart Postgres, query API | docker compose start postgres | Automatic recovery, 200 OK |
| F-03 | Kill all DB connections server-side | pg_terminate_backend() | Pool reconnects silently |
| F-04 | Stop Redis, query API | docker compose stop redis | Non-Redis routes still work |
| F-05 | Restart c2-engine container | docker compose restart c2-engine | Server boots cleanly |
| F-06 | Empty DATABASE_URL (dev mode) | APP_ENV=development | Falls back to local files |
| F-07 | Empty DATABASE_URL (prod mode) | APP_ENV=production | FATAL error, exit code 1 |
| F-08 | Malformed DATABASE_URL | DATABASE_URL="garbage" | Panic with clear error message |
| F-09 | DNS resolution failure | DATABASE_URL with bad host | Panic with connection error |
| F-10 | Postgres out of connections | Set max_connections=1 in Postgres | Pool waits, no crash |
```

---

#### Category E: Security & Input Validation Tests
These verify the system handles malicious or unexpected inputs safely.

```markdown
| ID | Test Case | Trigger | Expected Output |
|----|-----------|---------|-----------------|
| X-01 | SQL injection in strategy name | Name = `'; DROP TABLE strategies;--` | Escaped safely, no table drop |
| X-02 | XSS payload in strategy code | Code = `<script>alert(1)</script>` | Stored as literal string |
| X-03 | Extremely long strategy name | 10,000 character string | Handled or rejected gracefully |
| X-04 | Unicode/emoji in strategy name | Name = `🚀策略テスト` | Stored and retrieved correctly |
| X-05 | Null bytes in input | Name contains \x00 | Rejected or sanitized |
| X-06 | Empty string fields | All fields empty | Validation error, not crash |
| X-07 | Missing required fields | POST without `name` | 400 Bad Request |
| X-08 | Negative/zero strategy ID | GET /v1/strategies/-1 | 404 or validation error |
| X-09 | HTTP method not allowed | PATCH /v1/strategies | 405 Method Not Allowed |
| X-10 | Oversized request body | 10MB JSON payload | 413 or timeout, no OOM crash |
```

### Section 3: Observability Guide
Tell the user exactly what to watch in the logs during testing:

```markdown
## What to Watch in the Logs

While running tests, keep `docker logs -f smarttrade-c2-engine` open and look for:

| Log Pattern | Meaning | Concern Level |
|------------|---------|--------------|
| `connecting to PostgreSQL database...` | Pool initializing at boot | ✅ Normal |
| `WARN ... falling back to local file storage` | No DATABASE_URL set | ⚠️ Expected in dev only |
| `FATAL: DATABASE_URL ...` | Production safety triggered | ✅ Expected in prod test |
| `failed to connect to database` | Connection string invalid | 🔴 Fix config |
| `pool timed out waiting for connection` | All pool slots occupied | 🟡 Increase max_connections |
| No logs appearing on API call | Request not reaching server | 🔴 Check container is running |
```

### Section 4: Code Quality Review
After all tests pass, perform a structured code quality audit:

```markdown
## Code Quality Audit

### 4.1 Error Handling Review
- [ ] No bare `.unwrap()` calls in production code paths
- [ ] All `Result` types propagated with `?` or explicitly handled
- [ ] Error messages are descriptive (include context like variable names)
- [ ] No silent error swallowing (`let _ = ...` on Results)

### 4.2 Ownership & Memory Review
- [ ] No unnecessary `.clone()` calls (borrow instead where possible)
- [ ] No `Arc` wrapping where a reference would suffice
- [ ] No leaked resources (file handles, connections opened but never closed)
- [ ] All `Drop` implementations correct (if custom drops exist)

### 4.3 Concurrency Safety Review
- [ ] No data races (all shared state behind Arc/Mutex or channels)
- [ ] No deadlock potential (lock ordering consistent)
- [ ] Async functions do not hold locks across `.await` points
- [ ] Pool connections returned promptly (no long-held borrows)

### 4.4 API Design Review
- [ ] Public function signatures are minimal (don't expose internals)
- [ ] Return types use domain-specific errors, not generic strings
- [ ] Endpoint response shapes match documented API contracts
- [ ] HTTP status codes are semantically correct (404 for not found, 400 for bad input)

### 4.5 Code Hygiene Review
- [ ] No dead code (unused functions, imports, variables)
- [ ] No TODO/FIXME/HACK comments left unaddressed
- [ ] All public items have doc comments
- [ ] Consistent naming conventions (snake_case for functions, CamelCase for types)
- [ ] No hardcoded magic numbers (use named constants)
```

### Section 5: Post-Test Cleanup
After all tests are complete, restore the environment:

```markdown
## Post-Test Cleanup

` ` `powershell
# 1. Remove test data from the database
docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "DELETE FROM strategies WHERE name LIKE '%Test%';"

# 2. Verify cleanup
docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT count(*) FROM strategies;"

# 3. Check final connection count (should be back to baseline)
docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT count(*) FROM pg_stat_activity WHERE datname = 'smarttrade';"

# 4. (Optional) Stop all containers when done
docker compose down
` ` `
```

### Section 6: Test Results Analysis
When the user shares test output, analyze using this framework:

```markdown
## Test Results Analysis

| Test ID | Status | Observation | Root Cause (if failed) | Fix Applied |
|---------|--------|-------------|----------------------|-------------|
| U-01 | ✅ PASS | 0 warnings | — | — |
| S-03 | ❌ FAIL | Connection count rose to 16 | Pool scaled up under load | Expected behavior, not a bug |
```

If any test fails:
1. Identify the root cause.
2. Determine if it is a real bug or expected behavior.
3. If a bug, update the Design/Plan and re-implement.
4. Re-run only the failed tests.
5. Repeat until 100% pass.

### Section 7: Completion Report
Once all tests pass and code quality is verified:

```markdown
## Completion Report

| Metric | Value |
|--------|-------|
| Total Tests Executed | 50 |
| Tests Passed | 50 |
| Tests Failed | 0 |
| Code Quality Issues Found | 0 |
| Files Modified | 3 |
| Lines Changed | 25 |
| Commit Hash | `abc1234` |
| Pushed to Remote | ✅ Yes |
```

---

## Workflow Checklist

Before marking the Testing Artifact as complete, verify:

- [ ] Pre-test environment checklist provided
- [ ] At least 10 unit test cases defined
- [ ] At least 10 integration test cases defined
- [ ] At least 10 stress test cases defined
- [ ] At least 10 failover test cases defined
- [ ] At least 10 security test cases defined
- [ ] All test cases have copy-pasteable PowerShell commands
- [ ] Observability guide provided (what logs to watch)
- [ ] Code quality audit completed (5 review categories)
- [ ] Post-test cleanup commands provided
- [ ] Test results analyzed (if user has shared output)
- [ ] Completion report filled in
