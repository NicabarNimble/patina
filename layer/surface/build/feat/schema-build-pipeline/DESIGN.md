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

### Design principle: one entry point, fail closed

All fact validation stays inside `routing::validate_fact()`. No pre-validation
logic in `broker/mod.rs`. Caching is an implementation detail inside the
validation function, not a responsibility of the caller.

**Fail closed:** If a schema is declared in `child.toml` but not installed on
disk, facts for that schema are rejected with a clear error. The install step
is not optional — if you declare a schema, it must be installed. This enforces
the "runtime reads installed schemas only" model.

### Signature change

```rust
// routing.rs — validate_fact()
pub fn validate_fact(
    fact: &BrokerFact,
    manifest: &ChildManifest,
    child_name: &str,
    project_root: &Path,                              // NEW
    schema_cache: &mut HashMap<String, HashSet<String>>, // NEW — replaces _warned_schemas
) -> Result<ValidatedFact>
```

The `schema_cache` maps schema name → set of valid event_types. Populated on
first fact for each schema, then reused. Owned by the caller (`write_to_project`)
but only read/written through `validate_fact`.

### Implementation

```rust
// Step 2: schema must be declared in manifest (existing)
if !manifest.schemas.contains_key(&fact.schema) {
    bail!("child '{}': schema '{}' not declared in manifest — fact dropped",
        child_name, fact.schema);
}

// Step 3: validate fact_type against installed schema (NEW)
let event_type = format!("{}.{}", fact.schema, fact.fact_type);

let valid_types = match schema_cache.entry(fact.schema.clone()) {
    std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
    std::collections::hash_map::Entry::Vacant(e) => {
        // Load installed schema — fail closed if missing
        let schema_dir = patina::paths::project::schemas_dir(project_root)
            .join(&fact.schema);
        let metadata = crate::commands::schema::load_schema_metadata(&schema_dir)
            .with_context(|| format!(
                "schema '{}' declared in manifest but not installed — \
                 run: patina schema install wit/schema/{}",
                fact.schema, fact.schema
            ))?;
        let types: HashSet<String> = metadata.facts.iter()
            .map(|f| f.event_type.clone())
            .collect();
        e.insert(types)
    }
};

if !valid_types.contains(&event_type) {
    bail!(
        "child '{}': fact_type '{}' not declared in installed schema '{}' — fact dropped",
        child_name, event_type, fact.schema
    );
}
```

### Call site change

```rust
// broker/mod.rs write_to_project()
let mut schema_cache: HashMap<String, HashSet<String>> = HashMap::new();

let fetch_result = child.fetch(&fetch_params, &mut |fact| {
    match validate_fact(&fact, manifest, &child_name, project_root, &mut schema_cache) {
        Ok(validated) => {
            validated_facts.push(validated);
            Ok(())
        }
        Err(e) => {
            eprintln!("[broker] {}: {}", child_name, e);
            Ok(())
        }
    }
});
```

The caller is unchanged in structure — just passes `project_root` and
`schema_cache` instead of `_warned_schemas`. All validation logic stays
inside `validate_fact`.

### Schema loading

Expose `load_schema_metadata()` from `commands::schema` — a thin re-export
of the existing `parse_schema_toml()`. Internal import, not a public API leak.

### Files changed

- `src/broker/routing.rs` — add project_root + schema_cache params, add Step 3
- `src/broker/mod.rs` — pass project_root and schema_cache to validate_fact
- `src/commands/schema/mod.rs` — expose `load_schema_metadata()`

### Tests

Add to `src/broker/routing.rs` tests:

```rust
#[test]
fn validate_unknown_fact_type_rejects() {
    // Setup: temp dir with .patina/schemas/github/schema.toml
    //   containing facts with event_type = github.issue, github.pr
    // Emit fact with schema="github", fact_type="typo"
    // Assert: rejected with "not declared in installed schema"
}

#[test]
fn validate_known_fact_type_passes() {
    // Same setup, fact_type="issue"
    // Assert: passes validation
}

#[test]
fn validate_missing_installed_schema_rejects() {
    // Empty project_root (no .patina/schemas/)
    // Schema declared in manifest but not installed
    // Assert: rejected with "not installed" error
}
```

## Phase 2: CI drift checks

### Problem

Nothing prevents the canonical schema from diverging from the installed runtime
copy. The connector-local duplicate drifted exactly this way.

### Design

