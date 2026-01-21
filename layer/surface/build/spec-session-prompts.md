# Spec: Session Prompt Capture

**Status:** Complete (Phase 1)
**Created:** 2026-01-10
**Completed:** 2026-01-21
**Origin:** Session 20260110-075703

**Implementation:** Commit `1df7ecce feat: add vocabulary gap bridging and session prompt capture`
- `session-start.sh` records Start Timestamp (Unix ms) - all adapters
- `session-end.sh` extracts prompts from `~/.claude/history.jsonl` - Claude only

---

## Problem

Session files capture work completed, decisions made, and patterns observed - but not the **user prompts** that drove the session. This loses valuable context:

1. **Conversation trajectory** - How did we get from A to B?
2. **User intent** - What was actually asked vs. what was done?
3. **Prompt patterns** - What phrasings work well for different tasks?
4. **Future replay** - Could we reproduce a session from prompts alone?

Example from this session:
```
1. /session-start keyring popup
2. "i have noticed that the mac popup isnt poping up..."
3. "ahhh so i think the issue is..."
4. "yes lets begin forge abstraction anchor in our layer/core values..."
5. /session-update
6. "git update with scalpel not shotgun..."
7. "is spec doc and build doc updated..."
8. "what are all the prompts i made up to this one..."
9. "we should add prompt capture to our session-end..."
```

This is 9 prompts that tell a story: debug → pivot → implement → commit → verify → meta-reflection.

---

## Discovery: Claude Code Already Logs Prompts

Claude Code maintains a prompt history at `~/.claude/history.jsonl`:

```json
{
  "display": "git update with scalpel not shotgun...",
  "pastedContents": {},
  "timestamp": 1768054919145,
  "project": "/Users/nicabar/Projects/Sandbox/AI/RUST/patina",
  "sessionId": "04364008-cbbd-47ad-a754-2bfe18cf0dbd"
}
```

**Fields:**
- `display`: The prompt text
- `timestamp`: Unix timestamp (ms)
- `project`: Absolute path to project
- `sessionId`: UUID for the Claude Code conversation

**Per-session directories** also exist at `~/.claude/session-env/<sessionId>/` (currently empty, may store env vars).

---

## Design Options

### Option A: LLM-Captured (Simple)

**How:** Add instruction to `session-end.md` skill telling the LLM to list all user prompts and add them to the session file.

**Pros:**
- Zero infrastructure - LLM already has full conversation context
- Works immediately
- Captures exact user text

**Cons:**
- Requires LLM to follow instruction (could be skipped)
- Context window limits may truncate very long sessions
- Prompts only captured at session-end (not incrementally)

### Option B: Read from history.jsonl (Recommended)

**How:** At session-end, read prompts from Claude Code's existing log file.

```bash
# Extract prompts for this session
grep "$SESSION_ID" ~/.claude/history.jsonl | \
  jq -r 'select(.project == "'$PROJECT_PATH'") | .display'
```

**Pros:**
- Zero new logging infrastructure - Claude Code already captures everything
- Persistent - survives context window limits
- Complete - every prompt, not just what LLM remembers
- Timestamps available for ordering

**Cons:**
- Need to determine sessionId at session-end time
- Depends on Claude Code's internal format (may change)
- Global file - need to filter by project

**How to get sessionId:**
1. **LLM provides it** - LLM can extract from conversation context
2. **Timestamp range** - Filter by `timestamp` between session-start and session-end
3. **Project + recency** - Most recent prompts for this project path

### Option C: Hook-Based (Redundant)

**How:** Use Claude Code `user-prompt-submit` hook to log each prompt.

**Status:** Likely unnecessary given history.jsonl discovery. Only needed if:
- history.jsonl format changes
- Need custom filtering/formatting at capture time
- Want project-local storage instead of global

### Option D: Hybrid (LLM + history.jsonl)

**How:** Use history.jsonl for reliable capture, LLM for intelligent formatting.

```bash
# session-end.sh extracts raw prompts
grep "$SESSION_ID" ~/.claude/history.jsonl | jq -r '.display' > /tmp/prompts.txt
```

Then LLM reads `/tmp/prompts.txt` and formats for session file.

**Pros:**
- Reliable capture from log
- LLM can add context, categorize, truncate long prompts

---

## Recommendation

**Option B** (Read from history.jsonl) with timestamp-based filtering:

1. At session-start, record Unix timestamp
2. At session-end, filter history.jsonl by:
   - `project` matches current project
   - `timestamp` between session-start and now
3. Extract `display` field as prompt list
4. Add to session file as "## User Prompts" section

**Implementation:**
1. Modify `session-start.sh` to record start timestamp
2. Modify `session-end.sh` to extract prompts from history.jsonl
3. Append prompts section to session file before archiving

---

## Format

Proposed format for captured prompts:

```markdown
## User Prompts

1. `/session-start keyring popup`
2. `i have noticed that the mac popup isnt poping up last few sessions...`
3. `ahhh so i think the issue is i have been doing exploration work...`
4. `yes lets begin forge abstraction anchor in our layer/core values...`
5. `/session-update`
6. `git update with scalpel not shotgun and make sure we commit along the way`
7. `is spec doc and build doc updated to show completed work`
8. `what are all the prompts i made up to this one... can you list`
9. `we should add prompt capture to our session-end custom slash command?`
```

**Rules:**
- Include slash commands (they're user intent)
- Truncate very long prompts with `...` (keep first ~100 chars)
- Exclude system messages and tool outputs
- Number sequentially

---

## Implementation

### Phase 1: Timestamp-based extraction

**session-start.sh changes:**
```bash
# Record start timestamp (Unix ms)
START_TS=$(date +%s000)
echo "start_timestamp: $START_TS" >> "$ACTIVE_SESSION"
```

**session-end.sh changes:**
```bash
# Extract start timestamp from session file
START_TS=$(grep "start_timestamp:" "$ACTIVE_SESSION" | cut -d' ' -f2)
PROJECT_PATH=$(pwd)

# Extract prompts from history.jsonl
jq -r --arg start "$START_TS" --arg proj "$PROJECT_PATH" \
  'select(.timestamp >= ($start | tonumber) and .project == $proj) | .display' \
  ~/.claude/history.jsonl > /tmp/session-prompts.txt

# Format as markdown
echo "" >> "$SESSION_FILE"
echo "## User Prompts" >> "$SESSION_FILE"
echo "" >> "$SESSION_FILE"
i=1
while IFS= read -r prompt; do
  # Truncate long prompts
  if [ ${#prompt} -gt 100 ]; then
    prompt="${prompt:0:100}..."
  fi
  echo "$i. \`$prompt\`" >> "$SESSION_FILE"
  i=$((i+1))
done < /tmp/session-prompts.txt
```

### Phase 2: Add to scry index

Once prompts are in session files, they'll be indexed by `patina scrape sessions` and searchable via scry.

---

## Success Criteria

1. Session files include user prompts section
2. Prompts capture conversation trajectory
3. Searchable via `grep` or scry
4. Doesn't significantly slow down session-end

---

## Future Extensions

- **Prompt categorization**: question, instruction, feedback, meta
- **Prompt-to-outcome linking**: which prompt led to which commit?
- **Cross-session patterns**: what prompts work well for debugging vs. implementation?
- **Prompt templates**: discover reusable phrasings
- **sessionId correlation**: Store Claude Code sessionId in patina session for exact matching

---

## References

- Session 20260110-075703: Origin of this idea
- `~/.claude/history.jsonl`: Claude Code prompt log (discovered this session)
- `~/.claude/session-env/`: Per-session directories
- `.claude/commands/session-end.md`: Current session-end skill
