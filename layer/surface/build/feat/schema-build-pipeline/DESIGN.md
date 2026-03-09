# DESIGN: Schema Build Pipeline

## Current State

```
wit/schema/github/           ← canonical source (tracked in git)
  schema.toml                  schema metadata, projections, indexes, contracts
  github.wit                   WIT type definitions

.patina/schemas/github/      ← installed runtime copy (gitignored, per-project)
  schema.toml                  copied from canonical by `patina schema install`
  github.wit                   copied from canonical

children/github-connector/   ← child binary directory
  child.toml                   manifest — declares [schemas.github] package ref
  src/                         connector source code
  (schema.toml DELETED)        was a hand-maintained duplicate, removed in ebdc7bcb
```

### What works today

- `patina schema install wit/schema/github` → copies to `.patina/schemas/github/`
- `patina schema list` / `patina schema show github` → reads installed schemas
- `load_all_installed()` → used by projection engine, FTS, oxidize, search, doctor
- `validate_fact()` in `schema/internal.rs:1124` → WIT-based field validation (exists, `#[allow(dead_code)]`)
- `validate_fact()` in `broker/routing.rs:34` → checks schema name in manifest, NOT fact_type
- Pre-push checks: WIT consistency, formatting, clippy, tests, broker integration

### What's missing

1. Broker doesn't validate `fact_type` against installed schema `facts[].event_type`
2. No CI check that canonical schema installs cleanly or matches installed copy
3. No single command for validate → install → generate
4. `src/generated/schemas/` is dead code (forge-only, no imports)

## Phase 4 (first): Delete dead generated schemas

### Problem

`src/generated/schemas/` contains forge-only generated types that nothing imports.
Confirmed dead: `rg 'mod generated|use crate::generated' src/` returns zero.

### Design

Delete `src/generated/schemas/` (4 files: `mod.rs`, `forge.rs`, `embeddings.rs`,
`forge_migration.sql`). If `schema generate --types` is used in the future, it
regenerates from installed schemas.

### Files

- `src/generated/schemas/` — delete directory

## Phase 1: Broker fact-type validation

### Problem

`routing.rs:50-57` checks that `fact.schema` is declared in `child.toml`, but
doesn't check that `fact.fact_type` maps to a real `facts[].event_type` in the
installed schema. A child emitting `github.typo` passes silently.

### Design

Add Step 3 to `validate_fact()` in `routing.rs`. The project root is available —
`write_to_project()` receives `project_root: &Path`. Thread it through to
`validate_fact()`.

```rust
// routing.rs — validate_fact() signature change
pub fn validate_fact(
    fact: &BrokerFact,
    manifest: &ChildManifest,
    child_name: &str,
    project_root: &Path,           // NEW
    warned_schemas: &mut HashSet<String>,
) -> Result<ValidatedFact>
```

Step 3 implementation:

```rust
// Step 3: validate fact_type against installed schema
let event_type = format!("{}.{}", fact.schema, fact.fact_type);
let schemas_dir = patina::paths::project::schemas_dir(project_root);
let schema_dir = schemas_dir.join(&fact.schema);
if schema_dir.join("schema.toml").exists() {
    if let Ok(metadata) = crate::commands::schema::load_schema_metadata(&schema_dir) {
        let valid = metadata.facts.iter().any(|f| f.event_type == event_type);
        if !valid {
            bail!(
                "child '{}': fact_type '{}' not declared in schema '{}' — fact dropped",
                child_name, event_type, fact.schema
            );
        }
    }
}
```

### Schema loading

Use the existing `SchemaMetadata` deserialization (`pub(crate)`). Expose a thin
`load_schema_metadata(dir: &Path) -> Result<SchemaMetadata>` from
`commands::schema` — just re-exports the existing `parse_schema_toml()`. No new
parsing code.

### Performance

`parse_schema_toml()` is called per-fact currently. To avoid re-parsing on every
fact, cache parsed event types. The `warned_schemas` `HashSet` is already threaded
through for this purpose (currently unused, prefixed `_`). Replace it with:

