Mark something interesting and update the current Patina session:

1. Run the session update script with your observation:
   - Execute: `.claude/bin/session-update.sh "Your interesting observation"`
   - This captures your mark and creates a time span to fill

2. Confirm what was marked:
   - Say: "Noted: [observation]"

3. Immediately fill in the work context:
   - Navigate to the session file
   - Find the newly created time span (shows your mark in the header)
   - Fill in what work happened during that period
   - Include: files edited, features implemented, decisions made, problems solved

4. Example workflow:
   ```
   User: "/session-update Successfully implemented smart defaulting"
   Claude: Executes script → "Noted: Successfully implemented smart defaulting"
   Claude: Fills time span with actual work done:
           - Modified src/commands/init.rs to add determine_dev_environment()
           - Tested Docker fallback when Go not available
           - Added CI detection via environment variables
   ```

5. Keep the context factual:
   - What files were changed and why
   - What was tested or verified
   - Key decisions and their rationale
   - Problems encountered and solutions

Note: The mark captures what's interesting; the context captures what happened. Both are essential for pattern extraction at session end.