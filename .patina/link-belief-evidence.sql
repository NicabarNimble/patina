-- Retroactive Evidence Linking Script
-- Links existing beliefs to supporting observations (patterns, technologies, decisions)
-- Run with: sqlite3 .patina/db/facts.db < .patina/link-belief-evidence.sql

BEGIN TRANSACTION;

-- Clear any existing links (in case of re-run)
DELETE FROM belief_observations;

-- ============================================================================
-- SECURITY BELIEFS → SECURITY PATTERNS
-- ============================================================================

-- Belief 3: never_commit_secrets_to_disk
-- Links to: tmpfs-for-secrets, credential-management, security-review-generated-code
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 3, p.session_id, 'pattern', p.id, 1
FROM patterns p
WHERE p.id IN (5, 7, 8);  -- tmpfs-for-secrets, credential-management, security-review

-- Belief 4: prefers_1password_for_credentials
-- Links to: 1password-integration pattern + 1password-over-bitwarden decision
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 4, p.session_id, 'pattern', p.id, 1
FROM patterns p WHERE p.id = 6  -- 1password-integration
UNION ALL
SELECT 4, d.session_id, 'decision', d.id, 1
FROM decisions d WHERE d.id = 3;  -- 1password-over-bitwarden

-- Beliefs 13-16: ai_code requires guidance/safeguards + review/checks
-- Links to: security-review-generated-code pattern
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT b.id, p.session_id, 'pattern', p.id, 1
FROM beliefs b, patterns p
WHERE b.id IN (13, 14, 15, 16) AND p.id = 8;  -- security-review-generated-code

-- ============================================================================
-- ARCHITECTURE BELIEFS → ARCHITECTURE PATTERNS
-- ============================================================================

-- Belief 1: prefers_unix_style_focused_tools
-- Links to: tool-vs-system-distinction pattern
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 1, p.session_id, 'pattern', p.id, 1
FROM patterns p WHERE p.id = 9;  -- tool-vs-system-distinction

-- Beliefs 6-7: pattern consistency across scales
-- Links to: tool-vs-system-distinction, pattern-selection-framework
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT b.id, p.session_id, 'pattern', p.id, 1
FROM beliefs b, patterns p
WHERE b.id IN (6, 7) AND p.id IN (9, 10)  -- tool-vs-system, pattern-selection-framework
UNION ALL
SELECT b.id, d.session_id, 'decision', d.id, 1
FROM beliefs b, decisions d
WHERE b.id IN (6, 7) AND d.id = 5;  -- pattern-polymorphism decision

-- Belief 8: values_stable_interfaces_replaceable_cores
-- Links to: architecture patterns (general architectural thinking)
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 8, p.session_id, 'pattern', p.id, 1
FROM patterns p WHERE p.category = 'architecture';

-- Belief 20: knowledge_should_be_queryable
-- Links to: neuro-symbolic-persona, domain-buckets patterns
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 20, p.session_id, 'pattern', p.id, 1
FROM patterns p WHERE p.id IN (1, 2, 11);  -- neuro-symbolic-persona, domain-buckets, git-aware-navigation

-- Beliefs 21-22: escape hatches
-- Links to: tool-vs-system-distinction, pattern-selection-framework
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT b.id, p.session_id, 'pattern', p.id, 1
FROM beliefs b, patterns p
WHERE b.id IN (21, 22) AND p.id IN (9, 10);

-- ============================================================================
-- WORKFLOW BELIEFS → WORKFLOW PATTERNS
-- ============================================================================

-- Belief 5: uses_patina_integration_branch
-- Links to: patina-branch-strategy pattern
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 5, p.session_id, 'pattern', p.id, 1
FROM patterns p WHERE p.id = 3;  -- patina-branch-strategy

-- Belief 12: prefers_small_focused_commits
-- Links to: patina-branch-strategy workflow pattern
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 12, p.session_id, 'pattern', p.id, 1
FROM patterns p WHERE p.id = 3;  -- patina-branch-strategy

-- ============================================================================
-- INFRASTRUCTURE BELIEFS → INFRASTRUCTURE PATTERNS
-- ============================================================================

-- Belief 10: prefers_public_repos_for_free_ci (FALSE)
-- Links to: public-repo-for-automation pattern + public-automation-repo decision
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 10, p.session_id, 'pattern', p.id, 1
FROM patterns p WHERE p.id = 4  -- public-repo-for-automation
UNION ALL
SELECT 10, d.session_id, 'decision', d.id, 1
FROM decisions d WHERE d.id = 2;  -- public-automation-repo

-- ============================================================================
-- META BELIEFS → DECISIONS
-- ============================================================================

-- Belief 2: structured_data_is_canonical_source
-- Links to: markdown-is-generated-output decision
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 2, d.session_id, 'decision', d.id, 1
FROM decisions d WHERE d.id = 1;  -- markdown-is-generated-output

-- Belief 11: decisions_grounded_in_evolving_beliefs
-- Links to: all decisions as meta-observation
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 11, d.session_id, 'decision', d.id, 1
FROM decisions d;

-- ============================================================================
-- TECHNOLOGY BELIEFS → TECHNOLOGY USAGE
-- ============================================================================

-- Belief 19: uses_docker_when_project_fits
-- Links to: technologies where purpose indicates conditional usage
-- (No direct evidence in current 7 sessions, linking to general infrastructure patterns)
INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
SELECT 19, p.session_id, 'pattern', p.id, 1
FROM patterns p WHERE p.category = 'infrastructure';

COMMIT;

-- Report results
SELECT
    'Evidence links created: ' || COUNT(*) as result
FROM belief_observations;

SELECT
    b.id,
    b.statement,
    b.observation_count as claimed,
    COUNT(bo.id) as actual_links,
    CASE
        WHEN COUNT(bo.id) >= b.observation_count THEN '✓'
        ELSE '⚠ needs more'
    END as status
FROM beliefs b
LEFT JOIN belief_observations bo ON b.id = bo.belief_id
GROUP BY b.id
ORDER BY b.id;
