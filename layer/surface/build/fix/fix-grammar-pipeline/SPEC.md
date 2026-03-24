---
type: fix
id: fix-grammar-pipeline
status: draft
created: 2026-03-24
sessions:
  origin: 20260324-105924-440442000
exit_criteria: []
---
# fix: Fix WASM grammar child discovery and native Rust fallback

> Plugin→child rename broke WASM grammar discovery (installed plugins have plugin.toml with [plugin] section, but ChildManifest::from_path() now requires [child]). Additionally, the native Rust fallback has a tree-sitter ABI mismatch (0.24.7 vs 0.24.0).

## Problem

## Root Cause

## Fix

## Exit Criteria
