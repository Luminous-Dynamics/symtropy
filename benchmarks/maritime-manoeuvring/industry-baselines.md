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

## 2026 IHO operational product profiles

The IHO's Standards-in-Force register was updated on **30 October 2026**. Besides the established navigation/hydrographic products, it now lists several profiles directly relevant to port and MASS operating-domain evidence:

| IHO product | Edition / in-force date | Useful evidence role | Claim ceiling |
|---|---|---|---|
| S-102 Bathymetric Surface | 3.0.0 / 1 Jan 2026 | Bathymetry with product-specific coverage/quality metadata | Not an under-keel or navigability verdict by itself |
| S-104 Water Level Information for Surface Navigation | 2.0.0 / 1 Jan 2026 | Time-bounded water-level inputs | Not a forecast validity or clearance policy by itself |
| S-111 Surface Currents | 2.0.0 / 1 Jan 2026 | Surface-current inputs | Does not establish full-depth currents or manoeuvring force response |
| S-122 Marine Protected Areas | 2.0.0 / 10 Oct 2026 | Protected-area spatial evidence | Not a complete ecological impact / authorization record |
| S-123 Marine Radio Services | 2.0.0 / 10 Oct 2026 | Radio-service locations/services | Not proof of live end-to-end link coverage, latency or ROC availability |
| S-127 Marine Traffic Management | 2.0.0 / 10 Oct 2026 | VTS, pilotage, routing-measure and ship-reporting service context | Simplified supplementary geometry; not detailed ENC, live resource status or route authorization |
| S-129 Under Keel Clearance Management | 2.0.0 / 1 Jan 2026 | Under-keel clearance data/product context | Does not independently validate a local clearance model or authorize a passage |
| S-131 Marine Harbour Infrastructure | 2.0.0 / 10 Oct 2026 | Harbour infrastructure product references | Not actual live berth availability, structural adequacy or manoeuvring approval |

IHO sources:
- Standards in force, page updated 30 October 2026: https://iho.int/en/standards-in-force-0
- S-127 registry detail: https://registry.iho.int/productspec/view.do?category=product_ID&domainS=ALL&idx=232&product_ID=S-127&searchValue=&statusS=5

The S-127 registry explicitly describes VTS, pilotage, routing measures and ship-reporting systems, including service areas and service information, and notes that the product is supplementary to ENC with simplified geometry. This is useful for #3798 mobile-resource context and #3893 mode/communications evidence, but is not a dynamic port-operation authority.

All product use must pin the exact IHO product specification and edition, S-100 framework version, producer/source identity, jurisdiction, dataset/cell, issue/update sequence, coverage, validity/currentness, datum/CRS, quality/uncertainty, licence/rights and exact raw bytes/digest. A schema-conforming product is not proof of physical correctness, source authenticity, currentness, licence or authorization.


### Restricted-water and interaction benchmark families in ITTC

The ITTC benchmark catalogue is more useful than a single generic “restricted-water” label. Procedure 7.5-02-06-06 (2024 Rev. 01, pp. 7–8 and benchmark summary table on p. 12) identifies distinct MASHCON data families:

- **Bank effects:** captive tests of a single-screw 800-TEU container carrier with surface-piercing banks and banks with a submerged platform/slope configuration.
- **Lock approach/departure:** free-running data for a 12,000-TEU container ship at 1:80 model scale and captive data for a 1:75 bulk-carrier model.
- **Lightering / ship-to-ship interaction:** five static tests and one dynamic rudder test with an Aframax service ship and a KVLCC2 vessel being lightered.
- **Passing-ship interaction:** a KCS passing close to moored Neo-Panamax container and Aframax tanker hulls, including close-distance and/or high-speed conditions.

These must be separate benchmark families with their own state variables, geometry, measurements and uncertainty. Bank suction/cushion, lock-approach effects, lightering interaction and passing-ship loads are not interchangeable. A free-running lock-approach record is also not equivalent to captive force data. The ITTC catalogue identifies these data sets, but each artifact still needs exact-byte, source, rights/access and metadata review before use in a reproducible run.

Source: ITTC 7.5-02-06-06 (2024 Rev. 01), pp. 7–8 and p. 12: https://www.ittc.info/media/11874/75-02-06-06.pdf

## 2. Current 2026 research comparators

