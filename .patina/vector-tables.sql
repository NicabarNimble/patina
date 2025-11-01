-- ============================================================================
-- VECTOR TABLES INITIALIZATION
-- ============================================================================
-- Creates sqlite-vec virtual tables for semantic search
--
-- PREREQUISITE: sqlite-vec extension must be registered before running this:
--   use sqlite_vec::sqlite3_vec_init;
--   unsafe {
--       sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
--   }
--
-- Usage from Rust:
--   conn.execute_batch(include_str!("vector-tables.sql"))?;
-- ============================================================================

-- Vector embeddings for beliefs
-- Note: Uses rowid for belief_id mapping
CREATE VIRTUAL TABLE IF NOT EXISTS belief_vectors USING vec0(
    embedding float[384]  -- all-MiniLM-L6-v2 dimension
);

-- Vector embeddings for observations (patterns, technologies, decisions, challenges)
-- Note: Uses rowid for observation_id, stores type as metadata column
CREATE VIRTUAL TABLE IF NOT EXISTS observation_vectors USING vec0(
    embedding float[384],
    observation_type TEXT  -- 'pattern', 'technology', 'decision', 'challenge'
);
