# Power Fabric dependency-aware change impact v0.1

**Status:** executable dependency graph and invalidation planner; not an electrical solver or control system  
**Date:** 2026-11-14  
**Owner issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5213  
**Parent:** diagnostics PR #5217 → IR PR #5211

## 1. Purpose

A source change must invalidate every derived output whose assumptions depend on it, while preserving unrelated outputs only when their dependency inventory is complete and compiler/profile versions remain current.

This module computes invalidation. It does not perform subsequent engineering recomputation or prove full/incremental semantic equivalence. That is a separate benchmark and compiler-orchestrator responsibility.

## 2. Model

- Input fingerprints bind an input ID to a caller-supplied content digest.
- Derived artifacts bind output digest, compiler version, profile version, explicit dependency IDs, and a dependency-inventory completeness flag.
- Dependencies may point to original inputs or other derived artifacts.
- The graph rejects duplicate IDs, cross-kind ID collisions, unknown dependencies, duplicate dependencies, self-dependencies and cycles.
- Reverse dependencies are built deterministically.
- Every changed input includes an expected old digest, preventing invalidation against the wrong snapshot.
- A missing new digest still invalidates dependents and sets reconciliation-required. It never permits reuse.

## 3. Invalidation rules

Invalidate an artifact when a direct input dependency changed, a transitive derived dependency changed, its compiler version differs, its profile version differs, or its dependency inventory is incomplete.

Invalidation propagates through derived artifacts. Unaffected artifacts are retained only when their declared dependency graph is complete and versions match. An incomplete inventory is not an empty dependency list: its artifact is always invalidated, and downstream dependents are invalidated transitively.

## 4. Change receipts and concurrency

Every mutation includes input ID, expected old digest, and new digest or null if the new state is unresolved. If the expected digest does not match the graph snapshot, the calculation rejects the change. Same-digest mutations are no-ops. Duplicate changes and unknown inputs are rejected.

The report records old/new digests, invalidated artifacts with stable reason codes, retained output digests, and whether reconciliation is required. The graph itself is immutable; callers create a new snapshot only after reconciling new inputs.

## 5. Determinism and complexity

IDs and dependency lists are ordered. The Kahn topological cycle check is deterministic. Reverse-dependency traversal uses ordered sets. The implementation favors transparent deterministic behavior over a specialized large-scale graph engine.

Benchmark at 100, 1,000, 10,000, and 100,000 nodes. Record p50/p95 time, peak memory, fan-out/fan-in distribution, and mutation shape. Do not publish performance claims until measured on disclosed hardware and an exact compiler revision.

## 6. Frozen acceptance corpus

The synthetic fixture includes a catalog update affecting compatibility, source-map, resident view, and emergency projection transitively, while an unrelated carbon report remains reusable. A second unknown-digest change must invalidate dependent outputs and require reconciliation.

Targets:
- 100% invalidation recall against the frozen dependency oracle;
- zero unrelated invalidations for independent nodes;
- 100% rejection of stale expected-old-digest changes;
- 100% fail-closed behavior for unknown new digests and incomplete inventories;
- exact semantic equality between full and incremental recomputation in the later compiler integration benchmark.

The final target is not established by this invalidation planner alone.

## 7. Claim ceiling

An invalidation report says which derived artifacts must be recomputed or reconciled. It does not establish safety, compliance, powered/de-energized state, physical isolation, or approval. It does not trigger switching or device control. Unknown outcomes block reuse until reconciliation.
