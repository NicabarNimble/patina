//! MCP tool schema definitions
//!
//! All 21 tool schemas live here. handle_list_tools() returns them
//! as the tools/list response.

use super::super::protocol::{Request, Response};

pub(super) fn handle_list_tools(req: &Request) -> Response {
    Response::success(
        req.id.clone(),
        serde_json::json!({
            "tools": [
                {
                    "name": "scry",
                    "description": "Search codebase knowledge - semantic vector search over indexed commits, beliefs, and patterns. Finds conceptually related results even when exact keywords differ. For factual/keyword search (FTS5, temporal co-change, belief grounding), use assay instead. Use expanded_terms to bridge vocabulary gaps between your query and code terminology.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Natural language question or code search query"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["find", "detail", "full", "orient", "recent", "why", "use", "belief"],
                                "default": "find",
                                "description": "Query mode: 'find' (default — returns snippets), 'detail' (fetch full content for one result by query_id+rank), 'full' (full content for all results — escape hatch), 'orient' (structural ranking), 'recent' (temporal ranking), 'why' (explain result), 'use' (log result usage), 'belief' (grounding)"
                            },
                            "query_id": {
                                "type": "string",
                                "description": "Query ID from previous scry response (used with 'detail' and 'use' modes)"
                            },
                            "rank": {
                                "type": "integer",
                                "description": "Result rank (1-based) for 'detail' and 'use' modes"
                            },
                            "path": {
                                "type": "string",
                                "description": "Directory path for orient mode (e.g., 'src/retrieval/')"
                            },
                            "days": {
                                "type": "integer",
                                "default": 7,
                                "description": "Days to look back for recent mode (default: 7)"
                            },
                            "doc_id": {
                                "type": "string",
                                "description": "Document ID for why mode (e.g., 'src/retrieval/engine.rs')"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum results to return (default: 10)",
                                "default": 10
                            },
                            "repo": {
                                "type": "string",
                                "description": "Query a specific registered repo by name (from registry)"
                            },
                            "all_repos": {
                                "type": "boolean",
                                "description": "Query all registered repos (default: false)",
                                "default": false
                            },
                            "include_issues": {
                                "type": "boolean",
                                "description": "Include GitHub issues in results (default: false)",
                                "default": false
                            },
                            "expanded_terms": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Additional search terms to include (LLM-provided synonyms or code-specific terms that bridge vocabulary gap between user query and codebase terminology)"
                            },
                            "belief": {
                                "type": "string",
                                "description": "Belief ID for grounding queries — find nearest code, commits, sessions for a belief (E4.6a). Use instead of query."
                            },
                            "content_type": {
                                "type": "string",
                                "enum": ["code", "commits", "sessions", "patterns", "beliefs"],
                                "description": "Filter results by content type (used with belief mode)"
                            },
                            "impact": {
                                "type": "boolean",
                                "default": true,
                                "description": "Show belief impact for code results — which beliefs reach code via multi-hop grounding (E4.6a). Default: true."
                            }
                        },
                        "required": []
                    }
                },
                {
                    "name": "context",
                    "description": "Get project patterns and conventions - USE THIS to understand design rules before making architectural changes. Returns core patterns (eternal principles) and surface patterns (active architecture). When a topic is provided, includes project beliefs ranked by semantic relevance, plus factual matches (assay keyword search) and semantic matches (scry vector search) fused with facts-first priority.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "topic": {
                                "type": "string",
                                "description": "Optional topic to focus on (e.g., 'error handling', 'testing', 'architecture')"
                            },
                            "repo": {
                                "type": "string",
                                "description": "Query a specific repo by name (from registry)"
                            },
                            "all_repos": {
                                "type": "boolean",
                                "description": "Query all registered repos (default: false)",
                                "default": false
                            }
                        }
                    }
                },
                {
                    "name": "mother",
                    "description": "Search cross-project knowledge - federated FTS5 search over project beliefs and persona values indexed in Mother's graph.db. Returns belief entries from all registered projects and persona. Run `mother graph sync` to populate the index. For project-scoped semantic search, use scry instead.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query text (FTS5 full-text search)"
                            },
                            "limit": {
                                "type": "integer",
                                "default": 10,
                                "description": "Maximum results to return (default: 10)"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["search", "supports", "attacks", "projects"],
                                "default": "search",
                                "description": "Query mode: 'search' (default — FTS5 keyword search), 'supports' (beliefs supporting belief_id), 'attacks' (beliefs attacking belief_id), 'projects' (projects holding belief_id)"
                            },
                            "belief_id": {
                                "type": "string",
                                "description": "Belief ID for supports/attacks/projects modes"
                            }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "assay",
                    "description": "Query codebase structure - modules, imports, functions, call graph. Use for exact structural questions like 'list all modules', 'what imports X', 'show largest files'. For semantic similarity, use scry instead. Use 'derive' to compute/view structural signals (usage, activity, centrality). Use 'search' for ranked FTS5 text search, 'cochange' for temporal co-change analysis, 'belief' for belief grounding.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query_type": {
                                "type": "string",
                                "enum": ["inventory", "imports", "importers", "functions", "callers", "callees", "derive", "search", "cochange", "belief"],
                                "default": "inventory",
                                "description": "Type of structural query. Use 'search' for ranked FTS5 text search, 'cochange' for temporal co-change analysis, 'belief' for belief grounding."
                            },
                            "query": {
                                "type": "string",
                                "description": "Search query text (used with 'search' query_type for ranked FTS5 search across code, commits, and patterns)"
                            },
                            "pattern": {
                                "type": "string",
                                "description": "Path pattern or function name to filter results"
                            },
                            "limit": {
                                "type": "integer",
                                "default": 50,
                                "description": "Maximum results to return"
                            },
                            "repo": {
                                "type": "string",
                                "description": "Query a specific registered repo by name (from registry)"
                            },
                            "all_repos": {
                                "type": "boolean",
                                "default": false,
                                "description": "Query all registered repos (default: false)"
                            }
                        }
                    }
                },
                {
                    "name": "spec.list",
                    "description": "List all specs with optional filters. Returns specs from the filesystem merged with DB data.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "status": {
                                "type": "string",
                                "description": "Filter by status (draft, ready, active, paused, blocked, complete, abandoned)"
                            },
                            "target": {
                                "type": "string",
                                "description": "Filter by target version (e.g., v0.12.0)"
                            }
                        }
                    }
                },
                {
                    "name": "spec.ready",
                    "description": "Show specs ready to work on — status ready/active with all blockers complete.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "spec.blocked",
                    "description": "Show specs blocked by incomplete dependencies, with blocker details.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "spec.next",
                    "description": "Recommend the next spec to work on. Ranks by: active > unblocked > paused (by age) > ready (by impact) > draft.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "spec.promote",
                    "description": "Promote a spec: draft → ready, or ready → active. Creates git tag on activation.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to promote"
                            }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "spec.complete",
                    "description": "Complete an active spec — triggers release (version bump) + archive (git tag + remove). Validates exit criteria are met unless force is true.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to complete"
                            },
                            "major": {
                                "type": "boolean",
                                "default": false,
                                "description": "Force major version bump (for 1.0.0 moments)"
                            },
                            "force": {
                                "type": "boolean",
                                "default": false,
                                "description": "Bypass exit criteria check"
                            }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "spec.abandon",
                    "description": "Abandon a spec — archive without release. Any non-terminal status can be abandoned.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to abandon"
                            },
                            "reason": {
                                "type": "string",
                                "description": "Reason for abandoning"
                            }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "spec.pause",
                    "description": "Pause an active spec. Enforces one-paused-spec rule. Creates WIP commit if dirty, tags state.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to pause"
                            },
                            "reason": {
                                "type": "string",
                                "description": "Why this spec is being paused"
                            }
                        },
                        "required": ["id", "reason"]
                    }
                },
                {
                    "name": "spec.resume",
                    "description": "Resume a paused or blocked spec. For blocked specs, checks all blockers are complete (or use force).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to resume"
                            },
                            "force": {
                                "type": "boolean",
                                "default": false,
                                "description": "Force resume even if blockers aren't complete"
                            }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "spec.block",
                    "description": "Block an active spec on another spec. Appends to blocked_by list, updates dependency tracking.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to block"
                            },
                            "by": {
                                "type": "string",
                                "description": "Blocking spec ID"
                            },
                            "reason": {
                                "type": "string",
                                "description": "Reason for blocking"
                            }
                        },
                        "required": ["id", "by", "reason"]
                    }
                },
                {
                    "name": "spec.split",
                    "description": "Split a spec: complete original with release, create new draft for remaining work.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to split"
                            },
                            "new_id": {
                                "type": "string",
                                "description": "Override new spec ID (defaults to <id>-v2, -v3, etc.)"
                            },
                            "description": {
                                "type": "string",
                                "description": "Description for the new spec's remaining work"
                            }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "spec.show",
                    "description": "Show spec context — frontmatter, body, DESIGN.md, and key files in a single call. Use this to load all spec context before working on it.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to show"
                            },
                            "full": {
                                "type": "boolean",
                                "description": "Return full body and DESIGN.md text (default: false — returns heading outlines + file paths for targeted reading)"
                            }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "spec.check",
                    "description": "Check exit criteria status for a spec. Returns pass/fail with details on which criteria are checked/unchecked. Specs without exit_criteria pass by default.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to check"
                            }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "spec.history",
                    "description": "Show spec lifecycle history from git tags — chronological timeline with timestamps, state transitions, and time-in-state calculations. Works for archived specs.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to show history for"
                            }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "spec.create",
                    "description": "Create a new spec draft — scaffold directory, write frontmatter, commit.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "spec_type": {
                                "type": "string",
                                "description": "Spec type: feat, fix, refactor, explore"
                            },
                            "id": {
                                "type": "string",
                                "description": "Spec identifier (kebab-case)"
                            },
                            "title": {
                                "type": "string",
                                "description": "Human title"
                            },
                            "description": {
                                "type": "string",
                                "description": "One-line problem statement"
                            },
                            "blocked_by": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Spec IDs this is blocked by"
                            },
                            "related": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Related spec IDs"
                            }
                        },
                        "required": ["spec_type", "id"]
                    }
                },
                {
                    "name": "spec.set",
                    "description": "Set a metadata field on a spec. For list fields (beliefs, related, references, blocked_by), prefix value with + to add or - to remove. For scalar fields (target), set directly or pass empty string to clear.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Spec ID to modify"
                            },
                            "field": {
                                "type": "string",
                                "description": "Field to set (beliefs, related, references, blocked_by, target)"
                            },
                            "value": {
                                "type": "string",
                                "description": "Value to set (+value to add, -value to remove for lists; value for scalars)"
                            }
                        },
                        "required": ["id", "field", "value"]
                    }
                },
                {
                    "name": "measure",
                    "description": "Show project health from measurement data — returns JSON summary of all 5 protocol verbs (capture, index, search, believe, evolve) with status, metrics, and action items. Use this to check project health before making recommendations.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "schemas.list",
                    "description": "List installed fact schemas - shows all schema packages installed in the current project with their versions, packages, and fact types. Use this to discover what fact types are available for querying.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "schemas.show",
                    "description": "Show details of an installed fact schema - returns full metadata including facts, embedding config, and index definitions. Use this to understand the structure of a specific fact type before querying or emitting facts.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Schema name (e.g., 'forge')"
                            }
                        },
                        "required": ["name"]
                    }
                }
            ]
        }),
    )
}
