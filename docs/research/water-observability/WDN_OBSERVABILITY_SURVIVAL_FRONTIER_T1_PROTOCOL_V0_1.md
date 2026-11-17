# WDN observability survival frontier T1 protocol v0.1

**Tracking issue:** #5893 WATER-OBSERVABILITY-SURVIVAL-FRONTIER-001  
**Upstream qualification:** #5045 / #5047 failure-domain observability  
**Downstream optimizer:** #4006 Pareto sensor placement  
**Date:** 17 November 2026  
**Status:** synthetic known-answer contract only.

## 1. Purpose and ownership

This evaluator takes one declared sensor layout and measures its *combinatorial survivability* under all channel and shared-dependency failures of order 0, 1 and 2.

It does not generate layouts or choose a winner. Candidate generation, cost/maintenance objectives and Pareto selection remain with #4006. The T1 evaluator gives that optimizer a robustness vector, minimum cut sets and a directly observable survival frontier.

## 2. Failure-component model

For every baseline-eligible sensor, the failure universe contains:
- one channel component identified by its sensor ID;
- one component for each unique (domain type, domain ID) pair among its six dependency domains.

A dependency instance shared by several sensors is one common-cause component and is failed once. It must not be counted as multiple independent failures. In particular, sensors using the same measurement principle must carry the same measurement-principle domain ID; assigning a different ID to each device would falsely manufacture independent modality paths.

A sensor is excluded from the active failure-component universe before enumeration if it is already failed, stale/un-calibrated, incorrectly bound or time-invalid. Its presence cannot dilute the denominator with no-op failure events.

For a declared failure set F, a sensor survives only when its channel component and all six dependency components are absent from F. A critical state is directly observable when at least one surviving sensor is directly bound to that state.

The v0.1 input contains exactly four critical pressure states. The enumeration order is fixed to 2. Component and combination budgets are checked before enumeration.

## 3. Combinatorial counts, not probabilities

For N active failure components, v0.1 enumerates:
- order 0: one no-failure combination;
- order 1: N single-component failures;
- order 2: N(N-1)/2 two-component failures.

For each state and each order, the evaluator reports:
- total combinations;
- combinations retaining at least one direct channel;
- combinations rendering the state unobservable;
- exact reduced survival fraction.

These fractions are unweighted fractions of enumerated failure sets. They are **not** failure probabilities, availability, reliability, MTBF, risk or expected service. Equal enumeration weight is a counting convention only.

## 4. Minimal cut sets

A cut set is a set of failed components that removes all eligible direct channels for one critical state. The output retains only minimal cut sets within the declared maximum order: no proper subset of a reported cut set already makes that state unobservable.

For each state, the receipt includes:
- the first failure order with at least one unobservable combination;
- all minimal cut sets found through order 2;
- the per-order survival fractions.

A missing cut set above order 2 is not evidence of unbreakability. It means only that no cut was found within the declared enumeration order.

## 5. Monotonicity invariant

Adding failed components cannot restore a direct measurement. The evaluator checks every enumerated failure-set inclusion A ⊆ B. If a state is unobservable under A but observable under B, the monotonicity check fails.

This catches incorrect shared-domain handling and accidental sensor reactivation.

## 6. Frozen 12-layout challenge pack

1. Independent single-channel baseline.
2. Two channels with shared power supply.
3. Independent pressure/acoustic backup.
4. Shared communications gateway.
5. Shared physical process interface.
6. Site-wide clock domain.
7. High-fan-out calibration domain.
8. Distinct measurement principles and independent dependencies.
9. Stale channel excluded before enumeration.
10. Invalidly bound channel excluded before enumeration.
11. Two-device minimum cut.
12. Sensor input-order permutation invariance.

The 12-layout pack covers 25–35 active failure components per layout; the oracle freezes exact order-0/1/2 counts, per-state fractions, cut sets, dependency fan-out and monotonicity.

## 7. Reproducible receipt and tests

The Rust evaluator reuses the existing water-service package and pinned serde_json / sha2 dependencies. The exact-revision receipt binds:
- input layouts;
- input Draft 2020-12 schema;
- independent frozen oracle;
- Cargo.toml and Cargo.lock;
- evaluator implementation;
- exact checkout revision.

Tests cover canonical oracle matching, shared-domain fan-out, singleton common-cause cuts, independent redundant channels requiring two failures, site-wide clock cut sets, pre-invalid sensor exclusion, input permutation, cut-set minimality, monotonicity, duplicate IDs/domains, unknown states, component and combination budget rejection, invalid maximum order, and altered-oracle detection.

Receipt qualification remains NotRun. The implementation is read-only; it adds no hydraulic solver, sensor-placement optimizer, SCADA write path or actuator control.

## 8. Claim ceiling

A passing T1 receipt establishes only deterministic combinatorial results for the declared synthetic layouts. It does not assign real-world failure probabilities, measure service availability, qualify physical sensors, demonstrate observability of Johannesburg or another live network, qualify controller safety or establish leak-detection performance.
