# MCT child WIT provenance

This directory is the SDK-owned canonical WIT surface for child authors targeting the standalone MCT runtime.

Copied contracts:

- `deps/logging.wit` copied from `wit/child/deps/logging.wit` on 2026-07-04.
- `deps/patina-measure.wit` copied from `wit/child/deps/patina-measure.wit` on 2026-07-04.
- `deps/patina-git.wit` copied from `wit/child/deps/patina-git.wit` on 2026-07-04.

The legacy sources remain untouched. Future deduplication should compare these paths first.

`child.wit` defines the default `patina:mct/child@0.1.0` export interface and imports only the non-filesystem WIT host adapters currently linked by MCT (`wasi:logging/logging@0.1.0`, `patina:measure/measure@0.1.0`, `patina:git/git@0.1.0`). Filesystem preopens are intentionally not imported by the default echo fixture; MCT gates filesystem imports on explicit preopens and exact WASI package identity.
