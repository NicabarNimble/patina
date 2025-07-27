# Dagger Container-Use Pattern for Patina

## Overview
Based on https://github.com/dagger/container-use - a pattern for interactive container development that Patina should adopt for agent workflows.

## Key Concepts

### Container-Use Pattern
- Keep a container running for the duration of development
- Execute commands inside the persistent container
- Maintain state between commands (unlike traditional CI)
- Enable interactive workflows with AI agents

### Current Issues
1. **Wrong Directory Mount**: Currently mounting pipelines/ as workspace instead of project root
2. **Session Isolation**: Need to maintain container sessions for agent work
3. **Context Preservation**: Container should preserve work between commands

### Implementation Direction

1. **Fix Directory Mounting**
   ```go
   // Should mount parent directory (project root) not pipelines/
   WithDirectory("/workspace", client.Host().Directory(".."))
   ```

2. **Session-Based Containers**
   - Each agent session gets its own container
   - Container persists for the session duration
   - Can execute multiple commands in same context

3. **Agent Workflow Integration**
   ```bash
   patina agent start    # Start persistent container
   patina agent exec ... # Run commands in container
   patina agent stop     # Clean up container
   ```

## Connection to Brain Architecture

This aligns with our findings that:
- Dagger is infrastructure in the environment dimension
- Provides safe sandbox for autonomous agent work
- Enables the LLM to test and iterate without breaking host
- Brain patterns guide what the agent does in the container

## Next Steps
1. Study the container-use example implementation
2. Adapt pattern for Patina's agent command
3. Enable persistent containers for development sessions
4. Integrate with brain patterns for guided execution