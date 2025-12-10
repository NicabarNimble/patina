---
id: reference-slash-commands
status: active
created: 2025-12-09
updated: 2025-12-09
oxidizer: nicabar
tags: [reference, commands, claude, session-management, git]
references: [architecture-patina-system]
---

# Patina Custom Slash Commands - Comprehensive Reference

## Overview

Patina provides **7 custom slash commands** that integrate with Claude Code to create a Git-aware session management and knowledge capture system. Each command consists of:

1. **Command Definition** (`.claude/commands/*.md`) - Instructions for the AI on how to execute the command
2. **Shell Script** (`.claude/bin/*.sh`) - The actual implementation that performs Git operations, file management, and database tracking

The same files exist in `resources/claude/` as **templates** that get copied to new projects during `patina init`.

---

## Command Architecture

```
User types: /session-start "My Feature"
     ↓
Claude reads: .claude/commands/session-start.md
     ↓
AI executes: .claude/bin/session-start.sh "My Feature"
     ↓
Script creates: .claude/context/active-session.md
     ↓
AI reads result and provides feedback
```

---

## File System Structure

### Active Files (Project-Specific)
```
.claude/
├── commands/           # Slash command definitions (AI instructions)
│   ├── session-start.md
│   ├── session-update.md
│   ├── session-note.md
│   ├── session-end.md
│   ├── launch.md
│   ├── persona-start.md
│   └── patina-review.md
├── bin/                # Shell script implementations
│   ├── session-start.sh
│   ├── session-update.sh
│   ├── session-note.sh
│   ├── session-end.sh
│   ├── launch.sh
│   └── persona-start.sh
└── context/            # Runtime state files
    ├── active-session.md       # Current session (deleted on end)
    ├── active-persona-session.md  # Current persona session
    ├── last-session.md         # Pointer to most recent session
    ├── .last-update            # Timestamp marker
    └── sessions/               # Archived sessions
```

### Template Files (For New Projects)
```
resources/claude/
├── *.md                # Command definitions (copied to .claude/commands/)
└── *.sh                # Script implementations (copied to .claude/bin/)
```

---

## Command 1: `/session-start [name]`

### Purpose
Initializes a new development session with Git tracking. Creates session boundaries using Git tags rather than branches.

### Command File: `.claude/commands/session-start.md`
**AI Instructions:**
1. Execute `.claude/bin/session-start.sh $ARGUMENTS`
2. Read `.claude/context/last-session.md` if exists
3. Read the full session file referenced (e.g., `layer/sessions/20250904-102821.md`)
4. Fill in "Previous Session Context" with substantive summary
5. Read newly created `.claude/context/active-session.md`
6. If prior conversation context exists, update Goals section
7. Ask user if they want todos created
8. Remind about `/session-update`, `/session-note`, `/session-end`

### Script: `.claude/bin/session-start.sh`

**What It Does:**

1. **Cleanup Check** (lines 6-20)
   - Detects if `active-session.md` already exists
   - If >10 lines: runs `session-end.sh --silent` to archive it
   - If trivial: deletes it

2. **Session ID Generation** (lines 22-25)
   ```bash
   SESSION_ID="$(date +%Y%m%d-%H%M%S)"  # e.g., 20251209-155355
   SESSION_TITLE="${1:-untitled}"
   SAFE_TITLE=$(echo "$SESSION_TITLE" | tr ' ' '-' | ...)
   ```

3. **Git Integration** (lines 27-70)
   - Gets current branch and HEAD commit
   - Creates session tag: `session-{ID}-start`
   - **Branch Logic:**
     - If on `work` branch → stays there
     - If on work sub-branch (descendant of work) → stays there
     - If on `main`/`master` → switches to `work` (creates if needed)
     - If on unrelated branch → warns but continues

4. **SQLite Tracking** (lines 72-93)
   - If `.patina/navigation.db` exists:
   - Inserts into `state_transitions` table with JSON metadata:
     ```json
     {
       "session_id": "20251209-155355",
       "title": "Commands Explained",
       "branch": "patina",
       "parent_commit": "bd6960d8..."
     }
     ```

5. **Session File Creation** (lines 95-123)
   - Creates `.claude/context/active-session.md` with:
     - Session metadata (ID, timestamp, branch, tag, commit)
     - `## Previous Session Context` placeholder
     - `## Goals` section with session title as first item
     - `## Activity Log` with session start entry

