---
type: feat
id: patina-sdk-rebuild
status: complete
created: 2026-04-09
sessions:
  origin: 20260409-070410-485377000
related:
- feat/pando-execution-mvp
- feat/voice-lake-mvp1
- refactor/child-typed-composition
- sdk/patina-sdk
beliefs:
- '[[children-have-agency-toys-are-capabilities]]'
- '[[wasi-is-foundation-not-option]]'
- '[[compiler-enforced-safety]]'
exit_criteria:
- id: cs1-crate-exists
  text: sdk/patina-sdk/ crate exists with patina:records types re-exported as Rust types. Children add patina-sdk as a dependency alongside wit_bindgen (which they keep for world declaration).
  checked: true
- id: cs2-toy-helpers
  text: 'Outside toy helpers exist: toys::log (info/warn/error), toys::keyvalue (open/get/set/exists with error mapping), toys::measure (counter/gauge), toys::config (get). No raw WIT binding calls needed in child code.'
  checked: true
- id: cs3-config-toy
  text: "Existing patina:config@0.1.0 toy (WIT + Mother impl from pando-execution-mvp) is wrapped in SDK as toys::config::get(key). No new WIT or host code — SDK surface only."
  checked: true
- id: cs4-template
  text: sdk/template/ exists. cargo generate produces a buildable push-pure child with correct wit/ structure, Cargo.toml (wasm32-wasip2), child.toml, and a skeleton process() function.
  checked: true
- id: cs5-children-migrated
  text: "All 6 children (file-system-monitor, content-extractor, schema-enforcer, dedup-filter, record-writer, lakehouse-catalog) add patina-sdk dependency and use toys::* helpers. Duplicated keyvalue_error_to_string removed from dedup-filter, record-writer, and lakehouse-catalog."
  checked: true
- id: cs6-legacy-documented
  text: 'patina-sdk-legacy README marks handle-based path as legacy. New patina-sdk README points developers to patina-sdk for new children.'
  checked: true
- id: cs7-decision-tree
  text: 'SDK README or AGENTS.md contains decision tree: child (patina-sdk) vs legacy service child (patina-sdk-legacy) vs grammar pipeline (patina-sdk-legacy pipeline feature).'
  checked: true
- id: cs8-proof
  text: cargo check --workspace passes. All 6 children build to wasm32-wasip2 using patina-sdk. cargo nextest run passes (existing tests + new SDK tests).
  checked: true
---
# feat: Patina SDK Rebuild

## Problem

The current `patina-sdk` crate serves legacy handle-based children (`Child` trait,
`register_child!`, `handle(string, string)`). This is the old model for service
children (belief-verifier, session-writer, spec-manager, doctor).

But Patina's child model is now push-pure (Fix 2): typed WIT interfaces,
no upstream imports, composed via pando adapters. The 6 children in
folder-text-to-parquet use `wit_bindgen::generate!` directly with no SDK support.

This means:
- Developers building new children have zero guidance or tooling
- Common patterns are duplicated across children (keyvalue error mapping, logging
  calls, measure emission)
- Each child independently manages its WIT deps directory structure
- There's no template for new children
- The existing SDK actively misleads — it only documents the legacy pattern

## Goal

`patina-sdk` becomes the child SDK. The current handle-based crate
is renamed to `patina-sdk-legacy` (internal use only, legacy children).

The new `patina-sdk` provides:

1. Shared type re-exports from `patina:records`
2. Ergonomic outside toy helpers (log, keyvalue, measure, config)
3. A `cargo generate` template for new children
4. SDK wrapper for the existing `patina:config` toy

`patina-sdk` is what you use to build a child. Period.

## Non-Goals

- Wrapping wit_bindgen — children still use `wit_bindgen::generate!` for their
  world. The SDK provides helpers alongside it, not instead of it.
- Adapter SDK — pando adapters are too thin (16 lines) to need an SDK.
  They import SDK types but don't need framework support.
