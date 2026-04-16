# Design: Spec Atlas + MCT Visibility

## Intent

Create a deterministic, local-first visibility surface for project governance and architecture:
- spec sprawl control (inventory + graph + progress)
- MCT inventory visibility (children + toys)

The atlas is a read lens. It does not mutate spec state or runtime state.

## Build Target

Add a new CLI surface:

- `patina atlas --json`
- `patina atlas --output <path>`
- `patina atlas --html --output <path>`

The command builds one in-memory snapshot from repository truth and renders either JSON or a standalone HTML file.

## Data Sources

1. Specs
   - Scan `layer/surface/build/**/SPEC.md`
   - Parse frontmatter via `patina::spec::parse_spec_file`
   - Compute criteria progress (`checked/total`)
   - Build dependency edges:
     - `blocked_by` -> blocks edge
     - resolvable `related` -> related edge

2. Children
   - Scan `children/*/child.toml`
   - Parse `[child]` (`name`, `kind`, `role`)
   - Parse `[needs].toys`
   - Derive lane hints (`typed` vs `legacy` toy aliases)

3. Toys
   - Parse `wit/toys/deps/toys-registry.toml`
   - Expose toy names + file/version/source

## Output Model

A normalized snapshot object:

- metadata (generated_at, project_root)
- spec summary + specs list + graph edges
- child inventory
- toy inventory

The same model drives both JSON and HTML output.

## HTML Strategy

Render a self-contained HTML file with inline CSS/JS and embedded snapshot JSON.
No framework runtime is required for the first slice.
This keeps artifact portability high and allows future SvelteKit migration as a view layer using the same snapshot schema.

## Fail-Closed Contract

- Malformed SPEC frontmatter causes command error with source path.
- Missing optional inventories (children directory absent) resolve to empty arrays.
- Contradictory flags (`--json` + `--html`) error explicitly.

## Direct Code Targets

- `src/main.rs` (new `atlas` command wiring)
- `src/commands/mod.rs` (register module)
- `src/commands/atlas/mod.rs` (public command API)
- `src/commands/atlas/internal.rs` (scan/build/render implementation + tests)
- `src/commands/pando.rs` (register native command name)
- `README.md` (new command example)

## Verification Plan

```bash
cargo test -q atlas
cargo check -q
patina atlas --json
patina atlas --html --output .tmp/atlas/spec-atlas.html
```
