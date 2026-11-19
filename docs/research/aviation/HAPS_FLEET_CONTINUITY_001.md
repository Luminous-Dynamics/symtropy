# AERO-HAPS-FLEET-CONTINUITY-001 — weather-correlated fleet availability and service continuity

**Status:** synthetic-first benchmark v0.1  
**Tracker:** [#6223](https://github.com/Luminous-Dynamics/symtropy/issues/6223)  
**Parent HAPS service programme:** [#5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885)  
**Claim ceiling:** deterministic synthetic event replay only.

## 1. Why this matters now

The field is moving from flight demonstration toward service operations, but endurance and service availability are different quantities.

- SoftBank's September 2026 HAPS trial announcement explicitly describes limited trial duration, areas and devices, and targets commercial services in Japan from 2027 onward. It reports the Sceye LTA platform travelled from the US, then held within a radius as small as 5 km for an extended period over its test region. [SoftBank trial announcement](https://www.softbank.jp/en/corp/news/press/sbkk/2026/20260902_01/).
- SoftBank's 27 October 2026 operational interview says it plans facilities in Japan for HAPS storage, takeoff and recovery within the next two years. It also says large-area disasters may require multiple HAPS platforms. This makes the ground segment, launch/recovery throughput and turnaround part of the service architecture, not an afterthought. [SoftBank operational interview](https://www.softbank.jp/en/sbnews/entry/20261027_01).
- Sceye and SoftBank's 3 November 2026 five-year agreement moves toward manufacturing, launch and operation of a HAPS fleet; it is a deployment plan, not proof of an already operating nationwide fleet. [Agreement](https://sceye.com/press-releases/sceye-softbank-sign-agreement-for-fleet-of-stratospheric-platforms/).
- HAPS Alliance's March 2026 6G white paper describes disaster restoration through HAPS, terrestrial and satellite backhaul and gradual transition to repaired ground networks. [6G from the Stratosphere](https://hira.hope.ac.uk/id/eprint/4881/1/HAPSAlliance_6G_WhitePaper_MAR26.pdf).

## 2. Existing state of the art

The benchmark deliberately starts from current reliability research:

- **Jain et al. (2026), multiple-HAPS semi-Markov reliability:** repairable (k)-out-of-(n) fleet model using non-exponential sojourn times, reporting steady-state availability, reliability and MTTF. [Reliability Engineering & System Safety, DOI 10.1016/j.ress.2025.111450](https://doi.org/10.1016/j.ress.2025.111450).
- **Jain et al. (2026), ITS-HAPS reliability + DRL:** CTMCs and reliability block diagrams for system layers; a Markov reward model for cell availability, packet blocking and dropping; simulation validation; and a DQN policy for adaptive transmission decisions. [Vehicular Communications, DOI 10.1016/j.vehcom.2026.101040](https://doi.org/10.1016/j.vehcom.2026.101040).
- **Gautam et al. (2026), energy subsystem reliability:** repairable/non-repairable (k)-out-of-(n) energy-management model for hydrogen fuel cells, with discrete-event simulation and XGBoost/KNN reliability prediction. The paper reports that no dataset was generated or analysed, so it is a method comparator, not an external operational trace. [QRE International, DOI 10.1002/qre.70254](https://doi.org/10.1002/qre.70254).
- **Liang et al. (2026), system-level review:** examines coupling between wind, thermal state, propulsion, energy, control, payload and network performance; notes fully coupled multi-day assessments remain less common. [Progress in Aerospace Sciences, DOI 10.1016/j.paerosci.2026.101243](https://doi.org/10.1016/j.paerosci.2026.101243).

These are the baselines to reproduce. The integration gap this project tests is not "use a more complicated Markov chain"; it is whether fleet availability and maintenance decisions translate into actual critical/full service continuity when weather, ground infrastructure, backhaul and repair logistics share common causes.

## 3. Paradigm shift: from aircraft reliability to service continuity

The core object is a **time-varying service graph**. HAPS are one set of capacity-bearing nodes. Terrestrial gateways, LEO/satellite backhaul, adjacent HAPS, local drone edge and ground infrastructure are other nodes. The graph can remain serviceable after an aircraft is lost, or fail even when every aircraft is technically healthy if a shared gateway, maintenance team, launch site or weather corridor is unavailable.

A service controller must therefore optimize:
- critical vs standard demand satisfaction;
- the number and geometry of on-station HAPS;
- maintenance and corrective repair schedules;
- reserve activation delay and launch/recovery throughput;
- correlated weather and region-wide restrictions;
- terrestrial / satellite / neighbouring-HAPS fallback;
- repair queues, crew bottlenecks, spares and configuration/inspection holds;
- handback to terrestrial service once infrastructure is restored.

This avoids two misleading metrics: counting an airframe as "available" when it cannot reach the service region, and counting an airframe as "failed" when an alternate network layer keeps the customer service running.

## 4. First deterministic event-replay fixture

Machine-readable artefacts:
- [haps_fleet_continuity_case_v1.schema.json](schemas/haps_fleet_continuity_case_v1.schema.json)
- [synthetic_haps_fleet_continuity_30day_v1.json](examples/synthetic_haps_fleet_continuity_30day_v1.json)

The synthetic case covers 720 hours and uses four heterogeneous HAPS identifiers. Two normalized capacity units are required for full service; one unit is sufficient for critical service. The unit is deliberately abstract—not Mbps, coverage area, or a physical RF link budget.

One common 24-hour regional-weather event removes all HAPS capacity in the declared service region. Terrestrial and satellite layers together provide 1.1 units during this event, enough for critical service but not full service. An overlapping six-hour gateway/backhaul impairment drops fallback capacity to 0.2 units, causing critical service loss during those six hours.

### Matched policies

| Policy | Fleet/maintenance | Reserve behavior |
|---|---|---|
| 2-of-3, no reserve | Three active assets; clustered 48-hour maintenance intervals | Fourth asset remains ready but is not dispatched |
| Cold spare after delay | Same three active assets and maintenance schedule | Fourth asset is requested when capacity drops, but needs 12 hours to arrive |
| Staggered four-node rotation | Same four assets, one 48-hour maintenance interval at a time | Fourth asset participates in the rotation |

### Independently replayed synthetic results

| Metric | 2-of-3, no reserve | Cold spare after 12 h | Staggered four-node rotation |
|---|---:|---:|---:|
| HAPS-only full-service availability | 90.00% | 95.00% | 96.67% |
| End-to-end critical-service availability | 99.17% | 99.17% | 99.17% |
| End-to-end full-service availability | 90.00% | 95.00% | 96.67% |
| Useful service capacity delivered | 95.79% | 97.54% | 98.13% |
| Full-service degraded hours | 72 h | 36 h | 24 h |
| Critical-service outage hours | 6 h | 6 h | 6 h |
| Maximum contiguous full-service degradation | 48 h | 24 h | 24 h |
| Planned maintenance hours (sum across assets) | 144 h | 144 h | 192 h |

Interpretation:
- A 12-hour reserve activation delay cuts maintenance-induced full-service degradation from 48 h to 12 h, reducing total full-service degraded time from 72 h to 36 h after the shared weather event.
- Staggered four-node rotation eliminates the planned-maintenance overlap; the remaining 24 h full-service degradation is entirely due to the common weather restriction.
- The critical-service outage remains six hours under every policy because the gateway/backhaul impairment is correlated with the weather event and defeats the finite fallback layer. Better airframe rotation alone cannot fix a shared network-layer failure.
- These values are arithmetic results on a synthetic event trace. They are not probabilistic availability estimates, a 99.9% annual SLA, or measured fleet operations.

The executable reference is [tools/haps-fleet-continuity](../../../tools/haps-fleet-continuity), an isolated Rust crate with a JSON replay CLI, schema/semantic guards and fourteen unit tests. It rejects cyclic parent-event graphs and overlapping reserve-dispatch windows, and composes simultaneous network-capacity overrides by minimum remaining capacity so JSON event order cannot alter results. The tests cover exact 720-hour oracle reproduction, reserve activation delay, staggered rotation, critical/full service separation, unknown platform rejection, common-cause event parentage, out-of-horizon and off-grid events, standby-state enforcement, and a subset-weather event against an already unavailable platform. The dedicated workflow runs rustfmt, Clippy, tests and the committed-fixture replay. Those Rust tests are authored but not yet executed at this revision; the independent 720-step hourly replay below is separate from the crate.

## 5. What to benchmark next

### Reliability models
1. Independent exponential CTMC and classic (k)-out-of-(n).
2. Non-exponential semi-Markov failure/repair model (Jain et al. 2026).
3. Layered CTMC + RBD + Markov reward model with blocking/dropping (ITS-HAPS paper).
4. Semi-Markov / renewal model with explicit common-cause weather and correlated gateway loss.
5. Discrete-event simulation with finite maintenance crew, launch/recovery capacity, spares and repair queues.
6. Exact small-case maintenance scheduling oracle; then MILP/CP-SAT, robust recourse and learned policies.

### Service / operations scenarios
- One platform; no spare; planned inspection.
- 2-of-3, independent repair.
- 2-of-4 with staggered maintenance and cold reserve.
- Clustered maintenance / shared crew bottleneck.
- Weather restriction affecting every HAPS in a service footprint.
- Weather plus gateway/backhaul outage.
- Launch site closure and delayed reserve dispatch.
- Shared configuration defect and fleet-wide quarantine.
- Energy/thermal derating and early return-to-base.
- Terrestrial restoration, service handback and HAPS redeployment.
- Mixed lift-class portfolio with separate weather susceptibility, control and maintenance distributions.

## 6. Benchmark metrics

- Fleet (k)-of-(n) availability, mission availability and service-region availability.
- Critical-service and full-service availability reported separately.
- Time-integrated useful capacity delivered / demand.
- Outage count and duration distribution, p95/p99 restoration, maximum contiguous outage.
- Handover interruption, packet blocking/dropping and unmet critical demand.
- MTTF, MTTR, turnaround, time-to-launch, time-to-recover, maintenance backlog and spare utilization.
- Common-cause event exposure and conditional service availability during adverse-weather regimes.
- Crew, ground site, spare-parts and launch/recovery utilization.
- Energy/thermal reserve at every maintenance rotation, launch and handover.
- Confidence calibration, event replay exactness and out-of-domain/NOT_COVERED rates.

## 7. Stretch targets (research goals, not achieved results)

- Reproduce the 2026 semi-Markov multi-HAPS results and the 2026 ITS-HAPS CTMC/MRM/DQN baseline on their published assumptions before claiming superiority.
- On independent weather-year and maintenance-campaign blocks, target at least 20% relative reduction in p95 critical-service outage duration versus a matched fixed-rotation baseline, at the same fleet/crew/spares budget.
- Target ≥99.9% critical-service availability only for a fully specified scenario, geography, service tier, time horizon, redundancy and backup-layer contract, with uncertainty bounds. A 30-day synthetic replay cannot substantiate an annual 99.9% guarantee.
- Zero avoidable maintenance overlaps that push service below its critical threshold when a feasible staggered schedule exists.
- Zero availability claims that omit common-cause weather, ground-site, backhaul, crew or shared-configuration events.
- Report worst-regime and confidence-interval results, not only a mean across random seeds.

## 9. Validation and evidence status

- Machine-readable schema and synthetic fixture parsed from the research branch.
- Independent hourly event replay: 720 one-hour steps × three policies; every policy output matched the committed expected oracle.
- Rust evaluator/CLI and ten unit tests are authored. They have **not been executed** because no local Rust toolchain is available in the authoring environment. The dedicated GitHub Actions workflow is committed but no CI pass is claimed here.
- No full JSON Schema validator, stochastic/semi-Markov probability estimation, real weather trace, maintenance telemetry, measured aircraft availability, radio propagation, thermal simulation or physical operation is included.

## 8. Ownership boundaries

Compose existing work:
- HAPS service and network: [#5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885), [network orchestration #5948](https://github.com/Luminous-Dynamics/symtropy/pull/5948).
- Wind navigation: [#5925](https://github.com/Luminous-Dynamics/symtropy/issues/5925).
- Edge placement: [#5989](https://github.com/Luminous-Dynamics/symtropy/pull/5989).
- Multi-day HAPS energy/reserve: [Symthaea #7343](https://github.com/Luminous-Dynamics/symthaea/issues/7343), [PR #12674](https://github.com/Luminous-Dynamics/symthaea/pull/12674).
- Battery and installed-pack evidence: [#3903](https://github.com/Luminous-Dynamics/symtropy/issues/3903).
- Mission electro-thermal coupling: [Symthaea #11411](https://github.com/Luminous-Dynamics/symthaea/issues/11411).
- Aircraft inspection/configuration/maintenance provenance: [Mycelix #3638](https://github.com/Luminous-Dynamics/mycelix/issues/3638).

No duplicate vehicle, weather, thermal, battery, RF or repair-distribution solver is created. No flight, launch, recovery, maintenance release, spectrum use or commercial service authorization is implied.
