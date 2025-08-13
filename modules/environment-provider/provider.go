package provider

import (
	"context"
	"fmt"
	"time"

	"dagger.io/dagger"
)

// Config defines environment configuration
type Config struct {
	Name      string
	BaseImage string
	Mounts    map[string]string  // Read-only mounts
	Copies    map[string]string  // Copy directories (writable)
	EnvVars   map[string]string
}

// Environment represents a created environment
type Environment struct {
	ID        string
	Name      string
	Container *dagger.Container
	Config    *Config
	CreatedAt time.Time
}

// Provider creates isolated development environments
type Provider struct {
	client *dagger.Client
}

// New creates a new environment provider
func New(client *dagger.Client) *Provider {
	return &Provider{
		client: client,
	}
}

// Create creates a new isolated environment
func (p *Provider) Create(ctx context.Context, config *Config) (*Environment, error) {
	if config == nil {
		return nil, fmt.Errorf("config is required")
	}

	if config.BaseImage == "" {
		config.BaseImage = "ubuntu:latest"
	}

	// Create base container
	container := p.client.Container().From(config.BaseImage)

	// Apply environment variables
	for key, value := range config.EnvVars {
		container = container.WithEnvVariable(key, value)
	}

	// Apply read-only mounts
	for source, target := range config.Mounts {
		dir := p.client.Host().Directory(source, dagger.HostDirectoryOpts{
			Exclude: []string{".git"},
		})
		container = container.WithMountedDirectory(target, dir)
	}

	// Apply copied directories (writable)
	for source, target := range config.Copies {
		dir := p.client.Host().Directory(source, dagger.HostDirectoryOpts{
			Exclude: []string{".git"},
		})
		container = container.WithDirectory(target, dir)
	}

	// Set working directory
	container = container.WithWorkdir("/workspace")

	// Generate ID
	id := fmt.Sprintf("%s-%d", config.Name, time.Now().Unix())

	return &Environment{
		ID:        id,
		Name:      config.Name,
		Container: container,
		Config:    config,
		CreatedAt: time.Now(),
	}, nil
}

// CreateWithDefaults creates an environment with default settings
func (p *Provider) CreateWithDefaults(ctx context.Context, name string) (*Environment, error) {
	return p.Create(ctx, &Config{
		Name:      name,
		BaseImage: "ubuntu:latest",
	})
}