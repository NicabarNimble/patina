package internal

import (
	"context"
	"fmt"
	"strings"
	"time"
)

// GitOptions configures git operations
type GitOptions struct {
	Message string `json:"message,omitempty"`
	Author  string `json:"author,omitempty"`
	Email   string `json:"email,omitempty"`
}

// GitStatus represents the git status of a workspace
type GitStatus struct {
	Branch        string   `json:"branch"`
	Clean         bool     `json:"clean"`
	Modified      []string `json:"modified,omitempty"`
	Untracked     []string `json:"untracked,omitempty"`
	CurrentCommit string   `json:"current_commit"`
}

// CreateBranch creates a new git branch in the workspace
func (m *Manager) CreateBranch(ctx context.Context, workspaceID, branchName string) error {
	ws, err := m.GetWorkspace(workspaceID)
	if err != nil {
		return err
	}

	if ws.Status != StatusReady {
		return ErrContainerNotReady
	}

	// Create and checkout new branch
	opts := &ExecOptions{
		Command: []string{"git", "checkout", "-b", branchName},
		WorkDir: "/workspace/project",
	}

	result, err := m.Execute(ctx, workspaceID, opts)
	if err != nil {
		return fmt.Errorf("failed to create branch: %w", err)
	}

	if result.ExitCode != 0 {
		return fmt.Errorf("git checkout failed: %s", result.Stderr)
	}

	// Update workspace branch name
	ws.BranchName = branchName
	ws.UpdatedAt = time.Now()

	m.logger.Info("created git branch", "workspace", workspaceID, "branch", branchName)
	return nil
}

// GetGitStatus returns the current git status of the workspace
func (m *Manager) GetGitStatus(ctx context.Context, workspaceID string) (*GitStatus, error) {
	// Check branch
	branchResult, err := m.Execute(ctx, workspaceID, &ExecOptions{
		Command: []string{"git", "branch", "--show-current"},
		WorkDir: "/workspace/project",
	})
	if err != nil {
		return nil, fmt.Errorf("failed to get branch: %w", err)
	}

	branch := strings.TrimSpace(branchResult.Stdout)

	// Check for modifications
	statusResult, err := m.Execute(ctx, workspaceID, &ExecOptions{
		Command: []string{"git", "status", "--porcelain"},
		WorkDir: "/workspace/project",
	})
	if err != nil {
		return nil, fmt.Errorf("failed to get status: %w", err)
	}

	// Parse status output
	var modified, untracked []string
	for _, line := range strings.Split(statusResult.Stdout, "\n") {
		if line == "" {
			continue
		}

		status := line[:2]
		file := strings.TrimSpace(line[2:])

		if strings.Contains(status, "M") {
			modified = append(modified, file)
		} else if status == "??" {
			untracked = append(untracked, file)
		}
	}

	// Get current commit
	commitResult, err := m.Execute(ctx, workspaceID, &ExecOptions{
		Command: []string{"git", "rev-parse", "HEAD"},
		WorkDir: "/workspace/project",
	})
	if err != nil {
		return nil, fmt.Errorf("failed to get commit: %w", err)
	}

	return &GitStatus{
		Branch:        branch,
		Clean:         len(modified) == 0 && len(untracked) == 0,
		Modified:      modified,
		Untracked:     untracked,
		CurrentCommit: strings.TrimSpace(commitResult.Stdout),
	}, nil
}

// CommitChanges commits all changes in the workspace
func (m *Manager) CommitChanges(ctx context.Context, workspaceID string, opts *GitOptions) error {
	if opts == nil {
		opts = &GitOptions{}
	}

	// Set default message
	if opts.Message == "" {
		opts.Message = "Workspace changes"
	}

	// Add all changes
	addResult, err := m.Execute(ctx, workspaceID, &ExecOptions{
		Command: []string{"git", "add", "."},
		WorkDir: "/workspace/project",
	})
	if err != nil {
		return fmt.Errorf("failed to add changes: %w", err)
	}

	if addResult.ExitCode != 0 {
		return fmt.Errorf("git add failed: %s", addResult.Stderr)
	}

	// Commit changes
	commitCmd := []string{"git", "commit", "-m", opts.Message}

	// Add author if specified
	if opts.Author != "" && opts.Email != "" {
		commitCmd = append(commitCmd, "--author", fmt.Sprintf("%s <%s>", opts.Author, opts.Email))
	}

	commitResult, err := m.Execute(ctx, workspaceID, &ExecOptions{
		Command: commitCmd,
		WorkDir: "/workspace/project",
	})
	if err != nil {
		return fmt.Errorf("failed to commit: %w", err)
	}

	if commitResult.ExitCode != 0 {
		// Check if there's nothing to commit
		if strings.Contains(commitResult.Stdout, "nothing to commit") {
			m.logger.Info("no changes to commit", "workspace", workspaceID)
			return nil
		}
		return fmt.Errorf("git commit failed: %s", commitResult.Stderr)
	}

	m.logger.Info("committed changes", "workspace", workspaceID, "message", opts.Message)
	return nil
}

// PushBranch pushes the current branch to origin
func (m *Manager) PushBranch(ctx context.Context, workspaceID string) error {
	ws, err := m.GetWorkspace(workspaceID)
	if err != nil {
		return err
	}

	pushResult, err := m.Execute(ctx, workspaceID, &ExecOptions{
		Command: []string{"git", "push", "-u", "origin", ws.BranchName},
		WorkDir: "/workspace/project",
	})
	if err != nil {
		return fmt.Errorf("failed to push: %w", err)
	}

	if pushResult.ExitCode != 0 {
		return fmt.Errorf("git push failed: %s", pushResult.Stderr)
	}

	m.logger.Info("pushed branch", "workspace", workspaceID, "branch", ws.BranchName)
	return nil
}
