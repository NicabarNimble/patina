-- High-quality observations extracted from Patina documentation
-- Source: CLAUDE.md, layer/core/dependable-rust.md, layer/surface/modular-architecture-plan.md
-- Date: 2025-11-16
-- Purpose: Fill data gaps identified in Topic 0 smoke test

-- =============================================================================
-- CI/CD & Development Workflow (from CLAUDE.md)
-- =============================================================================

INSERT INTO observations (id, observation_type, content, metadata, created_at)
VALUES

-- CI Requirements
('a1b2c3d4-0001-4001-8001-000000000001', 'pattern',
 'Run cargo fmt --all, cargo clippy --workspace, and cargo test --workspace before every push. CI enforces these checks - running locally prevents push failures and saves time.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":1.0,"section":"CI Requirements"}',
 datetime('now')),

-- Testing Workflow
('a1b2c3d4-0001-4001-8001-000000000002', 'pattern',
 'Always build release binary (cargo build --release), install it (cargo install --path .), then test with the actual installed binary. This catches issues that only appear in release mode or with real installations.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":1.0,"section":"Testing Guidelines"}',
 datetime('now')),

-- Git Discipline
('a1b2c3d4-0001-4001-8001-000000000003', 'pattern',
 'Commit often using a scalpel not a shotgun. One commit = one purpose (fix one bug, add one feature, refactor one function). Use git add -p for surgical staging when files have multiple changes.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":1.0,"section":"Git Discipline"}',
 datetime('now')),

('a1b2c3d4-0001-4001-8001-000000000004', 'pattern',
 'Keep commit messages clean and professional. Focus on what changed and why, not who or what tool wrote it. Never add AI attribution like Generated with Claude Code or Co-Authored-By.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":1.0,"section":"Git Commit Guidelines"}',
 datetime('now')),

-- Design Philosophy
('a1b2c3d4-0001-4001-8001-000000000005', 'pattern',
 'Patina design philosophy: Knowledge First (patterns are core value), LLM Agnostic (work where AI lives), Container Native (reproducible everywhere), Escape Hatches (never lock users in).',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":1.0,"section":"Design Philosophy"}',
 datetime('now')),

-- =============================================================================
-- Module Structure & Organization (from dependable-rust.md)
-- =============================================================================

-- Module Size Limits
('a1b2c3d4-0002-4001-8001-000000000001', 'pattern',
 'Keep external interface (mod.rs) under 150 lines: module docs, type names, minimal constructors, and curated pub use statements only. Push implementation details to internal.rs or internal/ folder.',
 '{"source_type":"documentation","source_id":"dependable-rust.md","reliability":1.0,"section":"External interface rules"}',
 datetime('now')),

('a1b2c3d4-0002-4001-8001-000000000002', 'pattern',
 'Extract to internal.rs or internal/ folder when: module exceeds 150 lines, has complex implementation logic, or needs multiple helper functions. Keep external interface stable and small.',
 '{"source_type":"documentation","source_id":"dependable-rust.md","reliability":0.95,"section":"Canonical layout"}',
 datetime('now')),

-- Module Extraction Criteria
('a1b2c3d4-0002-4001-8001-000000000003', 'pattern',
 'Extract module when it has single clear responsibility, exceeds 150 LOC in external interface, or has 3+ distinct concerns. Signal for extraction: multiple impl blocks, unclear naming, or and in module name.',
 '{"source_type":"documentation","source_id":"modular-architecture-plan.md","reliability":0.95,"section":"Module Decomposition"}',
 datetime('now')),

-- Visibility Patterns
('a1b2c3d4-0003-4001-8001-000000000001', 'technology',
 'Default to pub(crate) for internal items. Only the external interface (mod.rs) decides what becomes pub. This keeps API surface small and prevents accidental exposure of implementation details.',
 '{"source_type":"documentation","source_id":"dependable-rust.md","reliability":1.0,"section":"Visibility pattern"}',
 datetime('now')),

-- Error Handling
('a1b2c3d4-0003-4001-8001-000000000002', 'technology',
 'Provide single Error enum per module, mark as non_exhaustive if variants may grow. Export from mod.rs, implement in internal.rs. Makes error handling predictable and documentation clear.',
 '{"source_type":"documentation","source_id":"dependable-rust.md","reliability":1.0,"section":"External interface rules"}',
 datetime('now')),

-- =============================================================================
-- Testing Strategy (from dependable-rust.md)
-- =============================================================================

-- Test Organization
('a1b2c3d4-0004-4001-8001-000000000001', 'pattern',
 'Three-layer testing: Doctests in mod.rs show intended usage, unit tests colocated under internal/ for edge cases, integration tests in tests/ exercise only external interface.',
 '{"source_type":"documentation","source_id":"dependable-rust.md","reliability":1.0,"section":"Testing strategy"}',
 datetime('now')),

('a1b2c3d4-0004-4001-8001-000000000002', 'pattern',
 'Add at least one runnable doctest to every public module. Doctests serve as executable documentation and verify that examples compile and run correctly.',
 '{"source_type":"documentation","source_id":"dependable-rust.md","reliability":1.0,"section":"External interface rules"}',
 datetime('now')),

