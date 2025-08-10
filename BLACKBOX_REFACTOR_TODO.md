# Black-Box Refactor TODO

## Core Philosophy (Eskil Steenberg-Inspired)
A black-box module should be **conceptually simple** - you can explain what it does in one sentence, even if the implementation is complex inside. The module should be "finished" - stable enough that one developer owns it, everyone else just uses the API, and if needed, it can be completely rewritten with the same interface.

## Goal
Complete the Dependable Rust black-box refactoring by ensuring ALL refactored versions are fully functional drop-in replacements before removing old code.

## Approach
1. Keep BOTH old and refactored versions running in parallel ✅
2. Test BOTH versions thoroughly to ensure feature parity
3. Use environment variables to switch between versions ✅
4. Only remove old code when refactored version is 100% compatible

## Key Learnings

### Black-Box Naming Clarity
**NEW INSIGHT**: A module's name should immediately tell you its purpose. If you need to look inside to understand what it does, the name (and possibly the module boundary) is wrong.

**Good Names** (clear purpose):
- `text_renderer` - Renders text
- `pattern_storage` - Stores patterns  
- `session_tracker` - Tracks sessions
- `container_runner` - Runs containers

**Vague Names** (need investigation):
- `workspace_client` - What's a "workspace"? What does the client do?
- `hybrid_database` - Hybrid of what? For what purpose?
- `navigation_map` - Navigate what? Code? Filesystem? UI?

**Note**: We should identify better names but NOT rename modules yet. First understand what they actually do, then rename in a coordinated effort.

### Order Matters!
**Discovery**: We need to black-box commands FIRST before the indexer, because commands directly use indexer internals. By black-boxing commands, we remove dependencies on internal types, making it possible to properly black-box the indexer.

### When Black-Box Refactoring is Needed vs Not Needed

**Refactoring IS needed when:**
- Module exposes multiple public functions (e.g., agent.rs with start/stop/status/list)
- Module exposes internal types/structs (e.g., workspace_client with CreateWorkspaceRequest)
- Module is large (>500 lines) with mixed concerns
- Module leaks implementation details through its public API

**Refactoring is NOT needed when:**
- Module has single public `execute()` function ✅
- Module is small (<150 lines) with focused responsibility ✅
- Module already hides all implementation details ✅
- The entire file serves as the minimal public API ✅

**Key Principle from modularity-through-interfaces.md:**
> "Modularity comes from small trait surfaces, not small files"

Files like `build.rs` (45 lines, 1 public function) are ALREADY perfect black boxes!

### Domain Primitives vs Implementation Mechanisms

