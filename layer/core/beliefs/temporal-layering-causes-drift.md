---
type: belief
id: temporal-layering-causes-drift
status: active
confidence: high
created: 2026-02-05
evidence:
  - layer/surface/build/feat/mother-delivery/analysis-three-servers.md
  - d0-unified-search
---

# Temporal Layering Causes Drift

When new capabilities are added alongside old ones with opt-in flags (like `--hybrid`), neither system gets retired. Features accumulate on both paths because "the old one works" and "the new one isn't proven yet."

## The Pattern

1. System A works well for weeks/months
2. System B is built as a better approach, added with opt-in flag
3. Both systems coexist — "let B prove itself"
4. Features get added to both A and B
5. Neither is retired, drift accelerates
6. You now maintain two diverging implementations

## The Fix

Either:
- **Replace** — Make the new system the default immediately, remove the old
- **Commit** — If keeping both, set a deadline to retire one

The "bridge period" with opt-in flags tends to become permanent unless actively managed.

## Evidence

The CLI/MCP/serve bifurcation in Patina: CLI direct search (Nov 25) → serve as wrapper (Dec 3) → QueryEngine+MCP born together (Dec 12) → `--hybrid` flag as bridge (Dec 16) → features accumulated on both paths (Dec 16 – Feb 2026). Three search implementations, each with own formatting, logging, and persona integration.
