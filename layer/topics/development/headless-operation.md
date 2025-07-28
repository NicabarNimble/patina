# Headless Operation Pattern

## Overview
Patina commands support both interactive and non-interactive (headless) modes to enable automation, CI/CD integration, and scripting.

## Design Principles

1. **Dual Mode by Default**: Every command that might prompt should support headless operation
2. **Explicit Control**: Use flags and environment variables for clear intent
3. **Machine-Readable Output**: JSON format for parsing by other tools
4. **Meaningful Exit Codes**: Enable proper error handling in scripts

## Implementation

### Command-Line Flags

```bash
# Update command
patina update --yes      # Auto-approve updates
patina update --no       # Check only, don't update
patina update --check    # Check without prompting
patina update --json     # Machine-readable output

# Doctor command
patina doctor --check    # Check only, don't fix
patina doctor --fix      # Auto-fix issues
patina doctor --json     # Machine-readable output
```

### Environment Variables

```bash
# Disable all interactive prompts
export PATINA_NONINTERACTIVE=1

# Auto-approve all prompts (use with caution)
export PATINA_AUTO_APPROVE=1
```

### Exit Codes

#### Update Command
- `0` - Success (updated or already up-to-date)
- `1` - Error occurred
- `2` - Updates available but not applied (--check mode)

#### Doctor Command
- `0` - Environment healthy
- `1` - Error running doctor
- `2` - Issues found but fixable
- `3` - Critical issues (missing required tools)

## JSON Output Format

### Update Command
```json
{
  "patina_version": "0.1.0",
  "components": [
    {
      "name": "claude-adapter",
      "current_version": "0.1.0",
      "available_version": "0.2.0",
      "updated": false
    }
  ],
  "updates_available": true,
  "updates_applied": []
}
```

### Doctor Command
```json
{
  "status": "warning",
  "environment_changes": {
    "missing_tools": [
      {
        "name": "dagger",
        "old_version": "0.9.0",
        "new_version": null,
        "required": true
      }
    ],
    "new_tools": [],
    "version_changes": []
  },
  "project_config": {
    "llm": "claude",
    "adapter_version": "0.2.0",
    "brain_patterns": 15,
    "sessions": 3
  },
  "recommendations": [
    "Install dagger: curl -L https://dl.dagger.io/install.sh | sh"
  ]
}
```

## Use Cases

### CI/CD Pipeline
```yaml
# GitHub Actions example
- name: Check Patina Health
  run: |
    if ! patina doctor --json > health.json; then
      echo "::error::Environment issues detected"
      cat health.json | jq -r '.recommendations[]'
      exit 1
    fi

- name: Update Adapters
  run: patina update --yes
```

### Pre-commit Hook
```bash
#!/bin/bash
# .git/hooks/pre-commit

# Ensure environment is healthy
if ! patina doctor --check >/dev/null 2>&1; then
  echo "⚠️  Environment issues detected. Run 'patina doctor' for details."
  exit 1
fi

# Ensure adapters are up to date
if patina update --check --json | jq -e '.updates_available' >/dev/null; then
  echo "⚠️  Adapter updates available. Run 'patina update' to update."
  exit 1
fi
```

### Automation Script
```bash
#!/bin/bash
# update-all-projects.sh

for project in ~/projects/*/; do
  if [ -f "$project/.patina/config.json" ]; then
    echo "Checking $project..."
    cd "$project"
    
    # Update adapters silently
    patina update --yes --json > /tmp/update.json
    
    # Check health and report issues
    if ! patina doctor --json > /tmp/health.json; then
      echo "Issues in $project:"
      jq -r '.recommendations[]' < /tmp/health.json
    fi
  fi
done
```

## Best Practices

1. **Default to Interactive**: Only use headless mode when explicitly requested
2. **Respect User Intent**: `--no` should never perform actions
3. **Provide Escape Hatches**: Always allow users to check without changing
4. **Clear Output**: JSON for machines, friendly text for humans
5. **Document Exit Codes**: Make scripting predictable

## Future Considerations

- Add `--quiet` flag for minimal output
- Support `--format` for different output formats (yaml, toml)
- Add `--dry-run` for preview of changes
- Consider `--force` for bypassing safety checks (with warnings)