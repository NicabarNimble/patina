---
type: fix
id: fetch-result-deserialize
status: draft
created: 2026-03-09
related:
- pipe-contract-safety
exit_criteria: []
---
# fix: FetchResult response parsing should use serde_json::from_value()

> broker::lifecycle::NativeChild::fetch() manually plucks fields from pipe/fetch response JSON instead of deserializing via shared FetchResult type — silently defaults missing emitted to 0

## Problem

## Root Cause

## Fix

## Exit Criteria
