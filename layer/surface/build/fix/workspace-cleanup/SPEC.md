---
type: fix
id: workspace-cleanup
status: ready
created: 2026-02-15
sessions:
  origin: 20260215-204444
related:
- layer/core/patina-identity.md
- layer/core/dependable-rust.md
- layer/core/unix-philosophy.md
beliefs:
- root-communicates-identity
---

# fix: Workspace Cleanup — Consolidate Root Sprawl

> Reduce 26 root directories to ~10 by moving grammar plugins into `grammars/`,
> workspace plugin crates into `plugins/`, deleting deprecated crates and dead dirs.
> Per [[root-communicates-identity]]: the root should communicate what Patina IS.

## Problem

The project root has 26 directories. 18 are plugin/crate infrastructure that
buries the core project (`src/`, `layer/`, `wit/`):

- 9 `grammar-*` dirs — standalone WASM plugins (own Cargo.lock, wasm32-wasip2)
- 4 `patina-*-api` dirs — deprecated API crates ("superseded by patina-sdk")
- 3 `patina-*` dirs — active workspace plugin crates
- 1 `patina-sdk` — published SDK
- 1 `patina-metal` — dead directory (empty scaffold, unreferenced)

A new contributor `ls`ing the root sees grammar-c through grammar-typescript
before they see `src/`. The root communicates "grammar plugin collection" instead
of "knowledge protocol engine."

## What Exists Today

### Grammar crates (9) — standalone, movable

Each `grammar-*` dir is a standalone WASM crate:
- Own `Cargo.lock` and `.cargo/config.toml` (target = wasm32-wasip2)
- Depends on `patina-sdk` from **crates.io** (version 0.21), NOT path deps
- NOT workspace members (root Cargo.toml excludes them)
- No `wit/` dirs (WIT comes from patina-sdk)
- 8/9 use tree-sitter + `cc` build-dep; grammar-cairo uses native Rust parser

**Key finding:** Zero path dependencies into the workspace. Moving them changes
nothing about how they build or resolve dependencies.

### Active workspace crates (4) — entangled

- `patina-sdk` — published to crates.io, zero path deps. Features: pipeline,
  command, mother-child, task.
- `patina-doctor` — `path = "../patina-sdk"` (command plugin)
- `patina-plugin-models` — `path = "../patina-sdk"` (mother-child plugin)
- `patina-plugin-repos` — `path = "../patina-sdk"` (mother-child plugin)

All are workspace members sharing root `Cargo.lock`.

### Deprecated crates (4) — removal candidates

- `patina-command-api` — `publish = false`, "superseded by patina-sdk"
- `patina-task-api` — `publish = false`, "superseded by patina-sdk"
- `patina-pipeline-api` — `publish = false`, "superseded by patina-sdk"
- `patina-plugin-api` — `publish = false`, "superseded by patina-sdk"

Kept alive for: WIT file distribution (pre-push-checks.sh) and 3 test fixtures
(`tests/echo-pipeline`, `tests/hello-task`, `tests/panic-pipeline`).

**SDK parity verified:** Every public export from all 4 deprecated crates exists
in patina-sdk with the corresponding feature flag. `register_pipeline!`,
`register_task!`, `register_command!`, `register_plugin!` macros all re-exported.
`PipelinePlugin`, `TaskPlugin`, `CommandPlugin`, `MotherChildPlugin` traits,
`ChildHealth`, `HealthStatus`, `Toy` types — all present. The internal macro paths
differ (`$crate::pipeline::__register_pipeline` vs `$crate::__register_pipeline`)
but this is transparent to plugin authors.

### WIT file distribution model

The canonical source of truth is `wit/` at the repo root. WIT files are
distributed to crates via two mechanisms:

- **Hard links:** `wit/deps/patina-host/host.wit` is hard-linked (same inode)
  to 7 copies across canonical wit/ subdirs and deprecated crates.
- **Symlinks:** `patina-plugin-api/wit/` → `../wit`, `patina-plugin-models/wit/`
  → `../wit`, `patina-plugin-repos/wit/` → `../wit` (full tree via symlink).
- **Copies:** `patina-sdk/wit/` has content-matching files but different inodes
  (NOT hard links). It also lacks the top-level `wit/deps/` directory.

`pre-push-checks.sh` validates two things:
1. **Step 1 (content):** `diff -r` of wit/ trees in 4 crates
   (patina-plugin-api, patina-plugin-models, patina-plugin-repos, patina-command-api)
