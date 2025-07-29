# Go + Dagger Constraints for LLMs

## Purpose
These constraints guide LLMs to write simple, maintainable Dagger pipelines that serve as "execution scripts" rather than complex programs.

## Core Rules

1. **Pipeline Scripts, Not Programs**
   - Max 200 lines per file
   - Max 50 lines per function
   - No abstract interfaces
   - No design patterns

2. **Direct Dagger Usage**
   - Use Dagger SDK directly
   - No wrapper abstractions
   - Clear container operations
   - Explicit error handling

3. **No Complex Go Features**
   - No interfaces (avoid mocks)
   - No channels (unless required)
   - No reflection
   - Minimal goroutines

4. **Data Structures**
   - Simple structs for config
   - No nested maps
   - No interface{} types
   - Clear field names

5. **Error Handling**
   - Always return errors
   - No panic() calls
   - Log then return
   - Clear error messages

6. **Escape Hatches**
   - Comment complex logic
   - Note when to move to Rust
   - Keep business logic minimal
   - Pipelines execute, not decide

## Examples

GOOD:
```go
func build(ctx context.Context, c *dagger.Client) error {
    _, err := c.Container().
        From("rust:latest").
        WithExec([]string{"cargo", "build"}).
        Sync(ctx)
    return err
}
```

BAD:
```go
type BuildStrategy interface {
    Execute(BuildContext) BuildResult
}
```

## When to Break Rules
- Parallel container orchestration may use goroutines
- Dagger-specific patterns may require channels
- Always document why rules are broken