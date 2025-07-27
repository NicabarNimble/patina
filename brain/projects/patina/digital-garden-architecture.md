# Digital Garden Architecture - Patina MVP Plan

## Vision
Patina is an orchestration function that connects users, LLMs, and accumulated wisdom (brain) to enable intelligent software development. It acts as the nervous system between your brain (knowledge) and your hands (AI assistants).

## Core Formula
```
patina(user, llm, brain) → orchestrated_intelligence
```

## Existing Architecture (Keep & Enhance)

### Development Environments (Already Implemented)
The three-environment system is core to Patina and must be preserved:

1. **Dagger** (Preferred when Go available)
   - Template-based pipeline generation
   - Fast, cached builds
   - Container-native development

2. **Docker** (Universal fallback)
   - Always available
   - Simple Dockerfile generation
   - Works everywhere

3. **Native** (Direct execution)
   - For simple tools
   - No container overhead
   - Direct cargo commands

### LLM Adapters (Keep & Expand)
1. **Claude** (Primary - FULLY IMPLEMENTED)
   - Session commands working perfectly
   - Knowledge capture via markdown sessions
   - Git-aware context tracking
   - Must preserve and enhance

2. **Gemini** (Secondary - TODO)
   - Planned as "the other LLM"
   - Will follow adapter pattern
   - Different context format (GEMINI.md)

## The Digital Garden Evolution

### Phase 1: SQL Brain Foundation (Current Sprint)
Build the storage layer while preserving Claude's session workflow.

#### Architecture
```
Claude Sessions (Markdown)  →  Patina Orchestrator  →  SQL Brain (SQLite)
     (capture)                    (distill)              (store/evolve)
```

#### SQL Schema for Brain
```sql
-- Core tables for MVP
CREATE TABLE patterns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('core', 'topic', 'project', 'decision', 'principle', 'constraint')),
    content TEXT,
    template_engine TEXT DEFAULT 'static',  -- 'static', 'handlebars', 'liquid'
    hooks JSON,  -- Pre/post hook scripts
    metadata JSON,
    source_file TEXT,
    project_origin TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    usage_count INTEGER DEFAULT 0,
    success_rate REAL DEFAULT 0.0
);

CREATE TABLE pattern_evolution (
    pattern_id TEXT,
    version INTEGER,
    changes TEXT,
    reason TEXT,
    project_context TEXT,
    timestamp TIMESTAMP,
    FOREIGN KEY (pattern_id) REFERENCES patterns(id)
);

CREATE TABLE pattern_relationships (
    parent_id TEXT,
    child_id TEXT,
    relationship_type TEXT,  -- 'extends', 'requires', 'conflicts', 'evolved_from'
    FOREIGN KEY (parent_id) REFERENCES patterns(id),
    FOREIGN KEY (child_id) REFERENCES patterns(id)
);
```

### Phase 2: Liquid Templates & Living Patterns

#### The `.liquid` Convention
```
brain/
├── topics/
│   └── web-api/
│       ├── pattern.toml
│       ├── Cargo.toml.liquid      # Dynamic dependencies
│       └── src/
│           ├── main.rs.liquid     # Conditional features
│           └── models.rs          # Static file
```

#### Pattern with Liquid
```toml
# pattern.toml
[pattern]
name = "web-api"
type = "topic"
engine = "liquid"

[variables]
api_name = { prompt = "API name?", default = "my_api" }
database = { prompt = "Database?", choices = ["postgres", "sqlite", "none"] }
auth_type = { prompt = "Auth type?", choices = ["jwt", "oauth", "none"] }

[template]
# Liquid template in main.rs.liquid
content = '''
{% if database != "none" %}
use sqlx::{Pool, {{database | capitalize}}};
{% endif %}
{% if auth_type == "jwt" %}
use jsonwebtoken::{encode, decode, Header, Validation};
{% endif %}

struct {{api_name | pascal_case}}Api {
    {% if database != "none" %}
    db: Pool<{{database | capitalize}}>,
    {% endif %}
    config: Config,
}
'''
```

### Phase 3: Hook System for Active Patterns

#### Modular Hook Architecture
```rust
// Pattern hooks have access to modules
pub struct PatternHooks {
    file_mod: FileModule,      // Create, modify, delete files
    brain_mod: BrainModule,    // Access/modify brain patterns
    cargo_mod: CargoModule,    // Modify Cargo.toml
    env_mod: EnvModule,        // Check environment
    session_mod: SessionModule, // Access current session
}
```

#### Pattern with Hooks
```toml
[hooks.pre]
# Check prerequisites
script = '''
if !env_mod.command_exists("docker") && variables.database == "postgres" {
    error("PostgreSQL pattern requires Docker for local development");
}
'''

[hooks.post]
# Set up after pattern application
script = '''
// Add dependencies based on choices
if variables.auth_type == "jwt" {
    cargo_mod.add_dependency("jsonwebtoken", "9.0");
    cargo_mod.add_dependency("serde", "1.0", ["derive"]);
}

// Start local services
if variables.database == "postgres" {
    system_mod.run("docker-compose up -d postgres");
    brain_mod.add_note("Started PostgreSQL container on port 5432");
}

// Track pattern success
session_mod.mark("Applied {{pattern.name}} with {{variables.database}}");
'''

[hooks.evolve]
# Run when pattern succeeds
script = '''
if tests.passed() && brain_mod.usage_count(pattern.id) > 5 {
    brain_mod.suggest_promotion(pattern.id, "topic");
}
'''
```

## The Complete Digital Garden Flow

### Pattern Lifecycle
```
1. Discover (in project) → "This JWT approach works well"
2. Capture (session end) → Extract as project pattern
3. Plant (add to brain) → Store with Liquid template + hooks
4. Grow (use in projects) → Track success, evolve
5. Harvest (proven) → Promote to topic/core
6. Spread (hive) → Share with team/community
```

### Local-First Hive Architecture
```
~/.patina/
├── brain.db              # Your personal brain
├── gardens/              # Connected brains
│   ├── team/            # Team patterns
│   └── community/       # Public patterns
└── seeds/               # Patterns being tested
```

## Implementation Roadmap

### MVP: Foundation (Weeks 1-2)
1. ✓ Keep existing Claude session workflow
2. ✓ Keep Dagger/Docker/Native environments
3. □ Add SQLite brain storage
4. □ Implement `patina distill` for session → SQL
5. □ Update context generation to query SQL

### Post-MVP: Living Patterns (Weeks 3-4)
1. □ Integrate liquid-rust for templates
2. □ Add `.liquid` file processing
3. □ Variable prompting system
4. □ Pattern success tracking

### Future: Active Gardens (Month 2)
1. □ Rhai scripting for hooks
2. □ Hook module system
3. □ Pattern evolution tracking
4. □ Brain federation/hive

## Why This Architecture Matters

1. **Patterns Evolve**: Not just static templates, but living knowledge
2. **Active Learning**: Hooks track what works, patterns improve
3. **Contextual**: Liquid makes patterns adapt to each project
4. **Distributed**: Local-first but can connect to other gardens
5. **Intelligent**: Patterns learn from usage and evolve

The progression is:
- **Now**: Static patterns in files
- **Soon**: Dynamic patterns with Liquid
- **Future**: Active patterns with hooks
- **Vision**: Evolutionary digital gardens

This preserves our core (environments + Claude) while building toward the intelligent pattern system.