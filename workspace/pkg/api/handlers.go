package api

import (
	"encoding/json"
	"log/slog"
	"net/http"
	"strings"

	"github.com/patina/workspace/pkg/workspace"
)

// Handlers contains HTTP handlers for the workspace API
type Handlers struct {
	manager workspace.WorkspaceManager
	logger  *slog.Logger
}

// NewHandlers creates a new handlers instance
func NewHandlers(manager workspace.WorkspaceManager, logger *slog.Logger) *Handlers {
	return &Handlers{
		manager: manager,
		logger:  logger,
	}
}

// HandleWorkspaces handles /workspaces endpoints
func (h *Handlers) HandleWorkspaces(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case http.MethodGet:
		h.listWorkspaces(w, r)
	case http.MethodPost:
		h.createWorkspace(w, r)
	default:
		h.methodNotAllowed(w, r)
	}
}

// HandleWorkspace handles /workspaces/{id} endpoints
func (h *Handlers) HandleWorkspace(w http.ResponseWriter, r *http.Request) {
	// Extract workspace ID from path
	path := strings.TrimPrefix(r.URL.Path, "/workspaces/")
	parts := strings.Split(path, "/")
	
	if len(parts) < 1 || parts[0] == "" {
		h.notFound(w, r)
		return
	}
	
	workspaceID := parts[0]
	
	// Route based on remaining path
	if len(parts) == 1 {
		switch r.Method {
		case http.MethodGet:
			h.getWorkspace(w, r, workspaceID)
		case http.MethodDelete:
			h.deleteWorkspace(w, r, workspaceID)
		default:
			h.methodNotAllowed(w, r)
		}
	} else if len(parts) == 2 {
		switch parts[1] {
		case "exec":
			if r.Method == http.MethodPost {
				h.execInWorkspace(w, r, workspaceID)
			} else {
				h.methodNotAllowed(w, r)
			}
		case "git":
			h.handleGitOperations(w, r, workspaceID)
		default:
			h.notFound(w, r)
		}
	} else if len(parts) == 3 && parts[1] == "git" {
		h.handleSpecificGitOperation(w, r, workspaceID, parts[2])
	} else {
		h.notFound(w, r)
	}
}

// HandleHealth handles health check requests
func (h *Handlers) HandleHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "healthy",
	})
}

// listWorkspaces returns all workspaces
func (h *Handlers) listWorkspaces(w http.ResponseWriter, r *http.Request) {
	workspaces, err := h.manager.ListWorkspaces()
	if err != nil {
		h.error(w, err, http.StatusInternalServerError)
		return
	}
	
	// Convert to interface slice for response
	items := make([]interface{}, len(workspaces))
	for i, ws := range workspaces {
		items[i] = ws
	}
	
	h.json(w, ListWorkspacesResponse{Workspaces: items})
}

// createWorkspace creates a new workspace
func (h *Handlers) createWorkspace(w http.ResponseWriter, r *http.Request) {
	var req CreateWorkspaceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.error(w, err, http.StatusBadRequest)
		return
	}
	
	// Validate request
	if req.Name == "" {
		h.errorWithCode(w, "name is required", "INVALID_REQUEST", http.StatusBadRequest)
		return
	}
	
	// Create workspace config
	config := &workspace.Config{
		BaseImage:   req.BaseImage,
		Environment: req.Env,
	}
	
	// Create workspace
	ws, err := h.manager.CreateWorkspace(r.Context(), req.Name, config)
	if err != nil {
		h.error(w, err, http.StatusInternalServerError)
		return
	}
	
	h.json(w, CreateWorkspaceResponse{Workspace: ws})
}

// getWorkspace returns a specific workspace
func (h *Handlers) getWorkspace(w http.ResponseWriter, r *http.Request, id string) {
	ws, err := h.manager.GetWorkspace(id)
	if err != nil {
		h.error(w, err, http.StatusNotFound)
		return
	}
	
	h.json(w, ws)
}

// deleteWorkspace removes a workspace
func (h *Handlers) deleteWorkspace(w http.ResponseWriter, r *http.Request, id string) {
	if err := h.manager.DeleteWorkspace(r.Context(), id); err != nil {
		h.error(w, err, http.StatusInternalServerError)
		return
	}
	
	w.WriteHeader(http.StatusNoContent)
}

