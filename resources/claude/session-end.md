End the current Patina session with Git work classification:

1. First, run a final update to capture recent work:
   - Execute `/session-update` command
   - This captures activity since the last update
   - Ensure all artifact references use `[[wikilinks]]` (beliefs, sessions, commits, specs)

2. Then archive the session:
   `patina session end`

   This will:
   - Check for uncommitted changes (warns but doesn't block)
   - Classify work type based on commits (Exploration/Experiment/Feature)
   - Archive session to layer/sessions/<ID>.md
   - Update last-session.md pointer
   - Clean up active-session.md
   - Tag the session end point for preservation

3. The command will show:
   - "Session archived: <ID>.md"
   - Work classification
   - Session tags: session-[timestamp]-start..session-[timestamp]-end

4. After archiving, you can:
   - View session work: `git log session-[timestamp]-start..session-[timestamp]-end`
   - Cherry-pick commits: `git cherry-pick session-[timestamp]-start..session-[timestamp]-end`
   - Continue on current branch or switch as needed

5. **Linking convention** — before archiving, verify the activity log uses `[[wikilinks]]` for all artifact references:
   - Beliefs: `[[belief-id]]`, Sessions: `[[session-YYYYMMDD-HHMMSS]]`, Commits: `[[commit-SHA]]`
   - Specs: `[[spec-id]]` or relative path links, Source files: backtick paths
   - Unlinked plain-text mentions are invisible to `patina scrape` and the knowledge graph.

Note: All sessions are preserved via tags as searchable memory - failed experiments prevent future mistakes.
