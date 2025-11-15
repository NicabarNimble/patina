-- Topic 0: Manual Smoke Test Observations
-- Extracted from sessions: 20251111-152022, 20251108-075248, 20251107-124740
-- Date: 2025-11-15
-- Purpose: Test semantic retrieval before building extraction automation

-- Create table if not exists (schema from analysis doc)
CREATE TABLE IF NOT EXISTS observations (
    id TEXT PRIMARY KEY,
    observation_type TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata TEXT,
    created_at TEXT NOT NULL
);

-- ============================================
-- Session 20251111-152022: Architectural Decisions
-- ============================================

INSERT INTO observations (id, observation_type, content, metadata, created_at)
VALUES
('obs_001', 'decision',
 'Build core value proposition (Ingest → Structure → Retrieve) before optimizing for performance',
 '{"source_type":"session","source_id":"20251111-152022","domains":["architecture","optimization","yagni"],"reliability":0.95}',
 '2025-11-15T20:00:00Z'),

('obs_002', 'challenge',
 'SQLite Connection uses RefCell internally and is not Sync - cannot be shared across threads with Arc<RwLock>',
 '{"source_type":"session","source_id":"20251111-152022","domains":["rust","sqlite","threading","concurrency"],"reliability":1.0}',
 '2025-11-15T20:01:00Z'),

('obs_003', 'pattern',
 'Ask "Are we solving the wrong problem?" when hitting unexpected technical constraints - may indicate invented requirements',
 '{"source_type":"session","source_id":"20251111-152022","domains":["problem-solving","architecture","yagni"],"reliability":0.90}',
 '2025-11-15T20:02:00Z'),

('obs_004', 'technology',
 'Use tiny_http over axum to maintain 100% synchronous architecture and avoid async/await spread',
 '{"source_type":"session","source_id":"20251111-152022","domains":["rust","async","architecture","dependencies"],"reliability":0.85}',
 '2025-11-15T20:03:00Z'),

('obs_005', 'decision',
 'Profile before adding daemon infrastructure - 500ms embeddings load may not require optimization',
 '{"source_type":"session","source_id":"20251111-152022","domains":["performance","profiling","yagni"],"reliability":0.90}',
 '2025-11-15T20:04:00Z'),

('obs_006', 'pattern',
 'Patina core mission: Enable user + any LLM to share accumulated knowledge across projects and time',
 '{"source_type":"session","source_id":"20251111-152022","domains":["vision","llm-integration","knowledge-management"],"reliability":1.0}',
 '2025-11-15T20:05:00Z'),

-- ============================================
-- Session 20251108-075248: Event Sourcing & Design
-- ============================================

('obs_007', 'decision',
 'Observations are immutable events - use event sourcing from start, not as later migration',
 '{"source_type":"session","source_id":"20251108-075248","domains":["event-sourcing","architecture","database"],"reliability":0.95}',
 '2025-11-15T20:06:00Z'),

('obs_008', 'pattern',
 'LLM interchangeability via adapters - patina init --llm=claude|gemini keeps framework LLM-agnostic',
 '{"source_type":"session","source_id":"20251108-075248","domains":["llm-integration","adapters","architecture"],"reliability":0.90}',
 '2025-11-15T20:07:00Z'),

('obs_009', 'decision',
 'Use manual triggers (git commits, /session-note, /session-update) over real-time file watching',
 '{"source_type":"session","source_id":"20251108-075248","domains":["capture","git","session-tracking"],"reliability":0.85}',
 '2025-11-15T20:08:00Z'),

('obs_010', 'pattern',
 'Separate scrape (extraction) from oxidize (vectorization) - extraction and embedding are different concerns',
 '{"source_type":"session","source_id":"20251108-075248","domains":["modularity","embeddings","extraction"],"reliability":0.90}',
 '2025-11-15T20:09:00Z'),

('obs_011', 'technology',
 'Git as event log - events are JSON files committed to repo for auditability, review, and distributed sync',
 '{"source_type":"session","source_id":"20251108-075248","domains":["git","event-sourcing","version-control"],"reliability":0.95}',
 '2025-11-15T20:10:00Z'),

-- ============================================
-- Session 20251107-124740: Domain Intelligence & Patterns
-- ============================================

('obs_012', 'decision',
 'Domains emerge automatically - LLM tags during scrape, discovers relationships during oxidize, no manual organization',
 '{"source_type":"session","source_id":"20251107-124740","domains":["domains","llm-analysis","emergent-design"],"reliability":0.95}',
 '2025-11-15T20:11:00Z'),

('obs_013', 'pattern',
 'Invisible intelligence - best features are ones users don''t think about (auto-tagging, relationship discovery)',
 '{"source_type":"session","source_id":"20251107-124740","domains":["ux","automation","design-principles"],"reliability":0.85}',
 '2025-11-15T20:12:00Z'),

('obs_014', 'pattern',
 'Emergent design over prescriptive - let system discover patterns rather than predefine them',
 '{"source_type":"session","source_id":"20251107-124740","domains":["design","emergence","complexity"],"reliability":0.90}',
 '2025-11-15T20:13:00Z'),

('obs_015', 'pattern',
 'Cross-domain learning - patterns from software (modularity, iteration) transfer to creative work (writing, games)',
 '{"source_type":"session","source_id":"20251107-124740","domains":["knowledge-transfer","cross-domain","universal-patterns"],"reliability":0.85}',
 '2025-11-15T20:14:00Z'),

('obs_016', 'technology',
 'Neuro-symbolic architecture - combine Scryer Prolog (logical validation) with vector embeddings (semantic search)',
 '{"source_type":"session","source_id":"20251107-124740","domains":["prolog","embeddings","neuro-symbolic","validation"],"reliability":1.0}',
 '2025-11-15T20:15:00Z'),

('obs_017', 'pattern',
 'Belief formation system over knowledge base - focus on forming validated beliefs from accumulated observations',
 '{"source_type":"session","source_id":"20251107-124740","domains":["beliefs","validation","knowledge-management"],"reliability":0.90}',
 '2025-11-15T20:16:00Z'),

('obs_018', 'decision',
 'Islands and gods model - projects are self-contained islands, persona observes all (read-only), knowledge flows up',
 '{"source_type":"session","source_id":"20251107-124740","domains":["architecture","multi-project","persona"],"reliability":0.85}',
 '2025-11-15T20:17:00Z'),

('obs_019', 'pattern',
 'Avoid command proliferation - bake intelligence into existing commands rather than creating new ones',
 '{"source_type":"session","source_id":"20251107-124740","domains":["cli-design","simplicity","ux"],"reliability":0.85}',
 '2025-11-15T20:18:00Z'),

('obs_020', 'technology',
 'Patina is universal knowledge system - works for any creative work (code, novels, research, games), not just development',
 '{"source_type":"session","source_id":"20251107-124740","domains":["vision","universal-design","knowledge-management"],"reliability":0.90}',
 '2025-11-15T20:19:00Z');

-- Verify count
SELECT COUNT(*) as observation_count FROM observations;