- Replacing patina-sdk-legacy — the legacy crate stays for handle-based children.
- Auto-generating child.toml or adapters — future tooling, not this spec.

## What patina-sdk Provides

### 1. Type Re-exports

```rust
// patina-sdk re-exports patina:records types as Rust structs
pub use types::{
    RecordEnvelope, FileFound, TransformResult, 
    RejectedRecord, FileWritten, CatalogEntry,
};
```

**Type identity note:** Each child runs `wit_bindgen::generate!` against its own
WIT world, producing local types (e.g., `patina::records::types::RecordEnvelope`).
These WIT-local types are authoritative in `impl Guest` trait signatures — the
Rust compiler requires them. SDK re-exports exist for toy helper signatures
and standalone business logic functions. Since both the SDK crate and each child
generate from the same WIT source (`wit/toys/deps/patina-record.wit`), the SDK
should use `wit_bindgen::generate!` against the shared WIT and re-export those
types. Children that need to pass data between their local WIT types and SDK
helpers will use the same underlying WIT, keeping types compatible within a
single `wit_bindgen::generate!` call. The practical pattern: SDK provides toy
wrappers that accept/return WIT-generated types; children call those wrappers
with their local types which are identical because they share the same WIT source.

### 2. Outside Toy Helpers

```rust
pub mod toys {
    pub mod log {
        pub fn info(context: &str, message: &str);
        pub fn warn(context: &str, message: &str);
        pub fn error(context: &str, message: &str);
    }
    pub mod keyvalue {
        pub fn open(identifier: &str) -> Result<Bucket, String>;
        // Bucket wraps wasi:keyvalue with error mapping
        impl Bucket {
            pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String>;
            pub fn set(&self, key: &str, value: &[u8]) -> Result<(), String>;
            pub fn exists(&self, key: &str) -> Result<bool, String>;
        }
    }
    pub mod measure {
        pub fn counter(name: &str, delta: f64) -> Result<(), String>;
        pub fn gauge(name: &str, value: f64) -> Result<(), String>;
    }
    pub mod config {
        pub fn get(key: &str) -> Result<String, String>;
    }
}
```

This eliminates duplicated `keyvalue_error_to_string()` across children and
gives every child a clean, testable interface to outside toys.

### 3. Template

```
sdk/template/
├── cargo-generate.toml
├── Cargo.toml          # depends on patina-sdk, targets wasm32-wasip2
├── child.toml          # [child] name/kind/role, [needs].toys
├── wit/
│   ├── world.wit       # skeleton world importing outside toys + exporting one interface
│   └── deps/           # symlinked or copied patina:records types
└── src/
    └── lib.rs          # wit_bindgen::generate!, process() skeleton, export!()
```

`cargo generate --path sdk/template` scaffolds a buildable child.

### 4. Config Toy

`patina:config@0.1.0` already exists (landed in pando-execution-mvp):
- WIT: `wit/pando/deps/patina-config.wit`
- Mother host impl: `src/commands/mother/daemon.rs` composed_bindings

The SDK wraps the existing toy as `toys::config::get(key)`. No new WIT or
Mother code needed — this criterion is about SDK surface only.

## What a Child Looks Like With the SDK

```rust
use patina_sdk::toys;            // log, keyvalue, measure, config

wit_bindgen::generate!({
    path: "wit",
    world: "schema-enforcer",
    generate_all,
});

// WIT-local types are authoritative in trait impls:
use patina::records::types::{RecordEnvelope, TransformResult, RejectedRecord};

struct SchemaEnforcer;

impl exports::patina::records::transform::Guest for SchemaEnforcer {
    fn transform(records: Vec<RecordEnvelope>) -> Result<TransformResult, String> {
        toys::log::info("schema-enforcer", "validating batch");

        let mut accepted = Vec::new();
        let mut rejected = Vec::new();

        for record in records {
            match validate(&record) {
                Ok(()) => accepted.push(record),
                Err(reason) => rejected.push(RejectedRecord { reason, envelope: record }),
            }
        }

        toys::measure::counter("validated_records", accepted.len() as f64)?;
        Ok(TransformResult { accepted, rejected })
    }
}

fn validate(r: &RecordEnvelope) -> Result<(), String> {
    if r.record_id.is_empty() { return Err("missing record_id".into()); }
    if r.source_path.is_empty() { return Err("missing source_path".into()); }
    Ok(())
}

export!(SchemaEnforcer);
```

