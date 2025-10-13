# Patina YOLO Development Container
# Isolated Linux environment for safe LLM experimentation

FROM ubuntu:24.04

# Avoid prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install base utilities
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    vim \
    ca-certificates \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

{{RUST_SETUP}}

{{NODE_SETUP}}

{{PYTHON_SETUP}}

{{GO_SETUP}}

# Create workspace directory
WORKDIR /workspace

# Set up shell
CMD ["/bin/bash"]
