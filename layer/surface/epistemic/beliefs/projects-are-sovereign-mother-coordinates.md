---
type: belief
id: projects-are-sovereign-mother-coordinates
persona: architect
facets: [architecture, mother, projects, federation]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-30
revised: 2026-03-30
---

# projects-are-sovereign-mother-coordinates

Projects are sovereign islands that own their knowledge (layer/) and identity (uid). Mother coordinates access, orchestrates children, and enables cross-project queries — but does not own project data. A project at rest is complete without Mother.

## Statement

Projects are sovereign islands that own their knowledge (layer/) and identity (uid). Mother coordinates access, orchestrates children, and enables cross-project queries — but does not own project data. A project at rest is complete without Mother.

## Evidence

- [[session-20260330-083255-177610000]] - Derived through 3 spec rewrites of greenfield-mother-patina-data-platform. Initially designed Mother as centralized data owner; corrected after recognizing core-verbs-standalone-mother-additive principle and prior art (LiveStore, Matrix, Git — all sovereign-node models). (weight: 0.95)

## Supports

- [[five-boundaries-no-overlap]] — projects are the development zone, Mother is infrastructure. Sovereignty follows from non-overlapping roles.
- [[core-verbs-standalone-mother-additive]] — core protocol works without Mother. Projects must be complete at rest for this to hold.
- [[standards-are-storage-coordination-sits-above]] — SQLite per-project is the storage unit (sovereign). DuckDB/Mother coordinate above without replacing.

## Attacks

- Centralized data ownership models where a server owns all state and clients are thin.

## Attacked-By

- Operational complexity: per-project databases in Mother's directory means Mother must enumerate and manage N database sets. More moving parts than a single centralized DB.

## Applied-In

- [[spec-greenfield-mother-patina-data-platform]] — databases at `~/.patina/mother/projects/{uid}/` but CLI opens them directly as local files. Mother coordinates, doesn't gate.
- `src/paths.rs` paths::mother::projects module — path resolution from project_uid to Mother-scoped database locations.

## Revision Log

- 2026-03-30: Created — metrics computed by `patina scrape`
