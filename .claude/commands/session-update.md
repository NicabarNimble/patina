Update the current Patina session with Git-aware progress tracking:

1. Launch an Agent (subagent) to run the session update backend. The agent should:
   - Run: `.claude/bin/session-update.sh`
     If multiple active sessions exist, use `.claude/bin/session-update.sh --session <id>`.
   - Parse the returned JSON and extract all fields
   - Read the session artifact at the returned `artifact_path`
   - Return to the main context: `artifact_path` and the full text of the session artifact

2. Using the returned artifact content, find the new update section. Note the time period to document.

3. **First-update naming hook (auto-session enrichment)**:
   - If this update is the first substantive update in the session (period indicates session start) and the title is still a default interface title (for example: `claude session`, `opencode session`, `gemini session`, `pi session`), propose a concise task-specific title from actual work completed.
   - Ask for confirmation: "Rename this session to '<proposed title>'?"
   - If approved, update the session artifact frontmatter `title:` before continuing.

4. Fill in the update section with what happened during that time period:
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

5. **Check for beliefs to capture**: Review the update and ask yourself:
   - Any design decisions made? ("We chose X because Y")
   - Any repeated patterns? (Said 3+ times)
   - Any strong principles? ("Never do X", "Always Y")
   - Any lessons learned? ("That was wrong because...")

   If yes, suggest to user: "This sounds like a belief worth capturing: '{statement}'. Should I create it?"

6. Include current git-range context in the update narrative using the start tag from JSON (`<start_tag>..HEAD`) so tag boundaries stay visible.

7. If the update shows a large or risky change set (30+ minutes of work or 100+ lines changed), suggest a checkpoint commit before continuing.
