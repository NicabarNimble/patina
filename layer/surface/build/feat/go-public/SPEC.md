---
type: feat
id: go-public
status: in_progress
created: 2026-01-23
updated: 2026-01-26
sessions:
  origin: 20260123-082104
  work:
    - 20260116-105801
    - 20251216-085711
    - 20260123-050814
    - 20260125-211931
    - 20260126-060540
related:
  - layer/surface/build/deferred/spec-version-simplification.md
  - layer/surface/build/explore/anti-slop/SPEC.md
  - layer/surface/build/refactor/spec-system/SPEC.md
milestones:
  - version: "0.8.2"
    name: Version command with automation
    status: complete
  - version: "0.8.3"
    name: Spec-linked versioning
    status: complete
  - version: "0.8.4"
    name: GitHub config and branch protection
    status: complete
  - version: "0.8.5"
    name: Version safeguards
    status: in_progress
  - version: "0.9.0"
    name: Public release
    status: pending
current_milestone: "0.8.5"
---

# feat: Go Public

> Make Patina ready for open source contributions with quality gates for the slop era.

**Goal:** Open the repo publicly with defenses against low-quality contributions. Trust is earned, not assumed. CI is surgical, not bloated. Patina dogfoods its own signal-over-noise principles.

---

## The Problem

Open source in 2026 is drowning in AI-generated slop:
- PRs that "fix" nothing or introduce subtle bugs
- Issues that waste maintainer time with hallucinated problems
- Drive-by contributions with no context or follow-through

Traditional open source assumed good faith and human effort. That era is over.

## The Model

**Not closed** (like old SQLite) - contributions welcome.
**Not wide open** (like traditional GitHub) - quality gates required.

**Trust ladder:**
1. **New contributors** - High bar. PRs require linked issue, clear rationale, passing CI.
2. **Proven contributors** - Earned through track record. More latitude, faster reviews.
3. **Maintainers** - Can merge to main. Responsible for quality.

**Dogfood Patina:** Use Patina's own linkage/signal tools on the repo. Demonstrate the discipline we advocate.

---

## Reality Check

What EXISTS vs. what needs TO BE BUILT.

### EXISTS (can use today)

| Component | Location | Status |
|-----------|----------|--------|
| Forge abstraction | `src/forge/` | ✓ ForgeReader, ForgeWriter traits |
| GitHubWriter | `src/forge/writer.rs` | ✓ Uses `gh` CLI |
| CI test workflow | `.github/workflows/test.yml` | ✓ Runs tests |
| release-plz workflow | `.github/workflows/release-plz.yml` | ✗ Broken (9 failures) |
| Pre-push checks | `resources/git/pre-push-checks.sh` | ✓ fmt, clippy, test |
| `.patina/` structure | `.patina/` | ✓ config.toml, versions.json |

### NEEDS TO BE BUILT (code)

| Component | Effort | Notes |
|-----------|--------|-------|
| `patina version show` | Small | Read Cargo.toml + state file |
| `patina version milestone` | Medium | Update files, git tag, history |
| `patina version phase` | Medium | Same + phase transition logic |
| `.patina/version.toml` | Small | State schema |
| Session version integration | Small | Hook into session-end prompt |
| `patina contributor register` | Medium | New command, hash generation |
| `patina contributor verify` | Small | Check contributors.json |
| `patina pr create` | Medium | Extend ForgeWriter, signature logic |
| `patina pr push` | Medium | Re-sign, update PR body |
| `patina pr verify` | Medium | Verify signature in CI |
| `.patina/contributors.json` | Small | Schema + read/write |
| Signature logic | Medium | Hash computation, embed/extract |
| Session contributor field | Small | Update session-start script, source from git/gh |

### NEEDS TO BE CREATED (docs)

