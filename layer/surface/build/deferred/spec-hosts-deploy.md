# Spec: Hosts and Deploy Architecture

**Status:** Ideas (Design Only)

**Purpose:** Define how Patina manages deployment targets (hosts) and deploys projects to them.

**Origin:** Session 20251225 - Design discussion about persistent servers, remote docker-compose workflows, and the relationship between projects and infrastructure.

---

> **Why Deferred:**
>
> This is design exploration for the "awaken" layer - the shipping/deployment story.
> No implementation exists yet.
>
> **Reason:**
> - Need to clarify three-layer architecture (mother/patina/awaken) first
> - Current yolo command generates containers but doesn't deploy
> - Deploy story depends on understanding target use cases
>
> **Resume trigger:** When spec-three-layers.md clarifies awaken responsibilities and user has deployment needs.

---

## Core Model

Two concepts only:

```
┌─────────────────────────────────────────────────────────────┐
│                     MOTHERSHIP (Mac)                         │
│                                                              │
│  ~/.patina/                                                  │
│    ├── persona/         (cross-project knowledge)           │
│    ├── hosts.yaml       (YOUR infrastructure)               │
│    ├── registry.yaml    (known repos)                       │
│    └── cache/           (models, cloned repos)              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ manages
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                        PROJECTS                              │
│                    (anywhere, any platform)                  │
│                                                              │
│  Each project may have a deploy target (host + path)        │
│  Projects are platform-agnostic                             │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Two Levels of Configuration

### Mother Level: What Hosts Do I Have?

```yaml
# ~/.patina/hosts.yaml
unraid:
  address: root@unraid
  type: linux

hetzner:
  address: root@vps.example.com
  type: linux

macbook:
  address: localhost
  type: macos

cloudflare:
  type: cloudflare-workers
  # Different deployment mechanism
```

This is your **infrastructure inventory**. Like `~/.ssh/config` but for Patina.

### Project Level: Where Does THIS Project Deploy?

```toml
# project/.patina/config.toml
[deploy]
host = "unraid"
path = "/mnt/vault/appdata/ethereum-rust"
```

This is **project-specific**. Optional - not every project deploys somewhere.

---

## Relationship

```
Mother                              Projects
(your infrastructure)               (individual apps)

┌─────────────────┐
│ hosts.yaml      │
│                 │
│ • unraid ───────┼──────► ethereum-rust deploys here
│ • hetzner ──────┼──────► my-website deploys here
│ • macbook ──────┼──────► patina itself runs here
│ • cloudflare    │
└─────────────────┘
```

**Mother = what you have**
**Project = what goes where**

---

## Projects Are Just Projects

No types. Capabilities are detected from what exists:

| If project has... | Then you can... |
|-------------------|-----------------|
| `src/`, code files | `patina scrape code`, build, test |
| `docker-compose.yml` | `patina deploy` to a host |
| `layer/` | `patina scrape layer`, query patterns |
| `.patina/vault.age` | `patina secrets` management |
| `[deploy]` config | deploy to configured host |

A project can have all, some, or none of these. `patina init .` works for anything.

---

## Example Projects

```
patina/                    ethereum-rust/         notes/
├── src/                   ├── docker-compose.yml ├── layer/
├── layer/                 ├── .patina/           └── .patina/
├── .patina/               │   ├── config.toml
│   └── (no deploy)        │   │   [deploy]
│                          │   │   host = unraid
│                          │   └── vault.age
│                          └── layer/ (optional)
│
└── Code project           └── Service project    └── Docs only
    (builds locally)           (deploys to host)      (no deploy)
```

---

## Commands

### Host Management (Mother)

```bash
# Add a host to your infrastructure
patina host add unraid --address root@unraid

# List known hosts
patina host list

# Remove a host
patina host remove old-vps

# Check host connectivity
patina host check unraid
```

### Deploy (Project)

```bash
# Deploy to configured host
patina deploy

# Deploy to specific host (override)
patina deploy hetzner

