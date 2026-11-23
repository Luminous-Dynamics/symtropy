# Manufacturing Route Recourse Benchmark v0.1

**Owner:** Symtropy issue [#7053](https://github.com/Luminous-Dynamics/symtropy/issues/7053)  
**Implementation:** draft [PR #7083](https://github.com/Luminous-Dynamics/symtropy/pull/7083)  
**Evidence class:** synthetic, offline  
**Claim ceiling:** route recommendations only; no reservations, dispatch, machine authority, physical execution, product acceptance or supplier qualification.

## 1. Problem and boundary

The primary-route benchmark [MFG-ROUTE-ROBUST-001 #7019 / PR #7021](https://github.com/Luminous-Dynamics/symtropy/pull/7021) evaluates one route choice at a decision cut-off. It does not describe what happens when that route becomes unavailable, calibration turns stale, an execution outcome is unknown, or several jobs must share alternative capacity.

This tranche evaluates the semantics of **non-anticipative recommendation and recovery**:

> At every decision time, can a recommendation use only currently observed evidence, respect current admissibility and shared capacity, freeze when execution is unresolved, and produce the same decision for histories with the same visible information?

This is deliberately not another scheduler, provider registry, resource-offer ledger, execution journal, quality authority, product acceptance service or machine controller.

It composes:
- Symtropy #6790 for route admissibility and hard blockers.
- Symtropy #7019/#7021 for primary route-policy baselines and scenario-family comparison.
- Symtropy #5758 and Mycelix execution owners for intended/recorded/observed execution reconciliation.
- Mycelix #6541 for resource-offer declarations, not capacity reservation.
- Mycelix #5762 and Symthaea #5774 for common-cause dependency closure.
- Symthaea #7677 for production scheduling.
- Mycelix #3084 for accepted manufactured-unit evidence.
- Symthaea #7706 for governed machine interaction.

## 2. Formal decision information

A decision is evaluated over:
- a frozen route catalogue (IDs, owner disposition, blockers, common-cause domains, declared capacity, unit cost and lead time);
- a frozen set of jobs (quantity, deadline and cost ceiling);
- a decision-hour cut-off;
- an ordered list of observations available by that cut-off.

Evaluator-only future events are stored separately in the fixture. The policy and same-information oracle do not receive them.

A canonical information key is constructed from the decision hour, route catalogue, jobs and observed events. It excludes case identity and hidden future events. All recommendations sharing this key must be identical.

### Observation semantics

| Observation | Effect |
|---|---|
| Route failed | Block only the identified route |
| Common cause failed | Block every route explicitly declaring that failed domain |
| Capacity reduced | Replace the route's usable capacity with the observed lower value |
| Route evidence unknown | Exclude route; do not assume feasible |
| Calibration stale | Exclude route until owner-native resolution |
| Material incompatible | Block route for the relevant job |
| Execution outcome unknown | Freeze all new recommendations |
| Execution reconciled failure | Clear pending execution and block its failed route |
| Execution reconciled accepted | Clear pending execution; acceptance remains an owner-native claim |

An unresolved execution outcome is not success and not failure. The policy cannot recommend duplicate or alternate production until the authoritative execution owner reconciles the outcome.

### Shared capacity

A single recommendation containing multiple job allocations is validated as one batch. Allocated quantities are summed by route and compared with the currently observed available capacity. Capacity cannot be reused for two jobs inside one plan.

This is an allocation-plan invariant only. The benchmark does not reserve capacity across separate plans or create commitments. A real cross-job reservation system remains owned by Mycelix/other execution owners.

## 3. Frozen synthetic corpus

The fixture contains:
- 6 scenario families;
- 20 seeds per family;
- 120 scenario cases;
- 5 decision points per case;
- 600 decision states in total.

| Family | Injected observed condition |
|---|---|
| Route-specific failure | One route becomes unavailable |
| Common-cause failure | Two nominal routes share a failed dependency domain |
| Capacity contraction | Route capacities are reduced and jobs compete for remaining capacity |
| Calibration uncertainty | A previously preferred route becomes unqualified/unknown |
| Material compatibility | One route is blocked for the required job material |
| Demand surge / no capacity | All declared alternative capacities collapse to zero |

Every case includes two initial decision points with identical route/job/observation prefixes but different hidden future events. These paired points test that a policy cannot read the future.

Every case also includes:
- a post-observation decision;
- an unresolved execution decision;
- a later decision after explicit failure reconciliation.

All routes, dependencies, costs, quantities, lead times and event outcomes are synthetic challenge data. No supplier identities or failure probabilities are represented as real.

## 4. Policies and same-information oracle

### Candidate policy

The first policy is intentionally simple and auditable:
- sort jobs by stable job ID;
- for each job, consider only already-admissible routes not blocked by observations;
- respect deadline, cost ceiling and remaining capacity;
- choose lowest unit-cost route, then lead time, then route ID;
- abstain when none remains;
- freeze globally while any execution outcome remains unresolved.

It is a transparent baseline, not the intended final Luminous optimizer.

### Exhaustive per-decision assignment oracle

For each visible decision state, enumerate all feasible job-to-route assignments plus abstention. Minimize:

`sum(route unit cost × assigned quantity) + sum(unserved penalty × unserved quantity)`

The current synthetic unserved penalty is 1,000 challenge units per job unit. It is a fixture parameter, not a monetary amount or a validated policy weight.

The oracle receives the same observed state as the candidate policy. It is exhaustive over the current decision's job-to-route assignment, not a multi-stage optimal policy over a complete scenario tree. A clairvoyant policy oracle is deliberately not used.

The next tranche should extend the exact oracle over small multi-stage scenario trees, retaining non-anticipativity at every shared information set.

## 5. Adversarial and known-answer controls

The Rust evaluator enforces:
- same visible information ⇒ same action;
- different hidden futures cannot change a current recommendation;
- all routes sharing an observed failed common-cause domain are blocked;
- unresolved execution ⇒ global freeze;
- reconciliation without a preceding unknown outcome is rejected;
- a reconciled failed route remains blocked;
- route capacity cannot be increased by a capacity-reduction observation;
- each job is assigned or abstained exactly once;
- per-route assignments cannot exceed observed capacity;
- no route with unknown, failed, incompatible or stale evidence is assigned;
- strict Draft 2020-12 validation for corpus and known-answer fixtures;
- deterministic known-answer agreement.

The separate cross-language known-answer fixture covers all 600 decision states and both the candidate recommendation and exhaustive assignment oracle.

## 6. Initial fixture results

A separate JavaScript recomputation of the frozen known-answer fixture gives the following descriptive totals for the simple greedy policy:

| Measure | Synthetic result |
|---|---:|
| Decision states | 600 |
| Freeze decisions | 120 |
| Plan decisions | 480 |
| Assignments | 900 |
| Abstentions | 540 |
| Candidate objective total | 1,094,626 |
| Exhaustive per-decision oracle objective total | 581,126 |
| Candidate-minus-oracle regret | 513,500 |

The greedy policy had positive objective regret on 260 of the 480 plan decisions. The remainder tied the per-state oracle.

These values illustrate that:
- a cheap-first greedy allocation can consume a route needed by a constrained job;
- common-cause events invalidate more than one nominal route;
- capacity contraction creates legitimate abstention;
- unknown execution blocks mutation until reconciliation;
- an exhaustive same-information assignment oracle can expose avoidable allocation regret.

These are properties of this deliberately constructed synthetic fixture, not evidence of actual supplier reliability, economic loss, recovery speed, manufacturing yield or physical service performance.

## 7. Standards and industry comparison

Kim and Jung (2026), *On-demand supply chain configuration and planning for shared additive manufacturing*, integrates provider selection, part-to-job grouping, job-to-machine assignment and daily scheduling in a shared-AM platform. Its MILP and GA results provide an important matched optimization baseline. The published model assumes deterministic provider operating hours/schedules, fixed machine capacity without breakdown during the planning horizon, and deterministic known transport time/cost. It is an order-level planning contribution, not a benchmark for unknown-outcome reconciliation or non-anticipative recovery.

Source: https://doi.org/10.1080/00207543.2026.2728013

Current standards provide useful boundaries:
- ISO/ASTM 52951:2026 specifies AM-part data-package methods and a digital-thread reference from design to acceptance. Its detailed basis is PBF-LB/M; generalized workflow requirements can inform other AM processes, but MEX-specific application still needs process-appropriate evidence. https://www.iso.org/standard/76828.html
- ISO/ASTM 52966:2026 establishes a resource/personnel capability-level framework for temporary self-sufficient AM systems. It does not certify household printers or this route-policy software. https://www.iso.org/standard/89300.html
- ISO/ASTM 52902:2023 and 52924:2023 remain relevant geometric-artifact and polymer-part-property references.

The proposed Luminous research extension is to combine:
1. an integrated provider/job/machine planning baseline;
2. time-ordered observations and non-anticipative route recourse;
3. owner-native execution reconciliation;
4. common-cause capability dependencies;
5. product-specific quality/interchangeability disposition;
6. independently accepted physical output.

The new work must be evaluated against matched MILP/GA or exact small-instance baselines before making any SOTA superiority claim.

## 8. Proposed benchmark targets

These are project hypotheses, not achieved performance claims:
- 100% same-prefix action invariance in frozen tests;
- zero assignments to routes invalidated by observed blockers;
- zero unresolved-outcome mutations;
- zero capacity overcommitments;
- exact assignment-oracle agreement on tiny exhaustive fixtures;
- deterministic replay for identical evidence streams and policy versions;
- explicit abstention when all alternatives are blocked;
- per-family and per-seed reporting without pooling different scenario families as IID observations.

Later physical benchmarks must report request-to-accepted-output P50/P90, failed attempts, rework, inspection, full burdened cost, and time to restored accepted service. A synthetic task completion is never a manufactured-unit acceptance.

## 9. Current verification boundary

The executable Rust crate uses strict Serde types plus a pinned Draft 2020-12 JSON Schema validator configured offline. The focused workflow performs formatting, schema validation, Rust tests, exact known-answer checks, adversarial mutations, report validation, SHA-256 generation and strict Clippy.

The first exact-head workflow passed schema validation, Rust tests, evaluator execution, negative controls and hashing, but failed strict Clippy on one `too_many_arguments` finding in the exhaustive recursion. That was refactored into an explicit `OracleSearch` state object without suppressing the lint. The next exact-head workflow is the evidence source for the corrected code.

No local cargo/rustc verification is claimed from the working environment. A focused workflow pass validates the frozen synthetic software contract only; it does not establish operational or physical performance.

## 10. Claim ceiling and next research step

This tranche does not implement time-expanded reservations, a booking/commitment ledger, transport routing, multi-stage scenario-tree optimization, execution dispatch, real supplier data, machine control, quality acceptance or physical recovery. It does not certify safety or product conformity.

The next research increment should extend the exhaustive oracle from per-decision allocation to small scenario-tree policy optimization, with explicit observation partitions and non-anticipativity constraints at every tree stage. Only after that should an offline adapter consume owner-native execution and resource evidence. Physical trials require a released low-risk product, qualified cell, independent measurement and authorized production owners.
