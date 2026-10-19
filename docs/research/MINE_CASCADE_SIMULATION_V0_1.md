# MINE-CASCADE-001: simulation contract v0.1

## Purpose

This benchmark tests the composition of service availability over time in a synthetic underground-mine dependency graph. It complements MINE-SYSTEM-001: the campaign evaluator accounts for material, energy, product and constraints; this module checks whether local service states compose into a viable system as faults and evidence updates arrive.

It does not simulate detailed mine physics. Ventilation, geotechnics, water inflow, radio propagation, machine behavior and safety procedures remain separate models and authorities. The simulator consumes explicit service capacities, local states, evidence freshness, dependencies and failure domains.

## Paradigm shift

A component marked available is not sufficient to claim that its dependent service is viable. Service viability requires a known available local state, current evidence, capacity at or above the declared minimum envelope, satisfied dependency rules, and no active common-cause failure in its declared failure domain.

Dependencies support AllOf (every prerequisite) and AtLeast (explicit redundancy/quorum). This avoids assuming either that any upstream service suffices or that every redundant supply must be healthy.

A stale or unknown service is Unknown, not Viable. For an AtLeast rule, unknown candidates prevent a definitive failure classification only when they could still satisfy the threshold; they never count as viable.

## Deterministic simulation semantics

- Time is represented by integer ticks; callers declare the real-world duration per tick.
- Events are sorted by (at_tick, event_id). IDs are unique and define same-tick ordering.
- All services are reevaluated after every event; restoration of one component does not bypass downstream prerequisites.
- Service availability/unknown burden is measured as service-ticks over [0, horizon_tick). Concurrent affected services count separately; this is not wall-clock mine downtime.
- Cyclic graphs, unknown service references, duplicate IDs, invalid thresholds, non-finite/negative capacities, events past the horizon and exceeded service/event caps are rejected.
- Invalid input or a cap breach returns an explicit error; no partial run is reported as complete.
- Reports preserve post-event dispositions, canonical event order, per-service burden and critical-service burden.

## Frozen v0.1 cases

1. prerequisite loss and later restoration;
2. two redundant pumps with a shared power failure domain;
3. two redundant pumps with independent failure domains;
4. stale evidence propagation;
5. deterministic same-tick replay independent of input vector order;
6. cyclic graph rejection and explicit service-count cap;
7. unknown redundancy candidate not counted as viable.

Positive and negative controls are both required. The target is not to fail everything closed: independently powered redundancy should remain viable in the control case.

## Proposed benchmark gates

All are proposed targets, not observed results:

- 100% repeatability for identical scenario/policy/build inputs.
- Zero false-viable outcomes in frozen unknown/stale/common-cause controls.
- 100% agreement with an independently implemented reference on tiny graphs.
- Explicit failure for malformed references, cycles and cap overflow.
- Policy comparisons use identical event traces, evidence delivery times and resource envelopes.
- Adaptive recovery is not presumed better; it must improve a declared held-out metric without violating hard invariants.
- No weighted aggregate can hide unknown or critical-service loss.
- Synthetic results remain synthetic until calibrated models and held-out physical observations support stronger evidence claims.

## Competitive baseline and architectural boundaries

This work must be assessed against existing capabilities rather than marketed as a blanket replacement for mine-planning software.

### Existing Symtropy work to compose

