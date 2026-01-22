---
type: refactor
id: skill-enforcement
status: design
created: 2026-01-22
sessions:
  origin: 20260122-102703
  work: []
related:
  - .claude/skills/epistemic-beliefs/SKILL.md
  - .claude/bin/session-update.sh
---

# refactor: Skill Enforcement

> Skills should be rails, not just guidance.

**Problem:** Skills are documented but not enforced. LLM can bypass them:
- Tried to create belief with `cat > file << 'EOF'` instead of using epistemic-beliefs skill
- Session-update accepted abstract summary without requiring raw context detail

**Solution:** Make skills more triggering and enforcing.

---

## Exit Criteria

- [ ] epistemic-beliefs skill is the ONLY path to create beliefs
- [ ] session-update enforces minimum detail level
- [ ] Skills trigger automatically when relevant patterns detected
- [ ] Cannot bypass skill system for covered operations

---

## Observed Issues

### 1. epistemic-beliefs bypass

**What happened:** LLM noticed a belief worth capturing, but instead of invoking the skill, tried to write the file directly with bash heredoc.

**Root cause:** Skill says "proactive" but there's no mechanism that forces routing through the script. LLM can still use Write/Bash tools directly.

**Evidence:** Session 20260122-102703 - user had to reject the tool use and ask "what broke?"

### 2. session-update accepts low detail

**What happened:** Update was written as abstract summary ("Deep state-of-union analysis: compared all active specs against code implementation") instead of raw context (which specs, what was wrong, what the actual discussion was).

**Root cause:** The update template has placeholders but no examples of good vs bad, no minimum requirements, no checklist.

**Evidence:** User feedback: "lines like this lose a ton of context.. our active sessions are the last line of defense on context"

---

## Possible Solutions

### For epistemic-beliefs

1. Add to skill prompt: "NEVER create belief files directly with Write/Bash - always use the script"
2. Add hook that detects belief file creation outside skill and warns
3. Make the skill more discoverable when belief-like patterns are detected

### For session-update

1. Add examples of good vs bad updates in the skill prompt
2. Add "raw context checklist":
   - File paths touched (actual paths)
   - Function/struct names modified
   - Error messages encountered (verbatim)
   - User questions (quoted)
   - Decision points (what options, why this choice)
3. Require minimum sections before accepting
4. Add word count or specificity check

### Meta: Skill as Rails

The deeper issue is that skills are opt-in guidance. Consider:
- Pre-commit hooks that validate skill compliance
- Tool use interceptors that route to skills
- Explicit "skill required" markers for certain operations

---

## Open Questions

- How much enforcement is possible given LLM tool access?
- Should skills block non-compliant operations or just warn?
- What's the right balance between guidance and rails?

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-22 | design | Spec created from session observation |
