# Functional Recovery Curve Prototype v0.2

Status: experimental reference metrics only. This is a first step toward issue #2193; it is not a complete recovery simulator.

## Why change the target?

Asset damage state does not directly tell an operator or community when useful service returns. NIST's functional-recovery work frames infrastructure around restoration of services to users, and its SP 1310/1311 set out an initial framework for service-level targets plus both asset and organizational actions. A 2025 tunnel-recovery study models recovery as damage-level-dependent repair tasks, while noting the dependence of restoration models on assumptions and expert-derived data. Those frameworks motivate measuring a time-dependent safe-service curve, not only component damage or time-to-repair.

Research anchors:

- NIST SP 1310, *Initial Framework to Design Lifeline Infrastructure for Post-Earthquake Functional Recovery, Volume 1* (2024): https://doi.org/10.6028/NIST.SP.1310
- NIST SP 1311, *Initial Framework to Design Lifeline Infrastructure for Post-Earthquake Functional Recovery, Volume 2* (2024): https://doi.org/10.6028/NIST.SP.1311
- NIST, *Functional Recovery Performance Targets Workshop Report* (published 2025): https://www.nist.gov/publications/functional-recovery-performance-targets-workshop-report
- *Resilience Models for Tunnel Recovery After Earthquakes* (2025): https://doi.org/10.1016/j.eng.2025.06.028
- FHWA, tunnel seismic screening/retrofit guidance: https://www.fhwa.dot.gov/publications/research/infrastructure/pavements/ltpp/05067/

These works are methodological comparators, not proof that this Rust module matches their numerical results. Direct numerical comparison needs a shared scenario definition, units, assumptions, and reproducible source inputs.

## Contract

The input is a scenario with an event class, service basis (passenger trips or freight tonnes), evidence class and references, three required gates, and time-ordered samples. Each sample supplies:

- elapsed time after the event, in hours;
- demand rate, in the scenario's service units per hour;
- fraction of demand safely delivered, in [0, 1].

The final sample defines the declared evaluation horizon. The evaluator assumes the supplied safe-service curve is already computed or observed elsewhere. It does not generate the curve from damage states.

### Reported values

- **T50/T90/T95:** first threshold crossing after which the sampled curve remains at or above that threshold for all later samples. Interpolation is linear between the crossing samples. This “sustained” definition prevents an earlier optimistic crossing from surviving a later sampled aftershock-related service loss.
- **Cumulative unmet service:** trapezoidal integration of the pointwise unmet-service rate between samples, reported in the contract's service unit (passenger trips or freight tonnes). Demand rate and unmet rate are each interpolated linearly between sample endpoints.
- **Delivered fraction over horizon:** delivered service divided by demanded service over the declared horizon.
- **Right-censoring:** if T90 or T95 is not sustained before the last sample, the metric is `None`. It is not imputed as the horizon end or as infinity.

A threshold that is already met at hour zero and is never lost is reported as zero hours. A later drop below the threshold invalidates that early time; a later sustained crossing can establish a new recovery time.

## Hard-stop behavior

All three gates must be explicitly `Satisfied` before metrics are produced: post-event inspection, emergency egress, and assessment of critical dependencies for the declared scenario. Failed gates return `INELIGIBLE`; unknown or `NotApplicable` required gates return `NEEDS_EVIDENCE`. Both emit no rankable metrics.

A satisfied gate is an input assertion, not independent verification. The gate for critical dependencies is only a required completeness check; this module does **not** compute dependency propagation, common-cause failure, or spare-path capacity. Service samples must not be interpreted as real safe operations until the underlying model, field evidence, and responsible authority decisions have been validated outside this crate.

## Deterministic fixture and exact hand check

The test curve has 100 service units/hour and safe-service fractions:

| Time (h) | Safe service fraction | Unmet rate (units/h) |
|---:|---:|---:|
| 0 | 0.00 | 100 |
| 10 | 0.50 | 50 |
| 20 | 0.90 | 10 |
| 30 | 0.95 | 5 |

