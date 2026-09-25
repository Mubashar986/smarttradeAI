# Task 2.2 — Testing & Verification

> Stage: 4 — Testing and completion protocol\
> Implementation status: code and migration written; runtime commands are intentionally user-run under the project workflow.\
> Agent-side checks completed: identifier call-site search found no AtomicU64, session-{id}, task-{id}, or numeric strategy parsing; git diff --check passed.

## 1. Pre-Test Environment Checklist

Open two PowerShell terminals.

Terminal A:

```powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine"
docker compose up -d postgres redis c2-engine
docker compose ps
docker logs -f smarttrade-c2-engine
```

Terminal B:

```powershell
Set-Location "C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine\rust"
docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT 1;"
Invoke-WebRequest -Uri "http://localhost:3000/health" -Method Get
```

Take a PostgreSQL backup before an upgrade-migration rehearsal. Do not run destructive cleanup against data you need.

## 2. Test Matrix

### A. Unit and static checks

| ID   | Case                   | PowerShell command                                                         | Expected    |
| ---- | ---------------------- | -------------------------------------------------------------------------- | ----------- |
| U-01 | Format                 | cargo fmt --all -- --check                                                 | No diff     |
| U-02 | Type check             | cargo check --workspace --all-targets                                      | Exit 0      |
| U-03 | UUID allocator test    | cargo test -p server generated\_entity\_ids\_are\_unique\_uuid\_v4\_values | Pass        |
| U-04 | Local strategy ID test | cargo test -p runtime save\_strategy\_falls\_back\_to\_local\_files        | Pass        |
| U-05 | Session API test       | cargo test -p server creates\_lists\_and\_gets\_v1\_sessions               | Pass        |
| U-06 | Task API test          | cargo test -p server accepts\_v1\_turn\_and\_exposes\_task\_status         | Pass        |
| U-07 | Test compilation       | cargo test --workspace --no-run                                            | Exit 0      |
| U-08 | Full tests             | cargo test --workspace                                                     | All pass    |
| U-09 | Lint                   | cargo clippy --workspace --all-targets -- -D warnings                      | No warnings |
| U-10 | Documentation          | cargo doc --workspace --no-deps                                            | Exit 0      |

### B. Integration and database checks

| ID   | Case                 | PowerShell command                                                                                                                                                    | Expected             |
| ---- | -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------- |
| I-01 | Migration applied    | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT version FROM \_sqlx\_migrations ORDER BY version;"                                     | Includes 2           |
| I-02 | Strategy ID column   | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "\d strategies"                                                                                | id is varchar        |
| I-03 | Session check        | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "\d sessions"                                                                                  | UUID check present   |
| I-04 | Task check           | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "\d tasks"                                                                                     | UUID check present   |
| I-05 | Create session       | $session = Invoke-RestMethod <http://localhost:3000/v1/sessions> -Method Post                                                                                         | session\_id returned |
| I-06 | Parse session        | \[Guid]::Parse(\$session.session\_id)                                                                                                                                 | No exception         |
| I-07 | Submit turn          | $turn = Invoke-RestMethod "http://localhost:3000/v1/sessions/$(\$session.session\_id)/turn" -Method Post -ContentType application/json -Body '{"text":"Explain RSI"}' | task\_id returned    |
| I-08 | Parse task           | \[Guid]::Parse(\$turn.task\_id)                                                                                                                                       | No exception         |
| I-09 | Read task            | Invoke-RestMethod "<http://localhost:3000/v1/tasks/$($turn.task_id)>" -Method Get                                                                                     | Same task ID         |
| I-10 | Opaque strategy path | Invoke-WebRequest <http://localhost:3000/v1/strategies/550e8400-e29b-41d4-a716-446655440000> -SkipHttpErrorCheck                                                      | 404, never 500       |

### C. Stress and concurrency checks

