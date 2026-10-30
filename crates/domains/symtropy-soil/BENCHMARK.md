# Soil Water Kernel Benchmark and Conformance Protocol

## Scope

This v0.1 panel checks water-accounting identities, explicit unit conversions, invalid-input rejection, declared storage bounds, and deterministic replay. It does not simulate soil hydraulic processes and is not a comparison with a soil model.

All fixtures are synthetic and hand-constructed. They are not field measurements, soil observations, agronomic recommendations, or evidence of amendment efficacy.

## Reproduction

From the repository root:

    cargo test --locked -p symtropy-soil

Record the exact git commit, Rust compiler version, target triple, operating system, and command exit status with any published result. Do not describe queued CI as a pass.

## Frozen reference cases

| Case | Inputs | Expected outcome |
|---|---|---|
| Zero flux | 40 mm start storage; all fluxes 0 | 40 mm end storage; zero residual |
| Inflow | 10 mm storage; 8 mm precipitation; 4 mm irrigation | 22 mm end storage |
| Outflow | 50 mm storage; 10 mm precipitation; 5 mm ET; 4 mm runoff; 11 mm drainage | 40 mm end storage |
| Unit conversion | 1 mm over 100 m² | 0.1 m³ |
| Empty store | 0 mm start storage; zero flux | 0 mm end storage |
| Impossible overdraw | 3 mm storage; requested outflow 4 mm | Explicit StorageOverdraw error |
| Capacity boundary | 95 mm storage; 10 mm precipitation; 100 mm capacity | Explicit StorageCapacityExceeded error |
| Non-finite input | NaN precipitation | Explicit NonFiniteValue error |
| Negative input | Negative precipitation | Explicit NegativeValue error |
| Replay | Frozen 64-step synthetic schedule | Structurally and bitwise equal reports across identical replays |

## Required observations

For every successful step, retain:

- start and end storage (mm equivalent);
- each supplied flux and its interval semantics;
- total inflow and outflow (mm equivalent and m³);
- balance residual in mm;
- the scale-aware residual tolerance;
- control area and timestep duration.

The balance residual is computed from the accounting identity. For a well-conditioned case, the acceptance threshold is:

    tolerance_mm = 64 * f64::EPSILON * max(sum_of_absolute_balance_terms_mm, 1)

There is no tolerance that silently authorizes negative storage or capacity overflow. Such cases are typed errors instead.

## Promotion gates

1. The crate compiles and all locked tests pass on the exact PR head.
2. The fixed analytical fixtures and unit conversions remain unchanged.
3. Invalid and overdraw inputs fail closed; no hidden clamp or synthetic outflow is introduced.
4. Replay is repeatable for the same compiler/build and identical input sequence.
5. Future flux generators receive separate validation and uncertainty treatment before they can be described as predictive.
6. External model comparisons use the same control-volume area, units, timestep, boundary conditions, parameterization, and compatible observables.

No speed, physical-fidelity, field efficacy, or state-of-the-art claim is made by this v0.1 kernel. Performance benchmarking should be added only if throughput becomes a relevant bottleneck; correctness and conservation remain the first gates.
