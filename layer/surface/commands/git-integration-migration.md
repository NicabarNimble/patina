---
id: git-integration-migration
status: active
created: 2025-08-17
tags: [commands, git-integration, migration, testing-strategy]
references: [session-git-start-flow.md, session-git-update-flow.md, session-git-end-flow.md, session-git-note-flow.md]
---

# Git Integration Migration Strategy

## The Challenge
- Current `/session-*` commands work well and must not break
- LLMs need integrated git context to maintain memory across conversations
- Testing requires fresh Claude instance (loses current context)

## The Solution: Parallel Implementation  

### Phase 1: Create Testing Commands (CURRENT)
Create parallel commands with simple git integration:
- `/session-git-start` - Creates session branch
- `/session-git-update` - Shows git status, suggests commits
- `/session-git-end` - Preserves branch as memory
- `/session-git-note` - Adds notes with git SHA context

### Phase 2: Testing Protocol

#### Setup in Fresh Claude Instance
```bash
# 1. Init patina project
patina init test-project --llm=claude

# 2. Create the session-git-* scripts
cp resources/claude/session-start.sh resources/claude/session-git-start.sh
cp resources/claude/session-start.md resources/claude/session-git-start.md
# ... repeat for all four commands

# 3. Add git integration to session-git-* versions
# (Implementation from flow docs)

# 4. Register commands in .claude/commands/
```

#### Test Workflow
```bash
# Start fresh conversation
/session-git-start "test feature"
# Verify: Branch created, session started, LLM acknowledges

# Work naturally, make changes
# ...

/session-git-update
# Verify: Git status shown, appropriate reminders

/session-git-note "discovered optimization"
# Verify: Note includes git context

/session-git-end
# Verify: Classification works, cleanup options presented
```

### Phase 3: Validation Checklist

Before migration, ensure:
- [ ] Session functionality preserved (active-session.md works)
- [ ] Git branches created automatically
- [ ] LLM naturally follows git workflow
- [ ] No breaking changes to session archives
- [ ] Pattern extraction still works
- [ ] Clean working tree enforcement works
- [ ] Branch classification makes sense

### Phase 4: Migration

Once validated:

```bash
# 1. Backup original commands
mkdir -p resources/claude/backup
cp resources/claude/session-*.sh resources/claude/backup/
cp resources/claude/session-*.md resources/claude/backup/

# 2. Replace with integrated versions
mv resources/claude/session-git-start.sh resources/claude/session-start.sh
mv resources/claude/session-git-start.md resources/claude/session-start.md
# ... repeat for all four

# 3. Update adapter registration
# Ensure src/adapters/claude/mod.rs uses updated scripts

# 4. Test with existing projects
# Verify backward compatibility
```

## Implementation Notes

### Key Differences in Git-Integrated Version

#### session-git-start
```bash
# Simple additions:
- Creates session branch (with warning if dirty)
- Adds branch name to active-session.md
- Basic git reminder for LLM
```

#### session-git-update
```bash
# Simple additions:
- Shows git status
- Suggests commits based on time/changes
- No complex logic
```

#### session-git-end
```bash
# Simple additions:
- Ensures work is committed
- Preserves branch (never deletes)
- Archives session with git info
```

#### session-git-note
```bash
# Simple additions:
- Adds git SHA to note
- Shows current branch
- Suggests commit for important notes
```

## Risk Mitigation

### Rollback Plan
If git integration causes issues:
```bash
# Immediate rollback
cp resources/claude/backup/session-*.sh resources/claude/
cp resources/claude/backup/session-*.md resources/claude/
```

### Gradual Adoption Alternative
Instead of full replacement, could add flags:
```bash
# In .patina/config.toml
[session]
git_integration = false  # Set to true when ready
```

But this adds complexity and the LLM won't know which mode it's in.

## Success Criteria

The migration succeeds when:
1. **LLM Memory**: Git provides context between conversations
2. **Natural Flow**: LLM commits without being reminded constantly  
3. **No Breaks**: Existing projects continue working
4. **Better Outcomes**: Cleaner commit history emerges naturally
5. **Failed Experiments**: Preserved as valuable memory

## Testing Communication

When testing in fresh Claude instance, start with:
```
I'm testing new git-integrated session commands. 
Please use /session-git-start instead of /session-start.
Follow git best practices: commit early and often.
```

## Final Notes

The goal is **deep integration** where session and git are inseparable:
- Every session is a branch
- Every note has git context  
- Every update shows git state
- Every end classifies the work

This solves the core problem: **LLM memory persistence through git history**.