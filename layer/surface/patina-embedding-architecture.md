---
id: patina-embedding-architecture
layer: surface
status: emerging
created: 2025-11-20
tags: [architecture, embeddings, adapters, ml, vision]
references: [adapter-pattern, patina-system-architecture, session-capture]
---

# Patina Embedding Architecture
**Progressive Adapter Design for Context-Aware Code Understanding**

> One engine, variable patina thickness. Same architecture, different training data richness.

---

## Executive Summary

Patina uses a **progressive adapter architecture** that transforms general-purpose embeddings into multidimensional code understanding. Rather than fine-tuning a base model (risking catastrophic forgetting), we freeze E5-base-v2 and train small specialized adapters that act as "lenses" highlighting different code relationships.

**Key Concepts:**
- **Base Model**: E5-base-v2 (768-dim, frozen, never changes)
- **Dimension Adapters**: 6 small MLPs (1-2M params each) trained on specific relationships
- **Output**: ~2,000-dimensional multidimensional embedding
- **Patina Thickness**: Training data richness determines adapter quality, not architecture changes

---

## Why Adapters Instead of Fine-Tuning?

### Option A: Fine-Tune E5 Directly (Bad)

```python
model = E5BaseV2()  # 110M parameters

for epoch in range(10):
    for batch in your_pairs:
        loss = train_step(model, batch)
        model.update()  # Updates ALL 110M parameters

# Problems:
# - Risk destroying E5's general knowledge (catastrophic forgetting)
# - Need 100K+ pairs to avoid degradation
# - Training takes days/weeks
# - If you mess up, you lose E5's quality
```

### Option B: Adapters (Good)

```python
base_model = E5BaseV2()
base_model.freeze()  # Lock weights forever

adapter = Adapter(input=768, hidden=1024, output=256)  # 1-2M params

for epoch in range(10):
    for batch in your_pairs:
        base_emb = base_model(batch)  # E5 unchanged
        adapted = adapter(base_emb)    # Only adapter trains
        loss = compute_loss(adapted)
        adapter.update()  # Updates only 1-2M params

# Benefits:
# - E5 quality preserved forever
# - Only need 10K pairs per adapter
# - Training takes hours on Mac Studio
# - Can't break E5 - it's frozen
# - Train multiple adapters independently
```

---

## The Six Dimensions

Each adapter transforms the 768-dim E5 embedding to highlight specific relationships:

| Dimension | Output Dim | Training Data | What It Highlights |
|-----------|------------|---------------|-------------------|
| **Semantic** | 768 | Session observations | Meaning relationships, domain concepts |
| **Temporal** | 256 | Git co-change history | Files that change together over time |
| **Dependency** | 256 | Call graph analysis | Functions that call each other |
| **Syntactic** | 256 | AST similarity | Similar code structure/patterns |
| **Architectural** | 256 | Directory structure | Position in system hierarchy |
| **Social** | 256 | GitHub metadata | Contributor/issue/PR relationships |

**Total output: ~2,000 dimensions** (semantic + 5 specialized adapters)

### Analogy: Photographic Filters

E5-base-v2 is like a high-resolution photograph of code - captures everything but treats all aspects equally. Adapters are specialized filters:

- **Temporal adapter** = Motion blur filter (highlights what moves together)
- **Dependency adapter** = Blueprint filter (highlights structural connections)
- **Semantic adapter** = Color filter (highlights meaning relationships)
- **Syntactic adapter** = Pattern filter (highlights similar shapes)
- **Architectural adapter** = Zoom filter (highlights position in system)

---

## Adapter Architecture

### Single Adapter Structure

```python
class Adapter:
    def __init__(self, input_dim=768, hidden_dim=1024, output_dim=256):
        # Layer 1: Expand dimensions
        self.fc1 = Linear(input_dim, hidden_dim)  # 768 → 1024

        # Layer 2: Compress to specialized space
        self.fc2 = Linear(hidden_dim, output_dim) # 1024 → 256

        # Total parameters: (768*1024) + (1024*256) = ~1M params

    def forward(self, x):
        h = relu(self.fc1(x))      # Hidden layer (1024-dim)
        output = self.fc2(h)        # Specialized embedding (256-dim)
        return output
```

