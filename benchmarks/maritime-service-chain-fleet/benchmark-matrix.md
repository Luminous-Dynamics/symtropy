# Fleet and corridor benchmark matrix v0.1

Status: research benchmark plan. All fleet-oracle fixtures currently synthetic. No vendor comparison, held-out operational result, or better-than-SOTA result is claimed.

## 1. Competitive baseline matrix

| ID | Baseline family | Matched comparison | What it establishes / does not establish |
|---|---|---|---|
| B0 | Existing/no-build operating policy | Reconstruct an observed/static operating plan where source data permits; otherwise mark NotReproduced | Operational reference, not a fabricated schedule |
| B1 | Silo composition | Optimize voyage, locks, berth/handling, shore energy and inland transfer independently, then compose their plans | Measures coordination value and detects cross-silo infeasibility |
| B2a | FCFS | Same arrivals, operation durations, resource calendars and hard gates | Transparent queue baseline |
| B2b | Directional batching | Same lock direction, switching and capacity rules | Measures convoy/batching effects and direction-switch cost |
| B2c | Least-wait-slack / oldest-first | Same queue and eligibility rules | Measures deadline/fairness sensitivity |
| B2d | Published fairness/efficiency rules | Reproduce Liao et al.'s FSR/ESR definitions and tuning protocol; do not relabel a locally invented rule as FSR/ESR | Compare average wait, chamber-area load factor and inversion rate |
| B3 | Serial-lock + sailing-speed MILP | Reproduce Ji et al. (EJOR, 16 Sep 2026, DOI 10.1016/j.ejor.2026.01.038) on the recovered decision variables and constraints | Strong reference for serial-lock/speed co-optimization; no claim until matched reproduction |
| B4a | NSGA-II | Same encoded decision space, objectives, constraints, initialization, evaluation budget and stopping rule | Multi-objective reference |
| B4b | NSGA-III | Same conditions as B4a | Multi-objective reference |
| B4c | MOEA/D | Same conditions as B4a | Multi-objective reference |
| B4d | SPEA-II | Same conditions as B4a | Multi-objective reference |
| B5 | Exact tiny oracle | Exhaustive enumeration or solver-proven tiny cases with independent constraint/metric implementation | Feasibility and Pareto-front correctness reference |
| B6 | Greedy operational rules | Earliest feasible, minimum-energy feasible, just-in-time, oldest-first, greedy shared-resource fit | Simple reproducible operational comparators |
| B7 | Commercial JIT/berth coordination capability | Match event semantics and available outputs to DCSA Port Call Standard; use public product feature claims only as capability context unless matched data/experiments are available | The competitive bar for voyage/berth synchronization, not a reproducible performance result |
| B8 | Integrated berth / quay-crane / yard-crane / shore-power / multi-source energy MILP | Reproduce Li & Hu (Journal of Cleaner Production, 18 Aug 2026, DOI 10.1016/j.jclepro.2026.149105) using matched berth, crane, OPS, storage, renewable and grid inputs; compare centralized, two-stage and ADMM coordination where the formulation can be recovered | Tests whether berth/terminal decisions and port energy dispatch are jointly represented; paper-specific savings are not portable |
| B9 | Robust berth / yard / shore-power planning under arrival uncertainty | Reproduce Wang et al. (Ocean Engineering, 15 Jun 2026, DOI 10.1016/j.oceaneng.2026.125850) with the paper's uncertainty set, action space and compute budget; compare robust optimization and its hierarchical multi-agent SAC approach | Tests uncertainty-aware planning and OPS compatibility; its reported 4–13% cost savings are study-specific, not a Symtropy target |

### Industry anchors and scope boundaries

- **StormGeo / Portchain:** the acquisition announced 30 October 2026 connects StormGeo voyage route/speed optimization with Portchain berth, crane and workforce planning using shared operational data. StormGeo reports a Portchain network of more than 200 terminals and over 40% of global container-terminal capacity. This means just-in-time berth alignment alone is not a sufficient differentiation claim. Source: https://stormgeo.com/insights/stormgeo-acquires-portchain-to-advance-just-in-time-voyage-optimization-at-scale .
- **DCSA Port Call Standard:** provides a shared operational language and digital data exchange for berthing, pilotage, towage, cargo operations and bunkering. Conform event semantics and information cutoffs; do not claim to replace the standard. Source: https://dcsa.org/standards/just-in-time-port-call- .
- **World Bank CPPI 2025:** compares container-port efficiency using vessel time in port, including waiting and time at berth. It is a useful port-side diagnostic but is not a lock, waterway, inland-terminal or door-to-door freight metric. Source: https://www.worldbank.org/en/topic/transport/publication/cppi .
- **IHO S-100:** the current IHO in-force page (updated 29 October 2026) lists S-102 bathymetry, S-104 water levels, S-111 currents, S-122 marine protected areas, S-127 marine traffic management, S-129 under-keel clearance management and S-131 harbour infrastructure among the operational product specifications. Adapter conformance and source fitness remain separate from physical or navigation qualification. Source: https://iho.int/en/standards-in-force-0 .
- **Integrated terminal-energy scheduling:** Li & Hu (Journal of Cleaner Production, 18 Aug 2026, DOI 10.1016/j.jclepro.2026.149105) linearize an integrated berth, quay/yard crane, onshore power and multi-source energy dispatch MINLP into MILP and compare centralized, two-stage and ADMM coordination. The reported up-to-19.87% cost reduction belongs to that paper's case, not a transferable target. Source: https://www.sciencedirect.com/science/article/pii/S095965262601646X . 
- **Robust berth/yard/OPS planning:** Wang et al. (Ocean Engineering, 15 Jun 2026, DOI 10.1016/j.oceaneng.2026.125850) combine uncertain vessel arrivals, OPS compatibility and carbon costs with adversarial reinforcement learning and hierarchical multi-agent SAC. The reported 4–13% cost savings are specific to the study's instances and assumptions. Source: https://doi.org/10.1016/j.oceaneng.2026.125850 .

