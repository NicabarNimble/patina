Start a new Patina persona session for belief discovery and codification:

1. Execute the persona session start script:
   `.claude/bin/persona-start.sh`

2. Read the newly created `.claude/context/active-persona-session.md` file to understand:
   - Available databases and tools
   - The session flow
   - Your role as intelligent orchestrator

3. Begin the intelligent agent loop:

   **Step 1: Domain Selection**
   - Query SQLite to find most active domain:
     ```bash
     sqlite3 .patina/db/facts.db "SELECT category, COUNT(*) as count FROM patterns GROUP BY category ORDER BY count DESC LIMIT 1"
     ```
   - Announce: "I'm analyzing your sessions... Found 'X' domain with Y patterns."

   **Step 2: Gap Detection**
   - Find observations not yet codified as beliefs
   - Query patterns in that domain
   - Check which ones don't have corresponding beliefs
   - Pick ONE to explore

   **Step 3: Evidence Search**
   - Use semantic search to find related observations:
     ```bash
     patina query semantic "pattern description" --type pattern,decision --limit 10
     ```
   - Fall back to SQL for exact matches if needed:
     ```bash
     sqlite3 .patina/db/facts.db "SELECT s.id, s.started_at FROM sessions s JOIN patterns p ON s.id = p.session_id WHERE p.pattern_name = 'X'"
     ```
   - Semantic search finds evidence beyond keyword matching (e.g., "code audit" → "security review")
   - Results include similarity scores and evidence strength (strong/medium/weak)
   - Count occurrences, note context

   **Step 4: Generate ONE Question**
   - Make it atomic (yes/no)
   - Show evidence: "I see you used X in N sessions"
   - Ask: "Do you consider this a pattern you follow?"

   **Step 5: Capture Answer**
   - If "yes" or "no": codify directly
   - If conditional ("yes, but only when X"): Note the condition, ask refining follow-up

   **Step 6: Contradiction Detection**
   - Use semantic search to find potentially contradicting observations:
     ```bash
     patina query semantic "opposite of current belief" --limit 5
     ```
   - If found: ask clarifying question
   - Build exceptions into belief

   **Step 7: Validate and Codify Belief**
   - First, validate using neuro-symbolic reasoning:
     ```bash
     patina belief validate "belief statement from user answer" --min-score 0.50 --limit 20
     ```
   - Check validation result:
     - If `valid: true` → evidence is adequate, safe to codify
     - If `valid: false` with `reason: "weak_evidence"` → ask clarifying question or find more evidence
   - Use validation metrics to inform confidence:
     - `weighted_score >= 5.0` → high confidence (0.85-0.95)
     - `weighted_score >= 3.0` → moderate confidence (0.70-0.85)
     - Strong evidence count and source diversity boost confidence
   - Insert into beliefs table with evidence-based confidence:
     ```bash
     sqlite3 .patina/db/facts.db "INSERT INTO beliefs (statement, value, confidence, created_at) VALUES ('uses_pattern_X', 1, 0.85, datetime('now'))"
     ```
   - Show: "✓ Codified: [statement] (confidence: X.XX, weighted_score: Y.YY)"

   **Step 8: Repeat**
   - Find next gap
   - Generate next question
   - Continue until user says stop or saves

4. Update the "Beliefs Created" section as you go

5. Important principles:
   - ONE question at a time (natural dialogue, not a form)
   - Your answers shape next questions (adaptive)
   - Search ALL history for evidence (thorough)
   - **ALWAYS validate beliefs** using `patina belief validate` before codifying
   - Build exceptions when conditionals appear (precise)
   - Show your thinking: "[Searching...]", "[Validating...]", "[Found X sessions...]"
   - Trust the symbolic layer: if validation fails, gather more evidence or ask clarifying questions

6. **Strategic Questioning** (maximize information gain):

   Instead of asking about ONE observation at a time, find CLUSTERS of related observations and ask questions that update MULTIPLE beliefs.

   **Example workflow**:
   ```bash
   # Search for related observations
   patina query semantic "security practices" --limit 10
   ```

   **Analyze results** to find patterns:
   - Multiple observations about secret scanning, credential management, vault solutions
   - Different sources: session_distillation (reliability: 0.85), commit_message (reliability: 0.70)
   - Evidence strength varies: strong (similarity ≥ 0.70), medium (0.50-0.70), weak (< 0.50)

   **Identify the cluster**:
   - Observation #1: "security-review-generated-code" (session, 0.85 reliability)
   - Observation #2: "researched vault solutions" (session, 0.85 reliability)
   - Observation #3: "add pre-commit hook for secret scanning" (commit, 0.70 reliability)
   - Observation #4: "fix: remove .env from git history" (commit, 0.70 reliability)

   **Generate strategic question**:
   "I notice you invest heavily in preventing secrets in code (vault research, pre-commit hooks, cleaning git history). Do you apply this same rigor to other sensitive data like PII?"

   **Why this is strategic**:
   - If YES → Updates beliefs about security, compliance, data handling, tooling (8+ beliefs)
   - If NO → Creates conditional belief: "rigorous about secrets, not other sensitive data"
   - One question, multiple belief updates

   **When to use strategic vs linear questioning**:
   - Linear: Single observation, straightforward pattern
   - Strategic: 3+ related observations from different sources, cascading implications

7. End when user says:
   - "save" or "stop" → remind them to use `/persona-end`
   - Or naturally after exploring several beliefs

Remember: You ARE the intelligent agent. Use the database actively, reason about gaps, synthesize evidence, and guide discovery through dialogue. Use strategic questioning to maximize information gain when you find clusters of related observations.
