Start a new Patina development session using the following steps:

1. Run the session start script with the provided arguments:
   - Execute: `!.claude/bin/session-start.sh $ARGUMENTS`
   - The script will create a session file with git context and output its location

2. Analyze the current context:
   - Check if we're continuing from previous work (look for context in our conversation)
   - Note any problems or goals already discussed
   - Identify the current development focus

3. Read the created session file using the Read tool

4. Enhance the session file with:
   - Expanded goals based on our discussion (if we've already talked about what to do)
   - Context about why this session was started
   - Any relevant decisions or constraints from our conversation
   - Links to previous sessions if this is a continuation

5. If continuing work, add a section:
   ```
   ## Continuing From
   - Previous discussion: [summary of what we talked about]
   - Current focus: [what we're working on]
   - Open questions: [unresolved issues from earlier]
   ```

6. Confirm the session has started and remind the user:
   - Update progress with `/session-update` to capture decisions
   - End with `/session-end` to distill learnings
   - Updates will include both git changes AND conversation context

Note: Sessions now capture both code evolution and the reasoning behind it.