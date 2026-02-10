# Science Commands

> Is it working? How well? Compare alternatives.

## Overview

| Command | Status | One-Line Description |
|---------|--------|---------------------|
| `eval` | EXISTS | Evaluate retrieval quality |
| `bench` | EXISTS | Benchmark with ground truth |
| `compare` | **NEW** | A/B config comparison |
| `feedback` | EXISTS (flag) | Real-world precision from sessions |
| `config` | **NEW** | Manage experiment configurations |

---

## `eval` — Evaluate Retrieval Quality

### What does it do?
Runs quality evaluation across retrieval dimensions (semantic, temporal, dependency).

### Current Interface
```
patina eval                       # Run all dimensions
patina eval --dimension semantic  # Specific dimension
patina eval --feedback            # Real-world precision from sessions
```

### What does it read?
- `.patina/local/data/patina.db` (for queries, ground truth)
- `.patina/local/data/embeddings/.../semantic.usearch` (for semantic eval)

### What does it write?
- stdout (metrics report)

### Who uses it?
- User: "Is retrieval working well for me?"
- Dev: "Did my change improve MRR?"

### When is it used?
Periodic — after config changes, model updates.

### Gaps
- No `--config` flag to specify which config to evaluate
- No `--compare` to diff two configs
- Results not persisted for tracking over time

### Overlaps
- `--feedback` could be its own subcommand

---

## `bench` — Benchmark with Ground Truth

### What does it do?
Run benchmarks against predefined ground truth test cases.

### Current Interface
```
patina bench                      # Run all benchmarks
patina bench --suite <name>       # Specific test suite
```

### What does it read?
- Ground truth files (location?)
- `.patina/local/data/patina.db`
- `.patina/local/data/embeddings/.../semantic.usearch`

### What does it write?
- stdout (benchmark results)

### Who uses it?
- Dev: Validate retrieval against known-good queries

### When is it used?
Rare — during development, before releases.

### Gaps
- Unclear where ground truth is stored
- No CI integration documented

### Overlaps
- Similar to `eval` but with explicit ground truth

---

## `compare` — A/B Config Comparison (NEW)

### What would it do?
Compare two configurations and show delta in metrics.

### Proposed Interface
```
patina science compare baseline experiment
  baseline:     MRR 0.72, P@5 0.65
  experiment:   MRR 0.74, P@5 0.68
  delta:        +0.02 MRR (+2.8%), +0.03 P@5 (+4.6%)
```

### What would it read?
- Two config files (from `~/.patina/configs/` or `.patina/configs/`)
- Same sources as `eval`

### What would it write?
- stdout (comparison report)
- Optionally persist comparison results

### Who would use it?
- User: "Is nomic-embed better than e5-base-v2 for my codebase?"
- Dev: "Did my oracle weight change improve things?"

### When would it be used?
Periodic — when evaluating new models/configs.

### Dependencies
- Requires `config` command to exist first

---

## `feedback` — Real-World Precision (Currently a Flag)

### What does it do?
Shows precision based on actual usage: did queries lead to commits touching those files?

### Current Interface
```
patina eval --feedback
```

### Proposed Interface
```
patina science feedback           # Dedicated subcommand
patina science feedback --session <id>  # Specific session
patina science feedback --days 7  # Time range
```

### What does it read?
- `.patina/local/data/patina.db` (eventlog: `scry.query`, `scry.use`)
- Feedback views (`feedback_query_hits`, `feedback_usage`, etc.)

### What does it write?
- stdout (precision report)

### Who uses it?
- User: "Are my searches actually helping?"
- Dev: "Is the feedback loop working?"

### When is it used?
Rare — for retrospective analysis.

### Gaps
- Currently buried as a flag on `eval`
- Could show more detail (which queries hit, which missed)

---

## `config` — Manage Experiment Configurations (NEW)

### What would it do?
Create, list, and manage experiment configurations (model + adapter + weights).

### Proposed Interface
```
patina science config list                    # Show all configs
patina science config show <name>             # Show config details
patina science config create <name>           # Interactive creation
patina science config create <name> \
  --model e5-base-v2 \
  --adapter claude \
  --belief-weight 0.4                         # Non-interactive

patina science config active                  # Show current active config
patina science config use <name>              # Set active config
```

### Config Schema
```toml
# ~/.patina/configs/experiment.toml
name = "experiment"
created = "2026-02-05"

[model]
name = "e5-base-v2"
# or "nomic-embed-text-v1.5"

[adapter]
name = "claude"
# Informational — which adapter this was tested with

[oracle_weights]
semantic = 0.4
lexical = 0.2
temporal = 0.1
persona = 0.0
belief = 0.3

[notes]
description = "Testing higher belief weight"
```

### What would it read?
- `~/.patina/configs/*.toml` (user-level configs)
- `.patina/configs/*.toml` (project-level configs, optional)

### What would it write?
- Config files (create)
- Active config pointer (use)

### Who would use it?
- User: "I want to try a different model"
- Dev: "I want to test different oracle weights"

### When would it be used?
Periodic — when setting up experiments.

### Dependencies
- Sessions should record active config
- `eval` and `compare` should accept `--config` flag

---

## Summary

| Command | Status | User | Dev | Frequency |
|---------|--------|------|-----|-----------|
| `eval` | Exists | Yes | Yes | Periodic |
| `bench` | Exists | No | Yes | Rare |
| `compare` | **NEW** | Yes | Yes | Periodic |
| `feedback` | Flag → Subcommand | Yes | Yes | Rare |
| `config` | **NEW** | Yes | Yes | Periodic |

## Integration Points

1. **Sessions record config**: `session start` captures active config
2. **Eval accepts config**: `eval --config <name>` uses specific config
3. **Compare uses configs**: `compare A B` loads both and evaluates
4. **Scrape/oxidize respect config**: Model selection from config (future)
