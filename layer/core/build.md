# Build Recipe

Persistent task tracking across sessions. Check items as completed, add notes inline.

**Specs:** Detailed implementation specs live in `layer/surface/spec-*.md`. Each phase below links to its spec.

---

## Active

- [ ] Event emitter - session-end writes to `.patina/events/`

## Queued

### Phase 1: Event Foundation
**Spec:** [layer/surface/spec-event-foundation.md](../surface/spec-event-foundation.md)
- [ ] Event emitter (session-end → events)
- [ ] Manifest tracker (`manifest.json`)
- [ ] Materializer command (`patina materialize`)
- [ ] Backfill existing sessions to events

### Phase 2: Mothership Service
**Spec:** [layer/surface/spec-mothership-service.md](../surface/spec-mothership-service.md)
- [ ] `patina serve` daemon (axum REST)
- [ ] `/embed` endpoint
- [ ] `/persona/query` endpoint
- [ ] `projects.registry` (YAML)

### Phase 3: Persona Capture
**Spec:** [layer/surface/spec-persona-capture.md](../surface/spec-persona-capture.md)
- [ ] `patina persona note` command
- [ ] Persona events at `~/.patina/persona/events/`
- [ ] Persona materializer

### Phase 4: Progressive Adapters
**Spec:** [layer/surface/spec-progressive-adapters.md](../surface/spec-progressive-adapters.md)
- [ ] Training pair generators
- [ ] 6 dimension adapters on frozen E5
- [ ] Dimension-weighted search
- [ ] Patina thickness model

### Phase 5: Cross-Project
**Spec:** [layer/surface/spec-cross-project.md](../surface/spec-cross-project.md)
- [ ] Query routing (project → mothership)
- [ ] Result tagging
- [ ] Container support

---

## Done

- [x] E5-base-v2 model working (2025-11)
- [x] USearch HNSW indices working (2025-11)
- [x] SQLite + call_graph data available (2025-11)
- [x] Event schema designed - LiveStore sequence model (2025-11-21, session 20251121-042111)
- [x] Mothership architecture clarified - Ollama-style daemon (2025-11-21, session 20251121-042111)
- [x] README rewritten with accurate commands (2025-11-21, bf22318e)
- [x] MIT license added (2025-11-21, bf22318e)

---

## Notes

- Transport: REST + optional WebSocket (not gRPC)
- Registry format: YAML content in `projects.registry` (no extension)
- Persona vs Session: same event mechanism, different scope/location
- Design docs: `layer/surface/patina-embedding-architecture.md`
