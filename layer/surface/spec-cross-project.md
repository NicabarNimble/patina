# Spec: Cross-Project Queries

## Overview
Projects can query the mothership for cross-project knowledge when local results are insufficient. Results are tagged with their source and adoptability status.

## Query Flow
```
┌─────────────────────────────────────────────────────────────┐
│  User Query: "error handling patterns"                       │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. Query Local Project (.patina/)                          │
│     - Search observations.usearch                            │
│     - Join with observations.db for metadata                 │
└─────────────────────────┬───────────────────────────────────┘
                          │
            ┌─────────────┴─────────────┐
            │                           │
     Results found?              No results / low score
            │                           │
            ▼                           ▼
┌───────────────────┐    ┌────────────────────────────────────┐
│ Return [PROJECT]  │    │  2. Query Mothership (:50051)      │
│ tagged results    │    │     POST /persona/query            │
└───────────────────┘    └─────────────────────┬──────────────┘
                                               │
                                               ▼
                         ┌────────────────────────────────────┐
                         │  3. Evaluate Adoptability          │
                         │     - Check for contradictions     │
                         │     - Tag as [ADOPTABLE] or [REF]  │
                         └─────────────────────┬──────────────┘
                                               │
                                               ▼
                         ┌────────────────────────────────────┐
                         │  4. Return merged results          │
                         │     [PROJECT] + [PERSONA-*]        │
                         └────────────────────────────────────┘
```

## Result Tags

| Tag | Meaning | Display |
|-----|---------|---------|
| `[PROJECT]` | From local project knowledge | Primary results |
| `[PERSONA-RUST]` | From persona, domain-matched | Secondary, relevant |
| `[PERSONA]` | From persona, general | Secondary |
| `[ADOPTABLE]` | Non-contradictory, can adopt | Green indicator |
| `[REFERENCE]` | Contradicts project, show as alternative | Yellow indicator |

## Components

### 1. Query Router
**Location:** `src/query/router.rs`

```rust
pub struct QueryRouter {
    local_index: USearchIndex,
    local_db: SqliteConnection,
    mothership_url: Option<String>,
}

impl QueryRouter {
    pub fn query(&self, query: &str, options: QueryOptions) -> Result<QueryResults> {
        // 1. Always query local first
        let local_results = self.query_local(query, &options)?;

        // 2. Check if we should query mothership
        let should_query_mothership = options.include_persona
            && (local_results.is_empty() || local_results.best_score() < options.threshold);

        let persona_results = if should_query_mothership {
            self.query_mothership(query, &options)?
        } else {
            vec![]
        };

        // 3. Merge and tag results
        let merged = self.merge_results(local_results, persona_results)?;

        Ok(merged)
    }

    fn query_mothership(&self, query: &str, options: &QueryOptions) -> Result<Vec<PersonaResult>> {
        let url = self.mothership_url.as_ref()
            .ok_or_else(|| anyhow!("Mothership not configured"))?;

        let response = reqwest::blocking::Client::new()
            .post(format!("{}/persona/query", url))
            .json(&PersonaQueryRequest {
                query: query.to_string(),
                limit: options.limit,
                threshold: options.threshold,
            })
            .send()?
            .json::<PersonaQueryResponse>()?;

        Ok(response.results)
    }
}
```

### 2. Adoptability Checker
**Location:** `src/query/adoptability.rs`

Determines if a persona belief contradicts project knowledge:

```rust
pub enum Adoptability {
    Adoptable,   // No contradiction, can adopt
    Reference,   // Contradicts project, show as alternative
    Unknown,     // Can't determine
}

pub fn check_adoptability(
    persona_result: &PersonaResult,
    project_beliefs: &[ProjectBelief],
) -> Adoptability {
    // Simple heuristic: check domain overlap and semantic similarity
    for belief in project_beliefs {
        if domains_overlap(&persona_result.domains, &belief.domains) {
            let similarity = cosine_similarity(&persona_result.embedding, &belief.embedding);

            if similarity < 0.3 {
                // Low similarity in same domain = potential contradiction
                return Adoptability::Reference;
            }
        }
    }

    Adoptability::Adoptable
}
```