| Study | Modelled scope | Reported result / comparator | Gap it sets for our benchmark |
|---|---|---|---|
| Xu et al., Frontiers in Marine Science (9 Sep 2026), DOI 10.3389/fmars.2026.1939678 | Dual one-way channels; vessel traffic organization; heterogeneous tug scheduling and repositioning; bi-objective waiting time / tug fuel | NSGA-II with archive-guided adaptive multi-neighbourhood search compared with CPLEX, NSGA-II and MOSA | Must include multiple channels, heterogeneous capability, tug location/repositioning and multi-objective evaluation. |
| Sun et al., Ocean Engineering (2026), DOI 10.1016/j.oceaneng.2026.127433 | Vessel scheduling plus multi-task tug assignment, including pilot transfer, escort and berthing/unberthing continuity | Real-port case reports 22.73% lower vessel time in port and 12.82% lower tug fuel than that port's traditional rules | Pilot transfer must be an explicit task with location, time, resource and continuity—not an abstract pilot-count constraint. |
| Liu et al., Transportation Research Part E (2026), DOI 10.1016/j.tre.2026.105144 | Joint vessel sequencing, pilot assignment and tug allocation with multi-segment navigation and channel restrictions | Qinzhou case combines MILP decomposition, column generation and reinforcement-learning subproblems; reports solutions within 15 minutes and about RMB 232,000 daily savings | Include integrated VTS decomposition and scale/runtime comparators, not only monolithic MILP. |
| Duan et al., Ocean Engineering (30 Oct 2026), DOI 10.1016/j.oceaneng.2026.128093 | Multi-basin / multi-anchorage port with one-way channel; traffic sequencing and berth allocation; safety intervals, navigation continuity and tidal windows | MILP plus heuristic comparator | Include basin/anchorage/turning-basin topology, berth compatibility, navigation continuity and tidal feasibility. |

These are independent studies with different ports, demand mixes, definitions, assumptions and objective normalization. Their percentages, monetary savings, hypervolume and IGD values are **not directly comparable**. Use published figures as evidence that a capability exists and as a guide to case design—not as a performance target copied into a different problem.

### Detailed reproducibility note — Xu et al. 2026

The paper's experiment section gives useful settings for a future matched recreation:

- The survey-derived case uses 10 tugboats, waypoint-specific repositioning times, vessel-type tug rules and a 13-minute minimum vessel separation.
- In the small S10 and S22 groups, the merged AAMNS Pareto front matches CPLEX on 8 of 10 instances and is described as negligibly different on the other 2. Reported mean runtimes: CPLEX 166.73 s vs AAMNS 44.19 s (S10), and 2,444.02 s vs 74.94 s (S22).
- For S28 and S40, five instances per group and 10 runs per algorithm per instance are reported. Mean HV/IGD are AAMNS 0.832/0.044, MOSA 0.588/0.226, NSGA-II 0.552/0.257. Average S28/S40 runtimes are 106.19 s, 130.07 s and 116.91 s respectively (AAMNS, MOSA, NSGA-II). The paper reports paired Wilcoxon p < 0.001 for instance-wise mean HV/IGD.
- Environment: MATLAB R2022b, Intel Core i7-6700 CPU, 16 GB RAM. Vessel/tug/berth IDs were anonymized; records were derived from field surveys.
- Their objective normalization pools all algorithms' runs per instance, uses the pooled non-dominated set as IGD reference and HV point (1,1). Their service-level upper bound is FCFS weighted wait.

A parity reproduction must repeat the published normalization, data-derived constraints, FCFS bound, run count and solver settings as closely as the available artifacts allow. Our cross-study benchmark must separately use a preregistered fixed normalization/reference point. Report both. These paper-specific numbers are not evidence that Symtropy has been outperformed or surpassed yet.

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


## Commercial port-call and port-management platforms

The competitive set extends beyond ship-handling solvers. Established platforms already coordinate berth plans, ETAs, port calls and mobile services; a Luminous claim to be the first integrated JIT/port-call platform would be incorrect.

| Platform | Publicly stated capability | Benchmark implication |
|---|---|---|
| **StormGeo + Portchain** | StormGeo announced acquisition of Portchain on 30 Oct 2026. Portchain Connect provides berth alignment and digital schedule/data handshake between terminals and carriers; Portchain Quay covers berth, crane and workforce planning. The announcement says its network links 200+ terminals, representing over 40% of global container-terminal capacity (company-reported). StormGeo brings voyage/weather optimization, positioning the combined group to connect terminal berth readiness with at-sea speed/arrival decisions. | Treat as a direct JIT arrival + berth-alignment competitor. Benchmark ETA/ETB alignment, data freshness, change propagation, speed advice, achieved waiting/fuel and decision latency. The acquisition creates a strong reference for sea-to-port coordination, not proof that the combined product already optimizes every tug/channel/lock/inland constraint. |
| **PortXchange Synchronizer** | Real-time berth-plan updates, vessel speed adjustment to reach pilot boarding/terminal at the planned time, API integration and deviation alerts. | Benchmark update latency, data authorization, arrival prediction and JIT speed-plan response to berth-plan changes. |
| **Awake.AI Port Vision** | Publicly describes port-call planning/JIT, berth planning, resource scheduling, geofenced maps, weather warnings, event/chat handling and infrastructure management. Its site reports >1.5m voyage predictions/day and analytics configured for >3,000 ports; these are provider-reported scale claims, not independent accuracy results. | Benchmark ETA calibration and tail errors, resource assignment, update cadence, coverage, forecast horizon, workload/cost and outcomes, not raw prediction volume alone. |
| **Kongsberg PMIS** | Integrated port-management system for vessel traffic, berth/anchorage, tug and pilot utilization KPIs, JIT arrivals, planning and validated Master-Pilot Exchange information. | Benchmark resource utilization, VTS/PMIS interoperability, role/authority separation, data quality and operational workflow integration. |
| **DCSA Port Call 2.0 / IAPH port-call guidance** | Common port-call event/API and data-exchange semantics. | Conform to interoperable event/timestamp vocabulary and map source events faithfully. Do not invent a competing data standard or assume API conformance establishes authorization or operational truth. |

