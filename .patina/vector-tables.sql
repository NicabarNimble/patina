-- ============================================================================
-- VECTOR TABLES INITIALIZATION
-- ============================================================================
-- Creates sqlite-vss virtual tables for semantic search
--
-- PREREQUISITE: sqlite-vss extension must be loaded before running this:
--   .load /path/to/vss0
--
-- Usage from Rust:
--   conn.load_extension("vss0", None)?;
--   conn.execute_batch(include_str!("vector-tables.sql"))?;
-- ============================================================================

-- Vector embeddings for beliefs
CREATE VIRTUAL TABLE IF NOT EXISTS belief_vectors USING vss0(
    belief_id INTEGER PRIMARY KEY,
    embedding(384)  -- all-MiniLM-L6-v2 dimension
);

-- Vector embeddings for observations (patterns, technologies, decisions, challenges)
CREATE VIRTUAL TABLE IF NOT EXISTS observation_vectors USING vss0(
    observation_id INTEGER PRIMARY KEY,
    observation_type TEXT,  -- 'pattern', 'technology', 'decision', 'challenge'
    embedding(384)
);

-- Metadata index for filtered searches
CREATE INDEX IF NOT EXISTS idx_observation_vectors_type
    ON observation_vectors(observation_type);
