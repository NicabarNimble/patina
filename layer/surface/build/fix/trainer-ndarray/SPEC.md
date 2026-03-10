---
type: fix
id: trainer-ndarray
status: draft
created: 2026-03-10
sessions:
  origin: 20260309-182853
related:
- oxidize-parallelism
exit_criteria: []
---
# fix: MLP trainer: Vec of Vec to ndarray with BLAS

> Projection trainer uses Vec<Vec<f32>> for weight matrices and hand-rolled dot/backprop loops — no SIMD, no BLAS, pointer-chasing on every multiply. 92k triplets × 10 epochs takes minutes on M2 Max. ndarray with blas-src/accelerate-src would use Apple Accelerate sgemm.

## Problem

## Root Cause

## Fix

## Exit Criteria
