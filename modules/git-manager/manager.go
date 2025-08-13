package gitmanager

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// Manager handles git operations for workspaces
type Manager struct {
	repoPath     string
	worktreeRoot string
}

// NewManager creates a new git manager
func NewManager(repoPath, worktreeRoot string) (*Manager, error) {
	// Verify repo path is a git repository
	cmd := exec.Command("git", "-C", repoPath, "rev-parse", "--git-dir")
	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("not a git repository: %s", repoPath)
	}

	// Ensure worktree root exists
	if err := os.MkdirAll(worktreeRoot, 0755); err != nil {
		return nil, fmt.Errorf("failed to create worktree root: %w", err)
	}

	return &Manager{
		repoPath:     repoPath,
		worktreeRoot: worktreeRoot,
	}, nil
}

// CreateWorktree creates a new git worktree
func (m *Manager) CreateWorktree(ctx context.Context, id, branch string) (string, error) {
	worktreePath := filepath.Join(m.worktreeRoot, id)

	// Check if branch exists
	checkCmd := exec.CommandContext(ctx, "git", "-C", m.repoPath, 
		"show-ref", "--verify", "--quiet", "refs/heads/"+branch)
	branchExists := checkCmd.Run() == nil

	var cmd *exec.Cmd
	if branchExists {
		cmd = exec.CommandContext(ctx, "git", "-C", m.repoPath, 
			"worktree", "add", worktreePath, branch)
	} else {
		cmd = exec.CommandContext(ctx, "git", "-C", m.repoPath, 
			"worktree", "add", "-b", branch, worktreePath)
	}

	output, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("failed to create worktree: %w\nOutput: %s", err, output)
	}

	return worktreePath, nil
}

// RemoveWorktree removes a git worktree
func (m *Manager) RemoveWorktree(ctx context.Context, id string) error {
	worktreePath := filepath.Join(m.worktreeRoot, id)

	// Remove worktree
	cmd := exec.CommandContext(ctx, "git", "-C", m.repoPath, 
		"worktree", "remove", "--force", worktreePath)
	
	if output, err := cmd.CombinedOutput(); err != nil {
		// If already removed, don't error
		if strings.Contains(string(output), "is not a working tree") {
			return nil
		}
		return fmt.Errorf("failed to remove worktree: %w\nOutput: %s", err, output)
	}

	// Prune worktree list
	pruneCmd := exec.CommandContext(ctx, "git", "-C", m.repoPath, "worktree", "prune")
	pruneCmd.Run() // Best effort

	return nil
}

// GetStatus returns git status for a worktree
func (m *Manager) GetStatus(ctx context.Context, worktreePath string) (*Status, error) {
	// Get current branch
	branchCmd := exec.CommandContext(ctx, "git", "-C", worktreePath, 
		"branch", "--show-current")
	branchOut, err := branchCmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to get branch: %w", err)
	}

	// Get status
	statusCmd := exec.CommandContext(ctx, "git", "-C", worktreePath, 
		"status", "--porcelain")
	statusOut, err := statusCmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to get status: %w", err)
	}

	// Parse status
	var modified, untracked []string
	for _, line := range strings.Split(string(statusOut), "\n") {
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
	commitCmd := exec.CommandContext(ctx, "git", "-C", worktreePath, 
		"rev-parse", "HEAD")
	commitOut, err := commitCmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to get commit: %w", err)
	}

	return &Status{
		Branch:        strings.TrimSpace(string(branchOut)),
		Clean:         len(modified) == 0 && len(untracked) == 0,
		Modified:      modified,
		Untracked:     untracked,
		CurrentCommit: strings.TrimSpace(string(commitOut)),
	}, nil
}

// Status represents git status
type Status struct {
	Branch        string
	Clean         bool
	Modified      []string
	Untracked     []string
	CurrentCommit string
}