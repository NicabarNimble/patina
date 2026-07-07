# Session 3 — Slide Deck: Allium, From a Sticky Note to the Deep End

Full slides + speaker notes for the language deep dive. 36 slides,
90–120 minutes. One running example (a library book) is grown slide by
slide from a single English sentence into a checked, analysed,
obligation-generating spec — then the same shapes are shown at production
scale in `mother-lifecycle.allium` and `mct-product-map.allium`, and the
deck ends in the toolchain's deep end.

**Every snippet on these slides is real.** The running example is
[`library.allium`](library.allium) in this directory — `check` and
`analyse` clean, 18 obligations — and every diagnostic and finding shown
was produced live by `allium 3.5.0`. Where a slide says "the tool stays
silent," that was tested too.

**Legend** — each slide has three parts:
- **SLIDE**: what's on screen (keep it that sparse; the notes carry the rest).
- **SAY**: speaker script (paraphrase freely, keep the beats).
- **DO** (some slides): a live command to run. Practice these; the deck's
  credibility rests on the terminal, not the bullets.

Timing marks (⏱) are cumulative targets for a 100-minute run.

---

## Act I — Extremely simple (slides 1–9) · one sticky note

---

### Slide 1 — Title ⏱ 0:00

**SLIDE**

> **Allium: from a sticky note to the deep end**
> One behavior, one hour, five levels deeper each time.
> Session 3 of the Allium curriculum.

**SAY**
Today has one trick, repeated: we take a behavior you could write on a
sticky note, and we keep asking "and what exactly do you mean by that?"
Every time the sticky note can't answer, Allium gives us one more
construct. By the end, the same ladder takes us into the specs that run
this team's actual product. Nothing on these slides is hypothetical — every
snippet ran through the real CLI, and I'll run the important ones live.

---

### Slide 2 — A behavior in English ⏱ 0:02

**SLIDE**

> *"A member can borrow a book if it's on the shelf."*
>
> Simple. Done. Ship it?

**SAY**
Everyone understands this sentence. Now: what happens if the book is
already borrowed — error, queue, silence? Who is recorded as having it? For
how long? What happens when time runs out — and *who finds out*? The
sentence hides at least four decisions. Prose always does. It's not that
prose is bad — it's that prose can't *refuse to be ambiguous*. We want a
medium that argues back.

---

### Slide 3 — The same sentence in Allium ⏱ 0:05

**SLIDE**

```allium
rule BorrowBook {
    when: MemberBorrowsBook(member)
    requires: book.status = on_shelf
    ensures: book.status = borrowed
}
```

> Read it aloud. It's still almost English.

**SAY**
This is the whole language in miniature. *When* this happens, *provided*
this is true, *ensure* this becomes true. Trigger, guard, effect. If you
can read an if-statement you can read this — that's deliberate. Allium's
bet is that the spec must stay readable by three audiences at once: humans,
LLM agents, and a checking tool. Everything else in the language is this
shape, decorated.

---

### Slide 4 — Anatomy of a rule ⏱ 0:08

**SLIDE**

```text
rule <Name> {
    when:     <the event that triggers it>      ← exactly one
    requires: <guards — ALL must hold>          ← zero or more
    ensures:  <effects — state changes + emitted events>
}
```

> `when` = trigger · `requires` = guard · `ensures` = effect

**SAY**
Three keywords, three jobs. Two things to burn in now, because everything
later leans on them. One: `requires` lines are ANDed — every guard must
hold, so case analysis means writing *several rules* for one trigger, not
stacking conditionals inside one rule. Two: `ensures` can do two different
things — change state, or *emit an event* other rules and systems can
observe. Hold onto that second one; it becomes the star of a plot twist
around slide 21.

---

### Slide 5 — Where does `book.status` live? Entities. ⏱ 0:11

**SLIDE**

```allium
entity Book {
    status: on_shelf | borrowed
    title: String
}

entity Member {
    name: String
}
```

> An entity = identity + state. The `a | b` field is an inline enum —
> a state machine waiting to happen.

**SAY**
Entities are the nouns. The interesting field is `status` — an inline
enum. The moment you write one, you've made a claim: *this thing is always
in exactly one of these states*. The tool will hold you to it — it will
notice states nothing can reach or leave. We'll watch it do exactly that,
to me, on slide 19 — I did not stage that one; the tool genuinely caught
the deck's first draft.

---

