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

## 7. Executable analytic evaluator

A standalone Rust evaluator now exists at `tools/haps-fleet-availability`:
- parser validates the synthetic evidence ceiling, probability domains, conditional-independence contract, fleet-size rows and maintenance accounting;
- evaluator computes HAPS-only, HAPS+terrestrial and HAPS+terrestrial+LEO availability;
- the CLI prints a JSON report from the checked-in case or a supplied case path;
- eight unit tests cover all 45 fixture rows, common-weather saturation, layered fallback, correlated external outage, monotonicity, invalid probabilities and maintenance double-counting.

The evaluator is dependency-pinned (`serde 1.0.228`, `serde_json 1.0.150`) with a dedicated path-scoped workflow for rustfmt, Clippy, unit/fixture tests and CLI replay. The source and independent arithmetic were structurally checked, but the Rust workflow has not yet been observed passing in this response. Do not claim compilation or test success until exact-head CI reports success.

## 8. Ownership

- Parent HAPS network/persistence: [#5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885).
- Traffic/beam allocation: [#5948](https://github.com/Luminous-Dynamics/symtropy/pull/5948).
- Edge placement/co-scheduling: [#5989](https://github.com/Luminous-Dynamics/symtropy/issues/5989).
- Battery installed-pack evidence: [#3903](https://github.com/Luminous-Dynamics/symtropy/issues/3903).
- Mission electro-thermal model: [Symthaea #11411](https://github.com/Luminous-Dynamics/symthaea/issues/11411).

No flight authority, network service authorization, physical HAPS operation or measured reliability claim is implied by these files.