**Future enhancement:** Use Prolog or LLM reasoning for deeper contradiction detection.

### 3. Result Merger
**Location:** `src/query/merger.rs`

```rust
pub struct MergedResults {
    pub results: Vec<TaggedResult>,
    pub sources: ResultSources,
}

pub struct TaggedResult {
    pub content: String,
    pub score: f32,
    pub source: ResultSource,
    pub adoptability: Option<Adoptability>,
    pub domains: Vec<String>,
    pub metadata: Value,
}

pub enum ResultSource {
    Project,
    Persona { domain: Option<String> },
}

impl ResultMerger {
    pub fn merge(
        local: Vec<LocalResult>,
        persona: Vec<PersonaResult>,
    ) -> MergedResults {
        let mut results = Vec::new();

        // Add local results first (always priority)
        for r in local {
            results.push(TaggedResult {
                content: r.content,
                score: r.score,
                source: ResultSource::Project,
                adoptability: None,  // Local doesn't need adoptability
                domains: r.domains,
                metadata: r.metadata,
            });
        }

        // Add persona results with adoptability check
        for r in persona {
            let adoptability = check_adoptability(&r, &local);
            results.push(TaggedResult {
                content: r.content,
                score: r.score * 0.9,  // Slight penalty for non-local
                source: ResultSource::Persona { domain: r.primary_domain() },
                adoptability: Some(adoptability),
                domains: r.domains,
                metadata: r.metadata,
            });
        }

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        MergedResults {
            results,
            sources: ResultSources {
                project_count: local.len(),
                persona_count: persona.len(),
            },
        }
    }
}
```

### 4. Container Support
**Location:** `src/query/container.rs`

Containers need to reach the host's mothership:

```rust
pub fn get_mothership_url() -> Option<String> {
    // Check if running in container
    if is_in_container() {
        // Use Docker's host gateway
        Some("http://host.docker.internal:50051".to_string())
    } else {
        // Use localhost
        Some("http://127.0.0.1:50051".to_string())
    }
}

fn is_in_container() -> bool {
    // Check for /.dockerenv or cgroup indicators
    Path::new("/.dockerenv").exists()
        || fs::read_to_string("/proc/1/cgroup")
            .map(|s| s.contains("docker") || s.contains("containerd"))
            .unwrap_or(false)
}
```

**YOLO container integration:**
```rust
// In yolo container generation
fn generate_docker_compose() -> String {
    format!(r#"
services:
  dev:
    ...
    extra_hosts:
      - "host.docker.internal:host-gateway"
    environment:
      - PATINA_MOTHERSHIP_URL=http://host.docker.internal:50051
"#)
}
```

### 5. CLI Interface
**Location:** `src/commands/query/mod.rs`

```bash
# Query local only (default)
patina query "error handling"

# Query with persona fallback
patina query --persona "error handling"

# Query persona only
patina query --persona-only "design patterns"

# Show adoptability status
patina query --persona --show-adoptability "error handling"
```

**Output:**
```
[PROJECT] 0.92  Use Result<T, AppError> with thiserror
          src/error.rs:15 | domains: rust, error-handling

[PERSONA-RUST] [ADOPTABLE] 0.87  Always use Result over panics
          domains: rust, error-handling | captured: 2025-11-20

[PERSONA] [REFERENCE] 0.71  Exceptions are fine for truly exceptional cases
          domains: general, error-handling | captured: 2025-11-15
          ⚠️  May conflict with project's no-panic approach
```

## Configuration

**Project config:** `<project>/.patina/config.yaml`
```yaml
query:
  include_persona: true
  threshold: 0.7
  mothership_url: http://127.0.0.1:50051  # or auto-detect
```

**Mothership config:** `~/.patina/config.toml`
```toml
[cross_project]
enabled = true
cache_ttl_seconds = 3600
```

## Acceptance Criteria
- [ ] `patina query` searches local index first
- [ ] `patina query --persona` falls back to mothership when local results poor
- [ ] Results tagged correctly with source
- [ ] Adoptability check identifies potential contradictions
- [ ] Containers can query mothership via `host.docker.internal`
- [ ] Results sorted by score with local priority
