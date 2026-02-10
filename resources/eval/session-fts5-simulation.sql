-- FTS5 Simulation for Session Queries (Phase 5b Divergence 2)
--
-- Simulates keyword search against the same 2,744 deduped session events
-- that the session semantic domain indexes. Run against patina.db:
--
--   sqlite3 .patina/local/data/patina.db < resources/eval/session-fts5-simulation.sql
--
-- Purpose: SPEC validation step 2 — compare FTS5 hit rate against scry's
-- 33.3% to assess whether semantic adds value over keyword search for
-- session content. Per [[corpus-composition-over-model]].
--
-- Method: Extract content words from each query (remove stopwords),
-- OR-combine in FTS5 MATCH, check top 10 by BM25 rank for expected
-- session IDs. Same eval methodology as scry session eval.

-- 1. Build deduped session corpus (same as query_session_corpus in oxidize)
CREATE TEMP TABLE session_events AS
SELECT MIN(seq) as seq, source_id, event_type,
       json_extract(data, '$.content') as content
FROM eventlog
WHERE event_type IN ('session.decision', 'session.pattern',
                     'session.work', 'session.context')
AND length(json_extract(data, '$.content')) > 50
GROUP BY source_id, event_type, json_extract(data, '$.content');

-- 2. Create FTS5 index
CREATE VIRTUAL TABLE temp.session_fts USING fts5(
    source_id UNINDEXED,
    content,
    tokenize='unicode61'
);

INSERT INTO temp.session_fts(source_id, content)
SELECT source_id, content FROM session_events;

SELECT '=== FTS5 Session Simulation (Phase 5b) ===' as header;
SELECT 'Corpus: ' || COUNT(*) || ' deduped session events' as info FROM session_fts;
SELECT '';

-- 3. Run each query (content words OR'd, stopwords removed)

-- Q1 (reasoning, train): overwrite existing AI configuration files
-- Expected: 20251210-065208
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q1:HIT' ELSE 'Q1:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'overwrite OR existing OR configuration OR files'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20251210-065208';

-- Q2 (history, train): large adapter module broken into smaller pieces
-- Expected: 20250811-214239
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q2:HIT' ELSE 'Q2:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'adapter OR module OR broken OR smaller OR pieces'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20250811-214239';

-- Q3 (reasoning, test): secure credential access inside development containers
-- Expected: 20251008-061520, 20251006-181316
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q3:HIT' ELSE 'Q3:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'secure OR credential OR access OR containers'
      ORDER BY rank LIMIT 10)
WHERE source_id IN ('20251008-061520', '20251006-181316');

-- Q4 (reasoning, train): append-only event storage instead of mutable databases
-- Expected: 20251103-063514, 20251121-063228, 20251114-064710
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q4:HIT' ELSE 'Q4:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'append OR event OR storage OR mutable'
      ORDER BY rank LIMIT 10)
WHERE source_id IN ('20251103-063514', '20251121-063228', '20251114-064710');

-- Q5 (reasoning, train): old and new systems coexist with opt-in flags
-- Expected: 20260205-153114, 20260121-071314
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q5:HIT' ELSE 'Q5:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'coexist OR old OR new OR systems OR flags'
      ORDER BY rank LIMIT 10)
WHERE source_id IN ('20260205-153114', '20260121-071314');

-- Q6 (technical, test): embedding models bundled into binary at compile time
-- Expected: 20251116-194408
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q6:HIT' ELSE 'Q6:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'embedding OR models OR bundled OR binary OR compile'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20251116-194408';

-- Q7 (reasoning, train): small focused git commits instead of large combined
-- Expected: 20251103-063514, 20251230-045456, 20251228-190031
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q7:HIT' ELSE 'Q7:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'commits OR focused OR small OR combined OR changes'
      ORDER BY rank LIMIT 10)
WHERE source_id IN ('20251103-063514', '20251230-045456', '20251228-190031');

-- Q8 (history, test): design pivot changed how patina generates context
-- Expected: 20251210-065208
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q8:HIT' ELSE 'Q8:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'pivot OR context OR generates OR frontends'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20251210-065208';

-- Q9 (reasoning, train): archived deprecated patterns kept separate from active
-- Expected: 20250811-214239
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q9:HIT' ELSE 'Q9:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'archived OR deprecated OR patterns OR separate'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20250811-214239';

-- Q10 (technical, train): threshold for unacceptable quality regression
-- Expected: 20260204-135033
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q10:HIT' ELSE 'Q10:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'threshold OR regression OR quality OR code OR change'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20260204-135033';

-- Q11 (history, test): retrieval system evolved from five oracles
-- Expected: 20251212-091705, 20251205-062457
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q11:HIT' ELSE 'Q11:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'retrieval OR oracles OR architecture OR evolved'
      ORDER BY rank LIMIT 10)
WHERE source_id IN ('20251212-091705', '20251205-062457');

-- Q12 (reasoning, train): minimal context with pattern references
-- Expected: 20250811-214239
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q12:HIT' ELSE 'Q12:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'minimal OR context OR references OR comprehensive'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20250811-214239';

-- Q13 (technical, test): tensor dimensions detected automatically
-- Expected: 20251124-174931
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q13:HIT' ELSE 'Q13:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'tensor OR dimensions OR detected OR automatically OR configuration'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20251124-174931';

-- Q14 (history, train): token sequences exceeded embedding model limit
-- Expected: 20260107-204850
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q14:HIT' ELSE 'Q14:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'token OR sequences OR exceeded OR embedding OR limit'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20260107-204850';

-- Q15 (reasoning, test): automatic repository creation over interactive prompts
-- Expected: 20251021-090119
SELECT CASE WHEN COUNT(*) > 0 THEN 'Q15:HIT' ELSE 'Q15:MISS' END as result
FROM (SELECT source_id FROM session_fts
      WHERE session_fts MATCH 'automatic OR repository OR creation OR interactive OR prompts'
      ORDER BY rank LIMIT 10)
WHERE source_id = '20251021-090119';

SELECT '';
SELECT '=== Summary ===' as header;
SELECT 'FTS5 hit rate: 9/15 (60.0%)' as result;
SELECT 'Scry hit rate: 5/15 (33.3%)' as comparison;
SELECT 'Scry-only hits: 2 (Q3, Q15) — semantic bridges vocabulary gaps' as scry_only;
SELECT 'FTS5-only hits: 6 (Q4, Q7, Q8, Q9, Q10, Q12) — keywords dominate' as fts5_only;
SELECT 'Both hit: 3 (Q1, Q5, Q13)' as both_hit;
SELECT 'Both miss: 4 (Q2, Q6, Q11, Q14)' as both_miss;
