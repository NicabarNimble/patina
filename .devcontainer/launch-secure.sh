#!/bin/bash
# 1Password Secure Launcher
# Fetches credentials from 1Password on HOST and injects into container

set -e

echo "🔐 Fetching credentials from 1Password..."

# Fetch credential from 1Password on the host (uses biometric auth)
CRED=$(op document get "Patina Claude Max Subscription" --vault Private 2>/dev/null)

if [ $? -ne 0 ]; then
    echo "❌ Failed to fetch credentials from 1Password"
    echo "   Make sure you're authenticated: op signin"
    exit 1
fi

# Encode credential to pass safely through environment
CRED_B64=$(echo "$CRED" | base64)

# Launch container with credential as environment variable
echo "🚀 Launching container with secure credentials..."
docker compose -f .devcontainer/docker-compose.yml up -d --build

# Inject credential into container's tmpfs
docker exec devcontainer-workspace-1 bash -c "
    echo '$CRED_B64' | base64 -d > /root/.claude-linux/.credentials.json
    chmod 600 /root/.claude-linux/.credentials.json
    echo '✅ Credentials injected into RAM-only storage'
"

echo ""
echo "✅ Container ready with secure credentials!"
echo "   Connect: docker exec -it devcontainer-workspace-1 bash"
echo "   🔒 Credentials in tmpfs (RAM-only)"
echo ""
