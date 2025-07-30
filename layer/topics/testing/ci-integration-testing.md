---
title: CI Integration Testing Strategy
description: How to handle integration tests that require external dependencies in CI
created: 2025-07-30
type: pattern
---

# CI Integration Testing Strategy

## Context

When building features that depend on external tools (like Dagger, Docker, or databases), we face a conflict between our testing philosophy and CI constraints:

1. **No-mock philosophy**: We prefer testing against real systems to catch real bugs
2. **CI limitations**: GitHub Actions doesn't have all tools pre-installed
3. **Speed considerations**: Installing tools in CI can be slow and flaky

## Pattern: Skip Integration Tests in CI

### Implementation

Add environment detection to skip tests that require unavailable dependencies:

```go
func Test_Execute_WorkspaceNotReady(t *testing.T) {
    if os.Getenv("GITHUB_ACTIONS") == "true" {
        t.Skip("Skipping Dagger integration test in CI")
    }
    
    // Rest of integration test using real Dagger
}
```

### Benefits

1. **Maintains test purity**: No mocks, real tests when it matters
2. **Fast CI**: Unit tests still run and catch issues
3. **Developer experience**: Full test coverage locally where tools are available
4. **Clear signal**: Skipped tests show explicitly what's not being tested

### When to Use

- Tests that require specific tools not available in CI (Dagger, local databases)
- Tests that would be too slow or flaky in CI (large container pulls)
- Tests that require special permissions or access

### Alternatives Considered

1. **Docker-in-Docker**: Complex setup, can be flaky
2. **Mocks**: Goes against our testing philosophy
3. **Separate test commands**: Using `go test -short` - less explicit

## Example: Dagger Integration

The Patina workspace service uses Dagger for container management. In CI:

- Unit tests run normally (testing business logic)
- Integration tests that create real containers are skipped
- Developers with Dagger installed get full test coverage locally

## Future Considerations

- Consider adding a CI job that runs integration tests in a full environment (nightly/weekly)
- Document which tests are skipped in CI in the README
- Add test coverage reports that show integration vs unit test coverage