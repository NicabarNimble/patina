End the current Patina session using the truthful Claude runtime surface:

1. First, run a final update to capture recent work:
   - Execute `/session-update` command
   - This captures activity since the last update
   - Ensure all artifact references use `[[wikilinks]]` (beliefs, sessions, commits, specs)

2. Read root `AGENTS.md` first. Even if the `Claude Code` runtime section says Patina MCP is available, session lifecycle uses the native machine-readable CLI path for this phase.

3. Archive the session with:
   - `patina ai session end --json`

4. Read the returned JSON and confirm the archive artifact and end tag.

5. After archiving, you can:
   - View session work: `git log session-[timestamp]-start..session-[timestamp]-end`
   - Cherry-pick commits: `git cherry-pick session-[timestamp]-start..session-[timestamp]-end`
   - Continue on current branch or switch as needed

6. **Linking convention** — before archiving, verify the activity log uses `[[wikilinks]]` for all artifact references:
   - Beliefs: `[[belief-id]]`, Sessions: `[[session-YYYYMMDD-HHMMSS]]`, Commits: `[[commit-SHA]]`
   - Specs: `[[spec-id]]` or relative path links, Source files: backtick paths
   - Unlinked plain-text mentions are invisible to `patina scrape` and the knowledge graph.
