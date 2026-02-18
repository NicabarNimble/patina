---
type: feat
id: spec-agent-checklists
status: draft
created: 2026-02-17
sessions:
  origin: 20260217-102300
related:
  - layer/surface/build/feat/spec-remote-ingestion/SPEC.md
  - layer/surface/build/feat/mother-design/SPEC.md
beliefs:
  - work-triages-specs
  - humans-hold-the-pen
  - bridges-become-permanent
---

# feat: Spec Checklists & Agent Guardrails — Human-in-the-Loop Execution

> Build the control layer on top of the remote spec graph. Every spec gains a
> normalized checklist, test manifest, and approval gate that both humans and
> agents must satisfy before status changes propagate.

## Motivation

Remote ingestion (see [[spec-remote-ingestion]]) makes spec metadata available
through Mother. To actually let agents help, we need structured checklists,
status gating, and cueing. Today:

- Specs embed ad-hoc “Exit Criteria” prose. Agents can't parse or update them.
- Anyone can run `patina spec status <id> complete`, even if tests never ran.
- There's no artifact record linking checklist items to logs/commits.

## Proposal

### Phase 1 — Checklist Schema & Template

1. Extend SPEC frontmatter with:
   ```yaml
   owner: your-handle
   reviewers:
     - partner-handle
   checklist:
     - id: docs-updated
       text: Update README + layer docs
       status: pending
     - id: tests-green
       text: Run cargo test --workspace
       command: cargo test --workspace
       status: pending
   ```
2. Update `patina::spec::serialize_spec_file` to preserve checklist order and
   emit defaults (`owner: todo`, empty reviewers) when missing.
3. Teach `patina spec sync` to parse `checklist[]`, `command`, and `status`,
   storing them in the spec cache & Mother tables.

### Phase 2 — Guarded Status Workflow

1. New subcommand `patina spec checklist <id> --set tests-green=passed --log path`
   updates a checklist row plus attaches an artifact reference (log path or gist).
2. Status transitions enforce:
   - `ready` requires owner + at least one reviewer field populated.
   - `active` requires owner acknowledgement (`--ack` flag or interactive prompt).
   - `complete` requires all checklist items `passed` and proof uploaded for any
     item with a `command` value.
3. Mother exposes `POST /specs/{id}/status` that enforces the same invariants.
4. CLI refuses to promote status if Mother reports staleness (e.g., an agent
   already updated checklist elsewhere) — human must resolve diff.

### Phase 3 — Cueing & MCP Hooks

1. Add a `specs.cues` stream to Mother: whenever a checklist item flips to
   `failed` or `blocked`, emit a cue referencing owner/reviewer.
2. MCP exposes two tools:
   - `specs.list_ready`: returns specs whose guardrails are satisfied.
   - `specs.update_checklist`: gated by capability token so only approved agents
     can mark items complete.
3. Document the human+agent workflow in `layer/core/spec-driven-design.md`:
   - Agents may draft checklists and mark test evidence, but humans must run
     `patina spec status` for final promotion.

### Rollback / Safety

- Environment flag `PATINA_SPEC_GUARDRAILS=0` bypasses the enforcement logic
  (restores current CLI) for emergency fixes.
- Checklist edits always write back to SPEC.md, so `git revert` restores prior
  state even if caches/graph diverge.

## Deliverables

1. Updated SPEC template + docs describing checklist syntax.
2. Database migrations for local cache + Mother to store checklist/test data.
3. CLI + MCP commands for listing, updating, and enforcing guardrails.
4. Integration tests: `cargo test spec_guardrails` covering promotion paths and
   rollback flag.
5. Demo session capturing human+agent collaboration on a spec end-to-end.
