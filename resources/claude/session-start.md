Start a new Patina development session with Git branch creation:

1. Execute the session start script:
   `.claude/bin/session-start.sh $ARGUMENTS`

2. Read `.claude/context/last-session.md` if it exists. This file contains a reference to the full session file in `layer/sessions/`. You MUST read the full session file referenced there (e.g., if it says "See: layer/sessions/20250904-102821.md", read that file) to understand what actually happened. Then fill in the "Previous Session Context" section with a substantive 2-3 sentence summary of what was actually accomplished, key fixes/changes made, and any open items. Don't write generic fluff - include specific accomplishments.

3. Read the newly created `.claude/context/active-session.md` file

4. Note the session tracking:
   - If on work or work sub-branch: stays on current branch
   - If on main/master: switches to work branch
   - Session tagged as: `session-[timestamp]-start`

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