# Magnetic recovery passport evaluator

**Programme:** MAG-REPM-RECOVERY-PASSPORT-001  
**Issue:** #7778  
**Status:** standalone Rust 2024 evidence-contract reference; synthetic mass-flow controls only

## Purpose

Bind an identified end-of-life magnet/component and its material/process history to a scoped recovery-route eligibility decision, route-stage outputs, recovered-product quality and downstream remanufacturing evidence.

This is not a recovery-process operating manual, chemistry recipe, LCA/TEA engine or plant-control system.

## Core semantics

- Feedstock family and product form remain explicit (NdFeB/SmCo/SmFeN/ferrite; polymer-bonded/sintered/powder/hybrid).
- Route eligibility is a separate, digest-bound decision for the exact feedstock lot and proposed route. The evaluator does not infer compatibility from a generic material label.
- Every recovery stage binds one input lot and mass measurement to uniquely identified recovered products and residual outputs.
- Mass-balance closure is evaluated as an interval. Definite unaccounted input mass, output mass excess and uncertainty-overlapping outcomes are distinguished.
- Missing or ambiguous flow records cannot be called recovered.
- Recovered products need form-specific composition/quality evidence. Magnet-feedstock reuse also requires magnetic characterization.
- Circularity/use claims require an exact downstream material lot, manufacturing genealogy and as-built functional acceptance receipt.
- This v0.1 does not compute a life-cycle impact or population recovery rate.

## Read first

- [Recovery passport protocol](../MAG_REPM_RECOVERY_PASSPORT_2026-11-27.md)
- [Benchmark plan](benchmark-plan.toml)
- [Schema](evaluator/recovery-passport-v0.1.schema.json)
- [Synthetic known-answer fixture](evaluator/fixtures/synthetic-recovery-passport-v0.1.json)
- [Recovery-route source register](fixtures/recovery-route-source-register-v0.1.json)
- [Rust evaluator](evaluator/src/lib.rs)

## Claim boundary

The evaluator validates identity, declared scope, route-eligibility receipts, event sequencing, mass-flow identities and supplied quality/acceptance evidence. The v0.1 implementation computes total mass-balance intervals only; element-specific balance is a later method-scoped assay adapter. It does not authenticate evidence producers, validate laboratory methods, establish process safety, calculate magnetic fields, perform LCA/TEA, or authorize physical recovery operations.

## Exact evidence joins

- Source-lot feedstock mass and recovery-stage input mass each have a distinct digest-bound weighing record.
- Their uncertainty intervals must overlap; point-value equality is not required. Disjoint intervals remain unresolved.
- Recovery outputs have unique product IDs and method-scoped composition, quality or residue-disposition evidence according to product form.
- A closed-loop use claim binds a recovered magnet-feedstock/alloy output to a positive digest-bound consumed-mass record, a new material lot, downstream genealogy and the as-built functional acceptance receipt.
- The consumed mass cannot exceed the identified recovered output; residues and elemental products are not magnet-feedstock reuse.

## Validation inventory

Twenty-eight Rust unit tests are authored. The three-platform workflow is in `.github/workflows/magnetic-recovery-passport.yml`. The current branch head still requires a completed exact-head run; no compile, Clippy or test pass is claimed until the runner reports it.
