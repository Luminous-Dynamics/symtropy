# AERO-HAPS-FLEET-AVAIL-001 — fleet availability, maintenance turnaround and correlated weather

**Status:** synthetic-first research specification v0.1  
**Tracker:** [#6074](https://github.com/Luminous-Dynamics/symtropy/issues/6074)  
**Parent programme:** [AERO-HAPS-ENERGY-NETWORK-001 #5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885)  
**Claim ceiling:** deterministic platform-count event arithmetic only; no fleet operating or physical reliability claim.

## 1. Why the benchmark changes now

On 3 November 2026, Sceye and SoftBank announced a five-year agreement to manufacture, launch and operate a HAPS fleet targeting nationwide coverage in Japan. The announcement follows the single 30-day ST1 campaign and its week of connectivity demonstrations; it does not establish that a nationwide fleet is already in service. [Sceye announcement](https://sceye.com/press-releases/sceye-softbank-sign-agreement-for-fleet-of-stratospheric-platforms/).

A Payload interview the same day adds an operational detail that flight-endurance comparisons can miss: recovered Sceye vehicles are reportedly disassembled and tested before parts are reused, and turnaround is currently described as "in the months", with an ambition to reduce it to weeks. The same interview describes an aspiration for six-month individual missions. This is interview-attributed information, not audited fleet telemetry or a measured turnaround distribution. [Payload interview, 3 November 2026](https://payloadspace.com/exclusive-sceye-signs-five-year-deal-to-fly-haps-fleet-for-softbank/).

The resulting question is not merely "How long can one aircraft remain aloft?" It is: **How much qualified service can the entire HAPS system deliver, through flight, weather, maintenance, recovery, inspection, reconditioning, launch and handover?**

## 2. State of the art

### HAPS-specific fleet reliability

Jain et al., *Semi-Markov modeling approach for the reliability analysis of a multiple high altitude platform station system*, Reliability Engineering & System Safety, January 2026, applies a repairable k-out-of-n framework to multi-HAPS service and uses semi-Markov sojourn times to represent non-exponential failure and repair durations. It evaluates steady-state availability, reliability and MTTF. This is the closest HAPS-specific published fleet reliability baseline. [DOI 10.1016/j.ress.2025.111450](https://doi.org/10.1016/j.ress.2025.111450).

### Cross-domain maintenance optimization

Rasappan et al., *Reliability and Availability of a Repairable k-Out-of-n System via Event-Triggered Maintenance, Semi-Markovian Modelling, Reinforcement Q-Learning, and Copula Repair*, Algorithms, 13 October 2026, combines event-triggered repair intensification, Gumbel–Hougaard copula restoration, semi-Markov transitions and Q-learning threshold selection for a two-tier computer network. The paper reports improved availability-AUC under its simulated configurations and 30 Monte Carlo replications. It is a useful methodological comparator, not a HAPS operations result. [DOI 10.3390/a19100879](https://doi.org/10.3390/a19100879).

## 3. Paradigm shift: qualify the service, not just the vehicle

The fleet model must distinguish:

- **Airborne:** physically aloft, regardless of whether it can provide the required service.
- **On station:** within a declared target region and altitude/operating envelope.
- **Service ready:** payload, power, thermal, backhaul and coverage constraints are all satisfied.
- **Stand-down:** weather or operating restrictions make the platform unavailable.
- **Recovering / maintenance / reconditioning:** not available to serve customers.
- **Ready reserve:** inspected, configured, logistically supported and launchable within a defined time.
- **Launched / transit / handover:** contributes to service only after the qualified coverage handover has completed.

A platform that is airborne but has no usable backhaul, inadequate energy reserve or failed payload is not a service-available node. V1 deliberately counts only platform states and labels its service boundary as 'platform_count_only'; later versions must compose coverage, wind, backhaul, edge, energy and thermal receipts.

### Common-cause exposure

A spare protects against an individual aircraft being maintained. It may not protect against several co-located platforms being stood down by the same regional weather system, launch-site closure, shared part defect, maintenance bottleneck or gateway outage. The benchmark therefore assigns nodes to explicit weather domains and models common-cause events as affecting every node in the impacted domain.

The key optimization variable becomes **diversity of exposure and recovery pathways**, not only the number of vehicles.

### Recovery pipeline as infrastructure

Fleet service depends on recovery assets, inspection and reconditioning capacity, parts inventory, technician capacity, launch slots, transport and the time needed to restore a platform to ready reserve. A slow return-to-ready loop can dominate service availability even when the individual airframe is highly reliable.

## 4. Machine-readable v1 artifacts

- [haps_fleet_availability_case_v1.schema.json](schemas/haps_fleet_availability_case_v1.schema.json)
- [synthetic_haps_fleet_maintenance_common_cause_30day_v1.json](examples/synthetic_haps_fleet_maintenance_common_cause_30day_v1.json)
- Standalone Rust checker: [Cargo.toml](../../../tools/haps-fleet-availability/Cargo.toml), [lib.rs](../../../tools/haps-fleet-availability/src/lib.rs), [main.rs](../../../tools/haps-fleet-availability/src/main.rs)
- Dedicated workflow: [haps-fleet-availability.yml](../../../.github/workflows/haps-fleet-availability.yml)

The executable v1 accepts synthetic, daily-step cases only. It computes platform unavailability as the union of maintenance and common-cause weather impacts for each node/day, counts active nodes, and checks service against a k-out-of-n threshold. Unknown launch/recovery/return-to-ready logistics are explicit and excluded; the tool will not silently infer them.

## 5. Deterministic 30-day micro-benchmark

Three matched variants use the same k=3 service threshold, 30-day window and staggered three-day inspection windows.

| Fleet topology | Fleet | Scheduled inspection | Common-cause weather | Service days | Availability |
|---|---:|---:|---|---:|---:|
| No spare / shared weather domain | 3 | 9 platform-days | All three stood down for 2 days | 19/30 | 63.33% |
| Four nodes / two clustered domains | 4 | 12 platform-days | Two co-located nodes stood down for 2 days | 28/30 | 93.33% |
| Four nodes / four diverse domains | 4 | 12 platform-days | One node stood down for 2 days | 30/30 | 100% |

The first case loses nine days to one-at-a-time inspections because k=n=3, then two days to a shared weather event. In the clustered four-node case, the single spare protects the scheduled maintenance windows, but a two-node common-cause event still drops active nodes below k. In the diversified case, the same two-day regional event affects only one node, leaving three active.

This is a deterministic proof-of-semantics example. It is not a forecast of the availability of any operator's real fleet, and four weather domains are not asserted to be sufficient for actual stratospheric operations.

### Independent replay

An independent evaluator recomputed all daily active-node counts, availability flags, unavailable-day offsets, maintenance and weather platform-days, maximum simultaneous outages and longest outage:
- 3-node variant: 19 service days, 11 unavailable; longest outage 3 days.
- 4-node clustered: 28 service days, 2 unavailable; longest outage 2 days.
- 4-node distributed: 30 service days, zero unavailable.

Structural required-field and event-window checks passed. This was not full JSON Schema validator execution, Monte Carlo validation or a physical campaign.

## 6. Benchmark methods to compare

1. Independent exponential failure/repair k-out-of-n baseline.
2. HAPS-specific semi-Markov k-out-of-n (Jain et al. 2026).
3. Weibull and empirical non-exponential time-to-failure and repair distributions.
4. Independent weather assumptions versus spatially correlated common-cause weather regimes.
5. Fixed rotation and staggered maintenance policies.
6. Condition-based / event-triggered maintenance and repair intensification.
7. Copula-based correlated restoration when supported by data.
8. Reserve fleet positioning, launch-slot and recovery-capacity optimization.
9. Proposed service-aware fleet planner with hard k-of-n service constraints and evidence-qualified fallbacks.

The comparators must share the same campaign logs, weather-domain maps, k threshold, repair-time samples, launch/recovery resources, spare pool, and held-out weather years/regions. Monte Carlo seeds are not independent evidence when they reuse the same weather episode.

## 7. Metrics and programme targets

Report service availability and reliability separately:
- annual and monthly service availability;
- probability of uninterrupted service over a stated mission interval;
- time below k-of-n, longest outage and outage frequency;
- p50/p95/p99 recovery-to-ready and launch-to-service;
- fleet time in airborne, on-station, stand-down, recovery, inspection, reconditioning, ready-reserve and transit states;
- service-area-hours and weighted critical-service completion;
- common-cause losses by region/altitude/weather regime;
- spare ratio, maintenance queue age, parts fill rate, launch/recovery throughput and technician utilization;
- cost and useful service per platform-day, rather than airborne hours alone.

Luminous programme targets (not industry guarantees or achieved results):
- initial annual service availability ≥99.5%; protected critical-service target ≥99.9%, with terrestrial/LEO fallback and a clearly defined end-to-end service boundary;
- zero admitted planned-maintenance schedules that violate the required k-of-n threshold;
- publish p95/p99 recovery-to-ready and launch-to-service times; after instrumenting a baseline, target 50% reduction against our own measured process;
- explicitly test common-cause loss of one, two and all weather domains, shared parts shortage, launch-site closure and gateway/backhaul outage;
- after distributions are grounded, at least 10,000 campaign replications per held-out weather/repair regime, with confidence intervals, worst-decile and CVaR outage;
- no availability claim based solely on physical airborne status.

The 99.5% / 99.9% values are internal design goals. They must not be represented as published HAPS service guarantees.

## 8. Next implementation steps

1. Extend the deterministic oracle to interval-level state transitions and explicit ready-reserve, launch, transit, recovery and reconditioning events.
2. Bind a source-qualified turnaround distribution; the public "in the months" statement is not sufficient to derive a numeric distribution.
3. Add weather-year and altitude-domain holdouts; calibrate common-cause intensity using measured campaign evidence.
4. Implement a semi-Markov benchmark using empirical sojourn distributions and compare with exponential/Markov assumptions.
5. Add service boundary integration: active nodes must meet coverage, wind, energy, thermal, payload and backhaul requirements.
6. Add maintenance resource constraints, spares, parts commonality, launch-site capacity and reserve repositioning.

## 9. Ownership and claim ceiling

Compose:
- HAPS programme: [#5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885).
- Demand/beam orchestration: [#5948](https://github.com/Luminous-Dynamics/symtropy/pull/5948).
- Wind-aware navigation: [#5925](https://github.com/Luminous-Dynamics/symtropy/issues/5925).
- Edge placement: [#5989](https://github.com/Luminous-Dynamics/symtropy/pull/5989).
- Battery installed-system evidence: [#3903](https://github.com/Luminous-Dynamics/symtropy/issues/3903).
- Configuration, inspection and maintenance provenance: Mycelix [#3638](https://github.com/Luminous-Dynamics/mycelix/issues/3638).

V1 is a deterministic platform-count event checker over synthetic inputs. Launch/recovery turnaround, service geometry, wind, backhaul, energy, thermal, probabilistic reliability and airworthiness are outside its claim ceiling.
