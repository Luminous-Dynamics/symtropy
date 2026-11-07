# Maritime service-chain fleet: industry and research baselines

Research snapshot: 7 November 2026. This is a benchmark-design note, not a claim that the fleet oracle outperforms any cited system or paper.

## What the field already does well

### 1. Lock scheduling research

- Liao et al., *Scheduling rules of a waterway lock: fairness and efficiency*, *Ocean Engineering*, 1 May 2026. The paper compares a fairness-based rule (FSR) and an efficiency-based rule (ESR). Its reported measures include average waiting time, average chamber-area load factor and an inversion rate for fairness. It reports a Huai'an/Grand Canal one-way arrival-rate threshold of 7.6 vessels/hour for that case. **Do not transplant that threshold** to another lock, corridor, vessel mix or waterway. Reproduce the precise rule definitions, case data and tuning procedure first. DOI: https://doi.org/10.1016/j.oceaneng.2026.125004.

- Xu et al., *Intelligent Multi-Objective Optimization on Ship Lock Scheduling Considering Energy Consumption and Resource Constraints*, *Systems*, 3 May 2026. The published comparison evaluates NSGA-II, NSGA-III, MOEA/D and SPEA-II on a historical ship-lock hub instance, using average waiting time, lock utilization, total energy consumption, arrival rescheduling rate and berth-overcapacity penalty. NSGA-III is reported as balanced on the preserved instance; this does not establish universal superiority. Reproduce all four algorithms on matched decision variables, constraints, seeds and tuning budgets. https://www.mdpi.com/2079-8954/14/5/507.

- Segovia et al., *Multi-vessel placement in multi-chamber inland waterway transport locks using switching max-plus algebra*, *Ocean Engineering*, 15 January 2026. The model jointly handles route/waypoint timing, vessel order, chamber assignment and vessel placement, and translates switching max-plus constraints into MILP. This is a required future comparator for multi-chamber cases; it is **outside the current fleet oracle's model**, which has fixed route segments and no geometric chamber packing. DOI: https://doi.org/10.1016/j.oceaneng.2025.123414. https://research.tudelft.nl/en/publications/multi-vessel-placement-in-multi-chamber-inland-waterway-transport/.

- Xu et al., *Integrated optimization of vessel traffic organization and heterogeneous tugboat scheduling in a seaport with dual one-way channels*, *Frontiers in Marine Science*, 9 September 2026. Its bi-objective MILP minimizes total weighted vessel waiting and tugboat fuel; the NSGA-II-AAMNS solver coordinates channel traffic, tugboat capabilities/horsepower, task sequences and tug repositioning. The reported real-port case uses 18 vessels and the test groups span 10–40 vessels. This is a direct comparator for mobile assistance resources, but is not matched to the current fixed-capacity fleet oracle. DOI: https://doi.org/10.3389/fmars.2026.1939678. https://www.frontiersin.org/journals/marine-science/articles/10.3389/fmars.2026.1939678/full.

- Duan et al., *Joint optimization of vessel traffic scheduling and berth allocation in a multi-basin and multi-anchorage port with one-way channel*, *Ocean Engineering*, 30 October 2026. Its MILP jointly handles vessel inbound/outbound sequencing and discrete berth assignment with berth compatibility, channel conflicts, navigational continuity, safety intervals and tidal windows; OHE-IGA is compared with CPLEX, GA, PSO and FCFS. The article reports a Jingtang Port case and a paper-specific 30.98% objective reduction against FCFS. That number is not portable to our case and must not be claimed as our gain. DOI: https://doi.org/10.1016/j.oceaneng.2026.128093.

### 2. Operational port-call systems and standards

- DCSA Port Call Standard 2.0.0 provides a shared data/API process for port-call timestamps and service coordination. Its estimated/requested/planned/actual (ERPA) pattern distinguishes a carrier estimate, a service request, an agreed plan and observed actual event. The fleet benchmark must preserve these meanings and information cutoffs; an ETA or a proposed slot is not an agreed berth plan. https://dcsa.org/standards/port-call and https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope.

- The IAPH/IHMA Port Call Optimization Guide, published March 2026 and launched at IMO FAL 50, standardizes a minimum high-value exchange of nautical and operational port-call data. It is supported by 40 maritime organizations and is intended to reduce time waiting for or at berth. Our role is to consume this shared vocabulary and source-quality data, not invent a competing port-call event model. https://www.iaphworldports.org/pickup/21900/ and https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/.

- On 30 October 2026, StormGeo (part of Alfa Laval) completed its acquisition of Portchain. The announced product direction connects voyage intelligence with live berth plans and terminal crane/workforce planning. StormGeo/Portchain reports a network of more than 200 terminals representing over 40% of global container-terminal capacity. Treat these network figures as company-reported coverage, not independent performance evidence. This makes **JIT arrival alone an inadequate differentiation claim**. https://stormgeo.com/insights/stormgeo-acquires-portchain-to-advance-just-in-time-voyage-optimization-at-scale.

- The World Bank/S&P Global CPPI 2025 compares container-port performance using vessel time in port, including waiting and berth time, based on vessel movement records. It is a useful externally recognizable diagnostic, but it is not a door-to-door inland freight, lock-water, participant fairness or corridor resilience measure. We should report comparable vessel/port time components alongside broader chain metrics rather than relabeling our own composite as CPPI. https://www.worldbank.org/en/topic/transport/publication/cppi.