- [MINE-SYSTEM-001 #1933 / PR #1934](https://github.com/Luminous-Dynamics/symtropy/pull/1934) owns mine-campaign resource/product accounting and Pareto comparison.
- [MINE-FLEX-001 #2068](https://github.com/Luminous-Dynamics/symtropy/issues/2068) owns safety-first flexible electricity/water scheduling.
- [MINE-FLEX-RECOURSE-001 #2098](https://github.com/Luminous-Dynamics/symtropy/issues/2098) owns causal recourse under sequential observations.
- [MINE-MISSION-TRACE-001 #2097](https://github.com/Luminous-Dynamics/symtropy/issues/2097), [FLOOD-EGRESS-001 #1963](https://github.com/Luminous-Dynamics/symtropy/issues/1963), and Symthaea's [MINE-COMMS-TRACE-001 #8066](https://github.com/Luminous-Dynamics/symthaea/issues/8066) own time-ordered communications / egress fault semantics.

Symtropy also has [MARITIME-CASCADE-001 / PR #1988](https://github.com/Luminous-Dynamics/symtropy/pull/1988). That evaluator addresses fixed-schedule operation timing, precedence-propagated delay, shared-resource overlap and causal-driver attribution; its PR describes an independent oracle sweep across 855 tiny schedule/shock configurations. It is related precedent, not a drop-in replacement: this mining module models three-state service viability, required AllOf versus AtLeast redundancy rules, multiple intersecting common-cause failure domains, capacity thresholds, and stale/unknown evidence. Keep the contracts separate unless a reviewed shared interface eliminates more duplication than it introduces. Do not create a second generic schedule propagation engine here.

### External industry and research baselines

- **Commercial mine planning / haulage:** Deswik Planning integrates designs and schedules; Deswik NOVA describes integrated scheduling, blending and haulage scenario management; Deswik LHS supports haulage flow, congestion / seasonal scenarios and calibration from site GPS and truck cycles: https://www.deswik.com/products/planning , https://www.deswik.com/products/nova , https://www.deswik.com/products/lhs . These are capability baselines and potential import/export counterparts, not independently scored benchmark contestants in this synthetic service simulator.
- **Public optimisation instances:** MineLib provides block models / precedence data across open-pit problems and also lists underground agricola and catan instances: https://minelib.org/v1/Datasets.xhtml . Use the existing MINE-REFERENCE / MINE-UPIT ownership for source-pinned scheduling adapters; this cascade package must not grow a second MineLib parser or production scheduler.
- **Interoperability:** the Global Mining Guidelines Group describes Open Mining Format (OMF) 2.0-beta as an open, Rust-based library and data-exchange format for 3D mining data: https://gmggroup.org/omf/ . Sol-Atlas should investigate OMF compatibility for spatial/block-model exchange instead of inventing a competing 3D interchange format. OMF does not define these service-failure semantics.
- **Validated process twins:** a 2025 SAG-mill digital-twin paper reports prediction errors below 5% for its specific 2.5-minute prediction horizon and 30-second sampling setup: https://doi.org/10.1016/j.mineng.2024.109076 . Treat this as a study-specific process-model comparison; it cannot be compared numerically to an E0 synthetic dependency simulator.
- **Model drift:** a 2025 paper on autonomous residual monitoring for metallurgical digital twins evaluates multivariate residual monitoring in offline and online settings: https://doi.org/10.1016/j.mineng.2024.109107 . Add residual / model-discrepancy monitoring when calibrated physical submodels enter the simulator.

Industry packages already handle important point solutions. Our plausible differentiator is an open, auditable composition benchmark for cross-service viability, common-cause resilience, uncertainty-aware safe restart, and fair policy comparisons across otherwise separate physical and operational models. Superiority remains unproven until equivalent inputs, outcomes, and evaluation boundaries exist.

### Proposed advancement gates

| Evidence stage | Gate |
| --- | --- |
| E0: contract semantics | deterministic replay; exhaustive small-state comparison against an independent recursive oracle; reject malformed inputs and exceeded work budgets |
| E1: cross-service behaviour | zero false-viable states in the frozen critical-hazard controls; shared-cause redundancy fails while independent redundancy remains viable |
| E2: policy quality | fixed and adaptive recovery policies compared on paired held-out disturbances; no hard-invariant regressions; publish per-class loss and regret |
| E3: calibrated models | define model-specific error, interval-coverage and drift alarms before running held-out physical records |
| E4: site evaluation | read-only shadow operation, domain expert review, operational change-control and explicit site authorization before any safety-relevant use |

E0/E1 results demonstrate software semantics only. The 144-state exhaustive oracle fixture is authored but not yet executed in this environment; it is not a test pass. Do not claim any percentage performance improvement until the named baselines, data and holdout protocol are frozen.

## Updated competitive benchmark map (19 Oct 2026)

### Decision-policy baselines

- **Static / stochastic / rolling-horizon planning:** Armstrong et al. (2021) compare rolling-horizon mine planning with a two-stage method under geologic uncertainty; the reported advantage depends on sampling-grid conditions, so do not assume adaptive planning always wins: https://doi.org/10.1016/j.resourpol.2021.102086
- **Partially observed sequential planning:** the 2026 POMDP mine-planning preprint reports large improvements over one-shot planning in its copper-gold case, while explicitly investigating prior misspecification. Reproduce its baseline only where case assumptions and code/data permit; treat reported preprint results as a hypothesis, not a generic target: https://arxiv.org/abs/2605.13702
- **Resource-constrained underground scheduling:** Hill et al. (2022) evaluate mathematical / constraint-programming strategies and report several difficult practical instances solved to within about 5% of optimality after hours of computation. This is a stronger scheduling comparator than a hand-built priority rule, but it does not directly score this service-viability contract: https://doi.org/10.1287/ijoc.2022.1222
- **Truck-dispatch RL:** an October 2026 *Computers & Industrial Engineering* study reports 15–17% relative improvement in productive hauling time and roughly 27% lower average queue waiting than a greedy baseline across six simulation scenarios. Use the published scenario definitions where available and attempt an apples-to-apples reproduction; do not lift those gains as expected outcomes for a different environment: https://doi.org/10.1016/j.cie.2026.112378
- **Configurable dispatch benchmark:** *Mining-gym* (Applied Soft Computing, available online 1 August 2026) describes a configurable discrete-event truck-dispatch environment with stochastic equipment behavior and failures. Investigate compatibility and shared scenario semantics before building another truck-dispatch RL simulator: https://doi.org/10.1016/j.asoc.2026.116153
- **Simulator-guided language-model scheduler:** the June 2026 Sim2Schedule preprint reports 94–99% of MILP-optimal NPV in its evaluated open-pit instances. It is a preprint comparator, not a verified performance claim for this repo, and any language-model policy must remain subordinate to an independent hard-constraint verifier: https://arxiv.org/abs/2606.10286

### Qualified physical model interfaces

- **NIOSH MFIRE** is established software for transient underground mine-fire and contaminant-spread effects on ventilation. Investigate it as an external qualified model / reference case for the relevant fire scenarios rather than writing a competing, unvalidated fire-physics model: https://www.cdc.gov/niosh/mining/tools/mfire.html
- Ventilation digital twins have published uncertainty analyses showing that airflow confidence-interval coverage and interval width depend on resistance estimation methods; therefore compare model error, interval coverage, sharpness and operational decision flips, not average airflow error alone: https://www.nature.com/articles/s41598-024-77978-3
- OMF 2.0-beta is a 3D mining-data interchange format. Use the Sol-Atlas spatial owner for any future adapter and retain the distinction between geometry interchange and service-dependency semantics: https://gmggroup.org/omf/

### Evidence hygiene

Search results identify the 2025 Scientific Reports article “Digital twin-driven deep learning prediction and adaptive control for coal mine ventilation systems” (DOI 10.1038/s41598-025-30513-4) as **retracted**. Do not use its reported accuracy, energy savings or response-time claims as supporting evidence or a benchmark target. Always check article status and correction/retraction notices before importing headline numbers into benchmark contracts: https://pubmed.ncbi.nlm.nih.gov/41345224/

### What “better than state of the art” means here

Do not compare this E0/E1 service simulator directly with a physical digital twin's percentage prediction error, a mining schedule's NPV gap, or an RL dispatch policy's hauling-time gain. These are different tasks and evidence levels.

A valid comparative claim must specify: task and decision boundary; input / event trace; source and holdout split; hard constraints and objective; policy-visible information; runtime/hardware; reference/oracle status; metrics and uncertainty; and whether the evidence is synthetic, calibrated simulation, retrospective site data or prospective shadow operation.

For this simulator, first target exact semantic agreement on tiny instances and zero false-viable results in its frozen adversarial cases. For the future repair-policy layer, target exact optimum on tiny cases, then measure regret, critical-service loss and feasibility against fixed, dependency-aware, and resource-constrained optimisation baselines on paired held-out traces. For physical submodels, compare only against validated domain references and qualified measurements.

## Next model layer

Add repair crews, spares, access-constrained repair precedence, restoration delays and policy decisions limited to observations delivered by the current tick. Integrate by references/adapters to existing communication-timeline and flood-egress benchmarks, not by duplicating their physics or receipt schemas.

## Research anchors

- Danko et al., Applications and verification of a computational energy dynamics model for mine climate simulations (2020): https://doi.org/10.1016/j.ijmst.2020.03.019
- A digital engineering framework for information-centric modelling of drill-and-blast tunnel excavation processes (2026): https://doi.org/10.1016/j.dte.2026.100096
- A framework to define, design and construct digital twins in the mining industry (2025): https://doi.org/10.1016/j.cie.2024.110805
- MineLib benchmark data and descriptions: https://mansci-web.uai.cl/minelib/Datasets.xhtml

These sources motivate model verification, data dependencies and benchmark design; none validates this synthetic service simulator as a mine-safety model.


## MINE-REPAIR-ORACLE-001 implementation slice

A separate Rust module, repair_oracle, provides a bounded exhaustive scheduler for tiny repair cases. The model represents repair duration, crew class/count, consumable spare inventory, job prerequisites, access-opening work and a weighted service-restoration objective. It rejects invalid/cyclic inputs, enforces explicit hard search caps, and returns SearchLimitExceeded rather than calling a partial search optimal.

This is a small exact oracle, not a scalable industrial optimiser. All jobs in a problem are required to complete within its finite horizon. The objective is the sum of critical_weight × completion_tick; this is a transparent synthetic proxy for restoration delay, not a complete mine-economics function.

Authored tests cover one-crew priority trade-offs, independent crew parallelism, access-opening prerequisites, spare scarcity, input-order determinism, search-cap behavior and invalid dependency/access graphs. A separate brute-force Cartesian-product oracle sweeps 450 feasible two-job combinations across duration, critical weights, horizon and crew capacity, independently enumerating every pair of start times and checking the optimizer's objective. They remain unverified until the focused exact-head workflow completes. See [MINE-REPAIR-ORACLE-001 #2169](https://github.com/Luminous-Dynamics/symtropy/issues/2169).

