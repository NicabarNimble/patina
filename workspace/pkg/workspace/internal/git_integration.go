package internal

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

const (
	// Git notes refs for workspace state (following container-use pattern)
	gitNotesStateRef = "patina-workspace-state"
	gitNotesLogRef   = "patina-workspace-log"
)

// GitIntegration handles git worktree operations for workspaces
type GitIntegration struct {
	repoPath     string // Main repository path
	worktreeRoot string // Root directory for all worktrees
}

// NewGitIntegration creates a new git integration handler
func NewGitIntegration(repoPath, worktreeRoot string) (*GitIntegration, error) {
	// Verify repo path is a git repository
	cmd := exec.Command("git", "-C", repoPath, "rev-parse", "--git-dir")
	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("not a git repository: %s", repoPath)
	}

	// Ensure worktree root exists
	if err := os.MkdirAll(worktreeRoot, 0755); err != nil {
		return nil, fmt.Errorf("failed to create worktree root: %w", err)
	}

	return &GitIntegration{
		repoPath:     repoPath,
		worktreeRoot: worktreeRoot,
	}, nil
}

// CreateWorktree creates a new git worktree for a workspace
func (g *GitIntegration) CreateWorktree(ctx context.Context, workspaceID, branchName string) (string, error) {
	worktreePath := filepath.Join(g.worktreeRoot, workspaceID)

	// Check if branch already exists
	checkCmd := exec.CommandContext(ctx, "git", "-C", g.repoPath, "show-ref", "--verify", "--quiet", "refs/heads/"+branchName)
	branchExists := checkCmd.Run() == nil

	var cmd *exec.Cmd
	if branchExists {
		// If branch exists, create worktree pointing to it
		cmd = exec.CommandContext(ctx, "git", "-C", g.repoPath, "worktree", "add", worktreePath, branchName)
	} else {
		// Create new branch with worktree
		cmd = exec.CommandContext(ctx, "git", "-C", g.repoPath, "worktree", "add", "-b", branchName, worktreePath)
	}

	output, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("failed to create worktree: %w\nOutput: %s", err, output)
	}

	// Create an initial commit in the worktree to ensure each workspace has its own HEAD
	// This prevents git notes from being shared across worktrees
	gitkeepPath := filepath.Join(worktreePath, ".patina-workspace")
	if err := os.WriteFile(gitkeepPath, []byte(workspaceID), 0644); err != nil {
		return "", fmt.Errorf("failed to create workspace marker: %w", err)
	}

	addCmd := exec.CommandContext(ctx, "git", "-C", worktreePath, "add", ".patina-workspace")
	if err := addCmd.Run(); err != nil {
		return "", fmt.Errorf("failed to add workspace marker: %w", err)
	}

	commitCmd := exec.CommandContext(ctx, "git", "-C", worktreePath,
		"commit", "-m", fmt.Sprintf("Initialize workspace %s", workspaceID))
	if output, err := commitCmd.CombinedOutput(); err != nil {
		// It's OK if there's nothing to commit (branch might already have the file)
		if !strings.Contains(string(output), "nothing to commit") {
			return "", fmt.Errorf("failed to create initial commit: %w\nOutput: %s", err, output)
		}
	}

	return worktreePath, nil
}

// RemoveWorktree removes a git worktree
func (g *GitIntegration) RemoveWorktree(ctx context.Context, workspaceID string) error {
	worktreePath := filepath.Join(g.worktreeRoot, workspaceID)

	// Remove the worktree
	cmd := exec.CommandContext(ctx, "git", "-C", g.repoPath, "worktree", "remove", "--force", worktreePath)
	output, err := cmd.CombinedOutput()
	if err != nil {
		// If worktree doesn't exist, that's fine
		if strings.Contains(string(output), "is not a working tree") {
			return nil
		}
		return fmt.Errorf("failed to remove worktree: %w\nOutput: %s", err, output)
	}

	return nil
}

// GetCurrentCommit gets the current commit SHA of a worktree
func (g *GitIntegration) GetCurrentCommit(ctx context.Context, worktreePath string) (string, error) {
	cmd := exec.CommandContext(ctx, "git", "-C", worktreePath, "rev-parse", "HEAD")
	output, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("failed to get current commit: %w", err)
	}

	return strings.TrimSpace(string(output)), nil
}

// GetBranchName gets the current branch name of a worktree
func (g *GitIntegration) GetBranchName(ctx context.Context, worktreePath string) (string, error) {
	cmd := exec.CommandContext(ctx, "git", "-C", worktreePath, "branch", "--show-current")
	output, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("failed to get branch name: %w", err)
	}

	return strings.TrimSpace(string(output)), nil
}

// ListWorktrees lists all active worktrees
func (g *GitIntegration) ListWorktrees(ctx context.Context) ([]WorktreeInfo, error) {
	cmd := exec.CommandContext(ctx, "git", "-C", g.repoPath, "worktree", "list", "--porcelain")
	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to list worktrees: %w", err)
	}

	var worktrees []WorktreeInfo
	lines := strings.Split(string(output), "\n")

	var current WorktreeInfo
	for _, line := range lines {
		if line == "" {
			if current.Path != "" {
				worktrees = append(worktrees, current)
				current = WorktreeInfo{}
			}
			continue
		}

		parts := strings.SplitN(line, " ", 2)
		if len(parts) != 2 {
			continue
		}

		switch parts[0] {
		case "worktree":
			current.Path = parts[1]
		case "HEAD":
			current.Head = parts[1]
		case "branch":
			current.Branch = strings.TrimPrefix(parts[1], "refs/heads/")
		}
	}

	// Don't forget the last one
	if current.Path != "" {
		worktrees = append(worktrees, current)
	}

	return worktrees, nil
}

