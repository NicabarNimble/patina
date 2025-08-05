# Patina Bootstrap System

Complete system setup for Patina development in minutes.

## Quick Start

```bash
# One-line installation
curl -fsSL https://raw.githubusercontent.com/patina-dev/patina/main/scripts/bootstrap.sh | bash

# Or clone and run locally
./scripts/bootstrap.sh
```

## What It Does

The bootstrap system prepares your entire development environment:

### Core Tools (Always Installed)
- **Rust & Cargo** - Via rustup for easy toolchain management
- **Git** - Version control
- **Curl** - For downloading tools

### Optional Tools (Skip with `--minimal`)
- **Docker** - Container runtime
- **Go** - Required for Dagger
- **Dagger** - Smart build pipelines
- **Make** - Build automation

### System Setup
- Detects OS (macOS/Linux) and architecture
- Installs appropriate package manager (Homebrew on macOS)
- Updates shell configuration (bash/zsh/fish)
- Creates PROJECT_DESIGN.toml (with `--with-design`)

## Usage Options

```bash
# Full installation with prompts
./scripts/bootstrap.sh

# Automatic yes to all prompts
./scripts/bootstrap.sh --yes

# Minimal installation (Rust, Git, Curl only)
./scripts/bootstrap.sh --minimal

# Also create a PROJECT_DESIGN.toml
./scripts/bootstrap.sh --with-design

# Combine options
./scripts/bootstrap.sh --yes --minimal --with-design
```

## Platform Support

### macOS
- Installs Homebrew if needed
- Uses brew for most packages
- Handles M1/M2 (arm64) paths correctly

### Linux
- Supports: Ubuntu/Debian (apt), Fedora/RHEL (dnf/yum), Arch (pacman)
- Falls back to binary installations when needed
- Handles Docker group permissions

## Post-Installation

After bootstrap completes:

1. **Restart your shell** or run:
   ```bash
   source ~/.bashrc  # or ~/.zshrc
   ```

2. **Verify installation**:
   ```bash
   patina --version
   ```

3. **Initialize a project**:
   ```bash
   patina init myproject --llm=claude --design=PROJECT_DESIGN.toml
   ```

## Bootstrap as Rust Binary

For more control, build the Rust version:

```bash
# Add to Cargo.toml
[[bin]]
name = "patina-bootstrap"
path = "src/bin/patina-bootstrap.rs"

# Build and install
cargo build --release --bin patina-bootstrap
cp target/release/patina-bootstrap ~/bin/

# Run
patina-bootstrap --help
```

## Design Philosophy

This bootstrap tool embodies Patina's principles:

1. **Fast Setup** - Get developers productive quickly
2. **Smart Defaults** - Detect and adapt to the system
3. **Escape Hatches** - Never force installations
4. **Clear Feedback** - Show what's happening and why

## Troubleshooting

### Permission Denied
```bash
# Make script executable
chmod +x scripts/bootstrap.sh
```

### Docker Requires Sudo
The script adds your user to the docker group. Log out and back in.

### Path Not Updated
Manually add to your shell config:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
export PATH="/usr/local/go/bin:$PATH"
export PATH="$HOME/.dagger/bin:$PATH"
```

### Tool Installation Failed
Check the verbose output:
```bash
./scripts/bootstrap.sh --verbose
```

## Security

The bootstrap script:
- Only downloads from official sources
- Verifies HTTPS connections
- Never runs with unnecessary privileges
- Shows all commands in verbose mode

## Contributing

To improve the bootstrap experience:
1. Test on your platform
2. Add support for new tools
3. Improve error messages
4. Add platform-specific optimizations