// execInWorkspace executes a command in a workspace
func (h *Handlers) execInWorkspace(w http.ResponseWriter, r *http.Request, id string) {
	var req ExecRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.error(w, err, http.StatusBadRequest)
		return
	}
	
	// Validate request
	if len(req.Command) == 0 {
		h.errorWithCode(w, "command is required", "INVALID_REQUEST", http.StatusBadRequest)
		return
	}
	
	// Create exec options
	opts := &workspace.ExecOptions{
		Command:     req.Command,
		WorkDir:     req.WorkDir,
		Environment: req.Env,
	}
	
	// Execute command
	result, err := h.manager.Execute(r.Context(), id, opts)
	if err != nil {
		if workspace.IsNotFound(err) {
			h.error(w, err, http.StatusNotFound)
		} else if workspace.IsNotReady(err) {
			h.errorWithCode(w, "workspace not ready", "NOT_READY", http.StatusServiceUnavailable)
		} else {
			h.error(w, err, http.StatusInternalServerError)
		}
		return
	}
	
	// Convert to response
	resp := ExecResponse{
		ExitCode: result.ExitCode,
		Stdout:   result.Stdout,
		Stderr:   result.Stderr,
	}
	
	h.json(w, resp)
}

// Helper methods

func (h *Handlers) json(w http.ResponseWriter, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(data); err != nil {
		h.logger.Error("failed to encode response", "error", err)
	}
}

func (h *Handlers) error(w http.ResponseWriter, err interface{}, status int) {
	h.errorWithCode(w, err, "", status)
}

func (h *Handlers) errorWithCode(w http.ResponseWriter, err interface{}, code string, status int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	
	resp := ErrorResponse{
		Code: code,
	}
	
	switch v := err.(type) {
	case string:
		resp.Error = v
	case error:
		resp.Error = v.Error()
	default:
		resp.Error = "unknown error"
	}
	
	json.NewEncoder(w).Encode(resp)
}

func (h *Handlers) methodNotAllowed(w http.ResponseWriter, r *http.Request) {
	h.error(w, "method not allowed", http.StatusMethodNotAllowed)
}

func (h *Handlers) notFound(w http.ResponseWriter, r *http.Request) {
	h.error(w, "not found", http.StatusNotFound)
}

// Git operations handlers

func (h *Handlers) handleGitOperations(w http.ResponseWriter, r *http.Request, workspaceID string) {
	// GET /workspaces/{id}/git - Get git status
	if r.Method == http.MethodGet {
		h.getGitStatus(w, r, workspaceID)
	} else {
		h.methodNotAllowed(w, r)
	}
}

func (h *Handlers) handleSpecificGitOperation(w http.ResponseWriter, r *http.Request, workspaceID, operation string) {
	if r.Method != http.MethodPost {
		h.methodNotAllowed(w, r)
		return
	}

	switch operation {
	case "branch":
		h.createBranch(w, r, workspaceID)
	case "commit":
		h.commitChanges(w, r, workspaceID)
	case "push":
		h.pushBranch(w, r, workspaceID)
	default:
		h.notFound(w, r)
	}
}

func (h *Handlers) getGitStatus(w http.ResponseWriter, r *http.Request, workspaceID string) {
	status, err := h.manager.GetGitStatus(r.Context(), workspaceID)
	if err != nil {
		if workspace.IsNotFound(err) {
			h.error(w, err, http.StatusNotFound)
		} else {
			h.error(w, err, http.StatusInternalServerError)
		}
		return
	}
	
	h.json(w, status)
}

func (h *Handlers) createBranch(w http.ResponseWriter, r *http.Request, workspaceID string) {
	var req CreateBranchRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.error(w, err, http.StatusBadRequest)
		return
	}
	
	if req.BranchName == "" {
		h.errorWithCode(w, "branch_name is required", "INVALID_REQUEST", http.StatusBadRequest)
		return
	}
	
	if err := h.manager.CreateBranch(r.Context(), workspaceID, req.BranchName); err != nil {
		if workspace.IsNotFound(err) {
			h.error(w, err, http.StatusNotFound)
		} else {
			h.error(w, err, http.StatusInternalServerError)
		}
		return
	}
	
	w.WriteHeader(http.StatusNoContent)
}

func (h *Handlers) commitChanges(w http.ResponseWriter, r *http.Request, workspaceID string) {
	var req CommitRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.error(w, err, http.StatusBadRequest)
		return
	}
	
	if req.Message == "" {
		h.errorWithCode(w, "message is required", "INVALID_REQUEST", http.StatusBadRequest)
		return
	}
	
	opts := &workspace.GitOptions{
		Message: req.Message,
		Author:  req.Author,
		Email:   req.Email,
	}
	
	if err := h.manager.CommitChanges(r.Context(), workspaceID, opts); err != nil {
		if workspace.IsNotFound(err) {
			h.error(w, err, http.StatusNotFound)
		} else {
			h.error(w, err, http.StatusInternalServerError)
		}
		return
	}
	
	w.WriteHeader(http.StatusNoContent)
}

func (h *Handlers) pushBranch(w http.ResponseWriter, r *http.Request, workspaceID string) {
	if err := h.manager.PushBranch(r.Context(), workspaceID); err != nil {
		if workspace.IsNotFound(err) {
			h.error(w, err, http.StatusNotFound)
		} else {
			h.error(w, err, http.StatusInternalServerError)
		}
		return
	}
	
	w.WriteHeader(http.StatusNoContent)
}