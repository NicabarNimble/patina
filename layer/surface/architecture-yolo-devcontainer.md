---
id: architecture-yolo-devcontainer
status: active
created: 2025-10-07
updated: 2025-12-09
oxidizer: nicabar
tags: [architecture, yolo, devcontainer, buildpack-pattern, modular-system, detection]
references: [architecture-patina-system]
---

# YOLO Devcontainer Architecture - Buildpack-Style Generation System

**Core Concept**: A modular detection and generation system that scans repositories and creates devcontainer environments optimized for AI assistants. Follows Cloud Native Buildpacks' design philosophy: isolated "packs" that detect, declare, and build container layers.

---

## System Overview

The YOLO command implements a **4-phase pipeline** that transforms repository analysis into executable devcontainer configurations:

```
Repository → Scanner → Profile → FeatureMapper → Generator → DevContainer Files
   (input)   (detect)  (model)    (transform)    (build)       (output)
```

Each phase has a single responsibility and communicates through well-defined data structures.

---

## Architecture Phases

### Phase 1: Scanner (Detection)

**Location**: `src/commands/yolo/scanner.rs`

**Purpose**: Scans repository to identify languages, tools, frameworks, and services. Acts like buildpack `detect` scripts.

#### Detection Strategy (4 Layers)

1. **`scan_manifests()`** - Primary detection via manifest files
   - `package.json` → JavaScript/Node
   - `Cargo.toml` → Rust
   - `go.mod` → Go
   - `requirements.txt` / `pyproject.toml` → Python
   - Detects package managers: `pnpm-lock.yaml` → pnpm, `yarn.lock` → yarn

2. **`scan_configs()`** - Framework-specific configuration files
   - `foundry.toml` → Foundry + Solidity
   - `hardhat.config.*` → Hardhat
   - `mud.config.ts` → MUD Framework
   - `tsconfig.json` → TypeScript

3. **`scan_source_files()`** - Fallback detection via file extensions
   - Counts `.sol`, `.cairo`, `.js`, `.ts` files using glob patterns
   - Updates file counts for detected languages
   - Provides confidence metrics

4. **`apply_smart_inference()`** - Dependency and service inference
   - If Foundry detected → add Anvil service (local blockchain on port 8545)
   - If MUD detected → add Indexer service
   - **Key insight**: Some tools imply required services

#### Key Methods

- `count_files_with_extension()` - Uses glob patterns to count files
- `read_*_version()` - Extracts version info from `.nvmrc`, `.python-version`, `rust-toolchain.toml`
- `extract_*_version()` - Parses version constraints from config files (TODO: many unimplemented)

#### Output

Returns a `RepoProfile` containing:
- Detected languages with versions and file counts
- Detected tools with versions
- Required services with images and ports
- Detection provenance (what triggered each detection)

#### Design Pattern: Cascading Detection

Uses **layered detection** to prevent false positives:

1. Manifest files (authoritative)
2. Config files (framework-specific)
3. Source files (fallback)
4. Smart inference (dependencies)

Each layer adds confidence and context to detections.

---

### Phase 2: Profile (Data Model)

**Location**: `src/commands/yolo/profile.rs`

**Purpose**: Structured representation of detected components. Acts like CNB's "build plan" - the normalized model that bridges detection and generation.

#### Data Structures

```rust
RepoProfile {
    languages: HashMap<Language, LanguageInfo>,  // Detected languages
    tools: HashMap<Tool, ToolInfo>,              // Package managers, frameworks
    services: Vec<Service>,                      // Docker services
    project_name: Option<String>
}

LanguageInfo {
    detected_by: Vec<String>,  // What triggered detection
    version: Option<String>,   // Desired version
    file_count: usize,         // Confidence metric
}

ToolInfo {
    detected_by: Vec<String>,
    version: Option<String>,
}

Service {
    name: String,              // "anvil", "indexer"
    image: Option<String>,     // Docker image
    ports: Vec<u16>,           // Exposed ports
}
```

#### Supported Types

**Languages**: Rust, Go, Python, JavaScript, TypeScript, Solidity, Cairo, C, C++

**Tools**:
- Package Managers: Npm, Yarn, Pnpm, Cargo, Poetry, Pip
- Blockchain: Foundry, Hardhat, Truffle, MudFramework, Scarb
- Dev Tools: Git, Docker, DockerCompose

**Services**: Dynamically inferred based on tools (Anvil, Indexer, etc.)

#### Profile Operations

- `add_language()` - Register detected language
- `add_tool()` - Register detected tool
- `add_service()` - Add required service
- `add_tool_override()` - Handle `--with` flag overrides
- `exclude_tool()` - Handle `--without` flag overrides

#### Key Insight: Detection Provenance

Each detected item tracks **what triggered detection**:
- `["package.json"]` - Found via manifest
- `["42 .sol files"]` - Found via source scan
- `["--with flag"]` - User override
- `["foundry.toml"]` - Framework inference

This provides transparency and debugging capability.

---

### Phase 3: FeatureMapper (Transformation)

**Location**: `src/commands/yolo/features.rs`

**Purpose**: Converts `RepoProfile` → `Vec<DevContainerFeature>`. Declares what each "buildpack" contributes to the container.