6. **Update Marker** (line 126)
   ```bash
   echo "$(date +"%H:%M")" > .claude/context/.last-update
   ```

**Output Example:**
```
⚠️  Warning: Uncommitted changes exist
📌 Staying on work sub-branch: patina
✅ Session tagged: session-20251209-155355-start
✓ Session started: Commands Explained
  ID: 20251209-155355
  Branch: patina
  Tag: session-20251209-155355-start

📍 Session Strategy:
- You're on 'patina' (work sub-branch)
- Session tagged as: session-20251209-155355-start
- Commit early and often
```

---

## Command 2: `/session-update`

### Purpose
Captures progress during a session with Git-aware context. Adds timestamped checkpoints to the session log.

### Command File: `.claude/commands/session-update.md`
**AI Instructions:**
1. Execute `.claude/bin/session-update.sh`
2. Note the time period shown (e.g., "14:15 → 14:45")
3. Note Git status (uncommitted changes, last commit time)
4. Read `active-session.md` and find new update section
5. Fill in: work completed, key decisions, challenges, patterns
6. If script suggests commit (30+ min or 100+ lines), consider checkpoint

### Script: `.claude/bin/session-update.sh`

**What It Does:**

1. **Validation** (lines 8-11)
   - Exits if no `active-session.md` exists

2. **Time Window** (lines 13-15)
   ```bash
   LAST_UPDATE=$(cat "$LAST_UPDATE_FILE" 2>/dev/null || echo "session start")
   CURRENT_TIME=$(date +"%H:%M")
   ```

3. **Git Status Analysis** (lines 24-86)
   - Shows current branch
   - Counts: modified files, staged files, untracked files
   - Calculates lines changed via `git diff --stat`
   - Shows last 5 commits with `git log --oneline -5 --decorate`
   - **Smart Reminders:**
     - If last commit >1 hour ago → strong warning
     - If >100 lines changed → suggests breaking into smaller commits
     - If >30 minutes since last commit → gentle reminder

4. **Session File Update** (lines 88-99)
   - Appends to `active-session.md`:
   ```markdown
   ### 15:30 - Update (covering since 15:08)

   **Git Activity:**
   - Commits this session: 2
   - Files changed: 17
   - Last commit: 48 seconds ago
   ```

5. **Session Health Indicator** (lines 105-115)
   ```
   Session Health: 🟢 Excellent (clean working tree)
   Session Health: 🟡 Good (commit recommended)
   ```

6. **Git Philosophy Reminder** (lines 127-136)
   - Random 1-in-3 chance to show commit philosophy reminder

---

## Command 3: `/session-note [text]`

### Purpose
Adds a timestamped note with Git context to the session log. Detects "breakthrough" keywords and suggests commits.

### Command File: `.claude/commands/session-note.md`
**AI Instructions:**
1. Execute `.claude/bin/session-note.sh "$ARGUMENTS"`
2. Confirm: "Note added [branch@sha]: [what user said]"
3. If note contains keywords (breakthrough, discovered, solved, fixed):
   - Script suggests checkpoint commit
   - Consider: `git commit -am "checkpoint: [discovery]"`
4. Purpose: Create searchable memory tied to specific code states

### Script: `.claude/bin/session-note.sh`

**What It Does:**

1. **Validation** (lines 8-16)
   - Checks for active session
   - Requires note text

2. **Git Context Extraction** (lines 18-36)
   ```bash
   CURRENT_BRANCH=$(git branch --show-current)
   CURRENT_SHA=$(git rev-parse --short HEAD)
   GIT_CONTEXT=" [${CURRENT_BRANCH}@${CURRENT_SHA}]"
   ```
   - **Keyword Detection:**
     - If note contains: `breakthrough`, `discovered`, `solved`, `fixed`, `important`
     - Shows: "💡 Important insight detected!"
     - Suggests: `git commit -am "checkpoint: $NOTE"`

3. **Note Addition** (lines 38-41)
   - Appends to `active-session.md`:
   ```markdown
   ### 15:45 - Note [patina@bd6960d]
   discovered dual session architecture is key
   ```

4. **Random Reminder** (lines 45-50)
   - 1-in-4 chance to remind about searchable memory

---

## Command 4: `/session-end`

### Purpose
Archives the session, classifies work type, creates end tag, and preserves session history.

### Command File: `.claude/commands/session-end.md`
**AI Instructions:**
1. First run `/session-update` to capture recent work
2. Execute `.claude/bin/session-end.sh`
3. Script will:
   - Check for uncommitted changes (warns but continues)
   - Classify work type based on commits
   - Archive to `layer/sessions/` and `.claude/context/sessions/`
   - Update `last-session.md` pointer
   - Create session end tag
