---
type: feat
id: go-public
status: in_progress
created: 2026-01-23
updated: 2026-01-25
sessions:
  origin: 20260123-082104
  work:
    - 20260116-105801
    - 20251216-085711
    - 20260123-050814
    - 20260125-211931
related:
  - layer/surface/build/deferred/spec-version-simplification.md
  - layer/surface/build/explore/anti-slop/SPEC.md
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

## Exit Criteria

### Phase 1: Foundation (Do Now)

Infrastructure and GitHub config - enables clean releases and proper branch flow.

**Versioning:**
- [x] Git history audit complete (`git-history-audit.md` artifact)
- [x] Fresh start version decided (v0.8.1)
- [x] Historical record documented (`version-history.md` artifact)
- [x] Versioning policy established (`versioning-policy.md`)
- [ ] `patina version show` command implemented
- [ ] `patina version milestone` command implemented
- [ ] `patina version phase` command implemented
- [ ] `.patina/version.toml` schema defined
- [ ] Remove release-plz workflow (`.github/workflows/release-plz.yml`)

**Branch Flow:**
- [ ] `main` branch protected (require PR, require maintainer review)
- [ ] `patina` branch protected (require PR, require CI pass)
- [ ] Default branch set to `patina`
- [ ] CI passing on main branch

### Phase 2: Quality Gates (Post-Launch)

Contributor and PR signing system - builds after going public, before first external PR.

**Contributor System:**
- [ ] `patina contributor register` command implemented
- [ ] `patina contributor verify` command implemented
- [ ] `.patina/contributors.json` schema defined
- [ ] CI workflow to verify contributor on PR

**Patina-Signed PRs:**
- [ ] `patina pr create` command implemented
- [ ] `patina pr push` command implemented (re-signs on update)
- [ ] `patina pr verify` command implemented
- [ ] Signature block format defined
- [ ] CI workflow to verify PR signature

### Phase 3: Launch

Documentation and final checks before flipping public.

**Session Transparency:**
- [ ] Session-start script adds `**Contributor**` field
- [ ] Contributor sourced from git config or gh auth

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

### Why Not release-plz

release-plz is designed for:
- Conventional commits → automatic semver bumps
- Every `feat:` = minor bump, every `fix:` = patch bump

Our model (see `versioning-policy.md`):
- Phase transitions are intentional decisions
- Milestones are "I completed something meaningful"
- Not every feat commit is a version bump

**release-plz would fight our model, not help it.**

### The `patina version` Command

```bash
# Show current version and phase
patina version show
# → v0.8.1 (Go Public phase, milestone 1)

# Bump milestone within current phase
patina version milestone "Versioning policy established"
# → 0.8.0 → 0.8.1
# → Updates Cargo.toml
# → Updates version-history.md
# → Creates git tag v0.8.1
# → Optionally creates GitHub release

# Start new phase
patina version phase "Production Ready"
# → 0.8.5 → 0.9.0
# → Same automation as above
```

### What It Tracks

State stored in `.patina/version.toml`:
```toml
[version]
current = "0.8.1"
phase = 8
phase_name = "Go Public"
milestone = 1

[history]
# Points to version-history.md for full record
```

### Automation Features

1. **Cargo.toml sync** - Updates version automatically
2. **Git tagging** - Creates annotated tag with milestone description
3. **History update** - Appends to `version-history.md`
4. **GitHub release** - Optional, creates release with notes

### Session Integration

Hook into session-end workflow:
```
Session ending. You completed:
- Versioning policy established
- Git history audited
- Cargo.toml updated

Bump version? [y/N] y
Milestone description: Versioning system designed

✓ Version bumped: 0.8.0 → 0.8.1
✓ Tagged: v0.8.1
✓ History updated
```

This addresses the "fear of not keeping up" - the prompt is there when you finish meaningful work.

### Implementation

| Component | Effort | Notes |
|-----------|--------|-------|
| `patina version show` | Small | Read Cargo.toml + state file |
| `patina version milestone` | Medium | Update files, git tag |
| `patina version phase` | Medium | Same as milestone + phase logic |
| `.patina/version.toml` | Small | State schema |
| Session integration | Small | Hook into session-end |
| GitHub release | Small | Use existing ForgeWriter |

### Migration from release-plz

1. Remove `.github/workflows/release-plz.yml`
2. Remove `release-plz.toml` if exists
3. Add `.patina/version.toml` with current state
4. Use `patina version` going forward

No backwards compatibility needed - clean break.

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

---

## See Also

- [[explore/anti-slop/SPEC.md]] - Signal over noise research
- [[deferred/spec-version-simplification.md]] - Version simplification (future)
- [git-cliff](https://git-cliff.org/) - Changelog generator
- [release-plz docs](https://release-plz.dev/)
- Session 20260123-050814 - Signal over noise exploration