#### The Mapping Logic

`map_profile()` transforms profile components into features:

**Languages → Features**:
```rust
JavaScript/TypeScript → DevContainerFeature::Node { version }
Rust                  → DevContainerFeature::Rust { version }
Python                → DevContainerFeature::Python { version }
Go                    → DevContainerFeature::Go { version }
Solidity              → DevContainerFeature::Solc { version } (only if no Foundry)
Cairo                 → DevContainerFeature::Cairo { version }
```

**Tools → Features**:
```rust
Pnpm         → DevContainerFeature::Pnpm { version }
Yarn         → DevContainerFeature::Yarn { version }
Foundry      → DevContainerFeature::Foundry { version }
Hardhat      → DevContainerFeature::Hardhat
MudFramework → DevContainerFeature::MudCli
Scarb        → DevContainerFeature::Scarb { version }
Poetry       → DevContainerFeature::Poetry
```

**Always Added**: `Git`, `GitHubCli` (essential for development)

#### DevContainerFeature Enum

Each variant represents a **modular "buildpack"** - an isolated unit that contributes something specific:

```rust
pub enum DevContainerFeature {
    // Official features (ghcr.io/devcontainers/features/*)
    Node { version: Option<String> },
    Python { version: String },
    Rust { version: String },
    Go { version: String },
    Pnpm { version: Option<String> },
    Yarn { version: Option<String> },
    Git,
    GitHubCli,
    DockerInDocker,

    // Custom Patina features (ghcr.io/patina/features/*)
    // These require custom Dockerfile installation
    Foundry { version: String },
    Cairo { version: String },
    Scarb { version: String },
    Solc { version: String },
    Hardhat,
    MudCli,
    Poetry,
}
```

#### The `to_feature_spec()` Method

Converts each feature to DevContainer JSON format (feature ID + configuration):

```rust
DevContainerFeature::Node { version } →
  ("ghcr.io/devcontainers/features/node:1", {"version": "20"})

DevContainerFeature::Foundry { version } →
  ("ghcr.io/patina/features/foundry:1", {"version": "latest"})
```

This method is the **contract** between features and the generator - it declares:
1. Where to find the feature (registry URL)
2. How to configure it (JSON spec)

#### Official vs Custom Features

**Official DevContainer Features**:
- Hosted at `ghcr.io/devcontainers/features/*`
- Maintained by Microsoft/community
- Just referenced in `devcontainer.json`
- Examples: Node, Python, Rust, Go, Git

**Custom Patina Features**:
- Target: `ghcr.io/patina/features/*` (not yet published)
- Require custom Dockerfile installation scripts
- For tools without official features
- Examples: Foundry, Cairo, Scarb, Solc

---

### Phase 4: Generator (Build)

**Location**: `src/commands/yolo/generator.rs`

**Purpose**: Takes `RepoProfile` + `Vec<DevContainerFeature>` and generates actual files. Like buildpack `build` scripts.

#### Main Flow

```rust
pub fn generate(&self, profile: &RepoProfile, features: &[DevContainerFeature]) -> Result<()> {
    1. fs::create_dir_all(".devcontainer")
    2. generate_devcontainer_json()  // Main config
    3. generate_dockerfile()         // If custom features needed
    4. generate_docker_compose()     // Always (for CLI usage)
    5. generate_yolo_setup()         // Post-create script
}
```

#### Key Generation Methods

##### 1. `generate_devcontainer_json()` (lines 45-128)

Creates the main `devcontainer.json` configuration:

**Features Object**:
```rust
let mut features_obj = serde_json::Map::new();
for feature in features {
    let (id, spec) = feature.to_feature_spec();
    features_obj.insert(id, spec);
}
```

**Mounts**:
- `layer:/workspace/layer` - Pattern storage
- `~/.patina/credentials:/root/.credentials:ro` - API credentials

**Environment Variables**:
- `PATINA_YOLO=1` - YOLO mode indicator
- `SKIP_PERMISSIONS=1` - Bypass permission checks
- `AI_WORKSPACE=1` - AI assistant marker
- `IS_SANDBOX=1` - Isolated environment marker

**VSCode Customizations**:
- Extensions based on features (rust-analyzer, ms-python.python, golang.go, etc.)
- Terminal defaults (bash)

**Port Forwarding**:
- Common dev ports: 3000, 8000, 8080
- Service-specific ports from `profile.services`

**Build Configuration**:
- If `needs_custom_dockerfile()` → reference Dockerfile
- Otherwise → use base image `mcr.microsoft.com/devcontainers/base:ubuntu`

**Docker Compose Integration**:
- If services detected → reference `docker-compose.yml`
- Set service name to `workspace`
- Set shutdown action to `stopCompose`

**Post-Create Hook**:
- Execute `yolo-setup.sh` after container creation

##### 2. `generate_dockerfile()` (lines 130-205)

**Only generated if** `needs_custom_dockerfile()` returns true.

**Custom features requiring Dockerfile**:
- `DevContainerFeature::Foundry` → `get_foundry_install()`
- `DevContainerFeature::Cairo` → `get_cairo_install()`
- `DevContainerFeature::Scarb` → `get_scarb_install()`