### Multidimensional Embedding

```python
def embed_multidimensional(text: str) -> np.ndarray:
    # Step 1: Get E5 embedding (one forward pass)
    base_emb = e5.encode(text)  # 768-dim

    # Step 2: Pass through all adapters (6 forward passes)
    semantic_emb = semantic_adapter(base_emb)       # 768-dim
    temporal_emb = temporal_adapter(base_emb)       # 256-dim
    dependency_emb = dependency_adapter(base_emb)   # 256-dim
    syntactic_emb = syntactic_adapter(base_emb)     # 256-dim
    architectural_emb = architectural_adapter(base_emb)  # 256-dim
    social_emb = social_adapter(base_emb)           # 256-dim

    # Step 3: Concatenate into multidimensional vector
    return np.concatenate([
        semantic_emb,       # 768
        temporal_emb,       # 256
        dependency_emb,     # 256
        syntactic_emb,      # 256
        architectural_emb,  # 256
        social_emb,         # 256
    ])  # Total: 2,304 dimensions
```

---

## Training Data Sources

### Data Source Reality

| Dimension | Data Source | Available in New Repo? |
|-----------|-------------|------------------------|
| Semantic | observations.db (from sessions) | No - needs lived experience |
| Syntactic | code.db (Tree-sitter AST) | Yes - extract from code |
| Dependency | call_graph table | Yes - extract from code |
| Temporal | Git commit history | Yes - exists from day 1 |
| Architectural | File paths + directory structure | Yes - exists from day 1 |
| Social | GitHub API (issues, PRs, contributors) | Yes - if GitHub repo |

**Key insight**: Only semantic dimension requires sessions. The other 5 can be trained from code + git alone.

### Training Pair Generation

```python
# Temporal: Git co-change pairs
def generate_temporal_pairs(git_history):
    pairs = []
    for commit in git_history:
        files_changed = commit.files
        # Positive: files changed in same commit
        for f1, f2 in combinations(files_changed, 2):
            pairs.append((f1, f2, similar=True))
    # Negative: files never changed together
    for pair in pairs:
        pair['negative'] = sample_non_cochanged_file(pair['anchor'])
    return pairs

# Semantic: Session observation pairs
def generate_semantic_pairs(observations):
    pairs = []
    for obs in observations:
        # Observations from same session are related
        same_session = get_same_session_observations(obs)
        for related in same_session:
            pairs.append((obs.content, related.content, similarity=0.8))
    return pairs

# Dependency: Call graph pairs
def generate_dependency_pairs(call_graph):
    pairs = []
    for caller, callees in call_graph.items():
        for callee in callees:
            pairs.append((caller, callee, similar=True))
    return pairs
```

### Contrastive Training

```python
def contrastive_loss(anchor, positive, negative, margin=0.5):
    pos_dist = sum((anchor - positive) ** 2)
    neg_dist = sum((anchor - negative) ** 2)
    loss = max(0, pos_dist - neg_dist + margin)
    return loss

def train_adapter(adapter, pairs, epochs=10):
    optimizer = Adam(learning_rate=1e-4)

    for epoch in range(epochs):
        for batch in batches(pairs, batch_size=32):
            anchor_adapted = adapter(batch['anchor'])
            positive_adapted = adapter(batch['positive'])
            negative_adapted = adapter(batch['negative'])

            loss = contrastive_loss(anchor_adapted, positive_adapted, negative_adapted)
            optimizer.update(adapter, loss)
```

---

## Patina Thickness Model

Same adapter architecture, progressively richer training data:

### Fresh Patina (Thin)

```python
# Temporal adapter trained on:
temporal_pairs = [
    # Only git co-change (10K pairs)
    ("file_a", "file_b", 0.8),  # Changed in same commit
]

# Semantic adapter trained on:
semantic_pairs = [
    # Only code text similarity (5K pairs)
    ("spawn_entity", "create_entity", 0.7),  # Similar function names
]
```
**Quality**: Basic structural understanding

### Working Patina (Building)

