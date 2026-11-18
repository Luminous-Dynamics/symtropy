# AERO-HAPS-FLEET-RESILIENCE-001 — service continuity across HAPS fleets and terrestrial/LEO fallback

**Status:** synthetic-first reliability specification v0.1  
**Parent:** [HAPS persistence and network service #5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885)  
**Qualification branch:** `research/haps-qualification-current-20261118`  
**Claim ceiling:** analytic reliability arithmetic only; no measured provider SLA.

## 1. The paradigm shift: optimize continuous service, not individual aircraft uptime

An aircraft endurance record is a vehicle-level result. A communications/observation infrastructure service needs a distinct service-level result across aircraft, maintenance, weather, handover, ground gateways, satellites and control systems.

A five-year fleet agreement to manufacture, launch and operate multiple stratospheric platforms is a meaningful industry transition, but it is a deployment plan rather than evidence of already-achieved nationwide service availability or a published availability SLA. Sceye/SoftBank's 3 November 2026 announcement explicitly combines stratospheric platforms, terrestrial network integration and an FSO subset. [Sceye/SoftBank MSA](https://sceye.com/press-releases/sceye-softbank-sign-agreement-for-fleet-of-stratospheric-platforms/).

ITU-T Y.3212 provides a useful standards anchor: it describes HAP cooperation with terrestrial, satellite and other HAP networks, overlapping coverage, collaborative location management and satellite backhaul. It also explicitly specifies group handover between serving aircraft/HAP/satellite nodes, with confirmation and failure subsets. [ITU-T Y.3212, §§7.3.3–7.4](https://www.itu.int/rec/dologin_pub.asp?id=T-REC-Y.3212-202409-I%21%21PDF-E&lang=e&type=items).

**Therefore:** the primary product metric is demand-weighted service continuity per region/mission, not days aloft. Aircraft uptime is one input into service availability.

## 2. Machine-readable starting point

- [haps_fleet_availability_case_v1.schema.json](schemas/haps_fleet_availability_case_v1.schema.json)
- [synthetic_haps_fleet_availability_case_v1.json](examples/synthetic_haps_fleet_availability_case_v1.json)

The first case is a deliberately transparent analytic model. It separates independent platform unavailability from a shared HAPS weather/common-mode outage. It models terrestrial and LEO fallback as additional layers under explicit independence assumptions.

Let:
- p: independent availability of each HAPS, including individually attributable faults and planned maintenance;
- q: shared HAPS-only common-mode weather outage probability;
- r: common-mode outage that disables terrestrial and LEO fallback together (for example, a shared ground/core/power/backhaul domain);
- g: terrestrial service availability conditional on no external common-mode event;
- s: LEO backhaul/service availability conditional on no external common-mode event;
- n: number of HAPS platforms.

Then:
- HAPS-only availability: A_H(n) = (1 - q) × [1 - (1 - p)^n]
- External fallback unavailability: U_G = r + (1 - r)(1 - g)
- Joint terrestrial+LEO unavailability: U_GL = r + (1 - r)(1 - g)(1 - s)
- HAPS + terrestrial availability: A_HG = 1 - (1 - A_H)U_G
- HAPS + terrestrial + LEO availability: A_HGL = 1 - (1 - A_H)U_GL

These formulas are valid only under the declared conditional-independence assumptions. Shared power, fibre, gateway, spectrum, cyber, satellite-space-weather, ground-station or geopolitical failures can invalidate the products. The case is not a Markov model, empirical reliability estimate or fault-tree qualification.

## 3. Synthetic sensitivity results

The fixture uses (p=0.98), (g=0.995), (s=0.99), and sweeps HAPS shared-weather/common-mode outage (q) over 0%, 3% and 10%. All are synthetic inputs, not industry reliability values.

For the 3% common-mode case:

| Architecture | Availability | Expected downtime / year |
|---|---:|---:|
| 1 HAPS only | 95.0600% | 432.744 h |
| 3 HAPS only | 96.999224% | 262.868 h |
| 10 HAPS only | 97.000000% | 262.800 h |
| 3 HAPS + terrestrial fallback | 99.984996% | 1.314 h |
| 3 HAPS + terrestrial + LEO fallback | 99.999850% | 0.01314 h (about 47.3 s) |

The key effect is the common-mode floor: going from 3 to 10 HAPS saves only about 0.068 hours (roughly four minutes) of expected HAPS-only downtime per year in this scenario. Additional same-class platforms do not remove a shared weather limit. A diverse fallback layer has much larger theoretical value—but only if it is genuinely independent and offers the required capacity, coverage, latency, power, security and operational permission.

At 10% common HAPS outage, three HAPS plus terrestrial fallback gives about 99.9500%, while adding the assumed LEO fallback gives about 99.9995%. Again, this is only the product-form synthetic sensitivity, not measured network availability.

### Correlated external fallback sensitivity

The case sweeps a second common-mode probability r that disables both terrestrial and LEO fallback together. Holding HAPS common-weather outage at 3% and using three HAPS:

| Shared terrestrial/LEO outage r | Expected layered downtime / year |
|---:|---:|
| 0% | 0.01314 h (about 47.3 s) |
| 0.1% | 0.2760 h (about 16.6 min) |
| 1% | 2.6417 h |

A nominally diverse architecture can still fail through common ground/core/power/backhaul dependencies. In the 1% synthetic common-external case, layered service availability is about 99.9698%, below the illustrative 99.99% objective. The independence contract is conditional: terrestrial and LEO are independent only when the shared external common-mode event does not occur, and that event is assumed independent of HAPS weather.

## 4. Maintenance and service continuity

The baseline maintenance policy is three HAPS with at most one planned maintenance at a time. Planned maintenance is already included in the per-HAPS independent availability input and must not be subtracted again.

The next event-driven extension must represent:
- maintenance windows and turnaround time, launch/recovery/reposition windows, spare readiness and missed maintenance;
- common weather that affects all HAPS or only a particular altitude/vehicle class;
- independent platform faults and repair time distributions;
- gateway, fibre, satellite and ground-station common-cause failures;
- user-group handover: offered, prepared, accepted, rejected and completed cohorts;
- service capacity and latency during handover, not merely binary availability;
- demand-weighted outage minutes, interruption duration distribution, failover time and data/control continuity.

The ITU group-handover concept is particularly important: users served by one aircraft can move as a group to another, and the target may accept only a subset. A benchmark that records only "target aircraft online" misses the service interruptions and failed handover cohort.

## 5. Benchmarks and research targets

Targets, not achievements:
- Initial field trial: publish the precise service boundary, outage denominator, scheduled-maintenance exclusions, coverage region, load, p50/p95/p99 interruption and failover response.
- Demonstrate at least 99.9% demand-weighted service availability over a 90-day operational campaign, including a declared treatment of planned maintenance.
- Mature regional service target: 99.99% (about 52.6 minutes/year) with explicit geographic coverage, capacity and latency conditions.
- Stress target: preserve service under one platform loss plus a declared common weather regime, using independent ground/LEO fallback where feasible.
- No single HAPS count should be called "N+1 resilient" until common-cause, correlated weather, backhaul, gateway and handover failure modes are tested.
- Report availability by zone and mission class, not only fleet-average uptime. Report whether emergency/health/critical-infrastructure demand was served during the outage.
- Compare the actual incremental cost per additional service-availability nine (platform acquisition, launch/recovery, crew/maintenance, spares, spectrum, ground/LEO contracts and reserved capacity).

No public source cited here establishes an achieved commercial HAPS nationwide availability SLA. The fleet agreement is a deployment milestone, not proof of a specific SLA.

## 6. Next executable layer

1. Convert the analytic formula into a small Rust reliability evaluator with unit/property tests and a versioned input parser.
2. Add an hourly event-trace case for staggered maintenance, faults and common weather. Compute service availability, outage minutes and handover interruptions.
3. Add explicit cross-layer correlation cases; do not assume terrestrial and LEO are independent in the stress suite.
4. Add group-handover cohort tests with partial acceptance, context transfer, timeout, rollback and no split-brain authority.
5. Integrate the demand trace and edge workload scheduler so availability is measured for required service, not merely platform heartbeat.
6. Compare 1/2/3/5 platform fleet layouts and diverse fallback by cost per served critical task and availability nine.

## 7. Executable fleet reliability evaluator

The canonical executable owner is the existing standalone Rust tool at `tools/haps-fleet-reliability`. It uses exact enumeration of independent Bernoulli node states, explicit common-cause events and shared dependencies. It also calculates a deliberately naïve k-out-of-n comparator that ignores common-cause events, making the resilience overstatement visible. It rejects temporal/semi-Markov or maintenance-profile inputs rather than silently treating them as static probabilities. This static probability mode remains the correct baseline for mean/steady-state independence and common-cause sensitivity; it is not a time-scheduled service replay.

A new heterogeneous cross-layer fixture, [synthetic_haps_fleet_cross_layer_common_mode_reliability_v1.json](examples/synthetic_haps_fleet_cross_layer_common_mode_reliability_v1.json), models three HAPS, a terrestrial gateway and LEO backhaul. A shared stratospheric-weather event grounds all three HAPS; a separate common external event disables the terrestrial and LEO fallback together. The exact enumerator evaluates 128 states and reports 99.9698437% service availability against the illustrative 99.99% target. The naïve independent-provider comparator reports 99.99999996%, overstating service availability by approximately 0.03016 percentage points. This is the failure mode the explicit dependency model is meant to expose.

The eight earlier reliability tests now include a ninth exact cross-layer test. The closed-form [fleet availability matrix](examples/synthetic_haps_fleet_availability_case_v1.json) remains as an independent 45-row analytic sensitivity oracle; it is not a second Rust implementation. Both models are synthetic. The current exact enumerator is still static: it does not simulate outage duration, weather episode persistence, repair, maintenance scheduling or group handover.

### 7.1 Deterministic event-schedule service replay — first operational increment

The same Rust crate now also exposes a separate deterministic schedule evaluator; this does not change or weaken the static Bernoulli method. New artifacts:

- [haps_fleet_service_availability_v1.schema.json](schemas/haps_fleet_service_availability_v1.schema.json)
- [synthetic_haps_fleet_30day_availability_v1.json](examples/synthetic_haps_fleet_30day_availability_v1.json)
- `tools/haps-fleet-reliability/src/service_availability.rs`
- `tools/haps-fleet-reliability/src/bin/service-replay.rs`

The fixture is a synthetic 720-hour campaign with three required service-capacity units, three primary platforms and one ground-ready reserve. It includes two staggered 24-hour maintenance windows, one 24-hour unscheduled repair, explicit 12-hour reserve launch lead, three reserve launch/recovery cycles, one maintenance bay, and a 24-hour weather common-cause event that derates all airborne platform capacity to 0.7 and closes launch/recovery sites.

The evaluator partitions time at every platform-state and common-cause event boundary, integrates strict availability and capacity adequacy separately, reports contiguous outages, and rejects gaps/overlaps in platform timelines, maintenance-bay overcommitment, launch/recovery concurrency violations, or operations during declared site closure. It remains deterministic and schedule-driven; it does not yet sample stochastic failure/repair distributions or claim Semi-Markov behavior.

**Independent synthetic oracle replay:**

| Measure | 3 primary / no spare | 3 primary + 1 reserve rotation |
|---|---:|---:|
| Strict service availability | 86.67% | 95.00% |
| Capacity adequacy | 95.67% | 98.44% |
| Strict outage hours / 720 h | 96 h | 36 h |
| Longest contiguous outage | 24 h | 24 h |
| Unserved service-equivalent hours | 31.2 h | 11.2 h |

The reserve improves strict availability by 8.33 percentage points and capacity adequacy by 2.78 percentage points under these synthetic assumptions. It covers both planned maintenance windows. It does not eliminate common-cause weather: the full fleet still loses strict service from hour 600 to 624 because effective capacity falls to 2.1 of the required 3 units. The unplanned platform repair creates a 12-hour gap while the reserve is in its launch lead phase (hours 360–372).

The deterministic service-schedule module now has ten authored Rust tests: ground-ready known-answer and no-spare results, airborne-holding trade-off, common-cause weather persistence, invalid timeline, maintenance-bay overcommitment, launch during site closure, missing airborne holding-slot negative control, 220-day cyclic schedule feasibility and a missing-reserve-capacity negative control. The static Bernoulli module retains nine tests, and the rotation-capacity lower-bound module adds seven tests. There are 26 authored Rust tests across the three modules. They have not been executed locally; no Cargo, rustfmt, Clippy or CI pass is claimed here. No field availability estimate follows from any synthetic fixture.

### 7.2 Ground-ready spare versus airborne holding reserve

A HAPS Alliance operations-at-scale concept describes aircraft remaining airborne between missions, moving to designated holding regions ("airborne parking lots"), and following regional or super-regional loops that can exploit prevailing winds. This is an industry operations concept, not evidence that a particular operator maintains a validated airborne spare pool. [HAPS Alliance, Operations at Scale](https://hapsalliance.org/wp-content/uploads/formidable/12/HAPS_Operations_At_Scale_White_Paper_2022-1.pdf).

This creates a reserve-topology decision:
- **Ground-ready spare:** lower standby energy and easier maintenance access, but requires launch/recovery infrastructure, weather windows and launch lead time.
- **Airborne holding reserve:** can reduce time-to-station and avoid a new launch for each handover, but burns energy while waiting, is exposed to the same weather regimes, accumulates flight hours and may be harder to inspect/repair.
- **Regional airborne rotation:** can move reserve capacity between service zones and exploit winds, but requires route/airspace, demand, weather, reserve and handover coordination.

The launch/recovery phase is itself a safety-relevant phase. JARUS CS-HAPS Subpart H includes requirements for launch/recovery systems, launch/recovery safety areas, continuation energy/controllability and load limits. This benchmark does not qualify any launch/recovery procedure. [JARUS CS-HAPS Rev. 1.0, §2710](https://jarus-rpas.org/wp-content/uploads/2024/07/CS-HAPS-Rev.-1.0.pdf).

A second synthetic case [synthetic_haps_fleet_30day_airborne_reserve_availability_v1.json](examples/synthetic_haps_fleet_30day_airborne_reserve_availability_v1.json) now holds the same 3-primary / 1-reserve service contract, primary maintenance/repair windows, common weather event, 720-hour window and service demand constant, but changes HAPS-D from ground-ready to airborne holding with a 2-hour reposition assumption.

| Reserve strategy | Strict availability | Capacity adequacy | Reserve energy (synthetic) | Longest outage |
|---|---:|---:|---:|---:|
| Ground-ready spare | 95.00% | 98.44% | 141.84 kWh | 24 h |
| Airborne holding reserve | 96.67% | 99.00% | 366.00 kWh | 24 h |

Under these deliberately synthetic power assumptions, airborne holding adds 1.67 percentage points of strict availability and 0.56 points of capacity adequacy, while reserve-strategy energy rises by 224.16 kWh (about 2.58× the ground-ready reserve). The airborne reserve eliminates the synthetic 12-hour launch-lead gap during an unplanned fault; it does **not** eliminate the shared 24-hour weather outage, because the weather event still derates the three service-region aircraft to 2.1 of the required 3 capacity units.

The reserve-energy ledger includes only the reserve platform's declared state power rates (ground-ready, airborne holding, launch, reposition, on-station and recovery). It excludes the primary fleet's energy and any unrepresented payload, avionics, thermal, communications or battery-degradation loads. The synthetic ground-ready reserve consumes 141.84 kWh across its standby/launch/on-station/recovery states, while the airborne-holding reserve consumes 366.00 kWh under the second fixture's power-rate assumptions. The 2.58× ratio is an illustrative Pareto comparison; a real reserve decision requires owner-qualified platform power, thermal, ageing and maintenance data. These rates are synthetic assumptions, not a measured HAPS power profile. A real comparison must use platform-specific mission-energy and maintenance owners and matched service regions/weather.

The event-schedule evaluator reports reserve energy separately from primary-fleet energy, and now supports `ground_ready`, `airborne_holding` and `repositioning` states. The new fixture adds six reposition events and checks reserve holding-slot capacity. This remains deterministic event replay, not an empirical reliability estimate or Semi-Markov model.

The general fleet reliability tool has a dedicated path-scoped workflow for rustfmt, Clippy and fixture replay. Exact-head CI status is tracked separately; no Rust test or physical reliability pass is inferred from structural inspection alone.

### 7.3 Cyclic fleet sizing: service duration versus refurbishment turnaround

*Payload* reported on 3 November 2026 that Sceye's recovered platforms were being dismantled and tested before component reuse, with turnaround then measured in months; the stated objective was to reduce this to weeks, while also pursuing individual platform flights of six months or more. The article also reports a five-year SoftBank fleet agreement and an executive target of at least 40 platforms. These are interview-reported operating context and plans, not a public achieved fleet-availability SLA. [Payload, 3 November 2026](https://payloadspace.com/exclusive-sceye-signs-five-year-deal-to-fly-haps-fleet-for-softbank/). The official five-year MSA announcement describes manufacturing, launch and operation toward nationwide Japanese coverage, but does not publish a fleet availability SLA. [Sceye/SoftBank MSA](https://sceye.com/press-releases/sceye-softbank-sign-agreement-for-fleet-of-stratospheric-platforms/).

This creates a first-order lifecycle trade-off. For identical aircraft with one unit of service capacity each, required continuous service (R), active service interval (D), and sequential recovery/refurbishment/relaunch turnaround (T), the idealized service duty fraction is:

`d = D / (D + T)`

The aggregate fleet-size lower bound is:

`N_min = ceil(R / d) = ceil(R × (D + T) / D)`

This is a **lower bound only**. It assumes perfect phase staggering, identical platforms, no failures during active service, no common-cause weather, unlimited maintenance bays and launch/recovery capacity, no spare failures and no handover gaps. It does not prove that a strict zero-outage rotation schedule exists. The ST1 30-day record was a US–Japan round-trip flight including transit and about a week of Japan operations; it is not treated here as a measured 30-day on-station duration. All durations below are synthetic what-if inputs.

The new machine-readable inputs and exact arithmetic evaluator are:
- [haps_fleet_rotation_capacity_v1.schema.json](schemas/haps_fleet_rotation_capacity_v1.schema.json)
- [synthetic_haps_fleet_rotation_capacity_v1.json](examples/synthetic_haps_fleet_rotation_capacity_v1.json)
- [rotation_capacity.rs](../../../tools/haps-fleet-reliability/src/rotation_capacity.rs)
- CLI: `cargo run --locked --manifest-path tools/haps-fleet-reliability/Cargo.toml --bin rotation-capacity -- docs/research/aviation/examples/synthetic_haps_fleet_rotation_capacity_v1.json`

<text></text>
| Scenario | Active service | Turnaround | Duty fraction | Fleet lower bound for 3 continuous units | Reserve lower bound |
|---|---:|---:|---:|---:|---:|
| Weeks-scale turnaround | 30 d | 14 d | 68.18% | 5 | 2 |
| One-month turnaround | 30 d | 30 d | 50.00% | 6 | 3 |
| Two-month turnaround | 30 d | 60 d | 33.33% | 9 | 6 |
| Three-month turnaround | 30 d | 90 d | 25.00% | 12 | 9 |
| Six-month turnaround | 30 d | 180 d | 14.29% | 21 | 18 |
| Six-month active / weeks turnaround | 180 d | 14 d | 92.78% | 4 | 1 |
| Six-month active / three-month turnaround | 180 d | 90 d | 66.67% | 5 | 2 |

The key implication is that **serviceability and turnaround engineering can reduce fleet capital almost as much as endurance improvements**. In this idealized three-unit service, moving from a 30-day active / 90-day turnaround cycle (12 aircraft lower bound) to a 180-day active / 14-day turnaround cycle (4 aircraft lower bound) reduces the theoretical fleet lower bound by two-thirds. This is not a forecast of any operator's actual fleet size; it is a sensitivity result that identifies the value of longer station service and faster depot turnaround.

The next qualification increment should replace this scalar lower bound with a constrained rotation scheduler that includes recovery/transport/inspection/repair/replenishment/launch stages, limited maintenance bays, spares, launch and recovery throughput, weather windows, transit, staggered schedules and shared-component shortages. After that, use empirical repair-duration distributions and weather-conditioned failure records to reproduce semi-Markov and discrete-event reliability baselines. Report median/p95/p99 turnaround, service interruption, fleet size, reserve ratio, maintenance-bay utilization and cost per continuously served region.

The current Rust module enforces the sequential turnaround components and checks fixture oracle values; its tests are authored but must not be described as executed until exact-head CI completes. It remains an analytical duty-cycle lower bound, not a semi-Markov or operational fleet model.

### 7.4 From fleet-size lower bound to an exact staggered schedule

The next fixture tests whether a lower-bound fleet can be *scheduled* under a small set of explicit operations constraints, rather than relying on the duty-cycle average alone.

[synthetic_haps_fleet_220day_cyclic_rotation_v1.json](examples/synthetic_haps_fleet_220day_cyclic_rotation_v1.json) defines five identical platforms, 30 days on station and a 14-day sequential turnaround, with start phases 0, 8.8, 17.6, 26.4 and 35.2 days. Across 220 days (five complete 44-day cycles), the deterministic event-schedule evaluator reports:
- 100% strict service availability for three required service units;
- 100% capacity adequacy and zero outage hours;
- 25 launch, 25 recovery and 25 reposition events;
- one maintenance bay, one launch slot, one recovery slot and one reposition slot, with no concurrency violation.

The schedule decomposes the 14-day turnaround into 1 day recovery, 1 day depot transfer, 5 days planned maintenance/refurbishment, 2 days ground-ready replenishment/preparation, 1 day launch and 4 days repositioning. Because the maintenance window is 5 days and platform cycle starts are 8.8 days apart, the single maintenance bay is sufficient in this idealized schedule.

**Comparator boundary:** the evaluator also reports the A/B/C primary subset under the same five-platform phase assignment. That subset is not an independently optimized three-platform rotation, so its strict-availability result must not be presented as the best possible no-spare baseline. The exact schedule demonstrates that the five-aircraft lower bound can be realized under this constructed periodic schedule and stated capacities; it does not prove that real launch windows, maintenance data, weather or repairs will permit it.

A negative-control Rust test sets one reserve-assigned platform's service capacity to zero and requires the full schedule to lose strict availability. The cyclic schedule fixture and both tests are included in the HAPS Fleet Reliability workflow path filters.

## 8. Ownership

- Parent HAPS network/persistence: [#5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885).
- Traffic/beam allocation: [#5948](https://github.com/Luminous-Dynamics/symtropy/pull/5948).
- Edge placement/co-scheduling: [#5989](https://github.com/Luminous-Dynamics/symtropy/issues/5989).
- Battery installed-pack evidence: [#3903](https://github.com/Luminous-Dynamics/symtropy/issues/3903).
- Mission electro-thermal model: [Symthaea #11411](https://github.com/Luminous-Dynamics/symthaea/issues/11411).

No flight authority, network service authorization, physical HAPS operation or measured reliability claim is implied by these files.