**Base Setup (Always Included)**:

```dockerfile
FROM mcr.microsoft.com/devcontainers/base:ubuntu

# Install Node.js 20 for Claude Code CLI
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g npm@latest pnpm@latest

# Install Claude Code CLI
RUN npm install -g @anthropic-ai/claude-code@latest \
    && mkdir -p /root/.claude-linux

# Create Claude wrapper with YOLO permissions bypass
RUN echo '#!/bin/bash' > /usr/local/bin/claude \
    && echo 'export CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS=true' >> ... \
    && chmod +x /usr/local/bin/claude
```

**Custom Feature Installation Scripts**:

Each custom feature adds a RUN command:

```dockerfile
# Install Foundry
RUN curl -L https://foundry.paradigm.xyz | bash && \
    /root/.foundry/bin/foundryup && \
    echo 'export PATH="/root/.foundry/bin:$PATH"' >> /etc/bash.bashrc
ENV PATH="/root/.foundry/bin:$PATH"
```

**Key Insight**: Each custom feature is a **separate layer** in the Docker image, following buildpack philosophy.

##### 3. `generate_docker_compose()` (lines 207-295)

**Always generated** (even without services) for consistent CLI workflow.

**Workspace Service**:
```yaml
workspace:
  build:                    # Or 'image' if no custom Dockerfile
    context: .
    dockerfile: Dockerfile
  volumes:
    - ..:/workspace:cached
    - ~/.patina/claude-linux:/root/.claude-linux:cached
    - ~/.claude:/root/.claude-macos:ro
  working_dir: /workspace
  command: sleep infinity
  ports:
    - "3000:3000"
    - "3001:3001"
    - "8000:8000"
    - "8080:8080"
    - "8545:8545"  # Anvil/blockchain
  environment:
    PATINA_YOLO: "1"
    SKIP_PERMISSIONS: "1"
    AI_WORKSPACE: "1"
    IS_SANDBOX: "1"
    CLAUDE_CONFIG_DIR: "/root/.claude-linux"
    CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS: "true"
```

**Detected Services**:

Uses `create_service_config()` to generate service-specific configs:

```yaml
# Anvil (if Foundry detected)
anvil:
  image: ghcr.io/foundry-rs/foundry:latest
  command: anvil --host 0.0.0.0
  ports:
    - "8545:8545"

# Indexer (if MUD detected)
indexer:
  image: ghcr.io/latticexyz/store-indexer:latest
  environment:
    RPC_HTTP_URL: http://anvil:8545
  depends_on:
    - anvil
```

**Key Design Decision**: Always generate docker-compose even without services
- Provides consistent CLI workflow
- Easier credential management via volume mounts
- Can add services later without changing structure
- Supports both VS Code and CLI usage

##### 4. `generate_yolo_setup()` (lines 297-362)

Post-create script (`yolo-setup.sh`) that runs inside container:

**Git Configuration**:
```bash
if [ -z "$(git config --global user.email)" ]; then
    git config --global user.email "ai@patina.dev"
    git config --global user.name "AI Assistant"
fi
```

**Directory Setup**:
```bash
mkdir -p ~/.credentials
mkdir -p ~/.claude-linux
```

**Shell Aliases**:
```bash
alias yolo='echo "YOLO mode active - permissions bypassed"'
alias status='git status'
alias commit='git add -A && git commit -m'
```

**Global Package Installation**:
```bash
if command -v npm &> /dev/null; then
    npm install -g typescript ts-node 2>/dev/null || true
fi
```

**Authentication Check**:
```bash
if [ -f ~/.claude-linux/.credentials.json ]; then
    echo "✅ Claude already authenticated"
else
    echo "⚠️  Claude not authenticated yet"
    echo "Run: claude login"
fi
```

**Helpful Guidance**:
- Display available commands
- Explain authentication flow
- Show how to use autonomous AI features

#### Helper Methods

**`needs_custom_dockerfile()`** - Determines if Dockerfile needed:
```rust
features.iter().any(|f| matches!(f,
    DevContainerFeature::Foundry { .. } |
    DevContainerFeature::Cairo { .. } |
    DevContainerFeature::Scarb { .. }
))
```

**`get_vscode_extensions()`** - Maps features to VSCode extensions:
```rust
DevContainerFeature::Rust { .. } → "rust-lang.rust-analyzer"
DevContainerFeature::Python { .. } → "ms-python.python"
DevContainerFeature::Go { .. } → "golang.go"
DevContainerFeature::Solc/Foundry { .. } → "JuanBlanco.solidity"
```

**`get_forward_ports()`** - Collects ports to forward:
```rust
let mut ports = vec![3000, 8000, 8080]; // Common dev ports
for service in &profile.services {
    ports.extend(&service.ports);
}
ports.sort_unstable();
ports.dedup();
```

**`create_service_config()`** - Generates service-specific docker-compose config:
```rust
match service.name.as_str() {
    "anvil" => /* foundry image with anvil command */,
    "indexer" => /* store-indexer with RPC config */,
    _ => /* generic config from service.image */
}
```

---

## Main Entry Point

**Location**: `src/commands/yolo/mod.rs`

