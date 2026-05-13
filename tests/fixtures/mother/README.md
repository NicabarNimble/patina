# Mother MCT fixtures

Durable fixtures for Mother/Child/Toy conformance and skill lifecycle sandbox tests.

Vocabulary:

- **children/**: durable child app fixtures Mother can treat as child sources.
- **scenarios/**: filesystem/projection states materialized into sandboxes.
- **golden/**: stable machine-readable command outputs.
- **capabilities/**: HITL capability matrix fixtures used by tests.

The dev sandbox CLI materializes these into isolated global-local sandboxes under `~/.patina/local/dev/mother-skill-sandboxes/<id>/`.
