# Maritime manoeuvring and port-operations industry baselines

Research snapshot: 2026-11-06. This file records comparator selection, not a claim that Symtropy has beaten any external solver, simulator or deployed port platform.

## 1. Standards and practice baselines

| Reference | What it governs | Correct use in this programme |
|---|---|---|
| IMO MSC.137(76), Standards for Ship Manoeuvrability (2002) | Standard manoeuvring criteria and tests, including turning circle, zig-zag and stopping | Reproduce the prescribed test conditions. Do not reinterpret calm/deep-water criteria as restricted-channel or harbour acceptance. |
| ITTC 7.5-02-06-03, 2024 Revision 05 | Validation of manoeuvring simulation models | Preserve its staged validation: mathematical model / forces; deep-water standard manoeuvres; shallow-water standard manoeuvres; restricted-water manoeuvres; free-running trajectories; pilot/expert assessment. |
| PIANC WG 171, September 2026 | Current ship-handling simulation workflow for channel and harbour design | Use as the study workflow / context-of-use baseline. PIANC guidance is not certification or a statutory standard. |
| IHO S-100 product family | Hydrographic data product semantics | Pin exact product-spec editions. S-102 bathymetry, S-104 water levels and S-111 surface currents are evidence inputs, not navigation clearance. |
| DCSA Port Call 2.0.0 | Port-call event exchange for planned/estimated/actual service times and container move counts | Implement the public event semantics rather than inventing new event names. Authentication and authorization are explicitly outside the API specification's scope. |

Authoritative links:

- IMO MSC.137(76): https://wwwcdn.imo.org/localresources/en/KnowledgeCentre/IndexofIMOResolutions/MSCResolutions/MSC.137%2876%29.pdf
- ITTC 7.5-02-06-03 (2024 Rev. 05): https://ittc.info/media/11868/75-02-06-03.pdf
- PIANC WG 171 (September 2026): https://www.pianc.org/publication/ship-handling-simulation-dedicated-to-channel-and-harbour-design/
- IHO standards in force: https://iho.int/en/enc-ecdis
- DCSA Port Call 2.0.0 OpenAPI: https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-openapi-specification

The ITTC procedure identifies hydrodynamic force prediction, model structure, numerical method, software, time/space resolution and experimental setup as contributors to prediction uncertainty. A single trajectory match must not be treated as universal model validity.

## 2. Current 2026 research comparators

| Study | Modelled scope | Reported result / comparator | Gap it sets for our benchmark |
|---|---|---|---|
| Xu et al., Frontiers in Marine Science (9 Sep 2026), DOI 10.3389/fmars.2026.1939678 | Dual one-way channels; vessel traffic organization; heterogeneous tug scheduling and repositioning; bi-objective waiting time / tug fuel | NSGA-II with archive-guided adaptive multi-neighbourhood search compared with CPLEX, NSGA-II and MOSA | Must include multiple channels, heterogeneous capability, tug location/repositioning and multi-objective evaluation. |
| Sun et al., Ocean Engineering (2026), DOI 10.1016/j.oceaneng.2026.127433 | Vessel scheduling plus multi-task tug assignment, including pilot transfer, escort and berthing/unberthing continuity | Real-port case reports 22.73% lower vessel time in port and 12.82% lower tug fuel than that port's traditional rules | Pilot transfer must be an explicit task with location, time, resource and continuity—not an abstract pilot-count constraint. |
| Liu et al., Transportation Research Part E (2026), DOI 10.1016/j.tre.2026.105144 | Joint vessel sequencing, pilot assignment and tug allocation with multi-segment navigation and channel restrictions | Qinzhou case combines MILP decomposition, column generation and reinforcement-learning subproblems; reports solutions within 15 minutes and about RMB 232,000 daily savings | Include integrated VTS decomposition and scale/runtime comparators, not only monolithic MILP. |
| Duan et al., Ocean Engineering (30 Oct 2026), DOI 10.1016/j.oceaneng.2026.128093 | Multi-basin / multi-anchorage port with one-way channel; traffic sequencing and berth allocation; safety intervals, navigation continuity and tidal windows | MILP plus heuristic comparator | Include basin/anchorage/turning-basin topology, berth compatibility, navigation continuity and tidal feasibility. |

These are independent studies with different ports, demand mixes, definitions, assumptions and objective normalization. Their percentages, monetary savings, hypervolume and IGD values are **not directly comparable**. Use published figures as evidence that a capability exists and as a guide to case design—not as a performance target copied into a different problem.