**The Complete Flow**:

```rust
pub fn execute(
    interactive: bool,
    defaults: bool,
    with: Option<Vec<String>>,
    without: Option<Vec<String>>,
    json: bool,
) -> Result<()> {
    println!("🎯 YOLO Mode: Scanning for autonomous workspace setup...");

    let work_dir = std::env::current_dir()?;

    // Phase 1: DETECT
    let scanner = Scanner::new(&work_dir);
    let mut profile = scanner.scan()?;

    // Phase 1.5: OVERRIDE (user preferences)
    if let Some(with_tools) = with {
        for tool in with_tools {
            profile.add_tool_override(&tool);
        }
    }
    if let Some(without_tools) = without {
        for tool in without_tools {
            profile.exclude_tool(&tool);
        }
    }

    // Display detection results
    if !json {
        display_detection_results(&profile);
    }

    // Phase 2: INTERACTIVE (if requested)
    if interactive && !defaults {
        profile = run_interactive_mode(profile)?; // TODO: Not yet implemented
    }

    // Phase 3: TRANSFORM
    let mapper = FeatureMapper::new();
    let features = mapper.map_profile(&profile)?;

    // Phase 4: BUILD
    let generator = Generator::new(&work_dir);
    generator.generate(&profile, &features)?;

    // Output results
    if json {
        output_json_results(&profile, &features)?;
    } else {
        display_success_message(&work_dir);
    }

    Ok(())
}
```

---

## The Buildpack Analogy

### Mapping to Cloud Native Buildpacks

| **YOLO System** | **CNB Equivalent** | **Purpose** |
|-----------------|-------------------|-------------|
| Scanner | `bin/detect` | Scan repo, decide what's needed |
| RepoProfile | Build Plan | Normalized model of requirements |
| FeatureMapper | `bin/provide` | Declare what each "pack" contributes |
| DevContainerFeature | Buildpack | Modular unit (e.g., "node", "foundry") |
| Generator | `bin/build` | Actually install/configure components |
| `to_feature_spec()` | Layer metadata | Map feature → container layer |
| `needs_custom_dockerfile()` | Custom buildpacks | Handle non-standard features |
| Official features | Standard buildpacks | Community-maintained, cached |
| Custom features | Custom buildpacks | Project-specific, inline install |

### Key Buildpack Principles Applied

1. **Detection Separation**: What you need vs how to provide it
   - Scanner detects requirements (what)
   - Generator provides implementations (how)

2. **Modular Contributions**: Each buildpack is isolated
   - Each `DevContainerFeature` is self-contained
   - Features don't know about each other
   - Composition happens in Generator

3. **Official vs Custom**: Prefer official, support custom
   - Use official DevContainer features when available
   - Fall back to custom Dockerfile for edge cases
   - Clear distinction via `needs_custom_dockerfile()`

4. **Layer Optimization**: Separate concerns into layers
   - Base image (Ubuntu)
   - Node.js + Claude (always)
   - Custom features (conditional)
   - Each RUN command is a cacheable layer

5. **Metadata-Driven**: Declare, don't execute
   - `to_feature_spec()` declares requirements
   - Generator executes based on declarations
   - Separation enables testing and introspection

---

## Extension Points: Adding New "Packs"

### Example: Adding Elixir Support

#### Step 1: Add to Profile Types (`profile.rs`)

```rust
pub enum Language {
    // ... existing
    Elixir,
}

pub enum Tool {
    // ... existing
    Mix,  // Elixir package manager
}

impl Language {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            // ... existing
            "elixir" | "ex" => Some(Language::Elixir),
            _ => None,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ... existing
            Language::Elixir => write!(f, "Elixir"),
        }
    }
}
```

#### Step 2: Add Detection Logic (`scanner.rs`)

```rust
fn scan_manifests(&self, profile: &mut RepoProfile) -> Result<()> {
    // ... existing detections

    // Elixir
    if self.root_path.join("mix.exs").exists() {
        let detected_by = vec!["mix.exs".to_string()];
        let version = self.read_elixir_version()?;

        profile.add_language(Language::Elixir, LanguageInfo {
            detected_by,
            version,
            file_count: 0,
        });

        // Mix is the default tool
        profile.add_tool(Tool::Mix, ToolInfo {
            detected_by: vec!["mix.exs".to_string()],
            version: None,
        });
    }

    Ok(())
}

fn scan_source_files(&self, profile: &mut RepoProfile) -> Result<()> {
    // ... existing scans

    // Count Elixir files
    let ex_files = self.count_files_with_extension("ex")?
        + self.count_files_with_extension("exs")?;
    if ex_files > 0 && !profile.languages.contains_key(&Language::Elixir) {
        profile.add_language(Language::Elixir, LanguageInfo {
            detected_by: vec![format!("{} .ex/.exs files", ex_files)],
            version: None,
            file_count: ex_files,
        });
    }

    Ok(())
}

fn read_elixir_version(&self) -> Result<Option<String>> {
    let version_path = self.root_path.join(".tool-versions");
    if version_path.exists() {
        let content = std::fs::read_to_string(version_path)?;
        // Parse .tool-versions for elixir line
        for line in content.lines() {
            if line.starts_with("elixir ") {
                return Ok(Some(line.strip_prefix("elixir ")?.trim().to_string()));
            }
        }
    }
    Ok(None)
}
```