2. **Step 2 (hard links):** Inode equality for 7 `host.wit` copies
   (4 in canonical wit/, 3 in deprecated crates)

**patina-sdk is NOT currently validated** by pre-push-checks.sh.
Canonical `wit/` remains the source of truth after cleanup.

### Dead directory (1)

- `patina-metal` — contains only `grammars/solidity/.vscode/launch.json`,
  a generic Node.js tree-sitter launch config. Not patina-specific. Not useful
  to migrate (grammar-solidity uses Rust + cc build-dep, not Node.js).
  Not referenced by any source code. Not a Cargo crate.

### All references to root-level crate dirs

Full `rg 'grammar-'` and `rg 'patina-(sdk|doctor|plugin|command|task|pipeline|metal)'`
across the repo (excluding target/, .patina/, and the crate dirs themselves):

| File | What it references | Category |
|------|-------------------|----------|
| `src/commands/setup/grammars.rs` | `format!("grammar-{}", name)` in find_source_root() + install() | **Code — must update** |
| `src/commands/bench/grammar.rs` | Error message: "grammar-rust plugin not installed" | **Code — cosmetic only** (references installed location, not source) |
| `resources/scripts/grammar-compare.sh` | PLUGIN_DIRS maps to installed `~/.patina/pipeline/grammar-*` | **Safe — no change** (install location unchanged) |
| `resources/grammar-defaults.toml` | Language names only ("rust", "go"), not dir paths | **Safe — no change** |
| `resources/git/pre-push-checks.sh` | WIT checks for 6 patina-* crate paths | **Script — must update** |
| `Cargo.toml` | 8 workspace member paths | **Config — must update** |
| `.gitignore` | `patina-metal/grammars/**/Cargo.lock` | **Config — remove in Phase D** |
| `tests/*/Cargo.toml` (3 files) | `../../patina-pipeline-api`, `../../patina-task-api` | **Config — must update** |
| `CLAUDE.md` (line 110) | `grammar-*/` in project structure diagram | **Docs — update in Phase A** |
| `README.md` (lines 232-237) | `patina-metal/`, `patina-plugin-api/`, `patina-doctor/`, etc. | **Docs — update in Phases A, B, C, D** |
| `.gitattributes` (line 2) | `patina-metal/grammars/** linguist-vendored` | **Config — remove in Phase D** |
| `.ignore` (line 2) | `patina-metal/grammars/*/` | **Config — remove in Phase D** |
| `resources/bench/patina-commits-v1.json` (line 389) | `patina-metal/Cargo.toml` in historical benchmark ground truth | **Safe — no change** (benchmark records past state) |
| `layer/sessions/*.md` (12 files) | Historical session references | **Archive — no change** |
| `layer/surface/epistemic/beliefs/*.md` (5 files) | Belief evidence references | **Archive — no change** |

**CI workflows are clean.** `.github/workflows/test.yml` and `release.yml` use
`cargo test --workspace` / `cargo clippy --workspace`. No hardcoded crate paths.

**Publishing is clean.** `cargo publish -p patina-sdk` uses package name, not
directory path. No release scripts or docs `cd` into `patina-sdk/`. The SDK's
`include` field uses relative paths (`src/**`, `wit/**`) that are directory-agnostic.

## What To Build

