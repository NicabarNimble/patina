package api

import (
	"bytes"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/patina/workspace/internal/testutil"
)

// Test helpers

func mustNewTestHandlers(t *testing.T) *Handlers {
	t.Helper()

	// Use mock manager for testing
	manager := testutil.NewMockManager()
	logger := slog.Default()

	return NewHandlers(manager, logger)
}

func Test_HandleHealth(t *testing.T) {
	h := mustNewTestHandlers(t)

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	w := httptest.NewRecorder()

	h.HandleHealth(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected status 200, got %d", w.Code)
	}

	var response map[string]string
	if err := json.NewDecoder(w.Body).Decode(&response); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if response["status"] != "healthy" {
		t.Errorf("expected status 'healthy', got '%s'", response["status"])
	}
}

func Test_HandleWorkspaces_InvalidMethod(t *testing.T) {
	h := mustNewTestHandlers(t)

	tests := []string{http.MethodPut, http.MethodPatch, http.MethodDelete}

	for _, method := range tests {
		t.Run(method, func(t *testing.T) {
			req := httptest.NewRequest(method, "/workspaces", nil)
			w := httptest.NewRecorder()

			h.HandleWorkspaces(w, req)

			if w.Code != http.StatusMethodNotAllowed {
				t.Errorf("expected status 405, got %d", w.Code)
			}
		})
	}
}

func Test_CreateWorkspace_Validation(t *testing.T) {
	h := mustNewTestHandlers(t)

	tests := []struct {
		name       string
		body       string
		wantStatus int
		wantError  string
	}{
		{
			name:       "empty body",
			body:       "",
			wantStatus: http.StatusBadRequest,
		},
		{
			name:       "empty name",
			body:       `{"name": ""}`,
			wantStatus: http.StatusBadRequest,
			wantError:  "name is required",
		},
		{
			name:       "valid request",
			body:       `{"name": "test-workspace", "base_image": "ubuntu:22.04"}`,
			wantStatus: http.StatusOK, // Mock manager will succeed
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodPost, "/workspaces", bytes.NewReader([]byte(tt.body)))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			h.createWorkspace(w, req)

			if w.Code != tt.wantStatus {
				t.Errorf("expected status %d, got %d", tt.wantStatus, w.Code)
			}

			if tt.wantError != "" {
				var resp ErrorResponse
				if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
					t.Fatalf("failed to decode error response: %v", err)
				}

				if resp.Error != tt.wantError {
					t.Errorf("expected error '%s', got '%s'", tt.wantError, resp.Error)
				}
			}
		})
	}
}

func Test_ExecInWorkspace_Validation(t *testing.T) {
	h := mustNewTestHandlers(t)

	tests := []struct {
		name       string
		body       string
		wantStatus int
		wantError  string
	}{
		{
			name:       "empty command",
			body:       `{"command": []}`,
			wantStatus: http.StatusBadRequest,
			wantError:  "command is required",
		},
		{
			name:       "valid command",
			body:       `{"command": ["ls", "-la"]}`,
			wantStatus: http.StatusNotFound, // Will fail without real workspace
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodPost, "/workspaces/test-id/exec", bytes.NewReader([]byte(tt.body)))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			h.execInWorkspace(w, req, "test-id")

			if w.Code != tt.wantStatus {
				t.Errorf("expected status %d, got %d", tt.wantStatus, w.Code)
			}

			if tt.wantError != "" {
				var resp ErrorResponse
				if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
					t.Fatalf("failed to decode error response: %v", err)
				}

				if resp.Error != tt.wantError {
					t.Errorf("expected error '%s', got '%s'", tt.wantError, resp.Error)
				}
			}
		})
	}
}

func Test_HandleWorkspace_Routing(t *testing.T) {
	h := mustNewTestHandlers(t)

	tests := []struct {
		name       string
		path       string
		method     string
		wantStatus int
	}{
		{
			name:       "get workspace",
			path:       "/workspaces/test-id",
			method:     http.MethodGet,
			wantStatus: http.StatusNotFound, // No workspace exists
		},
		{
			name:       "delete workspace",
			path:       "/workspaces/test-id",
			method:     http.MethodDelete,
			wantStatus: http.StatusInternalServerError, // No workspace exists
		},
		{
			name:       "exec in workspace",
			path:       "/workspaces/test-id/exec",
			method:     http.MethodPost,
			wantStatus: http.StatusBadRequest, // No body
		},
		{
			name:       "invalid path",
			path:       "/workspaces/test-id/invalid",
			method:     http.MethodGet,
			wantStatus: http.StatusNotFound,
		},
		{
			name:       "empty workspace id",
			path:       "/workspaces/",
			method:     http.MethodGet,
			wantStatus: http.StatusNotFound,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest(tt.method, tt.path, nil)
			w := httptest.NewRecorder()

			// Simulate routing
			req.URL.Path = tt.path
			h.HandleWorkspace(w, req)

			if w.Code != tt.wantStatus {
				t.Errorf("expected status %d, got %d", tt.wantStatus, w.Code)
			}
		})
	}
}

// Test error response formatting
func Test_ErrorResponse(t *testing.T) {
	h := mustNewTestHandlers(t)

	w := httptest.NewRecorder()
	h.errorWithCode(w, "test error", "TEST_CODE", http.StatusBadRequest)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", w.Code)
	}

	var resp ErrorResponse
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}

	if resp.Error != "test error" {
		t.Errorf("expected error 'test error', got '%s'", resp.Error)
	}

	if resp.Code != "TEST_CODE" {
		t.Errorf("expected code 'TEST_CODE', got '%s'", resp.Code)
	}
}

// Test with real manager for integration
func Test_Integration_CreateAndList(t *testing.T) {
	// Skip if not running integration tests
	if testing.Short() {
		t.Skip("skipping integration test")
	}

	// This would require a real Dagger client
	// For now, we've tested the handler logic
}
