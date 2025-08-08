---
id: unified-context-system
version: 1
status: draft
created_date: 2025-08-04
oxidizer: nicabar
references: [surface/git-aware-navigation-design.md, core/context-orchestration.md, topics/sessions/capture-raw-distill-later.md]
tags: [architecture, context, sessions, experiments, navigation, integration]
---

# Unified Context System for Patina

A tightly integrated system where sessions orchestrate all context, experiments are first-class citizens, and the LLM has complete awareness of your development history.

## Executive Summary

Current Patina has loosely coupled systems (sessions, navigation, git, workspaces). This design unifies them under a **context-first architecture** where:
- Sessions are the primary context orchestrator
- Every action enriches the context graph
- LLMs have full visibility into past experiments and decisions
- Dagger experiments are one command away with full history

## Core Concept: The Context Graph

```
Your Question: "What if we used Redis for tokens?"
                           ↓
                  ┌─────────────────┐
                  │  Context Graph  │
                  └────────┬────────┘
                           ↓
         ┌─────────────────┴─────────────────┐
         ↓                 ↓                 ↓
    Past Experiments   Current State    Pattern Evolution
    - jwt-redis ❌     - Branch: feat/   - Core principles
    - jwt-async ❌     - Files modified  - Failed approaches  
    - jwt-sqlite ✓    - Open questions  - Success patterns
                           ↓
                  "We tried Redis 2 days ago.
                   Issues: complexity, SPOF.
                   Current SQLite approach is working.
                   Want to revisit with clustering?"
```

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Active Session                        │
│  - Orchestrates all systems                             │
│  - Maintains context graph                              │
│  - Bridges human ↔ LLM understanding                    │
└─────────────────┬───────────────────────────────────────┘
                  │ Coordinates
    ┌─────────────┴─────────────┬─────────────┐
    ▼                           ▼             ▼
Context-Aware              Experiment      Pattern
Navigation                 Tracking       Evolution
- Query history            - Branches      - Success
- Result patterns          - Outcomes      - Failures
- Search evolution         - Artifacts     - Learnings
    │                           │             │
    └─────────────┬─────────────┘             │
                  ▼                           ▼
         ┌────────────────┐          ┌──────────────┐
         │  Git Integration│          │ Brain Storage│
         │  - Branch graph │          │ - Patterns   │
         │  - Commit intel │          │ - Decisions  │
         │  - Experiments  │          │ - Lessons    │
         └────────────────┘          └──────────────┘
```

## Key Components

### 1. Enhanced Session System

```rust
pub struct ContextualSession {
    // Identity
    id: String,
    started_at: DateTime<Utc>,
    
    // Current Focus
    goals: Vec<Goal>,
    active_questions: Vec<Question>,
    
    // Historical Context
    experiments: Vec<Experiment>,
    decisions: Vec<Decision>,
    failed_approaches: Vec<FailedApproach>,
    
    // Navigation Intelligence
    search_patterns: SearchIntelligence,
    frequently_accessed: Vec<PatternRef>,
    
    // Git Awareness
    branch_context: BranchContext,
    commit_intelligence: CommitPatterns,
}

pub struct Experiment {
    id: String,
    branch: String,
    hypothesis: String,
    
    // Execution
    dagger_pipeline: Option<DaggerRef>,
    container_logs: Vec<LogRef>,
    
    // Outcomes
    result: ExperimentResult,
    learnings: Vec<Learning>,
    artifacts: Vec<Artifact>,
    
    // Context
    triggered_by: QueryRef,
    related_experiments: Vec<ExperimentRef>,
}
```

### 2. Context-Aware Navigation

When you navigate, it's not just search - it's intelligence gathering:

```bash
patina navigate "jwt token"
```

The system tracks:
- What you searched for
- What results you got
- Which files you actually opened
- How this relates to your current goal
- Similar past searches and their outcomes

### 3. Experiment Lifecycle

#### Starting an Experiment
```
You: "Let's try distributed JWT validation"

Patina: 
┌─────────────────────────────────────────┐
│ Context Analysis:                        │
│ - No prior distributed JWT experiments   │
│ - Current JWT implementation uses SQLite │
│ - Related: "jwt-redis" experiment failed │
│                                         │
│ Suggested Experiment Setup:             │
│ - Branch from: feat/jwt-refresh         │
│ - Dagger: 3-node validation cluster     │
│ - Hypothesis: Distributed validation    │
│   improves resilience                   │
└─────────────────────────────────────────┘

Create experiment? [Y/n]
```

#### During Experiment
```bash
# Everything is tracked
patina experiment run
- Container outputs → archived
- Performance metrics → captured  
- Code changes → linked
- Decisions → recorded

# Mid-experiment insights
/experiment-note "Latency increases 10x with 3 nodes"
```

#### Experiment Completion
```bash
patina experiment complete --outcome=failed --reason="latency"