Add a check to `resources/git/pre-push-checks.sh` after the WIT consistency
checks. For each schema under `wit/schema/*/`:

1. **Installed drift**: if `.patina/schemas/<name>/` exists, diff the entire
   directory (schema.toml AND .wit files) against `wit/schema/<name>/`
2. **Manifest match**: for each `children/*/child.toml` declaring a schema,
   use Rust-based parsing to compare package versions (not grep)

```bash
# In pre-push-checks.sh, after WIT checks:
echo "📦 [N/M] Checking schema consistency..."
schema_ok=true

for schema_src in wit/schema/*/; do
    name=$(basename "$schema_src")
    [ -f "$schema_src/schema.toml" ] || continue

    # Drift check: diff entire installed directory against canonical
    installed=".patina/schemas/$name"
    if [ -d "$installed" ]; then
        if ! diff -r "$schema_src" "$installed" > /dev/null 2>&1; then
            echo "   ERROR: installed schema '$name' differs from canonical"
            echo "   Fix: patina schema install wit/schema/$name"
            schema_ok=false
        fi
    fi
done

# Manifest version check: use patina binary to parse TOML correctly
for child_toml in children/*/child.toml; do
    [ -f "$child_toml" ] || continue
    child_dir=$(dirname "$child_toml")
    child_name=$(basename "$child_dir")

    # Extract schema references from child.toml using cargo/patina parsing
    # For each [schemas.<name>] section, verify package matches canonical
    for schema_src in wit/schema/*/; do
        name=$(basename "$schema_src")
        [ -f "$schema_src/schema.toml" ] || continue

        # Parse canonical package from schema.toml
        canonical_pkg=$(python3 -c "
import tomllib, sys
with open('$schema_src/schema.toml', 'rb') as f:
    d = tomllib.load(f)
print(d.get('schema', {}).get('package', ''))
" 2>/dev/null || echo "")

        # Parse child manifest package for this schema
        child_pkg=$(python3 -c "
import tomllib, sys
with open('$child_toml', 'rb') as f:
    d = tomllib.load(f)
pkg = d.get('schemas', {}).get('$name', {}).get('package', '')
print(pkg)
" 2>/dev/null || echo "")

        if [ -n "$child_pkg" ] && [ -n "$canonical_pkg" ] && \
           [ "$child_pkg" != "$canonical_pkg" ]; then
            echo "   ERROR: $child_toml declares package '$child_pkg'"
            echo "          but canonical schema '$name' is '$canonical_pkg'"
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

Note: Uses `python3 -c` with `tomllib` (stdlib since 3.11) for reliable TOML
parsing instead of brittle grep. This is a CI-only tool, not a runtime
dependency — the pre-push script already shells out to cargo/patina.

Alternative: write a `patina schema check-consistency` subcommand in Rust
and call it from the script. Cleaner but more code for Phase 2. Can refactor
in a later pass.

### Files changed

- `resources/git/pre-push-checks.sh` — add schema consistency check section

## Phase 3: `patina schema build <name>`

### Problem

Today: `patina schema install wit/schema/github` then `patina schema generate`.
Two manual steps, easy to forget one.

### Design

Add `Build` variant to `SchemaCommands`. Orchestrates validate → install,
with optional generate outputs behind flags.

```rust
/// Build a schema: validate and install, with optional code generation
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
| 2nd | Phase 1: broker validation | Low | 1-2 commits | 3 changed |
| 3rd | Phase 2: CI checks | None | 1 commit | 1 changed |
| 4th | Phase 3: build command | None | 1 commit | 3 changed |

Phase 4 first (zero risk). Phase 1 next (the real safety win). Phases 2-3 additive.

## Commits

1. `delete src/generated/schemas/ dead code` — 4 files deleted
2. `broker: validate fact_type against installed schema, fail closed` — routing + cache
3. `ci: schema drift checks (full directory) and manifest version` — pre-push script
4. `feat: patina schema build — validate + install + optional generate` — new subcommand

## Key Files

- `src/broker/routing.rs` — fact_type validation with cache, fail closed
- `src/broker/mod.rs` — pass project_root and schema_cache to validate_fact
- `src/commands/schema/mod.rs` — expose load_schema_metadata, add Build command
- `src/commands/schema/internal.rs` — build_schema implementation
- `resources/git/pre-push-checks.sh` — schema consistency checks (full dir diff)
- `src/generated/schemas/` — deleted

## Open Questions

None — all design decisions resolved. Ready to build.