# Pull changes from host back to project
patina deploy --pull

# Dry run - show what would happen
patina deploy --dry-run
```

---

## Deploy Flow

```
patina deploy
     │
     ├── 1. Check for uncommitted changes (warn/fail)
     │
     ├── 2. Read [deploy] from .patina/config.toml
     │
     ├── 3. Look up host in ~/.patina/hosts.yaml
     │
     ├── 4. If .patina/vault.age exists:
     │       └── Decrypt secrets → generate .env
     │
     ├── 5. Sync project files to host:path
     │       └── rsync (exclude .git/, .patina/data/, etc.)
     │
     ├── 6. On host: docker-compose up -d (if compose file)
     │
     └── 7. Report status
```

---

## Pull Flow (Bidirectional)

```
patina deploy --pull
     │
     ├── 1. Backup remote state to temp
     │
     ├── 2. rsync host:path → project/
     │       └── (exclude volumes, data, .env)
     │
     ├── 3. git diff - show changes
     │
     ├── 4. Prompt: commit these changes?
     │
     └── 5. git commit if confirmed
```

This captures remote edits back into git.

---

## Secrets Integration

```
Project                              Host
.patina/vault.age     ──decrypt──►   .env (plaintext, chmod 600)
(encrypted)                          (or .env.age for extra security)
```

Options:
1. **Decrypt on deploy** - .env is plaintext on host (simple)
2. **Encrypted on host** - .env.age on host, decrypt at container start (secure)

---

## Fits Existing Pipeline

```
                    GIT (source of truth)
                            │
               ┌────────────┴────────────┐
               ▼                         ▼
         code, layer/              docker-compose.yml
               │                         │
               ▼                         ▼
            scrape ◄─────────────────────┘
               │
               ▼
          SQLite DB
               │
      ┌────────┴────────┐
      ▼                 ▼
   oxidize           assay
      │                 │
      ▼                 ▼
    scry ◄──────────────┘        patina deploy
      │                               │
      ▼                               ▼
  LLM Frontend                   Host (Unraid, etc.)
      │                               │
      ▼                               │
  git commit ◄────────────────────────┘
      │                          (pull changes back)
      └──────────────────────────────►
```

---

## Values Alignment

| Value | How This Honors It |
|-------|-------------------|
| **unix-philosophy** | deploy does one thing: sync project to host |
| **git as memory** | Project is git-tracked. Deploy syncs FROM git. |
| **local-first** | Mother on Mac. Hosts are just targets. |
| **escape hatches** | Can always SSH directly. rsync is standard. |
| **platform-agnostic** | Projects work on any platform (Mac, Linux, etc.) |

---

## Implementation Plan

### Phase 1: Host Registry

| Task | Effort |
|------|--------|
| Define hosts.yaml schema | ~20 lines |
| `patina host add/list/remove` commands | ~150 lines |
| Store in ~/.patina/hosts.yaml | ~50 lines |

### Phase 2: Deploy Command

| Task | Effort |
|------|--------|
| Read [deploy] from project config | ~30 lines |
| Implement rsync-based sync | ~100 lines |
| Secrets → .env generation | ~50 lines |
| docker-compose up on remote | ~50 lines |

### Phase 3: Pull (Bidirectional)

| Task | Effort |
|------|--------|
| `patina deploy --pull` | ~100 lines |
| Diff and commit flow | ~80 lines |

---

## Open Questions

1. **Multiple hosts per project?** (staging vs production)
2. **Host groups?** (deploy to all hosts in group)
3. **Non-docker deployments?** (systemd, cloudflare workers, etc.)
4. **Rollback?** (git revert + redeploy, or something more?)

---

## Exit Criteria

| Criteria | Status |
|----------|--------|
| spec-hosts-deploy.md written | [x] |
| Host registry implemented | [ ] |
| Deploy command works for docker-compose | [ ] |
| Secrets decrypt on deploy | [ ] |
| Pull flow captures remote changes | [ ] |
