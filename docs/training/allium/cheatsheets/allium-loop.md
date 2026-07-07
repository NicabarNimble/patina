# The Allium Loop — One Page

```text
            intent                          code
              │                               │
              ▼                               ▼
         ┌─────────┐                    ┌──────────┐
         │ /elicit │  ── gather ──      │ /distill │
         │ forward │     context        │ backward │
         └────┬────┘                    └────┬─────┘
              │            spec              │
              └──────────►(.allium)◄─────────┘
                             │
                             ▼  take action
                       ┌────────────┐
                       │ /propagate │──► tests (plan.json + obligation-
                       │            │    tagged tests), implementation
                       └─────┬──────┘
                             │
                             ▼  verify
              ┌──────────────────────────────┐
              │ /weed        spec ↔ code     │
              │ allium check    structure    │
              │ allium analyse  semantics    │
              └─────────────┬────────────────┘
                            │
                            ▼  iterate
                       ┌─────────┐
                       │  /tend  │──► behavior changes hit the
                       └─────────┘    spec FIRST, then re-propagate
```

## Entry points — you don't always start at the top

| Situation | Enter at |
|-----------|----------|
| New feature idea | `/elicit` |
| Legacy code, no spec | `/distill` |
| Bug report / "code does X, should do Y" | `/weed` |
| Requirement changed | `/tend` |
| Not sure | `/allium` (routes for you) |

## The one rule

**Every probabilistic step gets a deterministic gate.** After any skill
touches a spec: `allium check` + `allium analyse`. After any code change
against a spec: `/weed` + tests. Green gates or it didn't happen.

## Team pipeline (patina resume-style)

1. Edit `.allium` → 2. `check` + `analyse` clean → 3. `allium plan` →
`plan.json` committed → 4. tests tagged `// obligation id: ...` →
5. CI re-generates plans and fails on drift.
