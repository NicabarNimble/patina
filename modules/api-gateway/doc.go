// Package gateway is the coordination layer - the only "system" in our modular architecture.
//
// Unlike the other modules which are "tools" (single responsibility, clear I/O),
// the gateway is a system that coordinates multiple tools to provide workspace
// functionality. It orchestrates the environment-provider, registry, executor,
// and git-manager modules.
//
// This follows the pattern-selection-framework: decompose systems into tools,
// then coordinate those tools with a thin system layer.
package gateway