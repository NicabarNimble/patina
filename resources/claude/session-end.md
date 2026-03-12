End the current Patina session with Git work classification:

1. First, run a final update to capture recent work:
   - Execute `/session-update` command
   - This captures activity since the last update
   - Ensure all artifact references use `[[wikilinks]]` (beliefs, sessions, commits, specs)

2. Archive the session with the bundled wrapper:
   `.claude/bin/session-end.sh`
   If multiple active sessions exist, use `.claude/bin/session-end.sh --session <id>`.
   To include a final outcome sentence: `.claude/bin/session-end.sh --note "what we accomplished"`.

3. Read the returned JSON and confirm the archive artifact and end tag.
   - Work classification: Exploration / Experiment / Feature based on commit patterns
   - Session tags: `session-[timestamp]-[interface]-start..session-[timestamp]-[interface]-end`

4. After archiving, you can:
   - View session work: `git log session-[timestamp]-[interface]-start..session-[timestamp]-[interface]-end`
   - Cherry-pick commits: `git cherry-pick session-[timestamp]-[interface]-start..session-[timestamp]-[interface]-end`
   - Continue on current branch or switch as needed

5. **Linking convention** — before archiving, verify the activity log uses `[[wikilinks]]` for all artifact references:
   - Beliefs: `[[belief-id]]`, Sessions: `[[session-YYYYMMDD-HHMMSS]]`, Commits: `[[commit-SHA]]`
   - Specs: `[[spec-id]]` or relative path links, Source files: backtick paths
   - Unlinked plain-text mentions are invisible to `patina scrape` and the knowledge graph.

All sessions are preserved via tags as searchable memory — failed experiments prevent future mistakes.
