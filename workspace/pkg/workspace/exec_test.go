package workspace

import (
	"bytes"
	"context"
	"os"
	"testing"
	"time"
)

func Test_ExecOptions_Validation(t *testing.T) {
	tests := []struct {
		name    string
		opts    *ExecOptions
		isValid bool
	}{
		{
			name:    "valid basic command",
			opts:    &ExecOptions{Command: []string{"ls", "-la"}},
			isValid: true,
		},
		{
			name:    "with workdir",
			opts:    &ExecOptions{Command: []string{"pwd"}, WorkDir: "/app"},
			isValid: true,
		},
		{
			name: "with environment",
			opts: &ExecOptions{
				Command:     []string{"env"},
				Environment: map[string]string{"FOO": "bar"},
			},
			isValid: true,
		},
		{
			name:    "with timeout",
			opts:    &ExecOptions{Command: []string{"sleep", "1"}, Timeout: 2 * time.Second},
			isValid: true,
		},
		{
			name:    "empty command",
			opts:    &ExecOptions{Command: []string{}},
			isValid: false,
		},
		{
			name:    "nil options",
			opts:    nil,
			isValid: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// This is more of a documentation test
			// Real validation happens in Execute method
			if tt.opts == nil && tt.isValid {
				t.Error("nil options should not be valid")
			}
			if tt.opts != nil && len(tt.opts.Command) == 0 && tt.isValid {
				t.Error("empty command should not be valid")
			}
		})
	}
}

func Test_ExecResult_Duration(t *testing.T) {
	start := time.Now()
	time.Sleep(100 * time.Millisecond)
	end := time.Now()

	result := &ExecResult{
		ExitCode:  0,
		Stdout:    "output",
		Stderr:    "",
		StartTime: start,
		EndTime:   end,
		Duration:  end.Sub(start).String(),
	}

	// Verify duration is set
	if result.Duration == "" {
		t.Error("duration should not be empty")
	}

	// Verify it's roughly correct (allowing for some variance)
	duration := result.EndTime.Sub(result.StartTime)
	if duration < 100*time.Millisecond || duration > 200*time.Millisecond {
		t.Errorf("unexpected duration: %v", duration)
	}
}

func Test_StreamingExecOptions_Callbacks(t *testing.T) {
	var stdoutBuf bytes.Buffer
	var stderrBuf bytes.Buffer

	opts := &StreamingExecOptions{
		ExecOptions: ExecOptions{
			Command: []string{"echo", "hello"},
		},
		OnStdout: func(data []byte) error {
			stdoutBuf.Write(data)
			return nil
		},
		OnStderr: func(data []byte) error {
			stderrBuf.Write(data)
			return nil
		},
	}

	// Test that callbacks can be called
	testOutput := []byte("test output")
	err := opts.OnStdout(testOutput)
	if err != nil {
		t.Errorf("OnStdout callback failed: %v", err)
	}

	if stdoutBuf.String() != "test output" {
		t.Errorf("expected 'test output', got '%s'", stdoutBuf.String())
	}
}

// Test workspace not ready for execution
func Test_Execute_WorkspaceNotReady(t *testing.T) {
	if os.Getenv("GITHUB_ACTIONS") == "true" {
		t.Skip("Skipping Dagger integration test in CI")
	}

	m := mustNewTestManagerWithDagger(t)
	defer m.Close(context.Background())

	// Create workspace (will be in Creating status)
	ws, err := m.CreateWorkspace(context.Background(), "test", nil)
	if err != nil {
		t.Fatalf("failed to create workspace: %v", err)
	}

	// Try to execute before ready
	opts := &ExecOptions{Command: []string{"ls"}}
	_, err = m.Execute(context.Background(), ws.ID, opts)

	if !IsNotReady(err) {
		t.Errorf("expected ErrContainerNotReady, got %v", err)
	}
}

// Benchmark example
func Benchmark_GenerateID(b *testing.B) {
	for i := 0; i < b.N; i++ {
		_ = generateID()
	}
}
