---
id: git-hooks-integration
status: active
created: 2025-08-16
session: learn-new-git-commands
tags: [git, hooks, integration, auto-commit, survival-tracking]
references: [git-integration-timeline.md, git-knowledge-evolution.md, session-git-integration-ideas.md]
---

# Git Hooks Integration: Auto-Committing Everything to Git

A detailed plan for using Claude Code hooks to automatically commit all changes (code, markdown, session files) to Git, providing the memory layer that Patina needs.

## The Vision

Instead of separate tracking systems, use Git as the single source of truth for everything:
- Session markdown files → auto-committed to Git
- Pattern documentation → auto-committed to Git
- Code changes → auto-committed to Git
- Co-modification tracking → derived from Git history
- Survival metrics → calculated from Git age

## Current Patina Architecture

### What We Have

1. **Session Commands** (`.claude/bin/session-*.sh`)
   - `session-start.sh`: Creates `active-session.md`
   - `session-update.sh`: Appends updates with timestamps
   - `session-end.sh`: Archives to `layer/sessions/`
   - Work perfectly but don't use Git

2. **Git Commands** (`.claude/bin/git-*.sh`)
   - `git-start.sh`: Shows Git context and survival insights
   - `git-update.sh`: Tracks commits and modifications
   - `git-end.sh`: Classifies work and preserves experiments
   - Provide information but separate from sessions

3. **Rust Git Integration** (`src/indexer/internal/`)
   - `git_state.rs`: State machine (Untracked → Modified → Staged → Committed → Pushed → Merged)
   - `git_detection.rs`: Detects file states via shell commands
   - `git_confidence.rs`: Maps Git states to confidence levels
   - 80% complete but disconnected from actual Git events

4. **Navigation System** (`src/commands/navigate.rs`)
   - Shows patterns with confidence scores
   - Problem: Confidence always "High" after any commit
   - Should use survival time, not commit status

### The Disconnect

Three parallel systems that don't share intelligence:
- Session files track the "why" but aren't in Git
- Git commands provide context but don't update sessions
- Navigate shows confidence but doesn't use actual Git age

## The Hooks Solution

Claude Code hooks can bridge these systems by automatically committing everything to Git and tracking metadata.

### Hook Events We'll Use

1. **Stop**: When Claude finishes responding
   - Auto-commit session updates
   - Preserve work context

2. **PostToolUse**: After file modifications
   - Track which files changed
   - Build co-modification data

3. **PreToolUse**: Before file modifications
   - Check survival metrics
   - Warn about old code

4. **SessionStart**: When starting/resuming
   - Initialize Git tracking
   - Show previous work

## Implementation Plan

### Phase 1: Basic Auto-Commit

#### Hook 1: Auto-Commit Sessions on Stop

```ruby
#!/usr/bin/env ruby
# ~/.claude/hooks/patina_session_commit.rb

require 'json'
require 'time'

# Parse hook input
input = JSON.parse($stdin.read)
cwd = input['cwd']
session_id = input['session_id']
transcript_path = input['transcript_path']

Dir.chdir(cwd) if cwd

# Find last user prompt from transcript
last_prompt = File.foreach(transcript_path)
  .map { |line| JSON.parse(line) }
  .reverse
  .find { |e| e.dig('message', 'role') == 'user' }
  &.dig('message', 'content')
  &.split("\n")&.first&.slice(0, 72) || 'checkpoint'

# Check if we have active session
if File.exist?('.claude/context/active-session.md')
  # Add timestamp to session file
  File.open('.claude/context/active-session.md', 'a') do |f|
    f.puts "\n_Auto-checkpoint at #{Time.now.strftime('%H:%M')}_"
  end
  
  # Commit session files
  system("git", "add", ".claude/context/")
  system("git", "commit", "-m", "session: #{last_prompt}")
end

# Also commit any pattern changes
if Dir.exist?('layer/')
  changes = `git status --porcelain layer/`.strip
  unless changes.empty?
    system("git", "add", "layer/")
    system("git", "commit", "-m", "patterns: #{last_prompt}")
  end
end
```

#### Hook 2: Track Co-Modifications

