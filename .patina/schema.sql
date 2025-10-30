-- ============================================================================
-- PATINA PERSONA KNOWLEDGE BASE - COMPLETE SCHEMA
-- ============================================================================
-- Combines observation extraction (Layer 1) + persona beliefs (Layer 2)

-- ============================================================================
-- LAYER 1: OBSERVATION EXTRACTION (Evidence - What You DID)
-- ============================================================================

-- Core session metadata
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,                    -- e.g. '20251010-061739'
    started_at TIMESTAMP NOT NULL,          -- ISO 8601
    work_type TEXT NOT NULL,                -- exploration, pattern-work, experiment
    git_branch TEXT,
    starting_commit TEXT,
    session_tag TEXT,
    llm TEXT DEFAULT 'claude',
    commits_count INTEGER DEFAULT 0,
    files_changed INTEGER DEFAULT 0,
    duration_minutes INTEGER
);

CREATE INDEX IF NOT EXISTS idx_sessions_work_type ON sessions(work_type);
CREATE INDEX IF NOT EXISTS idx_sessions_branch ON sessions(git_branch);
CREATE INDEX IF NOT EXISTS idx_sessions_date ON sessions(started_at);

-- Pattern observations
CREATE TABLE IF NOT EXISTS patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    pattern_name TEXT NOT NULL,
    category TEXT NOT NULL,                 -- security, architecture, workflow, infrastructure
    description TEXT,
    first_seen TIMESTAMP,
    last_seen TIMESTAMP,
    observation_count INTEGER DEFAULT 1,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_patterns_name ON patterns(pattern_name);
CREATE INDEX IF NOT EXISTS idx_patterns_category ON patterns(category);

-- Technology usage
CREATE TABLE IF NOT EXISTS technologies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    tech_name TEXT NOT NULL,
    purpose TEXT NOT NULL,
    tech_category TEXT,                     -- language, tool, framework, service
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_technologies_name ON technologies(tech_name);

-- Key decisions made during sessions
CREATE TABLE IF NOT EXISTS decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    choice TEXT NOT NULL,
    rationale TEXT NOT NULL,
    decision_type TEXT,                     -- philosophical, pragmatic, technical
    alternatives_considered TEXT,           -- JSON array
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Challenges and solutions
CREATE TABLE IF NOT EXISTS challenges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    problem TEXT NOT NULL,
    solution TEXT NOT NULL,
    challenge_category TEXT,                -- performance, security, architecture, tooling
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_challenges_category ON challenges(challenge_category);

-- ============================================================================
-- LAYER 2: PERSONA BELIEFS (What You BELIEVE)
-- ============================================================================

-- Domains (Knowledge blobs)
CREATE TABLE IF NOT EXISTS domains (
    name TEXT PRIMARY KEY,                  -- e.g. 'rust', 'ecs', 'bevy', 'game-dev'
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Questions generated from observations
CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    text TEXT NOT NULL,                     -- "Do you prefer ECS for game projects?"
    generated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    asked BOOLEAN DEFAULT FALSE,
    asked_at TIMESTAMP,
    priority INTEGER DEFAULT 5,             -- 1-10, higher = more important
    status TEXT DEFAULT 'pending'           -- 'pending', 'answered', 'refined', 'skipped'
);

CREATE INDEX IF NOT EXISTS idx_questions_status ON questions(status);
CREATE INDEX IF NOT EXISTS idx_questions_priority ON questions(priority);

-- Link questions to domain tags (many-to-many)
CREATE TABLE IF NOT EXISTS question_domains (
    question_id INTEGER NOT NULL,
    domain_name TEXT NOT NULL,
    PRIMARY KEY (question_id, domain_name),
    FOREIGN KEY (question_id) REFERENCES questions(id),
    FOREIGN KEY (domain_name) REFERENCES domains(name)
);

-- Link questions to observation evidence
CREATE TABLE IF NOT EXISTS question_evidence (
    question_id INTEGER NOT NULL,
    evidence_type TEXT NOT NULL,            -- 'pattern', 'technology', 'decision', 'challenge'
    evidence_id INTEGER NOT NULL,           -- FK to patterns, technologies, etc.
    session_id TEXT NOT NULL,
    PRIMARY KEY (question_id, evidence_type, evidence_id),
    FOREIGN KEY (question_id) REFERENCES questions(id),
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Beliefs (Atomic facts from answers)
CREATE TABLE IF NOT EXISTS beliefs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    statement TEXT NOT NULL,                -- "prefers_ecs_for_game_projects"
    value BOOLEAN NOT NULL,                 -- true/false from yes/no answer
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_validated TIMESTAMP,               -- Last time observation reinforced this
    confidence REAL DEFAULT 0.5,            -- 0.0-1.0, starts at 0.5
    observation_count INTEGER DEFAULT 1,
    question_id INTEGER,                    -- Original question that created this
    parent_belief_id INTEGER,               -- If this refined another belief
    active BOOLEAN DEFAULT TRUE,            -- False if superseded
    FOREIGN KEY (question_id) REFERENCES questions(id),
    FOREIGN KEY (parent_belief_id) REFERENCES beliefs(id)
);

