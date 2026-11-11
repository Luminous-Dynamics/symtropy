# AER-01 dynamic coupling audit v0.1

**Tracking:** [#4275 — time-domain coupling stability, subcycling and energy closure](https://github.com/Luminous-Dynamics/symtropy/issues/4275)  
**Composes:** #4093 structural/aeroelastic bridge, #4112 spatial interface transfer, #4236 instrument-aware observation operator and #3237 solver interoperability.  
**Status:** reduced-order mathematical and transaction-contract oracle; no CFD/FEM solver.

## Why this exists

Aeroelastic interface transfer is not complete when a spatial mapper conserves force and moment. Dynamic partitioned FSI also needs a correct time contract, subcycling, convergent coupling, action/reaction exchange, work consistency, whole-window energy accounting and an all-or-nothing commit boundary.

preCICE documents serial/parallel and explicit/implicit coupling schemes, supports subcycling within a time window, and includes Aitken and quasi-Newton acceleration (IQN-ILS / IQN-IMVJ) for implicit coupling. The tool also emphasizes that each participant's internal convergence must be sufficiently strict relative to the coupling residual. These are the industry reference practices; this crate does not reproduce preCICE or claim equivalent solver capability.

## Implemented v0.1

### 1. Added-mass fixed-point oracle

A scalar reduced-order model uses:
- structural mass ms;
- fluid added mass ma;
- applied force F;
- exact coupled acceleration a* = F / (ms + ma);
- partitioned fixed-point update a(k+1) = (F - ma * a(k)) / ms.

With relaxation omega, the error amplification factor is 1 - omega * (1 + ma / ms). The locked corpus demonstrates:
- low added-mass ratio: unrelaxed iteration converges;
- high added-mass ratio: unrelaxed iteration diverges and publishes no acceleration;
- high added-mass ratio: a bounded relaxation recovers the closed-form solution.

This is a mathematical demonstration of added-mass sensitivity, not a physical fluid/structure time integrator.

### 2. Time-grid and synchronization contract

Each audit binds fluid and structure grids to the same integer-tick synchronization interval. Both participant grids must exactly cover that interval. Gaps, incomplete final steps, invalid intervals, stale timebase identity and temporal extrapolation are rejected.

The current v0.1 temporal policies are endpoint-hold and linear. The package records these declarations; it does not compute or validate interpolation accuracy.

### 3. Coupling convergence gate

A time window can be committed only if:
- the coupled iteration declares convergence within budget;
- the outer coupling residual meets its tolerance;
- fluid and structural inner residuals meet their tolerance;
- a declared inner/outer tolerance-separation policy is met;
- the exchange receipt binds the same window, state, timebase and spatial map;
- interface impulse, interface work and total energy audits pass.

The ratio between inner and outer tolerances is a declared policy parameter. A 100:1 separation is used in the synthetic fixture, not asserted as a universal physical rule.

### 4. Energy and action/reaction audit

Positive work is defined as energy entering a participant. In the closed two-participant test:
- fluid interface impulse plus structural interface impulse must be zero;
- fluid interface work plus structural interface work must be zero;
- combined system energy change must equal declared external work minus declared dissipation.

Internal interface work cancels in the combined-system balance. Force convergence alone cannot substitute for this energy audit.

### 5. Exactly-once committed time windows

A rejected window returns its original input-state digest as the committed state. It never promotes the proposed output-state digest. A committed sequence must be contiguous, preserve the prior output-to-next-input state chain, keep configuration and timebase identity stable, and avoid duplicate window or exchange IDs.

## Run locally

From the repository root:

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/dynamic-coupling-audit/Cargo.toml -- --check
cargo test --manifest-path tools/civil-aero-benchmark/dynamic-coupling-audit/Cargo.toml
cargo run --quiet --manifest-path tools/civil-aero-benchmark/dynamic-coupling-audit/Cargo.toml > /tmp/dynamic-coupling-audit-result.json
python3 tools/civil-aero-benchmark/dynamic-coupling-audit/verify_independent.py \
  --rust-output /tmp/dynamic-coupling-audit-result.json
~~~

The Rust program is dependency-free. The Python oracle uses only the standard library and independently recomputes the reduced-order solution, substep coverage, impulse/work balance and system energy balance.

## Claim ceiling

This package verifies a scalar added-mass fixed point and a finite time-window transaction contract. It does **not** implement CFD, FEM, Navier–Stokes, structural dynamics, flutter, buffet, modal identification, or general unsteady FSI.

A passing synthetic transaction is not evidence of physical CRM correlation, Wren 2 / Wayfarer 4 performance, structural adequacy, airworthiness, certification or permission to manufacture/test/fly.

## Next tranches

1. Add more added-mass ratios and time-grid refinement levels.
2. Add a bounded analytical oscillator with a closed-form time history to compare phase/amplitude error and timestep order.
3. Compare Aitken and IQN-ILS against the same reduced-order and then reference-participant cases; do not claim parity from the scalar relaxation case.
4. Compose receipts from #4118 spatial mapping and #4238's instrument-aware observation operator.
5. Only then attempt an admitted optional DPW-8 dynamic-FSI / JAXA Test Case 3 reproduction with legitimate solver access, source bytes, terms and timebase/synchronization metadata.
