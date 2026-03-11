Update the current Patina live session through MCP:

1. Call MCP tool `session.update`.
   - If multiple active sessions exist, call `session.list` first and retry with `session=<runtime_id|file_id>`.

2. Read the returned `artifact_path` and find the new update section.

3. Fill in that section with what happened since `since`:
   - Work completed
   - Key decisions and why
   - Challenges/debugging
   - Reusable patterns or lessons

4. Use the returned git metrics (`recent_commits`, `session_changed_files`, working tree fields) to anchor the summary in real changes.

5. If the update reveals spec movement, use `spec.show` or `spec.check` and record the relevant state truthfully.
