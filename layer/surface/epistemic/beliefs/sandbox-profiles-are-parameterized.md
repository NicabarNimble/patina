---
type: belief
id: sandbox-profiles-are-parameterized
persona: architect
facets: [architecture, security, sandbox, children, role-boundary]
entrenchment: medium
status: defeated
endorsed: true
extracted: 2026-03-08
revised: 2026-03-25
---

# sandbox-profiles-are-parameterized

Sandbox profiles are parameterized by child type — each child gets the minimum privilege its role requires. Connectors get deny-all filesystem + allowed network. Storage children get scoped filesystem access to a Mother-provided path + deny-all network. The sandbox is scoped, not weakened.

## Statement

Sandbox profiles are parameterized by child type — each child gets the minimum privilege its role requires. Connectors get deny-all filesystem + allowed network. Storage children get scoped filesystem access to a Mother-provided path + deny-all network. The sandbox is scoped, not weakened.

## Evidence

- [[session-20260308-184638]]: Resolved P1-a conflict — pipe-architecture §8.3 denied all filesystem for native children, but lakehouse child must write Parquet. Evaluated 3 options against 5-question role alignment test. Mother-mediated I/O failed Q5 (Mother executing data-plane). Scoped path allowlist passed all 5. Mother configures OS sandbox at spawn time from child.toml type + destination config. (weight: 0.9)
- [[session-20260305-224446]]: Original exploration session traced security model to host-side code, designed OS sandbox model. The deny-all profile was correct for connectors but implicitly assumed all children are connectors. (weight: 0.7)
- [[session-20260319-071818-503477000]]: Native child infrastructure was removed as dead code in `spec-native-child-removal`, invalidating parameterized native sandbox profiles as active child-runtime doctrine. (weight: 1.0)

## Supports

- [[pipes-are-processes-not-wasm]] — native children need OS sandboxing. This belief defines the sandbox parameterization that makes different child types viable under native transport.
- [[connectors-never-materialize]] — connectors stay deny-all filesystem because they never write storage. The sandbox enforces the role boundary at the OS level.
- [[mother-owns-destination-format]] — Mother configures the sandbox at spawn time (governance), OS enforces (execution). Mother decides what path a child can access, not the child.

## Attacks

- One-size-fits-all sandbox — the original pipe-architecture §8.3 design that denied all filesystem for all native children. This made lakehouse children impossible without role-smearing (Mother writing Parquet).
- Sandbox-off-for-storage — the temptation to just use `--no-sandbox` for lakehouse children. Defeats the security model rather than parameterizing it.

## Attacked-By

- "Parameterized sandboxes are harder to audit" — more profiles means more surface area to review. Counter: two well-defined profiles (deny-all vs scoped) are still simple. The alternative (no sandbox for storage children) is worse.
- "Scoped paths can be too broad" — if the lake root contains sensitive data from other personas. Counter: Mother controls the path and can scope it to persona-specific subdirectories.

## Applied-In

- [[spec-pipe-architecture]] — DESIGN.md §8.3 now defines two sandbox profiles: connector (deny-all fs) and storage child (scoped fs).
- [[spec-raw-lake-ingestion]] — DESIGN.md §Sandbox Profile specifies how Mother configures the lakehouse child sandbox with the lake root path.
- `crates/patina-pipe/src/sandbox.rs` — existing sandbox code accepts parameters (`_allowed_domains`); will gain `allowed_paths` parameter for filesystem scoping.

## Revision Log

- 2026-03-08: Created — metrics computed by `patina scrape`
- 2026-03-25: Defeated — native child sandbox profile strategy retired with native child lane removal.
