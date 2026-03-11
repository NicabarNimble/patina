Start a new Patina session using the truthful Claude runtime surface:

1. Read root `AGENTS.md` first and determine whether the `Claude Code` runtime section says Patina MCP is available in this runtime.

2. Start the session using exactly one truthful path:
   - If `AGENTS.md` says MCP is available for Claude Code, call MCP tool `session.start` with `title = $ARGUMENTS`.
   - Otherwise, execute `patina ai session start --json --adapter claude "$ARGUMENTS"`.

3. Read the returned JSON and extract `artifact_path`.

4. If `last_session_path` exists, read it. It points to the previous durable session in `layer/sessions/`; read that referenced file and fill the new session's "Previous Session Context" section with a concrete 2-3 sentence summary of what actually happened.

5. Update the new session artifact's Goals section with the real tasks, decisions, and constraints already established in this conversation.

6. Ask the user: "Would you like me to create todos for '$ARGUMENTS'?"

7. Remind the user about the workflow:
   - Use `/session-update` to capture progress
   - Use `/session-note` for important insights
   - Use `/session-end` to archive the session
   - Use `spec.next`, `spec.show`, and `spec.check` when spec workflow is relevant