4. After archiving, remind user of available commands:
   - `git log session-[timestamp]-start..session-[timestamp]-end`
   - `git cherry-pick session-[timestamp]-start..session-[timestamp]-end`

### Script: `.claude/bin/session-end.sh`

**What It Does:**

1. **Silent Mode** (lines 17-19)
   - `--silent` flag for automatic cleanup (no prompts)

2. **Metadata Extraction** (lines 26-37)
   - Parses `active-session.md` for:
     - SESSION_ID, SESSION_TITLE, SESSION_START
     - SESSION_TAG, GIT_BRANCH

3. **Session End Tag** (lines 39-45)
   ```bash
   SESSION_END_TAG="session-${SESSION_ID}-end"
   git tag -a "$SESSION_END_TAG" -m "Session end: ${SESSION_TITLE}"
   ```

4. **Session Metrics Calculation** (lines 48-51)
   ```bash
   FILES_CHANGED=$(git diff --name-only ${SESSION_TAG}..HEAD | wc -l)
   COMMITS_MADE=$(git log --oneline ${SESSION_TAG}..HEAD | wc -l)
   PATTERNS_TOUCHED=$(git diff --name-only ${SESSION_TAG}..HEAD | grep -E "layer/|\.md" | wc -l)
   ```

5. **Uncommitted Changes Warning** (lines 63-75)
   - If uncommitted files exist:
   - Shows warning with options
   - Waits for Enter or Ctrl+C

6. **Work Classification** (lines 85-100)
   | Condition | Classification | Icon |
   |-----------|---------------|------|
   | 0 commits | EXPLORATION | 🧪 |
   | Modified patterns/docs | PATTERN-WORK | 📚 |
   | >10 files changed | MAJOR-FEATURE | 🚀 |
   | <3 commits | EXPERIMENT | 🔬 |
   | Default | FEATURE | ✨ |

7. **SQLite Tracking** (lines 125-160)
   - Calculates session duration in minutes
   - Inserts into `state_transitions` with full metrics

8. **Archival** (lines 162-179)
   - Copies session to both locations:
     - `.claude/context/sessions/{ID}.md`
     - `layer/sessions/{ID}.md`
   - Creates `last-session.md` pointer:
   ```markdown
   # Last Session: Build
   See: layer/sessions/20251209-150810.md
   Tags: session-20251209-150810-start..session-20251209-150810-end
   Classification: pattern-work
   Quick start: /session-start "continue from Build"
   ```

9. **Cleanup** (lines 181-183)
   - Deletes `active-session.md`
   - Deletes `.last-update`

---

## Command 5: `/launch [type/]name`

### Purpose
Transitions from design/exploration session to implementation by creating a Git branch, IMPLEMENTATION_PLAN.md, and optionally a Draft PR.

### Command File: `.claude/commands/launch.md`
**AI Instructions:**
1. Extract implementation plan from current session
2. Create appropriate branch based on location:
   - `work` → `experiment/name` or `feature/name`
   - `work/something` → `work/something/name`
   - Other → suggests switching to work
3. Generate TODO from conversation markers
4. Create Draft PR with implementation plan
5. Continue session on new branch

**Extraction Markers (for AI to use in conversation):**
```markdown
## Implementation Tasks
- [ ] Add tree-sitter dependency

## Key Decisions
- Use DuckDB for storage

## Design
[High-level design description]

## Success Criteria
- Reduces tokens by 10x
```

### Script: `.claude/bin/launch.sh`

**What It Does:**

1. **Argument Parsing** (lines 21-62)
   - Accepts: `name`, `type/name`, `experiment/name`, `feature/name`
   - Auto-detects type from session content if not specified
   - Defaults to `feature` unless "experiment" found in session

2. **Branch Context Check** (lines 69-118)
   - If on `work` → creates `type/name`
   - If on work descendant → creates `current-branch/name`
   - If on unrelated branch → offers 4 options:
     1. Switch to work branch
     2. Create work branch here
     3. Create branch anyway
     4. Cancel

3. **Branch Creation** (lines 130-133)
   ```bash
   git checkout -b "$NEW_BRANCH"
   ```

