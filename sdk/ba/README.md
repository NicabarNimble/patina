# BA Truths

> **For any LLM or human reading this:** This folder is the entry point for
> Patina's relationship with the Bytecode Alliance (BA), the WebAssembly
> component model, WIT, WASI, and the broader WASM ecosystem. Read this whole
> document before taking action.

## Mission

Patina's MCT (Mother-Child-Toy) plumbing is a bet that the Bytecode Alliance's
direction — WASM component model, WIT, WAC, WASI preview 2 → 3 — is the
substrate for "correct by construction" software. To make that bet seriously,
Patina must track BA continuously, deeply, and honestly.

**The principle:** *We do not invent parallel mechanisms for things BA already
solves. We track what BA is doing, learn from it, and shape MCT to align.*

## Where BA Truth Lives

**BA truth lives in Patina's existing repo system, not in this folder.**

Patina already has the infrastructure to clone, scrape, index, and search
external repositories:

- `patina repo add <url>` — clone, scrape (FTS5 + structural facts), oxidize (semantic embeddings)
- `patina repo update <name>` — git pull + rescrape
- `patina repo show <name>` — sync status, commit lag, metadata
- `patina repo list` — registry of all tracked external repos
- `patina scry "<query>" --repos <name>` — semantic + lexical search
- `patina assay <command> --repo <name>` — structural query (callers, importers, function facts)

External repos cache at `~/.patina/cache/repos/<owner>/<repo>/`. They are full
git checkouts available for direct reading.

This folder is **not a parallel knowledge store**. It is:

1. The orientation document (this README)
2. The canonical list of BA repos to track (`repos.toml`)
3. Scripts that wrap Patina's repo tools for BA-specific workflows (`scripts/`)
4. Pointers to where BA-related knowledge actually lives (beliefs, specs, repos)

## Canonical BA Repo Set

`repos.toml` lists the BA and WebAssembly repos Patina tracks. Each entry has
priority (high/medium/low) and a one-line purpose. The list is authoritative —
add to it as the Patina–BA relationship deepens.

To register every canonical repo at once:

```bash
sdk/ba/scripts/add-all.sh
```

To register only the high-priority subset:

```bash
sdk/ba/scripts/add-all.sh --priority high
```

The script is idempotent. Re-running it skips already-registered repos.

## How to Learn About BA

Once repos are registered, use Patina's existing knowledge tools:

### Semantic search across BA

```bash
# What does the component model say about resource types?
patina scry "resource types component model" --repos WebAssembly/component-model

# How does wasmtime implement linker resources?
patina scry "linker add_to_linker resource" --repos bytecodealliance/wasmtime
```

### Structural query

```bash
# Who calls a specific function in wasmtime?
patina assay callers --pattern "Linker::instance" --repo bytecodealliance/wasmtime

# What does wit-bindgen export?
patina assay functions --pattern "generate" --repo bytecodealliance/wit-bindgen
```

### Direct read

```bash
# The cache is a real git checkout — read files directly
ls ~/.patina/cache/repos/WebAssembly/component-model/design/mvp/
cat ~/.patina/cache/repos/bytecodealliance/wac/README.md
```

### Cross-reference

When you find a BA fact that informs a Patina design decision, capture it as
a belief in `layer/surface/epistemic/beliefs/`. Beliefs are Patina's first-class
knowledge primitive — searchable via `patina scry`, queryable via the belief
metrics system, and traceable to the BA source via `[[wikilinks]]` to repos.

Naming convention for BA-aligned beliefs:

- `ba-aligns-<topic>.md` — Patina aligns with BA on this topic
- `ba-extends-<topic>.md` — Patina extends BA with a Patina-specific addition
- `ba-diverges-<topic>.md` — Patina deliberately differs from BA (with reason)

## BA Skills

The actual logic lives in `sdk/ba/scripts/` — portable shell scripts any LLM
or human can invoke. Claude skill manifests in `.claude/skills/ba-*/` are
optional thin wrappers (gitignored, local-only). Other LLM runtimes can wrap
the scripts in whatever skill format they use.

Current scripts:

- `add-all.sh` — register every canonical BA repo via `patina repo add`
- More to come (status, refresh, alignment audit) — added as patterns emerge

## Constraints

- **Do not duplicate canonical sources.** Link to the source. Capture insight
  as beliefs, not as copies of BA docs.
- **Do not paper over uncertainty.** "Status unknown as of <date>" is better
  than confident wrong.
- **Do not invent storage.** Use Patina's existing repo system, belief system,
  and spec system. New storage requires explicit justification.
- **Do not over-plan structure.** This folder grows from `repos.toml` and
  `scripts/`. Refactor only when patterns are obvious.

## What This Folder Is Not

- Not a fork of BA work. We link to canonical sources via `patina repo add`.
- Not a parallel knowledge store. Patina's existing tools handle storage and search.
- Not a Patina design doc. Design lives in `layer/` and `layer/surface/build/`.
- Not a substitute for reading BA repos. It's the orientation that points you at them.

## Iteration Model

This is a living system, not a project with a deadline:

1. **Discover** — find new BA material (reading, BA meetings, BA blog, conferences)
2. **Register** — `patina repo add` to bring it into Patina's knowledge system
3. **Search** — `patina scry` and `patina assay` to learn from it
4. **Capture insight** — write a belief that grounds a Patina design decision in BA truth
5. **Refresh** — `patina repo update` periodically to stay current
6. **Update repos.toml** — add the repo to the canonical list so others get it via `add-all.sh`

## Sources to Start With

If `repos.toml` is your starting point, these are the URLs the script registers.
For human exploration outside the script:

- https://bytecodealliance.org/
- https://component-model.bytecodealliance.org/
- https://wasi.dev/
- https://docs.wasmtime.dev/
- https://github.com/bytecodealliance
- https://github.com/WebAssembly
- https://bytecodealliance.org/articles (BA blog)

---

**Now go.** Run `sdk/ba/scripts/add-all.sh` if the canonical repos aren't yet
registered. Then use `patina scry`, `patina assay`, and direct reads to learn.
Capture insight as beliefs. The folder grows from there.
