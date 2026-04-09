# patina-sdk-legacy

`patina-sdk-legacy` is the legacy handle-based SDK.

Use this crate only when maintaining existing service children that implement the
legacy `Child` trait (`handle(action, payload)` flow) or when maintaining legacy
grammar pipelines through the `pipeline` feature.

For all new push-pure WASM children, use `sdk/patina-sdk`.

## Legacy Usage

Legacy child crates should keep the Rust import path `patina_sdk::*` by using a
Cargo package alias:

```toml
[dependencies]
patina-sdk = { package = "patina-sdk-legacy", path = "../../sdk/patina-sdk-legacy", features = ["child"] }
```

Enable exactly one legacy world feature per crate:

- `child` for legacy service children
- `pipeline` for legacy grammar pipeline lane

## Migration Note

If you are starting a new child, stop here and use `patina-sdk` +
`cargo generate --path sdk/template` instead.

## License

MIT