### Slide 6 — Second rule: a state machine appears ⏱ 0:14

**SLIDE**

```allium
rule ReturnBook {
    when: MemberReturnsBook(member)
    requires: book.status = borrowed
    ensures: book.status = on_shelf
}
```

```text
        BorrowBook ──────►
on_shelf              borrowed
        ◄────── ReturnBook
```

**SAY**
Two rules, and we have a state machine without ever saying "state machine."
This is the idiom for transitions in Allium v3: an enum status field plus
one rule per edge. No transition-table syntax to learn — the edges *are*
the rules. This scales: the daemon lifecycle spec we ship in this repo is
exactly this pattern with five states and a dozen edges.

---

### Slide 7 — What we did NOT say ⏱ 0:17

**SLIDE**

> Nothing about: a database · a `books` table · an HTTP endpoint ·
> a `BorrowService` class · a button
>
> **Behavioral specification: WHAT, not HOW.**

**SAY**
Notice everything absent. That's not laziness, it's the core design rule.
The moment a spec names a table, it dies twice: it goes stale when the
implementation churns, and it starts *competing* with the code as a second
source of truth. The spec owns intent; the code owns mechanism. There's a
litmus test you'll use forever: *could we swap the storage engine without
editing this line?* If no — it doesn't belong in the spec. Session 3's
refactor exercise makes you hunt violations of this by hand.

---

### Slide 8 — The tool, first contact ⏱ 0:20

**SLIDE**

```bash
$ allium check library.allium
{ "diagnostics": [], "findings": [] }     # exit 0
```

> A spec that a machine has read and not objected to.

**DO**
Run it live on `slides/library.allium` (the finished example — Act I's
fragments grow into it).

**SAY**
This is the smallest but most important habit of the whole course: after
*every* edit, `allium check`. It's fast, it's JSON, it exits nonzero on any
diagnostic — which is why both of this team's repos run it in CI. From this
slide on, the green check is our metronome: each new construct we add, we
re-run it.

---

### Slide 9 — Act I recap: the whole beginner's language ⏱ 0:22

**SLIDE**

> - `entity` — nouns with state
> - `a | b` enum field — allowed states
> - `rule` = `when` / `requires` / `ensures` — behavior
> - `allium check` — the metronome
>
> You can already read 60% of every spec in both repos.

**SAY**
That's the "extreme simple" end. Genuinely — with these four ideas you can
open `mother-lifecycle.allium` and follow most of it. The rest of this deck
is what happens when reality asks questions this much language can't
answer. Each act, reality gets meaner.

---

## Act II — The language grows with you (slides 10–18) · reality asks questions

---

### Slide 10 — "What if the book is already out?" Case analysis. ⏱ 0:25

**SLIDE**

```allium
rule BorrowBook {
    when: MemberBorrowsBook(member)
    requires: book.status = on_shelf
    ensures: book.status = borrowed
    ...
}

rule BorrowRefusedWhenTaken {
    when: MemberBorrowsBook(member)
    requires: not book.status = on_shelf
    ensures: BorrowRefused(book: book, member: member, reason: "not-on-shelf")
}
```

**SAY**
Same trigger, two rules, guards that partition the cases — this is Allium's
if/else. Two things to notice. First: the refusal is *explicit*. Deny is a
specified outcome with its own event, not the absence of a rule. This team
has that as product doctrine — the MCT product map literally has a
`DenyByDefault` invariant. Second: `BorrowRefused(...)` is an *emitted
event* carrying data. Effects aren't just field assignments.

---

### Slide 11 — "Who has it?" Attribution and optional fields. ⏱ 0:29

**SLIDE**

```allium
entity Book {
    status: on_shelf | borrowed
    title: String
    borrowed_by: String?      ← optional: only meaningful while borrowed
    due_at: Timestamp?
}
```

```allium
    ensures: book.borrowed_by = member.name    -- in BorrowBook
    ensures: book.borrowed_by = none           -- in ReturnBook
```

**SAY**
The `?` marks fields that don't always exist. On return we set them back to
`none` — the spec forces you to decide the *whole* lifecycle of a fact, not
just its creation. Deeper version of this idea, for later: the MCT specs
have fields that are conditional on state — `route_taken` exists *only
when* the outcome was executed. The type system encoding "a denied call has
no route." Same instinct, higher stakes.

---

### Slide 12 — "For how long?" Config. ⏱ 0:32

