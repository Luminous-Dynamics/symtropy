# Multimodal Transport Benchmark Ladder v0.1

**Status:** proposed benchmark contract; not an optimizer performance result, real-world validation, or operating approval.  
**Scope:** Symtropy route search and resource-constrained allocation, with evidence and projections composed from Symthaea, Mycelix, and Sol Atlas.  
**Canonical implementation work:** [TRANSPORT-OPT-001 #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601), [TRANSPORT-ALLOC-002 #1639](https://github.com/Luminous-Dynamics/symtropy/issues/1639), [data admission #1631](https://github.com/Luminous-Dynamics/symtropy/issues/1631).  
**Research reviewed:** 2026-10-15.

## 1. The benchmark question

Do not ask only whether an optimizer returns a cheap route. Test whether it returns a feasible, reproducible, evidence-bounded plan; identifies its actual search scope; exposes unserved demand; and improves service under normal and disrupted conditions without crossing capacity, safety, legal, or authority boundaries.

Keep four claim levels distinct:

1. **Algorithmic correctness:** the result matches an independent reference within a declared finite search domain.
2. **Scenario-model validity:** the scenario semantics, capacity/resource identities, timing, units, and perturbations represent the question being tested.
3. **Empirical transport validity:** results reproduce or improve on held-out observations from permitted, documented real sources.
4. **Operational qualification:** a separately authorized process has established that the real resources, timetable, commitments, and operating controls are valid.

Passing a lower level never establishes a higher one.

## 2. B0 — exactness and determinism (pull-request gate)

### Required tests

- Compare the batch allocator with a simple independent exhaustive enumerator over tiny networks.
- Exercise every currently implemented scalar objective: operator cost, user cash cost, external cost, energy, GHG, safety-risk proxy, and disruption-risk proxy.
- Check exact objective value and selected route signature, including the documented lexicographic tie-break.
- Permute input demand order and require byte-equivalent/structurally identical normalized reports.
- Include infeasible cases, zero/invalid inputs, checked-arithmetic overflow, and search-budget exhaustion. Never report an incomplete search as optimal.

### Targets

- **100% exact agreement** with the independent oracle for every feasible generated fixture within the declared candidate pools.
- **Zero capacity violations** and **zero silent demand omissions**.
- **100% deterministic replay** for fixed inputs, versions, limits, and configuration.
- Each report's optimality scope must match the candidate-pool coverage evidence; candidate-pool completeness remains unproven unless separately verified.

The tests added alongside this document cross-check 36 small capacity/quantity combinations for the cost objective and 252 combinations across the seven supported objectives. For each objective fixture they compare exact route signatures and objective values against an independent exhaustive reference, then verify the reported batch through the existing independent verifier under the original inputs, reversed demand order, and reversed candidate-route order. The reference scores from edge fields rather than production route aggregation or objective scoring. The verifier checks aggregate metrics, objective value, per-edge capacity use, demand coverage, candidate membership, and optimality scope. Candidate-pool completeness remains unproven: the oracle checks the declared candidate domain only, and caller completeness labels remain assertions. These are deterministic synthetic tests, not production-scale evidence.

## 3. B1 — resource identity and temporal feasibility

Exercise the next integration gate in #1639. A capacity constraint must follow the physical resource across graph-edge aliases and graph revisions, not merely an edge string.

Mandatory adversarial families:

- Two different graph edges alias the same crane, berth, rail section, siding, road segment, or other explicitly identified asset.
- Operations overlap on one physical resource while their graph edges or accounting scope labels differ.
- Adjacent half-open intervals reuse capacity only where the declared resource policy permits it.
- Evidence expires, is superseded, conflicts, uses incompatible units/time profiles, or fails to cover the complete proposed operation interval.
- A resource crosswalk changes under a graph revision and invalidates prior bindings.
- Missing capacity, source identity, unit, or time basis remains unknown/rejected; it is never coerced to unlimited, zero, or an assumed available slot.
- A valid plan never implies a reservation, dispatch, or authority to operate.

### Targets

- **Zero** false-feasible allocations in the mandatory negative-control corpus.
- **Zero** overcommitments in independently recomputed physical-resource/time-window accounting.
- **100%** of rejected cases include deterministic failure categories and retain source/rejection provenance.
- Cross-check the production sweep against an independent reference on generated small interval/resource cases before relying on complex scenarios.

## 4. B2 — search quality and computational scale

Publish separate results for the supplied candidate pool and for the actual candidate-generation procedure. Do not call a solution globally optimal merely because the allocator exhaustively searched a heuristic or Pareto-pruned subset.

### Initial fixed benchmark families

| Family | Workload | Expected check |
|---|---|---|
| Tiny exact oracle | Up to 7 demands and up to 4 candidate routes per demand; small capacities and mixed objectives | Exhaustive reference agreement within the declared search budget |
| Shared bottlenecks | Multiple origin approaches join the same downstream asset | Aggregated use follows physical-resource identity |
| Candidate omission | Remove a bottleneck-useful dominated detour | Result does not claim network-wide optimality |
| Infeasible / bounded | No feasible combination or deliberately small search cap | Explicit infeasible/budget-exceeded result; no partial optimum |
| Growth curve | 1, 2, 4, 8, 16, 32, 64 demands where the declared search budget permits | Search steps, route candidates, wall time, peak memory, feasible/infeasible rate |
| Input-order stress | Permute demand and candidate ordering with fixed content | Identical canonical result and normalized digest |

The seven-demand/four-route target is intended as a small exact qualification envelope, not a claim that arbitrary instances of that size are easy. Record actual search steps and keep a hard budget.

### Performance reporting

For every build and benchmark corpus, record:

- exact source commit and benchmark-manifest SHA-256;
- compiler/toolchain, build profile, OS, CPU, memory, and feature flags;
- demand count, candidate count/distribution, graph node/edge count, physical resource count, time-window count, and number of disruption scenarios;
- p50/p95/max wall time, peak memory where measured, search steps, and outcome counts;
- objective, units, rejected/unknown inputs, and whether the search was exact over caller-supplied pools or independently verified search domains.

First capture a reproducible baseline. Set machine-specific latency/throughput release budgets only after repeated measurement on pinned reference hardware; do not invent a universal latency threshold or compare dissimilar runners.

## 5. B3 — resilience and service outcomes

A resilient plan preserves the service people need, not just a connected path. Build deterministic, versioned scenarios for:

- port or terminal closure;
- rail/road corridor loss;
- constrained energy or charging availability;
- equipment outage and maintenance delay;
- weather disruption and increased transit-time variance;
- missed intermodal connection;
- stale/conflicting capacity evidence;
- communication partition that delays evidence synchronization but must not authorize physical movement.

Use identical demand, source snapshot, capacity semantics, and perturbation sets for each planner baseline.

### Report these metrics independently

- demand quantity served and unserved, including critical-demand cohorts;
- missed delivery windows and delay distribution;
- where source data support them, empirical on-time delivery, median and p95 end-to-end transit time, source-defined Planning Time Index / Buffer Index, terminal dwell, transfer wait, gate turn time, and mode-specific throughput; do not mix segment travel-time variability with shipment-level end-to-end reliability;
- whole-life/private/fiscal/social costs kept in separate ledgers;
- GHG and energy, with provenance-bearing factors;
- worst-case and distributional service degradation by geography/service class;
- recovery time and recovered service trajectory where a time-stepped model exists;
- alternative-route feasibility, capacity utilization, and residual bottlenecks;
- plan regret versus the best feasible plan in the declared finite benchmark domain;
- uncertainty/coverage and the share of inputs that are unknown, assumed, or disputed.

A review of multimodal freight resilience research (2026) notes that studies use heterogeneous combinations of cost, regret, unmet demand, throughput and delay; explicit time-evolving recovery trajectories are less common. The benchmark should therefore publish a stable metric contract rather than hide outcomes in one composite resilience score.

### Targets

- H0 regression corpus: at least **100 named deterministic disruption fixtures** across at least eight distinct failure families, including selected pairwise interactions. This is a coverage target, not evidence of real-world disruption probabilities.
- After the scenario semantics and parameter provenance qualify, add a separate **1,000-run seeded sensitivity ensemble** for declared uncertainty ranges. Do not attach real-world probability interpretations unless the probability model is externally supported; report Monte Carlo uncertainty and sensitivity rather than treating more seeds as validation.
- **100%** of generated disruptions have stable identities, explicit affected resources, and reproducible seeds.
- **100%** of unserved demand is reported; no silent dropping or implicit demand splitting.
- **Zero** safety/legal/capacity constraint violations in any scenario considered feasible.
- Compare optimized plans with at least a declared cheapest-route baseline and an operationally relevant baseline when one is available.
- Report improvements and regressions by scenario; do not impose an arbitrary positive benefit threshold before a real, representative baseline is established.

### Historical-event replay add-on (separate from synthetic stress tests)

A historical replay is a different evidence class from a generated disruption. The 2026 systematic review reports that dynamic cascade/recovery representation and historical-event validation remain relatively uncommon; the September 2026 port-disruption study illustrates why capacity loss, rerouting, queues and downstream consequences may interact nonlinearly ([systematic review](https://doi.org/10.3390/infrastructures11040130), [port-disruption study](https://doi.org/10.1016/j.multra.2026.100313)).

For each proposed replay:

- Freeze a pre-event information cutoff. Data published after the cutoff must be excluded from initial planning inputs; later arrivals may only enter as timestamped updates if the replay models those updates.
- Represent separate source publication/observation time, system acquisition/record time, policy/evidence effective-from time, and effective-until/supersession time. A policy can be known now but apply to future booking dates; an outcome can describe the past but only become published later.
- Bind the event chronology, affected physical resources, capacity degradation, source releases, temporal basis, units, geographic scope and observed service outcomes to immutable artifact hashes.
- Preserve contradictions between source narrative, tabular values, and later reports as unresolved records until precedence is justified. Do not auto-correct a source from another without retaining the conflict and the reason for resolution.
- A later advisory that supersedes an announced policy before the old policy becomes effective must make the earlier policy non-applicable for that window while preserving it in the knowledge history.
- Compare static/observed baselines, a declared deterministic routing baseline and event-triggered replanning on paired inputs and the same event timeline. Add further baselines only when their policies are specified before evaluating the holdout.
- Reconstruct queues, inventory drawdown, cross-modal substitution and recovery trajectories only where the observations identify those effects. Otherwise, label them as assumptions in a scenario, not as replayed facts.
- Stratify congestion measures by their actual population (e.g. booked vs unbooked arrivals, arrival date vs scheduled transit date), rather than treating every vessel at anchorage as one undifferentiated queue.
- Report quantity served/unserved, delay and missed windows where measured, throughput loss, service/recovery curve, time to a declared recovery threshold, replanning latency, plan churn, bottleneck displacement and uncertainty. Keep geography and service-class outcomes visible; do not collapse them into one resilience scalar.
- Preserve adverse events and failed replications. Unknown event probabilities stay unknown; hand-selected event catalogs are not a calibrated frequency distribution.

Concrete pilot: [Panama November 2026 prospective evaluation draft](TRANSPORT_PANAMA_2026_PROSPECTIVE_EVAL_V0_1.md) freezes simple persistence and trailing-rate baselines before the target month's report. It remains draft because original source artifacts and usage rights are not yet admitted; it does not yet contain a policy-aware model prediction. One month is a case study, not generalization evidence.

**Acceptance:** at least one candidate event has a rights/scope review and reproducible artifact receipt; temporal-leakage, timestamp-shift, conflicting-source, superseded-policy, queue-population and missing-capacity controls are caught by hostile fixtures; an independent checker recomputes the cutoff, chronology, service totals and recovery metrics. If a permitted pre-event snapshot or observed outcomes do not exist, retain the event as a labeled stress case rather than claiming empirical replay. A single event supports a case study, not generalized resilience claims.

Tracking: [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671). This gate composes with the Symthaea measurement-validity/evidence graph and does not create a parallel event/source registry.

## 6. B4 — distributional value and whole-life outcomes

Do not allow aggregate savings to conceal a loss of essential access for a subset of users or regions. When demand data support it, report:

- essential-service reachability and delivered quantity by region and service class;
- access-time distribution, affordability, accessibility constraints, and the worst-served cohorts;
- concentration/inequality measures only with a stated interpretation and sensitivity analysis;
- lifecycle cost, energy/GHG, maintenance burden, worker impacts, and infrastructure dependencies;
- Pareto alternatives and how selected policies change the trade-offs.

Equity has multiple defensible formulations; the chosen notion of fairness must be explicit and reviewed with affected stakeholders. A scalar weighted score is not a substitute for showing distributional outcomes.

## 7. B5 — empirical validation and data admission

Start with public or operator-provided artifacts only after source rights and allowed uses have been reviewed. Preserve exact artifact bytes, hashes, release/version, geographic and temporal coverage, units, extraction code, schema fingerprint, and rejected-row reasons.

For the South African freight pilot, issue #1631 identifies Transnet TRIM's 2026/27 network-statement annexures as promising section-level sources and Stats SA's land-transport series as an aggregate trend baseline. Those are source leads until the relevant files are acquired, rights-reviewed, inspected, and frozen; aggregate freight totals are not route capacity.

Before a performance claim, compose the Symthaea transport benchmark portfolio and evidence owners rather than starting another registry or ledger: [external benchmark architecture #6255](https://github.com/Luminous-Dynamics/symthaea/issues/6255), [minimal complementary portfolio #6270](https://github.com/Luminous-Dynamics/symthaea/issues/6270), [decision-bound evidence ledger #6277](https://github.com/Luminous-Dynamics/symthaea/issues/6277), and [measurement validity/generalization #6283](https://github.com/Luminous-Dynamics/symthaea/issues/6283). These own benchmark rights/coverage/transferability semantics; this Symtropy document owns only the optimizer and scenario benchmark acceptance criteria.

- compare against reproducible simple baselines and any permitted incumbent baseline;
- choose chronological and route/segment holdouts before tuning;
- report coverage, missingness, calibration/uncertainty, and subgroup support;
- prohibit future-information leakage and test deliberate leakage negative controls;
- retain failed scenarios and discrepancies instead of tuning them away;
- have an independent checker recompute split membership, capacity conservation, metrics, and report totals.

No real-data accuracy, capacity, reliability, cost saving, or generalization claim may be based on synthetic tests.

## 8. Composition boundary across projects

- **Symtropy:** deterministic routing, allocation, bounded scenario execution and physical-resource accounting; this benchmark ladder belongs here.
- **Symthaea:** engineering models, forecasting/advisory reasoning, uncertainty, transport benchmark registry/rights, benchmark portfolio and decision-bound validity/generalization evidence ([#6255](https://github.com/Luminous-Dynamics/symthaea/issues/6255), [#6270](https://github.com/Luminous-Dynamics/symthaea/issues/6270), [#6277](https://github.com/Luminous-Dynamics/symthaea/issues/6277), [#6283](https://github.com/Luminous-Dynamics/symthaea/issues/6283)); no self-granted dispatch authority.
- **Mycelix:** participant federation, signed/traceable operational events, standards interoperability, custody and owner-authoritative reservation/commit workflows where implemented.
- **Sol Atlas:** read-only spatial projection and inspection of evidence, uncertainty, conflicts and alternatives.

Transport geometry, physical movement, communication/contact availability, shipment-event semantics, evidence synchronization, and authority are different layers. Their edges must not be merged into a single unlabeled graph.

## 9. Research references

- Mandouri et al. (2026), *Cascading economic losses from port disruptions under capacity constrained multimodal freight networks*, Multimodal Transportation 5(3), 100313. https://doi.org/10.1016/j.multra.2026.100313
- *Modeling the Resilience of Multimodal Freight Networks Under Disruptions: A Systematic Review* (2026). https://www.mdpi.com/2412-3811/11/4/130
- Zhang et al. (2026), *Balancing accessibility, equity, and sustainability in public facility location-allocation*. Applied Geography 192, 104027. https://doi.org/10.1016/j.apgeog.2026.104027
- FHWA, *Freight Performance Measure Primer* — travel-time reliability, Planning Time Index / Buffer Index, terminal dwell, gate reliability and mode-specific flow measures. https://ops.fhwa.dot.gov/Publications/fhwahop16089/index.htm
- BTS/FHWA, *Freight Analysis Framework 6* (OD flows by region, commodity and mode; 2022 base year). https://www.bts.gov/faf

These references support benchmark design and metric selection. They do not demonstrate that the current implementation achieves any target above.
