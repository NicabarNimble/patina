package workspace

import "errors"

// Package-level error definitions following rqlite pattern
var (
	// ErrWorkspaceNotFound indicates the requested workspace doesn't exist
	ErrWorkspaceNotFound = errors.New("workspace not found")

	// ErrWorkspaceExists indicates a workspace with the same name already exists
	ErrWorkspaceExists = errors.New("workspace already exists")

	// ErrContainerNotReady indicates the container is not in ready state
	ErrContainerNotReady = errors.New("container not ready")

	// ErrContainerFailed indicates container operation failed
	ErrContainerFailed = errors.New("container operation failed")

	// ErrManagerClosed indicates the manager has been closed
	ErrManagerClosed = errors.New("manager is closed")

	// ErrInvalidConfig indicates invalid workspace configuration
	ErrInvalidConfig = errors.New("invalid workspace configuration")

	// ErrExecFailed indicates command execution failed
	ErrExecFailed = errors.New("command execution failed")

	// ErrTimeout indicates operation timed out
	ErrTimeout = errors.New("operation timed out")

	// ErrNoDaggerClient indicates Dagger client is not initialized
	ErrNoDaggerClient = errors.New("dagger client not initialized")
)

// IsNotFound returns true if the error is ErrWorkspaceNotFound
func IsNotFound(err error) bool {
	return errors.Is(err, ErrWorkspaceNotFound)
}

// IsNotReady returns true if the error is ErrContainerNotReady
func IsNotReady(err error) bool {
	return errors.Is(err, ErrContainerNotReady)
}