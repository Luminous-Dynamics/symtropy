# SAFRB v0.8 — service continuity and restoration certificate

**Status:** standalone read-only evaluator; synthetic known-answer fixture only  
**Research cut:** 24 November 2026  
**Language:** Rust 2024, dependency-free  
**Parent:** SAFRB v0.7 physical-capacity/access envelope; consumes service-boundary results from v0.6  
**Operational posture:** no optimization, dispatch, reservations, provider commitments or control-system writes

## Paradigm shift

A balanced energy schedule is not yet evidence that an essential service survived a disruption. A capacity envelope is not yet evidence that a service was restored. Resilience must be measured as a time trajectory at a named service boundary.

v0.8 turns a contiguous sequence of lower/upper delivered-energy intervals and critical-service floors into a deterministic evaluation receipt. It distinguishes:

- **Guaranteed service:** the delivered-energy lower bound meets or exceeds the critical floor.
- **Confirmed shortfall:** even the delivered-energy upper bound is below the critical floor.
- **Uncertain service:** the interval overlaps the critical floor, so continuity cannot be proven.
- **Sustained restoration:** the lower bound remains above the critical floor for a configured continuous duration.
- **Right-censored recovery:** the observation horizon ends before sustained restoration can be confirmed.

This prevents a single recovered interval, a point estimate, or a forecast from being presented as a durable restoration.

**Important service-quality boundary:** v0.8 measures delivered energy (Wh) at a named boundary against an externally defined floor. Wh alone does not establish adequate instantaneous power (W), voltage, frequency, waveform/power quality, process pressure/flow, water quality or telecom availability. Where those properties define whether the service is actually usable, a separate domain validator must provide them. v0.8 by itself supports an energy-floor statement, not a complete engineering or public-service certification.

## What the evaluator checks

1. One named service and service boundary per certificate.
2. Ordered, contiguous, non-overlapping time windows.
3. Unique window and period identities.
4. Explicit Wh units at a declared boundary, lower/upper delivery bounds and a critical service floor.
5. A full-service target that cannot fall below the critical floor.
6. Measurement or fixture evidence whose validity window covers the service window.
7. A digest reference and upstream accounting artifact ID for measured evidence; the crate checks shape only.
8. Explicitly namespaced synthetic fixtures; reported-only evidence is rejected.
9. Checked timestamp, duration, counter and energy arithmetic.
10. Sustained restoration confirmation and right-censoring when the horizon is insufficient.

The evaluator does not verify source bytes, hashes, signatures, calibration, measurement installation, physical state, upstream v0.6 calculations or issuer authority. A valid digest-shaped string is not proof that a file was hashed. Mycelix or an independent verifier must qualify upstream identities/receipts; a metrology or operator review is still required for field evidence.

## South African strategic-stock claim register

The March 2026 “eight million barrels of oil” statement and the September 2026 media-reported “6.9 million barrels / about 30 days of crude needs” statement are kept separate in [STRATEGIC_STOCK_CLAIM_REGISTER.md](research/STRATEGIC_STOCK_CLAIM_REGISTER.md). The latter remains secondary-source-attested pending the primary parliamentary answer.

## Compose with the existing Power Fabric restoration programme

SAFRB v0.8 is deliberately a **South African fuel-to-service domain profile**, not a second generic restoration planner, simulator, action schema or claim verifier. It should compose with the existing Symtropy Power Fabric contracts:

