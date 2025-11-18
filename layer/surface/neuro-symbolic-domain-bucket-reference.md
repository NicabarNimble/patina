# Patina Development Domain Bucket

**Neuro-Symbolic Knowledge Base for Patina Development Sessions**

## Architecture

This is a **domain bucket** - a self-contained knowledge repository using the neuro-symbolic AI pattern:

```
layer/buckets/patina-dev/
├── facts.pl          # Canonical structured data (Symbolic)
├── rules.pl          # Inference rules (Symbolic)
├── queries.md        # Example queries
└── README.md         # This file (Generated)
```

## The Neuro-Symbolic Pattern

**Neural (LLM)**: Extracts structured facts from unstructured session markdown
**Symbolic (Prolog)**: Applies logical rules to infer new knowledge

### Data Flow

```
layer/sessions/*.md  →  [LLM Extraction]  →  facts.pl
                                               ↓
                                          [Prolog Inference]
                                               ↓
                                          Answers & Insights
```

## What's in the Knowledge Base?

### Facts (facts.pl)

Extracted from 227 session files in `layer/sessions/`:

- **Sessions**: ID, date, work type, branch, commits, files changed
- **Patterns**: Observed patterns categorized (security, architecture, workflow, etc)
- **Technologies**: Tools used and their purposes
- **Decisions**: Key choices and rationale
- **Challenges**: Problems faced and solutions found
- **Domain Links**: Cross-domain relationships (UNSOLVED PROBLEM)

### Rules (rules.pl)

Inference rules for discovering knowledge:

- **Pattern Evolution**: Identify recurring and mature patterns
- **Session Classification**: Categorize work (productive, exploratory, etc)
- **Technology Discovery**: Find tool pairs and purposes
- **Problem-Solution Mapping**: Link challenges to solutions
- **Decision Analysis**: Classify decisions (philosophical vs pragmatic)
- **Knowledge Graph**: Workflow chains, temporal proximity, correlations

## Quick Start

### Install Scryer Prolog

```bash
brew install scryer-prolog
```

### Run Queries

```bash
cd layer/buckets/patina-dev

# Find all security patterns
scryer-prolog facts.pl rules.pl -g "pattern_in_category(P, security), write(P), nl, fail; halt."

# Find security-focused sessions
scryer-prolog facts.pl rules.pl -g "security_session(S), write(S), nl, fail; halt."

# Interactive mode
scryer-prolog facts.pl rules.pl
```

See `queries.md` for more examples.

## Example Queries & Insights

### Q: What security patterns have we discovered?

```prolog
?- pattern_in_category(P, security).
```

**Answer:**
- tmpfs-for-secrets
- 1password-integration
- credential-management
- security-review-generated-code

### Q: Which sessions focused on architecture?

```prolog
?- architecture_session(S).
```

**Answer:**
- 20251010-061739 (neuro-symbolic persona exploration)
- 20250813-055742 (tool vs system distinction)
- 20250809-211749 (git-aware navigation)

### Q: What technologies are used for security?

```prolog
?- tech_used(S, Tech, Purpose), sub_atom(Purpose, _, _, _, secure).
```

**Answer:**
- 1password-cli for secure credential storage

### Q: What challenges have we solved?

```prolog
?- challenge(_, Problem, Solution).
```

**Answer:**
- yaml-markdown-conflict → extract bash to external script
- kit-scanner-hang → replace glob with ignore crate
- hardcoded-home-path → use ${HOME} env var
- workspace-import-cycles → recognized wrong pattern for unstable code

## The Unsolved Problem: Cross-Domain Linking

Current approach stores cross-domain references as facts:

```prolog
domain_link('patina-dev', 'rust-development', 'implements-patterns-from').
domain_link('patina-dev', 'security', 'applies-patterns-from').
```

**Challenge**: How should domain buckets reference each other?
- Shared facts.db across domains?
- Foreign key relationships?
- Prolog imports?
- REST API between buckets?

This is the key architectural question for the neuro-symbolic persona system.

## Current Status

**Proof of Concept**: ✅
- Facts extracted from 7 recent sessions (manual)
- Rules implemented and tested
- Scryer Prolog queries working
- Demonstrates viability of approach

**Next Steps**:
1. **Automate extraction**: Build LLM-based parser (session.md → facts.pl)
2. **Expand coverage**: Extract all 227 sessions
3. **SQLite integration**: facts.pl ↔ facts.db for efficient querying
4. **Rust integration**: Embed Scryer Prolog in patina CLI
5. **Cross-domain linking**: Solve the architectural challenge

## Why This Matters

### Problem
- 227 sessions of accumulated knowledge
- Patterns buried in markdown prose
- No way to query: "What security patterns have we discovered?"
- Knowledge doesn't transfer between domains

### Solution
- **Neuro-Symbolic AI**: LLM extracts facts, Prolog infers knowledge
- **Queryable knowledge base**: Ask questions, get answers
- **Pattern evolution tracking**: Identify dust → surface → core promotion
- **Cross-project learning**: Patina persona loads canonical knowledge

### Vision

Persona becomes LLM by loading domain bucket:

```bash
patina persona load patina-dev
# LLM context now includes:
# - All extracted facts
# - All inference rules
# - Ability to query knowledge graph
```

LLM can then answer:
- "What patterns have we used for security?"
- "How did we solve scanner performance issues?"
- "What technologies work well together?"
- "Should this pattern be promoted from surface to core?"

## Architecture Integration

### Current Patina Architecture

```
patina/
├── layer/
│   ├── core/           # Eternal patterns
│   ├── surface/        # Active development
│   ├── dust/           # Historical archives
│   └── sessions/       # Session markdown (227 files)
```

### With Domain Buckets

```
patina/
├── layer/
│   ├── core/           # Eternal patterns
│   ├── surface/        # Active development
│   ├── dust/           # Historical archives
│   ├── sessions/       # Session markdown (227 files)
│   └── buckets/        # 🆕 Neuro-symbolic knowledge bases
│       ├── patina-dev/
│       │   ├── facts.pl
│       │   ├── rules.pl
│       │   └── queries.md
│       ├── rust-development/
│       ├── security/
│       └── devops/
```

## Technology Stack

- **Scryer Prolog**: Modern ISO Prolog written in Rust
- **SQLite**: Efficient fact storage (future)
- **Rust**: Patina CLI, LLM integration
- **LLM (Claude)**: Fact extraction from markdown

## Philosophy Alignment

This approach follows Patina's core philosophy:

✅ **Knowledge First**: Sessions → Facts → Rules → Insights
✅ **LLM Agnostic**: Prolog rules work with any LLM
✅ **Escape Hatches**: Can query via CLI, REPL, or Rust API
✅ **Rust Native**: Scryer Prolog is written in Rust
✅ **Tool-based Design**: Each component has clear input → output

## Learn More

- See `queries.md` for example queries
- See `facts.pl` for data schema
- See `rules.pl` for inference rules
- See `layer/sessions/20251010-061739.md` for neuro-symbolic exploration session
