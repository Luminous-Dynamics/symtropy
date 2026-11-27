# As-built magnetic functional acceptance

**Programme:** MAG-ASBUILT-FUNCTION-QUAL-001  
**Issue:** #7745  
**Revision:** 0.1-draft  
**Status:** protocol and synthetic semantic fixtures; no solver or physical result claimed

## Purpose

Determine whether an identified manufactured magnetic specimen meets its target field or component function after accounting for its actual geometry, material state, manufacturing genealogy, process history and post-build magnetization history.

This is deliberately not another magnetic field solver, material catalog, manufacturing ledger, metrology service or release authority. It is a small acceptance adapter over existing, separately qualified evidence.

## Read first

- [Protocol](../MAG_ASBUILT_FUNCTIONAL_ACCEPTANCE_2026-11-27.md)
- [Machine-readable benchmark plan](benchmark-plan.toml)
- [Synthetic known-answer fixtures](fixtures/acceptance-cases-v0.1.json)
- [Fixture structural schema](acceptance-cases-v0.1.schema.json)
- [Rust semantic evaluator](evaluator/README.md)

## Compose, do not duplicate

- Symtropy #2933 — manufacturing genealogy and process evidence.
- Symtropy #7296 — recycled NdFeB powder packing/alignment process twin.
- Symtropy #7310 — spatially programmed multi-material magnetic process twin.
- Symtropy #7332 — exact-grid vector field-map metrics.
- Symtropy #7639 — continuous-domain functional-margin certification.
- Symtropy #7667 / #7730 — physical-model discrepancy and physical-field-to-function transfer.
- Symthaea #13506 — recycled NdFeB process/material science.
- Symthaea #13521 / #13533 — field-function inverse design and process-uncertainty-aware robust design.

## Decision

A specimen is QUALIFIED only when every required evidence link is present, scope-matched and role-correct; the as-built solver and measurement records refer to the same specimen and operating conditions; all hard requirements are safe; uncertainty is included; and the continuous-domain assessment has complete coverage.

A missing link or unresolved margin is not a pass. A clear, admissible hard-limit violation is FAIL. Unsupported measurement/model scope is NOT_EVALUABLE. Other incomplete or conflicting cases are INDETERMINATE.

## Claim ceiling

The initial tranche is a protocol and synthetic semantic benchmark. It does not claim solver execution, physical fabrication, laboratory validation, certification, product release, reduced measurement count or superiority over commercial solvers.


## Executable semantic reference

The standard-library-only Rust 2024 crate in `evaluator/` checks specimen, manufacturing, as-built geometry, material, post-magnetization, calibration, solver, observation, exact-grid comparison, blind-holdout and spatial-certificate joins. It emits QUALIFIED / FAIL / INDETERMINATE / NOT_EVALUABLE with typed blockers.

Seventeen unit tests are authored. They have not been executed: the current local environment lacks `rustc` and `cargo`, and the feature branch's exact-head CI status must be checked independently.