**SLIDE**

```allium
config {
    loan_period: Duration = 21.days
}

    ensures: book.due_at = now + config.loan_period
```

**SAY**
Numbers with names. `21.days`, `5.seconds` — durations are first-class, and
they live in a `config` block, not inline in rules, because "how long" is a
*policy decision* someone will want to change without re-deriving the
behavior. The daemon spec does exactly this with its five-second stop
timeout.

---

### Slide 13 — "And when time runs out?" Temporal rules. ⏱ 0:36

**SLIDE**

```allium
rule BookGoesOverdue {
    when: _: Book.due_at <= now
    requires: book.status = borrowed
    ensures: BookOverdue(book: book, borrower: book.borrowed_by)
}
```

> `when: _:` — no actor. Time itself is the trigger.

**SAY**
Until now every trigger was someone *doing* something. This trigger is a
condition becoming true. The `_` says: no one performs this — the world
does. This is how every timeout, expiry, and deadline in the real specs
works: a config duration, a `Timestamp?` deadline field set by one rule,
and a temporal rule that fires on it. And keep one eye on the `ensures`
here — it only *emits an event*, it doesn't touch `status`. That's not an
accident, and the reason why is the best five minutes of this deck, in
Act III.

---

### Slide 14 — "Says who?" Where events come from: surfaces. ⏱ 0:41

**SLIDE**

```allium
surface LibraryDesk {
    facing member: Member
    context book: Book

    provides:
        MemberBorrowsBook(member)
        MemberReturnsBook(member)
            when book.status = borrowed
}
```

> No `provides` → `allium.rule.unreachableTrigger`. Every event needs a door.

**DO**
Delete a `provides:` line, run `allium check`, show the diagnostic, undo.

**SAY**
So far, events fell from the sky. A `surface` is a door into the system: it
declares *which actor* can inject *which triggers*, optionally guarded —
here you can only attempt a return while the book is out. The tool enforces
closure: a rule listening for an event that no surface provides and no rule
emits gets flagged. Watch. [demo] This diagnostic is the single most common
one you'll hit writing your first real spec — now you know what it means:
you built a room with no door.

---

### Slide 15 — "Who may SEE what?" Exposure. ⏱ 0:45

**SLIDE**

```allium
surface LibraryDesk {
    facing member: Member
    context book: Book

    exposes:
        member.name
        book.status
        book.title
        book.due_at
        -- note what's absent: book.borrowed_by
}
```

**SAY**
Surfaces have a second job: `exposes` is an allowlist of what this actor
may *see*. I deliberately left out `borrowed_by` — a member at the desk can
see a book is out, but not by whom. Privacy as a spec construct, checkable
and reviewable, instead of a code-review vigilance problem. This idea at
production scale is one of the crown jewels of the MCT product map — coming
on slide 27.

---

### Slide 16 — Nouns that aren't things: `value` vs `entity` ⏱ 0:48

**SLIDE**

```allium
value TraceContext {          -- from mct-product-map.allium
    trace_id: String
    span_id: String
}
```

> `entity` = identity + state, rules act on it.
> `value` = immutable data shape, no identity — it's *carried*, not tracked.

**SAY**
Quick but important distinction as specs grow: a `Book` is *tracked* — it
has identity and a lifecycle. A trace context is just *data* — two strings
that travel together. Declaring it a `value` tells the reader "nothing
transitions here" and keeps entities from bloating into god-objects — one
of the anti-patterns you'll hunt in the exercise.

---

### Slide 17 — Naming predicates: derived fields ⏱ 0:51

**SLIDE**

```allium
entity Book {
    status: on_shelf | borrowed
    ...
    is_available: status = on_shelf     ← derived field
}

    requires: not book.is_available     -- reads like the intent
```

**SAY**
A derived field is a named boolean over other fields. The real specs use
these to keep rules legible — `daemon.is_supervised` instead of a
three-way `or` repeated in four rules. One sharp edge I'll show you in
Act III: the *transition tracker* doesn't see through them, so guard your
state-changing rules with the direct `status = x` form and save derived
fields for everything else. That's a real 3.5.0 behavior, learned live.

---

### Slide 18 — Naming promises: contracts ⏱ 0:55

**SLIDE**

```allium
contract LendingHonesty {
    @invariant OneBorrowerAtATime
        -- A borrowed book is attributed to exactly one member until returned.

    @invariant RefusalIsExplicit
        -- Borrowing an unavailable book produces a refusal event, not silence.
}
```