```python
# Temporal adapter trained on:
temporal_pairs = [
    # Git (10K) + Session mentions (2K)
    ("file_a", "file_b", 0.8),  # Git co-change
    ("file_a", "file_c", 0.9),  # Mentioned together in session
]

# Semantic adapter trained on:
semantic_pairs = [
    # Code text (5K) + Session observations (3K)
    ("spawn_entity", "create_entity", 0.7),  # Similar names
    ("spawn_entity", "world.spawn()", 0.95), # Session observed this
]
```
**Quality**: Emerging contextual understanding

### Mature Patina (Thick)

```python
# Temporal adapter trained on:
temporal_pairs = [
    # Git (10K) + Session mentions (5K) + Cross-session patterns (2K)
    ("file_a", "file_b", 0.8),  # Git co-change
    ("file_a", "file_c", 0.9),  # Mentioned in 5 sessions
    ("file_c", "file_d", 0.95), # Pattern observed across 12 sessions
]

# Semantic adapter trained on:
semantic_pairs = [
    # Code (5K) + Sessions (15K) + Domain graph (5K)
    ("spawn_entity", "create_entity", 0.7),      # Similar names
    ("spawn_entity", "world.spawn()", 0.95),     # Session observed
    ("ECS pattern", "component caching", 0.92),  # Domain relationship
    ("gas optimization", "ECS caching", 0.88),   # Cross-cutting insight
]
```
**Quality**: Deep contextual wisdom

---

## Mothership Architecture

### Global Semantic Adapter

The mothership trains a cross-project semantic adapter from all primary projects:

```
~/.patina/
├─ persona/
│  ├─ persona.db                      # Your cross-project beliefs
│  └─ models/
│     └─ semantic_global.onnx         # Trained on ALL observations
│
└─ projects.registry                  # Tracks which projects contribute
```

### Three-Tier Project Model

```yaml
# ~/.patina/projects.registry
projects:
  patina:
    type: primary                      # You own it
    path: ~/projects/patina
    sessions: 277
    observations: 992
    adapters:
      semantic: local                  # Trained on 992 observations
      syntactic: local
      dependency: local
      temporal: local
      architectural: local

  new-starknet-game:
    type: primary                      # You own it, but just started
    path: ~/projects/new-starknet-game
    sessions: 0
    observations: 0
    adapters:
      semantic: global                 # Uses mothership global adapter
      syntactic: local                 # Trained from code immediately
      dependency: local
      temporal: local
      architectural: local

  dojo:
    type: reference                    # External repo
    path: ~/projects/dojo
    sessions: 0
    observations: 0
    adapters:
      semantic: none                   # Reference repos don't need semantic
      syntactic: local
      dependency: local
      temporal: local
      architectural: local
```

### Adapter Fallback Strategy

| Project Type | Semantic | Other Dimensions |
|--------------|----------|------------------|
| Primary (mature) | Local (sessions exist) | Local |
| Primary (new) | Global fallback | Local |
| Contributor | Mixed (some sessions) | Local |
| Reference | None needed | Local |

---

## Query Flow

### Dimension-Weighted Search

```python
def search(query: str, weights: dict) -> list:
    query_emb = embed_multidimensional(query)

    results = []
    for candidate in codebase:
        candidate_emb = embed_multidimensional(candidate)

        # Per-dimension similarity
        sim_semantic = cosine_similarity(query_emb[0:768], candidate_emb[0:768])
        sim_temporal = cosine_similarity(query_emb[768:1024], candidate_emb[768:1024])
        # ... etc

        # Weighted combination based on query intent
        final_similarity = (
            weights['semantic'] * sim_semantic +
            weights['temporal'] * sim_temporal +
            weights['dependency'] * sim_dependency +
            weights['syntactic'] * sim_syntactic +
            weights['architectural'] * sim_architectural +
            weights['social'] * sim_social
        )

        results.append((candidate, final_similarity))

    return sorted(results, key=lambda x: x[1], reverse=True)
```

### Query Examples

