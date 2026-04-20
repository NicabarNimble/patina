# Homebrew Packaging Notes

This directory stores the canonical Patina formula template for custom tap distribution.

Current target support:
- macOS Apple Silicon (`aarch64-apple-darwin`)
- Linux x86_64 (`x86_64-unknown-linux-gnu`)

## Usage

1. Copy `Formula/patina.rb` into your tap repository (`homebrew-tap/Formula/patina.rb`).
2. Replace SHA placeholders with release checksum values from `checksums.txt`.
3. Commit and push the tap update.

## Service model

The formula includes a `service` stanza:

```ruby
run [opt_bin/"patina", "mother", "start"]
```

Users can run:

```bash
brew services start patina
brew services restart patina
brew services stop patina
```

For Homebrew installs, prefer `brew services` instead of ad-hoc daemon process management.
