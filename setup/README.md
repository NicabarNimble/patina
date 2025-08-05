# Patina Setup Tool

A standalone tool to quickly set up a native development environment for Patina.

## Quick Start

```bash
cd setup/
./setup.sh
```

This will:
1. Install Rust (if needed)
2. Compile the bootstrap tool
3. Run interactive setup to install development tools

## Options

```bash
./setup.sh --minimal    # Just essentials (Rust, Git)
./setup.sh --full       # Install everything automatically
./setup.sh --dry-run    # See what would be installed
```

## What It Installs

- **Rust & Cargo** - Core language (required)
- **Git** - Version control (required)
- **Docker** - Container runtime (optional)
- **Go** - Programming language for Dagger (optional)
- **Dagger** - CI/CD pipeline tool (optional)

## How It Works

1. `setup.sh` - Shell script that ensures Rust is available
2. `bootstrap.rs` - Rust program that installs other tools
3. No dependencies except `curl` and basic shell

## Alternative: Docker

For a containerized development environment, see the main project's docker-compose.yml (coming soon).

## Notes

- This tool is for native installations only
- Works on macOS and Linux
- Windows users should use WSL2 or Docker Desktop
- The compiled `bootstrap` binary is gitignored