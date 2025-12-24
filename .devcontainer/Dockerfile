# YOLO Development Container
# Autonomous AI workspace with all detected toolchains

FROM mcr.microsoft.com/devcontainers/base:ubuntu

# Avoid prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install Node.js for Claude Code CLI
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g npm@latest \
    && npm install -g pnpm@latest

# Install Claude Code CLI for autonomous AI work
RUN npm install -g @anthropic-ai/claude-code@latest \
    && mkdir -p /root/.claude-linux

# Configure Claude settings using official settings.json API
# YOLO mode with extended timeouts (1 hour for long builds)
RUN echo '{"permissions":{"defaultMode":"bypassPermissions","allow":[],"deny":[]},"env":{"BASH_DEFAULT_TIMEOUT_MS":3600000,"BASH_MAX_TIMEOUT_MS":3600000}}' > /root/.claude-linux/settings.json

# Set Claude config directory for bind-mounted credentials
RUN echo 'export CLAUDE_CONFIG_DIR=/root/.claude-linux' >> /etc/bash.bashrc

# Install 1Password CLI for secure credential management
RUN curl -sS https://downloads.1password.com/linux/keys/1password.asc | \
    gpg --dearmor --output /usr/share/keyrings/1password-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/1password-archive-keyring.gpg] https://downloads.1password.com/linux/debian/$(dpkg --print-architecture) stable main" | \
    tee /etc/apt/sources.list.d/1password.list && \
    mkdir -p /etc/debsig/policies/AC2D62742012EA22/ && \
    curl -sS https://downloads.1password.com/linux/debian/debsig/1password.pol | \
    tee /etc/debsig/policies/AC2D62742012EA22/1password.pol && \
    mkdir -p /usr/share/debsig/keyrings/AC2D62742012EA22 && \
    curl -sS https://downloads.1password.com/linux/keys/1password.asc | \
    gpg --dearmor --output /usr/share/debsig/keyrings/AC2D62742012EA22/debsig.gpg && \
    apt-get update && apt-get install -y 1password-cli


# Create workspace directory
WORKDIR /workspace

# YOLO Mode indicator
ENV PATINA_YOLO=1

# Set up shell
CMD ["/bin/bash"]
