// Package gitmanager is a patch of patina - a focused tool for git operations.
//
// This module extracts git worktree management from the workspace system,
// providing a clean interface for version control operations. It follows
// the Eternal Tool pattern - stable operations that transform repository
// state in predictable ways.
//
// Operations include worktree creation/removal and status queries, with
// no dependencies on the larger workspace system.
package gitmanager