```bash
# Recent changes (temporal focus)
patina search "gas optimization" --weights temporal=1.0,semantic=0.5
# → Returns: Files recently modified with gas optimization work

# Architectural understanding (structure focus)
patina search "inventory system" --weights architectural=1.0,dependency=0.8
# → Returns: All inventory-related modules and their dependencies

# Code duplication detection (syntactic focus)
patina search "error handling pattern" --weights syntactic=1.0,semantic=0.3
# → Returns: Functions with similar error handling structure

# Session-guided (semantic focus)
patina search "ECS caching pattern" --weights semantic=1.0,temporal=0.5
# → Returns: Code matching session observations about ECS
```

---

## LiveStore Event Sourcing

### Adapters as Materialized Views

Following event-sourcing pattern, adapters are materialized views:

```
.patina/
├─ shared/events/ (committed to git)
│  ├─ code_indexed.json               # patina scrape code
│  ├─ git_pairs_generated.json        # patina train generate-pairs --source git
│  ├─ adapters_materialized.json      # patina train adapters
│  └─ session_*.json                  # Session events
│
└─ data/ (.gitignored - rebuildable)
   ├─ code.db                          # Materialized from code
   ├─ observations.db                  # Empty initially
   ├─ training/                        # Materialized training pairs
   │  ├─ syntactic_pairs.jsonl
   │  ├─ dependency_pairs.jsonl
   │  ├─ temporal_pairs.jsonl
   │  └─ architectural_pairs.jsonl
   │
   └─ models/                          # Materialized adapters
      ├─ syntactic.onnx                # Local
      ├─ dependency.onnx               # Local
      ├─ temporal.onnx                 # Local
      ├─ architectural.onnx            # Local
      └─ semantic.onnx -> ~/.patina/persona/models/semantic_global.onnx
         # Symlink to mothership global adapter (until local matures)
```

### Progressive Adapter Evolution

```
Day 1: patina init new-project
├─ Event: project_initialized.json
├─ Action: patina scrape code
└─ Materialize: 4 local adapters (no semantic yet)

Week 2: First sessions captured
├─ Events: session_20251120-*.json (10 sessions)
├─ Observations: 15 observations accumulated
└─ Still too few for local semantic → keep using global

Month 2: Regular development
├─ Events: 50+ sessions
├─ Observations: 150+ observations
└─ Threshold reached → materialize local semantic adapter

Month 6: Mature project
├─ Events: 200+ sessions
├─ Observations: 600+ observations
└─ Local semantic adapter now better than global for this domain
```

---

## Commands

```bash
# Check adapter status
patina adapters status
# Output:
# ✓ syntactic:     local (689 functions, trained)
# ✓ dependency:    local (8,475 calls, trained)
# ✓ temporal:      local (234 commits, trained)
# ✓ architectural: local (156 files, trained)
# ⚠ semantic:      global (0 local observations, using mothership)
#
# Tip: Accumulate 100+ observations to train local semantic adapter

# Train available adapters
patina train adapters --fallback global

# Retrain semantic after sessions accumulate
patina train adapters --retrain semantic

# Specify adapter sources manually
patina train adapters \
  --semantic global \
  --syntactic local \
  --dependency local \
  --temporal local \
  --architectural local

# Rebuild from events (after clone)
patina materialize adapters
```

---

## Benefits

1. **Preserves E5 Quality** - Base model trained on billions of pairs, never degraded
2. **Data Efficient** - 10K pairs per adapter (vs 100K+ for fine-tuning)
3. **Fast Training** - Hours on Mac Studio (vs days for full model)
4. **Independently Trainable** - Train all 6 in parallel, failures isolated
5. **Extensible** - Add new dimensions without retraining existing adapters
6. **Debuggable** - Evaluate each dimension separately
7. **Patina Growth Compatible** - Same architecture, richer data over time

---

## References

- `layer/core/adapter-pattern.md` - Trait-based adapter design principle
- `layer/surface/patina-system-architecture.md` - v0.1.0 implementation details
- `layer/core/session-capture.md` - Session system that generates semantic training data
- Session 20251120-110914 - "One Engine, Variable Patina Thickness" design
- Session 20251119-061119 - Islands & Gods, Mothership architecture

---

**Document Metadata:**
- **Created**: 2025-11-20
- **Status**: Emerging (designed, not yet implemented)
- **Scope**: ML embedding architecture for Patina v0.2+
- **Audience**: Implementers, contributors understanding the vision
