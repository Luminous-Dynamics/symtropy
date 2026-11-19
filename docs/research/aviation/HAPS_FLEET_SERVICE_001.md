# AERO-HAPS-FLEET-SERVICE-001 — fleet rotation and persistent service availability

**Status:** synthetic event-trace specification v0.1  
**Tracker:** [#6197](https://github.com/Luminous-Dynamics/symtropy/issues/6197)  
**Claim ceiling:** deterministic service-availability arithmetic; no empirical HAPS reliability rates.

## 1. The industry is moving from a vehicle demonstration to an operating system

Sceye ST1 completed a 30-day trans-Pacific flight, then delivered a limited communications trial over Japan. SoftBank's October 27, 2026 briefing states that recovered-aircraft hardware analysis was still underway, Japan-based storage/operation facilities were planned within two years, and the commercial service ambition was from 2027. SoftBank also explicitly says regional disaster response at scale would require multiple HAPS. This is evidence that maintenance, local operations and fleet scale are now operational design issues, but it does **not** publish MTBF, MTTR, fleet availability or a commercial service SLA. [SoftBank operational outlook, 27 October 2026](https://www.softbank.jp/en/sbnews/entry/20261027_01).

Sceye's November 3, 2026 five-year MSA with SoftBank covers manufacturing, launch and operation of a fleet targeting nationwide Japan coverage. It represents a move toward deployment, not proof that a nationwide fleet is already operational. [Sceye / SoftBank MSA](https://sceye.com/press-releases/sceye-softbank-sign-agreement-for-fleet-of-stratospheric-platforms/).

### Reliability SOTA

- Jain et al., *Semi-Markov modeling approach for the reliability analysis of a multiple high altitude platform station system*, Reliability Engineering & System Safety 265 (January 2026), model repairable multi-HAPS systems with a k-out-of-n structure and Semi-Markov sojourns. They emphasize that non-exponential sojourn distributions matter for time-dependent failure probability, steady-state availability, reliability and MTTF. [DOI 10.1016/j.ress.2025.111450](https://doi.org/10.1016/j.ress.2025.111450).
- Mittal et al. (2024) use semi-Markov k-out-of-n reliability for a tethered HAPS rotor subsystem, including a 5-out-of-8 octocopter case. This is a component-level redundancy baseline, not a service-fleet model. [DOI 10.1016/j.ress.2024.110419](https://doi.org/10.1016/j.ress.2024.110419).
- Liang et al.'s 2026 system review frames persistent service as a coupled problem spanning energy/control margin, wind, thermal-buoyancy interaction, recovery and multi-airship continuity. [DOI 10.1016/j.paerosci.2026.101243](https://doi.org/10.1016/j.paerosci.2026.101243).

**Evidence gap:** the public HAPS industry sources located for this work do not provide independently auditable operational fleet availability distributions, maintenance-duration distributions, repair queues, launch/recovery throughput, spare ratios or common-cause failure rates. These must remain unknown pending measured logs or operator disclosure.

## 2. Paradigm shift: from endurance to service continuity

A long flight is a platform metric. A reliable network is a service metric.

`service availability = time demand threshold is met / scheduled service time`

The fleet must absorb:
- planned inspection and maintenance;
- corrective repair and spare-parts delays;
- launch, recovery, transit and repositioning;
- reserve activation and handover;
- weather and other common-cause events;
- ground gateway, satellite and backhaul failures;
- thermal/energy derating and payload faults;
- post-maintenance verification and controlled return to service.

A platform can be airborne but not delivering service because it is out of footprint, payload-disabled, backhaul-constrained or derated. A fleet can preserve service while individual airframes rotate through inspection and repair. These must be accounted for separately.

## 3. Deterministic 72-hour event-trace fixture

Machine-readable artifacts:
- [haps_fleet_service_case_v1.schema.json](schemas/haps_fleet_service_case_v1.schema.json)
- [synthetic_haps_fleet_rotation_common_cause_v1.json](examples/synthetic_haps_fleet_rotation_common_cause_v1.json)

Two service zones each need one capacity unit. A 90% demand-weighted delivery threshold defines an available interval. The seven contiguous intervals total 72 hours:

| Elapsed hours | Event | Duration |
|---:|---|---:|
| 0–8 | Normal operation | 8 h |
| 8–14 | HAPS-01 scheduled maintenance | 6 h |
| 14–20 | Normal operation | 6 h |
| 20–30 | HAPS-02 corrective repair | 10 h |
| 30–40 | Normal operation | 10 h |
| 40–46 | Common-cause weather grounding of every HAPS in the footprint | 6 h |
| 46–72 | Normal operation | 26 h |

Three policies use the same service demand and event trace:

1. **Two HAPS, no spare:** one HAPS per zone; maintenance and repair each remove one service zone.
2. **Four HAPS, two rotating reserves:** HAPS-03 replaces HAPS-01 during planned maintenance; HAPS-04 replaces HAPS-02 during repair. This masks both individual events. The common-cause weather event still grounds all four HAPS.
3. **Four HAPS plus independent satellite fallback:** same rotation policy, but an independently available synthetic satellite layer supplies 95% of each zone's required capacity during common-cause weather.

| Policy | Demand-qualified availability | Time-weighted demand satisfaction | Below-threshold time | Longest outage |
|---|---:|---:|---:|---:|
| Two HAPS, no spare | 69.44% | 80.56% | 22 h | 10 h |
| Four HAPS, rotating reserves | 91.67% | 91.67% | 6 h | 6 h |
| Four HAPS + independent fallback | 100.00% | 99.58% | 0 h | 0 h |

The exact replay also records 4 handovers for each rotating policy, 6 platform-hours of planned maintenance, 10 platform-hours of corrective repair, and 24 common-cause grounded platform-hours for the four-platform fleet. The no-spare policy accumulates 12 common-cause grounded platform-hours across its two assets.

### What this proves—and what it does not

The arithmetic demonstrates two independent reliability levers:
- a reserve pool protects service against **idiosyncratic** maintenance and repair;
- an independently available alternate layer protects against a **common-cause** event that grounds the whole HAPS fleet.

Adding more HAPS alone cannot eliminate an event that disables every HAPS in the footprint. The fallback must actually be independent of the common cause and have enough capacity to meet the service threshold. The fixture's 95% satellite capacity and independence label are synthetic assumptions, not measured satellite availability or guaranteed weather independence.

This is a deterministic 72-hour control fixture. It is not a statistical Semi-Markov result, a 30-day service trial, an empirically supported maintenance model or an achieved fleet-availability result.

## 4. Benchmark suite and matched baselines

1. Single platform / no spare.
2. Independent-failure k-out-of-n Markov baseline (exponential time-to-failure/repair).
3. Repairable k-out-of-n Semi-Markov with non-exponential empirical or declared Weibull/lognormal sojourns.
4. Hot, warm and cold reserve pools with explicit launch/recovery and transition time.
5. Fixed staggered planned maintenance.
6. Condition-based/predictive maintenance using only telemetry available at decision time.
7. Weather-aware fleet positioning and geographically separated bases.
8. Explicit correlated common-cause events (weather, shared gateway, software/configuration, supplier lot, ground site).
9. Multi-layer fallback via terrestrial, satellite, neighboring HAPS or qualified FSO.
10. Joint event-driven fleet scheduler minimizing demand-weighted outage minutes, maintenance debt and operating cost under hard energy/safety constraints.

All controllers must use the same fleet size, service demand, failure/repair/maintenance distributions, common-cause correlations, base infrastructure, alternate-layer capacity and horizon. Compare independent weather campaigns and provide confidence intervals; many random seeds over one synthetic event trace are not independent evidence.

## 5. Metrics

- Demand-qualified availability by zone, criticality and service class.
- Time-weighted delivered demand, critical-task completion and degraded-service minutes.
- Maximum contiguous outage, p95/p99 outage duration, maximum gap, MTTR-to-service (not merely MTTR-to-repair), and handover interruption.
- Active platform hours, ready reserve hours, reserve ratio, transit/repositioning hours and fleet utilization.
- Planned/corrective maintenance hours, MTBF, MTTR, repair queue, parts/crew/hangar wait, post-maintenance verification hold and repeat-failure rate.
- Launch/recovery throughput, turnaround, base diversity, spare-pool size and maintenance backlog.
- Common-cause share of outage minutes, correlation sensitivity, simultaneous-failure count and alternate-layer independence evidence.
- For each result: source data, event-log digest, model applicability, confidence interval and NOT_COVERED/INDETERMINATE cases.

## 6. Research targets — not industry claims

- **Initial target:** at least 99.5% demand-qualified availability over a 30-day service window. This permits at most 3.6 hours below the declared service threshold.
- **Stretch target:** 99.9% over 30 days, permitting at most 43.2 minutes below threshold.
- Specify emergency/critical workload availability separately; do not average it away with best-effort traffic.
- At fixed fleet size and service demand, target at least 20% fewer demand-weighted outage minutes than fixed-reserve/staggered-maintenance baseline without weakening safety, energy, thermal or maintenance constraints.
- Every feasible simulation must account for common-cause scenarios, all transitions and all reserve/inspection states; no independent-failure-only result can be marketed as fleet availability.
- Zero unreported SLA gaps, reserve/thermal violations or missing asset states in a run labelled feasible.

These are proposed Luminous research goals. They are not published HAPS industry SLA targets or achieved performance.

## 7. Next increments

1. Extend the deterministic event trace with delayed launch/recovery, reserve unavailability and maintenance queueing.
2. Define a source-backed input contract for maintenance duration, MTBF/MTTR, launch/recovery turnaround and common-cause correlation. Keep unprovided parameters null/unknown.
3. Implement a replayable event-driven Semi-Markov simulator and compare against the published Jain et al. 2026 framework.
4. Add Monte Carlo campaigns with independent weather/repair scenarios, confidence intervals and rare-event sensitivity.
5. Couple service outage to the existing energy, wind, edge and traffic models; do not build duplicate physics or network engines.
6. Bind aircraft inspection/repair/configuration events to Mycelix provenance and explicit return-to-service evidence.

## 8. Ownership

Compose HAPS service/qualification (#5885 / #5910), network allocation (#5948), wind navigation (#5925), edge placement (#5989), battery qualification (#3903), mission electro-thermal (#11411), and maintenance/configuration provenance (Mycelix #3638).

No physical flight, maintenance release, airworthiness, fleet deployment, spectrum use or public-airspace authorization is implied by this specification.