**SAY**
Contracts collect the promises the rules are supposed to add up to. Each
`@invariant` is a *stable name* plus prose. Two reasons this matters more
than it looks. Reviewers and agents quote invariants by name — "does this
PR preserve `OneBorrowerAtATime`?" is a crisp review question. And the
names become test obligations in the planning pipeline — a named promise
becomes a grep-able ID in your test suite. That pipeline is Act V.

---

## Act III — The tool pushes back (slides 19–24) · live findings, real edges

---

### Slide 19 — The tool catches the deck (I): `status.noExit` ⏱ 1:00

**SLIDE**

```text
"Status 'overdue_flagged' in entity 'Book' has no observed
 transition to a different status."          allium.status.noExit
```

> The first draft of THIS DECK had a third state. The tool objected.

**SAY**
True story from building this deck. First draft: three states —
`on_shelf`, `borrowed`, `overdue_flagged`. The return rule used a set
guard, `status in {borrowed, overdue_flagged}` — perfectly legal syntax.
And check flagged `overdue_flagged` as having no exit. Why? The transition
tracker credits an exit only when a rule's guard names the state
*directly*, `status = overdue_flagged`. Set guards — and derived-field
guards, same edge — don't register. Two lessons: the tool genuinely does
state-graph analysis, and knowing *how* it observes transitions changes how
you write guards. Idiom: state-changing rules get direct equality guards.

---

### Slide 20 — The tool catches the deck (II): a conflict finding ⏱ 1:05

**SLIDE**

```json
{
  "type": "conflict",
  "rule_a": "ReturnBook", "rule_b": "BookGoesOverdue",
  "state": "borrowed",
  "summary": "... can both fire when 'Book' is in state 'borrowed',
              setting status to conflicting values",
  "values": { "ReturnBook": "on_shelf",
              "BookGoesOverdue": "overdue_flagged" }
}
```

**SAY**
Second catch, and this one's `analyse`, not `check`. Draft two: the
overdue rule *set* `status = overdue_flagged` while the return rule set
`status = on_shelf` — and both can fire from `borrowed`. A race, found
mechanically: a member returning the book at the same instant it goes
overdue. Two different triggers, one state, one field, two values. I didn't
plant this. I wrote the bug the way any of us would, and the analyser
caught it. So — how do you *fix* a race like this?

---

### Slide 21 — The fix, stolen from production ⏱ 1:09

**SLIDE**

```allium
-- mother-lifecycle.allium: the timeout does NOT set lifecycle
rule StopTimesOut {
    when: _: MotherDaemon.stop_deadline_at <= now
    requires: daemon.lifecycle = stopping
    ensures: MotherStopObserved(daemon: daemon, outcome: "timed_out")
}
```

> **Observe the fact; don't race the state.**

**SAY**
Here's the payoff of the "ensures can emit events" seed from slide 4. Look
at how the production daemon spec handles its stop timeout: the temporal
rule *records an observation* — it never touches the `lifecycle` field, so
it cannot race the normal stop-completion path. Our library fix is
identical: `BookGoesOverdue` emits `BookOverdue` and leaves `status` alone.
The state machine stays two states and boring; time produces *facts*, not
state fights. This is a load-bearing pattern in this team's architecture —
MCT's entire observation ledger is this idea: decisions and effects emit
observations; projections are derived, never authoritative.

---

### Slide 22 — What stays silent (and why you must know) ⏱ 1:14

**SLIDE**

```allium
    requires: book.staet = borrowed     -- typo: NO diagnostic

-- same trigger, same guard, contradictory ensures: NO finding
rule ApproveChild       { ... ensures: child.state = approved }
rule ApproveChildStrict { ... ensures: child.state = blocked  }
```

> Validated on 3.5.0. Green CLI ≠ correct spec.

**DO**
Typo a field name in a `requires`, run check, show the silence, undo.

**SAY**
Now the honesty slide, and don't skip it. Two things the current tool does
NOT catch. Field references in guards aren't name-checked — that typo runs
green. And *same-trigger* contradictions are treated as case analysis —
the tool assumes your guards partition, so it doesn't flag two identical
guards with opposite effects. Compare with slide 20: *cross-trigger*
conflicts on one state ARE caught. Know the net's exact shape: structural
closure, transition exits, cross-trigger races — caught. Typos and
same-trigger contradictions — yours. Which is why the loop has three nets:
CLI, `/weed`, human review. And here's the contributor hook: every silence
on this slide is a candidate diagnostic. Making one of them real is a
perfect first upstream PR — that's Sessions 4 and 6.

