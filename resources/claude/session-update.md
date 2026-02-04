Update the current Patina session with Git-aware progress tracking:

1. Execute the session update command:
   `patina session update`

2. The command will show what time period to document (e.g., "14:15 → 14:45")

3. Note the Git status shown (uncommitted changes, last commit time)

4. Read `.patina/local/active-session.md` and find the new update section

5. Fill in the update section with what happened during that time period:
   - **Work completed**: Code written, files modified, problems solved
   - **Discussion context**: Key questions asked, reasoning frameworks used, why we chose this approach
   - **Key decisions**: Design choices, trade-offs, reasoning behind changes
   - **Challenges faced**: Errors encountered, debugging steps, solutions found
   - **Patterns observed**: Reusable insights, things that worked well

   **Linking convention** — use `[[wikilinks]]` for all artifact references so `patina scrape` can trace them:
   - Beliefs: `[[belief-id]]` (e.g., `[[sync-first]]`, `[[read-code-before-write]]`)
   - Sessions: `[[session-YYYYMMDD-HHMMSS]]` (e.g., `[[session-20260202-155143]]`)
   - Commits: `[[commit-SHA]]` (e.g., `[[commit-09e2abbf]]`)
   - Specs: `[[spec-id]]` or relative path link (e.g., `[SPEC.md](layer/surface/build/feat/epistemic-layer/SPEC.md)`)
   - Source files: backtick paths (e.g., `src/mcp/server.rs`)
   Unlinked plain-text mentions are invisible to the knowledge graph.

6. **Check for beliefs to capture**: Review the update and ask yourself:
   - Any design decisions made? ("We chose X because Y")
   - Any repeated patterns? (Said 3+ times)
   - Any strong principles? ("Never do X", "Always Y")
   - Any lessons learned? ("That was wrong because...")

   If yes, suggest to user: "This sounds like a belief worth capturing: '{statement}'. Should I create it?"

7. If the command suggests a commit (30+ minutes or 100+ lines changed), consider:
   - Creating a checkpoint: `git commit -am "checkpoint: [description]"`
   - Breaking large changes into smaller logical commits

Note: Each update creates a time-stamped checkpoint with Git context for future reference.