## 3. Commercial interoperability baseline

A basic ETA dashboard, vessel queue, berth calendar or tug-assignment UI is not a defensible novelty claim. Commercial port-call products already support shared operational pictures and coordination of services such as berthing, pilotage, towage, cargo operations and bunkering. DCSA Port Call 2.0.0 provides a standard event vocabulary and API for interoperable exchange.

The Luminous differentiator must therefore be tested at the system boundary:

- physical/manoeuvring assumptions are tied to model-specific, evidence-qualified operating envelopes;
- schedule feasibility is recomputed by an independent oracle for small cases;
- water, energy, cargo and service ledgers are explicit and do not double-count;
- source currentness, datum, units, rights, forecast cutoff and uncertainty remain attached;
- stale, conflicting, unsupported or out-of-domain evidence leads to abstention or a bounded contingency—not an optimistic schedule;
- achieved outcomes are reconciled against predictions and ex-post evidence.

DCSA PC 2.0.0 is an interoperability baseline, not an authorization system: the specification explicitly leaves authentication and authorization out of scope.

## MASS operating-domain and port-digital-twin baseline

The IMO adopted the non-mandatory International Code of Safety for Maritime Autonomous Surface Ships (MASS Code, resolution MSC.595(111)) in May 2026; it took effect on 1 July 2026. The IMO identifies vessel-specific modes of operation, risk assessment, navigation, connectivity, remote operations, cybersecurity, emergency response and human oversight as central concerns. The master retains overall responsibility even when not onboard. The IMO FAQ says the mode description should define its conditions and limits and actions when conditions leave those limits. The MASS Safety Certificate requires an approval process; enhanced automation alone does not make a ship a MASS.

Authoritative sources:
- IMO adoption announcement (22 May 2026): https://www.imo.org/en/mediacentre/pressbriefings/pages/imo-adopts-mass-code.aspx
- IMO autonomous shipping FAQ: https://www.imo.org/en/mediacentre/hottopics/pages/autonomous-shipping.aspx
- IMO MSC 111 summary: https://www.imo.org/en/mediacentre/meetingsummaries/pages/temporary-msc-111.aspx

**Implication:** a maritime digital twin needs more than a vessel state estimate or an optimizer. It needs configuration-bound, function-specific, time-valid mode envelopes; named human/remote responsibility; explicit transition/acknowledgement and recovery semantics; and clear abstention when evidence becomes stale or conditions fall outside the declared domain.

