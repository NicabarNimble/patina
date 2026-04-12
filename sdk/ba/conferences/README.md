# Conference Intake (Lean)

Goal: capture only high-signal conference metadata for BA alignment.

## What we track per talk/session

- Event + year + date window
- Speaker
- Title
- Talk type (`keynote`, `workshop`, `talk`, `panel`)
- Topic summary (2-4 lines max)
- Primitive tags (`component-model`, `wit`, `wasi`, `composition`, etc.)
- Links: schedule, video, slides, repo (if available)
- Confidence/status fields from reality filter

## Lean rules

1. No long prose in catalog rows.
2. No binary slide dumps in git by default.
3. If slides exist, create a markdown companion note with extracted bullets.
4. Keep only decision-relevant sessions.

## Slides policy (markdown-compatible)

When a slide deck is relevant:

- Keep canonical link in `slides_url`.
- Add markdown extraction note at:
  - `sdk/ba/conferences/slides/<event>/<year>/<slug>.md`
- Optional visual companion (`.excalidraw`) only when diagram fidelity matters.

Minimal markdown note structure:

```md
# <talk title>
- Speaker:
- Event/date:
- Source slides URL:
- Primitive tags:

## Key claims
- ...

## Relevance to Patina
- aligns:
- extends:
- risks:
```

## Reality filter

Every row must include:
- `source_confidence`: `official_schedule | official_video | community_post`
- `status`: `confirmed | inferred | unverified`
- `decision_eligible`: true only for confirmed official evidence
