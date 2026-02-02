# Verification Query Schema Reference

Tables available for belief verification queries. All tables are populated by `patina scrape`.

## Code Analysis Tables

### function_facts
Functions extracted from source code (all supported languages).

| Column | Type | Description |
|--------|------|-------------|
| file | TEXT | Source file path |
| name | TEXT | Function name |
| is_async | BOOLEAN | Whether function is async |
| is_unsafe | BOOLEAN | Whether function is unsafe |
| is_public | BOOLEAN | Whether function is public |
| parameter_count | INTEGER | Number of parameters |
| returns_result | BOOLEAN | Returns Result type |
| returns_option | BOOLEAN | Returns Option type |

**PK:** (file, name)

### call_graph
Function call relationships.

| Column | Type | Description |
|--------|------|-------------|
| caller | TEXT | Calling function name |
| callee | TEXT | Called function name |
| file | TEXT | File containing the call |
| call_type | TEXT | Call type (default: 'direct') |
| line_number | INTEGER | Line of the call |

**PK:** (caller, callee, file, line_number)

### import_facts
Import/use statements.

| Column | Type | Description |
|--------|------|-------------|
| file | TEXT | Importing file path |
| import_path | TEXT | Imported module/path |
| imported_names | TEXT | Comma-separated imported names |
| import_kind | TEXT | Import kind |

**PK:** (file, import_path)

### code_search
Indexed code symbols for search.

| Column | Type | Description |
|--------|------|-------------|
| path | TEXT | File path |
| name | TEXT | Symbol name |
| kind | TEXT | Symbol kind |
| line | INTEGER | Line number |
| context | TEXT | Surrounding code context |

**PK:** (path, name, line)

## Git Tables

### commits
Git commit history.

| Column | Type | Description |
|--------|------|-------------|
| sha | TEXT | Commit SHA (PK) |
| message | TEXT | Commit message |
| author_name | TEXT | Author name |
| author_email | TEXT | Author email |
| timestamp | TEXT | ISO8601 timestamp |
| branch | TEXT | Branch name |

### commit_files
Files changed per commit.

| Column | Type | Description |
|--------|------|-------------|
| sha | TEXT | Commit SHA |
| file_path | TEXT | Changed file path |
| change_type | TEXT | A/M/D |
| lines_added | INTEGER | Lines added |
| lines_removed | INTEGER | Lines removed |

**PK:** (sha, file_path)

### co_changes
Files that change together frequently.

| Column | Type | Description |
|--------|------|-------------|
| file_a | TEXT | First file |
| file_b | TEXT | Second file |
| count | INTEGER | Co-change count |

**PK:** (file_a, file_b)

### git_tags
All git tags in the repository.

| Column | Type | Description |
|--------|------|-------------|
| tag_name | TEXT | Tag name (PK) |
| sha | TEXT | Tagged commit SHA |
| tag_date | TEXT | Tag date |
| tagger_name | TEXT | Tagger name |
| message | TEXT | Tag message |

### git_tracked_files
Files tracked by git (from `git ls-files`).

| Column | Type | Description |
|--------|------|-------------|
| file_path | TEXT | File path (PK) |
| status | TEXT | Tracking status (default: 'tracked') |

## Project Metadata Tables

### eventlog
Append-only event stream — all scraper activity.

| Column | Type | Description |
|--------|------|-------------|
| seq | INTEGER | Auto-increment PK |
| event_type | TEXT | e.g., 'git.commit', 'session.started', 'belief.surface' |
| timestamp | TEXT | ISO8601 when event occurred |
| source_id | TEXT | SHA, session_id, function name, etc. |
| source_file | TEXT | Original file path |
| data | TEXT | JSON payload |

**Event types:** belief.surface, code.call, code.function, code.import, git.commit, git.tag,
pattern.core, pattern.surface, scry.query, session.started, session.ended, session.decision,
session.goal, session.pattern, session.update, session.work

### beliefs
Belief metadata with computed metrics.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT | Belief ID (PK) |
| statement | TEXT | Belief statement |
| persona | TEXT | Epistemic agent |
| entrenchment | TEXT | low/medium/high/very-high |
| status | TEXT | active/scoped/defeated/archived |
| cited_by_beliefs | INTEGER | Cross-reference count |
| cited_by_sessions | INTEGER | Session reference count |
| verification_total | INTEGER | Total verification queries |
| verification_passed | INTEGER | Passing queries |
| verification_failed | INTEGER | Contested queries |

### patterns
Layer patterns (specs, core docs, surface docs).

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT | Pattern ID (PK) |
| title | TEXT | Pattern title |
| layer | TEXT | core/surface/dust |
| status | TEXT | active/complete/deferred/etc. |
| file_path | TEXT | File path |
| current_milestone | TEXT | Current milestone version |

### milestones
Spec milestones extracted from frontmatter.

| Column | Type | Description |
|--------|------|-------------|
| spec_id | TEXT | Parent spec ID |
| version | TEXT | Semver version |
| name | TEXT | Milestone name |
| status | TEXT | pending/complete |

## Common Query Patterns

```sql
-- Absence: verify something does NOT exist
SELECT COUNT(*) FROM function_facts WHERE is_async = 1
-- expect="= 0"

-- Existence: verify something DOES exist
SELECT COUNT(*) FROM function_facts WHERE name LIKE '%migration%'
-- expect=">= 1"

-- Architecture: verify cross-module usage
SELECT COUNT(DISTINCT file) FROM call_graph WHERE callee LIKE '%insert_event%'
-- expect=">= 5"

-- Git structure: verify file tracking
SELECT COUNT(*) FROM git_tracked_files WHERE file_path LIKE 'layer/core/%'
-- expect=">= 1"

-- Project state: verify no stale artifacts
SELECT COUNT(*) FROM patterns WHERE status = 'complete' AND file_path LIKE '%build/feat/%'
-- expect="= 0"

-- LIKE escaping: use ESCAPE '\' for literal _ or %
SELECT COUNT(*) FROM code_search WHERE context LIKE '%allow(dead\_code)%' ESCAPE '\'
-- expect="= 0"
```
