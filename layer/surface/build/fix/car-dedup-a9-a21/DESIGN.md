# Design: CAR Dedup and Path Truth (A9, A17-A21)

## Principle Alignment

- [[patina-identity]] and [[dependable-rust]]: one canonical source per concern.
- [[unix-philosophy]]: remove duplicate utility logic that fractures behavior.

## Gate Details

### A9: Path Truth

Migrate these 5 highest-traffic sites to `crate::paths`:
1. `project/internal.rs:312-334` — reimplements `patina_dir()`, `config_path()`, `local_dir()`, `backups_dir()`
2. `version.rs:55` — hardcodes `.patina/versions.json`
3. `commands/mother/daemon.rs:64` — hardcodes `.patina/uid`
4. `commands/launch/internal.rs:38` — hardcodes `.patina/config.toml`
5. `commands/ai/surface.rs:149` — hardcodes `.patina/config.toml`

Add CI guard to prevent regression:
```bash
grep -r '\.join(".patina")' src/ --include='*.rs' | grep -v 'src/paths.rs' | grep -v '#\[cfg(test)\]'
```
Must return empty (or only `migration.rs` which legitimately references old paths).

### A17: is_safe_identifier dedup

Move `is_safe_identifier()` to `src/commands/scrape/database.rs` (already the shared re-export facade for scrape). Both `events.rs` and `projection.rs` import from there. Delete both copies.

### A18: strip_frontmatter dedup

Make `strip_frontmatter()` a `pub(crate)` function in `oxidize/mod.rs`. Delete the copy in `oxidize/beliefs.rs`. Both files are in oxidize/ so this is a local move.

### A19: extract_section_items dedup

Keep the `queries.rs` version (main query path). Fix its single-digit limitation: change `"1. ".."9. "` enumeration to generic `starts_with(|c: char| c.is_ascii_digit())`. Delete the `packets.rs` copy; import from queries.

### A20: Semver bump consolidation

Keep `compute_next_version` and `update_cargo_version` in `release/internal.rs`. Delete copies in `dev/release.rs:102-153` and `dev/bump_version.rs:100-115`. Dev commands import from `crate::release`.

### A21: Test helper extraction

Move `with_temp_patina_home` to `src/test_support.rs` as:
```rust
pub fn with_temp_patina_home<T>(f: impl FnOnce(PathBuf) -> T) -> T
```
Update all 8+ test modules to import from `crate::test_support::with_temp_patina_home`.

## Strategy

- Prioritize high-traffic duplication first (A9 path construction, A20 semver).
- For each duplicate family: pick one canonical implementation, migrate all call sites, delete the duplicate.

## Verification

- `cargo check --workspace -q` and `cargo test -q --lib` after each dedup.
- Targeted tests around section parsing (A19), semver bumping (A20), and path computation (A9).

## Out of Scope

- Dead module deletion and deprecated command cleanup.
