Manage the Patina spec lifecycle using the truthful product surface for this runtime.

1. Read root `AGENTS.md` first. If the `OpenCode` runtime section says Patina MCP is available, you may use `spec.*` for read-only lookup when helpful, but prefer the Patina CLI for spec work because CLI is the product truth.

2. Use the CLI spec surface intentionally:
   - Query state: `patina spec next`, `patina spec list`, `patina spec show <id>`, `patina spec check <id> --json`
   - Mutate state only after user confirmation: `patina spec create`, `patina spec promote`, `patina spec pause`, `patina spec resume`, `patina spec block`, `patina spec complete`, `patina spec abandon`, `patina spec split`

3. When you create a spec:
   - Read the generated `SPEC.md`
   - Fill in the body immediately from the conversation and code
   - If `DESIGN.md` exists, fill it in before implementation
   - Keep exit criteria concrete and checkable

4. Before marking work complete, run `patina spec check <id> --json` and make sure the exit criteria are actually true in the repo.

5. When spec work changes during a session, record the real state in the session artifact with `[[spec-id]]` links.