#### Step 3: Add Feature Mapping (`features.rs`)

```rust
pub enum DevContainerFeature {
    // ... existing
    Elixir { version: String },
}

impl FeatureMapper {
    pub fn map_profile(&self, profile: &RepoProfile) -> Result<Vec<DevContainerFeature>> {
        let mut features = Vec::new();

        for (lang, info) in &profile.languages {
            match lang {
                // ... existing mappings
                Language::Elixir => {
                    features.push(DevContainerFeature::Elixir {
                        version: info.version.clone().unwrap_or_else(|| "1.17".to_string()),
                    });
                }
                _ => {}
            }
        }

        // ... rest of mapping
        Ok(features)
    }
}

impl DevContainerFeature {
    pub fn to_feature_spec(&self) -> (String, serde_json::Value) {
        match self {
            // ... existing specs

            DevContainerFeature::Elixir { version } => {
                let spec = serde_json::json!({ "version": version });

                // Check if official feature exists, otherwise use custom
                // For this example, assume we need custom installation
                ("ghcr.io/patina/features/elixir:1".to_string(), spec)
            }
        }
    }
}
```

#### Step 4: Add Custom Installation (`generator.rs`)

**Only needed if no official DevContainer feature exists.**

```rust
impl Generator {
    fn needs_custom_dockerfile(&self, features: &[DevContainerFeature]) -> bool {
        features.iter().any(|f| matches!(f,
            DevContainerFeature::Foundry { .. } |
            DevContainerFeature::Cairo { .. } |
            DevContainerFeature::Scarb { .. } |
            DevContainerFeature::Elixir { .. }  // ← Add here
        ))
    }

    fn generate_dockerfile(
        &self,
        devcontainer_path: &Path,
        _profile: &RepoProfile,
        features: &[DevContainerFeature],
    ) -> Result<()> {
        // ... base dockerfile setup

        for feature in features {
            match feature {
                // ... existing custom features

                DevContainerFeature::Elixir { .. } => {
                    dockerfile.push_str(&self.get_elixir_install());
                }
                _ => {}
            }
        }

        // ... rest of dockerfile
        Ok(())
    }

    fn get_elixir_install(&self) -> String {
        r#"
# Install Erlang and Elixir
RUN apt-get update && apt-get install -y wget gnupg && \
    wget https://packages.erlang-solutions.com/erlang-solutions_2.0_all.deb && \
    dpkg -i erlang-solutions_2.0_all.deb && \
    apt-get update && \
    apt-get install -y esl-erlang elixir && \
    mix local.hex --force && \
    mix local.rebar --force && \
    rm erlang-solutions_2.0_all.deb

# Add Mix to PATH
ENV PATH="/root/.mix:$PATH"

"#.to_string()
    }
}
```

#### Step 5: Add VSCode Extensions (`generator.rs`)

```rust
fn get_vscode_extensions(&self, features: &[DevContainerFeature]) -> Vec<String> {
    let mut extensions = vec![];

    for feature in features {
        match feature {
            // ... existing mappings

            DevContainerFeature::Elixir { .. } => {
                extensions.push("jakebecker.elixir-ls".to_string());
            }
            _ => {}
        }
    }

    extensions
}
```

### Extension Checklist

When adding support for a new language/framework:

- [ ] Add enum variant to `Language` or `Tool` in `profile.rs`
- [ ] Add `from_string()` case for CLI parsing
- [ ] Add `Display` implementation for pretty printing
- [ ] Add detection logic in `scanner.rs`:
  - [ ] Manifest detection (`scan_manifests()`)
  - [ ] Config detection (`scan_configs()`)
  - [ ] Source file detection (`scan_source_files()`)
  - [ ] Version extraction helper method
- [ ] Add feature mapping in `features.rs`:
  - [ ] Add `DevContainerFeature` enum variant
  - [ ] Add mapping case in `map_profile()`
  - [ ] Add `to_feature_spec()` case with registry URL
- [ ] If no official feature exists, add custom installation in `generator.rs`:
  - [ ] Add to `needs_custom_dockerfile()` check
  - [ ] Add case in `generate_dockerfile()` loop
  - [ ] Create `get_*_install()` helper with Dockerfile RUN commands
- [ ] (Optional) Add VSCode extension in `get_vscode_extensions()`
- [ ] (Optional) Add smart inference rules in `apply_smart_inference()`

---

## Key Design Patterns

### 1. Isolated "Blobs" (Buildpack Pattern)

Each `DevContainerFeature` is self-contained:
- **Detection** is separate from **installation**
- **Version** is captured during detection, passed through profile
- **Installation** is delegated to official features OR custom Dockerfile scripts
- Features don't know about each other
- Composition happens at generation time

**Benefits**:
- Easy to add new features without modifying existing ones
- Clear ownership and responsibility
- Testable in isolation

### 2. Layered Detection (Confidence Building)

Scanner uses **cascading detection** with increasing specificity:

