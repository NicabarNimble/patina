---
id: hook-based-automation
version: 1
created_date: 2025-07-29
confidence: medium
oxidizer: nicabar
tags: []
---

# Hook-Based Automation Pattern

## Overview
A pattern for automating development workflows using lifecycle hooks that trigger at specific events during AI-assisted development sessions.

## Core Concept
Hooks provide programmable interception points that:
- Execute automatically at defined lifecycle events
- Capture context without manual intervention
- Enable workflow automation
- Maintain separation between capture and processing

## Hook Types and Use Cases

### 1. Pre-Tool Hooks
Execute before tool usage, can block execution:
```json
{
  "PreToolUse": [{
    "matcher": "Write|Edit",
    "hooks": [{
      "type": "command",
      "command": ".hooks/check-permissions.sh"
    }]
  }]
}
```

Use cases:
- Permission validation
- Safety checks
- Context preparation

### 2. Post-Tool Hooks
Execute after tool completion:
```json
{
  "PostToolUse": [{
    "matcher": ".*",
    "hooks": [{
      "type": "command",
      "command": ".hooks/log-activity.sh"
    }]
  }]
}
```

Use cases:
- Activity logging
- Change tracking
- Metric collection

### 3. Session Lifecycle Hooks
Mark session boundaries:
```json
{
  "UserPromptSubmit": [{
    "hooks": [{
      "type": "command",
      "command": ".hooks/session-start.sh"
    }]
  }],
  "Stop": [{
    "throttle": 300000,
    "hooks": [{
      "type": "command",
      "command": ".hooks/session-end.sh"
    }]
  }]
}
```

Use cases:
- Session initialization
- Cleanup operations
- Trigger processing

### 4. Sub-Agent Hooks
Coordinate with specialized agents:
```json
{
  "SubAgentStop": [{
    "hooks": [{
      "type": "command",
      "command": ".hooks/process-agent-output.sh"
    }]
  }]
}
```

## Implementation Patterns

### 1. Workflow Enforcement
Ensure required steps are completed:
```bash
#!/bin/bash
# .hooks/session-stop-check.sh
if [ -f ".claude/context/active-session.md" ]; then
  echo "Please document your session before exiting" >&2
  exit 2  # Block until done
fi
```

### 2. Safety Mechanisms
Prevent dangerous operations:
```bash
#!/bin/bash
# .hooks/safety-check.sh
TOOL_INPUT=$(cat)
if echo "$TOOL_INPUT" | grep -q "rm -rf"; then
  echo "Dangerous command blocked" >&2
  exit 1
fi
echo "$TOOL_INPUT"
```

## Architecture Benefits

### 1. Separation of Concerns
- Hooks handle capture
- Processing happens separately
- AI focuses on core task

### 2. Non-Intrusive
- No changes to AI workflow
- Transparent operation
- Easy to disable

### 3. Composable
- Multiple hooks per event
- Chain operations
- Modular design

### 4. Testable
- Hook scripts are isolated
- Easy to unit test
- Mock events for testing

## Advanced Patterns

### 1. Conditional Execution
```json
{
  "PostToolUse": [{
    "matcher": "Read|Write",
    "condition": "test -f .track-files",
    "hooks": [{
      "type": "command",
      "command": ".hooks/file-tracker.sh"
    }]
  }]
}
```

### 2. Rate Limiting
```json
{
  "Stop": [{
    "throttle": 300000,  // 5 minutes
    "hooks": [{
      "type": "command",
      "command": ".hooks/periodic-task.sh"
    }]
  }]
}
```

### 3. Error Recovery
```bash
#!/bin/bash
# .hooks/safe-executor.sh
set -e
trap 'echo "Hook failed: $?" >> .logs/hook-errors.log' ERR

# Main hook logic here
```

### 4. Data Pipelines
```bash
#!/bin/bash
# .hooks/pipeline.sh
cat | tee .logs/raw.log | \
  .processors/filter.sh | \
  .processors/transform.sh | \
  .processors/store.sh
```

## Integration Strategies

### With Sub-Agents
1. Hooks capture raw events
2. Sub-agents provide enrichment
3. Processing merges both sources

### With Container Workflows
1. Hooks write to mounted volumes
2. Containers process asynchronously
3. Results available across environments

### With Version Control
1. Hooks track file changes
2. Auto-generate commit messages
3. Mark significant events

## Best Practices

1. **Keep Hooks Fast**: <100ms execution time
2. **Handle Failures Gracefully**: Don't break AI flow
3. **Use Structured Logging**: Consistent formats
4. **Avoid Side Effects**: Hooks should observe, not modify
5. **Document Hook Behavior**: Clear descriptions

## Testing Hooks

### Unit Testing
```bash
# Test individual hook
echo '{"session_id":"test"}' | .hooks/session-stop-check.sh
echo $?  # Check exit code
```

### Integration Testing
```bash
# Simulate hook events
export CLAUDE_SESSION_ID="test-123"
echo '{"tool_name":"Read","tool_input":{"file_path":"test.rs"}}' | \
  .hooks/track-tools.sh
```

### End-to-End Testing
Use test sessions with known inputs to verify complete flow.

## Evolution Path

1. **Start Simple**: Basic logging hooks
2. **Add Intelligence**: Pattern detection
3. **Enable Automation**: Trigger workflows
4. **Scale Up**: Distributed processing
5. **Share Patterns**: Community hooks

## Related Patterns
- Sub-Agent Architecture
- Session Persistence
- Event-Driven Processing