- IHO's Standards in Force page (last updated 29 October 2026) lists operational S-100 products including S-102 bathymetry, S-104 water levels, S-111 currents, S-122 protected areas, S-127 marine traffic management and S-131 harbour infrastructure. Consume their edition, update, provenance, coverage, datum, validity, uncertainty and terms explicitly. A specification being in force does not establish that a local dataset exists, is current, licensed or qualified for navigation. https://iho.int/fr/standards-in-force-0.

### 3. Additional 2026 research and deployed product baselines

- Sun et al., *Vessel scheduling and multi-task assignment of tugboats in seaports: A joint optimization approach using ADP-MLNS*, *Ocean Engineering* (2026), DOI 10.1016/j.oceaneng.2026.127433. This is a particularly relevant mobile-resource comparator because it includes pilot transfer as a tug task alongside escort and berthing/unberthing, minimizing vessel time in port and tug fuel. Its real-port case reports 22.73% lower vessel time in port and 12.82% lower tug fuel against that port's traditional rules. These are site-specific reported effects, not a general benchmark threshold. A fair reproduction needs its task continuity, vessel/tug qualifications, pilot transfer locations, operating data and fuel calculation. https://www.sciencedirect.com/science/article/pii/S0029801826032671.

- *Joint Optimization of Multi-Port Berth Allocation and Ship Lock Scheduling in River–Sea Direct Transport*, *Journal of Marine Science and Engineering* (published 9 October 2026; DOI 10.3390/jmse14201871). Its HICOA framework iterates between port berth and lock decisions, with vessel placement in lock chambers. The authors report up to 55.69% lower total port time than independent scheduling in their Pinglu Canal-related case, and small-instance optimum recovery under their model. This adds a direct comparator for **multi-port–lock coordination**, not just one fleet queue. Keep its result scoped to its case/model, and reproduce the independent-scheduling comparator before any comparison. https://www.mdpi.com/2077-1312/14/20/1871.

- KONGSBERG Foresight PMIS is a direct port-authority product comparator: its public product description includes shared situational awareness, berth/tug/pilot utilization KPIs, JIT arrivals, traffic display integration, port visits, resource services and incident management. https://www.kongsberg.com/what-we-do/ocean-space/surveillance-monitoring/foresight/pmis/.

- PortXchange Synchronizer is a direct commercial JIT/berth-alignment comparator: the vendor describes real-time terminal berth-plan updates and ETA-Berth-based speed guidance to reach the Pilot Boarding Place at the right time; its public feature set includes berth visit details, JIT reports, planning-deviation alerts, APIs and data authorization. https://port-xchange.com/synchronizer/.

- Awake.AI Port Vision is another commercial comparator for port-call planning, JIT, maps/geofences, weather alerts, event handling, berth/resource scheduling and situational awareness. Its public claims (port coverage and voyage-prediction volume) are vendor-reported scale indicators, not independently measured accuracy or operational impact. https://www.awake.ai/.

These product descriptions establish existing feature coverage, not head-to-head performance. They make the scope boundary clear: do not present a berth dashboard, ETA prediction, JIT speed recommendation, berth/tug/pilot KPI or event-sharing layer as novel on its own. The research opportunity is to make upstream physical/model limits and evidence quality explicit, bind assignments to the exact selected schedule, independently verify resource capability and movement feasibility, and measure ex-post delivered corridor service and recovery.

## 2026 Durban operational snapshot — contextual data, not a reproduced trial

The following dated observations can anchor an intake plan, but they are not a rights-reviewed event-level dataset or a continuous panel:

- Kuehne+Nagel's myKN operational update for **27 October–3 November 2026** reports a **7-day average Durban vessel waiting time of 7.54 days** and describes congestion, vessel bunching, high yard use, peak imports, road congestion and equipment failures. Its listed data sources are seaexplorer, Maersk, Hapag-Lloyd, CMA CGM, MSC and SACO. Preserve this as a logistics-provider weekly estimate with its exact issue/reporting window and population definition. https://mykn.kuehne-nagel.com/news/article/port-operational-updates-from-03-11-2026.

- Transnet's 12 May 2026 media statement reports **8,630 vessel arrivals across TNPA's eight commercial ports in FY2025/26**, up from 7,912, and roughly **304 million tonnes** of cargo throughput, up 4.2%. It separately reports container volumes +7.1%, with Durban automotive volumes exceeding their throughput targets. These are annual network/segment indicators, not Durban container-ship waiting-time observations. https://www.transnet.net/getFile.ashx?id=15673.

- World Bank/S&P Global CPPI 2025 remains the globally recognized container-port time-in-port comparison, combining waiting and berth time. Its 10 June 2026 methodological note should be pinned when reproducing the index. It is not a corridor-wide door-to-door metric. https://www.worldbank.org/en/topic/transport/publication/cppi and https://thedocs.worldbank.org/en/doc/855fe5e7123d85b05d4e84c95b8fd1e2-0400012026/cppi-methodological-note.

