#!/usr/bin/env bash
#
# Patina Setup - Native installation helper
# 
# This script:
# 1. Ensures Rust is installed (required to run our tool)
# 2. Compiles the bootstrap tool
# 3. Runs it to set up your environment
#

set -e

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${BLUE}${BOLD}🚀 Patina Setup${NC}"
echo -e "Native development environment setup\n"

# Step 1: Make sure we have Rust
if ! command -v rustc &> /dev/null; then
    echo "📦 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓${NC} Rust installed"
else
    echo -e "${GREEN}✓${NC} Rust is available"
fi

# Step 2: Compile our bootstrap tool
echo -e "\n📦 Compiling bootstrap tool..."

cd "$SCRIPT_DIR"
rustc bootstrap.rs -o bootstrap 2>/dev/null || {
    echo "Compilation failed, trying with newer edition..."
    rustc bootstrap.rs -o bootstrap --edition=2021
}

echo -e "${GREEN}✓${NC} Bootstrap tool ready"

# Step 3: Run it
echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"

exec ./bootstrap "$@"