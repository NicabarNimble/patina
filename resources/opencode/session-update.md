Update the current Patina live session:

1. Execute the bundled session update wrapper:
   - `.opencode/bin/session-update.sh`

2. Read the returned JSON, then use `artifact_path` to find the new update section.

3. Fill in that section with what happened since `since`:
   - Work completed
   - Key decisions and why
   - Challenges/debugging
   - Reusable patterns or lessons

4. Use the returned git metrics (`recent_commits`, `session_changed_files`, working tree fields) to anchor the summary in real changes.

5. If the update reveals spec movement, use `spec.show` or `spec.check` and record the relevant state truthfully.
