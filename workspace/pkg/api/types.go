package api

// CreateWorkspaceRequest represents a request to create a new workspace
type CreateWorkspaceRequest struct {
	Name      string            `json:"name"`
	BaseImage string            `json:"base_image,omitempty"`
	GitBranch string            `json:"git_branch,omitempty"`
	Env       map[string]string `json:"env,omitempty"`
}

// CreateWorkspaceResponse contains the created workspace
type CreateWorkspaceResponse struct {
	Workspace interface{} `json:"workspace"`
}

// ExecRequest represents a command execution request
type ExecRequest struct {
	Command []string          `json:"command"`
	Env     map[string]string `json:"env,omitempty"`
	WorkDir string            `json:"work_dir,omitempty"`
}

// ExecResponse contains command execution results
type ExecResponse struct {
	ExitCode int    `json:"exit_code"`
	Stdout   string `json:"stdout"`
	Stderr   string `json:"stderr"`
}

// ErrorResponse represents an API error
type ErrorResponse struct {
	Error   string `json:"error"`
	Code    string `json:"code,omitempty"`
	Details string `json:"details,omitempty"`
}

// ListWorkspacesResponse contains all workspaces
type ListWorkspacesResponse struct {
	Workspaces []interface{} `json:"workspaces"`
}

// CreateBranchRequest represents a git branch creation request
type CreateBranchRequest struct {
	BranchName string `json:"branch_name"`
}

// CommitRequest represents a git commit request
type CommitRequest struct {
	Message string `json:"message"`
	Author  string `json:"author,omitempty"`
	Email   string `json:"email,omitempty"`
}

// GitStatusResponse contains git status information
type GitStatusResponse struct {
	Branch        string   `json:"branch"`
	Clean         bool     `json:"clean"`
	Modified      []string `json:"modified,omitempty"`
	Untracked     []string `json:"untracked,omitempty"`
	CurrentCommit string   `json:"current_commit"`
}