Public source material:
- StormGeo acquisition announcement, 30 Oct 2026: https://portchain.com/insights/stormgeo-acquires-portchain-to-advance-just-in-time-voyage-optimization-at-scale
- Portchain Connect: https://portchain.com/portchain-connect
- PortXchange Synchronizer: https://port-xchange.com/synchronizer/
- Awake.AI Port Vision: https://www.awake.ai/
- Kongsberg PMIS: https://www.kongsberg.com/what-we-do/ocean-space/surveillance-monitoring/foresight/pmis/
- DCSA Port Call OpenAPI: https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-openapi-specification

**What remains differentiating for Luminous:** not a shared berth calendar, ETA forecast, digital handshake, or generic multi-resource optimizer. The research hypothesis is an independently auditable decision fabric connecting exact vessel/configuration identity, validated manoeuvring/ODD envelopes, current hydrographic and environmental evidence, spatial conflict topology, tug/pilot capability and position, port/lock/berth/energy/water capacity, cargo custody and inland delivery. This must be verified with counterfactual service-chain tests and ex-post realized outcomes; it is not established by the presence of an evidence schema.

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


## 2026 open manoeuvring benchmark landscape

Two particularly useful public comparison anchors are now explicit:

| Comparator | Declared scope | What it can establish | What it cannot establish |
|---|---|---|---|
| ITTC 7.5-02-06-06 (2024 Rev. 01) / SIMMAN 2014 and 2020 | Public benchmark families with captive PMM/CMT and free-running data for KVLCC2, KCS and ONRT; deep/shallow cases and selected regular-wave manoeuvres | Common experimental references for forces, moments and trajectories; uncertainty-aware validation and reproducible matched cases | Full-scale validation for every vessel; complete restricted-water/harbour coverage; a single universal solver ranking |
| OpenNAOE Manoeuvring v1.0.0 (2 Nov 2026), DOI 10.5281/zenodo.23817277 | MMG standard method, conventional single-propeller/single-rudder ship, calm deep water, 3-DOF surge/sway/yaw; turning and zig-zag, but not stopping or overall compliance | A recent, transparent, easily inspectable educational/early-design comparator within its declared domain | Merchant-ship full-scale or shallow/restricted-water validation; 4/6-DOF physics; evidence of class/IMO compliance or specialist commercial-grade capability |
| MITO 31 RIB sea-trial dataset (8 Jul 2026), DOI 10.17632/pz37h8vtjt.1 | Full-scale two-stepped high-speed planing RIB; raw IMU/GNSS/NMEA2000 and two turning manoeuvres | A potential independent sensor/trajectory processing and high-speed craft transfer test | A substitute for conventional merchant vessels or SIMMAN benchmark families; unrestricted reuse absent a verified licence/rights record |

Sources:
- ITTC benchmark guidance, 7.5-02-06-06 (2024 Rev. 01): https://ittc.info/media/11874/75-02-06-06.pdf
- ITTC benchmark repository: https://www.ittc.info/benchmark-repository/
- SIMMAN 2020: https://www.simman2020.kr/contents/Purpose.php?me_code=10
- OpenNAOE Manoeuvring: https://zenodo.org/records/23817277
- MITO 31 dataset: https://data.mendeley.com/datasets/pz37h8vtjt/1

Important ITTC findings for the programme:
- Its benchmark guidance says uncertainty assessment is strongly recommended and that even a reduced uncertainty-assessment scope is preferable to omitting it.
- In the cited SIMMAN comparisons, 3-DOF methods were inadequate for KCS manoeuvring predictions; low-GM cases may need at least 4 DOF. The ITTC procedure states that 4- or 6-DOF is appropriate where vertical motions matter, including high-speed vessels, low-GM vessels and manoeuvring in waves.
- SIMMAN 2014/2020 includes both free-running and captive data. Deep-water and shallow-water cases must be separate strata; the guideline reports greater variation and fewer submissions for shallow-water cases.
- These findings support a regime- and mechanism-conditioned model hierarchy, not a blanket requirement to use the most complex model everywhere.