**Domain Primitives (MUST be exposed - they're the language of your system)**:
- Types that define WHAT you're working with (e.g., `Pattern`, `Confidence`, `Layer`, `Location`)
- The "primitives" in format design terms - the semantic meaning
- Types that consumers need to speak the same language
- Example: `Confidence` and `Layer` are how we talk about patterns - hiding them creates a second incompatible format

**Implementation Mechanisms (MUST be hidden - they're HOW you do it)**:
- Storage mechanisms (e.g., `SqliteClient`, `HybridDatabase`)
- State management (e.g., `GitState`, `GitNavigationStateMachine`)
- Synchronization details (e.g., `NavigationCRDT`)
- Internal request structures (e.g., `CreateWorkspaceRequest`)
- Example: `GitState` is HOW we track changes, not WHAT patterns are

**The Key Question**: Is this type part of the problem domain (WHAT) or part of the solution mechanism (HOW)?
- Video editor: `Clip` and `Timeline` = WHAT, `OpenGLContext` = HOW
- Healthcare: `HealthEvent` = WHAT, `SQLDatabase` = HOW  
- Indexer: `Pattern` and `Location` = WHAT, `GitState` and `SqliteClient` = HOW

## Current State (2025-08-10) - UPDATED 13:35

### Today's Progress  
1. ✅ Restored deleted original versions (claude.rs, init/) that were mistakenly removed
2. ✅ Fixed workspace_client_refactored exports with deprecation warnings
3. ✅ Added environment variable switching via config.rs
4. ✅ Set up proper dual-version testing infrastructure
5. ✅ Verified all refactored modules work alongside originals
6. ✅ Black-boxed agent.rs command - single execute() function
7. ✅ Black-boxed dev_env/dagger.rs - hides workspace_client internals

### ✅ Complete Refactors (Ready for Migration)
- [x] `init_refactored` - Complete black-box, single public function (WITH original restored)
- [x] `claude_refactored` - Complete black-box, returns trait object (WITH original restored)
- [x] `navigate_refactored` - Complete black-box, hides all indexer internals (WITH original)
- [x] `workspace_client_refactored` - Exports fixed, ready for use (WITH original)
- [x] `agent_refactored` - Single execute() function, hides workspace_client usage (WITH original)
- [x] `dagger_refactored` - Hides all workspace_client internals (WITH original)

### ✅ Complete Refactors (Ready for Migration)
- [x] `init_refactored` - Complete black-box, single public function (WITH original restored)
- [x] `claude_refactored` - Complete black-box, returns trait object (WITH original restored)
- [x] `navigate_refactored` - Complete black-box, hides all indexer internals (WITH original)
- [x] `workspace_client_refactored` - Exports fixed, ready for use (WITH original)
- [x] `agent_refactored` - Single execute() function, hides workspace_client usage (WITH original)
- [x] `dagger_refactored` - Hides all workspace_client internals (WITH original)
- [x] `indexer_refactored` - Proper ~100-line wrapper exposing domain types, hiding implementation (WITH original)

### Previous Failed Attempt (Now Deleted)
1. Started as simple black-box wrapper (like init/claude)
2. Hit problem: indexer exposes 17+ public types that other code depends on
3. Tried adding exports → naming conflicts
4. Tried fixing conflicts → module dependency issues
5. Spiraled into full rewrite with different architecture

**Why It Failed**:
- The indexer is not a single module, it's an entire subsystem
- Exposes: Database clients, Git state, Navigation state, Pattern storage
- Other code directly uses these internal types
- Can't be black-boxed until consuming code is fixed

**NEW ANALYSIS (2025-08-10 14:17)**:
- Consumers like `navigate.rs` LEGITIMATELY need types: `Confidence`, `Layer`, `Location`, `NavigationResponse`
- These are NOT leaky abstractions - they're the domain model/public API
- The refactor tried to hide ALL types and create replacements (NavigationResult vs NavigationResponse)
- This broke all consumers that depend on these types

**DEEPER ANALYSIS (2025-08-10 14:30) - Based on Format Design Principles**:
After reviewing modular design principles (think Eskil Steenberg's approach):
- The REAL problem: Original indexer exposes **implementation mechanisms** not just primitives
- **Should be exposed** (domain primitives): `Pattern`, `Location`, `Confidence`, `Layer`, `NavigationResponse`
- **Should be hidden** (implementation details): `GitState`, `HybridDatabase`, `NavigationCRDT`, `SqliteClient`, `GitNavigationStateMachine`
- Current indexer is like exposing `OpenGLContext` and `ShaderCompiler` instead of just `Clip` and `Timeline`

**The Correct Refactor Would**:
1. Keep domain types public (Confidence, Layer, Location, Pattern)
2. Hide ALL implementation mechanisms (GitState, HybridDatabase, SqliteClient)
3. Create a clean API that only exposes what we're managing (patterns) not HOW we manage them

**Why Original Refactor Failed**:
- Tried to hide EVERYTHING including legitimate primitives
- Should have only hidden the implementation mechanisms
- Creating new types (NavigationResult) creates duplicate formats - MORE work for everyone

**Proposed Solution**:
Create a proper wrapper that:
- Exposes ONLY: `PatternIndexer`, `Pattern`, `Location`, `Confidence`, `Layer`, `NavigationResponse`
- Hides: All database, git, state machine, CRDT implementation details
- This is the TRUE black-box pattern - hide HOW, expose WHAT

#### `workspace_client_refactored` ✅ FIXED
**Problem**: Hidden request/response structs that other code needs
**Solution Implemented**:
- [x] `CreateWorkspaceRequest` struct - Exported with deprecation warning
- [x] `ExecRequest` struct - Exported with deprecation warning
- [x] `WorkspaceResponse` structs - Not needed, simplified API
- [x] `is_service_running()` function - Exported with deprecation warning

**Consuming Code That Needs It**:
- `dev_env/dagger.rs` - uses CreateWorkspaceRequest, ExecRequest
- `commands/agent.rs` - uses is_service_running()

### 🔍 Module Assessment

#### Commands That Are ALREADY Perfect Black-Boxes ✅
These follow black-box-boundaries.md perfectly - single public function, no exposed internals:
- `build.rs` - 45 lines, only `pub fn execute()` - **Already optimal!**
- `test.rs` - 42 lines, only `pub fn execute()` - **Already optimal!**
- `doctor.rs` - 302 lines, only `pub fn execute(json: bool)` - **Already optimal!**
- `upgrade.rs` - Single execute function - **Already optimal!**
- `version.rs` - Single execute function - **Already optimal!**

#### Successfully Refactored Commands ✅
- [x] `agent.rs` - Had 4 public functions → NOW: agent_refactored with single execute()
- [x] `navigate.rs` - Already refactored → navigate_refactored with single execute()

#### Dev Environments  
- [x] `dev_env/dagger.rs` - Had heavy workspace_client usage → NOW: dagger_refactored
- `dev_env/docker.rs` - Check if needs refactoring
- `dev_env/native.rs` - Doesn't exist yet

#### Modules Needing Clarity (name doesn't explain purpose)
- `workspace_client` - Currently does container/service management? Needs clearer name
- `indexer` - Actually does pattern finding/navigation? Consider `pattern_finder`
- `hybrid_database` - Storage mechanism for what? Needs purpose-driven name

## Testing Requirements

### For Each Refactored Module
1. [ ] Create parallel test that runs BOTH versions
2. [ ] Verify identical outputs for same inputs
3. [ ] Test error handling paths
4. [ ] Benchmark performance (refactored should not be slower)

### Integration Tests
1. [ ] Full init flow with both versions
2. [ ] Full build flow with both versions
3. [ ] Navigation queries with both indexers
4. [ ] Workspace operations with both clients

## Current Strategy (2025-08-10)

### New Approach: Commands First, Indexer Last
1. Black-box all commands that use indexer internals
2. Once commands only use clean APIs, black-box the indexer
3. This avoids the "refactoring spiral" we hit with indexer_refactored

### Progress:
- [x] `navigate_refactored` - DONE, no longer uses indexer internals
- [x] `agent.rs` - DONE, refactored to single execute()
- [x] `build.rs` - Already perfect black-box (no refactor needed)
- [x] `test.rs` - Already perfect black-box (no refactor needed)
- [x] `doctor.rs` - Already perfect black-box (no refactor needed)
- [x] `dev_env/dagger.rs` - DONE, refactored to hide workspace_client internals

## Migration Steps

### Phase 1: Black-Box Commands (IN PROGRESS)
1. [ ] Fix `indexer_refactored` to export required types (with deprecation notes)
2. [x] Fix `workspace_client_refactored` to export required types - DONE ✅
3. [x] Add environment variable switches - DONE (config.rs) ✅
4. [x] Restore deleted original versions - DONE ✅
5. [ ] Test both versions in parallel

### Phase 2: Refactor Consumers (MOSTLY COMPLETE)
1. [x] Black-box `navigate.rs` command - DONE (navigate_refactored)
2. [x] Black-box `agent.rs` command - DONE (agent_refactored)
3. [x] Black-box `dev_env/dagger.rs` - DONE (dagger_refactored)
4. [x] Identified that build.rs, test.rs, doctor.rs are ALREADY perfect black-boxes
5. [ ] Fix the broken indexer_refactored (replace with simple wrapper)

### Phase 3: Final Migration
1. [ ] Run with refactored versions by default for a period
2. [ ] Remove environment variable switches
3. [ ] Remove old implementations
4. [ ] Rename `_refactored` modules to original names

## Dual Version Status ✅

All modules now maintain BOTH original and refactored versions:

| Module | Original | Refactored | Switchable | Status |
|--------|----------|------------|------------|--------|
| Claude Adapter | `src/adapters/claude.rs` | `src/adapters/claude_refactored/` | ✅ via config | Working |
| Init Command | `src/commands/init/` | `src/commands/init_refactored/` | ✅ via config | Working |
| Workspace Client | `src/workspace_client.rs` | `src/workspace_client_refactored/` | ✅ via config | Working |
| Navigate Command | `src/commands/navigate.rs` | `src/commands/navigate_refactored.rs` | ❌ needs switch | Working |
| Agent Command | `src/commands/agent.rs` | `src/commands/agent_refactored.rs` | ✅ via config | Working |
| Dagger Dev Env | `src/dev_env/dagger.rs` | `src/dev_env/dagger_refactored.rs` | ✅ via config | Working |
| Indexer | `src/indexer/` | `src/indexer_refactored/` | ✅ via config | BROKEN - needs fix |

## Environment Variables for Testing

```bash
# Test with all refactored versions
export PATINA_USE_REFACTORED_INIT=1      # Uses init_refactored
export PATINA_USE_REFACTORED_CLAUDE=1    # Uses claude_refactored
export PATINA_USE_REFACTORED_INDEXER=1   # Will use indexer_refactored (when fixed)
export PATINA_USE_REFACTORED_WORKSPACE=1 # Will use workspace_client_refactored

# Run full test suite
cargo test --workspace

# Or test individual versions
PATINA_USE_REFACTORED_INIT=1 cargo run -- init test-project
```

## Success Criteria
- [ ] All tests pass with ONLY refactored versions
- [ ] No performance regression
- [ ] Clean module boundaries achieved through:
  - Small modules with single public function don't need refactoring (already black-boxed)
  - Large/complex modules have <150 line public API wrapping implementation
- [ ] All implementation details hidden
- [ ] No breaking changes for users
- [ ] Recognition that files like build.rs are ALREADY optimal (no unnecessary refactoring)

## Lessons Learned

### What Works
1. **Simple wrappers** (init, claude, navigate) - Just hide implementation, ~100-200 lines
2. **Parallel versions** - Keep both old and new running side-by-side
3. **Single public function** - execute() is all that's needed for commands

### What Doesn't Work
1. **Refactoring complex subsystems directly** - Leads to rewrites (indexer problem)
2. **Adding compatibility exports** - Creates naming conflicts and circular dependencies
3. **Bottom-up refactoring** - Need to fix consumers first, then providers

### Key Insights
- **Order matters**: Black-box from the outside in (commands → libraries → core)
- **Leaky abstractions cascade**: One exposed internal type leads to exposing many
- **Complete rewrites are risky**: Better to wrap existing code than rewrite
- **Test continuously**: Keep both versions working throughout the process

## Notes
- The refactored versions follow Dependable Rust principles
- Black-box boundaries improve maintainability and testing
- Parallel versions allow safe, gradual migration
- Some "compatibility exports" may be needed temporarily
- The indexer_refactored rewrite should probably be abandoned in favor of a simple wrapper