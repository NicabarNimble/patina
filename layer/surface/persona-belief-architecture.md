---
id: persona-belief-architecture
version: 1
status: draft
created_date: 2025-10-25
updated_date: 2025-10-25
oxidizer: nicabar
tags: [architecture, persona, beliefs, ontology, neuro-symbolic, domains]
---

# Persona-Belief Architecture

**Core Concept**: Patina hosts a single persona per installation—a belief system that interprets and structures all knowledge. Data only exists through the lens of this persona.

## The Shape

```
                    ╔═══════════════════════════╗
                    ║       PERSONA             ║
                    ║  (interpretive lens)      ║
                    ║                           ║
                    ║  - Belief system          ║
                    ║  - Ontology (structure)   ║
                    ║  - Rules (reasoning)      ║
                    ║  - Weights (confidence)   ║
                    ╚═══════════╤═══════════════╝
                                │
                    Interprets everything below
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
    ┌───▼─────┐           ┌────▼─────┐          ┌─────▼────┐
    │ DOMAIN  │           │ DOMAIN   │          │ PROJECT  │
    │ (rust)  │───────────│ (ai)     │          │ (patina) │
    │         │           │          │          │          │
    │ Knowledge│          │Knowledge │          │Multi-    │
    │ blob    │           │ blob     │          │domain +  │
    │         │           │          │          │software  │
    └─────────┘           └──────────┘          └──────────┘
        ↑                      ↑                      ↑
        └──────────────────────┴──────────────────────┘
                               │
                      Raw data (sessions)
```

## Core Principles

### 1. Data Only Exists Through Persona

Raw bits (session markdown, code, events) are meaningless until interpreted through the persona's belief system.

```
Session file: "Used hierarchical modules in Rust"

Without persona: Just text
With persona: → Interpreted as belief confirmation
              → Strengthens architectural preference
              → Updates rust domain knowledge
              → Informs future decisions
```

### 2. Beliefs Are Conditional Structures

Beliefs encode WHEN and WHY, not just preferences.

```prolog
belief(
  action: use_ecs_architecture,
  domain: rust,
  when_conditions: [
    has_entities,
    has_behaviors,
    game_like_or_simulation
  ],
  unless_conditions: [
    simple_state,
    small_project
  ],
  why: "separates data from logic, scales well for entity-heavy systems",
  weight: 0.85
).
```

**Not**: "I prefer ECS" (binary)
**But**: "I use ECS when X, unless Y, because Z" (structured knowledge)

### 3. Weight System Tracks Confidence

Weights represent confidence in beliefs, managed by the system through observation and refinement.

```
0.0 - 0.3: Low confidence / deprecated belief
0.4 - 0.6: Uncertain / needs more data (baseline: 0.5)
0.7 - 0.8: Confident
0.9 - 1.0: Very confident / core principle
```

**Weight Evolution:**
```
Initial declaration:
  User: "I prefer ECS for games"
  → belief(use_ecs, when: [is_game], weight: 0.5)

After sessions reinforce:
  Session 1: Uses ECS → weight: 0.6
  Session 3: Uses ECS → weight: 0.75
  Q&A confirmation → weight: 0.85

Session breaks pattern:
  Session 10: Doesn't use ECS
  → weight: 0.75 (weakened)
  → System queues question about conditions
```

### 4. User as Source, LLM as Tracker

**User provides beliefs** (the what)
**LLM manages weights and conditions** (the when/why refinement)

The system may observe patterns the user isn't consciously aware of:

```
LLM: "Based on 15 sessions, I observe:
     - You avoid global state (confidence: 0.8)
     - You prefer composition over inheritance (confidence: 0.9)

     Were these intentional principles?"

User: "I didn't realize, but yes, that's how I think"
```

## Architecture Components

### Persona (Central System)

```
~/.patina/
  persona/
    ontology.pl           # Defines what CAN exist
                         # (belief structures, relations, domains)

    rules.pl             # Reasoning logic
                         # (how beliefs interact, inference rules)

    ethics.pl            # Meta-beliefs about belief formation
                         # (when to trust observations, how to weight)

  knowledge.db           # Single database for all knowledge
    ├─ beliefs           # Conditional belief structures
    ├─ observations      # Raw observations from sessions
    ├─ weights           # Confidence tracking
    ├─ provenance        # Session → belief mapping
    └─ question_queue    # Refinement questions

  hub/
    sync.db              # Provenance, temporal state
    light_model.pl       # Brightness/decay rules (temporal relevance)
```

