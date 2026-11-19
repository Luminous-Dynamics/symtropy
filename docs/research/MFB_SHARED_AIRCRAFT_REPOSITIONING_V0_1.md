# Shared aircraft, regional repositioning and nonanticipative allocation oracle v0.1

**Benchmark:** `MFB-SHARED-AIRCRAFT-001`  
**Implementation:** `tools/material-flow-benchmark/src/shared_fleet.rs`  
**CLI:** `mfb-evaluate-shared-fleet`  
**Fixture:** `synthetic-two-region-shared-aircraft.json`  
**Parent:** [LOGISTICS-MODE-OPTIONS-001 #6153](https://github.com/Luminous-Dynamics/symtropy/issues/6153)  
**Claim ceiling:** exact synthetic small-instance oracle only; never dispatch authority

## Why this increment exists

The previous demand-adaptive mode-options oracle assigned aerial capacity independently to each region. That was useful for proving nonanticipative mode activation, but it did not represent one physical aircraft being shared across regions.

This v0.1 adds one aircraft shared by two or three regions. It has an initial region, finite energy, a protected reserve, repositioning energy/cost and a limited set of directed regional transfers. Each period the policy chooses one target region or an idle/recharge action. It cannot serve two regions in the same period.

This is deliberately not another route solver, aircraft flight simulator, battery model, dispatch controller or fleet database. It composes the existing carrier/co-availability, cargo, route-allocation and permission/evidence owners.

## Mathematical formulation

For each period (t), the shared aircraft has a single location (l_t), energy (e_t), and a policy action (a_tin Rcup{mathrm{idle}}). One aircraft implies exactly one action per period.

The finite scenario set contains demand, weather and permission observations for each region. Scenarios with identical observed histories through period (t) share one decision node. Therefore the policy cannot branch on future scenario information.

The exact policy is selected to minimize the probability-weighted operating cost:

[
min_{pi} sum_{omegainOmega}p_omega
left(C_{mathrm{road}}+C_{mathrm{air}}+C_{mathrm{unserved}}+
C_{mathrm{reposition}}+C_{mathrm{idle}}ight)
]

subject to:
- one aircraft cannot serve multiple regions in the same decision period;
- aerial service requires current `clear_fresh` weather and `approved_fresh` permission at the aircraft's region;
- repositioning requires a fresh-in-domain link evidence status and current clear/approved observations at both endpoints;
- repositioning consumes a complete decision period and cannot deliver in that period;
- reposition energy is deducted before the aircraft arrives at the destination;
- neither service nor reposition may consume the protected energy reserve;
- idle charging cannot exceed the declared maximum aircraft energy;
- every scenario probability is an integer basis-point value and the total is 10,000;
- the policy is nonanticipative over the frozen observed-history tree.

The objective is an expected-cost objective, not a weighted safety score. Weather, permission, route-evidence and reserve checks are hard constraints outside the objective.

## v0.1 transfer-time boundary

The first implementation intentionally permits exactly **one discrete period per regional reposition**. It rejects links with longer travel times. A genuine multi-period flight requires an explicit in-transit state, time-indexed route-segment clearance, weather/permission changes during transit and a qualified abort/continue/return semantics. A counter alone is not enough to claim that behavior.

Within v0.1, a regional reposition:
1. consumes the current decision period;
2. consumes its declared reposition energy and cost;
3. updates the aircraft's region at the end of that period;
4. cannot perform a delivery in the reposition period;
5. may only serve from the new region in a later period and only after that period's weather and permission are checked again.

## Evidence references

Every regional weather observation has separate weather and permission evidence references. Every reposition link has its own source reference and evidence status. Synthetic fixtures use explicit `synthetic://` references. Non-synthetic consumers must replace these with actual, scoped evidence identities and freshness state.

The current model does not authenticate a signature, prove that an observation came from a physical sensor, or adjudicate whether an external authority issued permission. Those are independent Mycelix / Sol-Atlas / Symtropy evidence responsibilities. This module only refuses to act on states declared unknown, stale, conflicted, revoked, blocked or not approved.

## Exact oracle and baselines

The committed fixture has two regions, three periods and two stochastic scenarios with a shared initial observation prefix. It creates five nonanticipative policy nodes and enumerates (3^5=243) policies (urban, rural, idle at each node).

It reports three independently optimized policies over the same exact feasible policy set:
- **expected-cost optimal**: minimizes probability-weighted total cost;
- **CVaR90 optimal**: minimizes the mean cost in the worst 10% probability tail;
- **worst-case optimal**: minimizes the maximum scenario cost.

Each policy includes its own nonanticipative decisions. Every outcome, including truck-only and fixed-initial-region baselines, reports expected cost, CVaR90 and worst-case cost. CVaR uses fractional probability mass at the 90th-percentile boundary rather than simply taking the most expensive whole scenario. Risk objectives change economic preference only; they never relax the weather, permission, reposition-evidence or protected-reserve hard gates.

It also reports:
- truck-only baseline;
- fixed-initial-region aircraft baseline;
- aerial/road/unserved units;
- aerial and reposition energy;
- reposition cost, events and periods;
- idle charging energy and idle periods;
- blocked actions due to weather/permission or energy;
- terminal aircraft energy;
- explicit zero unauthorized-dispatch and protected-reserve-violation invariants;
- canonical SHA-256 input digest.

An independent enumeration of this synthetic fixture produces expected cost 1,511 units for the exact policy, 1,576 for fixed initial-region assignment and 1,980 for truck-only. These figures merely pin fixture arithmetic; they are not operational cost estimates or savings claims.

## Benchmark tournament

The immediate comparison is against the fixed-assignment and truck-only baselines already emitted by v0.1. The next tournament should add:
- myopic best-current-region reposition heuristic;
- greedy nearest-free aircraft / cheapest feasible region;
- fixed one-aircraft-per-carrier assignment;
- independent per-region capacity from MFB mode-options v0.1;
- deterministic receding-horizon policy;
- exact exhaustive oracle on small trees;
- MIP/branch-and-cut baselines where the published model and instance are reproducible;
- ALNS / adaptive tabu search / RL-guided ALNS baselines;
- MIP-labelled behavioral cloning and RL policies only after matched exact labels exist.

Do not treat algorithm labels as evidence of superiority. Reproduce the original problem definition, input instances, objective, fleet constraints, energy model and compute environment before comparing numbers.

## Current research and industry comparators

- **Truck-drone mobile station trailer:** Shi, Hong and Li, *Transportation Research Part E* (September 2026), TSP-MDST, DOI [10.1016/j.tre.2026.104975](https://doi.org/10.1016/j.tre.2026.104975). This establishes that a trailer-based mobile drone station is already a published architecture. Our concept must differentiate through shared capacity, road/air/AMR fallback, EV energy accounting, serviceability and manufacturing—not merely mobile deployment.
- **Joint multi-truck/multi-drone fleet sizing and routing:** Xie et al., *Drones* (28 August 2026), DOI [10.3390/drones10090658](https://doi.org/10.3390/drones10090658). Their two-stage framework jointly sizes fleets and routes; their ablation highlights multi-visit service and cross-vehicle retrieval. This is a direct future benchmark target for multi-aircraft, multi-carrier extension.
- **MIP → behavioral cloning → RL:** Bi et al., *Transportation Research Part C* (October 2026), DOI [10.1016/j.trc.2026.105857](https://doi.org/10.1016/j.trc.2026.105857). The paper describes fixed drone stations plus mobile trucks and reports 70–91% delivery rates across its nine configurations, with around 2 ms inference and 38,500–85,000× speedups over rolling-horizon MIP on its own setup. These figures are not transferable to our model. They motivate the exact-oracle → Symthaea policy-distillation path.
- **Multi-rendezvous resupply:** Tran et al., *Transportation Research Part E* (November 2026), DOI [10.1016/j.tre.2026.105082](https://doi.org/10.1016/j.tre.2026.105082). Their MILP/ATS framework jointly coordinates release dates and multiple rendezvous, and reports ATS outperforming CPLEX by 7.39% on its own problem instances.
- **Heterogeneous en-route operations:** Liu et al., *Transportation Research Part E* (November 2026), DOI [10.1016/j.tre.2026.105114](https://doi.org/10.1016/j.tre.2026.105114). It includes heterogeneous drones, multi-visit sorties, en-route launch/recovery and energy dynamics, with an RL-guided ALNS and a feasibility proxy.
- **Real deployment comparator:** Walmart announced on 5 November 2026 that its Zipline network had completed more than 2 million deliveries and that 300 additional sites were being added. This is a mature fixed-network operational scale benchmark, not evidence that vehicle-mounted shared pods are commercially solved.
- **Electric-truck manufacturing context:** IEA's Global EV Outlook 2026 reports around 440,000 electric trucks produced globally in 2025, with more than 90% of production in China; Daimler and Volvo can assemble battery-electric trucks on the same lines as diesel vehicles. The factory benchmark should compare flexible mixed-model integration against dedicated module production, not assume that a bespoke line is required.

## Acceptance targets

### Correctness and physical/evidence constraints
- 100% nonanticipative policy decisions on the frozen scenario tree.
- Zero cross-region teleportation; all v0.1 transfers consume one full decision period.
- Zero aerial dispatch under stale/unknown/blocked weather or stale/unknown/revoked/not-approved permission.
- Zero reposition on stale/conflicted/revoked link evidence.
- Zero double-booked aircraft: at most one serviced region per aircraft per period.
- Zero protected energy reserve violations.
- All scenario weights sum to 10,000 basis points.
- 100% permutation invariance for region, link, scenario and observation-array order.
- 100% agreement with hand-computable cases and the exact oracle on all bounded benchmark instances.

### Performance and SOTA qualification
- Exact oracle: 0% optimality gap on all admitted small instances; report node count, policy count, runtime and memory.
- Symthaea distilled policy: target mean optimality gap ≤1% and 95th-percentile gap ≤5% on held-out instances where the exact oracle is feasible; zero hard-gate violations.
- Heuristic / learned policy: target p95 decision latency ≤50 ms on a declared reference CPU for the bounded 2–3-region case, with hardware and confidence interval reported. Separately benchmark against the ~2 ms reported in Bi et al.; do not claim parity without matching hardware and problem semantics.
- On reproduced published instances, first match the published feasibility/objective within stated numerical tolerances; only then test Pareto improvements in cost, unserved demand, energy, repositioning and runtime.
- Keep a full service-cost-energy Pareto frontier rather than collapsing service, cost, energy and reliability into one opaque score.

## v0.2 and beyond

1. Add an explicit in-transit state with per-period route-segment weather/permission, emergency safe-state semantics and no unmodeled flight continuation.
2. Introduce multiple aircraft and carriers with joint time-window / pad / crew / charger occupancy; integrate #6083 rather than duplicate it.
3. Add heterogeneous payload, load-dependent propulsion energy and reserve uncertainty using existing aircraft/battery physics owners.
4. Add fixed hubs, mobile trailers, detachable pods, lockers and AMRs; reuse #6147 cargo semantics and #1601/#1639 route/resource allocation.
5. Introduce fleet acquisition, activation/deactivation, maintenance availability and repositioning as real options.
6. Distill exact small-instance decisions into Symthaea policies, preserve the exact oracle as a regression and counterfactual judge.
7. Use Mycelix for asset identity, serial/configuration, custody and maintenance evidence; use Sol-Atlas for sourced regional constraints; neither grants physical flight authority.

## Non-claims

The fixture contains synthetic assumptions and does not model a physical aircraft, flight envelope, route/airspace, takeoff/landing, return/recovery, payload-induced propulsion energy, battery degradation, actual charger, crew, traffic, customer time windows, legal permission or commercial cost. It is not a fleet recommendation, flight approval, dispatch command or SOTA superiority claim.


## Risk-aware policy comparison

The first exact oracle minimized expected cost only. That can hide low-probability, high-impact service failures. The current increment solves three separate exact objectives over the identical admissible policy set:

[
min_pi mathbb{E}[C(pi)]
]

[
min_pi operatorname{CVaR}_{0.90}(C(pi))
]

[
min_pi max_{omega in Omega} C_omega(pi)
]

For CVaR90, the algorithm sorts scenario costs from highest to lowest and consumes exactly 1,000 probability basis points, splitting a scenario's probability mass if the 90th-percentile boundary falls inside it. This is important: choosing the worst whole scenario is not generally equivalent to CVaR when scenarios have unequal probabilities.

The report now includes four independently optimized policies over the same feasible set: expected-cost-optimal, CVaR90-cost-optimal, minimax-cost, and **service-robust** (minimum worst-case unserved units). Each has its own nonanticipative decision map. Every policy outcome and the truck-only/fixed-assignment baselines report both cost and unserved-demand distributions (expected, CVaR90 and worst case).

A unit test pins the fractional-tail arithmetic: a 5% scenario costing 100 and a 95% scenario costing 20 produces CVaR90 of 60, not 100 or 20. The frozen two-scenario fleet fixture also checks that the risk-optimal policies are no worse on their own objective than the expected-cost-optimal policy.

The separate service-robust policy minimizes the maximum unserved units across the supplied scenarios, with expected cost as a deterministic tie-breaker. It prevents cost-only tail metrics from concealing a poor service outcome. This remains a finite-scenario risk measure, not a calibrated probability forecast or a tail-risk guarantee outside the supplied scenario set. Next steps are to add stress/adversarial scenarios with probability provenance, distributionally robust ambiguity sets, chance constraints for service-level requirements, and out-of-sample calibration. Hard safety/permission constraints remain outside all four economic/service objectives.
