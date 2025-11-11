# Patina.app Architecture

**Decision Date**: 2025-11-10
**Model**: Tailscale-style Mac app (menu bar + CLI + daemon)
**Status**: Implementation ready

## Why We're Building This

### Requirements That Need Daemon
1. **Persona SQLite** - `~/.patina/persona/persona.db` must be always accessible across projects
2. **Embeddings Model** - 23MB model with 500ms cold start → load once, use instantly
3. **P2P Sync** (future) - Must listen for CRDT sync connections
4. **Cross-Project Aggregation** - Multiple projects need live persona data

### The Tailscale Model
- Menu bar app (minimal GUI)
- Background daemon (all logic)
- CLI works with daemon (instant) or standalone (slower fallback)
- Install via script/curl (not App Store required)

---

## Architecture

### Components

```
┌─────────────────────────────────────────────────┐
│  Patina.app (Menu Bar)                         │
│  ┌──────────────────────────────────────────┐  │
│  │ Swift NSStatusItem                       │  │
│  │ - Shows status from daemon               │  │
│  │ - Launches daemon on start               │  │
│  │ - Kills daemon on quit                   │  │
│  └──────────────────────────────────────────┘  │
│                    ↓ launch/status             │
│  ┌──────────────────────────────────────────┐  │
│  │ Rust Daemon (patina daemon start)        │  │
│  │ - HTTP server (localhost:42069)          │  │
│  │ - Embeddings loaded (23MB in memory)     │  │
│  │ - Persona DB open (~/.patina/persona/)   │  │
│  │ - Prolog engine initialized              │  │
│  │ - P2P listener ready (future)            │  │
│  └──────────────────────────────────────────┘  │
│                    ↑ HTTP requests             │
└────────────────────┼───────────────────────────┘
                     │
         ┌───────────┴────────────┐
         │                        │
    CLI (terminal)          Other Projects
    patina session build    patina query "..."
    → HTTP to daemon        → HTTP to daemon
    → Or direct if no daemon

```

### Directory Structure

```
Patina.app/
├── Contents/
│   ├── MacOS/
│   │   └── patina                    # Rust binary (daemon + CLI)
│   ├── Resources/
│   │   ├── models/
│   │   │   └── all-MiniLM-L6-v2.onnx # Embeddings model (23MB)
│   │   ├── AppIcon.icns
│   │   └── Base.lproj/
│   ├── Frameworks/                   # Rust dependencies
│   └── Info.plist

/usr/local/bin/patina → Patina.app/Contents/MacOS/patina  # CLI symlink
```

---

## Implementation Phases

### Phase 1: Daemon HTTP API Design

**Endpoints:**

```rust
// Health & Status
GET  /health                  → { "status": "ok" }
GET  /status                  → { "daemon": "running", "embeddings": "loaded", ... }
POST /shutdown                → Graceful shutdown

// Session Commands
POST /session/build           → Build current session into todos
  Body: { "session_path": ".claude/context/active-session.md" }
  Response: { "todos": [...], "decisions": [...], "topics": [...] }

// Query Commands
POST /query/semantic          → Semantic search
  Body: { "query": "error handling", "limit": 10 }
  Response: { "results": [...] }

// Belief Commands
POST /belief/validate         → Validate belief with Prolog
  Body: { "belief": "...", "min_score": 0.5 }
  Response: { "valid": true, "evidence": [...] }

// Persona Commands
GET  /persona/stats           → Persona database stats
POST /persona/sync            → Trigger persona sync (future P2P)
```

**Implementation:**
```rust
// src/daemon/mod.rs
use axum::{Router, Json};
use tower_http::cors::CorsLayer;

pub async fn start_daemon() -> Result<()> {
    // Load heavy resources ONCE
    let embedder = Arc::new(create_embedder()?);
    let prolog = Arc::new(ReasoningEngine::new()?);
    let persona_db = Arc::new(PersonaDatabase::open("~/.patina/persona")?);

    let app = Router::new()
        .route("/health", get(health))
        .route("/status", get(|| async { status(embedder, persona_db) }))
        .route("/session/build", post(session_build))
        .route("/query/semantic", post(query_semantic))
        .layer(CorsLayer::permissive())
        .with_state(AppState { embedder, prolog, persona_db });

    println!("🚀 Patina daemon started on localhost:42069");

    axum::Server::bind(&"127.0.0.1:42069".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

### Phase 2: Daemon Core Implementation

**Files:**
```
src/daemon/
├── mod.rs              # Main daemon entry point
├── server.rs           # HTTP server setup
├── routes/
│   ├── health.rs       # Health check endpoint
│   ├── session.rs      # Session build endpoint
│   ├── query.rs        # Query endpoints
│   └── belief.rs       # Belief validation
├── state.rs            # Shared daemon state
└── shutdown.rs         # Graceful shutdown handler
```

**Key Features:**
- Embeddings model stays in memory
- Persona DB connection pooling
- Graceful shutdown (save state, close DB)
- Request logging for debugging

### Phase 3: Swift Menu Bar App

**File:** `macos/Patina/StatusBarController.swift`

```swift
import Cocoa