1. **Manifest files** (highest confidence) - `package.json`, `Cargo.toml`
2. **Config files** (framework-specific) - `foundry.toml`, `tsconfig.json`
3. **Source files** (fallback) - Glob patterns, file counting
4. **Smart inference** (derived) - If X detected, also need Y

Each layer adds context and builds confidence. Prevents false positives.

**Example**: TypeScript detection
1. Found `tsconfig.json` (authoritative) → TypeScript
2. Count `.ts` files → Update file_count for confidence
3. If also found `package.json` → Node feature added

### 3. Official vs Custom Features (Optimization)

**Official features** (`ghcr.io/devcontainers/features/*`):
- Just referenced in `devcontainer.json`
- No Dockerfile needed
- Faster builds (cached layers from registry)
- Better maintained (community/Microsoft)
- Examples: Node, Python, Rust, Go, Git

**Custom features** (`ghcr.io/patina/features/*`):
- Need Dockerfile installation scripts
- Not yet published to registry (future work)
- Inline installation in Dockerfile
- For niche/emerging tools
- Examples: Foundry, Cairo, Scarb

**Decision Point**: `needs_custom_dockerfile()` determines which path

**Future**: Publish custom features to registry to enable same optimization

### 4. Service Orchestration (Dependency Inference)

Services are:
- **Detected** via inference in `apply_smart_inference()`
- **Added** to `RepoProfile.services`
- **Generated** as separate docker-compose services
- **Linked** automatically (Indexer depends_on Anvil)

**Key insight**: Some tools imply required services
- Foundry → Anvil (local blockchain)
- MUD → Indexer (blockchain data)
- Django → PostgreSQL (future)
- Rails → Redis (future)

This reduces configuration burden - system knows what you need.

### 5. Git Worktree Isolation (Credential Management)

**Problem**: Claude Code needs authentication, but container rebuilds lose state.

**Solution**: Volume mount credentials from host
- `~/.patina/claude-linux:/root/.claude-linux:cached` - Container config
- `~/.claude:/root/.claude-macos:ro` - Reference to host config
- OAuth tokens persist across container rebuilds
- Isolation: Container uses separate config directory

**Trade-off**: Convenience vs isolation (chosen: convenience)

### 6. YOLO Mode Philosophy (Permissions Bypass)

Environment variables enable autonomous AI operation:
- `PATINA_YOLO=1` - YOLO mode indicator
- `SKIP_PERMISSIONS=1` - Bypass permission checks
- `AI_WORKSPACE=1` - AI assistant marker
- `IS_SANDBOX=1` - Isolated environment (safe to experiment)
- `CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS=true` - Claude-specific bypass

Claude wrapper script includes `--dangerously-skip-permissions` flag.

**Philosophy**: Inside container, AI has full autonomy. Container isolation provides safety boundary.

**Trade-off**: Security vs AI autonomy (chosen: autonomy within sandbox)

---

## Example Walkthrough: Foundry Project

Let's trace execution for a Foundry smart contract project:

### Input Repository Structure
```
my-contracts/
├── foundry.toml
├── src/
│   ├── Counter.sol
│   ├── NFT.sol
│   └── Token.sol
├── test/
│   ├── Counter.t.sol
│   └── NFT.t.sol
├── script/
│   └── Deploy.s.sol
└── lib/
    └── forge-std/
```

### Phase 1: Scanner Detection

**`scan_manifests()`**:
- No detection (no package.json, Cargo.toml, etc.)

**`scan_configs()`**:
- Found `foundry.toml`
  - Add `Tool::Foundry` with `detected_by: ["foundry.toml"]`
  - Infer `Language::Solidity` with `detected_by: ["foundry.toml"]`
  - Try to extract Foundry version from config (currently TODO)
  - Try to extract Solc version from config (currently TODO)

**`scan_source_files()`**:
- Glob `**/*.sol` → finds 5 files (Counter, NFT, Token, 2 tests)
- Solidity already detected, update `file_count: 5`

**`apply_smart_inference()`**:
- Foundry detected → add Anvil service
  ```rust
  Service {
      name: "anvil",
      image: Some("ghcr.io/foundry-rs/foundry:latest"),
      ports: vec![8545]
  }
  ```

**Result**:
```rust
RepoProfile {
    languages: {
        Solidity: LanguageInfo {
            detected_by: ["foundry.toml"],
            version: Some("0.8.30"),  // From foundry.toml parsing (TODO)
            file_count: 5
        }
    },
    tools: {
        Foundry: ToolInfo {
            detected_by: ["foundry.toml"],
            version: Some("latest")
        }
    },
    services: [
        Service {
            name: "anvil",
            image: Some("ghcr.io/foundry-rs/foundry:latest"),
            ports: [8545]
        }
    ],
    project_name: Some("my-contracts")
}
```

### Phase 2: Profile (No transformation needed)

Profile passed directly to mapper.

### Phase 3: FeatureMapper Transformation

**`map_profile()`**:

For each language:
```rust
Language::Solidity => {
    // Skip if Foundry detected (Foundry includes Solc)
    if !profile.tools.contains_key(&Tool::Foundry) {
        features.push(DevContainerFeature::Solc { version: "0.8.30" });
    }
}
```
**Skipped** because Foundry detected.