```ruby
#!/usr/bin/env ruby
# ~/.claude/hooks/patina_track_modifications.rb

require 'json'
require 'fileutils'

input = JSON.parse($stdin.read)
session_id = input['session_id']
file_path = input.dig('tool_input', 'file_path')
cwd = input['cwd']

return unless file_path

Dir.chdir(cwd) if cwd

# Track co-modifications
comod_file = '.claude/context/git-work/comodifications.jsonl'
FileUtils.mkdir_p(File.dirname(comod_file))

entry = {
  session_id: session_id,
  file: file_path,
  timestamp: Time.now.iso8601,
  git_branch: `git branch --show-current`.strip
}

File.open(comod_file, 'a') do |f|
  f.puts entry.to_json
end

# Update session file if exists
if File.exist?('.claude/context/active-session.md')
  # Check file survival
  age = `git log -1 --format="%ar" -- "#{file_path}" 2>/dev/null`.strip
  if age.include?('month') || age.include?('year')
    File.open('.claude/context/active-session.md', 'a') do |f|
      f.puts "- Modified `#{File.basename(file_path)}` (survived #{age})"
    end
  end
end
```

#### Hook 3: Survival Warnings

```ruby
#!/usr/bin/env ruby
# ~/.claude/hooks/patina_survival_check.rb

require 'json'

input = JSON.parse($stdin.read)
tool = input['tool']
file_path = input.dig('tool_input', 'file_path')

# Only check for Edit/Write operations
return unless %w[Edit MultiEdit Write].include?(tool)
return unless file_path && File.exist?(file_path)

# Get file age
age = `git log -1 --format="%ar" -- "#{file_path}" 2>/dev/null`.strip
commits = `git log --oneline -- "#{file_path}" 2>/dev/null | wc -l`.to_i

# Warn about old, stable files
if age.include?('month') || age.include?('year')
  puts "⚠️  PATINA WARNING: #{file_path}"
  puts "   Survived: #{age}"
  puts "   Commits: #{commits}"
  puts "   This is a stable pattern - modify carefully!"
  
  # Check co-modification patterns
  related = `git log --name-only --pretty=format: -- "#{file_path}" | sort | uniq -c | sort -rn | head -5`
  puts "   Often changes with:"
  puts related.split("\n").map { |l| "     #{l.strip}" }
end

# Always allow the operation (exit 0)
exit 0
```

### Phase 2: Enhanced Git Memory

#### Hook 4: Session-Aware Branches

```ruby
#!/usr/bin/env ruby
# ~/.claude/hooks/patina_session_branch.rb

require 'json'

input = JSON.parse($stdin.read)
session_id = input['session_id']
cwd = input['cwd']

Dir.chdir(cwd) if cwd

# Create session branch if needed
branch = "session/#{session_id[0..7]}"
current = `git branch --show-current`.strip

# Only create branch if we're on main/master
if %w[main master].include?(current)
  unless system("git", "show-ref", "--verify", "--quiet", "refs/heads/#{branch}")
    system("git", "checkout", "-b", branch)
    puts "Created session branch: #{branch}"
  end
end
```

#### Hook 5: Pattern Survival Tracking

```ruby
#!/usr/bin/env ruby
# ~/.claude/hooks/patina_pattern_survival.rb

require 'json'
require 'date'

input = JSON.parse($stdin.read)
cwd = input['cwd']

Dir.chdir(cwd) if cwd

# Update pattern survival metrics
if Dir.exist?('layer/')
  survival_file = 'layer/surface/pattern-survival-metrics.md'
  
  patterns = {}
  
  # Check each pattern file
  Dir.glob('layer/**/*.md').each do |file|
    # Get creation date
    created = `git log --reverse --format="%ai" -- "#{file}" | head -1`.strip
    next if created.empty?
    
    created_date = Date.parse(created)
    age_days = (Date.today - created_date).to_i
    
    # Get modification count
    mods = `git log --oneline -- "#{file}" | wc -l`.to_i
    
    # Calculate survival score
    score = (age_days > 90 ? 'High' : age_days > 30 ? 'Medium' : 'Low')
    
    patterns[file] = {
      age_days: age_days,
      modifications: mods,
      score: score
    }
  end
  
  # Write metrics file
  File.open(survival_file, 'w') do |f|
    f.puts "# Pattern Survival Metrics"
    f.puts "Generated: #{Time.now}"
    f.puts
    f.puts "| Pattern | Age | Mods | Score |"
    f.puts "|---------|-----|------|-------|"
    patterns.sort_by { |_, v| -v[:age_days] }.each do |file, data|
      name = File.basename(file, '.md')
      f.puts "| #{name} | #{data[:age_days]}d | #{data[:modifications]} | #{data[:score]} |"
    end
  end
  
  system("git", "add", survival_file)
  system("git", "commit", "-m", "metrics: update pattern survival")
