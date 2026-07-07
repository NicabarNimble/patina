# Session 3: Language Deep Dive (1.5–2 hours)

**Objective**: Read and write Allium v3 fluently and understand the design
trade-offs.

> **Full slide deck available**: this session is expanded into 36 slides
> with speaker notes, live-demo scripts, and a validated running example in
> [`../slides/session-03-slides.md`](../slides/session-03-slides.md)
> (running example: [`../slides/library.allium`](../slides/library.allium)).
> The outline below remains the facilitator's summary view.

## Agenda

| Time | Segment |
|------|---------|
| 0:00–0:40 | Syntax breakdown (v3), construct by construct |
| 0:40–1:00 | Semantics: what the CLI actually checks |
| 1:00–1:15 | Anti-patterns |
| 1:15–1:30 | Patterns library reading |
| 1:30–2:00 | Exercise: refactor a flawed spec |

## 1. Syntax breakdown (v3) — 40 min

Teach from real specs, one construct at a time. Reference examples are in
`patina/layer/allium/mother/` and
`patina-mct/layer/allium/mct-product-map.allium`.

### Entities, fields, enums, conditionals

```allium
entity MotherDaemon {
    lifecycle: stopped | starting | running | stopping | failed   -- inline enum
    pid_file_state: absent | present | stale
    control_plane_ready: Boolean
    last_failure_stage: String?                                   -- optional
    stop_deadline_at: Timestamp?

    -- derived fields: named predicates over other fields
    is_supervised: supervisor_backend = launchd_patina or supervisor_backend = systemd_user
}
```

- Field types: `String`, `Boolean`, `Integer`, `Timestamp`, `Duration`,
  `List<T>`, inline enums, other `value`/`entity` types, `?` for optional.
- Conditional fields (`when`) — a field that only exists in some states:

```allium
entity MctResult {
    outcome: success | denied | failed | timed_out | cancelled
    route_taken: RouteTaken? when outcome = success | failed | timed_out
}
```

  (`mct-product-map.allium` — a denied call has no route, and the *type
  system* says so.)

### Values vs entities

`value` = immutable data shape, no identity (`TraceContext`,
`PayloadMetadata`). `entity` = identity + state that rules act on
(`MctCall`, `MotherDaemon`). Compare `value CallerIdentity` with
`entity MctCall` in the product map.

### Rules (`when` / `requires` / `ensures`)

```allium
rule StopRunningDaemon {
    when: OperatorStopsMother(operator)              -- trigger event
    requires: daemon.lifecycle = running             -- guards (all must hold)
    ensures: daemon.lifecycle = stopping             -- state effects
    ensures: daemon.stop_deadline_at = now + config.stop_timeout
    ensures: MotherSigtermSent(operator: operator, daemon: daemon)  -- emitted event
}
```

- Multiple rules sharing a `when` = case analysis; `requires` clauses must
  partition the cases (see the three `OperatorStopsMother` stop rules).
- Set membership: `requires: daemon.lifecycle in {stopped, failed}`
  (`StatusWhenStoppedOrFailed`).
- Temporal trigger form: `when: _: MotherDaemon.stop_deadline_at <= now`
  (`StopTimesOut`) — how timeouts/expiry are expressed.
- **Triggers must come from somewhere.** Every event a rule listens for is
  either provided by a surface (next section) or emitted by another rule's
  `ensures`; otherwise `allium check` reports
  `allium.rule.unreachableTrigger`.

### Transitions and invariants

Transitions in v3 are the discipline of *enum state field + one rule per
edge* (as above). Invariants live in contracts:

```allium
contract MctResultTerminality {
    @invariant ResultIsTerminal
        -- An MctResult records the final outcome for an MctCall and is immutable once constructed.
    @invariant DeniedResultHasNoRouteTaken
        -- If the call is denied before execution, no route was taken.
}
```

Each `@invariant` has a stable name (it becomes an obligation ID) and a
prose meaning. Contracts are the part of the spec that reviewers and agents
quote back.

### Surfaces, actors, contracts

```allium
actor ResultConsumer {
    within: MctResult
    identified_by: result.call_id
}

surface MctResultReturn {
    facing consumer: ResultConsumer
    context result: MctResult
    exposes:
        result.call_id
        result.outcome
        result.requester_message
}
```

A `surface` declares **exactly what a given actor may see** — information
flow as spec, not as code review vigilance. The MCT product map uses this
heavily for caller-safe vs operator-facing projections
(`RouteDecisionInspection` vs `MctResultReturn` is the canonical pair:
operators see candidate elimination, callers see a safe message).

Surfaces also **provide triggers** — this is how events enter the system,
optionally guarded:

```allium
surface MotherLifecycleCLI {
    facing operator: Operator
    context daemon: MotherDaemon

    exposes:
        daemon.lifecycle

    provides:
        OperatorStartsMotherManually(operator)
            when daemon.lifecycle = stopped
        OperatorStopsMother(operator)
}
```

(`mother-lifecycle.allium` — note `MotherRuntimeSignals`, a second surface
facing the host environment that provides the *runtime* events like
`MotherStartupCompleted`. Operator actions and system signals enter through
different doors.)

### Config, temporal rules, exceptions/timeouts