// WorktreeInfo contains information about a git worktree
type WorktreeInfo struct {
	Path   string
	Head   string
	Branch string
}

// SaveWorkspaceState saves workspace state to git notes
func (g *GitIntegration) SaveWorkspaceState(ctx context.Context, ws *Workspace) error {
	// Marshal workspace to JSON
	data, err := json.MarshalIndent(ws, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal workspace: %w", err)
	}

	// Ensure we have at least one commit in the worktree
	// Check if HEAD exists
	checkCmd := exec.CommandContext(ctx, "git", "-C", ws.WorktreePath, "rev-parse", "HEAD")
	if err := checkCmd.Run(); err != nil {
		// No commits yet, create an initial commit
		touchCmd := exec.CommandContext(ctx, "touch", filepath.Join(ws.WorktreePath, ".gitkeep"))
		if err := touchCmd.Run(); err != nil {
			return fmt.Errorf("failed to create .gitkeep: %w", err)
		}

		addCmd := exec.CommandContext(ctx, "git", "-C", ws.WorktreePath, "add", ".gitkeep")
		if err := addCmd.Run(); err != nil {
			return fmt.Errorf("failed to add .gitkeep: %w", err)
		}

		commitCmd := exec.CommandContext(ctx, "git", "-C", ws.WorktreePath,
			"commit", "-m", "Initial workspace commit")
		if output, err := commitCmd.CombinedOutput(); err != nil {
			return fmt.Errorf("failed to create initial commit: %w\nOutput: %s", err, output)
		}
	}

	// Write to temp file (git notes needs a file)
	tempFile, err := os.CreateTemp("", "patina-workspace-*.json")
	if err != nil {
		return fmt.Errorf("failed to create temp file: %w", err)
	}
	defer os.Remove(tempFile.Name())
	defer tempFile.Close()

	if _, err := tempFile.Write(data); err != nil {
		return fmt.Errorf("failed to write temp file: %w", err)
	}

	// Add note to the current commit in the worktree
	cmd := exec.CommandContext(ctx, "git", "-C", ws.WorktreePath,
		"notes", "--ref", gitNotesStateRef,
		"add", "-f", "-F", tempFile.Name())

	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("failed to save workspace state: %w\nOutput: %s", err, output)
	}

	return nil
}

// LoadWorkspaceState loads workspace state from git notes
func (g *GitIntegration) LoadWorkspaceState(ctx context.Context, worktreePath string) (*Workspace, error) {
	// First check if the worktree has any commits
	checkCmd := exec.CommandContext(ctx, "git", "-C", worktreePath, "rev-parse", "HEAD")
	if err := checkCmd.Run(); err != nil {
		// No commits in worktree yet
		return nil, ErrWorkspaceNotFound
	}

	// Get the state from git notes
	cmd := exec.CommandContext(ctx, "git", "-C", worktreePath,
		"notes", "--ref", gitNotesStateRef, "show")

	output, err := cmd.CombinedOutput()
	if err != nil {
		// Check both error message and output for "no note found"
		errStr := string(output)
		if strings.Contains(errStr, "no note found for object") || strings.Contains(errStr, "failed to resolve 'HEAD'") {
			return nil, ErrWorkspaceNotFound
		}
		return nil, fmt.Errorf("failed to load workspace state: %w\nOutput: %s", err, output)
	}

	// Unmarshal the JSON
	var ws Workspace
	if err := json.Unmarshal(output, &ws); err != nil {
		return nil, fmt.Errorf("failed to unmarshal workspace state: %w", err)
	}

	return &ws, nil
}

// LoadAllWorkspaceStates loads all workspace states by scanning worktrees
func (g *GitIntegration) LoadAllWorkspaceStates(ctx context.Context) ([]*Workspace, error) {
	// List all worktrees
	worktrees, err := g.ListWorktrees(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to list worktrees: %w", err)
	}

	var workspaces []*Workspace
	for _, wt := range worktrees {
		// Skip the main worktree
		if wt.Branch == "" || !strings.HasPrefix(wt.Branch, "workspace-") {
			continue
		}

		// Try to load workspace state
		ws, err := g.LoadWorkspaceState(ctx, wt.Path)
		if err != nil {
			// Log but continue - worktree might not have state yet
			// fmt.Printf("DEBUG: Failed to load state for worktree %s: %v\n", wt.Path, err)
			continue
		}

		workspaces = append(workspaces, ws)
	}

	return workspaces, nil
}

// AddWorkspaceLogEntry adds a log entry to git notes (for audit trail)
func (g *GitIntegration) AddWorkspaceLogEntry(ctx context.Context, worktreePath, entry string) error {
	cmd := exec.CommandContext(ctx, "git", "-C", worktreePath,
		"notes", "--ref", gitNotesLogRef,
		"append", "-m", entry)

	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("failed to add log entry: %w\nOutput: %s", err, output)
	}

	return nil
}
