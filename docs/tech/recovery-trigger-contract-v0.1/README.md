# Recovery Trigger Contract v0.1

This folder's companion specification is `docs/tech/RECOVERY_TRIGGER_CONTRACT_V0_1.md`; the frozen synthetic boundary corpus is `docs/tech/recovery-trigger-corpus-v0.1.json`.

## Current scope

The first tranche freezes the observable signpost fields, profile and criteria version, enter/exit boundary semantics, observation freshness/coverage, response lead-time check, review-only disposition and synthetic acceptance cases. It does not include a production controller or domain-calibrated trigger thresholds.

The corpus spans forest weather/operation review, PV degradation review and infrastructure crack-growth review. All values are invented to test boundary logic. No result authorizes a physical operation.

## Intended next implementation

Implement a deterministic Rust reference evaluator with a separate test oracle and exact-head receipt. Required tests include threshold equality, stale and incomplete evidence, units, prior active/inactive states, exit boundary, criteria version drift, out-of-order/duplicate observations and response that arrives too late.
