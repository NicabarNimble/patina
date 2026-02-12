---
type: belief
id: restructure-over-unsafe
persona: architect
facets: [rust, safety, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-12
revised: 2026-02-12
---

# restructure-over-unsafe

When the borrow checker blocks you, restructure data — don't reach for unsafe. Struct destructuring splits borrows, Mutex groups fields, and the compiler proves safety.

## Statement

When the borrow checker blocks you, restructure data — don't reach for unsafe. Struct destructuring splits borrows, Mutex groups fields, and the compiler proves safety.

## Evidence

- [[session-20260212-102737]]: F0 eliminated unsafe impl Sync for WasmChild by moving bindings::MotherChild behind Mutex with Store, then using destructuring (let WasmChildInner { store, instance } = &mut *inner) to split borrows. Zero performance cost, zero unsafe. (weight: 0.95)

## Supports

- [[compiler-enforced-safety]] — restructuring data to satisfy the borrow checker IS compiler enforcement; unsafe bypasses the compiler entirely
- [[world-boundary-is-type-safety]] — the WASM isolation boundary should have compiler-proven safety, not an unsafe escape hatch

## Attacks

- "Sometimes unsafe is the pragmatic choice" — when the cost of restructuring is zero (as in F0), pragmatism and safety align. Unsafe is justified only when restructuring would impose real costs (performance, API complexity) that outweigh the soundness risk.

## Attacked-By

- "Restructuring can increase complexity" — adding WasmChildInner added one struct, but the resulting code is simpler (no safety comment, no unsafe keyword, same lock pattern). Net complexity decreased.

## Applied-In

- `src/plugin/internal.rs` F0: WasmChild + WasmChildInner — instance moved behind Mutex with store, struct destructuring splits borrows in all 5 trait methods. Commit [[c1ede205]].
- Spec [[plugin-system-final-audit-fixes]] F0 section documents the before/after pattern in detail.

## Revision Log

- 2026-02-12: Created — metrics computed by `patina scrape`