class StatusBarController: NSObject {
    private var statusItem: NSStatusItem!
    private var daemonProcess: Process?

    override init() {
        super.init()

        // Create menu bar icon
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.button?.image = NSImage(named: "MenuBarIcon")
        statusItem.button?.image?.isTemplate = true

        // Create menu
        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "Patina", action: nil, keyEquivalent: ""))
        menu.addItem(NSMenuItem.separator())
        menu.addItem(withTitle: "Status: Loading...", action: nil, keyEquivalent: "")
        menu.addItem(NSMenuItem.separator())
        menu.addItem(withTitle: "Preferences...", action: #selector(openPreferences), keyEquivalent: ",")
        menu.addItem(withTitle: "Quit Patina", action: #selector(quit), keyEquivalent: "q")

        statusItem.menu = menu

        // Launch daemon
        startDaemon()

        // Update status periodically
        Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            self.updateStatus()
        }
    }

    private func startDaemon() {
        let daemonPath = Bundle.main.path(forResource: "patina", ofType: nil, inDirectory: "MacOS")!

        daemonProcess = Process()
        daemonProcess?.executableURL = URL(fileURLWithPath: daemonPath)
        daemonProcess?.arguments = ["daemon", "start"]
        daemonProcess?.launch()
    }

    private func updateStatus() {
        // HTTP GET to localhost:42069/status
        guard let url = URL(string: "http://localhost:42069/status") else { return }

        URLSession.shared.dataTask(with: url) { data, _, error in
            guard let data = data, error == nil else { return }

            // Parse JSON, update menu
            if let status = try? JSONDecoder().decode(DaemonStatus.self, from: data) {
                DispatchQueue.main.async {
                    self.statusItem.menu?.items[2].title = "Status: \(status.daemon)"
                }
            }
        }.resume()
    }

    @objc private func quit() {
        // Graceful shutdown
        let url = URL(string: "http://localhost:42069/shutdown")!
        var request = URLRequest(url: url)
        request.httpMethod = "POST"

        URLSession.shared.dataTask(with: request).resume()

        // Wait a bit, then force quit
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            NSApplication.shared.terminate(nil)
        }
    }
}

struct DaemonStatus: Codable {
    let daemon: String
    let embeddings: String
    let observations: Int
}
```

### Phase 4: CLI Auto-Detection

**Update:** `src/commands/session/build.rs`

```rust
pub fn build_session() -> Result<()> {
    // Try daemon first (fast path)
    if let Ok(result) = try_daemon_build() {
        println!("✓ Built via daemon ({:?})", result.duration);
        return Ok(());
    }

    // Fallback to direct (slow but works)
    println!("⚠️  Daemon not running, using direct mode (slower)");
    println!("   Tip: Start Patina.app for instant builds");
    build_session_direct()
}

fn try_daemon_build() -> Result<BuildResult> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()?;

    let response = client
        .post("http://localhost:42069/session/build")
        .json(&serde_json::json!({
            "session_path": ".claude/context/active-session.md"
        }))
        .send()?;

    Ok(response.json()?)
}

fn build_session_direct() -> Result<()> {
    // Original implementation (loads embeddings, etc)
    let embedder = create_embedder()?;  // 500ms cold start
    // ... process session
    Ok(())
}
```

### Phase 5: Packaging

**Info.plist:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>patina</string>
    <key>CFBundleIdentifier</key>
    <string>com.nicabar.patina</string>
    <key>CFBundleName</key>
    <string>Patina</string>
    <key>CFBundleVersion</key>
    <string>0.2.0</string>
    <key>LSUIElement</key>
    <true/>  <!-- No dock icon -->
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
```

**Build script:** `scripts/build-app.sh`