CREATE INDEX IF NOT EXISTS idx_beliefs_active ON beliefs(active);
CREATE INDEX IF NOT EXISTS idx_beliefs_statement ON beliefs(statement);
CREATE INDEX IF NOT EXISTS idx_beliefs_confidence ON beliefs(confidence);
CREATE INDEX IF NOT EXISTS idx_beliefs_active_confidence ON beliefs(active, confidence);

-- Link beliefs to domains (same belief, multiple perspectives)
CREATE TABLE IF NOT EXISTS belief_domains (
    belief_id INTEGER NOT NULL,
    domain_name TEXT NOT NULL,
    domain_statement TEXT NOT NULL,         -- How this domain expresses the belief
    PRIMARY KEY (belief_id, domain_name),
    FOREIGN KEY (belief_id) REFERENCES beliefs(id),
    FOREIGN KEY (domain_name) REFERENCES domains(name)
);

-- Track observations that validate or contradict beliefs
CREATE TABLE IF NOT EXISTS belief_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    belief_id INTEGER NOT NULL,
    session_id TEXT NOT NULL,
    observation_type TEXT NOT NULL,         -- 'pattern', 'technology', 'decision'
    observation_id INTEGER NOT NULL,
    validates BOOLEAN NOT NULL,             -- true = supports, false = contradicts
    observed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (belief_id) REFERENCES beliefs(id),
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_belief_observations_validates ON belief_observations(belief_id, validates);

-- Track conflicts when observation contradicts belief
CREATE TABLE IF NOT EXISTS belief_conflicts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    belief_id INTEGER NOT NULL,
    session_id TEXT NOT NULL,
    observation_type TEXT NOT NULL,
    observation_id INTEGER NOT NULL,
    conflict_description TEXT,
    detected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    resolved BOOLEAN DEFAULT FALSE,
    resolution_question_id INTEGER,         -- New question generated to resolve
    FOREIGN KEY (belief_id) REFERENCES beliefs(id),
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (resolution_question_id) REFERENCES questions(id)
);

CREATE INDEX IF NOT EXISTS idx_belief_conflicts_resolved ON belief_conflicts(resolved);

-- Persona sessions (Interactive extraction sessions)
CREATE TABLE IF NOT EXISTS persona_sessions (
    id TEXT PRIMARY KEY,                    -- e.g. 'persona-20251028-143000'
    domain_name TEXT NOT NULL,
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    ended_at TIMESTAMP,
    questions_asked INTEGER DEFAULT 0,
    beliefs_created INTEGER DEFAULT 0,
    beliefs_refined INTEGER DEFAULT 0,
    conflicts_resolved INTEGER DEFAULT 0,
    FOREIGN KEY (domain_name) REFERENCES domains(name)
);

-- Track which questions were asked in which persona session
CREATE TABLE IF NOT EXISTS persona_session_questions (
    persona_session_id TEXT NOT NULL,
    question_id INTEGER NOT NULL,
    asked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    answer TEXT,                            -- 'yes', 'no', 'conditional: ...'
    belief_created_id INTEGER,
    PRIMARY KEY (persona_session_id, question_id),
    FOREIGN KEY (persona_session_id) REFERENCES persona_sessions(id),
    FOREIGN KEY (question_id) REFERENCES questions(id),
    FOREIGN KEY (belief_created_id) REFERENCES beliefs(id)
);

-- ============================================================================
-- INDEXES for Query Performance
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_questions_domains ON question_domains(domain_name);
CREATE INDEX IF NOT EXISTS idx_belief_domains ON belief_domains(domain_name);
CREATE INDEX IF NOT EXISTS idx_beliefs_active_confidence ON beliefs(active, confidence);
CREATE INDEX IF NOT EXISTS idx_conflicts_unresolved ON belief_conflicts(resolved) WHERE resolved = FALSE;