| File | Status |
|------|--------|
| `go-public/git-history-audit.md` | ✓ Created |
| `go-public/versioning-policy.md` | ✓ Created |
| `go-public/version-history.md` | ✓ Created |
| CONTRIBUTING.md | ✗ Doesn't exist (must include session transparency consent) |
| CHANGELOG.md | ✗ Doesn't exist (or reference audit artifact) |
| PR template | ✗ Doesn't exist |
| CI workflow for PR verify | ✗ Doesn't exist |

### NEEDS TO BE REMOVED

| File | Reason |
|------|--------|
| `.github/workflows/release-plz.yml` | Replaced by `patina version` |
| `release-plz.toml` | If exists, no longer needed |

### CONFIGURATION (GitHub settings)

| Setting | Status |
|---------|--------|
| Branch protection on main | ✗ Not configured |
| Branch protection on patina | ✗ Not configured |
| Default branch = patina | ✗ Not configured |
| Repo public | ✗ Private |

### Summary

The forge abstraction exists and is the right foundation. But the contributor system and PR signing are entirely new code (~500-800 lines estimated). Docs don't exist. GitHub config not done.

---

## Branch Flow

Explicit path to main. No shortcuts.

```
main     ← protected, only maintainer merges from patina
   ↑        (triggers release)
patina   ← integration branch, PRs land here
   ↑        (review, CI, contributor verification)
feature  ← contributors fork/branch here
             (PR to patina, never to main)
```

**Rules:**
- `main` is the release branch. Protected. Only maintainer merges.
- `patina` is the integration branch. All PRs target here.
- Contributors never touch `main` directly.
- Maintainer periodically merges `patina` → `main` when ready to release.

**GitHub Configuration:**
- Branch protection on `main`: require PR, require specific reviewer (maintainer)
- Branch protection on `patina`: require PR, require CI pass
- Default branch: `patina` (so forks PR to right place)

This is like Linux kernel's `next` → `main` flow. Adds a staging layer.

---

## Contributor Verification

Friction before the gate. Not cryptographic proof, but ceremony that filters casual slop.

### The System

```
.patina/contributors.json
{
  "contributors": {
    "nicabar": {
      "registered": "2026-01-23",
      "hash": "sha256:abc123...",
      "status": "maintainer"
    },
    "new-person": {
      "registered": "2026-02-01",
      "hash": "sha256:def456...",
      "status": "contributor",
      "contributions": 0
    }
  }
}
```

### Registration Flow

1. Contributor runs `patina contributor register`
2. Generates hash from: GitHub username + email + timestamp + random salt
3. Opens PR to add themselves to `contributors.json`
4. Maintainer reviews, merges (first gate)
5. Now they can submit PRs that pass contributor check

### CI Verification

```yaml
# .github/workflows/verify-contributor.yml
- name: Check contributor registered
  run: |
    AUTHOR=$(git log -1 --format='%an')
    patina contributor verify "$AUTHOR"
```

PR fails if author not in contributors.json.

### Why This Works

- **Friction** - Must register before contributing. Slop generators won't bother.
- **Ceremony** - The act of registration signals intent.
- **Traceability** - Know who contributed, when they registered.
- **Gameable but costly** - Yes, you can fake it. But why? Maintainer still approves.

### Trust Progression

| Status | Can do |
|--------|--------|
| `pending` | Registered, awaiting first contribution |
| `contributor` | Can submit PRs, must pass full review |
| `trusted` | Lighter review, can be assigned issues |
| `maintainer` | Can merge to patina, can approve PRs |

---

## Patina-Signed PRs

PRs must be created and updated through Patina. No GitHub web UI. No raw `git push`. Patina is the interface.

### Why

The friction IS the feature. If you're not willing to:
1. Install Patina
2. Register as contributor
3. Work through Patina commands

...you're probably not a serious contributor.

### What We Verify

- **Same contributor throughout** - Can't hand off a PR mid-stream
- **Each push comes through Patina** - Can't sneak in web edits or raw git push
- **Issue linkage maintained** - PR stays connected to its purpose

### What We DON'T Freeze

- **The actual code** - It evolves during review. CI fails, you fix, you push again.

### Full Workflow

