Add a human note to the current Patina session:

1. Run the session note script with your insight:
   - Execute: `!.claude/bin/session-note.sh "$ARGUMENTS"`
   - Example: `!.claude/bin/session-note.sh "discovered dual session architecture is key"`

2. Confirm the note was added:
   - Say: "Note added: [what user said]"

3. Purpose of notes:
   - Capture human insights and "aha moments"
   - Mark important decisions or discoveries
   - Add context that might be missed in updates
   - High-signal input for session distillation

4. Note types (optional):
   - Simple: "great pattern found"
   - Question: "why does X do Y?"
   - Decision: "we should keep sessions git-aware"
   - Discovery: "dual architecture solves the problem"

Note: These notes are prioritized during session-end distillation as they represent human judgment and insight.