```rust
// In broker/mod.rs, at the call site:
let mut schema_event_types: HashMap<String, HashSet<String>> = HashMap::new();

// On first fact for a schema, populate:
if !schema_event_types.contains_key(&fact.schema) {
    // load + cache event types
}
```

This avoids changing the `validate_fact` signature further — the cache lives at
the call site in `write_to_project()`.

### Call site change

```rust
// broker/mod.rs write_to_project() line 106-107
let mut schema_event_types: HashMap<String, HashSet<String>> = HashMap::new();

let fetch_result = child.fetch(&fetch_params, &mut |fact| {
    // Cache-check fact_type before full validation
    let event_type = format!("{}.{}", fact.schema, fact.fact_type);
    if let Some(valid_types) = schema_event_types.get(&fact.schema) {
        if !valid_types.contains(&event_type) {
            eprintln!("[broker] {}: fact_type '{}' not in schema — dropped", child_name, event_type);
            return Ok(());
        }
    } else {
        // First fact for this schema — load and cache
        let schemas_dir = patina::paths::project::schemas_dir(project_root);
        let schema_dir = schemas_dir.join(&fact.schema);
        if let Ok(metadata) = crate::commands::schema::load_schema_metadata(&schema_dir) {
            let types: HashSet<String> = metadata.facts.iter()
                .map(|f| f.event_type.clone())
                .collect();
            let valid = types.contains(&event_type);
            schema_event_types.insert(fact.schema.clone(), types);
            if !valid {
                eprintln!("[broker] {}: fact_type '{}' not in schema — dropped", child_name, event_type);
                return Ok(());
            }
        }
        // If schema not installed, pass through (graceful degradation)
    }

    match validate_fact(&fact, manifest, &child_name, &mut warned_schemas) {
        // ... existing code
    }
});
```

This approach:
- Keeps `validate_fact()` signature unchanged (no project_root param)
- Caches per-schema, not per-fact
- Gracefully degrades if schema not installed
- Drops invalid facts with a log message, doesn't abort the fetch

### Files changed

- `src/commands/schema/mod.rs` — expose `load_schema_metadata()`
- `src/broker/mod.rs` — add fact_type cache and validation in `write_to_project()`

### Tests

Add to `src/broker/routing.rs` tests:

```rust
#[test]
fn validate_unknown_fact_type_rejects() {
    // Setup: temp dir with .patina/schemas/github/schema.toml containing github.issue, github.pr
    // Emit fact with schema="github", fact_type="typo"
    // Assert: fact is dropped
}

#[test]
fn validate_known_fact_type_passes() {
    // Same setup, fact_type="issue"
    // Assert: passes
}

#[test]
fn validate_no_installed_schema_passes() {
    // Empty project_root (no .patina/schemas/)
    // Assert: passes (graceful degradation)
}
```

Also update the existing broker integration test in pre-push-checks.sh to verify
that fact_type validation doesn't break the happy path.

## Phase 2: CI drift checks

### Problem

Nothing prevents `wit/schema/github/schema.toml` from diverging from
`.patina/schemas/github/schema.toml` after install.

### Design

Add a check to `resources/git/pre-push-checks.sh` after the WIT consistency
checks. For each schema under `wit/schema/*/`:

1. **Installed drift**: if `.patina/schemas/<name>/schema.toml` exists, diff
   against `wit/schema/<name>/schema.toml`
2. **Manifest match**: for each `children/*/child.toml` declaring
   `[schemas.<name>]`, verify `package` version matches canonical

