Capture Patina beliefs using the truthful project-local workflow for this runtime.

1. Treat beliefs as durable project truth. Watch for design decisions, repeated principles, strong "always/never" guidance, or lessons learned that should survive the session.

2. Ask the user before creating a new belief unless they already requested it explicitly.

3. Gather the required fields first:
   - `id`: lowercase kebab-case
   - `statement`: one clear sentence
   - `persona`: usually `architect`
   - `evidence`: at least one concrete source from this session or repo
   - optional `facets`

4. Create the belief with the deterministic script:
   - Execute `.opencode/bin/create-belief.sh --id "<id>" --statement "<statement>" --persona "<persona>" --evidence "<evidence>" [--facets "<facets>"]`

5. After the file is created:
   - Review `layer/surface/epistemic/beliefs/<id>.md`
   - Add `[[wikilinks]]` for beliefs, sessions, commits, and specs
   - Enrich supports/attacks/applied-in if the context is already known

6. Finish by recommending `patina scrape` so the belief gets indexed with real metrics.