# Automatically:
- Archives all artifacts
- Captures learnings
- Updates context graph
- Suggests related patterns
```

### 4. LLM Context Provision

The `CLAUDE.md` becomes a living document:

```markdown
## Active Context

### Current Session: "Implement JWT revocation"
Started: 2 hours ago
Branch: feat/jwt-revocation

### Recent Experiments
1. **jwt-distributed** (30 min ago) ❌
   - Tried: 3-node validation cluster
   - Failed: 10x latency increase  
   - Learning: Distributed validation overkill for our scale
   
2. **jwt-redis** (2 days ago) ❌
   - Tried: Redis for token storage
   - Failed: Operational complexity
   - Learning: SQLite sufficient for 10k tokens/sec

3. **jwt-sqlite** (2 days ago) ✓
   - Tried: SQLite with in-memory cache
   - Success: Meets all requirements
   - Artifacts: benchmarks/, jwt-storage.md

### Navigation Patterns
You frequently search for:
- "jwt" (47 times) → Found patterns in auth/, security/
- "token refresh" (23 times) → Implemented in current branch
- "revocation" (12 times) → Currently exploring

### Decision Log
- Chose SQLite over Redis (2 days ago)
- Rejected async token refresh (3 days ago)  
- Adopted rotation strategy (yesterday)

### Available Commands
- `patina experiment create <name>` - Start new experiment
- `patina navigate <query> --why` - Search with context
- `patina explain <decision>` - Show decision history
```

## Workflow Examples

### Example 1: Informed Navigation
```bash
patina navigate "cache" --why
```
```
🔍 Searching for: cache (with context)

Based on your session history:
- You searched "cache" 5 times while working on JWT
- Previous decision: "No Redis caching" (2 days ago)
- Related experiment: "jwt-redis" (failed)

Results:
✓ patterns/cache-strategies.md
  → You viewed this during jwt-redis experiment
  
? surface/jwt-memory-cache.md  
  → New file, might address your Redis concerns
  
📝 Suggestion: Given past Redis issues, consider 
   the in-process caching pattern
```

### Example 2: Experiment with Memory
```
You: "What if we tried GraphQL for the API?"

Claude: "I see you explored GraphQL before:
- Experiment 'graphql-api' (2 weeks ago)
- Outcome: Abandoned due to complexity
- Specific issues:
  - N+1 query problems
  - Type generation overhead
  - Team unfamiliar with ecosystem
  
However, the landscape has changed:
- New GraphQL code-first libraries available
- Your team now has 2 weeks more experience

Would you like to create a new experiment with these learnings in mind?"
```

### Example 3: Pattern Evolution Awareness
```bash
patina patterns "authentication"
```
```
📈 Authentication Pattern Evolution:

1. Session Tokens (3 weeks ago)
   Status: Rejected
   Reason: Scaling issues
   
2. Simple JWT (2 weeks ago)
   Status: Evolved
   Issue: No refresh mechanism
   
3. JWT + Refresh (1 week ago)
   Status: Evolved  
   Issue: No revocation
   
4. JWT + SQLite Store (current)
   Status: Active
   Next: Add revocation
   
Failed Branches:
- jwt-async: Deadlock issues
- jwt-redis: Operational complexity
- jwt-distributed: Performance overhead

Recommendation: Continue with current approach
```

## Implementation Plan

### Phase 1: Unified Session Structure
1. Extend session format to include experiments, decisions
2. Add navigation tracking to sessions
3. Create experiment entity with full lifecycle

### Phase 2: Git Integration
1. Track branch purposes and relationships
2. Link commits to experiments
3. Build branch genealogy tree

### Phase 3: Dagger Experiments
1. Template-based pipeline generation
2. Artifact collection system
3. Automated learning extraction

### Phase 4: Context Intelligence
1. Search pattern analysis
2. Decision impact tracking
3. Failure pattern recognition

### Phase 5: LLM Integration
1. Rich CLAUDE.md generation
2. Context-aware command suggestions
3. Historical insight injection

## Benefits

1. **No Repeated Mistakes**: "We tried that" → "Here's what happened"
2. **Faster Decisions**: Past context informs current choices
3. **Learning Accumulation**: Every experiment enriches future work
4. **Confident Exploration**: Know what's been tried and what hasn't
5. **Team Knowledge**: Onboard new devs with full context

## Migration Path

1. Start capturing richer session data
2. Build experiment tracking into brain
3. Enhance navigation with context
4. Generate enriched CLAUDE.md
5. Add Dagger experiment commands

## Open Questions

1. How much context is too much for LLMs?
2. Should experiments auto-branch or be explicit?
3. How to handle conflicting learnings?
4. Privacy: What context should be shareable?

## Conclusion

By unifying sessions, navigation, experiments, and git into a single context graph, Patina becomes not just a tool but a **development memory system**. Every question benefits from every past answer. Every experiment informs every future decision.

The LLM becomes a true pair programmer with institutional knowledge rather than a stateless assistant.