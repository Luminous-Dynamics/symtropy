# Composition-level co-simulation coupling benchmark (v0.1)

A dependency-free Rust reference for **synthetic known-truth** assessment of coupled digital-twin interfaces. It separates energy conservation, trajectory fidelity, and exchange-contract validity.

## Why this exists

A file format or API compatibility check does not establish a physically credible composition. FMI 3.0 standardizes model-exchange, co-simulation, and scheduled-execution interfaces and includes clocks, intermediate updates, and early return. The importer still owns the co-simulation algorithm. Consequently, communication semantics and numerical coupling behavior must be evaluated explicitly.

Energy-conservation-based co-simulation (ECCO) and power-bond residuals are established prior art. This reference does not claim to invent those methods. It provides a small, reproducible manufacturing-assurance benchmark that can become a known-truth negative-control suite for coupled models.

## Build and run

```sh
rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/composition_coupling.rs -o /tmp/mfg-coupling-tests
/tmp/mfg-coupling-tests
rustc --edition=2021 -D warnings tools/manufacturing-assurance/composition_coupling.rs -o /tmp/mfg-coupling
/tmp/mfg-coupling
```

Or run the full assurance suite:

```sh
bash tools/manufacturing-assurance/run-tests.sh
```

## Known-truth model

Two thermal capacities exchange heat through a conductance, with no external source:

- Body A heat capacity: 2 J/K
- Body B heat capacity: 5 J/K
- Conductance: 0.8 W/K
- Initial temperatures: 100 K and 20 K
- Duration: 2 seconds

The exact solution is available in `fixtures/analytic-composition-coupling.tsv`. The capacity-weighted mean temperature is invariant; the temperature difference decays exponentially with rate (G(1/C_A + 1/C_B)).

The benchmark compares:
- **Shared-flux Jacobi:** computes one transfer from a common start-of-step snapshot and applies equal-and-opposite energy changes.
- **Independent ports, one-step stale:** each port computes its own transfer using the peer's previous published value. This is a deliberate asynchronous negative control.

The shared-flux algorithm should conserve total energy to floating-point tolerance, yet still have nonzero trajectory error due to explicit time discretization. The stale independent-port control should create a measurable energy-balance residual. These two metrics are deliberately not collapsed into one score.

## Exchange contract

A receipt pair carries:
- stable exchange ID;
- component and peer IDs;
- local timestamp;
- signed component energy change;
- unit.

The validator rejects mismatched exchange IDs, non-reciprocal component identities, unsupported energy units, non-finite quantities, clock skew above the supplied bound, and non-antisymmetric transfer amounts. This is a local integrity check, not a cryptographic signature, authenticated clock, or trusted measurement.

## Frozen tests

Sixteen unit tests cover:
- fixture agreement with the closed-form solution;
- shared-flux conservation with nonzero trajectory error;
- stale-port energy leakage;
- first-order convergence under step halving;
- valid exchange acceptance;
- mismatched IDs and component pairs;
- wrong sign and unequal transfer magnitude;
- excessive clock skew and incompatible units;
- non-finite energy and invalid tolerances;
- invalid communication steps;
- deterministic report replay;
- the claim boundary that a passing transfer contract is not proof of numerical accuracy.

## Claim ceiling

This is synthetic known-truth evidence only. It is not:
- an FMI conformance suite;
- a validated plant or factory twin;
- a certified co-simulation master algorithm;
- a general proof of passivity or stability;
- evidence of real-time performance;
- a production release or machine actuation authorization.

The one-step-stale control is intentionally simplified and should not be represented as a characterization of any named vendor's solver.

## Next benchmark stages

1. Expand to three or more domains and multiple conservation laws (mass, charge, momentum, heat).
2. Add stiff and hybrid/event-driven systems, variable communication steps, rollback/retry, extrapolation, and clocked partitions.
3. Add correlation-aware uncertainty propagation and validity-domain intersection.
4. Import independently authored FMI 3.0 FMUs and run cross-master/cross-vendor experiments.
5. Validate against public measured manufacturing data before claiming field relevance.