```allium
config {
    stop_timeout: Duration = 5.seconds
}
```

Timeouts = config duration + deadline field + temporal rule
(`StopRunningDaemon` sets `stop_deadline_at`; `StopTimesOut` fires on it).
Failure paths are ordinary rules with failure events (`StartupFailed`
captures stage + message into fields).

### Spec-level scaffolding

- `-- allium: 3` version header, always first line.
- Scope block: `-- Scope:` / `-- Includes:` / `-- Excludes:` comments at the
  top. The *Excludes* list is as important as Includes — see the product
  map's explicit exclusion of "exact HTTP, SQLite, Wasmtime… APIs".
- `open question "..."` + `-- Decision:` comments — the spec as a decision
  log. Undecided things are *visible*, not silently absent.
- `given { daemon: MotherDaemon }` — the ambient instances rules refer to.

## 2. Semantics & analysis — what the CLI checks (20 min)

Run these live on `mother-lifecycle.allium` and show the JSON:

| Command | Checks |
|---------|--------|
| `allium check` | Structural diagnostics: malformed constructs, unused entities/fields (`allium.entity.unused`, `allium.field.unused`), rules listening for triggers nothing provides (`allium.rule.unreachableTrigger`) |
| `allium analyse` | Everything `check` does, plus process-level passes (per `allium help analyse`): data flow tracing, edge reachability, deadlock detection, conflict detection, invariant verification |
| `allium parse` | AST as JSON (Session 4 material) |
| `allium plan` | Test obligations (Session 5 material) |
| `allium model` | Domain model extraction for downstream tools |

**Honest calibration** (validated against CLI 3.5.0, live-demo it —
the slide deck's Act III walks each case):

- *Caught*: unused entities/fields, unreachable triggers, enum statuses
  with no exit (`allium.status.noExit`), and **cross-trigger conflicts** —
  two rules with different triggers both firing in one state and setting
  the same field to different values (`analyse` emits a `conflict` finding
  naming both rules, the state, and the values).
- *Silent*: a typo'd field name in a `requires`, and **same-trigger**
  rules with identical guards and contradictory `ensures` (assumed to be
  case analysis). The transition tracker also only credits status exits
  guarded by direct equality — `in {}` sets and derived-field guards
  don't register.

Teach the CLI as a fast structural gate, not a proof engine: `/weed` and
human review carry the semantic load (Session 2's break-it exercise makes
this visceral). Making one of the silent cases into a diagnostic is a
perfect first upstream contribution (Sessions 4/6).

**Why deliberately *not* full formal methods:** no quantifiers over
unbounded domains, no refinement proofs, no model checking of temporal
logic. The trade: analyses stay fast, findings stay explainable, and specs
stay writable/readable by LLMs and reviewers. Allium sits between "prose
that checks nothing" and "TLA+ that most teams won't write" — cheap
mechanical contradiction detection at the price of not proving liveness.

**Why not implementation details:** the moment a spec names a table or an
endpoint, it stops being durable (implementation churn forces spec churn)
and starts double-booking truth with the code.

## 3. Anti-patterns (15 min)

Each of these is planted in the
[refactor exercise](../exercises/03-spec-refactor/) — name them now, hunt
them next:

1. **Implementation leakage** — SQLite table names, HTTP status codes,
   function names in rules. Litmus test: "could we swap the storage engine
   without touching this line?"
2. **UI in the spec** — button labels, layout, wording. Surfaces expose
   *fields*, not widgets.
3. **Prose smuggling** — a comment that carries a behavioral requirement no
   rule enforces ("the operator should also be notified").
4. **God entity** — one entity accreting every field; missing `value` types.
5. **Enum states with no exit/entry rules** — usually a sign the state is
   imagined, not designed.
6. **Contradiction by accretion** — a new rule added without checking
   existing guards on the same `when`.
7. **Spec as backlog-dumping** — everything `open question`, nothing
   decided. Open questions are good; *only* open questions is avoidance.

## 4. Patterns library (15 min, reading)

Small-group reading, one exemplar each, report back in two sentences:

- **Authority record pattern** — `MctPeerBinding` (evidence ≠ authority ≠
  decision).
- **Two-phase decision pattern** — `TwoPhaseRouting` (filter then rank;
  ranking can't grant).
- **Safe-projection pattern** — `RouteDecisionPrivacy` (internal reasoning
  vs caller-safe message).
- **Terminal-result pattern** — `MctResultTerminality` (closed outcome set,
  immutability).
- **Lifecycle pattern** — `mother-lifecycle.allium` (supervisor-aware
  start/stop/restart).

## 5. Exercise: refactor a flawed spec (30 min)

**Exercise**: [Spec refactor](../exercises/03-spec-refactor/) — a
deliberately flawed `child-approval.allium` containing every anti-pattern
from §3. Fix it until `allium check` and `allium analyse` are clean *and*
a partner can't find a remaining anti-pattern. A worked solution is
provided.

## Resources

- Language reference (v3) — juxt.github.io/allium
- Patterns reference; parser docs in the allium-tools repo
- [Syntax cheat sheet](../cheatsheets/syntax.md)
- `patina-mct/layer/allium/mct-product-map.allium` — the largest real spec
  in the org; skim it start to finish once before Session 4
