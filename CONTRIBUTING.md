# Contributing to Patina

> Trust is earned, not assumed. Quality over quantity.

## The Problem We're Solving

Open source in 2026 faces a new challenge: AI-generated contributions that are syntactically correct but context-free. PRs that "fix" nothing. Issues that waste maintainer time. Drive-by contributions with no follow-through.

Patina exists to capture and surface project context. We practice what we preach.

## Our Model

**Not closed** - contributions welcome.
**Not wide open** - quality gates required.

### Trust Ladder

| Level | Who | What You Can Do |
|-------|-----|-----------------|
| New Contributor | First PR | High bar: linked issue, clear rationale, all CI passes |
| Proven Contributor | Track record | More latitude, faster reviews |
| Maintainer | Earned trust | Can merge to integration branch |

Trust is earned through quality contributions over time, not granted upfront.

### Branch Flow

```
main     <- Release branch. Protected. Maintainer merges only.
   |
patina   <- Integration branch. All PRs target here.
   |
feature  <- Your branch. Fork and PR to patina.
```

**Never PR directly to main.** The `patina` branch is where integration happens.

## Session Transparency

Patina tracks development sessions - the prompts, goals, activity logs, and decisions that lead to code. These sessions are committed to `layer/sessions/` and become project memory.

**If you use Patina's session workflow, your sessions become part of project history.**

This is intentional. We practice transparent AI-assisted development. The "why" behind code is as valuable as the code itself.

If you're not comfortable with this visibility, you can contribute without sessions (manual commits). But we encourage embracing the transparency - it's how we build trust and context.

## Quality Bar

### What We Expect

- **Linked issue or clear rationale** - Why does this change exist?
- **One PR = one purpose** - No scope creep, no "while I was here..."
- **All CI passes** - `cargo fmt`, `cargo clippy`, `cargo test`
- **Project context** - Show you understand how this fits

### What We Don't Accept

- PRs without context ("fixed typo" with no issue)
- Generic AI-generated contributions (detectable by lack of project knowledge)
- Scope creep ("improved X" that touches unrelated code)

### Before Submitting

```bash
# Run the same checks CI runs
./resources/git/pre-push-checks.sh
```

## How to Contribute

### 1. Find or Create an Issue

Don't start with code. Start with context:
- Search existing issues
- If none exists, open one explaining the problem/feature
- Wait for maintainer acknowledgment on larger changes

### 2. Fork and Branch

```bash
git clone https://github.com/YOUR-USERNAME/patina.git
cd patina
git checkout patina              # Work from integration branch
git checkout -b your-feature     # Create your branch
```

### 3. Make Your Changes

- Small, focused commits
- Clear commit messages (what and why)
- Run tests locally

### 4. Submit PR to `patina` Branch

- Reference the issue
- Explain what changed and why
- Be prepared for review feedback

## What's Coming

We're building toward a model where Patina itself is the contribution interface:

- **Contributor registration** - `patina contributor register`
- **Patina-signed PRs** - `patina pr create` ensures context travels with code
- **Linkage scoring** - Contributions traced back to specs and sessions

These aren't implemented yet. For now, the quality bar is human-enforced through review.

## Getting Help

- **Questions about contributing**: Open a discussion
- **Bug reports**: Open an issue with reproduction steps
- **Feature ideas**: Open an issue explaining the problem first

## License

By contributing, you agree that your contributions will be licensed under MIT.
