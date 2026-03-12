End the current Patina session:

1. First, run a final update to capture recent work:
   - Execute `/session-update` command
   - This captures activity since the last update
   - Ensure all artifact references use `[[wikilinks]]` (beliefs, sessions, commits, specs)

2. Archive the session with the bundled wrapper:
   - `.claude/bin/session-end.sh`

3. Read the returned JSON and confirm the archive artifact and end tag.

4. After archiving, you can:
   - View session work: `git log session-[timestamp]-start..session-[timestamp]-end`
   - Cherry-pick commits: `git cherry-pick session-[timestamp]-start..session-[timestamp]-end`
   - Continue on current branch or switch as needed

5. **Linking convention** — before archiving, verify the activity log uses `[[wikilinks]]` for all artifact references:
   - Beliefs: `[[belief-id]]`, Sessions: `[[session-YYYYMMDD-HHMMSS]]`, Commits: `[[commit-SHA]]`
   - Specs: `[[spec-id]]` or relative path links, Source files: backtick paths
   - Unlinked plain-text mentions are invisible to `patina scrape` and the knowledge graph.