### Domains (Knowledge Blobs)

Domains are organizational lenses within the knowledge system.

```
domains/
  rust/
    → All Rust-related beliefs, patterns, observations
    → Spans multiple projects

  ai/
    → All AI-integration beliefs
    → Spans multiple projects

  patina/
    → Project-specific domain
    → Also a software project
```

**Domains are tags/namespaces, not separate databases.**

```sql
-- All rust beliefs
SELECT * FROM beliefs WHERE domain = 'rust';

-- Rust beliefs specific to patina project
SELECT * FROM beliefs
WHERE domain = 'rust' AND project = 'patina';
```

### Projects (Multi-Domain + Software)

Projects are contexts where:
- Software is built
- Sessions are generated
- Multiple domains are active simultaneously

```
~/Projects/patina/
  .patina/
    project.toml:
      domains = ["rust", "ai", "cli", "git", "prolog"]
      persona_db = "~/.patina/knowledge.db"

  layer/sessions/        # Raw material (sessions)
  src/                   # Software being built
```

**A single session touches multiple domains:**

```
Session: 20251025-081846.md
  Contains work on:
    - Rust patterns        → feeds rust domain
    - AI integration       → feeds ai domain
    - Patina architecture  → feeds patina domain
    - CLI design           → feeds cli domain
```

## The Two-Phase Dialogue System

### Phase 1: Extraction (After Session)

Sessions are worked naturally, then extracted after completion.

```bash
# Work happens
/session-start "implement ecs"
[user works...]
/session-end

# Later: extraction
patina session extract 20251025-081846.md
```

**Extraction Process:**

```
1. Read session file (raw data)

2. Apply persona ontology
   - What can be extracted? (decisions, patterns, tech usage)
   - What structure should it have?

3. Apply persona rules
   - Does this match existing beliefs?
   - Does this contradict beliefs?
   - Is this a new pattern?

4. Generate observations
   observation(
     session: "20251025-081846",
     domain: rust,
     pattern: "used_ecs_architecture",
     context: ["game_project", "has_entities"]
   )

5. Update belief weights
   belief(use_ecs, when: [...], weight: 0.75 → 0.85)

6. Queue questions if needed
   question(
     domain: rust,
     observation: "ecs_not_used_in_cli_tool",
     ask: "You usually use ECS but not here. Should we refine when you use it?"
   )
```

**The persona guides extraction** - better persona rules = smarter extraction.

### Phase 2: Domain Refinement (Reflective Dialogue)

After many sessions accumulate, refine domain beliefs systematically.

```bash
patina persona refine rust

LLM: "I have 5 observations to discuss about your Rust work:

     1. You used ECS in 8/10 projects with entities
        Current belief: use_ecs when [game_like]
        Suggested refinement: use_ecs when [has_entities, has_behaviors]

        Accept this refinement?"

You: "Yes, and add 'unless small_project'"

LLM: [Updates belief]
     belief(use_ecs,
       when: [has_entities, has_behaviors],
       unless: [small_project],
       weight: 0.9)

     2. You avoided global state in all 15 sessions
        This seems like a principle. Should I add it?"

You: "Yes - that's a core belief"

LLM: [Creates new belief]
     belief(avoid_global_state,
       domain: rust,
       when: [always],
       why: "prevents coupling and testing issues",
       weight: 0.95)
```

**Refinement works through the question queue built during extraction.**

## Ontology-Driven Structure

### Ontology Defines Possibility Space

```prolog
% ontology.pl

% What is a belief?
belief(Action, Domain, Conditions, Weight) :-
    action(Action),
    domain(Domain),
    conditions(Conditions),
    weight_valid(Weight).

% What are valid conditions?
condition(has_entities).
condition(has_behaviors).
condition(game_like).
condition(simple_project).

% How do beliefs relate?
belief_conflicts(Belief1, Belief2) :-
    belief(Action, Domain, When1, _),
    belief(not(Action), Domain, When2, _),
    conditions_overlap(When1, When2).

% Weight constraints
weight_valid(W) :- W >= 0.0, W =< 1.0.

% Temporal decay
belief_brightness(Belief, Brightness) :-
    belief_last_observed(Belief, Days),
    decay_function(Days, Brightness).
```

