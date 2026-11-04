# CivBench Decision-to-Service Closure v0.1

**Status:** design-only composition contract; not externally pre-registered  
**Owner:** Symtropy / CivBench composition and qualification  
**Parent:** [#3527 — CIVBENCH-DECISION-CLOSURE-001](https://github.com/Luminous-Dynamics/symtropy/issues/3527)  
**Inputs:** [event evaluator PR #3513](https://github.com/Luminous-Dynamics/symtropy/pull/3513), [experiment qualification PR #3412](https://github.com/Luminous-Dynamics/symtropy/pull/3412)

## 1. Objective and claim ceiling

This contract evaluates a complete bounded decision episode:

`evidence → detection → optional acquisition → admissible decision → feasible action → independent verification → service trajectory`

The question is not whether a model predicts an event or reduces entropy. It is whether qualified information changes a permitted decision, the selected intervention is feasible under finite resources, and the matched physical/service outcome improves without violating protected service floors.

This is a **composition adapter**. It does not implement another:
- hydraulic or water-quality solver;
- OT monitoring/security platform;
- event detector or event matcher;
- value-of-information / evidence-acquisition optimizer;
- evidence/authority ledger;
- work-order, crew or repair lifecycle;
- asset registry or source of truth.

The contract is design-only until its exact source snapshots and schemas are pinned, and every numeric example in the accompanying fixture is synthetic/modelled. No result from this pack is municipal field evidence or an operational safety claim.

## 2. Why this is not "VOI sensor placement v2"

Pressure-sensor placement using Bayesian value of information and intervention/no-intervention costs was already published for a real water district in 2022. CivBench must not claim VOI itself as a new idea: https://doi.org/10.1061/(ASCE)WR.1943-5452.0001578.

Sol-Atlas already owns the broader evidence-aware abstention and VOI research:
- [#365 — benchmark evidence-aware abstention and value of information](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/365)
- [#357 — actionable evidence acquisition obligations](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/357)

The water-specific gap is the **composition after information acquisition**: decision admissibility, crew/parts/energy/budget feasibility, response contention, independent work verification and service trajectories. Symtropy consumes the acquisition decision/value receipt from Sol-Atlas; it does not independently reimplement VOI or silently reinterpret its cost model.

The 2026 review of 86 leak-detection studies identifies operational constraints and false-alarm burden as deployment bottlenecks, while recommending hierarchical sensor-aware evaluation and explicit SCADA/GIS feasibility: https://doi.org/10.2166/hydro.2026.069. A separate 2026 review recommends cross-site validation, task-valid explanations, controlled decision loops, lifecycle sustainability and explicit cost/equity measurement: https://www.mdpi.com/2073-4441/18/19/2443.

A 2026 standardization review of 90 studies also found that the leading shared public benchmarks remain unevenly reused: only 12 studies used BattLeDIM/L-Town, seven used LeakDB, and four used the Aghashahi acoustic dataset. It identifies inconsistent scenarios, validation strategies and metrics as a barrier to fair comparison, and recommends explicit in-domain, cross-scenario, cross-network and synthetic-to-real lanes: https://doi.org/10.3390/w18182267.

**New municipal digital-twin comparator (published 24 October 2026):** Li and Gao integrate state assimilation (EnKF/weighted least-squares recalibration), residual-based CUSUM/Mahalanobis detection, GAT pipe ranking and constrained pressure/pump regulation. They report pooled F1 0.916 and Top-5 hit rate 0.816, compared with 0.836 and 0.573 for their strongest baseline; per-network F1 is 0.903 on C-Town and 0.932 on Net3. Crucially, on an independent official BattLeDIM 2019 validation set, F1 falls to 0.818 and Top-5 hit rate loses about twenty percentage points. Their C-Town closed-loop control reports 37.5% leakage reduction versus 24.6% under uniform depressurization, with 99.4% service compliance. The paper states limitations for scalability beyond a few thousand nodes, transient hydraulics and dense-instrumentation capital cost. This is the most relevant current integrated-digital-twin comparator for CivBench; reconstruct the exact network splits, controls and service constraints before comparing any metric: https://doi.org/10.1038/s41598-026-73010-y.

**Concurrent abrupt and incipient leak comparator (Water Research, 1 May 2026):** Wang uses simulated SCADA and a Graph Transformer to predict junction pressure, with residual compensation and slope-based localization for simultaneous abrupt and incipient leaks. On BattLeDIM2020 the paper reports a €422,660 ± €11,553 mean economic score and 95.6% recall. The data are available on request, and the systematic review records validation details as not reported; this is therefore a paper-reported result to reconstruct where possible, not an openly reproducible held-out baseline: https://doi.org/10.1016/j.watres.2026.125538.


### 2.1 Real operating baselines already exist in Johannesburg

CivBench must not compare itself only against a hypothetical utility with no sensing or inspection workflow.

Johannesburg Water's published Active Leak Detection programme reports:
- 15 technical teams undertaking daily active leak detection;
- more than 10,500 km of infrastructure inspected per year, described as approximately 85% of the network;
- use of acoustic listening sticks, ground microphones and correlators;
- average annual detection figures of 1,456 burst pipes, 5,227 leaking meters, 341 leaking valves and 217 leaking hydrants;
- approximately 8,100 ML/year of water saved, as reported by the utility.

Source: [Johannesburg Water — Active Leak Detection Programme](https://johannesburgwater-bygkashncch7dndx.southafricanorth-01.azurewebsites.net/about-us/campaigns/active-leak-detection-programme/).

These are utility-published aggregate operational claims. The page does not provide event-level denominators, confidence intervals, counterfactual attribution or independent outcome audit. Treat the reported 8,100 ML as a **utility-reported estimate**, not an independently verified causal effect or a current CivBench baseline value.

Johannesburg Water's July 23, 2026 media release also reports a smart-control and sensor rollout:
- smart hydraulic-flow controls being retrofitted to 28 high-consumption reservoirs/towers;
- 53 smart pressure controllers commissioned/installed as of April 2025;
- 306 additional controllers procured and being installed, with a stated September 2026 completion target;
- 225 noise loggers installed, with 125 additional loggers planned by October 2026.

Source: [Johannesburg Water — Smart Technology Rollout, July 23, 2026](https://www.johannesburgwater.co.za/blog/2026/07/23/johannesburg-water-accelerates-smart-technology-rollout-to-reduce-water-losses/).

The September/October 2026 target dates have elapsed, but this release does not verify the final completion totals. Do not infer that all 306 controllers or 125 planned loggers were installed. A current installation inventory would require a later authoritative source.

**Benchmark implication:** include a public-information baseline representing the described inspection/acoustic/correlator/depot workflow, and keep the smart-controller/noise-logger deployment profile as a separate, date-qualified lane. Do not fabricate crew dispatch rates, sensor uptime, false-alarm rate, repair completion or savings attribution from the aggregate page. For a real utility comparison, obtain authorized event-level workflow data and independently qualified service outcomes.

**Current operating-envelope update (29 October 2026):** Johannesburg Water reports severely constrained supply systems, with high demand and excessive draw-off limiting reservoir/tower recovery, resulting in low pressure, intermittent supply and prolonged outages in multiple areas. The utility says it is reconfiguring networks and applying targeted throttling to support reservoir recovery; it also proposed stronger Level 1 restrictions targeting an approximately 15% demand reduction, with safeguards for basic supply and alternative supply for essential services. These are the utility's reported conditions and proposed interventions, not calibrated hydraulic inputs or measured intervention effects: https://www.johannesburgwater.co.za/blog/2026/10/29/johannesburg-water-implements-urgent-interventions-to-stabilise-severely-constrained-systems/.

This updates the benchmark envelope: **do not evaluate only isolated leak incidents under nominal supply**. Add a distinct constrained-service scenario lane in which demand/draw-off shocks, storage recovery, targeted throttling, repair response and emergency alternative supply interact. Measure worst-group service-floor compliance, shortfall duration/volume, reservoir recovery, safe fallback supply, pressure/quality violations, response burden and recurrence. Keep essential basic-supply floors as hard constraints; a leak reduction or faster recovery for one zone cannot compensate for a protected group losing minimum service. The published release does not give the calibrated network state, reservoir telemetry or event-level outcome data needed to parameterize a real Johannesburg hydraulic simulation; the first implementation must remain a synthetic/modelled stress case until qualified inputs are obtained.

A further 2026 research comparator is the DBSCAN-Leak campus case study, which reports 16 confirmed leaks detected versus one by its original DBSCAN comparator, with 100% precision, 80% recall and F1 0.89 on Lille University Scientific Campus flow data. This is a distinct site/task and a paper-reported case study, not a municipal-network or Johannesburg result: https://doi.org/10.1109/ACCESS.2026.3664702.

A 2026 Macao campus study provides another useful system-composition reference: monitored pressure/flow, pressure-driven demand modelling, digital-twin analytics and a digital work-order system were joined for leak detection, dispatch and repair confirmation. It reports hydraulic state errors below 5% and a two-week detection-to-validation cycle for its campus deployment. Again, this is a campus-specific case study, not proof of utility-scale transfer: https://doi.org/10.2166/aqua.2026.125.

## 3. Existing owners and reference boundaries

| Component | Authoritative owner | Composition rule |
|---|---|---|
| Event detection, alarm episodes, one-to-one matching | Symtropy #3513 | Consume event receipt and its evidence ceiling; do not rematch raw samples here |
| Paired scenario, finite resources, outcome evidence statuses, decision lineage | Symtropy #3412 / #3510 | Consume validated pair, resource and outcome receipts |
| Water stock-flow reconciliation | Symtropy #3320 / #3361 | Consume independent accounting result |
| Repair workflow, crew/parts capacity and exceptions | Symtropy #1645 | Consume lifecycle/capacity receipt; do not create another work-order engine |
| Maintenance debt and renewal feedback | Symtropy #3372 | Use only when the horizon includes asset ageing/renewal |
| Evidence obligations and VOI ranking | Sol-Atlas #357 / #365 | Consume request/response/evaluation references; never duplicate acquisition policy |
| Water-power dependencies and service floors | Sol-Atlas #147 | Consume declared trajectory/floor receipts; do not create another hydraulic model |
| Held-out topology and localization | Sol-Atlas #174 | Consume qualified localization/evaluation references |
| Institutional authority, privacy, contestability and appeal | Mycelix #2006 / #5699 | Consume authority/rights/provenance references; this adapter grants no authority |

A reference is not a copy of an owner's source record. It binds the owner's stable record ID, version, evidence class and content digest. A digest demonstrates byte identity, not truth or physical validity.

### 3.1 Receipt binding and actor-independence requirements

Every referenced receipt must have an explicit owner role (event_evaluator, acquisition_planner, human_authority, resource_ledger, repair_workflow, independent_verifier, service_outcome, or paired_outcome) and bind the same decision_episode_id and paired_block_id. Receipt IDs are unique across roles within one decision episode; a workflow receipt cannot be reused as an independent-verification receipt. Each binding carries a declared SHA-256 digest of the receipt payload. This adapter checks digest syntax, non-zero value, owner role, reference identity and context equality; it does **not** fetch or authenticate payload bytes, verify issuer signatures, or prove the issuer's authority.

Every human disposition carries an owner-issued authority receipt and opaque actor reference. Work progress carries an executor principal reference. Independently verified work carries a separate verifier principal reference that must differ from the executor. Identity references are opaque synthetic/provider IDs at this layer; Mycelix and the originating authority/verification systems remain responsible for identity proof, role validity, conflict-of-interest policy and source-dependency checks.

Resource feasibility and resource settlement are separate facts. The composition validator requires reserved while work is planned/in progress, settled after a repair claim or verification, released after cancellation, rejected after a denied reservation, and not_attempted when no feasibility check was made. These are cross-record consistency checks only: the source ledger's authoritative state remains authoritative.

## 4. Composition record

One `decision_episode_id` is bound to exactly one paired scenario block and the exact time-bounded evidence snapshot visible at the decision point.

The composition record contains references to:

1. **Scenario pair:** block ID; shared initial-state, topology, exogenous-schedule, model-family and time-grid identities; declared treatment factor; resource-envelope identity.
2. **Event result:** event-evaluator receipt ID and evaluator version; event IDs/episode IDs; event-level metric receipt and class-confusion result.
3. **Acquisition result (optional):** Sol-Atlas request/obligation ID, permitted source declaration, acquisition-result ID, realized observation snapshot and VOI/cost/latency result reference.
4. **Admissibility:** gate result and explicit reason codes; candidate action set; unknowns; operating/service/quality constraints; profile and authority references.
5. **Human disposition:** authorize, reject or defer, with actor/role reference, time and reason. Acquisition does not authorize work.
6. **Resource lifecycle:** reservation/settlement/release receipt references for crew skill-minutes, parts, contractor-minutes, operating/capital budget, energy, water and sensor units as applicable.
7. **Work lifecycle:** inspection, dispatch, repair claim, cancellation, retry/reopen and exception references from the existing workflow owner.
8. **Independent verification:** separate verifier identity/role and source-dependency declaration; verification evidence and disposition.
9. **Service trajectory:** physical/service outcome receipt, per-zone/group service floors, water-energy dependencies, time horizon and evidence status.
10. **Discrepancy review:** post-event model residual, unsupported assumptions, remaining uncertainty and explicit next-review outcome.

No required link can be satisfied by a model-generated inference from an adjacent stage. Missing, stale, withheld, unsupported or cross-status evidence remains explicit.

## 5. Non-equivalences that the adapter must preserve

```text
EventDetection != CorrectDiagnosis
CorrectDiagnosis != AdmissibleRecommendation
AdmissibleRecommendation != HumanAuthorization
HumanAuthorization != FeasibleResourceReservation
ResourceReservation != CompletedWork
RepairClaim != IndependentVerification
IndependentVerification != MeasuredServiceOutcome
MeasuredServiceOutcome != CausalTreatmentEffect
EntropyReduction != DecisionValue
DecisionValueEstimate != RealizedServiceBenefit
SyntheticReceipt != TestbedEvidence
TestbedEvidence != UtilityObservation
```

Rejected or deferred recommendations terminate without work. A resource reservation failure cannot become a dispatch. A repair claim without an independent verifier cannot become verified repair. A modelled service delta cannot be promoted to observed.

## 6. Resource-feasibility contract

The composition layer reads the existing finite-resource ledger and workflow outputs. It may aggregate them for reporting, but must not reserve, settle or mint resources itself.

At minimum, preserve these resource dimensions where applicable:
- crew/contractor minutes by skill or declared task category;
- parts by item/unit;
- capital and operating budgets in declared minor currency units and price year;
- energy in Wh;
- water in litres;
- installed/available sensor units;
- access/work-window and process constraints as explicit non-fungible feasibility conditions.

Every action attempt resolves to one of:
- `feasible_and_reserved`;
- `rejected_no_capacity`;
- `rejected_missing_part`;
- `rejected_budget`;
- `rejected_energy_or_access`;
- `not_authorized`;
- `deferred`;
- `cancelled_and_released`.

These are explanatory composition labels, not a new authoritative work-order status enum. The source workflow and ledger retain their own event state. No aggregate "resource units" may add unlike quantities.

## 7. Service outcome and comparison rules

Use the existing evidence-qualified `OutcomeVector` and `PairedOutcomeReceipt` from the experiment core. Do not define a competing outcome vector or combine its fields into one score.

Preserve, where supported:
- essential water shortfall (L);
- service interruption exposure (service-unit seconds);
- worst-group service performance (ppm);
- physical, commercial and unbilled-authorised loss components separately;
- verified repair count;
- whole-life cost;
- recovery time;
- safe fallback delivery;
- false dispatches.

A numeric paired delta is admitted only when both control and candidate values have exactly matching evidence classes and valid units/time windows. Observed-vs-modelled and other mismatched pairs remain explicit non-comparable results unless a separate versioned/pre-registered bridge exists. A missing or unsupported value is not zero.

Service floors are hard constraints, stratified by declared zone/group and time interval. A candidate is not robustly better when any protected group can be worse under the declared robustness rule, even when the network-wide average improves. Quality and safety constraints cannot be traded away for lower cost or higher average availability.

## 8. Required comparisons

Use the same scenario, event schedule, initial state, topology, time grid, model family, evaluation horizon, action set and resource envelope, except for explicitly registered treatment dimensions.

Compare, as data/owners permit:

1. Existing-information / no-new-acquisition baseline.
2. Fixed threshold or fixed-checklist measurement and response.
3. Risk-prioritised sensing/inspection.
4. Entropy/information-gain acquisition (diagnostic comparator, not assumed decision-optimal).
5. Sol-Atlas #365 expected-VOI acquisition policy.
6. Hierarchical cascade: network risk → DMA/zone warning → in-situ confirmation, with abstention when confidence or evidence is inadequate.
7. Resource-aware dispatch/repair policy using the existing #1645 lifecycle and #3412 resource ledger.
8. Bounded small-instance oracle over the same legal acquisition/action/resource set. It is an upper bound, not a deployable industry competitor.

No algorithm may receive hidden evaluator truth, future observations, post-decision fields, or resource availability unavailable at the declared decision time.

## 9. Primary evaluation lanes

### A. Detection and diagnosis
Use #3513 event-onset recall/precision, per-class confusion, false-alarm episodes per monitored system-day, p50/p95 delay and early lead, fragmentation and alarm duty cycle.

### B. Acquisition and decision change
Consume #365 references for request value/cost/latency. Report:
- whether acquisition returned valid, permitted, fresh and independent evidence;
- whether the evidence changed admissibility, action choice or uncertainty status;
- helpful, harmful and neutral decision changes separately;
- acquisition cost, time, failure and source-dependence;
- entropy/information gain separately from decision value;
- time-to-resolution, request count and abstention-resolution coverage.

### C. Dispatch and resource contention
Report feasible/infeasible recommendations, reservations/settlements, queue time, false dispatches, delay to true incidents due to capacity contention, cancellation/release, parts shortage and overtime/contractor use where evidenced.

### D. Service outcomes
Report per-group deficit area/duration, floor compliance, recovery time, recurrence, fallback delivery, quality excursions, water balance and worst-group service. Keep modelled/testbed/observed lanes separate.

### E. Assurance and lifecycle burden
Report source freshness/independence failures, invalid chain references, verifier independence, operator workload, sensor lifecycle/maintenance cost, compute, bandwidth and p50/p95/p99 decision-planning latency on pinned hardware.

No universal composite score. Report the complete vector and Pareto trade-offs; do not conceal a service-floor regression behind a weighted average.

## 10. Frozen synthetic controls in the fixture pack

The companion fixture defines five matched cases:

- **Positive service restoration:** additional evidence changes an admissible action; capacity is available; human authorizes; the existing workflow and ledger record repair; a different verifier verifies it; the modelled service trajectory improves.
- **No-value sensor:** the extra sensor reduces uncertainty but cannot change the safe action because crew capacity/access is binding. Decision value after acquisition cost is non-positive; no sensor-count/entropy victory is reported as a service benefit.
- **False-alarm capacity externality:** false alarms consume the only crew slot and delay a true leak response; the candidate can have a better detector-side metric and a worse service result.
- **Resource exhaustion:** a recommendation cannot be executed because a required part/crew/budget reservation is unavailable. No ghost dispatch, double reservation or completion is permitted.
- **Unverified repair / unknown truth:** a repair claim lacks independent verification and an alarm overlaps unresolved truth. Neither becomes a verified repair, a false positive, or a service effect by forced imputation.

Every pair holds the exogenous schedule and initial conditions fixed. Numeric examples are modelled/synthetic only. They are not Johannesburg utility observations or operational cost estimates.

## 11. Aspirational research targets (not achieved SOTA)

After reconstructing exact baselines and registering task definitions:

- Zero false authorizations, double spends, impossible dispatches and false verified-repair claims across the frozen negative-control corpus.
- At matched lifecycle cost and identical event schedules, seek either +5 percentage points timely event recall without worse p95 delay/worst-group floors, or 20% fewer avoidable dispatches at matched missed-event rate.
- At matched resource envelope, seek at least 10% lower service-deficit area on the registered synthetic/testbed task family without a worse protected floor.
- Improvements must persist on held-out scenario families and be reported separately for each dataset and evidence class.
- All small-state resource and outcome vectors must reconcile to hand calculations; replay must be deterministic and receipts byte-identical.

These are internal targets, not literature-derived universal limits. Utility-specific budgets, alarm limits, floors, action windows and costs require operator agreement before any prospective/shadow study.

## 12. Safety and claim boundaries

- Offline replay and isolated, purpose-built testbeds only. No production PLC/SCADA, utility credentials, public OT endpoints or operational network interaction.
- Acquisition requests are advisory and non-executing. Only the existing authority plane can authorize an action; human review, independent interlocks and rollback remain mandatory.
- No resident personal data or sensitive infrastructure coordinates in public fixtures.
- Synthetic, modelled, testbed, historical-SCADA and utility-observed claims remain separate.
- A digest proves byte identity, not source truth. A model output is not an observation. Independent verification is not by itself proof of causal service improvement.
- Any future shadow/pilot needs written data authority, named utility/operator, prospective baseline, pre-registration, privacy/security/safety review, stop/rollback plan and independent outcome review.

## 13. Exit criteria for the first adapter

1. Strict, versioned composition schema references existing owner records without redefining them.
2. The five synthetic controls validate and produce the expected explicit status/result.
3. The adapter rejects invalid, missing, out-of-order, cross-scenario and evidence-status-mismatched links.
4. Candidate-visible projections exclude evaluator-only truth.
5. Resource feasibility is read from the existing ledger/workflow; the adapter cannot mint or mutate authoritative resource state.
6. Independent evaluator recomputes the composite report without importing candidate-policy internals.
7. Same frozen inputs/toolchain produce byte-identical receipts.
8. Exact-head formatting, locked compilation, tests, strict Clippy and docs pass before a qualification claim.
9. The report preserves complete metrics and evidence claim ceilings; no pooled synthetic-to-field claim.

## 14. First implementation slice

Implement only:
- a strict parser/semantic adapter for the composition manifest;
- cross-reference validation across event receipt, paired experiment, acquisition, resource ledger, decision chain, verification and service outcome;
- deterministic positive/negative fixture execution;
- one composite receipt that embeds source references and per-domain statuses.

Do not modify EPANET, the hydraulic solver, the event-matching algorithm, Sol-Atlas acquisition optimizer, Mycelix authority or the existing work-order lifecycle in this slice.
