---
id: spec-repo-org-namespace
status: ready
created: 2026-01-19
tags: [spec, bug-fix, repo, implementation]
references: [unix-philosophy, dependable-rust]
---

# Spec: Repository Org/Name Namespacing

**Problem:** `patina repo add` uses only repo name as identifier, causing collisions.

```bash
patina repo add https://github.com/anthropics/skills   # Registers as "skills"
patina repo add https://github.com/huggingface/skills  # FAILS - "skills" exists
```

**Solution:** Use `org/repo` as the identifier, not just `repo`.

---

## Approach

1. **Fix the code** - Use `org/repo` as key going forward (no migration code)
2. **Manual migration** - Human runs bash commands to rename dirs + edit registry
3. **Test** - Verify old repos work and new repos use correct format

**Design decision:** No short-name convenience. Always require full `org/repo`.

---

## Phase 1: Code Changes

### Files to Modify

All changes in `src/commands/repo/internal.rs`:

| Location | Current | Change To |
|----------|---------|-----------|
| Line 92 | `let (owner, repo_name) = parse_github_url(url)?` | Keep, but use `github` instead of `repo_name` below |
| Line 99 | `registry.repos.contains_key(&repo_name)` | `registry.repos.contains_key(&github)` |
| Line 107-109 | Error message uses `repo_name` | Use `github` |
| Line 117 | `repos_path.join(&repo_name)` | `repos_path.join(&github)` |
| Line 174-175 | `registry.repos.insert(repo_name.clone(), ...)` | `registry.repos.insert(github.clone(), ...)` |
| Line 176 | `name: repo_name.clone()` | `name: github.clone()` |
| Line 196-203 | Print statements use `repo_name` | Use `github` |

### Lookup Functions

These already take `name: &str` - they'll work with `org/repo` format automatically:
- `update_repo()` (line 225)
- `remove_repo()` (line 293)
- `get_repo_db_path()` (line 379)

### Display Width

In `src/commands/repo/mod.rs`, update column widths for longer names:

| Location | Current | Change To |
|----------|---------|-----------|
| Line 253 | `{:<20}` for NAME | `{:<40}` |
| Line 261-262 | `{:<20}` for NAME | `{:<40}` |
| Line 266 | `{:<20}` for NAME | `{:<40}` |
| Line 273-275 | `{:<20}` for NAME | `{:<40}` |

---

## Phase 2: Manual Migration

**CHECKPOINT: Human runs these commands after code changes are complete.**

### Step 1: Backup Registry

```bash
cp ~/.patina/registry.yaml ~/.patina/registry.yaml.backup
```

### Step 2: Rename Directories

For each repo, create org dir and move:

```bash
cd ~/.patina/cache/repos

# Create org directories
mkdir -p anthropics libsdl-org unum-cloud openai daydreamsai \
         dojoengine dustproject litecanvas google-gemini \
         livestorejs sst mthom foundry-rs danielmiessler

# Move repos into org dirs
mv skills anthropics/
mv claude-code anthropics/
mv SDL libsdl-org/
mv USearch unum-cloud/
mv codex openai/
mv daydreams daydreamsai/
mv dojo dojoengine/
mv dust dustproject/
mv game-engine litecanvas/
mv gemini-cli google-gemini/
mv livestore livestorejs/
mv opencode sst/
mv scryer-prolog mthom/
mv starknet-foundry foundry-rs/
mv Personal_AI_Infrastructure danielmiessler/
```

### Step 3: Update Registry

Edit `~/.patina/registry.yaml`:

1. Change each key from `repo` to `org/repo`
2. Update each `path:` to include org dir

Example transformation:
```yaml
# Before
skills:
  path: /Users/nicabar/.patina/cache/repos/skills
  github: anthropics/skills
  ...

# After
anthropics/skills:
  path: /Users/nicabar/.patina/cache/repos/anthropics/skills
  github: anthropics/skills
  ...
```

### Step 4: Verify

```bash
patina repo list --status
```

Should show all repos with `org/repo` format in NAME column.

---

## Phase 3: Testing

### Test 1: Existing Repos Work

```bash
patina scry "test query" --repo anthropics/skills
```

### Test 2: New Repo Uses org/repo Format

```bash
# Add a test repo
patina repo add https://github.com/some-org/some-repo

# Verify it shows as "some-org/some-repo" not just "some-repo"
patina repo list

# Clean up
patina repo remove some-org/some-repo
```

### Test 3: Collision Now Works

```bash
# These should both succeed (if you want to test)
patina repo add https://github.com/org1/testname
patina repo add https://github.com/org2/testname

# Both should appear
patina repo list | grep testname

# Clean up
patina repo remove org1/testname
patina repo remove org2/testname
```

---

## Checklist

### Phase 1: Code Changes
- [ ] Update `add_repo()` to use `github` as registry key
- [ ] Update `add_repo()` path construction to use `github`
- [ ] Update `add_repo()` print statements to use `github`
- [ ] Update `repo list` display width (20 → 40)
- [ ] Build and install: `cargo build --release && cargo install --path .`

**CHECKPOINT: Confirm code compiles before migration**

### Phase 2: Manual Migration
- [ ] Backup registry.yaml
- [ ] Create org directories
- [ ] Move repo directories into org dirs
- [ ] Edit registry.yaml (keys + paths)
- [ ] Run `patina repo list --status` to verify

**CHECKPOINT: Confirm all repos appear correctly**

### Phase 3: Testing
- [ ] Test: existing repo query works
- [ ] Test: new repo add uses org/repo format
- [ ] Test: can add two repos with same name, different orgs

---

## Data Safety

Migration preserves all data because:

| Component | Stores Absolute Paths? | Safe to Rename? |
|-----------|------------------------|-----------------|
| SQLite databases | No (relative paths) | **Yes** |
| USearch indices | No (vectors only) | **Yes** |
| Safetensors | No (weights only) | **Yes** |
| Git repos | No | **Yes** |
| `registry.yaml` | Yes | **Updated manually** |

---

## Directory Structure After Migration

```
~/.patina/
├── registry.yaml              # Keys now "org/repo" format
└── cache/
    └── repos/
        ├── anthropics/
        │   ├── skills/
        │   └── claude-code/
        ├── sst/
        │   └── opencode/
        └── ...
```
