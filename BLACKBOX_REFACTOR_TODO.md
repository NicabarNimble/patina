# Black-Box Refactor TODO

## Goal
Complete the Dependable Rust black-box refactoring by ensuring ALL refactored versions are fully functional drop-in replacements before removing old code.

## Approach
1. Keep BOTH old and refactored versions running in parallel
2. Test BOTH versions thoroughly to ensure feature parity
3. Use environment variables to switch between versions
4. Only remove old code when refactored version is 100% compatible

## Key Learning: Order Matters!
**Discovery**: We need to black-box commands FIRST before the indexer, because commands directly use indexer internals. By black-boxing commands, we remove dependencies on internal types, making it possible to properly black-box the indexer.

## Current State (2025-08-10)

### ✅ Complete Refactors (Ready for Migration)
- [x] `init_refactored` - Complete black-box, single public function
- [x] `claude_refactored` - Complete black-box, returns trait object
- [x] `navigate_refactored` - Complete black-box, hides all indexer internals

### ❌ Incomplete Refactors (Need Work)

#### `indexer_refactored` - MAJOR ISSUE DISCOVERED
**Problem**: This is NOT a simple wrapper - it's a complete 4000+ line REWRITE!

**What Happened**:
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

**Solution**: Black-box commands FIRST to remove dependencies on indexer internals

#### `workspace_client_refactored`
**Problem**: Hidden request/response structs that other code needs
**Required Exports**:
- [ ] `CreateWorkspaceRequest` struct
- [ ] `ExecRequest` struct
- [ ] `WorkspaceResponse` structs
- [ ] `is_service_running()` function

**Consuming Code That Needs It**:
- `dev_env/dagger.rs` - uses CreateWorkspaceRequest, ExecRequest
- `commands/agent.rs` - uses is_service_running()

### 🔍 Modules That Need Black-Boxing

#### Commands
- [ ] `navigate.rs` - Exposes internal logic, should have clean execute() only
- [ ] `agent.rs` - Mixed concerns, should hide implementation
- [ ] `build.rs` - Should hide build logic
- [ ] `test.rs` - Should hide test logic
- [ ] `doctor.rs` - Should hide diagnostic logic

#### Dev Environments  
- [ ] `dev_env/dagger.rs` - Exposes WorkspaceClient usage
- [ ] `dev_env/docker.rs` - Should hide Docker details
- [ ] `dev_env/native.rs` - Should hide native build details

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
- [ ] `agent.rs` - Uses workspace_client internals
- [ ] `build.rs` - Needs black-boxing
- [ ] `test.rs` - Needs black-boxing  
- [ ] `doctor.rs` - Needs black-boxing
- [ ] `dev_env/dagger.rs` - Uses workspace_client internals

## Migration Steps

### Phase 1: Black-Box Commands (IN PROGRESS)
1. [ ] Fix `indexer_refactored` to export required types (with deprecation notes)
2. [ ] Fix `workspace_client_refactored` to export required types
3. [ ] Add environment variable switches for indexer and workspace_client
4. [ ] Test both versions in parallel

### Phase 2: Refactor Consumers
1. [ ] Black-box `navigate.rs` command
2. [ ] Black-box `agent.rs` command  
3. [ ] Black-box `dev_env/dagger.rs`
4. [ ] Update these to use cleaner APIs where possible

### Phase 3: Final Migration
1. [ ] Run with refactored versions by default for a period
2. [ ] Remove environment variable switches
3. [ ] Remove old implementations
4. [ ] Rename `_refactored` modules to original names

## Environment Variables for Testing

```bash
# Test with all refactored versions
export PATINA_USE_REFACTORED_INIT=1
export PATINA_USE_REFACTORED_CLAUDE=1  
export PATINA_USE_REFACTORED_INDEXER=1
export PATINA_USE_REFACTORED_WORKSPACE=1

# Run full test suite
cargo test --workspace
```

## Success Criteria
- [ ] All tests pass with ONLY refactored versions
- [ ] No performance regression
- [ ] Clean module boundaries (no more than 150 lines public API per module)
- [ ] All implementation details hidden
- [ ] No breaking changes for users

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