### SQLite Stores Instances

```sql
-- Instances of beliefs conforming to ontology
CREATE TABLE beliefs (
  id INTEGER PRIMARY KEY,
  action TEXT NOT NULL,
  domain TEXT NOT NULL,
  when_conditions JSON,      -- Array of condition atoms
  unless_conditions JSON,
  why TEXT,
  weight REAL CHECK(weight >= 0 AND weight <= 1),
  created_at TIMESTAMP,
  last_observed TIMESTAMP,
  observation_count INTEGER
);

-- Raw observations from sessions
CREATE TABLE observations (
  id INTEGER PRIMARY KEY,
  session_id TEXT,
  domain TEXT,
  pattern TEXT,
  context JSON,
  matches_belief INTEGER,    -- FK to beliefs if it reinforces
  conflicts_belief INTEGER,  -- FK to beliefs if it contradicts
  FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Question queue for refinement
CREATE TABLE questions (
  id INTEGER PRIMARY KEY,
  domain TEXT,
  observation_id INTEGER,
  question_text TEXT,
  priority INTEGER,
  asked BOOLEAN DEFAULT FALSE,
  FOREIGN KEY (observation_id) REFERENCES observations(id)
);
```

## Neuro-Symbolic Integration

```
┌──────────────────────┐         ┌──────────────────────┐
│   LLM (Neural)       │         │  Prolog + SQLite     │
│                      │         │  (Symbolic)          │
├──────────────────────┤         ├──────────────────────┤
│ • Interprets prose   │◄────────│ • Defines structure  │
│ • Extracts patterns  │         │ • Validates logic    │
│ • Generates language │─────────►│ • Infers relations   │
│ • Proposes beliefs   │         │ • Enforces ontology  │
└──────────────────────┘         └──────────────────────┘
         │                                  │
         └────────── Continuous ────────────┘
                     feedback loop
```

**LLM** reads sessions, proposes beliefs, asks questions
**Prolog** validates structure, checks consistency, enables inference
**SQLite** stores instances, enables efficient queries

They shape each other:
- Better ontology → Better LLM extraction prompts
- Better LLM observations → Refined ontology rules

## Temporal Light Model

Not all beliefs weigh equally in reasoning. The light model provides dynamic relevance.

```prolog
% Brightness = current relevance
belief_brightness(rust_ecs_pattern, 0.9).      % Recent + frequent
belief_brightness(old_docker_pattern, 0.3).    % Dated

% Decay over time
decay(Belief, DaysSinceObserved, Brightness) :-
    initial_weight(Belief, W),
    Brightness is W * exp(-DaysSinceObserved / 90).

% Reflection = influence propagation
reflects(rust, cli) :-
    domain_overlap(rust, cli, SharedPatterns),
    length(SharedPatterns, N),
    N > 3.

% Dim beliefs still influence through reflection
effective_weight(Belief, EffectiveWeight) :-
    belief_brightness(Belief, Brightness),
    reflected_domains(Belief, Domains),
    sum_reflection(Domains, ReflectionBoost),
    EffectiveWeight is Brightness + ReflectionBoost.
```

## The Complete Flow

### 1. Initial State (Cold Start)

```
User: "I prefer ECS for game-like projects"

System:
  → belief(use_ecs, when: [game_like], weight: 0.5)
  → Baseline belief created
```

### 2. Work Happens (Sessions Accumulate)

```
Session 1: Implements ECS in game
  → observation(used_ecs, context: [game, entities])
  → Matches belief → weight: 0.6

Session 3: Implements ECS in simulation
  → observation(used_ecs, context: [simulation, behaviors])
  → Matches belief → weight: 0.75
  → Queue question: "Refine 'game_like' condition?"

Session 5: Uses OOP in simple CLI tool
  → observation(used_oop, context: [cli, simple])
  → Doesn't contradict (different context)
  → No action

Session 8: Implements ECS again
  → observation(used_ecs, context: [game])
  → Matches belief → weight: 0.85
```

