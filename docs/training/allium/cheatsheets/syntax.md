# Allium v3 Syntax Cheat Sheet

One page. Examples adapted from `layer/allium/mother/*.allium` and
`patina-mct/layer/allium/mct-product-map.allium`.

## File scaffolding

```allium
-- allium: 3                       ← version header, always line 1
-- my-spec.allium

-- Scope: one sentence
-- Includes:
--   - what this spec covers
-- Excludes:
--   - what deliberately lives elsewhere

config {
    grace_period: Duration = 5.seconds
}

given {
    daemon: MotherDaemon           ← ambient instances rules refer to
}
```

## Entities & values

```allium
value TraceContext {               ← immutable data shape, no identity
    trace_id: String
    span_id: String
}

entity MotherDaemon {              ← identity + state, rules act on it
    lifecycle: stopped | starting | running | stopping | failed
    control_plane_ready: Boolean
    last_failure_message: String?              ← optional field
    stop_deadline_at: Timestamp?
    tags: List<String>

    is_supervised: backend = launchd or backend = systemd   ← derived field
}
```

Conditional field (exists only in some states):

```allium
route_taken: RouteTaken? when outcome = success | failed | timed_out
```

Types: `String` `Boolean` `Integer` `Timestamp` `Duration` `List<T>`
inline enums (`a | b | c`), other value/entity types, `?` optional.

## Rules

```allium
rule StopRunningDaemon {
    when: OperatorStopsMother(operator)           ← trigger event
    requires: daemon.lifecycle = running          ← guards (all must hold)
    ensures: daemon.lifecycle = stopping          ← state effect
    ensures: daemon.stop_deadline_at = now + config.grace_period
    ensures: MotherSigtermSent(operator: operator)   ← emitted event
}
```

- Case analysis = several rules sharing one `when`, with `requires` clauses
  partitioning the cases.
- Temporal trigger (timeout/expiry):
  `when: _: MotherDaemon.stop_deadline_at <= now`
- Negation: `requires: not daemon.manual_start_blocked`
- Set membership: `requires: daemon.lifecycle in {stopped, failed}`
- Every trigger a rule listens for must be **provided** by a surface or
  emitted by another rule's `ensures`, or check reports
  `allium.rule.unreachableTrigger`.
- Guard **state-changing** rules with direct equality (`status = x`):
  the transition tracker doesn't credit `in {}` or derived-field guards,
  and an uncredited state exit triggers `allium.status.noExit`.
- Temporal rules that could race a state field should **observe a fact
  instead** (emit an event, don't set the field) — see `StopTimesOut`.

## Surfaces (who may see what, and where events come from)

```allium
surface MotherLifecycleCLI {
    facing operator: Operator            ← an entity, or a declared actor
    context daemon: MotherDaemon

    exposes:                             ← ONLY these fields are visible
        operator.name
        daemon.lifecycle

    provides:                            ← how events enter the system
        OperatorStartsMotherManually(operator)
            when daemon.lifecycle = stopped      ← optional guard
        OperatorStopsMother(operator)
}
```

Sensitive reasoning gets its own operator-facing surface (see
`RouteDecisionInspection` vs `MctResultReturn` in the MCT product map).
Named actors are declared separately when identity matters:

```allium
actor ResultConsumer {
    within: MctResult
    identified_by: result.call_id
}
```

## Contracts & invariants

```allium
contract MctResultTerminality {
    @invariant ResultIsTerminal
        -- An MctResult records the final outcome and is immutable once constructed.

    @invariant DeniedResultHasNoRouteTaken
        -- If the call is denied before execution, no route was taken.
}
```

`@invariant` names are stable IDs — they become obligations.

## Decisions & open questions

```allium
open question "What routing inputs are mandatory vs advisory?"
-- Decision: Vision is the primary data-sharing boundary.
-- Principle: Fastest path wins only among authorized paths.
```

Undecided things stay visible; decisions are logged next to them.

## Top-level declarations (full v3 list, from the parser)

`entity` `rule` `enum` `value` `config` `surface` `actor` `given`
`default` `variant` `deferred` `use` `open question` `contract` `invariant`

## Smells (see Session 3 anti-patterns)

- Table names / status codes / function names in rules → implementation leak
- Button labels in surfaces → UI leak
- Behavioral requirement in a comment with no rule → prose smuggling
- Enum state no rule enters or leaves → imagined state
- Same `when`+`requires`, contradictory `ensures` → conflict (analyse finds it)