```bash
# 1. Create the PR
patina pr create --issue 42 --title "Add foo feature"
# → Verifies you're registered
# → Creates PR with initial signature
# → Signature includes: contributor + issue + diff hash + timestamp

# 2. CI runs... fails on tests

# 3. Fix locally
git add . && git commit -m "fix: address test failure"

# 4. Push the fix through Patina
patina pr push
# → Verifies same contributor as original PR
# → Updates signature with new diff hash
# → Pushes to PR branch

# 5. CI re-runs, verifies new signature
# → Signature valid? ✓
# → Same contributor? ✓
# → Passes

# 6. Repeat until CI green, then maintainer reviews
```

### Signature Block

Embedded in PR body, updated on each push:

```markdown
<!-- patina:begin -->
<!-- patina:sig:sha256:abc123def456... -->
<!-- patina:contributor:nicabar -->
<!-- patina:issue:42 -->
<!-- patina:created:2026-01-23T12:00:00Z -->
<!-- patina:updated:2026-01-23T14:30:00Z -->
<!-- patina:pushes:3 -->
<!-- patina:end -->
```

### CI Verification

```yaml
# .github/workflows/verify-pr.yml
- name: Verify Patina-signed PR
  run: |
    patina pr verify --pr ${{ github.event.pull_request.number }}
```

**Checks:**
- Signature block present
- Signature valid for current diff
- Contributor registered and matches original creator
- Issue linked

### What Gets Filtered

| Source | Result |
|--------|--------|
| GitHub web UI PR | Rejected - no signature |
| Raw `gh pr create` | Rejected - no signature |
| `git push` directly | Rejected - signature not updated |
| Web UI edit after PR created | Rejected - signature mismatch |
| Different person pushes | Rejected - contributor mismatch |
| AI agent without Patina | Rejected - no signature |
| Legit Patina workflow | Passes |

### Contributor Continuity

The PR is "owned" by the contributor who created it:
- Only they can `patina pr push` to it
- If someone else needs to take over, close and create new PR
- This prevents drive-by "fixes" that sneak in bad code

### Commands

```bash
# Create a PR
patina pr create --issue <num> --title "..."

# Push updates to existing PR (re-signs)
patina pr push

# Verify a PR (used by CI)
patina pr verify --pr <num>

# Check status of your PR
patina pr status
```

### Implementation: Forge Abstraction

`patina pr` commands use the existing forge abstraction (`src/forge/`).

**Architecture:**
```
ForgeWriter trait
├── GitHubWriter  → uses `gh` CLI
├── GiteaWriter   → would use `tea` or API (future)
└── NoneWriter    → returns "no forge configured"
```

**Extend ForgeWriter trait:**
```rust
// New methods for PR operations
fn create_pr(&self, title: &str, body: &str, base: &str) -> Result<i64>;
fn update_pr_body(&self, pr_number: i64, body: &str) -> Result<()>;
fn get_pr_body(&self, pr_number: i64) -> Result<String>;
```

**GitHub implementation wraps `gh`:**
```rust
impl ForgeWriter for GitHubWriter {
    fn create_pr(&self, title: &str, body: &str, base: &str) -> Result<i64> {
        let output = Command::new("gh")
            .args(["pr", "create", "--title", title, "--body", body, "--base", base])
            .output()?;
        // Parse PR number from output
    }
}
```

**Why `gh` CLI:**
- Already handles OAuth, token storage, enterprise GitHub
- Pattern already established in `GitHubWriter`
- `patina doctor` checks `gh auth status`
- No auth code to maintain

**Users without GitHub:**
- `NoneWriter` returns clear error: "No forge configured - cannot create PR"
- All other Patina features still work
- Future: Gitea/GitLab users get their own implementations

---

## Session Transparency

Sessions are project memory, not personal artifacts. All sessions committed, flat structure, attributed.

### The Model

```
layer/sessions/
├── 20260123-082104.md  # nicabar's session
├── 20260124-090000.md  # alice's session
├── 20260124-143000.md  # bob's session
└── ...                  # chronological, unified history
```

