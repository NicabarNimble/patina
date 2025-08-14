// Package provider is a patch of patina - a tool for creating environments.
//
// This module creates isolated development containers, extracting the
// environment creation logic from the workspace system. It follows the
// Eternal Tool pattern - a stable API for container creation that transforms
// configuration into running environments.
//
// The provider knows nothing about workspaces, registries, or execution -
// it only creates configured containers.
package provider