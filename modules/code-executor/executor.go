package executor

import (
	"context"
	"fmt"
	"io"
	"time"

	"dagger.io/dagger"
)

// Options configures command execution
type Options struct {
	Command     []string
	WorkDir     string
	Environment map[string]string
	Timeout     time.Duration
	Stdin       io.Reader
}

// Result contains the result of command execution
type Result struct {
	ExitCode  int
	Stdout    string
	Stderr    string
	StartTime time.Time
	EndTime   time.Time
	Duration  time.Duration
}

// Executor runs commands in containers
type Executor struct {
	client *dagger.Client
}

// New creates a new executor
func New(client *dagger.Client) *Executor {
	return &Executor{
		client: client,
	}
}

// Execute runs a command in a container
func (e *Executor) Execute(ctx context.Context, container *dagger.Container, opts *Options) (*Result, error) {
	if opts == nil || len(opts.Command) == 0 {
		return nil, fmt.Errorf("command is required")
	}

	// Apply timeout
	if opts.Timeout > 0 {
		var cancel context.CancelFunc
		ctx, cancel = context.WithTimeout(ctx, opts.Timeout)
		defer cancel()
	}

	startTime := time.Now()

	// Configure container
	if opts.WorkDir != "" {
		container = container.WithWorkdir(opts.WorkDir)
	}

	for key, value := range opts.Environment {
		container = container.WithEnvVariable(key, value)
	}

	// Execute command
	execContainer := container.WithExec(opts.Command)

	// Get outputs
	stdout, err := execContainer.Stdout(ctx)
	if err != nil {
		// Even on error, try to get stderr for debugging
		stderr, _ := execContainer.Stderr(ctx)
		endTime := time.Now()
		return &Result{
			ExitCode:  -1,
			Stdout:    stdout,
			Stderr:    stderr,
			StartTime: startTime,
			EndTime:   endTime,
			Duration:  endTime.Sub(startTime),
		}, fmt.Errorf("execution failed: %w", err)
	}

	stderr, _ := execContainer.Stderr(ctx)
	
	// Get exit code (Dagger doesn't expose this directly, infer from error)
	exitCode := 0
	if err != nil {
		exitCode = 1
	}

	endTime := time.Now()

	return &Result{
		ExitCode:  exitCode,
		Stdout:    stdout,
		Stderr:    stderr,
		StartTime: startTime,
		EndTime:   endTime,
		Duration:  endTime.Sub(startTime),
	}, nil
}

// ExecuteSimple runs a simple command without configuration
func (e *Executor) ExecuteSimple(ctx context.Context, container *dagger.Container, command ...string) (*Result, error) {
	return e.Execute(ctx, container, &Options{
		Command: command,
	})
}