end
```

### Phase 3: Connect to Navigation

#### Update Confidence Scoring

Instead of the broken confidence scoring in `navigate`, we can use Git hooks to maintain a confidence index:

```ruby
#!/usr/bin/env ruby
# ~/.claude/hooks/patina_update_confidence.rb

require 'json'
require 'sqlite3'

# This would update the SQLite database that navigate uses
db = SQLite3::Database.new('.patina/navigation.db')

# Update confidence based on actual survival
db.execute("UPDATE patterns SET confidence = ? WHERE file = ?", 
  calculate_confidence(file_age, modifications), 
  file_path)
```

## Configuration

### Settings.json

```json
{
  "hooks": {
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/patina_session_commit.rb"
      }]
    }],
    "PostToolUse": [{
      "matcher": "Edit|MultiEdit|Write",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/patina_track_modifications.rb"
      }]
    }],
    "PreToolUse": [{
      "matcher": "Edit|MultiEdit|Write",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/patina_survival_check.rb"
      }]
    }],
    "SessionStart": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/patina_session_branch.rb"
      }]
    }]
  }
}
```

### Installation via Patina

```bash
# Add new command to install hooks
patina init . --hooks

# This would:
# 1. Create ~/.claude/hooks/ directory
# 2. Install the Ruby scripts
# 3. Update ~/.claude/settings.json
# 4. Set up Git aliases for manual use
```

## Benefits Over Current System

### Before (Three Separate Systems)
- Session commands write to `.md` files (not in Git)
- Git commands show context (but don't update anything)
- Navigate shows confidence (but it's always "High")

### After (Unified Git Memory)
- Everything auto-committed to Git
- Co-modifications tracked automatically
- Survival metrics calculated from actual Git history
- Navigation confidence based on real survival time
- Session files include Git context automatically

## Git Query Examples

With everything in Git, we can answer questions like:

```bash
# What patterns survived longest?
git log --reverse --format="%ar %s" -- layer/core/

# What files change together?
git log --name-only --format="" | sort | uniq -c | sort -rn

# Session history
git log --grep="^session:" --oneline

# Pattern evolution
git log --follow layer/surface/specific-pattern.md

# What did I work on yesterday?
git log --since="yesterday" --grep="^session:" --format="%s"
```

## Integration with Existing Patina Code

### Fix Navigation Confidence

In `src/indexer/internal/git_detection.rs`, the confidence calculation would be fixed:

```rust
// Instead of:
if committed => Confidence::High

// Use:
pub fn calculate_confidence(file: &Path) -> Confidence {
    let age_days = get_file_age_days(file);
    let modifications = count_modifications(file);
    
    match (age_days, modifications) {
        (90.., _) => Confidence::Verified,
        (30..90, m) if m < 5 => Confidence::High,
        (7..30, _) => Confidence::Medium,
        _ => Confidence::Low,
    }
}
```

### Connect Git State Machine

The hooks would trigger state transitions in `git_state.rs`:

```rust
// Hook triggers this via IPC or file watch
impl GitNavigationStateMachine {
    pub fn file_modified(&mut self, file: PathBuf, session_id: String) {
        self.transition(GitEvent::FileModified { 
            path: file,
            workspace_id: session_id,
        });
    }
}
```

## Rollout Strategy

1. **Phase 1**: Basic auto-commit hooks (immediate value)
2. **Phase 2**: Co-modification and survival tracking
3. **Phase 3**: Fix navigation confidence scoring
4. **Phase 4**: Connect Git state machine
5. **Phase 5**: Full integration with pattern validation

## Conclusion

Using Claude Code hooks to auto-commit everything to Git solves the core problem: making Git the single source of truth for all changes. This approach:

- Requires minimal changes to existing code
- Provides immediate value (rollback points)
- Builds the data needed for survival metrics
- Creates the foundation for proper confidence scoring
- Maintains the simplicity of the current session system

The hooks act as the glue between Patina's sophisticated Git infrastructure and the actual Git repository, finally connecting the 80% complete system to real Git events.