One spec, four phases, executed as scalpel commits. **Phases must run in
order A → B → C → D.** Later phases depend on directory locations established
by earlier ones (Phase C fixture paths assume Phase B's `plugins/sdk/` exists;
Phase C pre-push edits assume Phase B's `plugins/models/` paths). Each phase
leaves the repo green — all tests pass and pre-push-checks.sh succeeds after
each commit.

**Verification after every phase:**

Each phase must leave the repo green. Run these after every phase commit:
```bash
cargo build --release && cargo test --workspace
./resources/git/pre-push-checks.sh
```

Path dependency checks and other targeted verifications vary by phase
(e.g., `plugins/` doesn't exist until Phase B, and `tests/` legitimately
references `../../patina-pipeline-api` until Phase C rewrites it). Each
phase's verification section below specifies the exact checks for that phase.

### Phase A: Move grammar-* into grammars/ (HIGH IMPACT, LOW RISK)

Move 9 standalone grammar crates into `grammars/{lang}/`:

```
grammar-c/          → grammars/c/
grammar-cairo/      → grammars/cairo/
grammar-cpp/        → grammars/cpp/
grammar-go/         → grammars/go/
grammar-javascript/ → grammars/javascript/
grammar-python/     → grammars/python/
grammar-rust/       → grammars/rust/
grammar-solidity/   → grammars/solidity/
grammar-typescript/ → grammars/typescript/
```

**Code changes:**
1. Create the destination directory and move crates into it:
   ```bash
   mkdir -p grammars
   for lang in c cairo cpp go javascript python rust solidity typescript; do
     git mv "grammar-$lang" "grammars/$lang"
   done
   ```
   (`git mv` preserves blame.)
2. Update `src/commands/setup/grammars.rs`:
   - Module doc comment (lines 3-4): Change `grammar-*/ directories` →
     `grammars/<lang>/ directories` so `--help` and code readers see the
     correct source layout.
   - `find_source_root()` (lines 187-228): Change detection heuristic from
     `grammar-rust/plugin.toml` → `grammars/rust/plugin.toml` (lines 201, 210, 217).
     Update the `bail!` message (line 223) to say `grammars/*/plugin.toml`.
   - `install()` (line 76): Change source path construction from
     `format!("grammar-{}", name)` → `format!("grammars/{}", name)`
   - `print_list()` (lines 150, 170): Same path format change
3. Update `CLAUDE.md` (line 110) and `README.md` (lines 233-237): Change
   `grammar-*/` → `grammars/` in project structure diagrams
4. Fix stale doc comments in `grammars/cairo/src/parser.rs`: Two lines
   reference patina-metal:
   - Line 1: `//! Cairo language parser — ported from patina-metal/src/cairo.rs.`
     → `//! Cairo language parser — native Rust implementation using cairo-lang-parser.`
   - Line 4: `//! Self-contained: no dependency on patina-metal or patina-ai.`
     → delete the line (the "self-contained" qualifier is meaningless for a
     standalone grammar plugin — there is nothing to be self-contained *from*)
   (Both references are historically stale; the parser has been a standalone
   grammar plugin since extraction. This eliminates false positives in
   Phase D's `rg 'patina-metal'` verification.)
5. Update vendored-file exclusions for the new grammar locations. After the
   move, 8 of 9 grammar crates contain vendored tree-sitter C sources at
   `grammars/<lang>/grammars/<lang>/src/` (grammar-cairo has no vendored C).
   Without these entries, GitHub Linguist counts tens of thousands of C lines
   as first-party code, and ripgrep/fd traverses them on every search.

   - `.gitattributes`: Add `grammars/*/grammars/** linguist-vendored`
     (existing `patina-metal/grammars/**` entry stays until Phase D removes it)
   - `.ignore`: Add `grammars/*/grammars/` before the existing `layer/dust/` line
     (existing `patina-metal/grammars/*/` entry stays until Phase D removes it)
   - `.gitignore`: Add `grammars/*/target/` after the existing `/target/` line
     (grammar WASM build artifacts — each crate has its own target/ directory;
     existing `patina-metal/grammars/**/Cargo.lock` stays until Phase D)

**What does NOT change:**
- Grammar `Cargo.toml` contents (crate names stay `grammar-c`, deps, targets)
- Grammar `.cargo/config.toml` (wasm32-wasip2 target, wasi-sdk paths)
- Grammar `Cargo.lock` files (deps are unchanged)
- Grammar `build.rs` files
- Installed plugin location (`~/.patina/pipeline/grammar-*`)
- Root workspace `Cargo.toml` (grammars are not workspace members)
- `resources/grammar-defaults.toml` (uses language names, not paths)
- `resources/scripts/grammar-compare.sh` (PLUGIN_DIRS maps to installed location
  `~/.patina/pipeline/grammar-*`, not source dirs — install location is unchanged)
- `src/commands/bench/grammar.rs` (references installed location `~/.patina/pipeline/`, not source)

**Verification:**
1. Build, test, pre-push:
   `cargo build --release && cargo test --workspace && ./resources/git/pre-push-checks.sh`
2. Exercise `find_source_root()` and `install()` end-to-end:
   ```bash
   cargo run --release -- setup grammars --list
   ```
   Every grammar should show "available" or "installed" with a source path
   under `grammars/`. If any show "missing", `find_source_root()` didn't
   find the new directory layout.
3. Build both grammar plugin archetypes from their new locations:
   - Tree-sitter + cc: `cd grammars/rust && cargo build --target wasm32-wasip2`
   - Pure Rust parser: `cd grammars/cairo && cargo build --target wasm32-wasip2`
     (grammar-cairo is the only non-tree-sitter implementation and the likeliest
     place for path-sensitive assumptions.)

**Root:** 26 → 18 dirs. 9 grammar dirs consolidated into 1.

### Phase B: Move workspace plugins into plugins/ (MEDIUM RISK)

Move 4 active workspace crates:

```
patina-sdk/           → plugins/sdk/
patina-doctor/        → plugins/doctor/
patina-plugin-models/ → plugins/models/
patina-plugin-repos/  → plugins/repos/
```

**Code changes:**
1. Create the destination directory and move crates into it:
   ```bash
   mkdir -p plugins
   git mv patina-sdk plugins/sdk
   git mv patina-doctor plugins/doctor
   git mv patina-plugin-models plugins/models
   git mv patina-plugin-repos plugins/repos
   ```
2. Update root `Cargo.toml` workspace members (line 2):
   `"patina-sdk"` → `"plugins/sdk"`, `"patina-doctor"` → `"plugins/doctor"`,
   `"patina-plugin-models"` → `"plugins/models"`,
   `"patina-plugin-repos"` → `"plugins/repos"`
3. Update path deps in 3 plugin Cargo.toml files:
   `path = "../patina-sdk"` → `path = "../sdk"`
4. Update `resources/git/pre-push-checks.sh` **in the same commit** as the moves.
   Phase B only updates paths for crates that moved — deprecated crates stay
   in the script because they still exist on disk and their checks still pass:
   - Step 1 mother-child loop (line 17): Change to
     `for crate_dir in patina-plugin-api plugins/models plugins/repos; do`
     (`patina-plugin-api` stays — it's deprecated but still on disk with a
     valid `wit/` symlink. Removing it is Phase C's job when the crate is deleted.)
   - Step 1 command loop (lines 27-35): No change. `patina-command-api` still
     exists at root with valid WIT files. Phase C deletes the loop.
   - Step 2 hard link COPIES array: No change. All 7 entries still exist on
     disk, hard links still resolve. Phase C removes the 3 deprecated entries.
5. Update `CLAUDE.md` and `README.md`: project structure diagrams
   (`README.md` lines 233-237 list `patina-plugin-api/`, `patina-doctor/`, etc.)
6. `.gitignore`: Add `plugins/*/target/` after the existing `grammars/*/target/`
   line (added in Phase A). Plugin crates are workspace members so builds
   normally use the root `target/`, but a direct `cargo build` inside a plugin
   crate would create a local `target/` that must not be tracked.
7. Crate names stay the same (Cargo package name ≠ directory name)

**Symlink update:** `patina-plugin-models/wit/` and `patina-plugin-repos/wit/`
are symlinks to `../wit`. After moving to `plugins/models/` and `plugins/repos/`,
these symlinks must point to `../../wit` instead. Fix with:
`cd plugins/models && rm wit && ln -s ../../wit wit` (and same for repos).
Verify with `ls -la plugins/*/wit` — should show `../../wit`.

**Publishing is safe:** `cargo publish -p patina-sdk` resolves by package name.
The SDK's `include` field uses relative paths. No release scripts or automation
`cd` into `patina-sdk/`. Session archives show the publish command as
`patina secrets run -- cargo publish -p patina-sdk` (package name, not path).

**Verification:**
1. Build, test, pre-push:
   `cargo build --release && cargo test --workspace && ./resources/git/pre-push-checks.sh`
2. Stale path dep check — must produce NO matches:
   `rg '../patina-sdk' plugins/`
   (Every plugin Cargo.toml should now say `path = "../sdk"`, not `"../patina-sdk"`.)
3. Confirm symlinks: `ls -la plugins/*/wit` — should show `../../wit`.
4. Confirm SDK packaging from new location: `cargo package -p patina-sdk`
   (Validates manifest, include/exclude globs, and tarball creation from
   `plugins/sdk/`. No registry credentials required — unlike `cargo publish
   --dry-run`, `cargo package` never contacts crates.io. Runs post-commit
   per the global verification preamble, so the working tree is clean.)

**Root:** 18 → 15 dirs.

### Phase C: Delete deprecated API crates (MEDIUM RISK)

Remove 4 deprecated crates that patina-sdk superseded. SDK parity has been
verified — zero API gaps (see "What Exists Today" section above).

**Step 1: Migrate 3 test fixtures to patina-sdk**

Each fixture needs two changes: Cargo.toml dep swap and import path swap.

`tests/echo-pipeline/Cargo.toml`:
```toml
# OLD:
patina-pipeline-api = { path = "../../patina-pipeline-api" }
# NEW (after Phase B, sdk is at plugins/sdk):
patina-sdk = { path = "../../plugins/sdk", features = ["pipeline"] }
```

`tests/echo-pipeline/src/lib.rs`:
```rust
// OLD:
use patina_pipeline_api::{parse_request, register_pipeline, PipelinePlugin};
// NEW:
use patina_sdk::{parse_request, register_pipeline, PipelinePlugin};
```

`tests/panic-pipeline/Cargo.toml`: Same dep swap as echo-pipeline.

`tests/panic-pipeline/src/lib.rs`:
```rust
// OLD:
use patina_pipeline_api::{register_pipeline, PipelinePlugin};
// NEW:
use patina_sdk::{register_pipeline, PipelinePlugin};
```

`tests/hello-task/Cargo.toml`:
```toml
# OLD:
patina-task-api = { path = "../../patina-task-api" }
# NEW:
patina-sdk = { path = "../../plugins/sdk", features = ["task"] }
```

`tests/hello-task/src/lib.rs`:
```rust
// OLD:
use patina_task_api::{register_task, TaskPlugin, Toy};
// NEW:
use patina_sdk::{register_task, TaskPlugin, Toy};
```

No other source changes needed — the registration macros, traits, and types
have identical signatures. The internal macro paths differ
(`$crate::pipeline::__register_pipeline` vs `$crate::__register_pipeline`)
but this is transparent to callers.

**Step 2: Update pre-push-checks.sh**

After test migration, the deprecated crates serve no purpose. Phase B left
them in the script because they were still on disk. Now they're being deleted,
so remove all references:

- **Step 1 mother-child loop:** Remove `patina-plugin-api` from the loop
  (Phase B already updated the other two to `plugins/models plugins/repos`).
  Final loop: `for crate_dir in plugins/models plugins/repos; do`
- **Step 1 command loop (lines 27-35):** Delete the entire loop block.
  `patina-command-api` is being deleted; there are no remaining command
  guest crates to check.
- **Step 2 hard link COPIES array:** Remove 3 entries for deleted crates:
  ```
  "patina-command-api/wit/command/deps/patina-host/host.wit"
  "patina-task-api/wit/task/deps/patina-host/host.wit"
  "patina-pipeline-api/wit/pipeline/deps/patina-host/host.wit"
  ```
  Remaining checks: 4 canonical `wit/` internal hard links.

**WIT source of truth:** Canonical `wit/` at repo root remains the source of
truth. It is NOT moved or changed. patina-sdk has content-matching WIT copies
(not hard links) and lacks top-level `wit/deps/` — it consumes WIT, it doesn't
distribute it.

**New WIT drift guard for patina-sdk (Step 1b):** Once the deprecated crates
are gone, pre-push-checks.sh no longer validates any downstream WIT copies —
the remaining Step 2 hard links all live inside canonical `wit/` itself. But
`plugins/sdk/wit/` ships to crates.io and must stay in sync.

Insert this new check **between the existing Step 1 (WIT consistency) and
the `if [ "$wit_ok" = false ]` guard block** that follows it. The new loop
sets `wit_ok=false` on failure, which the existing guard block then catches:

```bash
# Step 1b: SDK WIT consistency — ensure published SDK ships current WIT
# Compare world definitions AND their deps/patina-host/host.wit copies
echo "   Checking SDK WIT consistency..."
for world in command mother-child pipeline task; do
    if ! diff "wit/$world/$world.wit" "plugins/sdk/wit/$world/$world.wit" > /dev/null 2>&1; then
        echo "   ERROR: plugins/sdk/wit/$world/$world.wit differs from canonical"
        echo "   Fix: cp wit/$world/$world.wit plugins/sdk/wit/$world/$world.wit"
        wit_ok=false
    fi
    if ! diff "wit/$world/deps/patina-host/host.wit" "plugins/sdk/wit/$world/deps/patina-host/host.wit" > /dev/null 2>&1; then
        echo "   ERROR: plugins/sdk/wit/$world/deps/patina-host/host.wit differs from canonical"
        echo "   Fix: cp wit/$world/deps/patina-host/host.wit plugins/sdk/wit/$world/deps/patina-host/host.wit"
        wit_ok=false
    fi
done
# The existing guard block immediately after this:
#   if [ "$wit_ok" = false ]; then
#       echo "❌ WIT consistency check failed!"
#       exit 1
#   fi
# ...catches any failures from Steps 1, 1b, or 2.
```

This catches drift in both the world definitions and the `host.wit` copies
that get published. The SDK's copies are not hard links — content equality
via `diff` is the right check. Inode enforcement is not required since the
SDK is a consumer, not the canonical source.

**Step 3: Remove from workspace, delete, and update docs**

- Remove `"patina-command-api"`, `"patina-task-api"`, `"patina-pipeline-api"`,
  `"patina-plugin-api"` from `Cargo.toml` workspace members
- `git rm -r` the 4 directories
- Update `README.md` (lines 233-234): Remove `patina-plugin-api/` and
  `patina-command-api/` entries from the project structure diagram
- Update `CLAUDE.md`: Remove any remaining references to deleted crate dirs

**Verification:**
1. Build, test, pre-push (which now includes the SDK WIT drift guard):
   `cargo build --release && cargo test --workspace && ./resources/git/pre-push-checks.sh`
2. Stale path dep check — must produce NO matches:
   `rg '../../patina-pipeline-api|../../patina-task-api' tests/`
   (Every test fixture should now reference `../../plugins/sdk`.)
3. Build a test fixture WASM to confirm SDK bindings work:
   `cd tests/echo-pipeline && cargo build --target wasm32-wasip2`.

**Root:** 15 → 11 dirs.

### Phase D: Delete patina-metal (NO RISK)

`patina-metal/` contains only `grammars/solidity/.vscode/launch.json` — a generic
Node.js tree-sitter launch config (references `npm`, `node_modules/bin/tree-sitter`).
This is not useful to migrate: grammar-solidity uses Rust + `cc` build-dep, not
Node.js. No source code references patina-metal. No one uses the launch config
(there is no `.vscode/` at the repo root, and grammar-solidity itself has no
`.vscode/` directory).

**Changes:**
1. `git rm -r patina-metal/`
2. Remove `.gitignore` line: `patina-metal/grammars/**/Cargo.lock`
3. Remove `.gitattributes` line: `patina-metal/grammars/** linguist-vendored`
4. Remove `.ignore` line: `patina-metal/grammars/*/`
5. Update `README.md` (line 232): Remove `patina-metal/` entry from
   project structure diagram
6. `resources/bench/patina-commits-v1.json` (line 389): Lists
   `patina-metal/Cargo.toml` as a `relevant_docs` entry for a historical
   commit query. This is ground-truth benchmark data — the file was relevant
   *at that commit*. The benchmark compares search results against ground
   truth; a file that no longer exists simply won't be found, which is
   correct behavior for a historical query. **No change needed** — the
   benchmark doesn't read the file, it just records that it was relevant.

**Verification:**
1. Build, test, pre-push:
   `cargo build --release && cargo test --workspace && ./resources/git/pre-push-checks.sh`
2. Confirm no stale references in code, config, scripts, or docs:
   ```bash
   rg 'patina-metal' \
     src/ grammars/ plugins/ tests/ scripts/ examples/ \
     resources/git/ resources/scripts/ \
     Cargo.toml .gitignore .gitattributes .ignore README.md CLAUDE.md
   ```
   This should produce **zero matches**. The search targets only actionable
   locations — not archival knowledge (`layer/`), benchmark data
   (`resources/bench/`), or build artifacts (`target/`). Historical
   references in those directories are expected and harmless.

   If the command produces matches, the reference is stale — update or
   remove it. Every searched location is actionable by definition.

**Root:** 11 → 10 dirs.

## Target Root Structure

```
patina/
├── src/              → Protocol engine (Rust source)
├── layer/            → Knowledge product (git-tracked)
├── wit/              → Interface contract (WIT definitions)
├── grammars/         → Grammar WASM plugins (9 languages)
├── plugins/          → Workspace plugin crates (sdk, doctor, models, repos)
├── resources/        → Templates, scripts, configs
├── tests/            → Integration test fixtures
├── scripts/          → Dev scripts (model downloads)
├── examples/         → Example projects
└── target/           → Build output (gitignored)
```

10 directories. Each communicates a clear purpose. The root tells you what
Patina IS: a protocol engine (`src/`) that produces knowledge (`layer/`)
through a defined contract (`wit/`), extended by grammars and plugins.

## Exit Criteria

1. `ls` at root shows ≤12 directories (10 target + reasonable additions)
2. `cargo build --release` succeeds
3. `cargo test --workspace` passes
4. `cargo clippy --workspace -- -D warnings` passes
5. All grammar crates build with `cargo build --target wasm32-wasip2` from new locations
6. `patina setup grammars` finds and installs grammar plugins from `grammars/`
7. `./resources/git/pre-push-checks.sh` passes all checks (including new SDK WIT drift guard)
8. `git log --follow` preserves blame for moved files
9. `cargo run --release -- setup grammars --list` finds grammar sources under `grammars/`
10. `rg '../patina-sdk' plugins/` returns no matches (stale path deps)
11. `rg '../../patina-pipeline-api|../../patina-task-api' tests/` returns no matches

## Non-Goals

- Changing crate names (only directory locations change)
- Changing the plugin install location (`~/.patina/pipeline/`)
- Restructuring `src/` internals
- Modifying WIT interface definitions
- Changing CI workflow files (they're already clean)
- Making patina-sdk the WIT distribution point (canonical `wit/` stays)

## Evidence

| Claim | Source |
|-------|--------|
| paths.rs has zero root-level dir references | `src/paths.rs` (full read) |
| Grammar crates depend on patina-sdk from crates.io | All 9 `grammar-*/Cargo.toml` |
| Grammar crates have own Cargo.lock | All 9 `grammar-*/Cargo.lock` exist |
| Deprecated crates say "superseded by patina-sdk" | `patina-{command,task,pipeline,plugin}-api/Cargo.toml` description field |
| SDK parity: zero API gaps with deprecated crates | `patina-sdk/src/lib.rs` vs all 4 deprecated `src/lib.rs` |
| Test fixtures need only import path swaps | `tests/{echo,panic}-pipeline/src/lib.rs`, `tests/hello-task/src/lib.rs` |
| WIT canonical source stays at repo root | `wit/deps/patina-host/host.wit` is the hard-link source (inode 151275433) |
| patina-sdk WIT files are copies, not hard links | `stat` shows different inodes for sdk wit/ files |
| patina-metal is unreferenced by source code | `rg 'patina-metal' src/` returns nothing |
| patina-metal referenced by .gitignore, .gitattributes, .ignore | All three have `patina-metal/` entries that need removal |
| patina-metal in benchmark is historical ground truth | `resources/bench/patina-commits-v1.json:389` — records file relevance at a past commit, not a runtime path |
| patina-metal launch.json is generic Node.js config | `patina-metal/grammars/solidity/.vscode/launch.json` (full read) |
| README.md lists deleted crates in structure diagram | `README.md:232-237` — patina-metal/, patina-plugin-api/, patina-command-api/ |
| No release automation references patina-sdk by path | `.github/workflows/release.yml`, all `scripts/` — no `cd patina-sdk` |
| Publishing uses package name, not directory | Session archive: `patina secrets run -- cargo publish -p patina-sdk` |
| CI uses workspace-level commands | `.github/workflows/test.yml` |
| find_source_root() hardcodes grammar-rust | `src/commands/setup/grammars.rs:201,210` |
| CLAUDE.md references grammar-* in structure diagram | `CLAUDE.md:110` |
| README.md references patina-* in structure diagram | `README.md:233-237` |
| grammar-defaults.toml uses language names, not paths | `resources/grammar-defaults.toml` (full read) |
| grammar-compare.sh maps to installed location, not source | `resources/scripts/grammar-compare.sh:37-45,100-101` (PLUGIN_DIRS → `~/.patina/pipeline/`) |
| bench/grammar.rs references installed location only | `src/commands/bench/grammar.rs:63` (references `~/.patina/pipeline/`) |
| grammar-cairo is the only non-tree-sitter grammar | `grammar-cairo/Cargo.toml` — no `cc` build-dep, uses cairo-lang-parser |
| Phases depend on prior directory state | Phase C fixture paths use `../../plugins/sdk` (requires Phase B) |
| patina-sdk WIT files can drift silently post-Phase C | Only remaining Step 2 hard links are internal to canonical `wit/` |
