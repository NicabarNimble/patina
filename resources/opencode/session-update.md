Update the current Patina live session using the truthful Patina surface for this runtime:

1. Read root `AGENTS.md` first. Even if the `OpenCode` runtime section says Patina MCP is available, session lifecycle uses the native machine-readable CLI path for this phase.

2. Update the session with:
   - `patina ai session update --json`
   - If multiple active sessions exist, use `patina ai session list --json`, then retry with `--session <runtime_id|file_id>`.

3. Read the returned JSON, then use `artifact_path` to find the new update section.

4. Fill in that section with what happened since `since`:
   - Work completed
   - Key decisions and why
   - Challenges/debugging
   - Reusable patterns or lessons

5. Use the returned git metrics (`recent_commits`, `session_changed_files`, working tree fields) to anchor the summary in real changes.

6. If the update reveals spec movement, use `spec.show` or `spec.check` and record the relevant state truthfully.
