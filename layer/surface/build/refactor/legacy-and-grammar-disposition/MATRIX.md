# Legacy and Grammar Disposition Matrix

## Scope

Inventory of child lanes for disposition planning under `legacy-and-grammar-disposition`.

This slice is inventory-only (no keep/migrate/retire decisions yet).

## Typed baseline children (current typed lane)

| child | kind | sdk lane | current runtime lane | risk notes |
|---|---|---|---|---|
| file-system-monitor | child | patina-sdk | typed child | Low: typed lane active; monitor WIT dependency drift |
| content-extractor | child | patina-sdk | typed child | Low: typed lane active; monitor records/extract contract drift |
| schema-enforcer | child | patina-sdk | typed child | Low: typed lane active; monitor transform contract evolution |
| dedup-filter | child | patina-sdk | typed child | Low: typed lane active; monitor keyvalue + transform contract stability |
| record-writer | child | patina-sdk | typed child | Low: typed lane active; monitor write contract + provenance fields |
| lakehouse-catalog | child | patina-sdk | typed child | Low: typed lane active; monitor catalog contract + sql host assumptions |

## Legacy service children (current legacy lane)

| child | kind | sdk lane | current runtime lane | risk notes |
|---|---|---|---|---|
| belief-verifier | child | patina-sdk-legacy | service handle lane | Medium: legacy SDK + `handle/drain/tick` semantics; touches events/task/belief scopes |
| session-writer | child | patina-sdk-legacy | service handle lane | Medium: legacy SDK + session/peer toy coupling to interface/session workflows |
| spec-manager | child | patina-sdk-legacy | service handle lane (with builtin overlap today) | High: split authority risk (service child artifacts + Mother builtin dispatch history) |
| doctor | child | patina-sdk-legacy | service handle lane | Medium: legacy SDK lane and overlap with host-native doctor fallback behavior |

## Grammar children (current pipeline lane)

| child | kind | sdk lane | current runtime lane | risk notes |
|---|---|---|---|---|
| grammar-c | pipeline | n/a (grammar crate) | pipeline lane | Medium: pipeline-only contract, no typed composition decision locked yet |
| grammar-cairo | pipeline | n/a (grammar crate) | pipeline lane | Medium: pipeline-only contract, no typed composition decision locked yet |
| grammar-cpp | pipeline | n/a (grammar crate) | pipeline lane | Medium: pipeline-only contract, no typed composition decision locked yet |
| grammar-go | pipeline | n/a (grammar crate) | pipeline lane | Medium: pipeline-only contract, no typed composition decision locked yet |
| grammar-javascript | pipeline | n/a (grammar crate) | pipeline lane | Medium: pipeline-only contract, no typed composition decision locked yet |
| grammar-python | pipeline | n/a (grammar crate) | pipeline lane | Medium: pipeline-only contract, no typed composition decision locked yet |
| grammar-rust | pipeline | n/a (grammar crate) | pipeline lane | Medium: pipeline-only contract, no typed composition decision locked yet |
| grammar-solidity | pipeline | n/a (grammar crate) | pipeline lane | Medium: pipeline-only contract, no typed composition decision locked yet |
| grammar-typescript | pipeline | n/a (grammar crate) | pipeline lane | Medium: pipeline-only contract, no typed composition decision locked yet |
