# Maritime Staged Capital and Operations Benchmark v0.1

**Status:** proposed research contract; thresholds are targets, not measured results.  
**Tracker:** [MARITIME-CAPITAL-001 #1883](https://github.com/Luminous-Dynamics/symtropy/issues/1883).  
**Composes with:** [JIT arrival / port energy #1858](https://github.com/Luminous-Dynamics/symtropy/issues/1858), [contingent recovery #1867](https://github.com/Luminous-Dynamics/symtropy/issues/1867), [multimodal transport #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601), and [maritime replay #1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612).

## 1. Paradigm shift: optimize the evolution of the system, not only its operation

A locally efficient port-call plan can still lead to a poor long-lived system if the berth, yard, grid connection, shore-power installation, storage, charging or battery-exchange capacity was sized for the wrong demand mix, fleet compatibility, grid availability or policy regime. Overbuilding can strand capital; underbuilding can lock in congestion, expensive emergency responses and barriers to adopting cleaner vessels.

This benchmark asks whether the planner can select an **investment path plus the operations enabled by each commissioned stage**, instead of sizing infrastructure for one central forecast and optimizing dispatch afterward.

Recent research directly studies:
- Integrated berth, yard and shore-power planning under uncertain vessel arrivals (Ocean Engineering, 15 June 2026): https://doi.org/10.1016/j.oceaneng.2026.125850
- Collaborative berth scheduling and multi-energy dispatch for green ports (Omega, September 2026): https://doi.org/10.1016/j.omega.2026.103549
- Port integrated-energy planning with joint supply-demand uncertainty and scenario-dependent dispatch (Ocean Engineering, 2026): https://doi.org/10.1016/j.oceaneng.2026.126373

Reuters also reported on 16 October 2026 that lower-carbon marine-fuel demand had softened amid IMO policy uncertainty, with differing outcomes across fuels and ports. This is evidence of near-term uncertainty, not a permanent trend to hard-code: https://www.reuters.com/world/asia-pacific/shippers-demand-lower-carbon-marine-fuels-cools-imo-rules-hang-balance-2026-10-16/

These studies justify testing coupled investment and operations. Their reported savings, models, and assumptions are not results for Symtropy and must not be copied as our performance targets.

## 2. Ownership boundaries

| System | Canonical responsibility |
|---|---|
| Symtropy | Finite investment/dispatch portfolio enumeration, demand shifting, capacity accounting, objective vectors, scenario replay, and search-completeness reporting |
| Symthaea | Engineering admissibility for vessel/terminal/electrical/storage alternatives; declared physical limits and intended-use evidence; no invented design coefficients |
| Mycelix | Participant authority, procurement/asset provenance, owner-authorized contracts/capital commitments, and realized outcome events |
| Sol Atlas | Read-only spatial/temporal/economic-geography projection of existing/future capacity, exposure, access and evidence gaps |

Reuse existing owners including Symthaea [MARINE-FREIGHT-001 #7444](https://github.com/Luminous-Dynamics/symthaea/issues/7444), Mycelix [LOG-COMMONS-S0 #4901](https://github.com/Luminous-Dynamics/mycelix/issues/4901), [LOG-COMMONS-S1 #5044](https://github.com/Luminous-Dynamics/mycelix/issues/5044), [PROC-MULTIREQ-001 #5144](https://github.com/Luminous-Dynamics/mycelix/issues/5144) and [PROC-OUTCOME-001 #5120](https://github.com/Luminous-Dynamics/mycelix/issues/5120), plus Sol Atlas [MARITIME-ATLAS-001 #115](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/115) and [economic geography #90](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/90). Do not create a second simulator, canonical logistics event model, evidence store, energy ledger or finance authority.

## 3. Frozen synthetic problem

Start with finite portfolios of investment increments and period-by-period operating plans. Each investment option declares:
- stable ID/profile and admissibility status;
- upfront/fixed/variable cost and cost units;
- lead time, commissioning period, useful life, maintenance/replacement cost and explicit end-of-life/salvage assumptions;
- capacity increment by typed unit and scope;
- compatibility constraints with vessel/service profiles and existing assets;
- expansion granularity, minimum order and any decommission/stranding cost;
- source snapshot, version, assumptions, and evidence/uncertainty status.

Each period/scenario declares:
- demand by service/vessel/cargo class and decision-time information cutoff;
- compatible vessel/service uptake;
- existing/installed/available/reserved/used/outage/retired capacity as distinct states;
- electricity availability/prices and declared supply options; power (MW), energy (MWh), throughput and concurrent occupancy must remain separate;
- arrival/service windows, grid/shore-power compatibility and resource operation intervals;
- scenario-tagged policy, price, outage, demand and construction delay states with separate publication/effective/decision times.

All initial values are synthetic. Do not silently substitute guessed port demand, local equipment suitability, grid connection availability, equipment lifetimes or vessel performance for unknown real data.

## 4. Strategies to compare

1. **Central-forecast build-now:** commit capacity against one pinned forecast.
2. **High-envelope build-now:** size for a declared high-demand scenario, exposing stranded-capital risk.
3. **Defer for information:** wait for a declared update, counting service delay and lost opportunity during construction.
4. **Modular staged expansion:** add compatible increments after decision-relevant evidence arrives.
5. **Flexibility first:** use demand shifting, compatible alternatives and bounded recovery before large capacity commitments.
6. **Joint investment + operation + recourse:** choose stages and feasible operations across the frozen scenario family.
7. **Independent exhaustive tiny oracle:** enumerate the full finite investment/dispatch strategy space independently of production resource accounting and objective aggregation.

Do not assume that modular, deferred, electrified, or high-capacity options always win. An option has value only if its lead time, supplier availability, interfaces, financing/authorization constraints and activation conditions make it exercisable.

## 5. Feasibility and accounting invariants

- Track committed, under-construction, installed, commissioned, available, reserved, used, outaged and retired capacity separately.
- An investment cannot provide capacity before its declared commissioning time.
- Prevent future-information leakage: later demand/outage labels are inadmissible in earlier decisions.
- Keep power, energy, concurrency, mass/throughput and period quotas in typed, non-interchangeable units unless an explicit reviewed conversion exists.
- Count use by canonical physical-resource ID even when graph-edge/module IDs alias the same asset.
- Enforce budget, investment lead time, capacity, compatibility, service and declared engineering limits as hard constraints.
- Keep private cash flows, customer payments, public/fiscal costs and wider societal resource costs separate; do not double-count transfers as net social costs.
- Include served and unserved demand, delay, displacement, idle/stranded capacity, option costs and outage effects.
- Lifecycle emissions require a matched declared boundary and sourced factors. If only operating emissions are known, label them operational-only; do not compare them as full lifecycle totals.
- Unknown price, capacity, source, policy status or physical model means indeterminate—not zero, free, or available.

## 6. Metrics: expose regret and flexibility value

Report scenario-by-scenario and per strategy:
- capex by stage, opex, maintenance/replacement and declared end-of-life assumptions;
- net-present-cost calculation only when timing and discount assumptions are explicit;
- service fulfilled/unfulfilled, delays, queue/dwell, utilization by physical resource and emergency/expedite cost;
- shore-power utilization, storage throughput/curtailment and emissions inside a consistent boundary;
- unused/stranded capacity, wait-for-information benefits/costs and option activation costs;
- scenario-specific **hindsight regret** versus the best feasible strategy on that exact scenario, explicitly only as an evaluation comparator;
- worst-case outcomes and minimax regret when probabilities are unsupported;
- expected-value metrics only with defensible, calibrated scenario probabilities and out-of-sample evaluation;
- sensitivity to demand/fleet uptake, build delay, electricity/fuel price, policy regime, asset life, grid availability and discounting;
- distribution of benefits/costs by participant/service class when evidence supports that breakdown;
- run time, memory, candidates, search completeness and infeasible/incomplete/indeterminate counts.

Do not hide trade-offs in one weighted scalar. A strategy that wins under one central forecast may lose under a different plausible regime. Scenario regret on a hand-picked corpus does not prove robustness against unknown futures.

## 7. Frozen machine-readable corpus

The initial starter corpus is [MARITIME_STAGED_CAPITAL_V0_1.json](https://github.com/Luminous-Dynamics/symtropy/blob/research/maritime-capital-stage-v0-1/benchmarks/fixtures/MARITIME_STAGED_CAPITAL_V0_1.json). It contains 10 synthetic cases for pre-commissioning capacity, budget overruns, physical-resource aliasing, power/energy confusion, future-information leakage, policy effective dates, scenario-driven portfolio ranking reversal, truncated search, unknown cost, and input-order determinism.

This JSON is a versioned starter corpus, not an executable qualification result. The implementation gate requires an independent fixture loader, per-case assertion and a tiny exhaustive oracle. Expected dispositions must be recomputed/independently hand-checked before being treated as accepted oracle answers.

## 8. Independent oracle and frozen hostile cases

Use a tiny exhaustive enumerator that does not call production investment feasibility, capacity accounting or objective aggregation. Freeze hand-checkable portfolios with up to three candidate investment increments, three periods, three demands and three scenario branches.

Include at least:
- investment capacity counted before commissioning;
- investment budget overrun and simultaneous overlapping commitments;
- aliased resource IDs that overbook a shared charger, berth, feeder or storage asset;
- power confused with energy; storage energy incorrectly counted as instantaneous power;
- compatible-looking but unsupported vessel/shore-power interface;
- ignored outage or capacity degradation;
- demand/price/forecast known only after the decision cutoff;
- policy applied before publication/effective time;
- missing or contradictory cost/capacity evidence;
- decommission/salvage/lifetime counted inconsistently;
- low/high demand scenarios that reverse portfolio ranking;
- truncated strategy space falsely labelled globally optimal;
- equivalent input permutations with different output digest.

Distinguish INFEASIBLE, INDETERMINATE, INCOMPLETE, and fully enumerated output. Preserve negative controls and exact input manifests.

## 9. Proposed gates (not measured)

- 100% agreement with the independent oracle on every frozen complete tiny portfolio for feasible strategies, resource ledger, objective vector and tie-breaking.
- Zero budget overruns, hard-constraint violations, pre-commissioning capacity use and capacity double-counting among admitted plans.
- 100% detection of frozen future-information leakage, policy-effective-date and unit-confusion mutations.
- 100% correct INDETERMINATE / INCOMPLETE outcome for unknown mandatory data and search-budget exhaustion.
- 100/100 replay and equivalent canonical input permutations produce byte-identical result receipts.
- At least two frozen scenarios produce an intentional oracle-verified ranking reversal; the evaluator must report it rather than assert a false universal winner.
- Report Pareto-frontier coverage, regret metrics, capex/opex/service vectors, stranded capacity and compute cost at increasing finite sizes.
- Do not set numeric ROI, cost-savings, emissions-savings or throughput targets before source units/boundaries are qualified.
- After synthetic conformance, consider a rights-cleared historical case with original dated data and an explicit decision cutoff. Published study savings are not our measurements.

## 10. Authority and claim ceiling

Effects-disabled planning only. This benchmark does not buy equipment, contact suppliers, authorize a capital commitment, reserve a grid connection, authorize construction, dispatch a vessel, or qualify real port engineering. Each result is a scenario-bounded proposal. Passing the synthetic oracle establishes finite-model correctness, not ROI, real-world cost savings, emissions reduction, permitting, safety, physical grid feasibility, or regulatory compliance.

## 11. Research references

- Wang et al., *An integrated robust framework for berth, yard, and shore power planning considering vessel arrival uncertainty*, Ocean Engineering, 15 June 2026: https://doi.org/10.1016/j.oceaneng.2026.125850
- Li & Hu, *Collaborative optimization of berth scheduling and multi-energy dispatch for green ports*, Omega, September 2026: https://doi.org/10.1016/j.omega.2026.103549
- Che et al., *Port integrated energy system planning with multi-energy collaboration under uncertainty*, Ocean Engineering, 2026: https://doi.org/10.1016/j.oceaneng.2026.126373
- Reuters, *Shippers' demand for lower-carbon marine fuels cools as IMO rules hang in balance*, 16 October 2026: https://www.reuters.com/world/asia-pacific/shippers-demand-lower-carbon-marine-fuels-cools-imo-rules-hang-balance-2026-10-16/
