# Manual Hook Testing Protocol

## Prerequisites
1. Ensure you're on `feature/hook-based-sessions` branch
2. Copy `.claude/settings.hooks.json` to `.claude/settings.json`
3. Start a fresh Claude Code session

## Test Steps

### 1. Start Fresh Session
```bash
# Exit current Claude Code
exit

# Ensure hooks are configured
cp .claude/settings.hooks.json .claude/settings.json

# Start new session
claude-code
```

### 2. Test Basic Hook Capture
Run these commands in order:
```bash
# Test 1: Read a file
cat README.md

# Test 2: Edit a file
echo "test" > test-hooks.txt

# Test 3: Run a command
ls -la
```

### 3. Verify Hook Logs
```bash
# Check for log files
ls -la .claude/logs/

# View captured events
cat .claude/logs/hooks-*.log
```

### 4. Expected Output
You should see entries like:
```
2025-07-28T13:33:53.3NZ|PROMPT|cat README.md
2025-07-28T13:33:54.3NZ|TOOL|Bash|
2025-07-28T13:33:55.3NZ|TOOL|Write|test-hooks.txt
```

## Automated Verification Script
Run after manual testing:
```bash
./test/verify-hooks.sh
```