Under piecewise-linear interpolation and trapezoidal integration:

- Demand over 30 hours = 3,000 service units.
- Unmet service = (100 + 50)/2 × 10 + (50 + 10)/2 × 10 + (10 + 5)/2 × 10 = 1,125 service units.
- Delivered service = 1,875 units; horizon delivered fraction = 0.625.
- T50 = 10 h; T90 = 20 h; T95 = 30 h.

A second fixture forces service to fall from 80% to 30% before recovering, verifying that the evaluator rejects the early crossing and reports the later sustainable crossing instead. Other tests cover right-censoring, unresolved inspection, failed egress, malformed curves, and missing references for declared measured evidence.

## Benchmark ladder and targets

1. **Contract correctness:** hand calculations, boundary values, monotonic and non-monotonic curves, missing horizon recovery, malformed records, and numerical overflow.
2. **Safety negatives:** zero false-safe outputs in seeded hard-stop tests. This is only a finite-suite software acceptance criterion, not evidence of zero real-world risk.
3. **Scenario completeness:** the current branch now adds a bounded dependency-capacity kernel. Extend it with repair precedence and constrained crews, spatially correlated failures, and coupled infrastructure outage scenarios.
4. **Model validity:** reproduce a published scenario only when inputs and model assumptions can be reconstructed, then test on held-out experimental or historical cases. Separate synthetic, sourced, measured, and independently reviewed inputs.
5. **Industry comparison:** compare recovery curves, T50/T90/T95, cumulative unmet demand, censoring, and scenario coverage against traceable published frameworks and cases with a matched service contract. Do not claim “better than SOTA” until data, thresholds, uncertainty, and independent evaluation are comparable.

## Common-cause dependency capacity prototype

The `dependencies` module declares service paths, their nominal capacities (in the scenario's units per hour), and the required dependencies for each path. Each dependency has an explicit state. This models an important resilience distinction: two routes are not independent if both require the same traction-power feed, control network, portal, drainage outlet, or other component that can fail from the same event.

A deterministic fixture contains two nominally separate rail routes with capacities 60 and 50 service units/hour that both require `traction-power`, plus a surface backup path with capacity 40. When the shared traction-power dependency is `Failed`, the two rail paths are unavailable and only 40 of the 100 units/hour demand is credited. The expected safe-capacity lower bound is 40%, not the nominal 150% capacity suggested by adding all route capacities.

If a dependency is Unknown, NotApplicable, or missing:
- Its path contributes no safe-capacity credit.
- If remaining known-safe paths cannot meet demand and unresolved paths could change that result, the evaluator returns `NEEDS_EVIDENCE` with no metrics.
- If known-safe capacity already meets demand, it may return a `CONSERVATIVE_LOWER_BOUND` report that explicitly lists unresolved paths.
- Failed dependencies make affected paths unavailable. Output path lists are sorted for deterministic reports.

This module intentionally does not compute hazard-driven dependency failures, probability of common causes, network flow/assignment, demand elasticity, route travel times, egress/inspection authority, or repair actions. The set of dependency states and each path capacity remain supplied inputs; the evaluator cannot authenticate their evidence. Scenarios should eventually be generated from a declared dependency graph and correlated hazard model, then validated against observed incidents or published test cases. Never interpret the synthetic 40% example as a real corridor prediction.

## Exclusions

This prototype is not a tunnel/lining structural model; ground-motion, fragility, nonlinear ground-structure interaction, active-fault displacement, liquefaction constitutive behavior, portal stability, water ingress physics, or probabilistic seismic-hazard model. It does not authenticate data provenance, predict repair durations, schedule repair crews, calculate network flows, certify a tunnel, determine a safe re-entry time, or authorize construction. Real projects require licensed local professionals, current jurisdictional requirements, authoritative site evidence, and independent review.