-- =============================================================================
-- Architecture Patterns (from modular-architecture-plan.md)
-- =============================================================================

-- Tool vs System Pattern
('a1b2c3d4-0005-4001-8001-000000000001', 'pattern',
 'Design modules as Tools not Systems. Tools have: stateless operation, clear input → output transformation, single responsibility, no coordination logic. Systems coordinate multiple tools.',
 '{"source_type":"documentation","source_id":"modular-architecture-plan.md","reliability":1.0,"section":"Module Responsibilities"}',
 datetime('now')),

-- Module Success Criteria
('a1b2c3d4-0005-4001-8001-000000000002', 'pattern',
 'Module success criteria: single responsibility, can be tested independently, clear input → output interface, no circular dependencies, LLMs can easily understand purpose.',
 '{"source_type":"documentation","source_id":"modular-architecture-plan.md","reliability":1.0,"section":"Success Criteria"}',
 datetime('now')),

-- Refactoring Strategy
('a1b2c3d4-0005-4001-8001-000000000003', 'pattern',
 'When refactoring: start with simplest extractions first (read-only operations), maintain backward compatibility during migration, run both systems in parallel, keep rollback plan ready.',
 '{"source_type":"documentation","source_id":"modular-architecture-plan.md","reliability":0.95,"section":"Implementation Plan"}',
 datetime('now')),

-- Dependency Management
('a1b2c3d4-0005-4001-8001-000000000004', 'pattern',
 'Avoid circular dependencies between modules. If two modules need each other, extract shared interface to third module, or reconsider module boundaries - may indicate wrong decomposition.',
 '{"source_type":"documentation","source_id":"modular-architecture-plan.md","reliability":0.95,"section":"Success Criteria"}',
 datetime('now')),

-- =============================================================================
-- Container & Build System (from CLAUDE.md)
-- =============================================================================

-- Docker Philosophy
('a1b2c3d4-0006-4001-8001-000000000001', 'technology',
 'Use Docker for containerized builds and tests. Never require specific tools beyond Docker. Provides reproducible builds across different development environments.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":1.0,"section":"Build System"}',
 datetime('now')),

-- Rust Tooling
('a1b2c3d4-0006-4001-8001-000000000002', 'technology',
 'Use Rust for CLI and core logic - let the compiler be your guard rail. Rust type system catches errors at compile time that would be runtime bugs in other languages.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":1.0,"section":"Development Guidelines"}',
 datetime('now')),

-- =============================================================================
-- Pattern Evolution (from CLAUDE.md)
-- =============================================================================

-- Pattern Lifecycle
('a1b2c3d4-0007-4001-8001-000000000001', 'pattern',
 'Patterns evolve: projects → topics → core. Project-specific patterns become topic patterns when proven across multiple projects, then core patterns when universal and stable.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":0.95,"section":"Development Guidelines"}',
 datetime('now')),

-- Escape Hatches
('a1b2c3d4-0007-4001-8001-000000000002', 'pattern',
 'Always provide escape hatches. Never lock users into a specific workflow, tool, or pattern. Users should be able to drop down to lower abstraction levels when needed.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":1.0,"section":"Development Guidelines"}',
 datetime('now')),

-- =============================================================================
-- Naming & Documentation (from dependable-rust.md & modular-architecture-plan.md)
-- =============================================================================

-- Naming Conventions
('a1b2c3d4-0008-4001-8001-000000000001', 'pattern',
 'Keep names boring and clear for LLM comprehension. Prefer environment-provider over EnvironmentFactory, git-manager over VCSAdapter. Clear beats clever.',
 '{"source_type":"documentation","source_id":"modular-architecture-plan.md","reliability":0.95,"section":"Notes"}',
 datetime('now')),

('a1b2c3d4-0008-4001-8001-000000000002', 'pattern',
 'Default to internal.rs for private implementation. Team-approved alternatives: implementation.rs or imp.rs. Reserve sys/ or ffi/ for low-level bindings.',
 '{"source_type":"documentation","source_id":"dependable-rust.md","reliability":1.0,"section":"Naming policy"}',
 datetime('now')),

-- =============================================================================
-- Session Management (from CLAUDE.md)
-- =============================================================================

-- Session Workflow
('a1b2c3d4-0009-4001-8001-000000000001', 'technology',
 'Session commands: /session-git-start <name> begins session with Git tracking, /session-git-update tracks progress, /session-git-note captures insights, /session-git-end distills learnings.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":1.0,"section":"Session Management"}',
 datetime('now')),

('a1b2c3d4-0009-4001-8001-000000000002', 'pattern',
 'Session tracking integrates Git workflow: automatic tagging at session boundaries, work classification based on Git metrics, failed experiments preserved as memory.',
 '{"source_type":"documentation","source_id":"CLAUDE.md","reliability":0.95,"section":"Session-Git Commands"}',
 datetime('now'));

-- Verify insertions
SELECT
    observation_type,
    COUNT(*) as count
FROM observations
WHERE id LIKE 'doc_%'
GROUP BY observation_type
ORDER BY count DESC;

SELECT
    json_extract(metadata, '$.source_id') as source_doc,
    COUNT(*) as count
FROM observations
WHERE id LIKE 'doc_%'
GROUP BY source_doc;