### 3. Extraction Builds Question Queue

```
questions:
  1. "You used ECS in simulation (not game). Broaden condition?"
  2. "You used OOP in CLI. Is this an 'unless' case for ECS?"
  3. "You avoided global state consistently. Add as belief?"
```

### 4. Refinement Dialogue

```
patina persona refine rust

[Works through question queue]

Result:
  belief(use_ecs,
    when: [has_entities, has_behaviors],
    unless: [simple_project, cli_tool],
    weight: 0.9)

  belief(avoid_global_state,
    when: [always],
    weight: 0.95)
```

### 5. Improved Extraction

Next session extraction is smarter:

```
Session 10: User builds entity system

Extractor: "Entities + behaviors detected"
→ Check belief: use_ecs when [has_entities, has_behaviors]
→ Suggests during work: "This looks like an ECS use case?"
→ If confirmed: weight → 0.95
→ If rejected: Queue question about exception
```

## Implementation Path

### Phase 1: Foundation (Weeks 1-2)
- Design belief schema (SQL + Prolog ontology)
- Build basic extraction (LLM → observations)
- Manual belief creation (TOML → SQLite)

### Phase 2: Weight Management (Week 3)
- Implement weight tracking
- Observation → weight update logic
- Question queue system

### Phase 3: Extraction Intelligence (Week 4)
- Ontology-guided extraction
- Pattern matching against beliefs
- Conflict detection

### Phase 4: Refinement Dialogue (Week 5)
- Interactive refinement command
- Question presentation
- Belief update workflow

### Phase 5: Temporal Model (Week 6)
- Brightness/decay implementation
- Reflection propagation
- Dynamic weighting in queries

## Key Design Decisions

### 1. Single Database vs Distributed

**Decision**: Single database (`~/.patina/knowledge.db`)

**Rationale**:
- Domains are organizational, not technical boundaries
- Enables cross-domain queries efficiently
- Simplifies provenance tracking
- Persona emerges from totality more naturally

### 2. Prolog-First Ontology

**Decision**: Ontology defines structure before data

**Rationale**:
- Ensures semantic coherence
- Enables validation at insertion
- LLM prompts can reference ontology
- Clear separation: structure (Prolog) vs instances (SQLite)

### 3. User Declares, System Observes

**Decision**: Users create beliefs, system manages weights

**Rationale**:
- User is authority on their own beliefs
- System tracks confidence, not truth
- Observations surface unconscious patterns
- Avoids LLM presuming user's mind

### 4. Conditional Belief Structures

**Decision**: Beliefs encode when/why/unless

**Rationale**:
- More expressive than binary preferences
- Captures reasoning, not just conclusions
- Enables context-aware application
- Grows more nuanced over time

## Relationship to Existing Architecture

This complements the hybrid extraction architecture:

```
Cloud LLM (rules) ───┐
                     ├──► Ontology + Belief Rules
CoreML (facts) ──────┘

Sessions ──► Extraction ──► Observations ──► Beliefs
                ▲                              │
                └──────── Guided by ───────────┘
```

- Cloud LLM can generate ontology/rules (one-time)
- CoreML extracts observations (ongoing, private)
- Prolog validates and infers (local)
- Beliefs guide future extraction (self-improving)

## Philosophy Alignment

✅ **Knowledge First**: Beliefs are structured knowledge, not configuration
✅ **Privacy First**: All reasoning happens locally
✅ **User Authority**: User's beliefs are source of truth
✅ **Explainable**: Every belief traces to sessions (provenance)
✅ **Adaptive**: Weights evolve, conditions refine
✅ **Ontology-Driven**: Meaning precedes data
✅ **Tool Composition**: Neural + Symbolic working together

## Next Steps

1. Draft ontology.pl (belief structure definition)
2. Design belief schema (SQLite tables)
3. Build basic extraction (session → observations)
4. Implement weight tracking system
5. Create refinement dialogue prototype
6. Test with patina project's 243 sessions

---

*This architecture represents the converged vision from the persona design review session on 2025-10-25.*
