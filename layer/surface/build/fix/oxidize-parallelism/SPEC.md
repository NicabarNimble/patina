---
type: fix
id: oxidize-parallelism
status: draft
created: 2026-03-10
sessions:
  origin: 20260309-182853
exit_criteria: []
---
# fix: Oxidize embedding parallelism

> ONNX embedder hardcodes intra/inter threads to 1, embed_batch is sequential, and oxidize loops embed_passage one-at-a-time — large repos (92k functions) take 40+ minutes on M2 Max at 19% CPU instead of saturating all cores

## Problem

## Root Cause

## Fix

## Exit Criteria
