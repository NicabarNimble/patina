// Package executor is a patch of patina - a pure tool for command execution.
//
// This module provides stateless command execution in containers, extracting
// the execution logic from the workspace system. It follows the Eternal Tool
// pattern with clear input (command + container) to output (results) transformation.
//
// The executor knows nothing about workspaces, only how to run commands
// in Dagger containers and return structured results.
package executor