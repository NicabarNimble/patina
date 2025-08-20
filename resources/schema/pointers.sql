-- Pattern Pointer System Schema
-- Links patterns (ideas/rules) to code implementations

-- Track pointers from patterns to code locations
CREATE TABLE IF NOT EXISTS pattern_pointers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern_id TEXT NOT NULL,              -- Pattern filename (e.g., 'dependable-rust')
    file_path TEXT NOT NULL,               -- File being pointed to
    line_start INTEGER,                    -- Starting line (optional)
    line_end INTEGER,                      -- Ending line (optional)
    symbol TEXT,                            -- AST symbol (function/struct/module name)
    compliance TEXT NOT NULL,              -- 'compliant', 'violation', 'partial', 'unknown'
    explanation TEXT,                       -- LLM's explanation of compliance
    suggested_fix TEXT,                     -- LLM's suggested fix if violation
    confidence REAL,                        -- LLM confidence score (0.0-1.0)
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_verified DATETIME DEFAULT CURRENT_TIMESTAMP,
    commit_hash TEXT,                       -- Git commit when pointer was created
    UNIQUE(pattern_id, file_path, symbol)
);

-- Track pattern compliance over time
CREATE TABLE IF NOT EXISTS compliance_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pointer_id INTEGER NOT NULL,
    compliance TEXT NOT NULL,
    explanation TEXT,
    commit_hash TEXT NOT NULL,
    checked_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pointer_id) REFERENCES pattern_pointers(id)
);

-- Track pattern spread metrics
CREATE TABLE IF NOT EXISTS pattern_spread (
    pattern_id TEXT PRIMARY KEY,
    total_implementations INTEGER DEFAULT 0,
    compliant_count INTEGER DEFAULT 0,
    violation_count INTEGER DEFAULT 0,
    partial_count INTEGER DEFAULT 0,
    last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_pointers_pattern ON pattern_pointers(pattern_id);
CREATE INDEX IF NOT EXISTS idx_pointers_file ON pattern_pointers(file_path);
CREATE INDEX IF NOT EXISTS idx_pointers_compliance ON pattern_pointers(compliance);
CREATE INDEX IF NOT EXISTS idx_history_pointer ON compliance_history(pointer_id);
CREATE INDEX IF NOT EXISTS idx_history_commit ON compliance_history(commit_hash);