**Like commits:** You don't have `commits/alice/abc123` - you have commits with authors. Sessions work the same way. Contributed to the project, not segregated by owner.

### Session Metadata

Add contributor attribution to existing session format:

```markdown
# Session: feature name
**ID**: 20260123-082104
**Contributor**: nicabar          ← NEW: explicit attribution
**Started**: 2026-01-23T13:21:04Z
**LLM**: claude
**Git Branch**: patina
**Session Tag**: session-20260123-082104-claude-start
```

### Why Flat Works

1. **No collision** - Timestamp IDs unique to the second (YYYYMMDD-HHMMSS)
2. **No hierarchy** - Sessions aren't "yours" or "mine", they're the project's
3. **Chronological** - Natural sort shows project evolution across all contributors
4. **Transparent** - Anyone can see how decisions were made, by whom

### The Philosophy

Patina practices **transparent AI-assisted development**:
- Sessions show the actual conversation that led to code
- Prompts are visible - the "why" behind decisions
- Contributors opt into this transparency by using Patina

This is intentional. We're demonstrating the discipline we advocate.

### Contributor Consent

CONTRIBUTING.md must be explicit:

> **Session Transparency:** When you use Patina's session workflow, your sessions become part of project history. This includes your prompts, goals, and activity logs. This is intentional - we practice transparent AI-assisted development. If you're not comfortable with this visibility, you can contribute without using sessions (manual commits), but we encourage embracing the transparency.

Contributors who aren't comfortable self-select out. The friction works as intended.

### Multi-User Scenarios

| Scenario | Result |
|----------|--------|
| Alice and Bob start sessions same day | Different timestamps, no conflict |
| Both PR to patina | Each PR includes their session file, merges cleanly |
| Reviewer wants context | Can read contributor's session to understand "why" |
| Future maintainer | Can trace any decision back through session history |

### Implementation

- Update session-start script to add `**Contributor**` field
- Source contributor from `git config user.name` or `gh api user`
- No directory restructuring needed - keep flat

---

## Defense Layers

Multiple gates, each adding friction:

```
Layer 1: Contributor Registration
   ↓  (must install Patina, run `patina contributor register`)
Layer 2: Patina-Signed PR
   ↓  (must create PR via `patina pr create`)
Layer 3: Patina-Controlled Updates
   ↓  (must push via `patina pr push`, signature updated)
Layer 4: Branch Flow
   ↓  (must PR to patina, not main)
Layer 5: CI Checks
   ↓  (tests, clippy, fmt, signature verify, contributor verify)
Layer 6: Contributor Continuity
   ↓  (only original contributor can push to PR)
Layer 7: Human Review
   ↓  (maintainer approval required)
Layer 8: Integration
   ↓  (patina → main merge by maintainer only)
Release
```

Any layer can reject. Slop fails early. Quality contributions pass through.

**The key insight:** Requiring Patina as the interface means contributors must:
- Install the tool (friction)
- Understand the project enough to use it (context)
- Follow the workflow (discipline)
- Own their PR from start to finish (accountability)

This self-selects for serious contributors.

---

## Milestones

Version-linked outcomes for this spec. Each milestone = version bump.

| Version | Name | Status | Exit Criteria |
|---------|------|--------|---------------|
| 0.8.2 | Version command with automation | ✓ complete | `patina version show/milestone/phase` working |
| 0.8.3 | Spec-linked versioning | ✓ complete | Milestones in specs, scrape extracts them, version reads from index |
| 0.8.4 | GitHub config and branch protection | ✓ complete | Branch protection, default branch, CI passing |
| 0.8.5 | Version safeguards | → in_progress | Dirty tree check, sync check, branch check before version bump |
| 0.9.0 | Public release | ○ pending | Secrets audit, repo made public |

**Note:** Contributor system and PR signing (Phase 2 in original spec) deferred to post-0.9.0. Build after going public, before first external PR.

