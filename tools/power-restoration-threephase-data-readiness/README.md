# Power restoration native three-phase data readiness v0.1

**Owner:** Symtropy #6544  
**Programme:** Power Fabric #6320 / #5241  
**Benchmark case contract:** #6347  
**FLARE reproduction/qualification gap:** #6363

## Purpose

This crate audits whether a frozen benchmark case has complete, native unbalanced three-phase, short-circuit and protection inputs suitable for hand-off to an independent domain solver.

A benchmark case containing scalar balanced R/X/ampacity values or a positive-sequence equivalent is not promoted to three-phase readiness. Missing native data produces deterministic, object-specific blockers.

## Qualification scopes

The report has three independent but ordered gates:

1. **Three-phase load-flow inputs:** phase presence, neutral/grounding model, full primitive R/X conductor matrices, per-conductor ampacity, phase-resolved load P/Q, source phase-voltage phasors, base values and provenance.
2. **Short-circuit inputs:** complete Z1/Z2/Z0 sequence impedances for branches, transformers and sources; source short-circuit strength/XR; explicit fault scenarios at every declared fault location for SLG, LL, DLG and three-phase fault types.
3. **Protection-coordination inputs:** protection-device and CT/VT settings, phase/ground pickup, interruption rating, phase and ground time-current curves, coordination group and protection-zone coverage. A branch must have a declared device or a separately evidenced approved exemption.

Short-circuit readiness depends on complete three-phase load-flow inputs; protection readiness depends on both prior scopes. Missing or invalid fields appear as sorted blockers with scope, object kind/ID, field path and reason.

## Native electrical records

- Nodes declare A/B/C, neutral and protective-earth conductors as applicable, grounding type, neutral continuity and grounding impedance.
- Branches carry full primitive resistance/reactance matrices in an explicit conductor order. v0.1 requires square, symmetric matrices with a positive-definite resistance matrix. Unit is explicit: ohm/km for lines/cables or ohm on high-side base for transformer-classified records.
- Sequence records explicitly name positive, negative and zero sequence impedance in ohms. Equal Z1 and Z2 values are not rejected because they can be physically legitimate; the auditor refuses absent sequence records instead of manufacturing them from Z1.
- Transformers carry high/low winding connections, vector group/clock number, nominal voltages, rating, zero-sequence transfer behavior and neutral grounding.
- Loads carry per-phase P/Q values that must reconcile to the frozen case total within the declared numeric tolerance.
- Sources carry A/B/C voltage phasors, sequence impedance, source short-circuit MVA and X/R.
- Fault scenarios require every declared fault location × four fault types with explicit phases, impedance and contributing sources.
- Protection devices carry CT/VT ratios, pickup settings, breaker interruption rating, phase/ground curves and coordinated protected zones.

All evidence references require exact lower-case SHA-256 values and a producer/profile identity, and every evidence reference must be present in the package source-artifact inventory. This crate validates the declared URI/digest inventory; a separate bundle verifier must authenticate the actual bytes.

## Run

    cargo fmt --manifest-path tools/power-restoration-threephase-data-readiness/Cargo.toml --package power-restoration-threephase-data-readiness -- --check
    cargo clippy --locked --manifest-path tools/power-restoration-threephase-data-readiness/Cargo.toml --package power-restoration-threephase-data-readiness --all-targets -- -D warnings
    cargo test --locked --manifest-path tools/power-restoration-threephase-data-readiness/Cargo.toml --package power-restoration-threephase-data-readiness --all-targets
    cargo run --locked --manifest-path tools/power-restoration-threephase-data-readiness/Cargo.toml --bin power-restoration-threephase-data-readiness -- threephase-input-package.json benchmark-case.json

Exit codes:
- 0: all three input-readiness scopes are ready for independent domain solver hand-off;
- 3: package parsed and audit completed, but one or more input-readiness blockers remain;
- 1: malformed JSON or invalid case/package identity/canonicalization;
- 2: report serialization failure.

## Tests

20 authored Rust tests cover complete-input handoff, topology-only non-promotion, missing sequence, absent primitive matrix, non-passive resistance matrix, phase-load sum mismatch, fault-type coverage, protection curve/coverage, transformer vector-group requirement, evidence digest/inventory mismatch, duplicate object IDs, invalid matrix symmetry, grounding, base-case digest mismatch, missing source short-circuit data and missing protection CT/interruption settings.

Tests are authored; they are not considered passed until exact-head Rust CI executes.

## Claim boundary

The ready_for_independent_domain_solver result means only that the declared native input records are structurally complete and mutually consistent under this v0.1 contract. It is **not** a solved or converged three-phase AC load flow, short-circuit calculation, protection coordination result, transient study, switching safety approval, HIL result, commissioning acceptance or field-restoration certificate.

A solver must emit its own result artifact tied to the exact package and case digests, software/model revision, solver options and independent validator. A solver pass is a separate qualification tier.
