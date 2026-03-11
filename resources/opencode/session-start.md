Start a new Patina session through MCP, not shell-output scraping:

1. Call MCP tool `session.start` with:
   - `title`: `$ARGUMENTS`

2. Read the returned `artifact_path`.

3. If `last_session_path` exists, read it. It points to the previous durable session in `layer/sessions/`; read that referenced file and fill the new session's "Previous Session Context" section with a concrete 2-3 sentence summary of what actually happened.

4. Update the new session artifact's Goals section with the real tasks, decisions, and constraints already established in this conversation.

5. Ask the user: "Would you like me to create todos for '$ARGUMENTS'?"

6. Remind the user about the workflow:
   - Use `/session-update` to capture progress
   - Use `/session-end` to archive the session
   - Use `spec.next`, `spec.show`, and `spec.check` when spec workflow is relevant
