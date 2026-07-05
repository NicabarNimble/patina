# SDK MCT child-authoring track

- [ ] S0 — Housekeeping and oracle setup
- [ ] S1 — Canonical MCT WIT directory and `mct-child` world
- [ ] S2 — Scaffold: MCT default, integrated template explicit and preserved
- [ ] S3 — Build command
- [ ] S4 — Package command
- [ ] S5 — Verify command wired to recorded MCT oracle
- [ ] S6 — End-to-end echo acceptance child transcript

---

## Original prompt

```text
# SDK track — patina-sdk becomes the way developers build MCT children

You are working in the integrated Patina repository
(~/Projects/Sandbox/AI/RUST/patina). Read its AGENTS.md / CLAUDE.md first
and use THIS repo's validation gates and conventions throughout. The
consumer of this work is the standalone MCT runtime at
~/Projects/Patina/patina-mct (read-only reference; never commit there).

## Goal

The SDK's primary purpose is now: a developer builds a child app for MCT
using only the SDK — scaffold → build → package → verify — producing a
bundle that `mct-daemon children load --strict-integrity` accepts and
`mct-daemon wasm call-wit` executes. The MCT journey is the DEFAULT
journey. WASI target is wasip2 (0.2.x), deliberately pinned; wasip3 is
out of scope (wasmtime-wasi p3 is explicitly experimental) and the future
migration must be a world-definition bump invisible to child authors.

Legacy rule: integrated-Mother artifacts (existing worlds, templates, WIT
files under wit/**, the children/ directory, its builds) are legacy —
leave them bit-for-bit undisturbed. Temporary duplication of shared WIT
contracts into the SDK is ACCEPTED and preferred over moving legacy
files; record each copy's source path so future deduplication is
mechanical. If a task seems to require modifying or moving any legacy
file, STOP and report the conflict instead.

## Verified current state (grounded 2026-07-04 — re-verify, don't trust)

- `sdk/patina-sdk` is consumed by MCT at a pinned git rev with
  `features=["manifest"]` only (manifest.rs = the child.toml contract).
  Do not change the manifest contract in this track.
- `sdk/template` exists but targets integrated-Mother contracts
  (`patina:records/transform` export; `sdk/patina-sdk/wit/world.wit`
  imports wasi:keyvalue / patina:config / patina:records). A child built
  from it fails closed on MCT.
- MCT's actually-hosted child surface (from patina-mct's wasm host
  adapters): `wasi:logging/logging@0.1.0`, `patina:measure/measure@0.1.0`,
  `patina:git/git@0.1.0`, and selected WASI p2 imports gated by explicit
  directory preopens. Children export their own control interface (e.g.
  `patina:slate/control@0.1.0`).
- WIT files are scattered (wit/toys/**, sdk/patina-sdk/wit/,
  sdk/template/wit/); packaging/ contains homebrew only — no child
  bundle tooling. Locate how existing release bundles (e.g.
  slate-manager's) are built today and document it in the spec file.
- `sdk/patina-sdk-legacy`, `sdk/template-legacy`: out of scope entirely.

## Design decisions

1. **SDK-owned MCT world, the single source for child authors.** Define
   `patina:mct/child@0.1.0` (name per repo convention) in a NEW canonical
   directory (e.g. sdk/patina-sdk/wit/mct/). Imports = EXACTLY MCT's
   hosted surface above, nothing more; the wasip2 pin lives here. Copy
   shared contracts (patina:measure, patina:git, wasi:logging) into the
   canonical location; record source paths; touch no originals.
2. **MCT scaffold is the default.** `new child` scaffolds an MCT-world
   child with a manifest matching `patina_sdk::manifest` exactly. The old
   integrated-Mother template remains available under an explicit name
   (e.g. --template integrated); its output stays byte-identical to
   today.
3. **Journey commands** (attach to this repo's existing CLI per its
   conventions — discover, don't invent a parallel binary if one fits):
   - build: wraps the component toolchain (cargo component / wit-bindgen).
   - package: emits the exact bundle MCT verifies — package-relative
     artifact path per the manifest, `child.toml.sha256` and
     `<artifact>.sha256` sidecars. The historical "flattened wasm"
     mistake must be unproducible by construction.
   - verify: runs the ORACLE — invoke the mct-daemon binary built in S0b:
     `mct-daemon children load <bundle> --strict-integrity`, requiring
     loaded=1 failed=0 verified=true. "SDK says valid" and "Mother
     accepts" must be the same claim.
4. **End-to-end proof is the definition of done.** A child scaffolded,
   built, and packaged by the SDK passes the oracle AND executes via
   `mct-daemon wasm call-wit` returning a real result. Keep the minimal
   echo child in-repo as the living conformance fixture.

## Tasks

- S0: Housekeeping and oracle setup.
  a) Save this prompt verbatim as the track's spec/task file per this
     repo's convention (checklist header); commit before code. Re-verify
     the current-state section; correct the file where reality differs
     and say so.
  b) Resolve the MCT oracle yourself — no operator setup expected:
     - Locate the patina-mct checkout: use $MCT_CHECKOUT if set,
       otherwise the default ~/Projects/Patina/patina-mct. Verify it is
       a git repo whose README identifies Patina MCT.
     - Record in the spec file: the resolved path, its current commit
       hash, and its branch.
     - Build the oracle: `cargo build -p mct-daemon` in that checkout
       (read-only usage — build artifacts only, never commit or modify
       anything there). Smoke-test it:
       `<checkout>/target/debug/mct-daemon version` must print a version.
     - Record the binary path in the spec file; S5/S6 must invoke that
       recorded path, not a rebuilt guess.
     - If the checkout is missing, not MCT, or the build fails: STOP and
       report exactly what you found — that is the only case that goes
       back to the operator.
- S1: Canonical MCT WIT directory + `mct-child` world (copies with
  recorded provenance; legacy untouched). Document how existing release
  bundles are built today.
- S2: Scaffold — MCT default, integrated template preserved under an
  explicit name with byte-identical output.
- S3: Build command.
- S4: Package command (sidecars, package-relative layout).
- S5: Verify command wired to the S0b oracle; add to this repo's CI if a
  patina-mct checkout is available there, else document as a local gate.
- S6: End-to-end acceptance child (echo): scaffold → build → package →
  oracle → `wasm call-wit` execution; record the full transcript in the
  spec file.

## Working discipline

Read code before writing code. Scalpel commits, named-file staging, no
attribution footers or AI/tool branding anywhere, no history rewrites.
This repo's own validation gates green after every commit. Stop at a task
boundary if context runs low; the spec file on disk is the source of
truth. Final summary: commits, tasks done/remaining, the S6 transcript,
and any MCT-side changes found necessary but NOT made (report them for
the operator; e.g. adapter tweaks around the WASI preopen subset).
```

