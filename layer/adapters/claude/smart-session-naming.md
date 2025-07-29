# Smart Session Naming Pattern

**⚠️ NOT IMPLEMENTED**: This smart naming approach was designed but not implemented. The current session system passes user input directly to the session file without pre-processing. Only the bash script sanitization layer exists. See `layer/topics/development/session-implementation.md` for the current implementation.

## Overview
A two-layer approach that combines Claude's intelligence with robust bash script sanitization for session file naming.

## Architecture

### Layer 1: Claude Pre-processing (Smart Layer)
When user runs `/session-start <name>`, Claude:

1. **Analyzes the input** for complexity indicators:
   - Special characters beyond basic punctuation
   - Length exceeding 50 characters
   - Complex descriptions with multiple concepts

2. **Decides on action**:
   - **Simple names** (e.g., "implement auth") → Pass through unchanged
   - **Complex names** → Rewrite to concise, filesystem-safe version

3. **Rewriting rules** for complex names:
   - Extract key concepts (bug numbers, feature names, components)
   - Generate 3-7 word summary
   - Preserve important identifiers (e.g., bug-1234, jwt-auth)
   - Use only alphanumeric, dots, underscores, and hyphens
   - Result should be under 50 characters

### Layer 2: Bash Script Sanitization (Safety Layer)
The `session-start.sh` script always:

1. **Sanitizes any input**:
   ```bash
   SAFE_NAME=$(echo "$1" | tr -cs '[:alnum:]._-' '-' | sed 's/^-\+//;s/-\+$//' | cut -c1-50)
   ```

2. **Creates predictable filenames**:
   - Format: `YYYYMMDD-HHMM-<safe-name>.md`
   - Always filesystem-safe
   - Always under reasonable length

## Examples

### Simple Input (Claude passes through)
```
Input:  "implement jwt auth"
Claude: "implement jwt auth" (unchanged)
Script: "20250723-1630-implement-jwt-auth.md"
```

### Complex Input (Claude rewrites)
```
Input:  "fix bug #1234 where users/admins can't log in on Tuesdays after 3pm"
Claude: "fix-tuesday-3pm-login-bug-1234"
Script: "20250723-1631-fix-tuesday-3pm-login-bug-1234.md"
```

### Special Characters (Claude rewrites)
```
Input:  "implement feature: user@domain.com validation & sanitization"
Claude: "implement-email-validation-sanitization"
Script: "20250723-1632-implement-email-validation-sanitization.md"
```

## Benefits

1. **Fast for common cases**: Simple names start sessions instantly
2. **Smart for complex cases**: Long descriptions get meaningful compression
3. **Always safe**: Script sanitization ensures filesystem compatibility
4. **Predictable**: Users learn what makes a "good" session name
5. **Graceful degradation**: If Claude fails, script still works

## Implementation Status

- ✅ Script sanitization implemented in `session-start.sh`
- ⏳ Claude pre-processing logic (future implementation)
- ⏳ Pattern detection heuristics (future implementation)

## Design Rationale

This pattern maintains:
- **Separation of concerns**: Intelligence in Claude, safety in bash
- **Unix philosophy**: Script does one thing well (safe filenames)
- **Progressive enhancement**: Works without Claude, better with Claude
- **User agency**: Original input preserved in session file content