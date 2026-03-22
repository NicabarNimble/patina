# patina-sdk

`patina-sdk` is the authoring surface for Patina WASM children.

## SDK Tiers

- `patina-sdk-core`: child trait + core toys (`log`, `state`, substrate types)
- `patina-sdk-data`: data toys (`lake`, `checkpoint`, `measure`, `github`)
- `patina-sdk-agent`: agent/session toys (`query`, `emit`, `session`)
- `patina-sdk`: umbrella crate that re-exports tier APIs

Use the umbrella crate unless you are building advanced tooling around the tiers directly.

## 5-Minute Onramp

1. Generate a child from the template:

```sh
cargo generate --path children/template
```

2. Build the child (WASM):

```sh
cargo build --target wasm32-wasip2
```

3. Ensure `plugin.toml` uses `[needs].toys` and a `[provides]` child name.

4. Install the child artifact + manifest into Patina's children directory.

5. Start Mother and verify the child loads:

```sh
patina mother start
patina mother status
```

## Knowledge Child Baseline

Use this feature set for a minimal child:

```toml
[dependencies]
patina-sdk = { version = "0.21", features = ["knowledge-child", "toy-log"] }
```

Add toys incrementally (`toy-state`, `toy-checkpoint`, `toy-lake`, `toy-github`, `toy-session`, etc.)
as your `plugin.toml` grants expand.

## World Features

Enable exactly one world feature per crate:

- `knowledge-child` (default path)
- `task`
- `command`
- `pipeline`
- `mother-child` (legacy migration lane)

## Child Relationships

Children can declare mediated event relationships in `plugin.toml`:

```toml
[relationships]
emits = ["data-ingested"]
listens = ["data-ingested"]
```

Use this to describe child-to-child flow while keeping Mother as the routing authority.

Example pattern:

- `ducklake` emits `data-ingested` after sync
- `session-writer` listens to `data-ingested` and appends activity notes

## License

MIT
