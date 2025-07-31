---
id: layer-structure-evolution
version: 1
created_date: 2025-07-31
confidence: high
oxidizer: nicabar
tags: [architecture, layer-management, core-concept]
status: active
---

# Layer Structure Evolution: Core, Surface, and Dust

## The Breakthrough

Patina's layer system evolves from arbitrary topic organization to a natural oxidation-based structure that mirrors how knowledge actually accumulates and ages in software projects.

## The Three Layers

### Core Layer
**What it is**: Patterns actively implemented in code - the deep, protective patina.

**What belongs here**:
- Patterns you can grep for in the codebase
- Architectural decisions reflected in code structure
- Standards every module follows
- Workflows used daily

**Key insight**: Core isn't permanent - it's what's in use. When patterns stop being used, they move to dust.

### Surface Layer  
**What it is**: Active oxidation where new patterns form and experiments live.

**Structure**:
```
surface/
├── raw/           # Unprocessed sessions, fresh captures
├── dagger/        # Forming patterns about dagger
├── testing/       # Forming patterns about testing
└── .../          # Other domains as they emerge naturally
```

**What belongs here**:
- Raw session files (always in `raw/`)
- Emerging patterns being validated
- Experiments and explorations
- Ideas being refined before promotion to core

### Dust Layer
**What it is**: Valuable archive of deprecated, experimental, or unused patterns.

**Categories**:
```
dust/
├── sessions/      # Processed raw material
├── experiments/   # Good ideas, wrong time
├── deprecated/    # What we've moved past
└── inspirations/  # Loved by oxidizer, not used yet
```

**Key insight**: Dust isn't trash - it's wisdom gained but not currently applied.

## The Natural Flow

```
1. Sessions land in surface/raw/
2. Patterns extracted to surface domains
3. Proven patterns promote to core
4. Unused patterns scrape to dust
5. Dust eventually blows to database (rqlite)
```

## Project Lifecycle

### Early Stage (Days to Weeks)
- Rapid movement between core ←→ surface
- Core changes frequently as architecture solidifies
- Many experiments, high churn rate
- Chaotic but productive

### Mature Stage (Months to Years)
- Core becomes bedrock, rarely changes
- Innovation happens at surface
- Dust accumulates valuable history
- Patterns proven across time

## Domain Formation

Domains in the surface layer form organically based on where oxidation naturally occurs:
- Start with just `surface/raw/`
- As patterns cluster, create directories
- Names emerge from actual use (dagger/, testing/, auth/)
- Like paths that form where people walk

## Database Vision

As layers grow beyond single projects:
- **Active layers** (core/surface) stay in filesystem - fast, greppable
- **Dust blows to database** - searchable across projects and time
- **Cross-project wisdom** - "Show me all auth patterns I've ever used"
- **Historical queries** - "How did my testing philosophy evolve?"

## Commands

```bash
# See what's where
patina layer list core          # What patterns are we using?
patina layer list surface/raw   # What needs processing?
patina layer about dagger       # Search across all layers

# Move patterns through lifecycle  
patina layer extract            # Raw → Surface domains
patina layer promote            # Surface → Core
patina layer scrape             # Surface/Core → Dust
patina layer archive            # Dust → Database

# Maintain quality
patina layer consolidate        # Merge similar patterns
patina layer verify             # Check if core matches code
```

## Key Principles

1. **Core = Living Code Truth**: If it's in core, it's in the code
2. **Surface = Active Work**: Where oxidation happens
3. **Dust = Valuable Archive**: History and unused wisdom
4. **Natural Formation**: Let domains emerge, don't prescribe
5. **Lifecycle Awareness**: Young projects churn, mature projects stabilize

## Why This Works

- **Matches Reality**: Code patterns do oxidize and age
- **Scales Naturally**: From single project to career-long wisdom
- **Clear Workflow**: Raw → Surface → Core or Dust
- **Preserves Everything**: No wisdom is lost, just archived
- **Database Ready**: Filesystem for speed, database for scale

This structure lets Patina grow naturally while maintaining clarity about what's active, what's forming, and what's historical.