# Symtropy Natural Hazards Contract

This crate is the shared metadata and qualification boundary for a multi-hazard simulation program. It is not itself a physical hazard solver.

## Design rule

A model's capability is defined by the exact hazard kind, spatial and temporal envelope, required inputs, declared outputs and evidence class. An invariant-test pass does not imply held-out predictive skill. Unsupported and unavailable results must remain explicit.

## Initial API

- `HazardKind::family()` classifies catalogue entries but does not imply that Symtropy can simulate them.
- `HazardModelProfile::validate()` checks card structure.
- `HazardModelProfile::check_scope()` rejects unsupported hazards, out-of-range regimes and missing required inputs.
- `HazardModelProfile::qualification_decision()` requires a PASS for the exact requested evidence class.
- `HazardRunReceipt` binds scenario, source and input/output digests to an execution status.
- `BudgetResidual` checks a declared domain-specific ledger only. No universal conservation law is imposed on every hazard family.

## Example evidence policy

A shallow-water model may pass a closed-basin water-volume invariant while having no held-out flood-event validation. The API therefore returns `MissingEvidence(HeldOutObservation)` for that stronger claim. It cannot silently promote one kind of evidence into another.

## Current claim ceiling

The current implementation qualifies the contract behavior in unit tests only. It does not implement flood, earthquake, wildfire, tsunami, impact, volcanic, climate, biological or space-weather solvers, and it makes no operational forecast claims.