---

## S0 re-verification

Date: 2026-07-04.

Repository instructions read: `AGENTS.md`, `CLAUDE.md`, `.pi/skills/patina-mother-system/SKILL.md`, `.pi/skills/patina-slate-code/SKILL.md`, and relevant `layer/core/*` guidance.

Initial integrated Patina repo state before this track's files:

```text
branch: patina
head: 9a9347db chore(sdk): lock manifest dependencies
pre-existing dirty/untracked paths:
 M AGENTS.md
 M layer/slate/events.jsonl
 M src/interface/internal/launcher.rs
?? .patina/skills/
?? layer/sessions/20260627-083537-783605000.md
?? layer/slate/work/clean-pi-tmux-attach/
```

Those paths are outside this track and must not be staged by this work.

Verified current-state corrections:

- MCT consumes `patina-sdk` at git rev `5106ef0feb55aef7c4c4bc43aad49237768a32c2`, `default-features = false`, `features = ["manifest"]`, in `/Users/nicabar/Projects/Patina/patina-mct/crates/mct-daemon/Cargo.toml`.
- `mct-daemon` imports `patina_sdk::manifest` and converts SDK manifest fields into MCT domain fields in `/Users/nicabar/Projects/Patina/patina-mct/crates/mct-daemon/src/children.rs`.
- `sdk/template` targets integrated-Mother records contracts: `sdk/template/src/lib.rs` implements `exports::patina::records::transform::Guest`, and `sdk/template/wit/world.wit` exports `patina:records/transform`.
- Correction to prompt: `sdk/patina-sdk/wit/world.wit` exists and imports integrated-Mother records/config/keyvalue-style contracts; `sdk/patina-sdk/wit/child/child.wit` also exists and imports the broader integrated Mother child surface. Neither is the new MCT-only child-author world requested here.
- MCT's hosted WIT import surface is verified in `/Users/nicabar/Projects/Patina/patina-mct/crates/mct-daemon/src/wasm.rs`: `wasi:logging/logging@0.1.0`, `patina:measure/measure@0.1.0`, `patina:git/git@0.1.0`, and selected `wasi:filesystem/*@0.2.3` imports only when explicit preopens are configured.
- `packaging/` contains only `homebrew/`; no child bundle tooling exists there.
- Existing external child release bundles are built outside this repo:
  - Slate manager: `/Users/nicabar/Projects/Patina/patina-child-slate/RELEASING.md` documents `cargo component build --release`, tag-triggered release assets, `.wasm`, `.wasm.sha256`, `child.toml`, and `child.toml.sha256`. Its current manifest still references a `wasm32-wasip1` artifact path.
  - Watcher system: `/Users/nicabar/Projects/Patina/patina-child-watcher-system/RELEASING.md` documents per-child tags, `cargo component build --release -p ...`, and release assets `.wasm`, `.wasm.sha256`, `child.toml`, `child.toml.sha256`, plus `checksums.txt`. It also currently uses `wasip1` release paths.
- `sdk/patina-sdk-legacy` and `sdk/template-legacy` are out of scope and remain untouched.

MCT oracle:

```text
resolved_checkout: /Users/nicabar/Projects/Patina/patina-mct
branch: patina
commit: 181a98a59274f7fa5121c1486a882e738a238e26
readme_probe: README.md line 1 is "# Patina MCT"
build_command: cargo build -p mct-daemon --manifest-path /Users/nicabar/Projects/Patina/patina-mct/Cargo.toml
smoke_command: /Users/nicabar/Projects/Patina/patina-mct/target/debug/mct-daemon version
smoke_output: mct-daemon 0.1.0
oracle_binary: /Users/nicabar/Projects/Patina/patina-mct/target/debug/mct-daemon
mct_checkout_status_before_after_build: only pre-existing ?? brew-noncore-report.html
```