---

## Exit Criteria

### Phase 1: Foundation (Do Now)

Infrastructure and GitHub config - enables clean releases and proper branch flow.

**Versioning:**
- [x] Git history audit complete (`git-history-audit.md` artifact)
- [x] Fresh start version decided (v0.8.1)
- [x] Historical record documented (`version-history.md` artifact)
- [x] Versioning policy established (`versioning-policy.md`)
- [x] `patina version show` command implemented
- [x] `patina version milestone` command implemented
- [x] `patina version phase` command implemented
- [x] `.patina/version.toml` schema defined
- [ ] Version safeguards (dirty tree, sync check) - see detail below
- [x] Spec-linked versioning (milestones from spec index)

**Version Safeguards (0.8.5 detail):**

`patina version milestone` must check before proceeding:

| Check | Action | Rationale |
|-------|--------|-----------|
| Dirty tree (tracked files) | **Block** | Don't version uncommitted work |
| Behind remote | **Block** | Someone else pushed, pull first |
| Diverged from remote | **Block** | Merge conflict waiting, resolve first |
| Tag already exists | **Block** | Can't re-release same version |
| Index stale (spec newer than scrape) | **Block** | Could complete wrong milestone |

Non-blocking warnings:
| Check | Action | Rationale |
|-------|--------|-----------|
| Not on `patina` branch | **Warn** | Unusual but allowed |
| Untracked files present | **Warn** | May want to add them |
| Ahead of remote | **Allow** | Normal workflow - commit often, push later |
- [x] Remove release-plz workflow (`.github/workflows/release-plz.yml`)

**Branch Flow:**
- [x] `main` branch protected (require PR, require maintainer review)
- [ ] `patina` branch protected (require PR, require CI pass) - deferred to Phase 2
- [x] Default branch set to `patina`
- [x] CI passing on main branch

### Phase 2: Quality Gates (Post-Launch)

Contributor and PR signing system - builds after going public, before first external PR.

**Contributor System:**
- [ ] `patina contributor register` command implemented
- [ ] `patina contributor verify` command implemented
- [ ] `.patina/contributors.json` schema defined
- [ ] CI workflow to verify contributor on PR
- [ ] Session-start script adds `**Contributor**` field (verified identity)
- [ ] Contributor sourced from verified gh auth (not unverified git config)

**Patina-Signed PRs:**
- [ ] `patina pr create` command implemented
- [ ] `patina pr push` command implemented (re-signs on update)
- [ ] `patina pr verify` command implemented
- [ ] Signature block format defined
- [ ] CI workflow to verify PR signature

### Phase 3: Launch

Documentation and final checks before flipping public.

**Documentation:**
- [x] README explains what Patina is and how to install
- [x] CONTRIBUTING.md defines the trust model and quality bar
- [x] CONTRIBUTING.md includes session transparency disclosure
- [x] LICENSE clear and correct (MIT)

**Hygiene:**
- [ ] Secrets audit complete (no sensitive data in history)
- [ ] Repo made public on GitHub

---

## Release Audit

Before going public, audit git history to understand what releases *should have been*. Document this as a historical artifact, then start GitHub releases fresh.

### The Problem

release-plz was configured but broken (9 failed runs). Result: 494+ commits since v0.1.0, version still at 0.1.0. No GitHub releases created.

We need to:
1. Understand the git history
2. Find where it becomes "clean enough" to make sense
3. Document what releases would have been
4. Decide what version to start fresh at
5. Create the historical record

### Spec Artifacts

Spec folders can contain supporting files beyond SPEC.md:

```
layer/surface/build/feat/go-public/
├── SPEC.md                  # This spec
├── git-history-audit.md     # Release history analysis (TO CREATE)
```

### Git History Audit Process

**Step 1: Analyze history structure**
- When did conventional commits start?
- When did CI get stable?
- Major refactors or milestones?
- Where does the "real" project begin vs early experiments?