```bash
#!/bin/bash
set -e

echo "Building Patina.app..."

# Build Rust binary (universal)
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create \
    target/aarch64-apple-darwin/release/patina \
    target/x86_64-apple-darwin/release/patina \
    -output target/release/patina

# Build Swift app
xcodebuild -project macos/Patina.xcodeproj \
           -configuration Release \
           -arch arm64 -arch x86_64

# Copy Rust binary into app
cp target/release/patina \
   macos/build/Release/Patina.app/Contents/MacOS/

# Copy models
mkdir -p macos/build/Release/Patina.app/Contents/Resources/models
cp resources/models/*.onnx \
   macos/build/Release/Patina.app/Contents/Resources/models/

# Sign (if Developer ID available)
if [ -n "$DEVELOPER_ID" ]; then
    codesign --deep --force --sign "$DEVELOPER_ID" \
        macos/build/Release/Patina.app
fi

# Create DMG
hdiutil create -volname "Patina" \
               -srcfolder macos/build/Release/Patina.app \
               -ov \
               dist/Patina.dmg

echo "✓ Patina.app built: dist/Patina.dmg"
```

### Phase 6: Install Script

**File:** `install.sh`

```bash
#!/bin/bash
set -e

PATINA_VERSION="0.2.0"
DOWNLOAD_URL="https://github.com/nicabar/patina/releases/download/v${PATINA_VERSION}/Patina.dmg"

echo "Installing Patina ${PATINA_VERSION}..."
echo ""

# Download
echo "→ Downloading..."
curl -L "$DOWNLOAD_URL" -o /tmp/Patina.dmg

# Mount and install
echo "→ Installing to /Applications..."
hdiutil attach /tmp/Patina.dmg -quiet
cp -R /Volumes/Patina/Patina.app /Applications/
hdiutil detach /Volumes/Patina -quiet

# Symlink CLI
echo "→ Setting up CLI..."
sudo ln -sf /Applications/Patina.app/Contents/MacOS/patina /usr/local/bin/patina

# Download models
echo "→ Downloading embeddings model..."
/usr/local/bin/patina daemon setup

echo ""
echo "✓ Patina installed successfully!"
echo ""
echo "To start:"
echo "  open /Applications/Patina.app"
echo ""
echo "Or use CLI:"
echo "  patina --help"
```

**Usage:**
```bash
curl -sSL https://patina.sh/install.sh | bash
```

### Phase 7: Distribution

**GitHub Release:**
- Tag: `v0.2.0`
- Assets:
  - `Patina.dmg` (signed, notarized)
  - `install.sh`
  - `RELEASE_NOTES.md`

**Brew Cask (later):**
```ruby
cask "patina" do
  version "0.2.0"
  sha256 "..."

  url "https://github.com/nicabar/patina/releases/download/v#{version}/Patina.dmg"
  name "Patina"
  desc "Context orchestration for AI development"
  homepage "https://github.com/nicabar/patina"

  app "Patina.app"
  binary "#{appdir}/Patina.app/Contents/MacOS/patina"
end
```

---

## Success Criteria

### Phase 1-2: Daemon Working
- [ ] `patina daemon start` runs HTTP server
- [ ] Embeddings model loads (23MB)
- [ ] Persona DB opens successfully
- [ ] `/health` endpoint returns 200
- [ ] `/session/build` returns structured todos

### Phase 3: Menu Bar Working
- [ ] App appears in menu bar
- [ ] Shows "Status: Running"
- [ ] Daemon launches on app start
- [ ] Daemon shuts down on quit
- [ ] Status updates every 5 seconds

### Phase 4: CLI Integration
- [ ] `patina session build` hits daemon (<100ms)
- [ ] Falls back to direct if daemon not running
- [ ] All existing commands work

### Phase 5-6: Packaging
- [ ] Patina.app installs to /Applications
- [ ] CLI symlink works from any directory
- [ ] Models download on first run
- [ ] install.sh works via curl

### Phase 7: Distribution
- [ ] GitHub release created
- [ ] DMG signed and notarized
- [ ] Users can install without App Store

---

## Timeline Estimate

- **Phase 1**: 4 hours (API design + basic server)
- **Phase 2**: 8 hours (full daemon implementation)
- **Phase 3**: 6 hours (Swift menu bar app)
- **Phase 4**: 2 hours (CLI auto-detection)
- **Phase 5**: 4 hours (packaging + build scripts)
- **Phase 6**: 2 hours (install script)
- **Phase 7**: 4 hours (signing, release)

**Total: ~30 hours** (1 week focused work)

---

## Related Documents

- Session: layer/sessions/20251110-055746.md (discovery session)
- Design: layer/surface/patina-llm-driven-neuro-symbolic-knowledge-system.md (neuro-symbolic architecture)

---

## Next Steps

1. ✓ Document architecture (this file)
2. Start Phase 1: Design daemon HTTP API
3. Prototype `/session/build` endpoint
4. Test with current session (dogfood)
