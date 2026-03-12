# Design: Knowledge Child Platform Audit Fixes

## Why This Exists

The first `knowledge-child-platform` spec shipped the new runtime
successfully. This fix spec exists to tighten the architecture so the
Mother / Child / Toy model is enforced by the SDK and daemon shape, not
only by runtime validation.

The core idea is simple:

- Mother grants toys
- child receives toys
- ungranted toys do not exist in child code

That must become true in types.

## Design Decisions

### 1. Bundle injection before API polish

The highest-leverage change is to make the SDK expose granted toy
bundles. Without that, every other toy API improvement is cosmetic.

Target shape:

```rust
pub struct DuckLakeToys {
    pub fetch: FetchToy,
    pub lake: GrantedLakeToy,
    pub measure: MeasureToy,
    pub state: StateToy,
    pub checkpoint: CheckpointToy,
}

pub struct DuckLakeChild {
    toys: DuckLakeToys,
}
```

Not:

```rust
let host = GuestHost;
let fetch = FetchToy::<GuestHost>::new();
let lake = LakeToy::<GuestHost>::new();
let belief = BeliefToy::<GuestHost>::new();
```

### 2. Toy absence is part of authority

A denied toy should fail in two ways:

- it is absent from the granted bundle
- host/runtime still denies misuse if a child reaches lower-level calls

This gives defense in depth while making the child-facing authority model
truthful.

### 3. Legacy path must be visibly non-default

Leaving legacy `mother-child` in the default daemon path creates steady
architectural pressure toward "two real systems." Quarantine is enough
for this fix; full deletion can follow once migration confidence exists.

### 4. Scope belongs in the toy where it matters

The most important binding target is lake access. A child should receive
the lakes it was granted, not a generic lake client over all logical
names. That keeps the toy mental model concrete.

Fetch/query may remain runtime-validated if binding them in the toy type
adds too much ceremony, but the design should bias toward scoped toys
where the authority boundary is central to the app logic.

### 5. TaskIntent needs an explicit layer

If tasks remain child-facing, they should be treated as a dedicated toy.
If they remain substrate, the SDK/docs should say so directly. The
important thing is to prevent "task" from becoming an unprincipled
everything-backdoor.

## Smallest Safe Sequence

1. Change SDK/runtime construction so proof children receive granted toy
   bundles.
2. Add runtime tests that denied toys are absent/rejected.
3. Quarantine legacy daemon path.
4. Scope-bind lake toys.
5. Add DuckLake semantic proof tests.
6. Refine higher-level toy ergonomics only after the authority model is
   locked.

## Test Intent

Tests for this spec should prove architecture, not just mechanics:

- denied toy not present in bundle
- denied toy lower-level call still rejected
- default daemon path runs only knowledge children
- DuckLake owns workflow order while Mother owns execution safety/state
- DuckLake cursor semantics and partial success remain stable
