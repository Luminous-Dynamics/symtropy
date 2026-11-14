# AeroWeft — Recovery-Network Resilience and Common-Cause Infrastructure Risk

**Tracking issue:** [AEROWEFT-INFRA-RECOVERY-001 #5224](https://github.com/Luminous-Dynamics/symtropy/issues/5224)  
**Consumers:** Symtropy #1601/#5181/#5186/#5207; Sol-Atlas #189/#825; Symthaea #6249; Symtropy power resilience #3418  
**Status:** research specification / synthetic-first  
**Claim ceiling:** finite infrastructure-portfolio and outage benchmark only; no operational airfield approval, aircraft recovery guarantee, electrical qualification or aviation authority.

## 1. Research thesis

A network can have strong nominal geographic coverage while having very weak recovery diversity. Two mapped landing sites may share one weather region, power feeder, bridge, ground-transfer terminal, source dependency or operating-hour constraint. Different asset IDs do not prove independent failure modes.

AeroWeft should measure whether an aviation infrastructure portfolio preserves **complete, aircraft-applicable recovery chains** under declared single and compound outages—not merely how many candidate sites lie within a nominal radius.

The core research question is:

> Which combination of landing/recovery, energy and ground-transfer resources preserves useful air-ground service under bounded disruptions, and what common-cause dependencies make nominal alternatives fail together?

This work does not create another location-routing mixed-integer optimizer. It defines aviation-specific scenario contracts, recovery-chain semantics, common-cause mappings, a tiny exhaustive benchmark and evidence outputs to be consumed by the existing multimodal optimizer.

## 2. Existing ownership and non-duplication

| Responsibility | Existing owner | This work consumes |
|---|---|---|
| Whole-life multimodal investment and optimization | Symtropy #1601 | Aviation-specific feasibility, cost and resilience constraints |
| Medical air-ground chain and demand fixture | Symtropy #5181 | Synthetic missions, payload handoffs and community outcomes |
| Sequential/batch DCB scheduling | Symtropy #5186 | Candidate schedules and time-dependent resource demand |
| Reciprocal aircraft future options | Symtropy #5207 | Per-aircraft option and recoverability outcomes |
| Reachable occupancy / conflict constraints | Symtropy #5163/#2279/#2297 | Declared aircraft-specific occupancy and resource constraints |
| Fixed-wing physics and applicability | Symthaea #6249 | Versioned aircraft profile, never duplicated here |
| Spatial context, source freshness, rights and vertical datum | Sol-Atlas #189/#825 | Candidate site and infrastructure records with explicit unknowns |
| Power-system and charging resilience | Symtropy #3418 | Qualified energy-service mode and outage evidence |
| Identity/provider/reservation provenance | Mycelix #5023 | Evidence and reservation receipts, not physical or legal truth |

The new issue owns only the aviation-specific recovery-network composition, failure-scenario semantics, common-cause portfolio evaluation and tiny independent oracle. Site geometry, generic investment, aircraft physics, charging electrical behavior, generic cut-set engines, mission-chain logic and flight scheduling remain with their current owners.

## 3. Why this is a real research gap

A 2026 open-access Dubai UAM study jointly optimizes vertiport locations, fleet size, vehicle assignments, routes, multi-trip schedules and partial recharging. It reports a 4–30% profit improvement over the study's selected heuristic strategies. Its authors also identify important simplifications: deterministic demand, omission of simultaneous landing/parking capacity, and assumed adequate simultaneous charging plugs. The reported profit lift is not a transferable AeroWeft target and is not a directly matched executable baseline until the data, exact implementation and assumptions are pinned.

A 2026 scoping review of 81 UAM vertiport-planning publications finds that network-level siting, facility design, ground access, noise, weather, equity and governance remain fragmented. It identifies limited transferable design parameters for rooftop operations, charging layouts, passenger transfer and multi-scale governance.

AeroWeft's differentiator should therefore not be another integrated siting-and-routing model. It should test whether nominally attractive infrastructure portfolios remain recoverable when asset dependencies and common-cause disruptions are represented explicitly.

References:

- [An integrated optimization framework for UAM: vertiports, fleet size, routes and recharging (2026)](https://doi.org/10.1016/j.trip.2026.101945)
- [Urban air mobility vertiports: scoping review of physical site selection and ground integration (2026)](https://www.sciencedirect.com/science/article/pii/S2667091726000828)
- [Symtropy TRANSPORT-OPT-001 #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601)
- [Sol-Atlas AIRSPACE-ATLAS-001 #189](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/189)

## 4. Coverage is not recoverability

A site is not an eligible recovery resource merely because a point or polygon exists. The benchmark must keep separate claims for:

- feature existence and geometric completeness;
- horizontal CRS, axis order, vertical datum, units and transformation lineage;
- source authenticity, freshness, validity and rights;
- operational status and usable time window;
- capacity and reservation conflicts;
- aircraft-profile and operating-regime applicability;
- landing/approach/obstacle/weather constraints;
- energy/charging/fuel service where the next mission requires it;
- ground access, transfer and handoff continuity;
- common-cause dependencies and failure-domain completeness.

An emergency recovery location need not be a scheduled vertiport. A landing surface does not imply charging capability. A charger does not imply electrical resilience. A point geometry does not establish permission or capacity. Unknown, stale, unsupported, contradictory, rights-pending or missing required evidence makes that particular claim unresolved or unavailable; it never becomes a permissive default.

## 5. Recovery-chain contract

A recovery chain for a declared mission class may include:

1. Aircraft-specific occupancy and arrival evidence from Symthaea #6249 and Symtropy #5163.
2. A site record with aircraft applicability, source and time validity, geometry/datum, operating state and capacity.
3. Required approach, airspace, obstacle, weather and hazard constraints from admitted Sol-Atlas snapshots.
4. Energy service only where the declared aircraft/mission policy requires energy for turnaround, onward travel or return.
5. Ground-transfer capability when the service outcome requires transfer to a facility/community.
6. Payload and custody continuity from Symtropy #1887/#1636.
7. Source and dependency lineage for every required site, feeder, charger, road/bridge and service.

A chain is counted only when every required link is available for the exact scenario and validity interval. A mapped site with an unknown operational state is not counted. A power outage can invalidate a multi-sortie chain that requires recharging, while leaving a separately declared immediate landing-only recovery claim intact. These claims must not be collapsed into one site-level boolean.

## 6. Common-cause and outage semantics

V0 uses bounded adversarial scenarios. Do not invent failure probabilities. Every scenario declares:
- exact asset IDs affected;
- exact failure-domain IDs;
- time interval, geography, persistence and restoration state;
- source/model/observation status;
- correlated consequences and the evidence supporting that correlation.

Potential shared dependencies include:
- one weather/visibility/wind region affecting multiple candidate sites;
- a common grid feeder, transformer, switchgear or charger controller;
- one bridge, access road, ground terminal or depot;
- common staffing, operating window or maintenance provider;
- shared communications, source publisher, API, datum transformation or update service.

Different asset IDs, vendors or coordinates are not evidence of statistical independence. Where a failure-domain inventory is incomplete, report an incomplete resilience result rather than a numeric cut set based on the known subset.

For the finite benchmark, define a **minimal failure set** as a set of declared failure-domain events that is sufficient to remove every admissible recovery chain for a specific mission/target, while no proper subset of that set does so under the frozen model. This is a conditional graph property, not a real-world probability or reliability estimate.

## 7. First synthetic benchmark

The frozen starter portfolio contains three candidate sites:

- **North Hub A** and **North Hub B**: distinct nominal sites but sharing a synthetic coastal weather domain, power feeder and access bridge.
- **South Hub C**: different declared weather, power and road domains.
- One additional geometry-only site with unknown operational status, which must never count as usable.

The benchmark compares two portfolios with equal site count:

1. **Nominal-coverage portfolio A+B:** lower nominal cost, but all alternatives share the same three declared failure domains.
2. **Recovery-diverse portfolio A+C:** higher nominal cost but different declared weather, power and ground-access domains.

Each site has a finite capacity, a source/admission state, an aircraft applicability profile, an energy resource and a ground-transfer resource. The synthetic remote-clinic service floor requires at least one complete chain to remain in every declared singleton failure scenario.

A compound scenario affecting North Hub A's weather region and South Hub C's power domain is included to show that a diverse portfolio is not invulnerable. It is simply more resilient under the declared bounded fault model.

All geography, site status, aircraft profiles, capacities, costs, energy values, weather and demand are fictional. The machine-readable fixture is [fixtures/aeroweift-infrastructure-resilience/tiny-portfolio-v0.json](../../../fixtures/aeroweift-infrastructure-resilience/tiny-portfolio-v0.json).

## 8. Tiny exhaustive-oracle obligations

For the smallest portfolio, independently enumerate:
- all candidate site subsets;
- aircraft/mission-specific site applicability;
- each declared failure-domain combination;
- availability of the complete site + energy + transfer chain;
- whether a named demand/service floor is met;
- every minimal failure set for each portfolio;
- every unavailable chain and the exact failure-domain witness that removed it.

The oracle must preserve correlated/common-cause domains from the fixture. It must not cross-product mutually exclusive or physically incompatible events. The implementation under test must not call the oracle to generate expected outcomes.

The fixture's expected result is:

- A+B has three singleton minimal failure sets: coastal weather, north feeder, and north bridge.
- A+C has nine minimal two-domain cut sets: each of the three North A failure domains paired with each of the three South C failure domains.
- A+C survives every declared singleton failure; it loses the remote-clinic chain under the named compound North-weather + South-feeder outage.
- The unknown-status geometry-only site contributes no recovery option.

These are exact properties of the fictional finite fixture only.

## 9. Non-compensatory gates

- Unknown site status, capacity, datum, source validity or rights never becomes usable by default.
- Missing restriction geometry is not unrestricted airspace.
- No site is promoted to aircraft applicability without the profile-specific evidence.
- Charging is required only where the declared mission/profile says it is required.
- No backup, islanding or black-start capability is inferred from the presence of a generator or battery; consume the exact #3418 qualified mode/evidence.
- Common-cause groups must be preserved; distinct IDs do not imply independence.
- A portfolio fails a named community or mission service floor even if aggregate coverage is high.
- A synthetic reservation is not a flight authorization or dispatch decision.
- Hard-gate failures cannot be offset by profit, throughput, coverage or an aggregate resilience score.

## 10. Matched baselines and measurements

Use identical demand, aircraft profiles, site facts, source snapshots, cost assumptions, scenario set and compute budget:

1. nearest-site / demand-only coverage;
2. deterministic capacity-aware site selection;
3. site-count or nominal-coverage maximization;
4. published integrated siting/fleet/routing/recharging algorithm class;
5. scenario-robust portfolio selection through the existing #1601 optimizer;
6. recovery-diverse AeroWeft aviation constraints evaluated by the same optimizer.

Report separately:

**Resilience**
- service-floor pass/fail by named community/mission;
- recovery-chain count and exact chain identities by scenario;
- all minimal failure sets and their witnesses;
- common-cause overlap between nominal alternatives;
- scenario coverage and unresolved/incomplete scenarios;
- simultaneous diversion/landing/charging contention;
- restoration time only where a declared repair model exists.

**Public value and lifecycle**
- annualized and lifecycle investment;
- cost per service class protected;
- nominal and disrupted door-to-door service;
- energy and emissions per delivered unit where the accounting boundary supports them;
- ground-transfer continuity, affordability and remote-community coverage;
- land-use/noise/ground-risk constraints with explicit limitations.

**Computation and evidence**
- exact exhaustive-oracle agreement;
- scenario/failure-set enumeration counts;
- p50/p95/p99 runtime and memory;
- input/profile/source/toolchain manifests;
- stale, unsupported, rejected and unresolved records;
- deterministic replay.

No weighted scalar score can compensate for failure of a hard service or evidence gate.

## 11. Falsification and metamorphic controls

- A+B has two site IDs but one shared weather domain; the evaluator must find the common-cause outage.
- A+B shares a power feeder; a feeder outage invalidates both energy-dependent service chains.
- A+B shares a bridge; a bridge outage removes both ground handoffs.
- A+C has distinct dependencies and survives each singleton failure.
- A+C fails under the declared two-domain compound outage.
- Geometry-only unknown-status site does not add a chain.
- Unknown charger status does not imply unlimited charging.
- Rights-pending or superseded source does not support a live/usable claim.
- Adding a genuine independent compatible resource cannot shrink the robust feasible set under the declared ordering.
- Adding a failure cannot create a new recovery chain.
- Permuting records or relabelling synthetic IDs does not alter semantic results.
- Incomplete failure-domain inventory returns incomplete, not a deceptively optimistic numeric cut set.
- Exact replay from frozen manifests reproduces all chain and cut-set witnesses.

## 12. Acceptance targets

- 100% agreement with an independently authored exhaustive oracle on the frozen tiny portfolio.
- 100% exact minimal-failure-set identification on that corpus.
- Zero unqualified, unknown, stale or rights-pending resources admitted as available.
- Zero common-cause dependencies hidden by different asset IDs.
- 100% explicit service-floor outcomes for named groups.
- Exact deterministic output under ordering and ID relabelling.
- No probabilistic reliability claim without admitted event-frequency evidence, a defensible sampling frame and calibration.
- Full census of unsupported, incomplete, stale and rejected cases.

These are synthetic benchmark targets, not real-world aviation reliability, operational safety or regulatory qualification.

## 13. Execution sequence

1. Freeze the site/resource/failure-domain graph and exact outage scenarios.
2. Bind source-backed site claims to Sol-Atlas #189/#825; synthetic fixture records remain visibly synthetic.
3. Bind aircraft and mission applicability to Symthaea #6249 and AeroWeft #5163.
4. Bind charging/power-mode claims to Symtropy #3418; do not reimplement electrical qualification.
5. Bind end-to-end cargo outcomes to #5181/#1887/#1636 and candidate flight allocations to #5186/#5207.
6. Implement the smallest independent exhaustive portfolio/outage oracle.
7. Export aviation-specific constraints and outcomes into the existing #1601 optimizer.
8. Compare matched portfolios and Pareto trade-offs only after the non-compensatory gates pass.

**Core invariant:** two mapped alternative landing sites are not a resilient recovery network until their aircraft applicability, time validity, capacity, energy/access requirements and common-cause dependencies are explicit and qualified.