4. **Implementation Plan Extraction** (lines 135-168)
   - Extracts from session file:
     - TODOs: grep for "implementation tasks|todo|next:"
     - DESIGN: grep for "solution:|design:|approach:"
     - DECISIONS: grep for "key decisions:|decisions:|decided:"
   - Uses placeholders if not found

5. **IMPLEMENTATION_PLAN.md Creation** (lines 170-201)
   ```markdown
   # Implementation: semantic-scraping
   **Parent Session**: 20251209-155355
   **Branch**: experiment/semantic-scraping
   **Type**: experiment
   **Created**: 2025-12-09T20:53:55Z

   ## Design
   [Extracted or placeholder]

   ## Key Decisions
   [Extracted or placeholder]

   ## Implementation Tasks
   [Extracted or placeholder]

   ## Success Criteria
   - [ ] All tests pass
   - [ ] Code follows project patterns
   - [ ] Documentation updated

   ## Test Plan
   - [ ] Unit tests for core functionality
   - [ ] Integration tests for command
   - [ ] Manual testing checklist
   ```

6. **Initial Commit** (lines 203-208)
   ```bash
   git add IMPLEMENTATION_PLAN.md
   git commit -m "$BRANCH_TYPE: initialize $BRANCH_NAME from session $SESSION_ID"
   ```

7. **Draft PR Creation** (lines 210-253)
   - If `gh` CLI available:
   - Creates Draft PR with:
     - Parent session link
     - Implementation plan link
     - TODO checklist
     - Success criteria
     - Test plan template
   - Base branch: `work` if exists, else `main`

8. **Session Update** (lines 255-262)
   - Appends launch note to active session

---

## Command 6: `/persona-start`

### Purpose
Starts an interactive belief discovery session. Uses SQLite databases, semantic search, and neuro-symbolic validation to extract and codify user beliefs from session history.

### Command File: `.claude/commands/persona-start.md`
**AI Instructions (8-step loop):**

1. **Domain Selection**
   ```sql
   SELECT category, COUNT(*) FROM patterns GROUP BY category ORDER BY count DESC LIMIT 1
   ```

2. **Gap Detection**
   - Find observations not yet codified as beliefs
   - Pick ONE to explore

3. **Evidence Search**
   ```bash
   patina query semantic "pattern description" --type pattern,decision --limit 10
   ```

4. **Generate ONE Question**
   - Atomic (yes/no)
   - Show evidence count
   - Ask if this is a pattern they follow

5. **Capture Answer**
   - Yes/No → codify directly
   - Conditional → ask refining follow-up

6. **Contradiction Detection**
   ```bash
   patina query semantic "opposite of current belief" --limit 5
   ```

7. **Validate and Codify Belief**
   ```bash
   patina belief validate "belief statement" --min-score 0.50 --limit 20
   ```
   - If `valid: true` → insert into beliefs table
   - If `valid: false` → ask clarifying question

8. **Repeat** until user says "save" or "stop"

**Strategic Questioning:**
- Find clusters of related observations
- Ask questions that update multiple beliefs at once
- Example: Security cluster → one question updates 8+ beliefs

### Script: `.claude/bin/persona-start.sh`

**What It Does:**

1. **Cleanup** (lines 6-18)
   - Archives or deletes existing persona session

2. **Database Check** (lines 24-29)
   - Requires `.patina/data/facts.db` to exist

3. **Session File Creation** (lines 33-216)
   - Creates `.claude/context/active-persona-session.md` with:
     - Available tools documentation
     - Semantic search examples
     - Validation workflow
     - Evidence linking requirements
     - Activity log

4. **Validation Thresholds (embedded in session file):**
   - Weighted Score ≥ 3.0 → adequate evidence
   - Weighted Score ≥ 5.0 → high confidence
   - Strong Evidence Count ≥ 2 → diverse support

5. **Evidence Linking Requirements:**
   ```sql
   -- Step 1: Insert belief
   INSERT INTO beliefs (statement, value, confidence, observation_count)
   VALUES ('belief_name', 1, 0.85, 2);

   -- Step 2: Link evidence (REQUIRED)
   INSERT INTO belief_observations (belief_id, session_id, observation_type, observation_id, validates)
   VALUES (last_insert_rowid(), '20251008-061520', 'pattern', 5, 1);
   ```

---

## Command 7: `/patina-review`

### Purpose
Reviews recent session history and Git activity for discussion.

### Command File: `.claude/commands/patina-review.md`
```markdown
Review recent layer/sessions and git history. Let's discuss.
```

