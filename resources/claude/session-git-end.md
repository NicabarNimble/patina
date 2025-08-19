End the current Patina session with Git work classification:

1. First, run a final update to capture recent work:
   - Execute `/session-git-update` command
   - This captures activity since the last update

2. Then archive the session:
   `.claude/bin/session-git-end.sh`
   
   This will:
   - Check for uncommitted changes (warns but doesn't block)
   - Classify work type based on commits (Exploration/Experiment/Feature)
   - Archive session to .claude/context/sessions/<ID>.md
   - Archive session to layer/sessions/<ID>.md  
   - Update last-session.md pointer
   - Clean up active-session.md
   - Preserve the session branch (never deleted)

3. The script will show:
   - "✓ Session archived: <ID>.md"
   - Work classification (🧪 Exploration, 🔬 Experiment, or 🚀 Feature)
   - Branch preserved: session/[timestamp]-[name]

4. After archiving, you can:
   - Merge to main: `git checkout main && git merge [branch]`
   - Create PR: `gh pr create --base main --head [branch]`
   - Leave branch as permanent memory

Note: All session branches are preserved as searchable memory - failed experiments prevent future mistakes.