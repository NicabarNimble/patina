#!/bin/bash
# YOLO Workspace Setup Script

echo "🎯 Setting up YOLO workspace..."

# Ensure git is configured
if [ -z "$(git config --global user.email)" ]; then
    git config --global user.email "ai@patina.dev"
    git config --global user.name "AI Assistant"
fi

# Create Claude config directory
mkdir -p ~/.claude-linux

# Configure Claude settings using official settings.json API
# This ensures settings are up-to-date even if Dockerfile settings were cached
echo "🔧 Configuring Claude settings..."
cat > ~/.claude-linux/settings.json <<'EOF'
{
  "permissions": {
    "defaultMode": "bypassPermissions",
    "allow": [],
    "deny": []
  },
  "env": {
    "BASH_DEFAULT_TIMEOUT_MS": 3600000,
    "BASH_MAX_TIMEOUT_MS": 3600000
  }
}
EOF

echo "✅ Claude configured with:"
echo "  - Permissions: YOLO mode (bypassed)"
echo "  - Bash timeout: 1 hour (3600000ms)"

# Set up shell aliases for YOLO mode
cat >> ~/.bashrc <<'EOF'
alias yolo='echo "YOLO mode active - permissions bypassed"'
alias status='git status'
alias commit='git add -A && git commit -m'
EOF

# Install additional tools if needed
if command -v npm &> /dev/null; then
    echo "📦 Installing global npm packages..."
    npm install -g typescript ts-node 2>/dev/null || true
fi

# Check Claude authentication
echo ""
echo "🤖 Checking Claude Code authentication..."

# If 1Password CLI is available, fetch credentials from vault
if [ "${PATINA_USE_1PASSWORD}" = "1" ]; then
    echo "🔐 Fetching credentials from 1Password vault..."

    # Check if op CLI is available
    if command -v op &> /dev/null; then
        # Fetch credential from 1Password and save to tmpfs
        if op document get "Patina Claude Max Subscription" --vault Private > ~/.claude-linux/.credentials.json 2>/dev/null; then
            chmod 600 ~/.claude-linux/.credentials.json
            echo "✅ Claude authenticated with Max subscription (from 1Password)"
            echo "   🔒 Credentials in RAM-only storage (tmpfs)"
            echo "   🔒 Credentials never touch disk"
        else
            echo "⚠️  Failed to fetch credentials from 1Password"
            echo ""
            echo "To fix:"
            echo "  1. Ensure you're signed in: op signin"
            echo "  2. Store credential: op document create ~/.patina/claude-linux/.credentials.json --title 'Patina Claude Max Subscription'"
            echo ""
        fi
    else
        echo "⚠️  1Password CLI not available in container"
        echo "   Install op CLI: https://developer.1password.com/docs/cli/get-started/"
    fi
elif [ -f ~/.claude-linux/.credentials.json ]; then
    echo "✅ Claude already authenticated with Max subscription"
    echo "   Credentials shared from ~/.patina/claude-linux/"
else
    echo "⚠️  Claude not authenticated yet"
    echo ""
    echo "To enable autonomous AI work with Max subscription:"
    echo "  1. On your HOST machine (Mac), run: claude login"
    echo "  2. Move credentials: mv ~/.claude/.credentials.json ~/.patina/claude-linux/"
    echo "  3. Credentials will work in ALL patina containers"
    echo ""
    echo "Or use 1Password for secure credential storage:"
    echo "  1. Install op CLI: brew install --cask 1password-cli"
    echo "  2. Store credential: op document create ~/.patina/claude-linux/.credentials.json --title 'Patina Claude Max Subscription'"
    echo "  3. Regenerate devcontainer: patina yolo"
    echo ""
fi

echo "✅ YOLO workspace ready!"
echo ""
echo "💭 Available Commands:"
echo "  • claude 'task' - Autonomous AI assistant (Max subscription shared)"
echo "  • Language tools based on detected stack"
echo "  • git, npm, node - Standard development tools"
echo ""