Do not compare the 7-day waiting estimate, a port-wide vessel count, annual all-port arrivals, terminal weekly TEUs, or a terminal anchorage-hours statistic as if they measured the same cohort. Each observation needs source class, issue time, observation window, port/terminal boundary, vessel class, numerator/denominator, wait definition, censoring policy, revision and rights.

## Matched benchmark matrix

| Baseline | What it tests | Status in our programme |
|---|---|---|
| No-build / observed operating policy | Realistic service outcome rather than a synthetic optimizer-only comparison | Required when rights-qualified operating records exist; not yet reproduced |
| FCFS / oldest arrival first | Simple operational control | Existing lock scheduling comparator; freeze exact tie-breakers |
| FSR / ESR | Fairness versus lock throughput and chamber loading | Required literature reproduction; not yet claimed reproduced |
| Earliest feasible / least energy / minimum wait-slack | Transparent greedy rules | Required matched heuristic baselines |
| NSGA-II / NSGA-III / MOEA/D / SPEA-II | Multi-objective search quality | Required paper-aligned comparison on identical frozen cases |
| Switching max-plus / MILP | Serial multi-lock and multi-chamber placement | Required when vessel packing/chamber geometry enters scope; not represented by current model |
| MILP + NSGA-II-AAMNS | One-way channel traffic plus heterogeneous tug assignment and repositioning | Owned by MARITIME-MOBILE-OPS-001 #3798; current fleet crate has no named tug or reposition route |
| MILP + OHE-IGA | Multi-basin, multi-anchorage traffic sequencing plus discrete berth allocation | Must be reproduced for spatial port traffic cases; current fleet crate has no basin conflict graph or berth compatibility geometry |
| CP-SAT or MILP reference | Independent larger-instance formulation and optimality gap | Next external solver differential; not yet implemented/reproduced here |
| DCSA/IAPH/IHMA-aligned coordination | Operational data exchange and timestamp semantics | Consume standards; compare against real workflow only with authorized data |
| StormGeo/Portchain class of products | Voyage-speed decisions linked to berth/terminal readiness | Industry capability reference; no access to matched vendor data or independent head-to-head performance results |

Do not call an unavailable baseline a loss or replace it with a weaker straw man. Mark it NotReproduced and disclose the exact reason.

## What the current fleet oracle does—and does not—establish

Current scope is a finite, integer-tick synthetic service chain with at least two demands, shared physical resource IDs and a bounded set of prequalified voyage alternatives. The current exact head declares 32 unit tests and 3 independent resource-accounting integration tests, includes a test-only full-fleet enumerator and a 20-variant differential sweep. Exact-head formatting and test workflow passed on 7 November 2026 after the added terminal/inland throughput-quota controls were included. Workflow: https://github.com/Luminous-Dynamics/symtropy/actions/runs/42341759255. These tests are correctness evidence for the finite model only, not physical qualification or evidence of optimizer superiority.

Covered model features:
- Joint feasibility across demands rather than independent one-vessel feasibility.
- Concurrent occupancy for channel transit slots, lockages and berth slots.
- Cumulative window quotas for water, shore-power import, terminal handling and inland-transfer throughput; no silent quota/slot conflation.
- Exact equality of shared resource declarations and their evidence snapshots.
- Separate propulsion, auxiliary, shore-import and inland-transfer energy boundaries; no unproven sum.
- Independent reference accounting, a second full-fleet enumerator for frozen cases, deterministic case ordering and explicit search-budget exhaustion.

