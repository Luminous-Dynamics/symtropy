# Symtropy Natural Hazards Contract

This crate provides shared metadata, qualification gates, and baseline probabilistic-verification primitives for a multi-hazard simulation program. It is not itself a physical hazard solver or an operational warning system.

## Design rule

A model's capability is defined by the exact hazard kind, spatial and temporal envelope, required inputs, declared outputs and evidence class. An invariant-test pass does not imply held-out predictive skill. Unsupported and unavailable results must remain explicit.

## Qualification API

- `HazardKind::family()` classifies catalogue entries but does not imply that Symtropy can simulate them.
- `HazardModelProfile::validate()` checks card structure.
- `HazardModelProfile::check_scope()` rejects unsupported hazards, out-of-range regimes and missing required inputs.
- `HazardModelProfile::qualification_decision()` requires a PASS for the exact requested evidence class.
- `HazardRunReceipt` binds scenario, source and input/output digests to an execution status.
- `BudgetResidual` checks a declared domain-specific ledger only. No universal conservation law is imposed on every hazard family.

## Probabilistic scoring API

The `scoring` module currently implements:

- `brier_score` for binary event-probability forecasts;
- `reliability_bins` for reliability/calibration diagnostics, preserving empty bins as unknown rather than zero;
- `ensemble_crps` for scalar observations against equally weighted finite ensembles;
- `brier_skill_score` versus an explicitly declared reference score.

These functions validate basic input domains and make no assumptions about a hazard's units, geometry, observation window or forecast lead time. Those are benchmark metadata and must be declared by the calling evaluation harness. A good proper score does not prove the underlying physical model is valid outside the benchmarked regime.

## Manifest qualification

An integration test loads `docs/research/natural-hazards/benchmark-manifest-v0.1.json` and checks schema identity, unique hazard IDs, declared candidate outputs, allowed coverage states, reference link shape and traceable related work. This validates inventory structure only; it does not validate the scientific content or availability/license of each external dataset.

## Example evidence policy

A shallow-water model may pass a closed-basin water-volume invariant while having no held-out flood-event validation. The API therefore returns `MissingEvidence(HeldOutObservation)` for that stronger claim. It cannot silently promote one kind of evidence into another.

## Current claim ceiling

The unit tests qualify contract and score-function behavior on controlled examples only. They do not implement flood, earthquake, wildfire, tsunami, impact, volcanic, climate, biological or space-weather solvers, and make no operational forecast claims.
