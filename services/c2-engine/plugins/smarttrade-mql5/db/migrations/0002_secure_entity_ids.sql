-- Replace process-local/sequential entity identifiers with opaque UUID v4 strings.
-- The mapping tables keep existing sessions, tasks, strategies, and audit
-- references connected while primary-key values are rotated.

BEGIN;

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TEMP TABLE session_id_map ON COMMIT DROP AS
SELECT id AS old_id, gen_random_uuid()::text AS new_id
FROM sessions;
CREATE UNIQUE INDEX session_id_map_old_id_idx ON session_id_map(old_id);
CREATE UNIQUE INDEX session_id_map_new_id_idx ON session_id_map(new_id);

CREATE TEMP TABLE task_id_map ON COMMIT DROP AS
SELECT id AS old_id, gen_random_uuid()::text AS new_id
FROM tasks;
CREATE UNIQUE INDEX task_id_map_old_id_idx ON task_id_map(old_id);
CREATE UNIQUE INDEX task_id_map_new_id_idx ON task_id_map(new_id);

CREATE TEMP TABLE strategy_id_map ON COMMIT DROP AS
SELECT id::text AS old_id, gen_random_uuid()::text AS new_id
FROM strategies;
CREATE UNIQUE INDEX strategy_id_map_old_id_idx ON strategy_id_map(old_id);
CREATE UNIQUE INDEX strategy_id_map_new_id_idx ON strategy_id_map(new_id);

-- Every current FK is dropped before any referenced primary key is replaced.
ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_session_id_fkey;
ALTER TABLE audit_logs DROP CONSTRAINT IF EXISTS audit_logs_session_id_fkey;
ALTER TABLE audit_logs DROP CONSTRAINT IF EXISTS audit_logs_task_id_fkey;
ALTER TABLE strategy_audit_log
    DROP CONSTRAINT IF EXISTS strategy_audit_log_strategy_id_fkey;

ALTER TABLE sessions ADD COLUMN secure_id VARCHAR(255);
ALTER TABLE tasks ADD COLUMN secure_id VARCHAR(255);
ALTER TABLE tasks ADD COLUMN secure_session_id VARCHAR(255);
ALTER TABLE strategies ADD COLUMN secure_id VARCHAR(255);
ALTER TABLE strategies ADD COLUMN secure_session_id VARCHAR(255);
ALTER TABLE audit_logs ADD COLUMN secure_session_id VARCHAR(255);
ALTER TABLE audit_logs ADD COLUMN secure_task_id VARCHAR(255);
ALTER TABLE strategy_audit_log ADD COLUMN secure_strategy_id VARCHAR(255);

UPDATE sessions AS entity
SET secure_id = mapping.new_id
FROM session_id_map AS mapping
WHERE mapping.old_id = entity.id;

UPDATE tasks AS entity
SET secure_id = mapping.new_id
FROM task_id_map AS mapping
WHERE mapping.old_id = entity.id;

UPDATE tasks AS entity
SET secure_session_id = mapping.new_id
FROM session_id_map AS mapping
WHERE mapping.old_id = entity.session_id;

UPDATE strategies AS entity
SET secure_id = mapping.new_id
FROM strategy_id_map AS mapping
WHERE mapping.old_id = entity.id::text;

UPDATE strategies AS entity
SET secure_session_id = mapping.new_id
FROM session_id_map AS mapping
WHERE mapping.old_id = entity.session_id;

UPDATE strategies
SET secure_session_id = session_id
WHERE session_id IS NULL OR session_id = '';

UPDATE audit_logs AS entity
SET secure_session_id = mapping.new_id
FROM session_id_map AS mapping
WHERE mapping.old_id = entity.session_id;

UPDATE audit_logs AS entity
SET secure_task_id = mapping.new_id
FROM task_id_map AS mapping
WHERE mapping.old_id = entity.task_id;

UPDATE strategy_audit_log AS entity
SET secure_strategy_id = mapping.new_id
FROM strategy_id_map AS mapping
WHERE mapping.old_id = entity.strategy_id::text;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM sessions WHERE secure_id IS NULL)
        OR EXISTS (SELECT 1 FROM tasks WHERE secure_id IS NULL OR secure_session_id IS NULL)
        OR EXISTS (SELECT 1 FROM strategies WHERE secure_id IS NULL)
        OR EXISTS (
            SELECT 1
            FROM strategies
            WHERE session_id IS NOT NULL
              AND session_id <> ''
              AND secure_session_id IS NULL
        )
    THEN
        RAISE EXCEPTION 'secure entity ID backfill did not cover every row or strategy session reference';
    END IF;
END
$$;

ALTER TABLE sessions DROP CONSTRAINT IF EXISTS sessions_pkey;
ALTER TABLE tasks DROP CONSTRAINT IF EXISTS tasks_pkey;
ALTER TABLE strategies DROP CONSTRAINT IF EXISTS strategies_pkey;

ALTER TABLE sessions DROP COLUMN id;
ALTER TABLE sessions RENAME COLUMN secure_id TO id;
ALTER TABLE sessions ADD CONSTRAINT sessions_pkey PRIMARY KEY (id);

ALTER TABLE tasks DROP COLUMN id;
ALTER TABLE tasks RENAME COLUMN secure_id TO id;
ALTER TABLE tasks DROP COLUMN session_id;
ALTER TABLE tasks RENAME COLUMN secure_session_id TO session_id;
ALTER TABLE tasks ADD CONSTRAINT tasks_pkey PRIMARY KEY (id);

ALTER TABLE strategies DROP COLUMN id;
ALTER TABLE strategies RENAME COLUMN secure_id TO id;
ALTER TABLE strategies DROP COLUMN session_id;
ALTER TABLE strategies RENAME COLUMN secure_session_id TO session_id;
ALTER TABLE strategies ADD CONSTRAINT strategies_pkey PRIMARY KEY (id);

ALTER TABLE audit_logs DROP COLUMN session_id;
ALTER TABLE audit_logs RENAME COLUMN secure_session_id TO session_id;
ALTER TABLE audit_logs DROP COLUMN task_id;
ALTER TABLE audit_logs RENAME COLUMN secure_task_id TO task_id;

ALTER TABLE strategy_audit_log DROP COLUMN strategy_id;
ALTER TABLE strategy_audit_log RENAME COLUMN secure_strategy_id TO strategy_id;

ALTER TABLE tasks
    ADD CONSTRAINT tasks_session_id_fkey
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE;
ALTER TABLE audit_logs
    ADD CONSTRAINT audit_logs_session_id_fkey
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL;
ALTER TABLE audit_logs
    ADD CONSTRAINT audit_logs_task_id_fkey
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL;
ALTER TABLE strategy_audit_log
    ADD CONSTRAINT strategy_audit_log_strategy_id_fkey
    FOREIGN KEY (strategy_id) REFERENCES strategies(id);

CREATE INDEX IF NOT EXISTS idx_tasks_session_id ON tasks(session_id);
CREATE INDEX IF NOT EXISTS idx_strategies_session_id ON strategies(session_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_session_id ON audit_logs(session_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_task_id ON audit_logs(task_id);

ALTER TABLE sessions
    ADD CONSTRAINT sessions_id_uuid_format
    CHECK (id ~ '^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$');
ALTER TABLE tasks
    ADD CONSTRAINT tasks_id_uuid_format
    CHECK (id ~ '^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$');
ALTER TABLE strategies
    ADD CONSTRAINT strategies_id_uuid_format
    CHECK (id ~ '^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$');

COMMIT;
