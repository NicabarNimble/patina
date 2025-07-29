# Sub-Agent Architecture Pattern

## Overview
A modular architecture pattern for creating specialized AI assistants that handle specific tasks within a larger system.

## Core Concept
Sub-agents are focused AI assistants that:
- Handle specific domains or tasks
- Have tailored system prompts
- Use restricted tool sets
- Maintain separate context from main conversation
- Can be invoked automatically or explicitly

## Implementation Pattern

### 1. Sub-Agent Definition
```json
{
  "name": "pattern-manager",
  "description": "Analyzes sessions and extracts reusable patterns",
  "system_prompt": "You analyze development sessions and extract patterns...",
  "tools": ["Read", "Write", "Grep"],
  "config": {
    "trigger": "manual|automatic",
    "input_pattern": "*.md",
    "output_pattern": "layer/{category}/{name}.md"
  }
}
```

### 2. Storage Structure
```
.claude/agents/          # Project-specific agents
├── pattern-manager.json
├── test-runner.json
└── enricher.json

~/.claude/agents/        # User-level agents
├── code-reviewer.json
└── docs-generator.json
```

### 3. Invocation Patterns

#### Manual Invocation
```bash
# Via slash command
/agents pattern-manager analyze-session

# Via custom command
/extract-patterns
```

#### Automatic Invocation
- Hook-based triggers
- Pattern matching on events
- Scheduled intervals

### 4. Context Isolation
- Each agent maintains separate conversation context
- Can access shared resources (files, memory)
- Results returned to main conversation
- No cross-contamination of context

## Benefits

1. **Specialization**: Each agent optimized for its task
2. **Modularity**: Add/remove agents without affecting system
3. **Scalability**: Distribute work across multiple focused agents
4. **Maintainability**: Clear boundaries and responsibilities
5. **Reusability**: Agents can be shared across projects

## Use Cases

### Development Workflows
- **Test Runner**: Executes tests and analyzes failures
- **Code Reviewer**: Reviews changes against standards
- **Pattern Extractor**: Identifies reusable patterns
- **Documentation Generator**: Creates docs from code

### Session Management
- **Session Enricher**: Adds context to captured sessions
- **Decision Logger**: Extracts architectural decisions
- **Problem Analyzer**: Identifies recurring issues

### Knowledge Management
- **Pattern Classifier**: Categorizes patterns (core/topic/project)
- **Knowledge Migrator**: Promotes patterns up hierarchy
- **Insight Generator**: Discovers cross-project patterns

## Integration with Hooks

Hooks can trigger sub-agents at lifecycle points:
```json
{
  "hooks": {
    "Stop": [{
      "hooks": [{
        "type": "command",
        "command": "claude -p 'Use pattern-manager agent to analyze session'"
      }]
    }]
  }
}
```

## Best Practices

1. **Single Responsibility**: Each agent should do one thing well
2. **Clear Naming**: Agent names should indicate their purpose
3. **Tool Restrictions**: Only grant necessary tools
4. **Documentation**: Include clear descriptions and examples
5. **Error Handling**: Graceful degradation if agent fails

## Evolution Path

As sub-agents mature:
1. Start as project-specific agents
2. Prove value through usage
3. Promote to user-level for reuse
4. Eventually become core patterns
5. Share with community

## Related Patterns
- Hook-Based Automation
- Session Capture Systems
- Knowledge Hierarchies