# Pi prompts projection

## Story
Fix Patina AI launcher so the Pi interface scaffolds prompt templates under `.pi/prompts` instead of old `.pi/commands`, then commit the change.

## Why
Pi renamed project-local prompt-template discovery from `commands/` to `prompts/`. Patina was still projecting the Pi bundle to the legacy path, which made Pi repair the project on startup with `Migrated Project commands/ → prompts/`.

## Direction
- Keep Claude/OpenCode/Gemini on their native `commands/` layouts.
- Move only Pi markdown prompt templates to `prompts/`.
- Migrate legacy managed Pi `commands/` directories during Patina template sync when no `prompts/` directory exists.
- Mark the Pi builtin bundle stale for existing `version = "builtin"` managed projections so normal refresh/launch can rewrite with the corrected layout.

## Closure
Implemented and verified:

- Pi skill projection paths now use `prompts/*.md`.
- Managed Pi template sync migrates `.pi/commands` and `.patina/skills/pi/*/commands` to `prompts` if needed.
- Setup/surface/template tests assert `.pi/prompts`.
- Smoke setup produced `.pi/prompts/*.md` with no `.pi/commands` directory.

## Notes
The Slate CLI created this work item but could not continue through normal set/promote/check commands because existing historical design-kind Slate files currently fail the Slate parser. This work record was completed manually to preserve the proof trail without broadening the requested code change.
