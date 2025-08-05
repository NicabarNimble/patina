# Patina Setup Tool

Complete setup for Patina development - installs tools and creates PROJECT_DESIGN.toml.

## Quick Start

```bash
cd setup/
./setup.sh
```

This will:
1. Install Rust (if needed)
2. Compile the bootstrap tool
3. Install development tools interactively
4. Create PROJECT_DESIGN.toml for your project

## Options

```bash
./setup.sh --minimal    # Just essentials (Rust, Git)
./setup.sh --full       # Install everything automatically
./setup.sh --dry-run    # See what would be installed
```

## What It Does

### Installs Tools
- **Rust & Cargo** - Core language (required)
- **Git** - Version control (required)
- **Docker** - Container runtime (optional)
- **Go** - Programming language for Dagger (optional)
- **Dagger** - CI/CD pipeline tool (optional)

### Creates PROJECT_DESIGN.toml
After installing tools, it helps you create a PROJECT_DESIGN.toml with:
- Project name, type, and purpose
- Placeholder sections for you to fill in later
- Ready to use with `patina init`

## How It Works

1. `setup.sh` - Shell script that ensures Rust is available
2. `bootstrap.rs` - Rust program that:
   - Installs other tools
   - Creates PROJECT_DESIGN.toml
3. No dependencies except `curl` and basic shell

## Alternative: Docker

For a containerized development environment, see the main project's docker-compose.yml (coming soon).

## Notes

- This tool is for native installations only
- Works on macOS and Linux
- Windows users should use WSL2 or Docker Desktop
- The compiled `bootstrap` binary is gitignored