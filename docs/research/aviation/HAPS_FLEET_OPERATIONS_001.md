# AERO-HAPS-FLEET-OPERATIONS-001 — temporal fleet service continuity

**Status:** synthetic-first deterministic operations replay v0.1  
**Parent tracker:** [AERO-HAPS-FLEET-RESILIENCE-001 #6009](https://github.com/Luminous-Dynamics/symtropy/issues/6009)  
**Static reliability owner:** [tools/haps-fleet-reliability](https://github.com/Luminous-Dynamics/symtropy/tree/research/haps-qualification-current-20261118/tools/haps-fleet-reliability)  
**Claim ceiling:** deterministic replay of supplied event traces; no empirical failure probability or service SLA.

## 1. The missing bridge

The current HAPS reliability work already has two important pieces:
- an exact static Bernoulli enumerator for independent node availability, shared common-cause events and cross-layer dependencies;
- a deterministic cyclic rotation lower-bound / schedule, with single-bay maintenance and launch/recovery capacity checks.

Neither is a time-ordered operations simulator. Static probabilities cannot tell us whether maintenance windows overlap a storm, whether a repair is followed by a recovery/reposition window, whether a launch pad is double-booked, or whether a handover cohort actually transfers.

This increment adds the missing bridge: **a chronological event replay that calculates service capacity hour by hour**, while retaining the static Bernoulli and semi-Markov reliability owner as separate models.

## 2. Machine-readable artifacts

- [haps_fleet_operations_case_v1.schema.json](schemas/haps_fleet_operations_case_v1.schema.json)
- [synthetic_haps_fleet_operations_case_v1.json](examples/synthetic_haps_fleet_operations_case_v1.json)
- Rust crate: [tools/haps-fleet-operations](../../../tools/haps-fleet-operations)
- Workflow: [HAPS Fleet Operations Replay](../../../.github/workflows/haps-fleet-operations.yml)

The case has three heterogeneous HAPS nodes, terrestrial and LEO fallback, planned maintenance, individual faults, recovery transit, common weather, a shared ground/LEO outage, group handover, and an emergency-demand weighting interval.

## 3. Replay semantics

- Evaluation window: 720 hours (30 days), in 1-hour half-open intervals.
- Node and fallback capacity contributions are multiplied by all overlapping event capacity factors.
- A shared event is represented once, names its shared-cause ID, and applies to all listed affected nodes/dependencies in the same intervals.
- Service is adequate when total HAPS plus fallback capacity is at least 2.0 declared service units.
- Degraded service is adequate service with less than 0.75 units of spare capacity over the requirement.
- Demand-weighted availability counts each hour using its declared demand weight. It is not a substitute for strict time availability or latency/SLA evidence.
- Maintenance, launch, recovery, reposition and handover pools have maximum concurrent-operation limits. Conflicts are surfaced in the result even when service capacity remains adequate.
- Handover session counts and interruption percentiles are measured separately from fleet capacity; one cannot infer the other.

This is a deterministic event trace, not a probability distribution. Event frequencies, durations, capacity factors and platform behavior are synthetic inputs.

## 4. Independent synthetic result

The 30-day fixture has 3 HAPS × 1 service unit, 0.5 units from a terrestrial gateway and 0.5 from LEO, with a 2-unit service threshold.

| Metric | Independent expected result |
|---|---:|
| Strict service-adequate hours | 708 / 720 |
| Strict availability | 98.333% |
| Demand-weighted availability | 93.651% |
| Outage episodes | 1 |
| Total / maximum contiguous outage | 12 h / 12 h |
| Degraded-but-adequate service | 10 h |
| Minimum instantaneous capacity | 1.5 units |
| Capacity deficit | 6 capacity-unit-hours |
| Maintenance/launch/recovery/handover pool concurrency | 1 / 1 / 1 / 1 (within declared limits) |
| Handover session completion | 9,850 / 10,000 = 98.5% |
| Handover p95 / maximum interruption | 4.2 s / 24 s |

The 12-hour outage occurs when all HAPS are weather-derated to 0.5 capacity each **and** the terrestrial and LEO fallback paths share a common ground/core outage. This overlaps a synthetic four-times-weighted emergency-demand period. If the ground/LEO common-cause outage is removed, the cross-layer capacity absorbs the weather derate and the trace has no service outage. If the two fallback capacities are set to zero, the two all-HAPS weather derates produce 22 outage hours.

The percentages and capacities are illustrative test inputs. They are not Sceye, SoftBank, AALTO, UAVOS or Luminous operating data.

## 4.1 Weather-diverse reserve negative control

A second [weather-diverse reserve fixture](examples/synthetic_haps_fleet_weather_diverse_reserve_case_v1.json) uses four HAPS nodes, with a reserve platform launched over a 12-hour call-up window and placed in a declared different hazard domain. During the 12-hour shared weather event and shared ground/LEO outage, the original three platforms fall to 0.4 capacity each, while the reserve remains at 1.0. Total service capacity is 2.2, above the 2.0-unit threshold: 720/720 strict hours remain adequate, with 12 degraded-but-adequate hours.

The paired Rust negative control adds the reserve to the same shared weather event. Its capacity also falls to 0.4, restoring a 12-hour service outage and a 4.8 capacity-unit-hour deficit. This demonstrates that *declared exposure diversity*, not just adding a fourth airframe, drives the result. The different hazard domain is a synthetic hypothesis and must be established using real altitude/trajectory/weather exposure and component qualification data before an independence claim.

## 4.2 Industry operating constraints and current reliability baselines

Sceye/SoftBank's 3 November 2026 agreement is a five-year fleet manufacturing, launch and operations plan—not evidence of an achieved nationwide availability SLA. An industry interview reports an ambition to scale toward at least 40 platforms, current recovered-platform teardown/test/reuse turnaround measured in months, a target to reduce turnaround to weeks, and an objective for individual vehicles to remain aloft for six months or more. These are interview-reported plans and operating context, not achieved fleet size, production rate, turnaround distribution or service guarantee. [Official MSA](https://sceye.com/press-releases/sceye-softbank-sign-agreement-for-fleet-of-stratospheric-platforms/); [Payload interview, 3 November 2026](https://payloadspace.com/exclusive-sceye-signs-five-year-deal-to-fly-haps-fleet-for-softbank/).

Current reliability research already includes semi-Markov baselines:
- Jain et al. (Reliability Engineering & System Safety, 2026) model repairable multiple-HAPS service with a k-out-of-n configuration and a Semi-Markov Process to represent non-exponential sojourns, assessing availability, reliability and MTTF: [DOI 10.1016/j.ress.2025.111450](https://doi.org/10.1016/j.ress.2025.111450).
- Mittal et al. (2024) use semi-Markov k-out-of-n for redundant rotors on a tethered HAPS. This is component-level rotor reliability, not fleet service availability: [DOI 10.1016/j.ress.2024.110419](https://doi.org/10.1016/j.ress.2024.110419).
- A DCCN programme presentation on 22 September 2026 describes a Bayesian reliability/digital-twin framework that updates failure intensity from operational observations and reassesses mission readiness for a repairable tethered HAPS with hot standby. Treat it as a research comparator, not field-validated fleet evidence: [DCCN programme entry](https://dccn.ru/papers/4514).

This deterministic replay is not a novel reliability theory and does not replace those models. It supplies chronological operations traces, resource/coverage semantics and a reproducible event layer that can later feed semi-Markov or Bayesian updating when real failure, repair, weather and maintenance telemetry exists.

## 5. Paradigm shift and research questions

The product should be evaluated as a **service system with recoverable capacity**, not as a fleet of aircraft that happen to be airborne.

The key questions become:
1. How much spare coverage capacity is required to maintain service through correlated weather and repair windows?
2. When should maintenance be scheduled relative to forecast weather, demand surges, launch/recovery resources and cross-layer fallback?
3. How much does a pre-positioned reserve vehicle reduce critical-demand outage minutes after its launch/reposition delay is charged?
4. What is the best policy for group handover—how many sessions are prepared, accepted, completed, rejected, timed out, rolled back or interrupted?
5. Which common dependencies dominate residual risk: weather, gateways, ground power, fibre/core, spectrum, satellite ground stations, command/control, launch site or maintenance capacity?
6. What is the marginal cost of each additional availability nine in deployed airframes, crews, maintenance bays, spares, energy, launch/recovery slots and network fallback?
7. Can a heterogeneous fleet reduce common-cause exposure, and are the supposedly diverse platforms actually independent in the same weather and infrastructure domain?

## 6. Matched benchmarks and target ladder

Compare:
- static k-out-of-n Bernoulli availability;
- independent-node-only (incorrectly optimistic negative control);
- shared-cause-aware static exact enumeration;
- fixed staggered maintenance;
- naïve simultaneous maintenance;
- weather-aware maintenance/rotation scheduling;
- reserve-node dispatch with explicit launch and reposition latency;
- cross-layer fallback with independent and correlated dependency cases;
- event replay plus semi-Markov repair/sojourn distributions once empirical time-to-repair data exists.

Report strict and demand-weighted availability, critical-demand outage minutes, outage episode length, p95/p99 interruption, handover completion, recovery time, spare-capacity margin, weather/common-cause exposure, resource-pool congestion, maintenance burden and cost per served hour.

Targets (not achieved results):
- 99.9% demand-weighted service availability over a 90-day synthetic campaign as a first measurable gate.
- 99.99% regional service target only after the service boundary, independence domains, maintenance/launch/recovery capacity and failover SLA are justified.
- Zero unreported maintenance/launch/recovery resource-limit violations in schedules labelled feasible.
- Zero safety-critical handover sessions accepted without a completed state transfer or explicit rollback/safe fallback.
- Report worst contiguous outage and worst demand-weighted outage window, not just annualized average availability.

## 7. Ownership and limits

- Static Bernoulli/common-cause exact enumeration remains in [HAPS Fleet Reliability](https://github.com/Luminous-Dynamics/symtropy/tree/research/haps-qualification-current-20261118/tools/haps-fleet-reliability).
- Cyclic rotation lower bounds remain a separate capacity-planning oracle.
- The new crate consumes a supplied deterministic event trace; it does not infer failure rates or fit semi-Markov distributions.
- Battery, electro-thermal, radio, flight dynamics, weather forecasting, spectrum and operating authority remain with their existing domain owners.

The evaluator is software-only. No launch, maintenance dispatch, aircraft command, flight test, airspace operation or service authorization is performed or implied.
