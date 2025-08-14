package main

import (
	"context"
	"log"
	"net/http"
	"os"

	"dagger.io/dagger"
	gateway "github.com/your-org/patina/modules/api-gateway"
)

func main() {
	ctx := context.Background()

	// Connect to Dagger
	client, err := dagger.Connect(ctx, dagger.WithLogOutput(os.Stderr))
	if err != nil {
		log.Fatal("failed to connect to dagger:", err)
	}
	defer client.Close()

	// Get config from environment
	repoPath := os.Getenv("PROJECT_ROOT")
	if repoPath == "" {
		repoPath = "."
	}

	worktreeRoot := os.Getenv("WORKTREE_ROOT")
	if worktreeRoot == "" {
		worktreeRoot = "/tmp/patina-worktrees"
	}

	// Create gateway
	gw, err := gateway.New(client, &gateway.Config{
		RepoPath:     repoPath,
		WorktreeRoot: worktreeRoot,
	})
	if err != nil {
		log.Fatal("failed to create gateway:", err)
	}

	// Create HTTP handlers
	handlers := gateway.NewHTTPHandlers(gw)

	// Setup routes
	mux := http.NewServeMux()
	handlers.RegisterRoutes(mux)

	// Add health check
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("ok"))
	})

	// Start server
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	log.Printf("Starting modular workspace server on :%s", port)
	log.Printf("  Repo: %s", repoPath)
	log.Printf("  Worktrees: %s", worktreeRoot)
	
	if err := http.ListenAndServe(":"+port, mux); err != nil {
		log.Fatal("server failed:", err)
	}
}