**Step 2: Document in artifact**
Create `git-history-audit.md` with:
- Timeline of significant commits/milestones
- Analysis of what releases would have been (feat = minor, fix = patch)
- Recommendation for "clean start" point
- Proposed version to resume at

**Step 3: Decide fresh start version**
Options:
- **Stay at 0.1.0** - Pretend nothing happened (weird)
- **Bump to 0.2.0** - Acknowledge "stuff happened" (honest)
- **Jump to 1.0.0** - Going public = stable (bold)

**Step 4: Create historical record**
Either:
- `CHANGELOG.md` with "Pre-release History" section
- Or just point to `git-history-audit.md` for archaeology

### Output

The audit artifact answers:
1. When does our history become meaningful?
2. What would the release sequence have been?
3. What version should we start fresh at?
4. Where's the historical record for anyone who asks?

### Tools

- `git-cliff` - generates changelog from conventional commits
- Manual curation - for pre-conventional or messy periods
- The artifact is the curation - not generated, analyzed

---

## CI Philosophy

**Surgical, not bloated.** Every check must earn its place.

### Required Checks
| Check | Purpose | Why it matters |
|-------|---------|----------------|
| `cargo test` | Correctness | Catch regressions |
| `cargo clippy` | Quality | Catch common mistakes |
| `cargo fmt --check` | Consistency | No style debates |

### Not Required (for now)
- Coverage thresholds (adds friction, easy to game)
- Benchmarks (noise unless specifically needed)
- Multiple OS matrix (Mac-first is fine for now)

### Future: Patina-powered checks
- Linkage score (does PR connect to spec/issue?)
- Contribution quality signals
- Pattern compliance

---

## Patina-Native Versioning

**Decision:** Replace release-plz with `patina version` command that fits our milestone-based model.

**Core Principle:** Spec is truth. Everything else derives from it.

### Why Not release-plz

release-plz is designed for:
- Conventional commits → automatic semver bumps
- Every `feat:` = minor bump, every `fix:` = patch bump

Our model (see `versioning-policy.md`):
- Milestones are immutable goals defined in specs
- Completing a milestone = releasing that version
- Spec content evolves (the "how"), but milestone goals don't change
- If a goal was wrong, create new milestone - don't edit old one

**release-plz would fight our model, not help it.**

### Source of Truth: Spec Milestones

```yaml
# In spec YAML frontmatter
milestones:
  - version: "0.8.4"
    name: GitHub config and branch protection
    status: complete
  - version: "0.8.5"
    name: Version safeguards
    status: in_progress
  - version: "0.9.0"
    name: Public release
    status: pending
current_milestone: "0.8.5"
```

**Flow:**
```
Spec YAML (source of truth)
    ↓ patina scrape layer
Database (index for fast lookup)
    ↓ patina version milestone
Updates spec + Cargo.toml atomically
```

### Versioning Enabled/Disabled (Owned vs Fork)

Versioning behavior inferred from `[upstream]` config in `.patina/config.toml`:

| Config State | Inference | Versioning |
|--------------|-----------|------------|
| No `[upstream]` section | Local/owned project | ✓ Enabled |
| `upstream.remote = "origin"` | Owned repo | ✓ Enabled |
| `upstream.remote = "upstream"` | Fork/contrib | ✗ Disabled |

**For forks:** Milestones track YOUR contribution goals (e.g., "get PR merged"), but Cargo.toml is controlled by upstream. `patina version milestone` updates spec only, not Cargo.toml.

**For owned repos:** Milestones = release versions. `patina version milestone` updates spec AND Cargo.toml atomically.

### The `patina version` Command

```bash
# Show current version and spec milestone
patina version show
# → patina 0.8.4
# → Phase 8: Go Public (milestone 4)
# → Spec: go-public v0.8.5 → Version safeguards

# Complete current spec milestone (spec-aware, atomic)
patina version milestone
# → Reads current milestone from spec (via index)
# → Marks it complete in spec YAML
# → Advances current_milestone to next pending
# → Updates Cargo.toml to milestone version (if owned repo)
# → Re-scrapes layer to sync index
# → Creates git tag

# Start new phase
patina version phase "Production Ready"
# → 0.8.x → 0.9.0
# → Same atomic updates
```