**Benchmark design consequence:** compare the output metric verifier and the physical model as separate subjects. The verifier can be assessed against hand-derived geometry and independent calculations. A simulator must be assessed against experimental time histories (and force/moment data where available), with calibration and hold-out cases kept separate. A green metric-oracle suite says nothing by itself about hydrodynamic model accuracy.

No external dataset is copied into CI or treated as a mandatory fixture until source bytes, metadata, units, data-quality notes, provenance, licence and redistribution rights are reviewed and pinned. MITO 31 remains a transfer test because its hull class differs materially from merchant vessels.


## 2026 commercial engineering and simulator comparators

Public vendor material confirms that the incumbent market already has substantial capability. These are not weak baselines to caricature:

| Provider / platform | Publicly stated scope | What Symtropy should compare or complement |
|---|---|---|
| MARIN + Deltares SHIPMA | Fast-time manoeuvring simulation for ports and fairways; autopilot with track-keeping and harbour-manoeuvring modes; turning, reverse sailing and berthing; tug/thruster allocation; design evaluation and possible later real-time simulator study | Matched repeatable channel/harbour manoeuvre cases, vessel-specific physics, tug interaction, channel/terminal design and engineering-study workflow |
| MARIN aNySIM XMF | Time-domain nonlinear hydrodynamic/mechanical response for stationary offshore vessels and sailing ships; manoeuvring, sailing, mooring, operability and harsh-weather safety; verification against analytical solutions and validation against MARIN model tests, full-scale trials and other tools | Multi-physics reference arm and validation evidence; Symtropy should not pretend to replace this solver |
| Kongsberg K-Sim Navigation | Real-time bridge/ship-handling simulation with hydrodynamic modelling, environmental/object interaction, instructor/debrief functions and DNV/STCW training configurations | Scenario and bridge-team integration, real-time behaviour and human-in-the-loop trials; not a direct metric-oracle substitute |
| Wärtsilä NTPRO 7 | Current navigation/ship-handling training platform launched 21 Apr 2026; integrated bridge simulation, training and wider R&D/digital-simulation products | Human/operational workflows and real-time simulation integration; compare only with access to exact vessel models, scenarios and recorded outputs |
| FORCE Technology vessel manoeuvring evaluation / SimFlex ShipYard | Captive PMM tests, selected free-sailing manoeuvres and early-design prediction results; standard turning, zig-zag and crash-stop outputs can inform simulator preparation | Reference-data and model-preparation chain; a suitable partner/reference provider rather than an assumed open benchmark |

Vendor sources:
- MARIN SHIPMA: https://www.marin.nl/en/about/facilities-and-tools/software/shipma
- MARIN aNySIM XMF: https://www.marin.nl/en/about/facilities-and-tools/software/anysim
- Kongsberg K-Sim Navigation: https://www.kongsbergmaritime.com/products/simulation/k-sim-navigation/
- Wärtsilä NTPRO 7 launch (21 Apr 2026): https://www.wartsila.com/ind/media/news/21-04-2026-wartsila-launches-next-generation-simulation-platform-to-support-future-ready-maritime-training-3743424
- FORCE Technology manoeuvring evaluation: https://forcetechnology.com/en/expertise/development-design-and-test/maritime-and-offshore-engineering/vessel-manoeuvring-evaluation

**Important comparison boundary:** public product pages describe capabilities and marketed use cases, not independently comparable error distributions, exact vessel-model particulars, solver versions, calibration artefacts or benchmark outputs. A genuine head-to-head requires access to matched models, input decks, conditions, run settings and raw output. It is not defensible to infer that a vendor lacks a feature because it is not described publicly.

### Our credible differentiation hypothesis

Do not compete by claiming to have a better hydrodynamics engine than MARIN, Kongsberg, FORCE or Wärtsilä. Use them as specialist model/experiment/in-the-loop comparators.

The potential Luminous advantage is at the **cross-system evidence and decision layer**:
- preserve the exact chain from source data, rights, vessel configuration, model/calibration, run settings and raw solver output through normalization and independent re-computation;
- distinguish experimental uncertainty, model-form error, numerical error and operational forecast uncertainty;
- route each scenario to a model family shown adequate for that vessel/regime, with explicit abstention outside its demonstrated domain;
- connect qualified physical envelopes to pilot/tug/channel/berth/port-call and onward freight decisions;
- verify the optimizer independently and reconcile predicted versus realized outcomes;
- produce machine-readable reasons, lineage and non-authority-limited decision receipts.

This must be measured by reduced evidence-preparation/review effort, reproducibility, cross-tool interoperability, decision-regret reduction, service reliability and cost per qualified decision—not claimed from architecture diagrams.
