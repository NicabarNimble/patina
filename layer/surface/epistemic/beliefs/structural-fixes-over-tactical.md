---
type: belief
id: structural-fixes-over-tactical
persona: architect
facets: [architecture, design-principles]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# structural-fixes-over-tactical

Choose solutions where the problem cannot recur, not solutions that merely address the current instance. Plugins solve version conflicts structurally (each bundles its own deps) where compiled-from-source solved them tactically (git submodules, custom build.rs).

## Statement

Choose solutions where the problem cannot recur, not solutions that merely address the current instance. Plugins solve version conflicts structurally (each bundles its own deps) where compiled-from-source solved them tactically (git submodules, custom build.rs).

## Evidence

- [[session-20260214-130235]]: [[grammar-extraction]] — tree-sitter version conflict hell (0.23 vs 0.24 links conflicts, session [[20250901-135830]]) was tactically fixed by compiling from source. Plugin isolation makes the problem structurally impossible — each plugin bundles its own parser version. (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
