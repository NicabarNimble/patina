package gateway

import (
	"encoding/json"
	"net/http"
	"strings"
)

// HTTPHandlers provides HTTP API for the gateway
type HTTPHandlers struct {
	gateway *Gateway
}

// NewHTTPHandlers creates HTTP handlers for the gateway
func NewHTTPHandlers(g *Gateway) *HTTPHandlers {
	return &HTTPHandlers{gateway: g}
}

// CreateWorkspaceRequest represents workspace creation request
type CreateWorkspaceRequest struct {
	Name   string `json:"name"`
	Branch string `json:"branch,omitempty"`
}

// ExecuteRequest represents command execution request
type ExecuteRequest struct {
	Command []string `json:"command"`
}

// GitCommitRequest represents git commit request
type GitCommitRequest struct {
	Message string `json:"message"`
	Author  string `json:"author,omitempty"`
	Email   string `json:"email,omitempty"`
}

// HandleCreateWorkspace handles POST /workspaces
func (h *HTTPHandlers) HandleCreateWorkspace(w http.ResponseWriter, r *http.Request) {
	var req CreateWorkspaceRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if req.Branch == "" {
		req.Branch = "workspace-" + req.Name
	}

	id, err := h.gateway.CreateWorkspace(r.Context(), req.Name, req.Branch)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{"id": id})
}

// HandleGetWorkspace handles GET /workspaces/{id}
func (h *HTTPHandlers) HandleGetWorkspace(w http.ResponseWriter, r *http.Request) {
	id := strings.TrimPrefix(r.URL.Path, "/workspaces/")
	
	env, err := h.gateway.GetWorkspace(id)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(env)
}

// HandleListWorkspaces handles GET /workspaces
func (h *HTTPHandlers) HandleListWorkspaces(w http.ResponseWriter, r *http.Request) {
	envs, err := h.gateway.ListWorkspaces()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Ensure we return empty array instead of null
	if envs == nil {
		envs = []*registry.Environment{}
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(envs)
}

// HandleDeleteWorkspace handles DELETE /workspaces/{id}
func (h *HTTPHandlers) HandleDeleteWorkspace(w http.ResponseWriter, r *http.Request) {
	id := strings.TrimPrefix(r.URL.Path, "/workspaces/")
	
	if err := h.gateway.DeleteWorkspace(r.Context(), id); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// HandleExecute handles POST /workspaces/{id}/execute
func (h *HTTPHandlers) HandleExecute(w http.ResponseWriter, r *http.Request) {
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 4 {
		http.Error(w, "invalid path", http.StatusBadRequest)
		return
	}
	id := parts[2]

	var req ExecuteRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	result, err := h.gateway.Execute(r.Context(), id, req.Command)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(result)
}

// HandleGitStatus handles GET /workspaces/{id}/git/status
func (h *HTTPHandlers) HandleGitStatus(w http.ResponseWriter, r *http.Request) {
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 5 {
		http.Error(w, "invalid path", http.StatusBadRequest)
		return
	}
	id := parts[2]

	status, err := h.gateway.GetGitStatus(r.Context(), id)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(status)
}

// HandleGitCommit handles POST /workspaces/{id}/git/commit
func (h *HTTPHandlers) HandleGitCommit(w http.ResponseWriter, r *http.Request) {
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 5 {
		http.Error(w, "invalid path", http.StatusBadRequest)
		return
	}
	id := parts[2]

	var req GitCommitRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := h.gateway.CommitChanges(r.Context(), id, req.Message, req.Author, req.Email); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// RegisterRoutes registers all HTTP routes
func (h *HTTPHandlers) RegisterRoutes(mux *http.ServeMux) {
	mux.HandleFunc("/workspaces", func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodPost:
			h.HandleCreateWorkspace(w, r)
		case http.MethodGet:
			h.HandleListWorkspaces(w, r)
		default:
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		}
	})

	// Pattern matching for /workspaces/{id}
	mux.HandleFunc("/workspaces/", func(w http.ResponseWriter, r *http.Request) {
		path := r.URL.Path
		
		if strings.HasSuffix(path, "/execute") {
			if r.Method == http.MethodPost {
				h.HandleExecute(w, r)
			} else {
				http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			}
		} else if strings.HasSuffix(path, "/git/status") {
			if r.Method == http.MethodGet {
				h.HandleGitStatus(w, r)
			} else {
				http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			}
		} else if strings.HasSuffix(path, "/git/commit") {
			if r.Method == http.MethodPost {
				h.HandleGitCommit(w, r)
			} else {
				http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			}
		} else {
			// Base workspace operations
			switch r.Method {
			case http.MethodGet:
				h.HandleGetWorkspace(w, r)
			case http.MethodDelete:
				h.HandleDeleteWorkspace(w, r)
			default:
				http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			}
		}
	})
}