```bash
# In pre-push-checks.sh, after WIT checks:
echo "📦 [N/M] Checking schema consistency..."
schema_ok=true

for schema_dir in wit/schema/*/; do
    name=$(basename "$schema_dir")
    canonical="$schema_dir/schema.toml"
    [ -f "$canonical" ] || continue

    # Drift check: installed vs canonical
    installed=".patina/schemas/$name/schema.toml"
    if [ -f "$installed" ]; then
        if ! diff "$canonical" "$installed" > /dev/null 2>&1; then
            echo "   ERROR: installed schema '$name' differs from canonical"
            echo "   Fix: patina schema install wit/schema/$name"
            schema_ok=false
        fi
    fi

    # Manifest version check
    canonical_pkg=$(grep '^package' "$canonical" | head -1 | \
        sed 's/.*= *"\(.*\)"/\1/')
    for child_toml in children/*/child.toml; do
        [ -f "$child_toml" ] || continue
        child_pkg=$(grep -A1 "schemas.$name" "$child_toml" 2>/dev/null | \
            grep 'package' | sed 's/.*= *"\(.*\)"/\1/')
        if [ -n "$child_pkg" ] && [ "$child_pkg" != "$canonical_pkg" ]; then
            echo "   ERROR: $child_toml package '$child_pkg' != canonical '$canonical_pkg'"
            schema_ok=false
        fi
    done
done

if [ "$schema_ok" = false ]; then
    echo "❌ Schema consistency check failed!"
    exit 1
fi
echo "   ✓ Schema consistency OK"
```

### Files changed

- `resources/git/pre-push-checks.sh` — add schema consistency check section

## Phase 3: `patina schema build <name>`

### Problem

Today: `patina schema install wit/schema/github` then `patina schema generate`.
Two manual steps, easy to forget one.

### Design

Add `Build` variant to `SchemaCommands`:

```rust
/// Build a schema: validate, install, and optionally generate code
Build {
    /// Schema name (looks up wit/schema/<name>/)
    name: String,

    /// Also generate Rust types
    #[arg(long)]
    types: bool,

    /// Also generate SQLite migration DDL
    #[arg(long)]
    migrations: bool,

    /// Also generate embedding config
    #[arg(long)]
    embeddings: bool,
},
```

Implementation calls existing functions in sequence:

```rust
pub fn build_schema(name: &str, types: bool, migrations: bool, embeddings: bool) -> Result<()> {
    let root = find_project_root()?;
    let source = root.join("wit/schema").join(name);

    if !source.exists() {
        bail!("canonical schema not found: wit/schema/{}/", name);
    }

    // 1. Validate
    println!("Validating wit/schema/{}/...", name);
    let metadata = validate_package(&source)?;
    println!("  ✓ {} v{} — {} facts",
        metadata.schema.name, metadata.schema.version, metadata.facts.len());

    // 2. Install
    install_schema(&source.to_string_lossy())?;

    // 3. Generate (if requested)
    if types || migrations || embeddings {
        generate(types, migrations, embeddings, Some(name))?;
    }

    println!("\n✅ Schema '{}' built", name);
    Ok(())
}
```

### Files changed

- `src/commands/schema/mod.rs` — add `Build` variant, `build()` function
- `src/commands/schema/internal.rs` — add `build_schema()`
- `src/main.rs` — wire `Build` subcommand in match arm

## Execution Order

| Order | Phase | Risk | Effort | Files |
|-------|-------|------|--------|-------|
| 1st | Phase 4: delete dead code | None | 1 commit | 4 deleted |
| 2nd | Phase 1: broker validation | Low | 1-2 commits | 2-3 changed |
| 3rd | Phase 2: CI checks | None | 1 commit | 1 changed |
| 4th | Phase 3: build command | None | 1 commit | 3 changed |

Phase 4 first (zero risk). Phase 1 next (the real safety win). Phases 2-3 additive.

## Commits

1. `delete src/generated/schemas/ dead code` — 4 files deleted
2. `broker: validate fact_type against installed schema` — routing + cache
3. `ci: schema drift and manifest version checks` — pre-push script
4. `feat: patina schema build orchestration wrapper` — new subcommand

## Key Files

- `src/broker/mod.rs` — fact_type cache + validation in write_to_project
- `src/commands/schema/mod.rs` — expose load_schema_metadata, add Build command
- `src/commands/schema/internal.rs` — build_schema implementation
- `resources/git/pre-push-checks.sh` — schema consistency checks
- `src/generated/schemas/` — deleted

## Open Questions

None — all design decisions resolved. Ready to build.