The new workstream [MARITIME-ODD-001 #3893](https://github.com/Luminous-Dynamics/symtropy/issues/3893) owns this maritime profile. It reuses Symthaea SE-RTA #3779 for generic runtime assurance and Mycelix mobility configuration/authority objects. It does not implement navigation control, an autonomous-vessel controller or MASS certification.

A 3 April 2026 *Journal of Shipping and Trade* port-authority digital-twin adoption study draws on 77 Port of Barcelona interviews and 12 international-port interviews, identifies 80 use cases and evaluates them against nine criteria. It reports that many port authorities remain early in adoption, with high implementation costs and uncertain benefits constraining deployment. It supports prioritizing decision value relative to data/integration cost—not an assumption that comprehensive port twins are already standard practice:
https://doi.org/10.1186/s41072-026-00234-6

An April 2026 *Maritime Economics & Logistics* paper on the EU CRISTAL project couples inland-waterway water-level forecasting with port-call coordination. It highlights data-granularity and interoperability limits and provides a useful comparator for information-led reliability improvements before infrastructure expansion:
https://doi.org/10.1057/s41278-026-00352-9

A June 2026 *Marine Geology* study reports adaptive Delft3D sediment-transport calibration across storm/calm regimes, compared with static calibration and validated with ADCP and RTK-GPS observations at Nushahr Beach. This supports regime-conditioned evaluation, but only with frozen calibration/hold-out groups and transition-window tests; the reported site-specific gain is not portable as a universal target:
https://doi.org/10.1016/j.margeo.2026.107739

## 4. Matched comparison protocol

### Optimizer arms

B0. FCFS / FIFO with declared tie-breaking.

B1. Current-practice baseline, where operational data and decision rules are available and rights permit reproduction.

B2. Earliest-feasible / first-available resource heuristic.

B3. Published or faithful reimplementation of a relevant integrated heuristic (e.g. NSGA-II, MOSA, NSGA-II-AAMNS, ADP-MLNS), with matched constraints and the same objective definitions.

B4. MILP / HiGHS reference for the exact finite formulation; use solver bounds and optimality gaps on larger cases.

B5. Symtropy optimizer under evaluation.

B6. Robust / stochastic or contingency-aware arm when uncertainty and disruption distributions are explicitly specified.

B7. Independent exhaustive oracle for tiny cases. This is a correctness reference, not a scalable competitor.

Never compare unlike formulations. Record every omitted constraint, approximation, solver limit, time limit and termination status.

### Required instance families

- Tiny exact cases: 2–8 vessels, 1–6 mobile resources and small conflict graphs.
- Literature-scale cases: at least 10–40 vessels where the reproduced paper exposes a compatible instance family.
- Topology stress: dual one-way channels, shared approaches, multiple anchorages, multiple basins, turn basins and berth conflicts.
- Task continuity: pilot transfer, escort, berthing/unberthing and tug repositioning.
- Time dependence: tide/current windows, forecast validity and vessel arrival uncertainty.
- Disruption: tug outage, pilot unavailability, channel closure, berth outage, stale evidence and conflicting observations.
- Service chain: port-call completion plus cargo/rail/inland handoff, not simply vessel departure.

Use frozen instances, identical information cutoffs and deterministic seeds. For randomized algorithms, publish distributions over multiple seeds, not a best run.

### Report separately

**Physical/model validation**
- per-observable trajectory error (position, heading, yaw rate, speed);
- advance, transfer, tactical diameter, zig-zag overshoot and stopping metrics under the applicable test definitions;
- errors by vessel / hull / scale / loading / depth-draught / speed / environment;
- uncertainty coverage and interval width by adequately sampled subgroup;
- unsupported operating envelope and abstention rate.

**Scheduling and service**
- feasibility and hard-constraint violations;
- total and weighted waiting, P50/P95/max wait, and wait by participant/service class;
- service-window misses, cargo delivered within window, end-to-end delivery time;
- tug utilization, reposition distance/time, tug fuel/energy and emissions boundary;
- berth/channel/lock throughput with denominator and time window specified;
- water per lockage and water per delivered cargo unit where relevant;
- recovery time, unmet demand and decision regret under disruptions.

**Optimization quality**
- exact-oracle agreement on tiny cases;
- incumbent objective, best bound and optimality gap for MILP;
- hypervolume / IGD against a fixed, published reference frontier with declared normalization;
- runtime, memory, instance size and termination state;
- variability over seeds and sensitivity to workload density.

**Evidence integrity**
- stale/missing/unsupported rejection rate;
- time leakage and held-out campaign leakage;
- exact input / code / config / solver digests;
- independent verifier result;
- explicit PASS / FAIL / INCOMPLETE / NOT-RUN. Queued, skipped or stale-head evidence is not PASS.

## 5. Where Symtropy can credibly outperform

The defensible hypothesis is not “we have a better genetic algorithm.” It is:

1. Jointly optimize vessel, pilot, tug, channel, basin, berth and onward service decisions.
2. Restrict the optimizer to externally qualified physical operating envelopes, rather than letting it infer vessel manoeuvrability from route geometry.
3. Preserve time-valid hydrographic and forecast evidence, including vertical datum and uncertainty.
4. Optimize a vector of service, resource and resilience outcomes; do not hide trade-offs in arbitrary weights.
5. Verify tiny cases with an independent exhaustive oracle and larger MILP/heuristic comparators.
6. Measure decision regret and realized end-to-end service, including disruption and fairness.

Initial improvement targets should be declared only after the exact matched baseline has been reproduced. The 22.73% / 12.82% and 28.31% reported improvements in separate papers are useful evidence of attainable gains in their respective cases, not universal SOTA thresholds.

## 6. Workstream links

- Symtropy manoeuvring evidence contract: https://github.com/Luminous-Dynamics/symtropy/issues/3865
- Symtropy MASS operating-domain profile: https://github.com/Luminous-Dynamics/symtropy/issues/3893
- Draft contract / independent metric slice: https://github.com/Luminous-Dynamics/symtropy/pull/3878
- Symtropy mobile pilot/tug and spatial traffic scheduling: https://github.com/Luminous-Dynamics/symtropy/issues/3798
- Symtropy coupled coastal-to-inland service-chain benchmark: https://github.com/Luminous-Dynamics/symtropy/issues/3683
- Symthaea physical reference intake: https://github.com/Luminous-Dynamics/symthaea/issues/8146
- Symthaea calibration under marine regime shift: https://github.com/Luminous-Dynamics/symthaea/issues/8171