### Script
**No shell script** - this command is purely AI-driven. The AI:
1. Reads files from `layer/sessions/`
2. Examines git history
3. Summarizes for discussion

---

## Data Flow Summary

```
                              ┌─────────────────────────────────────┐
                              │         Git Repository               │
                              │                                       │
   /session-start ──────────► │  Tags: session-{ID}-start           │
                              │  Branch: work (or sub-branch)        │
                              │                                       │
   /session-update ─────────► │  (no Git changes, reads status)     │
                              │                                       │
   /session-note ───────────► │  [branch@sha] context in notes      │
                              │                                       │
   /launch ─────────────────► │  Creates: type/name branch          │
                              │  Creates: IMPLEMENTATION_PLAN.md     │
                              │  Creates: Draft PR (if gh available) │
                              │                                       │
   /session-end ────────────► │  Tags: session-{ID}-end             │
                              │  Range preserved: start..end         │
                              └─────────────────────────────────────┘
                                              │
                                              ▼
                              ┌─────────────────────────────────────┐
                              │         File System                  │
                              │                                       │
                              │  .claude/context/                    │
                              │    ├── active-session.md (runtime)  │
                              │    ├── last-session.md (pointer)    │
                              │    ├── .last-update (timestamp)     │
                              │    └── sessions/ (archived)         │
                              │                                       │
                              │  layer/sessions/ (archived)          │
                              └─────────────────────────────────────┘
                                              │
                                              ▼
                              ┌─────────────────────────────────────┐
                              │         SQLite Databases             │
                              │                                       │
                              │  .patina/navigation.db               │
                              │    └── state_transitions table       │
                              │                                       │
                              │  .patina/data/facts.db               │
                              │    ├── sessions                      │
                              │    ├── patterns                      │
                              │    ├── beliefs                       │
                              │    └── belief_observations           │
                              └─────────────────────────────────────┘
```

---

## Session Workflow Example

```bash
# 1. Start session
/session-start "Add authentication"
# Creates: session-20251209-160000-start tag
# Creates: .claude/context/active-session.md
# Stays on: work branch (or sub-branch)

# 2. Work for 30 minutes...

# 3. Capture progress
/session-update
# Shows: Git status, uncommitted files, time window
# Appends: Update section to active-session.md

# 4. Important discovery
/session-note "JWT refresh tokens solve the session timeout issue"
# Appends: Note with [patina@abc123] context
# Suggests: checkpoint commit

# 5. Ready to implement
/launch feature/jwt-auth
# Creates: feature/jwt-auth branch
# Creates: IMPLEMENTATION_PLAN.md
# Creates: Draft PR (if gh available)
# Continues: session on new branch

# 6. More work...

# 7. End session
/session-end
# Creates: session-20251209-160000-end tag
# Classifies: 🚀 FEATURE (based on commits)
# Archives: layer/sessions/20251209-160000.md
# Updates: last-session.md pointer
# Cleanup: Deletes active-session.md
```

---

## Git Tag Archaeology

All sessions are preserved via paired tags:
```bash
# List all sessions
git tag | grep session

# View specific session work
git log session-20251209-155355-start..session-20251209-155355-end

# Diff a session
git diff session-20251209-155355-start..session-20251209-155355-end

# Cherry-pick session to another branch
git cherry-pick session-20251209-155355-start..session-20251209-155355-end

# Search for sessions by title
git log --grep="Authentication"
```

---

## Work Classification Matrix

| Classification | Commits | Files | Patterns | Description |
|---------------|---------|-------|----------|-------------|
| 🧪 EXPLORATION | 0 | any | any | Research/reading only |
| 📚 PATTERN-WORK | any | any | >0 | Modified layer/ or .md files |
| 🚀 MAJOR-FEATURE | any | >10 | 0 | Large scope changes |
| 🔬 EXPERIMENT | <3 | any | 0 | Quick experiments |
| ✨ FEATURE | ≥3 | ≤10 | 0 | Normal development |

---

## Command Summary

| Command | Script | Purpose |
|---------|--------|---------|
| `/session-start` | `session-start.sh` | Begin tracked session with Git tag |
| `/session-update` | `session-update.sh` | Capture progress with Git status |
| `/session-note` | `session-note.sh` | Add insight with Git context |
| `/session-end` | `session-end.sh` | Archive and classify session |
| `/launch` | `launch.sh` | Bridge design to implementation branch |
| `/persona-start` | `persona-start.sh` | Interactive belief discovery |
| `/patina-review` | *(none)* | AI-driven history review |