### What Gets Updated (Atomic Operation)

When `patina version milestone` runs on an owned repo:

1. **Spec YAML** - Mark current milestone `status: complete`, advance `current_milestone`
2. **Cargo.toml** - Set version to completed milestone version
3. **Layer index** - Re-scrape to sync database
4. **Git tag** - Create annotated tag with milestone name

All or nothing. No partial updates that cause drift.

### version.toml: Deprecated

Previously tracked phase/milestone state separately. Now redundant because spec is truth.

Kept for backwards compatibility but not authoritative. May be removed in future.

### Dogfooding: Patina's Own Config

```toml
# .patina/config.toml
[upstream]
repo = "NicabarNimble/patina"
branch = "main"
remote = "origin"           # We own it → versioning enabled
include_patina = true
include_adapters = true
```

### Implementation

| Component | Status | Notes |
|-----------|--------|-------|
| `patina version show` | ✓ Done | Shows Cargo.toml + spec milestone from index |
| `patina version milestone` | Needs update | Must become spec-aware |
| `is_versioning_enabled()` | To build | Check upstream.remote |
| Spec YAML update | To build | Mark complete, advance current |
| Atomic operation | To build | All updates or rollback |

### Migration from release-plz

1. ✓ Remove `.github/workflows/release-plz.yml`
2. ✓ Remove `release-plz.toml`
3. Add `[upstream]` to `.patina/config.toml`
4. Ensure spec has milestones in frontmatter
5. Use `patina version milestone` going forward

---

## Trust Model Details

### For CONTRIBUTING.md

**New contributors:**
- Fork and PR workflow
- Must link to issue or create one first
- Must explain "why" not just "what"
- All CI must pass
- Expect thorough review, possible rewrite requests

**Earning trust:**
- Quality contributions over time
- Engagement with feedback
- Understanding of project patterns
- Eventually: triage rights, then maintainer

**What we don't accept:**
- PRs without context ("fixed typo" with no issue)
- Scope creep ("while I was here I also...")
- AI-generated drive-bys (detectable by lack of project knowledge)

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-23 | in_progress | Spec created with quality gate vision |
| 2026-01-23 | in_progress | Added branch flow and contributor verification system |
| 2026-01-23 | in_progress | Added Patina-signed PRs - Patina as contribution interface |
| 2026-01-23 | in_progress | Revised PR workflow - allow iterative updates via `patina pr push` |
| 2026-01-23 | in_progress | Added forge abstraction details - uses existing ForgeWriter + `gh` CLI |
| 2026-01-23 | in_progress | Added reality check - EXISTS vs TO BUILD audit |
| 2026-01-23 | in_progress | Added session transparency - flat sessions with contributor attribution |
| 2026-01-23 | in_progress | Added release audit - git history analysis before fresh start |
| 2026-01-23 | in_progress | Versioning policy (Phase.Milestone), history audit, v0.8.1 |
| 2026-01-23 | in_progress | Replace release-plz with `patina version` command |
| 2026-01-25 | in_progress | Phased exit criteria: P1 (foundation), P2 (quality gates post-launch), P3 (launch) |
| 2026-01-25 | in_progress | CONTRIBUTING.md created, README version fixed |
| 2026-01-26 | in_progress | `patina version` command implemented (show/milestone/phase) |
| 2026-01-26 | in_progress | Added milestones to spec, designing spec-linked versioning |

---

## See Also

- [[explore/anti-slop/SPEC.md]] - Signal over noise research
- [[deferred/spec-version-simplification.md]] - Version simplification (future)
- [git-cliff](https://git-cliff.org/) - Changelog generator
- [release-plz docs](https://release-plz.dev/)
- Session 20260123-050814 - Signal over noise exploration
