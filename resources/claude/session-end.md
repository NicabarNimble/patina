End the current Patina session with comprehensive distillation:

1. Run the session end script:
   - Execute: `.claude/bin/session-end.sh`
   - This will first run a final update, then add structure

2. Fill in ALL required sections in the session file:
   
   #### What We Did
   - Summarize key activities from the activity log
   - Include major files examined and changes made
   
   #### Key Insights  
   - Extract important discoveries from the session
   - Prioritize insights from user Notes
   - Include architectural patterns discovered
   
   #### Patterns Identified
   - List reusable patterns worth adding to brain
   - Be specific about pattern names and types
   - Example: "Rails-based session management (architecture pattern)"
   
   #### Next Session Should
   - Provide concrete next steps
   - Reference open questions or incomplete work
   - Guide the continuation of work

3. Verify your distillation:
   - [ ] Check all user Notes were addressed
   - [ ] Confirm activity log was summarized
   - [ ] Verify patterns are actionable
   - [ ] Ensure next steps are clear

4. Complete the process:
   - Say: "Session ended and distilled"
   - If patterns identified: "Found patterns: [X, Y]. Run `patina add <type> 'pattern'` to capture"
   - Archive happens automatically

IMPORTANT: Do not skip sections. Each serves a purpose for continuity and knowledge evolution.