For each tool:
```rust
Tool::Foundry => {
    features.push(DevContainerFeature::Foundry {
        version: "latest"
    });
}
```

Always add:
```rust
features.push(DevContainerFeature::Git);
features.push(DevContainerFeature::GitHubCli);
```

**Result**:
```rust
vec![
    DevContainerFeature::Foundry { version: "latest" },
    DevContainerFeature::Git,
    DevContainerFeature::GitHubCli,
]
```

### Phase 4: Generator Build

**`generate()` flow**:

1. **Create directory**: `.devcontainer/`

2. **Check custom Dockerfile need**:
   ```rust
   needs_custom_dockerfile([Foundry, Git, GitHubCli]) → true
   // Foundry matches custom feature pattern
   ```

3. **`generate_devcontainer_json()`**:
   ```json
   {
     "name": "my-contracts - YOLO Workspace",
     "features": {
       "ghcr.io/patina/features/foundry:1": {"version": "latest"},
       "ghcr.io/devcontainers/features/git:1": {},
       "ghcr.io/devcontainers/features/github-cli:1": {}
     },
     "build": {
       "dockerfile": "Dockerfile",
       "context": "."
     },
     "dockerComposeFile": "docker-compose.yml",
     "service": "workspace",
     "workspaceFolder": "/workspace",
     "containerEnv": {
       "PATINA_YOLO": "1",
       "SKIP_PERMISSIONS": "1",
       "AI_WORKSPACE": "1",
       "IS_SANDBOX": "1"
     },
     "mounts": [
       "source=${localWorkspaceFolder}/layer,target=/workspace/layer,type=bind",
       "source=${localEnv:HOME}/.patina/credentials,target=/root/.credentials,type=bind,readonly"
     ],
     "customizations": {
       "vscode": {
         "extensions": ["JuanBlanco.solidity"],
         "settings": {
           "terminal.integrated.defaultProfile.linux": "bash"
         }
       }
     },
     "forwardPorts": [3000, 8000, 8080, 8545],
     "remoteUser": "root",
     "overrideCommand": true,
     "postCreateCommand": "bash /workspace/.devcontainer/yolo-setup.sh"
   }
   ```

4. **`generate_dockerfile()`** (because custom features needed):
   ```dockerfile
   # YOLO Development Container
   FROM mcr.microsoft.com/devcontainers/base:ubuntu

   ENV DEBIAN_FRONTEND=noninteractive

   # Install Node.js for Claude Code CLI
   RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
       && apt-get install -y nodejs \
       && npm install -g npm@latest pnpm@latest

   # Install Claude Code CLI
   RUN npm install -g @anthropic-ai/claude-code@latest \
       && mkdir -p /root/.claude-linux

   # Create Claude wrapper with YOLO permissions
   RUN echo '#!/bin/bash' > /usr/local/bin/claude \
       && echo 'export CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS=true' >> /usr/local/bin/claude \
       && echo 'exec node /usr/.../claude-code/cli.js --dangerously-skip-permissions "$@"' >> /usr/local/bin/claude \
       && chmod +x /usr/local/bin/claude

   # Install Foundry (CUSTOM FEATURE)
   RUN curl -L https://foundry.paradigm.xyz | bash && \
       /root/.foundry/bin/foundryup && \
       echo 'export PATH="/root/.foundry/bin:$PATH"' >> /etc/bash.bashrc
   ENV PATH="/root/.foundry/bin:$PATH"

   WORKDIR /workspace
   ENV PATINA_YOLO=1
   CMD ["/bin/bash"]
   ```

5. **`generate_docker_compose()`**:
   ```yaml
   version: "3.8"
   services:
     workspace:
       build:
         context: .
         dockerfile: Dockerfile
       volumes:
         - ..:/workspace:cached
         - ~/.patina/claude-linux:/root/.claude-linux:cached
         - ~/.claude:/root/.claude-macos:ro
       working_dir: /workspace
       command: sleep infinity
       ports:
         - "3000:3000"
         - "3001:3001"
         - "3008:3008"
         - "8000:8000"
         - "8080:8080"
         - "8545:8545"
       environment:
         PATINA_YOLO: "1"
         SKIP_PERMISSIONS: "1"
         AI_WORKSPACE: "1"
         IS_SANDBOX: "1"
         CLAUDE_CONFIG_DIR: "/root/.claude-linux"
         CLAUDE_DANGEROUSLY_SKIP_PERMISSIONS: "true"

     anvil:
       image: ghcr.io/foundry-rs/foundry:latest
       command: anvil --host 0.0.0.0
       ports:
         - "8545:8545"
   ```

6. **`generate_yolo_setup()`**:
   ```bash
   #!/bin/bash
   echo "🎯 Setting up YOLO workspace..."

   # Configure git
   if [ -z "$(git config --global user.email)" ]; then
       git config --global user.email "ai@patina.dev"
       git config --global user.name "AI Assistant"
   fi

   # Create directories
   mkdir -p ~/.credentials
   mkdir -p ~/.claude-linux

   # Shell aliases
   cat >> ~/.bashrc <<'EOF'
   alias yolo='echo "YOLO mode active - permissions bypassed"'
   alias status='git status'
   alias commit='git add -A && git commit -m'
   EOF

   # Check Claude auth
   echo "🤖 Checking Claude Code authentication..."
   if [ -f ~/.claude-linux/.credentials.json ]; then
       echo "✅ Claude already authenticated"
   else
       echo "⚠️  Claude not authenticated yet"
       echo "Run: claude login"
   fi

   echo "✅ YOLO workspace ready!"
   ```