---

### Slide 23 — Why not full formal methods? ⏱ 1:18

**SLIDE**

> Allium sits between:
>
> - prose that checks **nothing**
> - TLA+/Alloy that most teams **won't write**
>
> Trades proofs for: fast checks · explainable findings ·
> specs that humans AND LLMs read and write.

**SAY**
The gaps you just saw aren't only immaturity — some are posture. No
quantifiers over unbounded domains, no refinement proofs, no temporal-logic
model checking. In exchange: analysis in milliseconds, findings a reviewer
understands in one read — you saw the conflict JSON, it names the two
rules, the state, and the values — and a language an agent can write
without a PhD in the loop. Where a contract someday genuinely warrants a
proof, the honest path is exporting the state graph to a model checker —
a future-directions topic in Session 6 — not making everyone write TLA+.

---

### Slide 24 — The finished sticky note ⏱ 1:21

**SLIDE**

> `slides/library.allium` — the whole journey in 80 lines:
> 2 entities · 1 value-free · 4 rules (1 temporal) · 1 surface
> (provides + exposes) · 1 contract (3 invariants) · config
>
> `check` ✅ `analyse` ✅ `plan` → **18 obligations**

**DO**
Run all three commands on `slides/library.allium`, ending on the
obligation ID list.

**SAY**
Act summary. The sticky note is now a machine-checked contract that
survived two real findings, and — watch the last command — it just handed
us 18 concrete test obligations, each with a stable ID. Keep those IDs on
screen for a second; in Act V they connect to Rust test files in this repo.
An hour ago this was one English sentence.

---

## Act IV — The same shapes at production scale (slides 25–30)

---

### Slide 25 — Nothing new, just bigger: the daemon lifecycle ⏱ 1:25

**SLIDE**

```text
LIBRARY (toy)                    MOTHER (production, this repo)
on_shelf | borrowed              stopped | starting | running | stopping | failed
BorrowBook / ReturnBook          ManualStartAccepted / StopRunningDaemon / ...
due_at + BookGoesOverdue         stop_deadline_at + StopTimesOut
LibraryDesk provides             MotherLifecycleCLI provides (7 operator triggers)
```

**SAY**
Open `layer/allium/mother/mother-lifecycle.allium` next to the library
spec. Every construct maps one-to-one — the daemon just has more states,
more cases, and *two* doors instead of one: a CLI surface for operator
actions and a runtime-signals surface for what the *system* reports
(startup completed, process exited). Operator actions and system facts
enter through different doors — steal that split. The point of this slide:
you already knew how to read this file. There was no new language to learn,
only a bigger domain.

---

### Slide 26 — The spec as decision log: the MCT product map ⏱ 1:29

**SLIDE**

```allium
open question "Confirm the one-sentence product statement for MCT."
-- Decision: MCT is a whole-app, local-first Mother/Child/Toy runtime ...
-- Decision: Iroh supplies the connectivity substrate; MCT supplies authority.
-- Principle: Fastest path wins only among authorized paths.
```

> ~1900 lines. Undecided things are VISIBLE, not silently absent.

**SAY**
`mct-product-map.allium` is the biggest spec in the org, and its most
radical feature is the humblest syntax: `open question` plus `-- Decision:`
comments. The spec doubles as the product's decision log — what's settled
sits next to what's still open, in the same reviewable file. This kills the
worst failure mode of specs, silent incompleteness. You diff decisions like
code. When your capstone spec has something you can't decide yet, don't
guess — write the open question. That's the spec working, not failing.

---

### Slide 27 — Exposure at scale: who learns what from a denial ⏱ 1:34

**SLIDE**

```text
one denied call, two surfaces:

MctResultReturn (caller)          RouteDecisionInspection (operator)
  outcome: denied                   phase1_evaluations (every candidate)
  requester_message (safe)          candidate eliminations + reasons
  audit_ref (opaque)                topology, load, policy revisions
```

