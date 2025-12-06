# Spec: patina repo

**Status:** Complete (Phase 3c)
**Location:** `src/commands/repo/`

---

## Purpose

Manage external repos for learning and contributing. All repos become full Patina projects with `.patina/`, `layer/`, and queryable indices.

---

## Commands

### patina repo add

```bash
patina repo <url>              # Clone + scaffold + scrape
patina repo <url> --with-issues # Also scrape GitHub issues
patina repo <url> --contrib    # Fork for contribution (partial)
```

**What happens:**
1. Clone to `~/.patina/repos/<name>/`
2. Create `patina` branch
3. Full `.patina/` scaffolding
4. Scrape codebase to `patina.db`
5. Register in `~/.patina/registry.yaml`
6. (--contrib) Fork on GitHub, add remote

### patina repo list

```bash
patina repo list
```

**Output:**
```
NAME              GITHUB                        CONTRIB   DOMAINS
dojo              dojoengine/dojo               ✓ fork    cairo, ecs
bevy              bevyengine/bevy               -         rust, ecs
```

### patina repo update

```bash
patina repo update <name>      # Pull + rescrape
patina repo update --all       # Update all repos
```

### patina repo show

```bash
patina repo show <name>
```

**Output:**
```
Name: dojo
GitHub: dojoengine/dojo
Path: ~/.patina/repos/dojo
Branch: patina
Events: 24,531
Last updated: 2 hours ago
```

---

## Query Integration

```bash
# Query specific repo
patina scry "spawn patterns" --repo dojo

# Query multiple repos (planned)
patina scry "entity component" --repos dojo,bevy
```

---

## Directory Structure

```
~/.patina/
├── registry.yaml           # Tracks all repos
└── repos/
    ├── dojo/
    │   ├── .patina/
    │   │   ├── data/
    │   │   │   └── patina.db
    │   │   └── config.toml
    │   ├── layer/
    │   │   └── sessions/   # YOUR learning sessions
    │   └── <source code>
    └── bevy/
        └── ...
```

---

## Registry Schema

```yaml
# ~/.patina/registry.yaml
version: 1

repos:
  dojo:
    path: ~/.patina/repos/dojo
    github: dojoengine/dojo
    contrib: true
    fork: nicabar/dojo
    registered: 2025-11-20T10:00:00Z
    domains: [cairo, starknet, ecs]
    github_data:
      issues: true
      issue_count: 347

  bevy:
    path: ~/.patina/repos/bevy
    github: bevyengine/bevy
    contrib: false
    registered: 2025-11-22T14:00:00Z
    domains: [rust, ecs, game-engine]
```

---

## Validation Criteria

- [x] `patina repo <url>` clones, scaffolds, scrapes
- [x] `patina repo list` shows registered repos
- [x] `patina repo update <name>` pulls and rescrapes
- [x] `patina scry --repo <name>` queries repo's database
- [x] Registry persists in `~/.patina/registry.yaml`
- [ ] `--contrib` fork mode (partial - gh cli dependency)
- [ ] `repo update` calls github scraper when issues present
