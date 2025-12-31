End the current Patina session with Git work classification:

1. First, run a final update to capture recent work:
   - Execute `/session-update` command
   - This captures activity since the last update

2. Then archive the session:
   `.opencode/bin/session-end.sh`
   
   This will:
   - Check for uncommitted changes (warns but doesn't block)
   - Classify work type based on commits (Exploration/Experiment/Feature)
   - Archive session to .opencode/context/sessions/<ID>.md
   - Archive session to layer/sessions/<ID>.md  
   - Update last-session.md pointer
   - Clean up active-session.md
   - Tag the session end point for preservation

3. The script will show:
   - "✓ Session archived: <ID>.md"
   - Work classification (🧪 Exploration, 🔬 Experiment, or 🚀 Feature)
   - Session tags: session-[timestamp]-start..session-[timestamp]-end

4. After archiving, you can:
   - View session work: `git log session-[timestamp]-start..session-[timestamp]-end`
   - Cherry-pick commits: `git cherry-pick session-[timestamp]-start..session-[timestamp]-end`
   - Continue on current branch or switch as needed

Note: All sessions are preserved via tags as searchable memory - failed experiments prevent future mistakes.