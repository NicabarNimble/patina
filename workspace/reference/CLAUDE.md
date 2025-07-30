# Reference Architectures

This directory contains reference implementations that guide our workspace service design.

## rqlite
- **Purpose**: Shows safe Go patterns for testing and service architecture
- **Key patterns**: Test files next to code, no external test frameworks, descriptive test names, error variables
- **Why it matters**: We follow their testing philosophy throughout our workspace service

## container-use
- **Purpose**: Demonstrates clever Dagger container usage for isolated environments
- **Key patterns**: Container persistence, environment management, agent isolation
- **Why it matters**: Shows how to use Dagger for workspace isolation similar to our needs