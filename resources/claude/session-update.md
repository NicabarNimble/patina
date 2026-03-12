Update the current Patina session using the truthful Claude runtime surface:

1. Read root `AGENTS.md` first. Even if the `Claude Code` runtime section says Patina MCP is available, session lifecycle uses the native machine-readable CLI path for this phase.

2. Update the session with:
   - `patina ai session update --json`

3. Read the returned JSON and extract `artifact_path`.

4. Read the session artifact and find the new update section.

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

7. If the update shows a large or risky change set, suggest a small checkpoint commit before continuing.
