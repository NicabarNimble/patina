# Capstone Project

Full Allium cycle on real code. Runs from Session 5 to Session 6 (1–2
weeks), in the same pairs as Session 2.

## Brief

Pick a real module or feature in `patina` or `patina-mct` and take it
through the complete loop:

```text
distill/elicit  →  implement/reconcile  →  weed + propagate  →  review + contribution idea
```

## Requirements

1. **Spec** (distill if the code exists, elicit if it's new):
   - ≤ ~150 lines of Allium; scope block with honest `-- Excludes:`;
   - at least one contract with named invariants;
   - at least one surface with `provides:` (triggers must be reachable);
   - `allium check` and `allium analyse` clean.
2. **Implementation** — new code implementing the spec, or existing code
   reconciled against it (drift found by `/weed` either fixed in code or
   `/tend`-ed into the spec — a conscious decision each time, recorded as
   `-- Decision:` comments).
3. **Obligations** — `plan.json` committed next to the spec
   (regenerate, never hand-edit), and ≥3 obligations tagged in tests
   (`// obligation id: ...`).
4. **CI** — the spec lives under the repo's Allium layer and passes the
   repo's existing gates (`allium check layer/allium` in patina-mct;
   plan-drift check in patina if you add to `layer/allium/mother/`).
5. **Contribution idea** — one written proposal (Session 4 format: defect →
   triggering snippet → expected finding/change), refined from your
   Session 4 draft. Presented at the Session 6 review.

## Suggested scopes

- A `patina` CLI subcommand family without a spec today (slate commands
  are a good size).
- One MCT slice from `patina-mct/layer/slate/work/` lacking spec coverage —
  cross-check against `mct-product-map.allium` so your spec nests under the
  map instead of contradicting it.
- Promote a behavior section from `layer/core/*.md` (prose today) into a
  checked spec.
- The install/checksum behavior of `scripts/install-allium-ci.sh`
  (self-referential, satisfying).

Facilitator reviews scope **before** work starts — see the
[facilitator guide](../facilitator-guide.md) pass bar.

## Review (Session 6 or async)

- Live spec review: facilitator + one peer from another pair, hunting
  anti-patterns for 5 minutes (Session 3 list).
- Walk one obligation end-to-end: spec construct → plan ID → tagged test →
  CI.
- Contribution proposals presented; team votes on 1–2 to submit upstream.