## 2. Never-collapse objective vector

Report each of these independently; no weighted scalar may compensate for a hard-constraint violation.

### Fulfilled service
- Requested, eligible, scheduled, delivered and unserved demand by service unit.
- Fulfilled / eligible ratio by participant and comparable service group.
- Missed delivery windows, partial service and unfulfilled amount.
- End-to-end elapsed time and each stage's contribution.

### Time and congestion
- Median, p90, p95 and maximum waiting/delay by participant and service group.
- Lock service time, queue inversion rate, direction-switch time and chamber load factor.
- Berth wait, berth time, terminal handling time, onward-transfer delay and connection misses.
- Schedule churn after each admissible observation.

### Physical/resource constraints
- Peak concurrent W at each shore-power connection and cumulative imported Wh per declared window (once the separate power-capacity contract is implemented).
- Propulsion, auxiliary, shore-grid import and inland traction energy at separate meter boundaries.
- Lock water use by lock cycle and cumulative accounting window.
- Resource utilization and headroom by time window; zero overcommitment.
- Lifecycle emissions only when source factors and boundaries support them; keep CO2, CH4 and N2O separate when evidence supports it.

### Fairness and distribution
- Per-participant fulfilled/eligible share, median/p95/max wait and service displacement.
- Maximum wait and max-min fulfillment only within comparable service units.
- Which demand or participant absorbs delay, cost, water use, risk, reserve consumption or flexibility burden.

### Algorithm quality
- Agreement with independent exact enumeration on tiny cases.
- Pareto precision/recall, hypervolume (with a pre-fixed reference point), epsilon indicator and IGD+ where a trusted reference front exists.
- Best-known objective gap and proven optimality gap; these are not interchangeable.
- Candidate count, operation-start states, memory, p50/p95/p99 runtime, timeout and completeness.

## 3. Qualification ladder

| Stage | Workload | Required result |
|---|---|---|
| Q0 | One-demand contract cases | 100% contract validation and independent temporal-oracle agreement |
| Q1 | At least 20 tiny multi-demand shared-resource fixtures | 100% independent fleet-reference agreement on complete frontier, feasibility, objective vector and joint infeasibility |
| Q2 | At least 30 frozen end-to-end synthetic corridor cases | Zero concurrent-capacity or cumulative-quota violations; zero false-eligible evidence; 100/100 canonical permutation replays |
| Q3 | Matched solver benchmark | Reproduced serial-lock MILP and NSGA-II/III, MOEA/D, SPEA-II with identical decision space, evidence cutoff, constraints and compute budget |
| Q4 | Held-out operational reconstructions | Rights-cleared records, source provenance, chronological cutoffs, independent outcome verification and paired baselines |
| Q5 | Contingent recovery | Bounded recourse policies tested against fixed-plan, greedy-replan and independent tiny policy enumeration; report displacement and recovery trajectories |

### Promotion target

A research improvement should show a repeatable **at least 5% improvement on one predeclared service or resource axis** on held-out cases, while all hard gates remain at zero violations and every other predeclared service/fairness axis remains within stated non-inferiority margins. This is an aspirational research gate, not an achieved result or universal industry threshold.

Do not set absolute fleet-size/runtime pass thresholds before a pinned harness, reference hardware, solver version, candidate set, warm-up policy and baseline run exist. Publish scaling curves for 2, 5, 10, 25, 50, 100 and larger demand counts as the implementation permits.

## 4. Paradigm-level differentiation hypothesis

Commercial JIT products connect vessel movement to port readiness. The research hypothesis here is a wider, evidence-qualified corridor service system:

1. Compose vessel alternatives with time-dependent channel, serial lock, water quota, berth, handling, shore-power, terminal and inland-transfer resources.
2. Preserve source revision, evidence validity and decision-time information cutoff throughout each resource constraint.
3. Offer a Pareto set and independently checkable decision certificate rather than one opaque recommendation.
4. Prepare bounded contingency branches for disruptions, but never imply that a plan booked or dispatched an asset.
5. Report delivery, fairness, water, power, energy, emissions, option cost and recovery independently.
6. Keep participant authority with Mycelix, engineering admissibility with Symthaea, deterministic evaluation with Symtropy, and spatial/source projection with Sol Atlas.

The differentiator must be measured on matched instances, not inferred from the architecture diagram.

## 5. Explicit claim ceiling

- A synthetic exact-oracle PASS proves only the declared finite model and its fixtures.
- A hash proves byte identity, not publisher authenticity or legal permission.
- S-100 product conformance does not prove regional coverage, freshness, physical suitability or navigational authority.
- A solver-proven optimum is only optimal within the recovered formulation, candidate set, parameters and information cutoff.
- CPPI port time must not be relabelled as waterway throughput or door-to-door delivery.
- Vendor capability descriptions and network footprint claims are not matched performance measurements.
- No SOTA superiority or operational benefit claim until the matched reproduction and held-out-data gates pass.