### Output Files Generated

```
.devcontainer/
├── devcontainer.json     # Main config, references Dockerfile and compose
├── Dockerfile            # Custom image with Foundry + Claude
├── docker-compose.yml    # Workspace + Anvil services
└── yolo-setup.sh         # Post-create setup script
```

### Usage

**Launch container**:
```bash
docker compose -f .devcontainer/docker-compose.yml up -d
```

**Enter workspace**:
```bash
docker exec -it my-contracts-yolo bash
```

**Inside container**:
```bash
# Foundry tools available
forge --version
anvil --version
cast --version

# Claude available with YOLO permissions
claude "help me write a test for the NFT contract"

# Anvil blockchain running at localhost:8545
cast block-number --rpc-url http://anvil:8545
```

---

## Future Enhancements

### Scanner Improvements

**Version Parsing** (many TODOs in scanner.rs):
- Parse `package.json` engines field for Node version
- Parse `rust-toolchain.toml` for Rust version
- Parse `go.mod` for Go version
- Parse `foundry.toml` for Foundry and Solc versions
- Parse `pyproject.toml` for Python version

**Service Detection**:
- Parse `docker-compose.yml` to extract existing services
- Parse `mprocs.yaml` to detect process orchestration
- Detect database requirements (PostgreSQL, MySQL, Redis, MongoDB)

**Framework Detection**:
- React/Next.js → detect via package.json dependencies
- Django → detect via manage.py or settings.py
- Rails → detect via Gemfile or config/application.rb
- Express → detect via package.json dependencies

### Smart Inference Enhancements

**Language-Specific Services**:
- Django detected → add PostgreSQL service
- Rails detected → add PostgreSQL + Redis services
- React detected → configure hot reload
- Phoenix detected → add PostgreSQL service

**Port Detection**:
- Parse package.json scripts for port hints
- Parse framework configs for default ports
- Detect port conflicts with host system

### Feature Mapper Additions

**More Languages**:
- Java (Maven, Gradle)
- C# (.NET)
- Ruby (Rails, Bundler)
- PHP (Composer)
- Elixir (Mix)

**More Blockchain Tools**:
- Anchor (Solana)
- CosmWasm (Cosmos)
- Hardhat (already detected, needs feature)
- Truffle (already in profile, needs feature)

**Database Tools**:
- PostgreSQL
- MySQL
- Redis
- MongoDB
- SQLite

**More Dev Tools**:
- Docker-in-Docker (for building images in container)
- Kubernetes CLI (kubectl)
- Terraform
- AWS CLI

### Generator Improvements

**Interactive Mode** (mod.rs line 103):
- Prompt user to confirm detected stack
- Allow manual version selection
- Enable/disable specific features
- Save preferences to `.patina/preferences.toml`

**JSON Output** (mod.rs line 108):
- Complete implementation of JSON format
- Include detected profile
- Include generated features
- Include file paths for generated files
- Enable programmatic usage

**Template System**:
- Extract Dockerfile generation to templates
- Support custom templates via `.patina/templates/`
- Allow project-specific customization

**Feature Publishing**:
- Publish custom Patina features to `ghcr.io/patina/features/`
- Implement DevContainer feature format
- Add install scripts and metadata
- Enable cached layer optimization

### Testing & Validation

**Detection Accuracy**:
- Unit tests for each detector
- Fixture repositories for common stacks
- Regression tests for false positives/negatives

**Generation Validation**:
- Validate generated JSON against DevContainer schema
- Validate generated Dockerfiles (docker build)
- Validate generated compose files (docker compose config)

**Integration Tests**:
- Actually build and launch generated containers
- Run smoke tests inside containers
- Verify tools are available and working

---

## Conclusion

The YOLO devcontainer architecture is a **well-structured buildpack-style system** for generating autonomous AI development environments. It follows solid engineering principles:

**Separation of Concerns**: Scanner → Profile → Mapper → Generator
- Each phase has single responsibility
- Clear contracts between phases (Profile, Features)

**Modular Design**: DevContainerFeature as buildpack abstraction
- Each feature is isolated and self-contained
- Easy to add new features without modifying existing code
- Composition happens at generation time

**Pragmatic Trade-offs**:
- Official features first, custom fallback
- Convenience over isolation (credential mounting)
- Autonomy over security (YOLO permissions, sandboxed)

**Extensible Architecture**:
- Clear extension points for new languages/tools
- Well-documented patterns
- Template for adding features

**Future-Ready**:
- Foundation for feature publishing
- Support for interactive mode
- Template system for customization

The system successfully applies **Cloud Native Buildpacks philosophy** to devcontainer generation, creating a maintainable and extensible architecture for autonomous AI development environments.
