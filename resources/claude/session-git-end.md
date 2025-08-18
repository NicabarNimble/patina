# /session-git-end

Conclude session with Git work classification and branch preservation.

**Testing Version**: This command will eventually replace `/session-end` once validated.

## Usage

```
/session-git-end
```

## Description

Archives the current session with Git-aware classification and branch preservation. This command:

1. **Checks for uncommitted work** - Warns but doesn't block
2. **Classifies session type** - Exploration, Experiment, or Feature
3. **Preserves branch as memory** - Never deletes session branches
4. **Archives to multiple locations** - Creates searchable history

## Git Integration Features

- Analyzes commit count and patterns
- Classifies work type:
  - 🧪 **EXPLORATION**: No commits (just looking around)
  - 🔬 **EXPERIMENT**: 1-2 commits (trying something)
  - 🚀 **FEATURE**: 3+ commits (substantial work)
- Preserves all branches as permanent memory
- Provides future action options (merge, PR, archive)

## Work Classification

The command analyzes your Git activity to classify the session:

| Type | Commits | Meaning |
|------|---------|---------|
| Exploration | 0 | Learning and discovery |
| Experiment | 1-2 | Testing an approach |
| Feature | 3+ | Building something substantial |

## Branch Preservation

**All branches are preserved** - this is core to the memory philosophy:
- Failed experiments prevent future mistakes
- Successful features show what worked
- Explorations map the problem space
- Everything becomes searchable knowledge

## Philosophy

**Every Branch is Memory**: Unlike traditional Git workflows that delete merged branches, Patina preserves everything. Failed experiments are as valuable as successful features - they prevent repeating mistakes.

## Examples

Clean session ending:
```
✓ Session archived
✓ Git branch preserved: session/20250818-135843-auth
  Classification: 🚀 FEATURE (5 commits)
```

Uncommitted changes warning:
```
⚠️ Uncommitted changes detected!
   You have 3 uncommitted files
   Options:
   1. Commit: git commit -am "session-end: final checkpoint"
   2. Stash: git stash -m "session work"
```

## Future Actions

After ending a session, you can:
1. **Merge**: `git checkout main && git merge [branch]`
2. **Create PR**: `gh pr create --base main --head [branch]`
3. **Reclassify**: `git branch -m [branch] exp/[name]`
4. **Search**: `git log --grep="session-name"`

## Related Commands

- `/session-git-start` - Begin session with Git branch
- `/session-git-update` - Track progress with Git awareness
- `/session-git-note` - Capture insights with Git context