-- SmartTradeAI — Database Schema
-- Creates the strategies table for C2 strategy persistence

CREATE TABLE IF NOT EXISTS strategies (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    code TEXT NOT NULL,
    explanation TEXT DEFAULT '',
    status VARCHAR(50) NOT NULL DEFAULT 'DRAFT',
    session_id VARCHAR(255) DEFAULT '',
    user_id VARCHAR(255) DEFAULT '',
    pair VARCHAR(20) DEFAULT '',
    timeframe VARCHAR(10) DEFAULT '',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_strategies_user_id ON strategies(user_id);
CREATE INDEX IF NOT EXISTS idx_strategies_status ON strategies(status);
CREATE INDEX IF NOT EXISTS idx_strategies_session_id ON strategies(session_id);

-- Audit log for strategy state transitions
CREATE TABLE IF NOT EXISTS strategy_audit_log (
    id SERIAL PRIMARY KEY,
    strategy_id INTEGER REFERENCES strategies(id),
    old_status VARCHAR(50),
    new_status VARCHAR(50),
    changed_by VARCHAR(255) DEFAULT 'c2-engine',
    changed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
