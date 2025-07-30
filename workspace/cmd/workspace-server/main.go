package main

import (
	"context"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"dagger.io/dagger"
	"github.com/patina/workspace/pkg/api"
	"github.com/patina/workspace/pkg/workspace"
)

func getEnvOrDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func main() {
	// Set up logger
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))

	ctx := context.Background()

	// Connect to Dagger
	logger.Info("connecting to dagger")
	dag, err := dagger.Connect(ctx, dagger.WithLogOutput(os.Stderr))
	if err != nil {
		logger.Error("failed to connect to dagger", "error", err)
		os.Exit(1)
	}
	defer dag.Close()

	// Create workspace manager
	config := &workspace.ManagerConfig{
		ProjectRoot:  os.Getenv("PROJECT_ROOT"),
		WorktreeRoot: getEnvOrDefault("WORKTREE_ROOT", "/tmp/patina-worktrees"),
		DefaultImage: getEnvOrDefault("DEFAULT_IMAGE", "ubuntu:latest"),
	}

	// Log configuration
	logger.Info("workspace server configuration",
		"project_root", config.ProjectRoot,
		"worktree_root", config.WorktreeRoot,
		"default_image", config.DefaultImage)

	manager, err := workspace.NewManager(dag, config, logger)
	if err != nil {
		logger.Error("failed to create manager", "error", err)
		os.Exit(1)
	}

	// Load existing workspaces from git notes
	if err := manager.LoadExistingWorkspaces(ctx); err != nil {
		logger.Error("failed to load existing workspaces", "error", err)
		// Not fatal - continue with empty workspace list
	}

	// Create API handlers
	handlers := api.NewHandlers(manager, logger)

	// Set up routes
	mux := http.NewServeMux()
	mux.HandleFunc("/workspaces", handlers.HandleWorkspaces)
	mux.HandleFunc("/workspaces/", handlers.HandleWorkspace)
	mux.HandleFunc("/health", handlers.HandleHealth)

	// Create server
	srv := &http.Server{
		Addr:    ":8080",
		Handler: mux,
	}

	// Start server
	go func() {
		logger.Info("starting workspace server", "addr", srv.Addr)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Error("server error", "error", err)
		}
	}()

	// Wait for interrupt signal
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	logger.Info("shutting down server")

	// Graceful shutdown
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		logger.Error("server shutdown error", "error", err)
	}

	// Close manager
	if err := manager.Close(ctx); err != nil {
		logger.Error("manager close error", "error", err)
	}

	logger.Info("server stopped")
}
