Create an explicit new Patina session boundary with Git tagging:

1. Launch an Agent (subagent) to initialize the session and gather context. The agent should:
   - Run: `.claude/bin/session-new.sh "$ARGUMENTS"`
   - Parse the returned JSON and extract all fields
   - If `last_session_path` exists in the JSON, read that file, follow the session artifact reference inside it, and read the full previous session artifact
   - Return to the main context: all JSON fields (`session_id`, `artifact_path`, `start_tag`, `last_session_path`) and the full text of the previous session artifact if it exists

2. Read the session artifact at the returned `artifact_path` in `layer/sessions/`.

3. If the agent returned previous session content, fill in the "Previous Session Context" section with a substantive 2-3 sentence summary. Don't write generic fluff — include specific accomplishments, key fixes, and open items.

4. If we've been discussing work already in this conversation:
   - Update the Goals section with specific tasks we've identified
   - Add context about why this new boundary was started
   - Note any decisions or constraints we've discussed

5. Ask the user: "Would you like me to create todos for '$ARGUMENTS'?"

6. Remind the user about session workflow:
   - Use `/session-update` periodically to capture progress
   - Use `/session-note` for important insights
   - Use `/session-end` to archive, classify work, and tag the session range
   - Use `spec.next`, `spec.show`, and `spec.check` when spec workflow is relevant

The session is now tracking code changes and Git history. Mother owns the session lifecycle.