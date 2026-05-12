# Slate

Slate is Patina's project-living work system for build/refactor/fix change transactions.

Durable Slate work items live with the project so collaborators and agents see the same work context alongside [[Allium]] intent and [[beliefs]]. Mother may project these files into a per-project `slate.db` for fast WIT/WASI routing, but the project artifacts remain the shareable source of truth.

Initial work-item layout:

```text
layer/slate/work/<slate-id>/work.toml
```

`patina spec` is a separate island. Explicit bridge/import/export operations may exist, but `SPEC.md` and `DESIGN.md` are not canonical Slate storage.

## Slate Child Package

Slate's standalone child package is public at <https://github.com/NicabarNimble/patina-child-slate> with a local checkout at `/Users/nicabar/Projects/Patina/patina-child-slate`.

Work on Slate manager behavior in that external repo. Patina consumes Slate as an installed child package rather than as an in-tree workspace crate.

## Child Skill Packages

Slate child skills are owned by `slate-manager` and should be discoverable by Mother for active children.

Current child-owned packages:

- `/Users/nicabar/Projects/Patina/patina-child-slate/skills/slate-code/SKILL.md`
- `/Users/nicabar/Projects/Patina/patina-child-slate/skills/slate-version-control/SKILL.md`

Mother exposes installed child skill discovery through commands such as `patina mother skills show slate-manager`.

## Version / Archive Semantics

Slate and `patina spec` remain separate islands, but for now Slate version/archive behavior should match spec behavior where possible.

Target Slate archive behavior:

1. work is terminal (`complete` or `abandoned`) unless forced;
2. tracked working tree is clean before archive;
3. archive removes `layer/slate/work/<id>/` from the working tree;
4. archive creates a commit such as `docs: archive slate/<id> (<status>)`;
5. archive creates recovery tag `slate/<id>` pointing at the pre-removal commit;
6. recovery instructions use `git show slate/<id>:layer/slate/work/<id>/work.toml`.

This is intentionally spec-parity for now and may diverge later if Slate's work-transaction model needs different version boundaries.
