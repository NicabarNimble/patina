#!/usr/bin/env bash
#
# Patina Bootstrap Script
# Fast system setup for Patina development
#
# Usage: curl -fsSL https://patina.dev/bootstrap.sh | bash
#    or: ./bootstrap.sh [--minimal] [--yes] [--with-design]

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m' # No Color

# Configuration
MINIMAL=false
AUTO_YES=false
WITH_DESIGN=false
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --minimal)
            MINIMAL=true
            shift
            ;;
        -y|--yes)
            AUTO_YES=true
            shift
            ;;
        --with-design)
            WITH_DESIGN=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Helper functions
print_header() {
    echo -e "${BLUE}${BOLD}🚀 Patina Bootstrap System${NC}"
    echo -e "${DIM}Setting up your development environment...${NC}\n"
}

print_section() {
    echo -e "${BOLD}$1${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ️${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠️${NC} $1"
}

detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        OS="linux"
        # Detect distribution
        if [ -f /etc/os-release ]; then
            . /etc/os-release
            DISTRO=$ID
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
        DISTRO="macos"
    else
        print_error "Unsupported OS: $OSTYPE"
        exit 1
    fi
    
    # Detect architecture
    ARCH=$(uname -m)
    if [[ "$ARCH" == "x86_64" ]]; then
        ARCH="amd64"
    elif [[ "$ARCH" == "aarch64" ]] || [[ "$ARCH" == "arm64" ]]; then
        ARCH="arm64"
    fi
}

detect_shell() {
    SHELL_NAME=$(basename "$SHELL")
    case "$SHELL_NAME" in
        zsh)
            SHELL_RC="$HOME/.zshrc"
            ;;
        bash)
            SHELL_RC="$HOME/.bashrc"
            ;;
        fish)
            SHELL_RC="$HOME/.config/fish/config.fish"
            ;;
        *)
            SHELL_RC="$HOME/.profile"
            ;;
    esac
}

detect_package_manager() {
    if command -v brew &> /dev/null; then
        PKG_MANAGER="brew"
    elif command -v apt-get &> /dev/null; then
        PKG_MANAGER="apt"
    elif command -v dnf &> /dev/null; then
        PKG_MANAGER="dnf"
    elif command -v yum &> /dev/null; then
        PKG_MANAGER="yum"
    elif command -v pacman &> /dev/null; then
        PKG_MANAGER="pacman"
    else
        PKG_MANAGER="none"
    fi
}

check_tool() {
    local tool=$1
    if command -v "$tool" &> /dev/null; then
        version=$("$tool" --version 2>&1 | head -n1 || echo "installed")
        print_success "$tool - $version"
        return 0
    else
        print_error "$tool - not installed"
        return 1
    fi
}

install_homebrew() {
    if [[ "$OS" == "macos" ]] && ! command -v brew &> /dev/null; then
        print_info "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        
        # Add Homebrew to PATH for M1 Macs
        if [[ "$ARCH" == "arm64" ]]; then
            echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> "$SHELL_RC"
            eval "$(/opt/homebrew/bin/brew shellenv)"
        fi
        
        PKG_MANAGER="brew"
        print_success "Homebrew installed"
    fi
}

install_rust() {
    if ! command -v rustc &> /dev/null || ! command -v rustup &> /dev/null; then
        print_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        print_success "Rust installed"
    fi
}

install_docker() {
    if ! command -v docker &> /dev/null && [[ "$MINIMAL" == "false" ]]; then
        print_info "Installing Docker..."
        
        case "$OS" in
            macos)
                if [[ "$PKG_MANAGER" == "brew" ]]; then
                    brew install --cask docker
                    print_success "Docker Desktop installed"
                    print_info "Please start Docker Desktop from Applications"
                else
                    print_warning "Please install Docker Desktop manually from docker.com"
                fi
                ;;
            linux)
                curl -fsSL https://get.docker.com | sh
                sudo usermod -aG docker "$USER" 2>/dev/null || true
                print_success "Docker installed"
                print_info "You'll need to log out and back in for group changes"
                ;;
        esac
    fi
}

install_go() {
    if ! command -v go &> /dev/null && [[ "$MINIMAL" == "false" ]]; then
        print_info "Installing Go..."
        
        GO_VERSION="1.23.0"
        
        case "$PKG_MANAGER" in
            brew)
                brew install go
                ;;
            apt)
                sudo apt-get update && sudo apt-get install -y golang
                ;;
            dnf|yum)
                sudo $PKG_MANAGER install -y golang
                ;;
            *)
                # Manual installation
                GO_OS=$([[ "$OS" == "macos" ]] && echo "darwin" || echo "linux")
                curl -L "https://go.dev/dl/go${GO_VERSION}.${GO_OS}-${ARCH}.tar.gz" | sudo tar -C /usr/local -xzf -
                echo 'export PATH="/usr/local/go/bin:$PATH"' >> "$SHELL_RC"
                export PATH="/usr/local/go/bin:$PATH"
                ;;
        esac
        
        print_success "Go installed"
    fi
}

install_dagger() {
    if ! command -v dagger &> /dev/null && [[ "$MINIMAL" == "false" ]]; then
        if command -v go &> /dev/null; then
            print_info "Installing Dagger..."
            curl -fsSL https://dl.dagger.io/dagger/install.sh | sh
            
            # Add to PATH
            if [[ -d "$HOME/.dagger/bin" ]]; then
                echo 'export PATH="$HOME/.dagger/bin:$PATH"' >> "$SHELL_RC"
                export PATH="$HOME/.dagger/bin:$PATH"
            fi
            
            print_success "Dagger installed"
        else
            print_warning "Skipping Dagger (requires Go)"
        fi
    fi
}

