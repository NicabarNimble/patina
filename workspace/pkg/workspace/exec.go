package workspace

import (
	"context"
	"fmt"
	"io"
	"strings"
	"time"
)

// ExecOptions configures command execution
type ExecOptions struct {
	Command     []string          `json:"command"`
	WorkDir     string            `json:"work_dir,omitempty"`
	Environment map[string]string `json:"environment,omitempty"`
	Timeout     time.Duration     `json:"timeout,omitempty"`
	Stdin       io.Reader         `json:"-"` // Not serialized
}

// ExecResult contains the result of command execution
type ExecResult struct {
	ExitCode  int       `json:"exit_code"`
	Stdout    string    `json:"stdout"`
	Stderr    string    `json:"stderr"`
	StartTime time.Time `json:"start_time"`
	EndTime   time.Time `json:"end_time"`
	Duration  string    `json:"duration"`
}

// Execute runs a command in the workspace container
func (m *Manager) Execute(ctx context.Context, workspaceID string, opts *ExecOptions) (*ExecResult, error) {
	// Validate manager state
	m.mu.RLock()
	if m.closed {
		m.mu.RUnlock()
		return nil, ErrManagerClosed
	}
	m.mu.RUnlock()

	// Get workspace
	ws, err := m.GetWorkspace(workspaceID)
	if err != nil {
		return nil, err
	}

	// Check workspace status
	if ws.Status != StatusReady {
		return nil, ErrContainerNotReady
	}

	// Validate options
	if opts == nil || len(opts.Command) == 0 {
		return nil, ErrInvalidConfig
	}

	// Apply timeout if specified
	if opts.Timeout > 0 {
		var cancel context.CancelFunc
		ctx, cancel = context.WithTimeout(ctx, opts.Timeout)
		defer cancel()
	}

	// Log execution
	m.logger.Info("executing command",
		"workspace", workspaceID,
		"command", strings.Join(opts.Command, " "),
		"workdir", opts.WorkDir,
	)

	startTime := time.Now()

	// Get container from ID
	// TODO: This needs proper implementation when we have real Dagger client
	// For now, create a new container - in real implementation we'd reconnect
	if m.dag == nil {
		return nil, ErrNoDaggerClient
	}

	container := m.dag.Container().From(ws.BaseImage)

	// Set working directory if specified
	if opts.WorkDir != "" {
		container = container.WithWorkdir(opts.WorkDir)
	}

	// Set environment variables
	for key, value := range opts.Environment {
		container = container.WithEnvVariable(key, value)
	}

	// Execute command
	execContainer := container.WithExec(opts.Command)

	// Get stdout
	stdout, err := execContainer.Stdout(ctx)
	if err != nil {
		m.logger.Error("failed to get stdout", "error", err)
		return nil, fmt.Errorf("%w: %v", ErrExecFailed, err)
	}

	// Get stderr
	stderr, err := execContainer.Stderr(ctx)
	if err != nil {
		m.logger.Error("failed to get stderr", "error", err)
		return nil, fmt.Errorf("%w: %v", ErrExecFailed, err)
	}

	// Get exit code
	exitCode, err := execContainer.ExitCode(ctx)
	if err != nil {
		m.logger.Error("failed to get exit code", "error", err)
		return nil, fmt.Errorf("%w: %v", ErrExecFailed, err)
	}

	endTime := time.Now()
	duration := endTime.Sub(startTime)

	result := &ExecResult{
		ExitCode:  exitCode,
		Stdout:    stdout,
		Stderr:    stderr,
		StartTime: startTime,
		EndTime:   endTime,
		Duration:  duration.String(),
	}

	m.logger.Info("command executed",
		"workspace", workspaceID,
		"exit_code", exitCode,
		"duration", duration,
	)

	return result, nil
}

// StreamingExecOptions extends ExecOptions with streaming callbacks
type StreamingExecOptions struct {
	ExecOptions
	OnStdout func(data []byte) error
	OnStderr func(data []byte) error
}

// ExecuteStreaming runs a command with real-time output streaming
func (m *Manager) ExecuteStreaming(ctx context.Context, workspaceID string, opts *StreamingExecOptions) (*ExecResult, error) {
	// For now, fall back to regular execute
	// TODO: Implement real streaming when Dagger supports it better
	result, err := m.Execute(ctx, workspaceID, &opts.ExecOptions)
	if err != nil {
		return nil, err
	}

	// Simulate streaming by calling callbacks with full output
	if opts.OnStdout != nil && result.Stdout != "" {
		if err := opts.OnStdout([]byte(result.Stdout)); err != nil {
			return result, err
		}
	}

	if opts.OnStderr != nil && result.Stderr != "" {
		if err := opts.OnStderr([]byte(result.Stderr)); err != nil {
			return result, err
		}
	}

	return result, nil
}
