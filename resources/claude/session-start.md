Start a new Patina development session with Git branch creation:

1. Execute the session start command:
   `patina session start "$ARGUMENTS"`

2. Read `.patina/local/last-session.md` if it exists. This file contains a reference to the full session file in `layer/sessions/`. You MUST read the full session file referenced there (e.g., if it says "See: layer/sessions/20250904-102821.md", read that file) to understand what actually happened. Then fill in the "Previous Session Context" section with a substantive 2-3 sentence summary of what was actually accomplished, key fixes/changes made, and any open items. Don't write generic fluff - include specific accomplishments.

3. Read the newly created `.patina/local/active-session.md` file

4. Note the session context printed to stdout:
   - Branch handling and session tag
   - Spec landscape: active, paused, blocked, and draft specs
   - Recommended next spec to work on
   - Previous session reference and beliefs

5. If we've been discussing work already in this conversation:
   - Update the Goals section with specific tasks we've identified
   - Add context about why this session was started
   - Note any decisions or constraints we've discussed

6. Ask the user: "Would you like me to create todos for '$ARGUMENTS'?"

7. Remind the user about session workflow:
   - Use `/session-update` periodically to capture progress
   - Use `/session-note` for important insights
   - End with `/session-end` to archive, distill learnings, and handle branch cleanup

The session is now tracking both code changes and Git history.