| ID   | Case                | PowerShell command                                                                                                                                                                                                                                      | Expected            |
| ---- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| S-01 | 20 sessions         | 1..20 | ForEach-Object { Invoke-RestMethod <http://localhost:3000/v1/sessions> -Method Post }                                                                                                                                                           | 20 responses        |
| S-02 | 20 distinct IDs     | $ids = 1..20 \| ForEach-Object { (Invoke-RestMethod http://localhost:3000/v1/sessions -Method Post).session_id }; ($ids \| Select-Object -Unique).Count                                                                                                 | 20                  |
| S-03 | Parse 50 IDs        | 1..50 | ForEach-Object { [Guid]::Parse((Invoke-RestMethod <http://localhost:3000/v1/sessions> -Method Post).session\_id) }                                                                                                                              | No exception        |
| S-04 | Five worker jobs    | 1..5 | ForEach-Object { Start-Job { 1..20 | ForEach-Object { Invoke-RestMethod <http://localhost:3000/v1/sessions> -Method Post } } } \| Wait-Job \| Receive-Job                                                                                        | 100 responses       |
| S-05 | 50 parallel IDs     | $ids = 1..50 \| ForEach-Object -Parallel { (Invoke-RestMethod http://localhost:3000/v1/sessions -Method Post).session_id }; ($ids \| Select-Object -Unique).Count                                                                                       | 50                  |
| S-06 | Restart then create | docker compose restart c2-engine; Start-Sleep 5; Invoke-RestMethod <http://localhost:3000/v1/sessions> -Method Post                                                                                                                                     | UUID, not session-1 |
| S-07 | 10 rapid tasks      | 1..10 \| ForEach-Object { Invoke-RestMethod "<http://localhost:3000/v1/sessions/$($session.session_id)/turn>" -Method Post -ContentType application/json -Body '{"text":"Explain RSI"}' }                                                               | 10 responses        |
| S-08 | 10 distinct tasks   | $taskIds = 1..10 \| ForEach-Object { (Invoke-RestMethod "http://localhost:3000/v1/sessions/$($session.session_id)/turn" -Method Post -ContentType application/json -Body '{"text":"Explain RSI"}').task_id }; ($taskIds \| Select-Object -Unique).Count | 10                  |
| S-09 | Connection baseline | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT count(\*) FROM pg\_stat\_activity WHERE datname='smarttrade';"                                                                                                           | Record count        |
| S-10 | Connection recovery | Repeat S-04, wait 30 seconds, rerun S-09                                                                                                                                                                                                                | Near baseline       |

ForEach-Object -Parallel requires PowerShell 7. Use S-04 jobs in Windows PowerShell 5.1.

### D. Failover and recovery checks

| ID   | Case                   | PowerShell command                                                                                                               | Expected              |
| ---- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| F-01 | Stop DB                | docker compose stop postgres; Invoke-WebRequest <http://localhost:3000/v1/strategies> -SkipHttpErrorCheck                        | Controlled error      |
| F-02 | Restore DB             | docker compose start postgres; Start-Sleep 5; Invoke-WebRequest <http://localhost:3000/health>                                   | 200                   |
| F-03 | Restart engine         | docker compose restart c2-engine; Start-Sleep 5; Invoke-WebRequest <http://localhost:3000/health>                                | 200                   |
| F-04 | Inspect migration boot | docker compose logs c2-engine --tail 100                                                                                         | No migration error    |
| F-05 | Dump before rehearsal  | docker exec -i smarttrade-postgres pg\_dump -U smarttrade smarttrade > smarttrade-before-task-2-2.sql                            | Backup created        |
| F-06 | Migration success flag | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT success FROM \_sqlx\_migrations WHERE version=2;" | true                  |
| F-07 | Local-mode boot        | docker compose run --rm -e DATABASE\_URL= -e APP\_ENV=development c2-engine                                                      | Local fallback        |
| F-08 | Production guard       | docker compose run --rm -e DATABASE\_URL= -e APP\_ENV=production c2-engine                                                       | Fails fast            |
| F-09 | Check errors           | docker logs smarttrade-c2-engine --tail 200                                                                                      | No FK/migration error |
| F-10 | UUID after recovery    | $recovered = Invoke-RestMethod http://localhost:3000/v1/sessions -Method Post; [Guid]::Parse($recovered.session\_id)             | No exception          |

### E. Security and input-validation checks

| ID   | Case                          | PowerShell command                                                                                                                                                                                                        | Expected                                |
| ---- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| X-01 | Old session probe             | Invoke-WebRequest <http://localhost:3000/v1/sessions/session-1> -SkipHttpErrorCheck                                                                                                                                       | 404                                     |
| X-02 | Old task probe                | Invoke-WebRequest <http://localhost:3000/v1/tasks/task-1> -SkipHttpErrorCheck                                                                                                                                             | 404                                     |
| X-03 | Numeric strategy probe        | Invoke-WebRequest <http://localhost:3000/v1/strategies/1> -SkipHttpErrorCheck                                                                                                                                             | 404                                     |
| X-04 | Unknown UUID strategy         | Invoke-WebRequest <http://localhost:3000/v1/strategies/550e8400-e29b-41d4-a716-446655440000> -SkipHttpErrorCheck                                                                                                          | 404                                     |
| X-05 | Injection-shaped path         | Invoke-WebRequest "<http://localhost:3000/v1/strategies/1%27%20OR%201%3D1-->" -SkipHttpErrorCheck                                                                                                                         | 404 or 400                              |
| X-06 | Reject sequential DB ID       | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "INSERT INTO sessions (id) VALUES ('session-1');"                                                                                                  | Constraint rejection                    |
| X-07 | Allow v4 UUID                 | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "INSERT INTO sessions (id) VALUES ('550e8400-e29b-41d4-a716-446655440000'); DELETE FROM sessions WHERE id='550e8400-e29b-41d4-a716-446655440000';" | Insert/delete succeeds                  |
| X-08 | Reject uppercase UUID         | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "INSERT INTO sessions (id) VALUES ('550E8400-E29B-41D4-A716-446655440000');"                                                                       | Constraint rejection                    |
| X-09 | Reject v1 UUID                | docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "INSERT INTO sessions (id) VALUES ('f47ac10b-58cc-11cf-a447-0011aabbccdd');"                                                                       | Constraint rejection                    |
| X-10 | Confirm auth remains separate | Invoke-WebRequest <http://localhost:3000/v1/strategies/550e8400-e29b-41d4-a716-446655440000> -Headers @{Authorization='Bearer invalid'} -SkipHttpErrorCheck                                                               | Auth rejects invalid token when enabled |

## 3. Observability Guide

Keep docker logs -f smarttrade-c2-engine visible.

| Signal                                     | Meaning                                 |
| ------------------------------------------ | --------------------------------------- |
| running PostgreSQL schema migrations       | Normal startup                          |
| failed to run database migrations          | Stop and inspect SQL                    |
| task\_id or session\_id in structured logs | Values should be UUID-shaped            |
| foreign key error                          | Reference migration defect              |
| pool timeout                               | Load issue, not an identifier collision |

## 4. Code Quality Audit

| Area                          | Status           | Evidence                                                    |
| ----------------------------- | ---------------- | ----------------------------------------------------------- |
| Error handling                | Reviewed         | SQL operations propagate errors; no new production unwrap   |
| Ownership                     | Reviewed         | Shared atomic counters removed; no new Arc/lock             |
| Concurrency                   | Reviewed         | UUID creation is local and contention-free                  |
| API shape                     | Reviewed         | Field names and string serialization preserved              |
| Hygiene                       | Reviewed         | Old counter/numeric parsing search clean; diff check passes |
| Runtime/database verification | Pending user run | U-01 through X-10                                           |

## 5. Post-Test Cleanup

```powershell
docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "DELETE FROM sessions WHERE id='550e8400-e29b-41d4-a716-446655440000';"
docker exec -i smarttrade-postgres psql -U smarttrade -d smarttrade -c "SELECT count(*) FROM pg_stat_activity WHERE datname='smarttrade';"
docker compose down
```

Do not use docker compose down -v unless you explicitly intend to erase local PostgreSQL volumes.

## 6. Completion Report

| Metric                   | Current state                                                                    |
| ------------------------ | -------------------------------------------------------------------------------- |
| Implementation           | Complete                                                                         |
| Agent-side static checks | Passed: diff integrity and call-site review                                      |
| Runtime/database tests   | Pending user execution per project workflow                                      |
| Remaining risk           | Rehearse primary-key rotation on a disposable upgrade database before production |

