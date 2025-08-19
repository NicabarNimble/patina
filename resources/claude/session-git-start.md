Start a new Patina development session with Git branch creation:

1. Execute the session start script:
   `.claude/bin/session-git-start.sh $ARGUMENTS`

2. Read `.claude/context/last-session.md` if it exists and use it to fill in the "Previous Session Context" section in the active session file. Provide a 2-3 sentence summary of what was accomplished and any open items.

3. Read the newly created `.claude/context/active-session.md` file

4. Note the Git branch created: `session/[timestamp]-[name]`

5. If we've been discussing work already in this conversation:
   - Update the Goals section with specific tasks we've identified
   - Add context about why this session was started
   - Note any decisions or constraints we've discussed

6. Ask the user: "Would you like me to create todos for '$ARGUMENTS'?"

7. Remind the user about session workflow:
   - Use `/session-git-update` periodically to capture progress
   - Use `/session-git-note` for important insights  
   - End with `/session-git-end` to archive, distill learnings, and handle branch cleanup

The session is now tracking both code changes and Git history.