Not yet covered:
- Partial-flow/service allocation, unserved-demand optimization and participant fairness policies (coordinate with TRANSPORT-ALLOC-002 #1639; do not duplicate its owner).
- Anchorage wait before the first operation, berth dwell, terminal queue delay, cargo dwell, end-to-end delivery-time decomposition, and full participant-level p50/p95/max-wait metrics.
- Stochastic arrival/duration distributions, correlated disruption, observation-triggered recovery, or calibrated probabilities (coordinate with MARITIME-REC-001 #1867 and historical replay #1671).
- Multi-chamber geometric packing, vessel dimensions/position, dynamic water-level hydraulics, current, under-keel clearance calculation, sediment and physical lock/port models (consume authoritative owner outputs; do not infer from this scheduler).
- An externally reproduced CP-SAT/MILP baseline, a real operational dataset, or an independent performance result against a commercial platform.

## Paradigm shift: optimize evidence-qualified delivery, not local asset utilization

The differentiation opportunity is not another ETA predictor or berth dashboard. It is a **composable, evidence-qualified service-chain decision certificate**:

1. Vessel, navigation, lock, port, energy and inland-transport owners publish bounded alternatives with units, validity windows, source revisions, uncertainty and claim ceilings.
2. Symtropy composes only alternatives whose hard constraints and shared resource calendars are mutually compatible.
3. The result reports a Pareto frontier and auditable witnesses: exact input manifest, per-demand selection, per-operation resource ledger, evidence references, excluded/rejected alternatives, deterministic digest and proof/search-completeness status.
4. A disruption does not silently rewrite a published plan. The bounded recovery owner evaluates alternatives against new admissible information; Mycelix remains the authority for participant consent and commitments.
5. Sol Atlas projects the source coverage and unresolved gaps; it does not turn a map layer into navigation authority.

This makes evidence quality, resource semantics, participant outcome and recovery part of the optimization boundary. It is only a credible improvement if matched benchmarks show gains without hard-gate violations or distributional harm.

## Fair-comparison protocol: what "better than SOTA" must mean

A headline percentage from a paper, a local synthetic gain, and a commercial product feature are three different evidence classes. Do not compare them as if they were one leaderboard. Before a head-to-head claim, pin the instance, model boundary, information cutoff, candidate alternatives, hard constraints, objective definitions, hardware and compute budget.

### Comparator tiers

| Tier | Baselines | What a valid comparison establishes |
|---|---|---|
| Correctness floor | Independent exact fleet enumerator; direct per-tick and per-window resource oracles | The implementation solves the finite integer-tick model correctly on matched tiny instances |
| Transparent operational rules | FCFS, earliest feasible, minimum wait-slack, minimum declared energy, current declared port/lock rule | Whether optimization improves over transparent, reproducible decision policies |
| Mathematical optimization | HiGHS MILP, published CPLEX/MILP formulations, max-plus-to-MILP where in scope | Objective/frontier quality and runtime under matched decision variables, constraints and stopping budgets |
| Multi-objective search | NSGA-II, NSGA-III, MOEA/D, SPEA-II, MOSA, and NSGA-II-AAMNS where the published formulation can be reproduced | Pareto quality under equal tuning/evaluation budgets and a fixed reference set |
| Robust/contingent optimization | Two-stage robust/column-and-constraint generation; bounded trigger-and-recourse policy; fixed plan and greedy event-by-event replanning | Regret and tail-service performance under the same frozen uncertainty set and information cutoffs |
| Operational systems | DCSA/IAPH/IHMA-aligned port-call workflows; commercial voyage/berth coordination platforms such as StormGeo/Portchain | Interoperability and workflow comparison; commercial feature presence is not an independently measured optimization win |

### Matched experiment controls

Every method must receive the same:
1. demand roster, vessel/alternative catalogue, route and segment revisions, resource calendars, units and evidence snapshots;
2. information cutoff and observation state (planned/requested/estimated/actual are not interchangeable);
3. hard constraint set, including direction windows, headway, interaction topology, resource capacity and cumulative quotas when those variables are in scope;
4. objective vector and tie-breaks, without silently changing the denominator or excluding unserved demands;
5. time/memory/CPU budget, number of seeds/runs, solver tolerances and termination criteria.

Pin the solver/library/compiler versions and machine profile. Report model-build time separately from solve time. An optimal MIP status is valid only for that model formulation; it is not proof the formulation captures a real port or its uncertainty.

For published reproduction, preserve the authors' data, variable definitions, bounds, normalizations, reference point, run count and hardware as closely as source access permits. Then separately run a preregistered common-corpus experiment on a frozen instance set. Paper-specific normalizations and thresholds must not be repurposed to make our scores look better.

### Required metrics (do not collapse)

| Dimension | Report |
|---|---|
| Service | Requested, accepted, delivered and unserved amount by demand/participant/service class; explicit reconciliation |
| Time | Arrival, anchorage, channel, lock, berth, handling, transfer and end-to-end dwell components; mean, median, p90, p95, max and CVaR where the corpus supports it |
| Traffic | Same-segment headway violations, missing/unknown topology, crossing/merge conflicts, missed direction/tidal windows |
| Mobile assistance | Tug/pilot compatibility, assignments, deadhead/reposition distance and time, idle time, duty/availability, fuel/energy only where a qualified model supplies it |
| Resources | Concurrent occupancy, cumulative quota use, spare/reserve use and overcommitment, all by canonical resource ID |
| Energy/water | Propulsion, auxiliary, shore-grid import and inland energy separated; lock water draw/return/reuse/loss with declared meter boundaries |
| Resilience | Service lost, time back above a declared service threshold, recovery regret, branch feasibility, reserve option cost and evidence-triggered abstention |
| Distribution | Delay and unserved amount by participant/service class, worst-group outcome, displacement caused by prioritization, allocation-policy sensitivity |
| Search | Feasibility, optimality proof, incumbent/dual bound/gap, frontier size, hypervolume/IGD+ under frozen normalization, state/node counts, memory, p50/p95 time and termination reason |

Hard-constraint violations are reported as counts and failure dispositions, never as soft penalties that can be offset by a better weighted score. If an algorithm returns a plan that fails an independent verifier, it receives no feasible-plan credit.

### Pareto and uncertainty rules

- Keep water, propulsion, auxiliary, shore import, inland-transfer energy, makespan, delivery, idle wait and service/fairness as separate dimensions until a justified boundary or policy explicitly combines them.
- Freeze normalization bounds and hypervolume reference points before opening the held-out results. Report Pareto precision/recall against exact tiny fronts, epsilon indicators and IGD+; do not compare hypervolume computed with different reference points.
- When probabilities are not calibrated, use per-scenario/worst-case or interval/range reporting. A set of hand-authored disruptions does not establish an event distribution.
- For uncertain berth service, arrivals, tug availability or water/tide conditions, compare deterministic, robust and bounded recourse policies on the identical frozen uncertainty scenarios. State the non-anticipativity / information available at each decision node.
- Keep hindsight/oracle solutions strictly evaluation-only. They are upper bounds, not deployable policies.

### Promotion criteria for a superiority statement

A "better than SOTA" claim requires all of the following, not just a better average objective:
1. Reproduce at least one strongest, scope-matched published baseline or document a specific data/licensing blocker.
2. Achieve 100% independent feasibility agreement on the exact tiny corpus and zero hard-constraint violations in every reported incumbent.
3. Use a predeclared held-out case/corridor/operating-regime split, with no overlapping campaign or scenario leakage.
4. Demonstrate a statistically and operationally meaningful improvement on a predeclared service or efficiency metric, with uncertainty intervals and no unacceptable degradation in tail delay, service coverage, fairness, water, energy or resilience.
5. Disclose all negative cases, infeasible cases, timeouts, unsupported evidence, solver gaps and uncompleted runs.
6. Preserve deterministic replay and exact commit/data/configuration identities for every result.

The current fleet oracle is a correctness reference, not an industry-leading optimizer. Its role is to make later matched comparisons falsifiable and independently checkable.

## Qualification targets

All values below are proposed promotion targets, not achieved results or universal industry standards:

- 100% agreement between production and independent exact fleet enumerators on every frozen tiny workload, including frontier, resource ledger, selected alternatives and objective vector.
- 0 concurrent-capacity violations, 0 cumulative quota violations and 0 accepted conflicting/stale/revoked resource snapshots.
- 100/100 byte-identical deterministic replay under input permutation for every deterministic frozen case.
- Every demand reconciles as requested = served + unserved once partial service is added; no denominator disappears.
- Report mean, median, p90, p95 and max waiting and delivery delay per comparable participant/service unit; preserve FSR/ESR inversion-rate and chamber-load-factor metrics where relevant.
- Pareto quality: precision/recall against exact reference fronts, epsilon indicator and IGD+; hypervolume only with a reference point frozen before evaluation.
- For larger cases, report p50/p95 runtime, peak memory, solver version, optimality gap, timeout/infeasibility rates, candidates evaluated and truncation. Never describe an interrupted search as optimal or infeasible.
- At least one held-out demand/arrival regime and one held-out corridor family before claiming generalization. Do not split adjacent samples from one voyage/corridor randomly across train/test.
- A performance claim requires paired identical information cutoffs and data, reproduced baselines, a predeclared metric, effect size and uncertainty interval; no one-number weighted score that hides displaced demand, water use or tail delay.

## Immediate research sequence

1. Finish exact-head fleet correctness and independent differential corpus; keep all workflows fail-closed.
2. Extend the model to report complete service-time decomposition, per-participant wait and unserved denominators, in coordination with #1639.
3. Compose bounded multi-demand recourse and disruption branches under #1867, including one reserve contested by two demands, stale/conflicting evidence and displacement accounting.
4. Add a matched solver-backed MILP/CP-SAT reference for larger cases and a switching max-plus/MILP subset when multi-chamber packing enters scope.
5. Only then use source-permitted port-call, lockage, terminal and water/energy records for a held-out operational replay. Preserve DCSA/IAPH/IHMA and IHO semantics; no synthetic-to-real extrapolation.

## Durban Gateway Terminal: direct operational snapshot intake (7 November 2026)

The official Durban Gateway Terminal (DGT) [Terminal Updates page](https://dgt.ictsi.com/terminalupdates) publishes several distinct source families. This is a stronger intake lead than a secondary weekly congestion summary, but **the records have not been admitted as a reusable dataset**: the page's privacy policy is not a data licence, and source reuse/redistribution terms and field definitions remain to be reviewed.

### Two direct source snapshots

| Source | Observed snapshot/report time | Contents | Correct benchmark role | Critical limit |
|---|---|---|---|---|
| [DGT Vessel Update @05H00](https://cdnweb-dgt.ictsi.com/users/user20/DGT%20VESSELS%20UPDATE%20SPREADSHEET%20%2823%29.pdf) | 7 Nov 2026, 05:00 (source-local timezone not explicitly verified) | Vessel name, vessel visit no., berth, ETA date, start-work date/time, status, discharge/load balances, completion date/time, notes | Source-native terminal operation snapshot / milestone status | Visit number is not IMO; ETA is not known to be PCO PTA-PBP; start-work is not berth arrival; completion is not departure; balance units and denominator require confirmation |
| [DGT Firm 21-day Berth Plan](https://cdnweb-dgt.ictsi.com/users/user13/DGT%20Firm%2021%20day%20berth%20plan%2006.11.2026.pdf) | 6 Nov 2026, 13:12:09, labelled Live Update:All | Vessel/visit references, berth placement, ETA/ETD fields, LOA, side-to / berth presentation and graphical berth/time occupancy | Versioned planning snapshot for forecast and plan-churn analysis | ETA/ETD are schedule fields until their precise semantics are confirmed; the graphical block is not observed berth occupancy, a swept path or physical clearance |
| [DGT Firm 14-day Berth Plan](https://cdnweb-dgt.ictsi.com/users/user36/14%20DAY%20DGT%20BERTH%20PLAN%2001-11-2026.pdf) | 1 Nov 2026, 12:53:16, labelled Live Update:All | Same broad planning family | Earlier immutable snapshot for plan revision comparison | Different horizon/revision; match visits only through reviewed stable keys |

The [DGT 4 November operations notice](https://cdnweb-dgt.ictsi.com/users/user13/2026.11.03%20DGT%204th%20November%20Operations.pdf) is a separate *planned disturbance* source: it described possible reduced staffing/landside appointments for the 4 November voting holiday and a planned 06:00–08:00 software suspension, while prioritising waterside continuity. It is useful for a planned-disturbance scenario, not proof that the expected capacity loss occurred.

The landing page also lists daily export-stack and import-storage snapshots. These are yard-state data and must not be joined to vessel events without their own unit, sampling boundary, storage rule and linkage semantics.

### Snapshot metadata and independent congestion statistics must not be conflated

A current inspection reveals a source-integrity negative control. On **7 November 2026**, the DGT Terminal Updates page displayed the vessel document as **DGT VESSEL UPDATE @17H00**, while the linked PDF's own embedded header says **07/11/2026 - 05H00**. The landing-page label and document-internal as-of time therefore disagree in the currently retrieved pair:
- Landing page: https://dgt.ictsi.com/terminalupdates
- Linked document: https://cdnweb-dgt.ictsi.com/users/user20/DGT%20VESSELS%20UPDATE%20SPREADSHEET%20%2823%29.pdf

Do not promote the landing-page label to the document's event/snapshot time. Preserve both, capture the retrieval time and exact bytes/digest, flag the mismatch, and hold the record at NeedsEvidence pending confirmation or a new exact document revision. The current 05H00 embedded timestamp is the only time asserted inside that PDF; the 17H00 text is only the page's current display label.

There is also a measurement-definition warning in secondary congestion dashboards:
- Portcast's public Durban page reports **4.27 days median vessel waiting** for 25–31 Oct 2026, last updated 1 Nov. It defines waiting as time at anchorage or drifting before securing a berth and describes P90 as the slowest 10% tail, but its public page does not disclose the detailed vessel inclusion/exclusion cohort for this sample: https://www.portcast.io/port-congestion/durban.
- Kuehne+Nagel reports **7.54 days 7-day average** for 27 Oct–3 Nov 2026. Its published method says the rolling mean is limited to mainline container vessels and excludes feeder vessels, idle ships without an announced next port, slow steamers not yet actively waiting at the designated anchorage, and vessels passing through without a port call: https://mykn.kuehne-nagel.com/news/article/port-operational-updates-from-03-11-2026 and its method description https://mykn.kuehne-nagel.com/news/article/port-operational-updates-from-12-08-2026.
- Kuehne+Nagel's later 2 Nov snapshot reports 7.46 days average waiting at anchorage, 6.02 days average arrival delay and 5.59 days average departure delay for container vessels over different windows: https://mykn.kuehne-nagel.com/news/article/durban-congestion-remains-severe-as-vessel.

These values are **not directly comparable**: median vs mean, different reporting windows, cohorts and possibly different anchorage/drifting definitions. The difference is not evidence that either source is wrong. The benchmark should store provider, cohort, port/terminal geography, time window, statistic, censoring, inclusion/exclusion rules, update time and units alongside every metric. The only fair cross-provider comparison is on a matched vessel/call cohort with aligned event definitions and observation cutoffs.

### Field mapping discipline

Reuse [PCO evidence #1930 / PR #1940](https://github.com/Luminous-Dynamics/symtropy/pull/1940), [DCSA JIT #1955 / PR #1965](https://github.com/Luminous-Dynamics/symtropy/pull/1965) and the provider-specific intake task [MARITIME-DGT-OBS-001 #4003](https://github.com/Luminous-Dynamics/symtropy/issues/4003). Do not add a competing global port-call vocabulary.

| DGT source field | Initial treatment | Forbidden silent mapping |
|---|---|---|
| Vessel Visit No. | Source-local visit reference; exact raw lexeme | Not IMO and not a globally unique port-call ID without a reviewed identity map |
| Vessel Name | Display/source attribute | Not a stable vessel identity |
| Berth | Raw terminal berth code/name; explicit versioned registry mapping required | Not a global berth ID by itself |
| Vessel E.T.A Date | Preserve labelled ETA and raw date | Not automatically ETA-Berth, PTA-PBP, actual arrival or anchorage arrival |
| Date/Time Started Working | Provider-specific terminal-work milestone | Not PBP arrival, berth arrival, pilot boarding or vessel departure |
| Status (NYI/INW/OWI/WORKING/COMPLETED/SAILED etc.) | Exact raw status plus source dictionary/version | No guessed DCSA classifier or lifecycle transition from string similarity |
| Balance to Discharge / Balance to Load | Preserve ordered raw values and lexemes | No guessed unit, remaining/total orientation, TEU conversion or productivity numerator |
| Date/Time Complete | Terminal completion milestone | Not actual departure from berth; SAILED is a separate source assertion |
| Firm-plan ETA/ETD | Snapshot-scoped plan assertions | Not actual arrival/departure timestamps |
| Firm-plan berth/time block | Versioned schedule placement | Not a navigability envelope, conflict-free route or actual occupancy observation |

### Minimum frozen source receipt

Before ingesting even a limited retrospective sample, freeze: publisher/terminal, exact URL/filename, retrieval time, reported as-of or plan publication time, file bytes/length/SHA-256, page count, source revision, source field dictionary version, clock/timezone/precision, rights/permission receipt, coverage window, extraction code/version and known omissions. Keep event/valid time, schedule time, source publication/snapshot time, ingestion time and decision cutoff separate.

A changed daily snapshot is a new immutable assertion. Compare snapshots only after stable call/visit identity and source revision are reconciled. Late corrections and conflicting sources remain visible; never use last-ingested as truth.

### Evaluation opportunity (not yet measured)

Once rights and definitions are cleared, the same vessel-call cohort can support:
- terminal work-start → terminal work-completion duration (not port time-in-port);
- ETA/berth-plan revision error, schedule churn and missed planned windows;
- terminal queue and service distributions only when independent arrival/anchorage and departure observations define the true interval;
- linkage from vessel calls to yard/gate/rail evacuation only when explicit cargo/visit keys exist;
- matched comparison of FCFS/operator rules, published berth/port optimizers and the joint fleet + tug/pilot + channel-conflict + service-chain policy.

Report p50/p90/p95/max, censoring, vessel class, call population, plan snapshot vintage and terminal-vs-port boundary. Do not compare DGT terminal work duration to CPPI port time-in-port or to a logistics provider's average anchorage wait as if they were the same measure.

## Primary sources

- Liao et al., 2026 lock fairness/efficiency paper: https://doi.org/10.1016/j.oceaneng.2026.125004
- Xu et al., 2026 multi-objective lock scheduling: https://www.mdpi.com/2079-8954/14/5/507
- Segovia et al., 2026 multi-chamber SMPL/MILP: https://doi.org/10.1016/j.oceaneng.2025.123414
- Xu et al., 2026 dual one-way channel / heterogeneous tug scheduling: https://doi.org/10.3389/fmars.2026.1939678
- Duan et al., 2026 multi-basin / berth co-scheduling: https://doi.org/10.1016/j.oceaneng.2026.128093
- DCSA Port Call Standard: https://dcsa.org/standards/port-call
- IMO/IAPH/IHMA Port Call Optimization Guide: https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/
- StormGeo/Portchain announcement, 30 October 2026: https://stormgeo.com/insights/stormgeo-acquires-portchain-to-advance-just-in-time-voyage-optimization-at-scale
- World Bank CPPI 2025: https://www.worldbank.org/en/topic/transport/publication/cppi
- IHO Standards in Force, updated 29 October 2026: https://iho.int/fr/standards-in-force-0

## Expanded 2026 integrated-operations benchmark: matched parity before superiority

Research snapshot updated: 7 November 2026. The competitive frontier is no longer just a tug assignment or a berth calendar. Current papers cover multi-channel traffic conflicts, heterogeneous tug service chains, pilot transfers, multi-basin geometry, uncertain arrivals, and simulation-in-the-loop resource allocation. Symtropy should compare these capabilities separately and then in one matched corridor case.

| Comparator | Scope that must be represented | Reported evidence | What a fair Symtropy comparison must do |
|---|---|---|---|
| Xu et al., Frontiers in Marine Science, 9 Sep 2026, DOI 10.3389/fmars.2026.1939678 | Dual one-way channels, 18-vessel port-day case (9 inbound / 9 outbound), heterogeneous tugs, waypoint repositioning, safety-time separation, tide window, berth assignment; bi-objective weighted vessel wait + tug fuel | Published study compares NSGA-II-AAMNS with CPLEX/NSGA-II/MOSA. Its 18-vessel case includes a 13-minute vessel separation and a tide-dependent Ship 14 window [540,850] minutes. Reported case data and algorithmic gains are study-specific. | Reproduce the exact ship/tug tables, waypoint/reposition matrix, base departures/returns, 13-minute separation, tide interval, objectives and FCFS comparator. Keep their original pooled Pareto normalization for a paper-parity replication; separately preregister a fixed reference point for cross-method comparisons. |
| Sun et al., Ocean Engineering (2026), DOI 10.1016/j.oceaneng.2026.127433 | Joint vessel scheduling with multi-task tug assignment including pilot transfer, escort, berthing and unberthing; tug service continuity | Real-port case reports 22.73% lower total vessel time-in-port and 12.82% lower tug fuel than that port's traditional scheduling rules. | Include pilot transfer as a timed task with a pickup/drop-off location and resource interval. Count the full tug cycle including base-to-first assignment, between-task reposition and final return. Do not transplant the percentage improvements into another port. |
| Duan et al., Ocean Engineering (30 Oct 2026), DOI 10.1016/j.oceaneng.2026.128093 | Multi-basin / multi-anchorage port, one-way channel, buoy and entrance conflicts, merge/diverge points, berth compatibility, navigation continuity, safety intervals and tidal windows | A distinct MILP formulation jointly schedules inbound/outbound vessels and discrete berths over the anchorage–channel–basin–berth process. | Extend the traffic-resource graph with independently reviewed conflict-zone and merge-zone IDs. Reproduce the published topology and constraints only after exact data/rights review; keep its different geography and demand set distinct from Xu's dual-channel test. |
| Wang et al., Computers & Industrial Engineering (July 2026), DOI 10.1016/j.cie.2026.112035 | Two-stage port scheduling under uncertain vessel arrivals/departures with rolling decision strategies | Real-data experiments study scheduling-cycle decisions and rolling rescheduling when vessel delays evolve. | Add a separate rolling-horizon replay arm using identical arrival observations/cutoffs. Measure update-to-replan latency, schedule churn, displaced work, missed windows and regret against hindsight. Avoid treating a point forecast as certainty. |
| Liu et al., Ocean Engineering (1 May 2026), DOI 10.1016/j.oceaneng.2026.124972 | Multi-source information fusion, MIP, cellular-automata + multi-agent digital twin and simulation-based multi-objective GA (SMOGA) | Tianjin Port case reports 16.5% greater throughput and 11.7% lower turnaround than its own comparison strategies. | Compare state assimilation lag, conflict detection, simulation feedback value, replanning latency and ex-post delivered service. The published percentages are case-specific and cannot be compared directly to another port or objective definition. |
| Liu et al., Autonomous Transportation Research (online 7 Sep 2026), DOI 10.1016/j.atres.2026.09.002 | Autonomous-tug collaborative berthing with a digital twin and reciprocal control-barrier safety constraints | The article presents a three-layer cyber-kinematic / optimization / kinetic architecture, stability analysis and numerical examples. It is not evidence of a field deployment or port-wide vessel scheduler. | Treat this as a separate future control-assurance comparator. Do not claim the current traffic verifier or fleet optimizer implements RCBFs, tug dynamics, closed-loop autonomy or collision avoidance. |

### Reproduction protocol for the 18-vessel dual-channel case

1. Freeze source semantics first. Record each vessel's inbound/outbound state, application/arrival/departure time meaning, priority weight, berth/quay/waypoint, channel-segment running times, berthing/unberthing duration, tug count and minimum horsepower. Record each tug's horsepower, base location, speed, task eligibility and fuel-rate assumptions. Pin the source table revision and extraction script.
2. Recreate the same feasibility problem. Preserve dual one-way direction rules, berth compatibility, navigation continuity, exact safety-time separation, tidal windows, tug horsepower compatibility, no-overlap and route reposition constraints. No missing field becomes a convenient default.
3. Match the algorithms and resource budget. Compare FCFS, published rule baselines, NSGA-II, MOSA, NSGA-II-AAMNS (if reproducible from its publication/artifacts), and the exact MILP/epsilon-constraint reference on enumerable small instances. Record CPU model, cores, RAM, solver/algorithm versions, stopping criteria, seeds and wall-clock budget.
4. Report both parity and portable metrics. For parity, reproduce the paper's objective and its original normalization/front definition. For cross-method comparisons, predeclare fixed common normalization/reference points before candidate runs. Report HV, IGD/IGD+, epsilon indicator, front coverage and uncertainty over independent runs. Never compare two papers' HV values that used different pooled reference fronts.
5. Keep a safety/feasibility ledger outside the fitness score. Report hard-constraint violations, assignment errors, channel/berth/tide-window misses and BudgetExhausted separately. No Pareto score can compensate for an infeasible operation.
6. Promote robustness only from evidence. Use held-out historical arrival/operation errors to fit regime-conditioned early-entry and late-clearance bounds. Compare nominal deterministic scheduling, deterministic interval envelopes, two-stage rolling replanning and stochastic/robust recourse under identical information cutoffs. Report empirical coverage and false-permissive conflicts per regime, not a generic "robust" label.

### System-level scorecard we should lead with

Do not optimize one aggregate that hides shifted waiting or resource burden. Report, per vessel class and participant/service:
- weighted and unweighted waiting, anchorage wait, time-in-port with explicit event boundaries, median/p90/p95/p99 and maximum wait;
- service-chain continuity: pilot transfer, escort, berthing, unberthing and any interrupted/missed task;
- tug assignments by ID/class, utilization, idle time, reposition time/distance and fuel at the declared meter boundary;
- direction-window adherence, merge/crossing/berth conflicts, under-keel/tide-window misses and headway violations;
- terminal/berth utilization, throughput actually delivered, cancellations, queue growth and plan churn;
- energy, water and emissions at disjoint, explicitly defined measurement boundaries;
- replan latency after an observation correction or disturbance, schedule instability and displaced/late work;
- runtime, memory, search states, optimality gap and exact-vs-budget-exhausted status.

### What PRs #3724, #3935 and #4032 currently establish

- PR #3724 has a bounded exact tiny-instance multi-demand fleet oracle. Exact-head run passes 32 unit tests, 3 direct accounting tests, formatting and Clippy. It is not the published MILP/CPLEX replication and not a large-scale optimizer.
- PR #3935 independently checks the named tug/pilot assignments supplied against selected fleet placements, including capability class, pull, qualifications, position, availability, reposition and subject-bound evidence. Its 32 unit tests plus a 512-case direct timing oracle pass on its current recorded exact head. It does not optimize the assignments.
- PR #4032 introduces the robust traffic envelope check over selected fleet operations. Its first run passed formatting and all 15 unit tests plus a 256-case independent interval oracle, but Clippy found a test fixture helper issue; the helper has since been refactored and a new exact-head run is required. This PR does not infer a real port conflict graph or certify navigability.

This architecture's potential differentiator is planner + independent, evidence-bound verifiers + post-operation outcome replay, not a claim that one heuristic is universally faster than NSGA-II-AAMNS, ADP-MLNS, SMOGA or commercial port systems. The comparison must be falsifiable on the published case and on held-out operating regimes.