- [#6346 / draft PR #6347](https://github.com/Luminous-Dynamics/symtropy/issues/6346): planner-neutral frozen restoration case and exact scenario identity.
- [#6348 / draft PR #6350](https://github.com/Luminous-Dynamics/symtropy/issues/6348): normalized planner/action proposals.
- [#6352 / draft PR #6366](https://github.com/Luminous-Dynamics/symtropy/issues/6352): run receipt, including per-step service trajectory and unit-bearing service metrics.
- [#6369 / draft PR #6370](https://github.com/Luminous-Dynamics/symtropy/issues/6369) and [#6397 / draft PR #6401](https://github.com/Luminous-Dynamics/symtropy/issues/6397): explicit information requests, later observation results, freshness and source-dependence metadata.
- [#6402 / draft PR #6405](https://github.com/Luminous-Dynamics/symtropy/issues/6402) and [#6419](https://github.com/Luminous-Dynamics/symtropy/issues/6419): offline artifact/signature and end-to-end claim verification.

The intended integration boundary is an artifact adapter: the v0.8 result should be serialized into a canonical service-trajectory artifact with separate critical-floor/target metrics, then referenced by the generic run receipt at the relevant decision steps and final horizon. **Current implementation boundary:** v0.8 returns a deterministic Rust result struct and the demonstration binary prints CSV. It does not yet serialize canonical JSON, compute artifact digests, sign outputs, or publish Power Fabric run-receipt artifacts. A later adapter must be built against the accepted schema version and have its content/signature chain independently verified. This crate does not create its own cryptographic success claim.

Those Power Fabric issues and draft PRs are separate works in progress; this v0.8 crate currently has no compile-time dependency on them. They must be integrated only against their accepted schema versions and exact artifact contracts.

## Power Fabric electricity-service interop

The next integration work item is [SAFRB-SERVICE-TRAJECTORY-INTEROP-001 #7338](https://github.com/Luminous-Dynamics/symtropy/issues/7338). Its mapping contract is documented in [POWER_FABRIC_SERVICE_TRAJECTORY_INTEROP.md](research/POWER_FABRIC_SERVICE_TRAJECTORY_INTEROP.md).

The existing [Power Fabric service trajectory auditor #6449 / PR #6450](https://github.com/Luminous-Dynamics/symtropy/pull/6450) accepts one delivered kWh value per frozen load/interval. Its uncertainty field is metadata; the current weighted scorer sums the delivered point value and does not independently calculate the v0.8 lower/upper interval. The adapter must therefore preserve the v0.8 interval and critical floor in a separate bound certificate, while projecting only the conservative lower bound into the generic score. A zero lower bound with a positive upper bound remains unknown/inconclusive.

This adapter is **not implemented yet**. It must wait for accepted case/run/service-trajectory schema versions and the independent artifact/claim-verification chain; no compile-time dependency or end-to-end integration is claimed.

## Additional temporal-assurance suite

The combined v0.8 crate now has **26 Rust tests** (20 original evaluator tests and six independent temporal-assurance tests). The temporal suite checks 3,276 exhaustive oracle comparisons, 5,151 interval-widening combinations, 20,604 delivery-improvement combinations, equivalent time partitioning, clock translation, exact restoration thresholds and repeated outages.

See [TEMPORAL_ASSURANCE_SUITE.md](research/TEMPORAL_ASSURANCE_SUITE.md), [SAFRB-TEMPORAL-ASSURANCE-001](https://github.com/Luminous-Dynamics/symtropy/issues/7336) and [draft PR #7337](https://github.com/Luminous-Dynamics/symtropy/pull/7337).

## Known-answer fixture

The fixture uses four synthetic one-hour periods, a 50 Wh critical floor and an 80 Wh target. Delivered energy is bounded as:

| Period | Delivered interval | Outcome |
|---|---:|---|
| 1 | 60–60 Wh | Guaranteed |
| 2 | 10–15 Wh | Confirmed critical shortfall |
| 3 | 60–65 Wh | Recovery candidate |
| 4 | 60–65 Wh | Sustained recovery confirmed |

The evaluator therefore reports:
- one confirmed shortfall period;
- 35–40 Wh of critical-floor shortfall;
- 3 of 4 hours guaranteed at or above the critical floor;
- restoration beginning 3,600 seconds after the episode begins;
- restoration confirmed after a further 7,200 seconds of sustained evidence.

These are synthetic known answers, not measured port performance.

## Run

~~~~sh
cargo fmt --manifest-path benchmarks/fuel-resilience/south-africa/v0.8/Cargo.toml -- --check
cargo test --manifest-path benchmarks/fuel-resilience/south-africa/v0.8/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/fuel-resilience/south-africa/v0.8/Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path benchmarks/fuel-resilience/south-africa/v0.8/Cargo.toml --release --locked
~~~~

## Claim ceiling

A passing unit test establishes deterministic behavior over the supplied inputs and evidence contract. It does not establish actual service continuity, field restoration time, fuel availability, operator readiness, power-system stability, legal rights, or system resilience. No certificate should be described as independently verified unless a separate verifier has actually checked its referenced source bytes, signatures, authority, calibration and physical measurement process.


## Next assurance and benchmark increments

The next research layer treats usable service as a non-compensatory vector rather than an energy-only scalar. It separates delivered Wh, instantaneous/sustained kW, voltage, frequency, reactive power, waveform quality, downstream process output and evidence qualification. See [SERVICE_ASSURANCE_VECTOR_AND_BENCHMARK_PROTOCOL.md](research/SERVICE_ASSURANCE_VECTOR_AND_BENCHMARK_PROTOCOL.md) and the stacked [draft PR #7375](https://github.com/Luminous-Dynamics/symtropy/pull/7375).

The next domain dataset workstream is [SAFRB-FUEL-TO-SERVICE-CORPUS-001 #7377](https://github.com/Luminous-Dynamics/symtropy/issues/7377). It begins with synthetic, abstract-ID fuel-chain cases and tests gross stock through usable/access-controlled/deliverable inventory, conversion, generation, network constraints and beneficiary service. It is not a claim about actual South African stock or site performance.

These are research specifications. They do not add an adapter implementation, field data or additional qualification to the v0.8 evaluator.
