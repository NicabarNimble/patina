End the current Patina session:

1. First, run a final update to capture recent work:
   - Execute `/session-update` command
   - This captures activity since the last update

2. Then archive the session:
   `.claude/bin/session-end.sh`
   
   This will:
   - Archive session to .claude/context/sessions/<ID>.md
   - Archive session to layer/sessions/<ID>.md  
   - Update last-session.md pointer
   - Clean up active-session.md

3. The script will show:
   - "✓ Session archived: <ID>.md"
   - Archive locations

4. After archiving, you can optionally:
   - Review the session for patterns worth capturing
   - Use `patina add <type> '<pattern>'` to save reusable insights
   - Check last-session.md for quick reference

Note: Sessions are archived as-is for later review. The "capture raw, distill later" approach keeps the workflow simple and preserves all context.