install_git() {
    if ! command -v git &> /dev/null; then
        print_info "Installing Git..."
        
        case "$PKG_MANAGER" in
            brew)
                brew install git
                ;;
            apt)
                sudo apt-get update && sudo apt-get install -y git
                ;;
            dnf|yum)
                sudo $PKG_MANAGER install -y git
                ;;
            pacman)
                sudo pacman -S --noconfirm git
                ;;
            *)
                print_error "Cannot install Git automatically"
                ;;
        esac
        
        print_success "Git installed"
    fi
}

install_patina() {
    if command -v cargo &> /dev/null; then
        print_info "Installing Patina..."
        cargo install patina || cargo install --git https://github.com/patina-dev/patina
        print_success "Patina installed"
    fi
}

create_design_toml() {
    if [[ "$WITH_DESIGN" == "true" ]]; then
        print_section "\n📝 Creating PROJECT_DESIGN.toml"
        
        echo -n "Project name: "
        read -r project_name
        
        echo -n "Project purpose (one line): "
        read -r purpose
        
        cat > PROJECT_DESIGN.toml <<EOF
[project]
name = "${project_name:-my-project}"
type = "application"
purpose = "${purpose:-TODO: Add project purpose}"

[why]
problem = "TODO: What problem does this solve?"
solution = "TODO: How does it solve it?"
users = "developers"
value = "TODO: Core value proposition"

[how]
patterns = []
architecture = "TODO: High-level architecture"
core_abstractions = []

[what]
core_features = []
future_features = []
non_goals = []

[technical]
language = "rust"
dependencies = []
constraints = []

[development]
[development.commands]
test = "cargo test"
build = "cargo build"
run = "cargo run"
EOF
        
        print_success "Created PROJECT_DESIGN.toml"
    fi
}

update_shell_config() {
    local updated=false
    
    # Rust/Cargo
    if [[ -d "$HOME/.cargo/bin" ]] && ! grep -q ".cargo/bin" "$SHELL_RC" 2>/dev/null; then
        echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$SHELL_RC"
        updated=true
    fi
    
    # Go
    if [[ -d "/usr/local/go/bin" ]] && ! grep -q "/usr/local/go/bin" "$SHELL_RC" 2>/dev/null; then
        echo 'export PATH="/usr/local/go/bin:$PATH"' >> "$SHELL_RC"
        updated=true
    fi
    
    # Dagger
    if [[ -d "$HOME/.dagger/bin" ]] && ! grep -q ".dagger/bin" "$SHELL_RC" 2>/dev/null; then
        echo 'export PATH="$HOME/.dagger/bin:$PATH"' >> "$SHELL_RC"
        updated=true
    fi
    
    if [[ "$updated" == "true" ]]; then
        print_info "Updated $SHELL_RC with PATH entries"
    fi
}

# Main execution
main() {
    print_header
    
    # System detection
    print_section "📋 System Information"
    detect_os
    detect_shell
    detect_package_manager
    
    echo "   OS: $OS ($DISTRO) $ARCH"
    echo "   Shell: $SHELL_NAME ($SHELL_RC)"
    echo "   Package Manager: $PKG_MANAGER"
    echo
    
    # Tool check
    print_section "🔧 Checking Tools"
    TOOLS_NEEDED=()
    
    check_tool "rust" || TOOLS_NEEDED+=("rust")
    check_tool "cargo" || true
    check_tool "git" || TOOLS_NEEDED+=("git")
    check_tool "curl" || TOOLS_NEEDED+=("curl")
    
    if [[ "$MINIMAL" == "false" ]]; then
        check_tool "docker" || TOOLS_NEEDED+=("docker")
        check_tool "go" || TOOLS_NEEDED+=("go")
        check_tool "dagger" || TOOLS_NEEDED+=("dagger")
    fi
    
    echo
    
    # Installation confirmation
    if [[ ${#TOOLS_NEEDED[@]} -gt 0 ]]; then
        print_section "📦 Tools to Install"
        for tool in "${TOOLS_NEEDED[@]}"; do
            echo "   - $tool"
        done
        echo
        
        if [[ "$AUTO_YES" != "true" ]]; then
            echo -n "Proceed with installation? [Y/n] "
            read -r response
            if [[ "$response" =~ ^[Nn] ]]; then
                echo "Installation cancelled."
                exit 0
            fi
        fi
        
        # Install tools
        print_section "📦 Installing Tools"
        
        # Ensure package manager on macOS
        if [[ "$OS" == "macos" ]]; then
            install_homebrew
        fi
        
        # Install each tool
        install_git
        install_rust
        install_docker
        install_go
        install_dagger
        
        echo
    else
        print_success "All required tools are installed!"
    fi
    
    # Install Patina
    install_patina
    
    # Update shell configuration
    update_shell_config
    
    # Create design TOML if requested
    create_design_toml
    
    # Summary
    print_section "\n🎯 Next Steps"
    echo "   1. Restart your shell or run: source $SHELL_RC"
    echo "   2. Verify installation: patina --version"
    
    if [[ -f "PROJECT_DESIGN.toml" ]]; then
        echo "   3. Initialize project: patina init <name> --llm=claude --design=PROJECT_DESIGN.toml"
    else
        echo "   3. Create a PROJECT_DESIGN.toml and initialize your project"
    fi
    
    echo -e "\n${BOLD}${GREEN}Happy coding! 🚀${NC}"
}

# Run main
main