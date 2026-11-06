# Maritime service-chain fleet: industry and research baselines

Research snapshot: 6 November 2026. This is a benchmark-design note, not a claim that the fleet oracle outperforms any cited system or paper.

## What the field already does well

### 1. Lock scheduling research

- Liao et al., *Scheduling rules of a waterway lock: fairness and efficiency*, *Ocean Engineering*, 1 May 2026. The paper compares a fairness-based rule (FSR) and an efficiency-based rule (ESR). Its reported measures include average waiting time, average chamber-area load factor and an inversion rate for fairness. It reports a Huai'an/Grand Canal one-way arrival-rate threshold of 7.6 vessels/hour for that case. **Do not transplant that threshold** to another lock, corridor, vessel mix or waterway. Reproduce the precise rule definitions, case data and tuning procedure first. DOI: https://doi.org/10.1016/j.oceaneng.2026.125004.

- Xu et al., *Intelligent Multi-Objective Optimization on Ship Lock Scheduling Considering Energy Consumption and Resource Constraints*, *Systems*, 3 May 2026. The published comparison evaluates NSGA-II, NSGA-III, MOEA/D and SPEA-II on a historical ship-lock hub instance, using average waiting time, lock utilization, total energy consumption, arrival rescheduling rate and berth-overcapacity penalty. NSGA-III is reported as balanced on the preserved instance; this does not establish universal superiority. Reproduce all four algorithms on matched decision variables, constraints, seeds and tuning budgets. https://www.mdpi.com/2079-8954/14/5/507.

- Segovia et al., *Multi-vessel placement in multi-chamber inland waterway transport locks using switching max-plus algebra*, *Ocean Engineering*, 15 January 2026. The model jointly handles route/waypoint timing, vessel order, chamber assignment and vessel placement, and translates switching max-plus constraints into MILP. This is a required future comparator for multi-chamber cases; it is **outside the current fleet oracle's model**, which has fixed route segments and no geometric chamber packing. DOI: https://doi.org/10.1016/j.oceaneng.2025.123414. https://research.tudelft.nl/en/publications/multi-vessel-placement-in-multi-chamber-inland-waterway-transport/.

### 2. Operational port-call systems and standards

- DCSA Port Call Standard 2.0.0 provides a shared data/API process for port-call timestamps and service coordination. Its estimated/requested/planned/actual (ERPA) pattern distinguishes a carrier estimate, a service request, an agreed plan and observed actual event. The fleet benchmark must preserve these meanings and information cutoffs; an ETA or a proposed slot is not an agreed berth plan. https://dcsa.org/standards/port-call and https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope.

- The IAPH/IHMA Port Call Optimization Guide, published March 2026 and launched at IMO FAL 50, standardizes a minimum high-value exchange of nautical and operational port-call data. It is supported by 40 maritime organizations and is intended to reduce time waiting for or at berth. Our role is to consume this shared vocabulary and source-quality data, not invent a competing port-call event model. https://www.iaphworldports.org/pickup/21900/ and https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/.

- On 30 October 2026, StormGeo (part of Alfa Laval) completed its acquisition of Portchain. The announced product direction connects voyage intelligence with live berth plans and terminal crane/workforce planning. StormGeo/Portchain reports a network of more than 200 terminals representing over 40% of global container-terminal capacity. Treat these network figures as company-reported coverage, not independent performance evidence. This makes **JIT arrival alone an inadequate differentiation claim**. https://stormgeo.com/insights/stormgeo-acquires-portchain-to-advance-just-in-time-voyage-optimization-at-scale.

- The World Bank/S&P Global CPPI 2025 compares container-port performance using vessel time in port, including waiting and berth time, based on vessel movement records. It is a useful externally recognizable diagnostic, but it is not a door-to-door inland freight, lock-water, participant fairness or corridor resilience measure. We should report comparable vessel/port time components alongside broader chain metrics rather than relabeling our own composite as CPPI. https://www.worldbank.org/en/topic/transport/publication/cppi.

- IHO's Standards in Force page (last updated 29 October 2026) lists operational S-100 products including S-102 bathymetry, S-104 water levels, S-111 currents, S-122 protected areas, S-127 marine traffic management and S-131 harbour infrastructure. Consume their edition, update, provenance, coverage, datum, validity, uncertainty and terms explicitly. A specification being in force does not establish that a local dataset exists, is current, licensed or qualified for navigation. https://iho.int/fr/standards-in-force-0.

## Matched benchmark matrix

| Baseline | What it tests | Status in our programme |
|---|---|---|
| No-build / observed operating policy | Realistic service outcome rather than a synthetic optimizer-only comparison | Required when rights-qualified operating records exist; not yet reproduced |
| FCFS / oldest arrival first | Simple operational control | Existing lock scheduling comparator; freeze exact tie-breakers |
| FSR / ESR | Fairness versus lock throughput and chamber loading | Required literature reproduction; not yet claimed reproduced |
| Earliest feasible / least energy / minimum wait-slack | Transparent greedy rules | Required matched heuristic baselines |
| NSGA-II / NSGA-III / MOEA/D / SPEA-II | Multi-objective search quality | Required paper-aligned comparison on identical frozen cases |
| Switching max-plus / MILP | Serial multi-lock and multi-chamber placement | Required when vessel packing/chamber geometry enters scope; not represented by current model |
| CP-SAT or MILP reference | Independent larger-instance formulation and optimality gap | Next external solver differential; not yet implemented/reproduced here |
| DCSA/IAPH/IHMA-aligned coordination | Operational data exchange and timestamp semantics | Consume standards; compare against real workflow only with authorized data |
| StormGeo/Portchain class of products | Voyage-speed decisions linked to berth/terminal readiness | Industry capability reference; no access to matched vendor data or independent head-to-head performance results |

Do not call an unavailable baseline a loss or replace it with a weaker straw man. Mark it NotReproduced and disclose the exact reason.

## What the current fleet oracle does—and does not—establish

Current scope is a finite, integer-tick synthetic service chain with at least two demands, shared physical resource IDs and a bounded set of prequalified voyage alternatives. The current source declares 32 unit tests and 3 independent resource-accounting integration tests, includes a test-only full-fleet enumerator and a 20-variant differential sweep. The last recorded exact-head pass predates the two newest terminal/inland throughput-quota tests (30 unit tests + 3 integration tests); the newest workflow is queued, so do not describe all 32 unit tests as passed until that exact-head run succeeds. These tests are correctness evidence for the finite model only.

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

## Primary sources

- Liao et al., 2026 lock fairness/efficiency paper: https://doi.org/10.1016/j.oceaneng.2026.125004
- Xu et al., 2026 multi-objective lock scheduling: https://www.mdpi.com/2079-8954/14/5/507
- Segovia et al., 2026 multi-chamber SMPL/MILP: https://doi.org/10.1016/j.oceaneng.2025.123414
- DCSA Port Call Standard: https://dcsa.org/standards/port-call
- IMO/IAPH/IHMA Port Call Optimization Guide: https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/
- StormGeo/Portchain announcement, 30 October 2026: https://stormgeo.com/insights/stormgeo-acquires-portchain-to-advance-just-in-time-voyage-optimization-at-scale
- World Bank CPPI 2025: https://www.worldbank.org/en/topic/transport/publication/cppi
- IHO Standards in Force, updated 29 October 2026: https://iho.int/fr/standards-in-force-0