# MCT child WIT provenance

Patina's WIT convention makes `wit/mct/` the canonical home and `sdk/patina-sdk/wit/mct/` a byte-identical SDK mirror. The pre-push WIT consistency hook enforces this canonical-to-SDK mirror relationship for enrolled worlds.

This inverts the original SDK-track framing, which initially treated the SDK directory as canonical. The effective source of truth is now:

- `wit/mct/mct.wit`
- `wit/mct/deps/logging.wit`
- `wit/mct/deps/patina-measure.wit`
- `wit/mct/deps/patina-git.wit`

SDK mirror paths:

- `sdk/patina-sdk/wit/mct/mct.wit`
- `sdk/patina-sdk/wit/mct/deps/logging.wit`
- `sdk/patina-sdk/wit/mct/deps/patina-measure.wit`
- `sdk/patina-sdk/wit/mct/deps/patina-git.wit`

Copied contract sources for the initial MCT surface:

- `logging.wit` copied from `wit/child/deps/logging.wit` on 2026-07-04.
- `patina-measure.wit` copied from `wit/child/deps/patina-measure.wit` on 2026-07-04.
- `patina-git.wit` copied from `wit/child/deps/patina-git.wit` on 2026-07-04.

The legacy sources remain untouched. Future deduplication should compare these paths first.

`mct.wit` defines the default `patina:mct/child@0.1.0` export interface and imports only the non-filesystem WIT host adapters currently linked by MCT (`wasi:logging/logging@0.1.0`, `patina:measure/measure@0.1.0`, `patina:git/git@0.1.0`). Filesystem preopens are intentionally not imported by the default echo fixture; MCT gates filesystem imports on explicit preopens and exact WASI package identity.