**SAY**
Remember hiding `borrowed_by` from the library desk? Here's that idea
carrying real security weight. When MCT denies a route, the *caller* gets a
safe message and an opaque audit reference; the *operator* surface exposes
the full candidate-elimination reasoning — which nodes were considered,
why each fell out. Same underlying facts, two exposure contracts, and the
spec — not code-review vigilance — is what separates them. There's an
invariant spelling it out: *the audit log may know more than the requester
is allowed to see*. When you write a spec with any authority decision in
it, you now owe it two surfaces.

---

### Slide 28 — Four patterns to steal ⏱ 1:38

**SLIDE**

> 1. **Authority record** — evidence ≠ authority ≠ decision
>    (`IrohConnectionPresentation` / `MctPeerBinding` / `MctPeerAdmissionDecision`)
> 2. **Two-phase decision** — filter (authority) then rank (planner);
>    ranking can never grant
> 3. **Terminal result** — closed outcome set; immutable once constructed
> 4. **Observe, don't race** — temporal facts as events (slide 21)

**SAY**
Read the product map once end-to-end and these four shapes repeat. Pattern
one is the deepest: three separate entities for *what was presented*, *what
authority exists*, and *what was decided* — so reachability evidence can
never quietly become permission. Pattern two has its own contract:
optimization must never make an inadmissible route admissible. These are
this team's reusable spec vocabulary — your capstone gets reviewed partly
on whether you reached for them.

---

### Slide 29 — From spec to test suite: the obligation pipeline ⏱ 1:43

**SLIDE**

```bash
allium plan mother-lifecycle.allium > mother-lifecycle.allium.plan.json
```

```rust
// obligation id: rule-success.StartupSucceeded
#[test]
fn startup_success_marks_control_plane_ready() { /* ... */ }
```

**SAY**
Act V's bridge. `plan` derives obligations — verify these fields, this
rule's success case, its failure case, that temporal trigger — each with a
stable ID. The team convention tags Rust tests with those IDs in comments,
resume-style. Now the traceability chain is grep-able in both directions:
spec construct → obligation → test → CI. "Are we actually testing what the
spec promises" stops being a meeting and becomes a query.

---

### Slide 30 — CI: the loop, mechanized ⏱ 1:46

**SLIDE**

```text
patina CI:     cargo install allium-cli
               regenerate plans
               git diff --exit-code -- *.plan.json    ← drift fails the build

patina-mct:    install-allium-ci.sh   (version + SHA-256 pinned)
               allium check layer/allium              ← tier-0 gate
```

**SAY**
Both repos enforce this today. Patina regenerates the plan files in CI and
fails if they differ from what you committed — you cannot change behavioral
obligations silently. MCT pins the exact toolchain by hash and makes spec
validity a tier-0 gate, the same class as "it compiles." Note what CI runs:
`check`, deterministic and fast. The probabilistic parts of the loop —
elicit, weed — live with agents and humans; the gates in CI are the parts
that never hallucinate.

---

## Act V — The deep end (slides 31–36) · under the hood, and beyond it

---

### Slide 31 — One parser, three consumers ⏱ 1:50

**SLIDE**

```bash
allium parse library.allium | jq '..|objects|select(.name?=="BorrowBook")'
```

> Recursive-descent parser → AST with byte-offset `source_span`s.
> The SAME spans appear in: diagnostics · plan obligations · LSP.

**SAY**
Under everything: a hand-written recursive-descent parser in Rust — one
parse function per construct, which is why error messages can be precise
(you saw one earlier list every legal top-level keyword). Every AST node
carries byte-offset spans, and those spans thread through the whole
toolchain — the diagnostic that pointed at line 9, the obligation's
source_span, the squiggle in your editor: same parse, same offsets. When
you trace a bug in tooling behavior, this is your map: parse → AST →
passes → JSON.

---

### Slide 32 — The analysis passes, honestly mapped ⏱ 1:54

**SLIDE**

