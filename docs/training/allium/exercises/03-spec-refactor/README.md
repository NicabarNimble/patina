# Exercise 03 — Spec Refactor (Session 3)

`starter/child-approval.allium` is deliberately flawed. Refactor it until
`allium check` is completely clean **and** a partner can't find a remaining
anti-pattern in 5 minutes. A worked solution is in `solution/`.

## Procedure

1. Run the tool first, fix what it finds:

   ```bash
   allium check starter/child-approval.allium
   ```

2. Then hunt what the tool *cannot* find. This is the real lesson: the CLI
   verifies structure; meaning is on you (and `/weed`, and review).
3. Compare with `solution/child-approval.allium` — differences are worth
   discussing, not necessarily wrong.

## Facilitator answer key — the nine planted flaws

**Caught by `allium check` (3 of 9):**

| # | Flaw | Diagnostic |
|---|------|-----------|
| 1 | `RecordDeprecation` listens for `RegistrarDeprecatesChild`, but no surface provides it — so `deprecated` is an imagined state | `allium.rule.unreachableTrigger` (info) |
| 2 | `AuditLog` entity is dead weight (and storage-flavored) | `allium.entity.unused` (warning) + `allium.field.unused` |
| 3 | `Child.wasm_hash_hex` declared, never referenced — hash verification belongs to the package-install spec, not this one | `allium.field.unused` (info) |

**Not caught by the CLI (6 of 9) — human/agent territory:**

| # | Flaw | Anti-pattern |
|---|------|--------------|
| 4 | `child.stae = approved` in `BlockChild` — a typo the CLI accepts silently. Green check ≠ correct spec. | (tool limitation) |
| 5 | `RowInsertedIntoSqlite(table: "mother_child_registry_entries")` | Implementation leakage (storage) |
| 6 | `reason: "HTTP 403"` | Implementation leakage (transport) |
| 7 | `approve_button_label` field + its `exposes` line | UI in the spec |
| 8 | The `-- NOTE:` comment requiring email + dashboard badge with no rule behind the notification requirement | Prose smuggling |
| 9 | `ApproveChild` vs `ApproveChildStrict`: same `when` + same `requires`, contradictory `ensures` (approved vs blocked) | Contradiction by accretion |

## Debrief points

- Flaw 4 is the most important one in the exercise: **field references in
  `requires`/`ensures` are not name-checked** in the current CLI
  (validated against 3.5.0). This is exactly why the loop pairs the
  deterministic gate with `/weed` and human review — each net catches what
  the others miss.
- Flaw 9: the two candidate-decision outcomes belong to two *distinct
  triggers* (approve vs reject), each with its own rule — see the solution's
  `CandidateDecisionsArePartitioned` invariant. Nuance worth teaching:
  the analyser stays silent here because the rules share one trigger
  (assumed case analysis), but the *cross-trigger* version of this defect
  — two different triggers racing to set one field from the same state —
  **is** caught as a `conflict` finding. Slide deck Act III demonstrates
  both live.
- Flaw 8: the fix is not deleting the comment — it's promoting the
  *behavioral* part (operator notification) into an `ensures` and dropping
  the UI part (badge color).