Clean. No raw WIT calls. No duplicated error mapping. Toys from SDK,
types from WIT-local generation (same WIT source guarantees compatibility).

## SDK Layout After

```
sdk/
├── patina-sdk/          # NEW: child SDK (push-pure)
│   ├── src/lib.rs       #   Type re-exports + prelude
│   ├── src/toys/        #   log, keyvalue, measure, config helpers
│   └── src/types.rs     #   Re-exported patina:records types
│
├── patina-sdk-legacy/   # RENAMED: legacy children + grammar pipelines (internal only)
│   ├── src/child.rs     #   Child trait, register_child!, handle(string,string)
│   ├── src/toys.rs      #   17 toy implementations for handle world
│   └── src/pipeline.rs  #   Grammar pipeline lane
│
├── template/            # NEW: child template
└── template-legacy/     # RENAMED: legacy template (internal)
```

Legacy children (belief-verifier, session-writer, spec-manager, doctor)
update their Cargo.toml path to `sdk/patina-sdk-legacy/` but keep `package`
alias so Rust code still imports `patina_sdk::*` without code changes:
```toml
patina-sdk = { package = "patina-sdk-legacy", path = "../../sdk/patina-sdk-legacy", features = ["child", ...] }
```

## Implementation Order

1. Create `sdk/patina-sdk/` crate with type re-exports from patina:records WIT
2. Add outside toy helpers (log, keyvalue, measure) wrapping raw bindings
3. Wrap existing patina:config toy as `toys::config` helper in SDK
4. Create `sdk/template/` with cargo-generate config
5. Migrate 6 children to use patina-sdk (remove duplicated helpers)
6. Update patina-sdk README — mark handle-based as legacy
7. Add decision tree to AGENTS.md or SDK README
8. Tests

## Resolved Decisions

- **Crate name**: `patina-sdk` takes the name. The old crate becomes `patina-sdk-legacy`.
- **Legacy crate stays**: Renamed, not removed. Legacy children and grammar pipelines update their dependency name.
- **wit_bindgen stays in children**: The SDK doesn't wrap or replace it.
  Children still declare their world via `wit_bindgen::generate!`.
  The SDK provides types and helpers alongside, not instead of.
- **Config toy already exists**: `patina:config@0.1.0` landed in pando-execution-mvp.
  SDK wraps it as `toys::config::get(key)`. No new WIT or Mother code.
- **Adapters don't use SDK**: Pando adapters are 16 lines of glue.
  They may import patina-sdk types but don't need framework support.

## Verification

```bash
cargo check --workspace -q
cargo nextest run

# All 6 children build with patina-sdk:
for child in file-system-monitor content-extractor schema-enforcer \
             dedup-filter record-writer lakehouse-catalog; do
  cargo build -p "patina-ai-child-${child}" --target wasm32-wasip2
done

# No duplicated helpers in children:
grep -rn "keyvalue_error_to_string" children/*/src/lib.rs
# ^ should return zero (moved to SDK)

# Template generates buildable child:
cd /tmp && cargo generate --path /path/to/sdk/template --name test-child
cd test-child && cargo build --target wasm32-wasip2
```

## Build Readiness

All prerequisites exist:
- patina:records WIT types defined
- 6 children as reference implementations
- Outside toy bindings working in daemon.rs (composed_bindings module)
- patina:config already implemented in Mother for pando execution
- Existing patina-sdk as structural reference