> Documented (help text): data flow · edge reachability · deadlock ·
> conflict · invariant verification
>
> Observed (3.5.0, this deck's own findings):
> ✅ unused entity/field · ✅ unreachable trigger · ✅ status no-exit ·
> ✅ cross-trigger conflicts
> ❌ guard typos · ❌ same-trigger contradictions · ❌ prose invariants

**SAY**
The two-column truth. Everything with a checkmark, you watched happen in
this deck. Everything with a cross, we tested and got silence. Note the
subtle one: invariant *verification* is documented, but our invariants are
named prose — there's nothing formal to verify yet, so the pass has nothing
to chew on. That gap between the documented ambition and observed behavior
isn't a scandal — it's a *map of contribution surface*. Which is the next
slide.

---

### Slide 33 — Extending it: three tiers, cheapest first ⏱ 1:58

**SLIDE**

> 1. **Markdown** — new skills, team patterns (no Rust)
> 2. **Rust** — new diagnostics/analysis passes
>    e.g.: name-check guard fields · flag same-trigger contradictions ·
>    require terminal states
> 3. **Integration** — consume `model`/`plan` JSON in a harness
>    (Mother readiness gates; external control planes)
>
> Proposal format: defect → triggering snippet → expected finding JSON

**SAY**
Three ways in, by cost. Tier one is prompt engineering under version
control — a team pattern like "authority record" can ship as a skill
fragment this week. Tier two is where slide 22's silences become PRs — and
notice you already know the finding JSON shape to emit, you've been reading
it all session. Tier three doesn't even need upstream changes: the JSON
output *is* the integration contract. Session 4's exercise has you write
one tier-two proposal; the good ones historically become real.

---

### Slide 34 — The whole ladder, one slide ⏱ 2:01

**SLIDE**

> 1. *Sentence*: "a member can borrow a book"
> 2. *Rule*: when / requires / ensures
> 3. *Machine*: states, cases, time, doors, exposure
> 4. *Contract*: named invariants, obligations, tagged tests, CI gates
> 5. *System*: decision-log specs, authority patterns, safe projections
> 6. *Toolchain*: parser → AST → passes → JSON → your extension
>
> Same behavior. Six altitudes.

**SAY**
The deck in one breath. Nothing at level six contradicted level one — that
is the language's actual design achievement: the sticky note and the
product map are the same medium at different altitudes, and one afternoon
of syntax buys you the whole ladder. Where the tool is strong, lean on it;
where it's silent, you know exactly where, and you know the loop's other
two nets; where it's extensible, you know the entry points.

---

### Slide 35 — Now you: the refactor exercise ⏱ 2:03

**SLIDE**

> `exercises/03-spec-refactor/starter/child-approval.allium`
>
> Nine planted flaws. The CLI finds three. You find six.
> Done = check fully clean + a partner can't find a remaining
> anti-pattern in 5 minutes.

**SAY**
Everything from today, weaponized: the starter has implementation leakage,
UI in the spec, prose smuggling, a dead entity, an unreachable trigger, a
guard typo, and a same-trigger contradiction. You now know which of those
the tool will hand you and which you must hunt. Run check *first*, spend
your human attention only where the machine is blind. Answer key exists;
earn it before you open it.

---

### Slide 36 — Resources ⏱ end

**SLIDE**

> - `slides/library.allium` — today's example, runnable
> - `layer/allium/mother/mother-lifecycle.allium` — first real read
> - `patina-mct/layer/allium/mct-product-map.allium` — skim once, fully
> - Cheat sheets: `cheatsheets/syntax.md` · `cheatsheets/cli.md`
> - Language reference (v3): juxt.github.io/allium · allium-tools repo
>
> Next: Session 4 — internals (parser, skills, extension points)

**SAY**
Homework before Session 4: skim the product map end to end once —
don't study it, just let the four patterns from slide 28 jump out — and do
the refactor exercise. Bring your six-flaw list. Session 4 opens the parser
and the skills up and ends with you proposing the diagnostic that catches
one of slide 22's silences.

---

## Appendix — facilitator crib

- **Timing pressure points**: Act II slide 14 (the provides demo) and
  Act III (three live moments) are where a 90-minute run slips. If tight:
  slide 16 (value/entity) and slide 23 (formal methods) compress to one
  sentence each; never cut slides 19–22 — the live findings are the deck's
  spine.
- **Demo prep**: have `slides/library.allium` and
  `layer/allium/mother/mother-lifecycle.allium` open in splits; terminal
  with `PATH` including allium 3.5.0; pre-type the four DO commands in
  shell history.
- **If a demo surprises you** (new CLI version, different diagnostics):
  say so and explore live — the deck's thesis is that the terminal
  outranks the slides. Update the deck afterward; slide 32's two-column
  table is version-stamped for exactly this reason.
- **Audience calibration**: for a spec-experienced room, Act I compresses
  to 10 minutes (slides 2, 4, 7, 8) and the recovered time goes to Act V
  and live product-map reading. For a junior room, protect Act I and II
  fully and trim Act V to slides 31 and 34.
