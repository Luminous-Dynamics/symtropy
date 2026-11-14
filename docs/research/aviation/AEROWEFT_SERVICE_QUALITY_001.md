# AeroWeft — Endogenous Demand and Service-Quality Feedback

**Tracking issue:** [AEROWEFT-SERVICE-QUALITY-001 #5248](https://github.com/Luminous-Dynamics/symtropy/issues/5248)  
**Consumers:** Symtropy #1601/#5181/#5186/#5207/#5224; Sol-Atlas #189/#825; Symthaea #6249  
**Status:** synthetic-first research specification  
**Claim ceiling:** finite choice/queue feedback measurement only; no passenger-demand forecast, real UAM adoption estimate, fare policy, operating approval or aviation authority.

## 1. Research thesis

Demand is not always an exogenous number. When a service changes its waiting time, reliability, price, access and transfer costs, users' mode choices may change. That changed demand then alters queues, resource utilization, waiting time and the service users experience.

AeroWeft should measure this feedback loop rather than optimize a fixed passenger matrix and assume the service-quality outcome follows.

The research question is:

> When service capacity changes the generalized door-to-door cost of aerial travel, what stable demand/service outcomes emerge, who benefits, and does the resulting system improve public value rather than only operator throughput?

The benchmark explicitly separates potential demand, demand choosing each mode, demand actually served, queueing and service performance, operator private benefit, user generalized cost, public/network externalities, uncertainty and model applicability.

Increased adoption is not automatically a public benefit. A higher operator profit is not a social optimum. A demand-choice model is not empirical truth.

## 2. 2026 research frontier and required comparisons

The field already has substantive integrated models. AeroWeft must respect them:

1. **Lyu & Feng (July 2026), Integrated planning of vertiports and vertipads infrastructure for Urban Air Mobility, Transportation Research Part A 209, 105027.** It jointly selects vertiport locations and pad capacities, uses multinomial-logit passenger demand, multi-server queueing for waiting time, and Lagrangian relaxation plus Benders decomposition. Its Shanghai case reports strong influence from service quality and ground-access disutility, with diminishing returns above roughly two to three vertipads per site in that case. Data are available on request; do not assume raw data or full implementation are openly reusable. DOI: https://doi.org/10.1016/j.tra.2026.105027
2. **Qin et al. (November 2026), Vertiport capacity planning and operations for urban air mobility commuting, Transportation Research Part E 215, 105103.** It develops a bilevel capacity/commuter-behaviour model, identifies market underinvestment where wider travel benefits are not internalized, and reports that poorly designed policies can backfire. DOI: https://doi.org/10.1016/j.tre.2026.105103
3. **Gottschalg et al. (May 2026), Vertiport location planning for urban air mobility airport shuttle services under Demand uncertainty, Transportation Research Part A 207, 104957.** It uses risk-averse two-stage stochastic site/fleet planning: strategic investment first, operations under realized demand second. Its Madrid airport case reports a reduction in the probability of financial loss from 34% to below 10% relative to the deterministic benchmark described in the paper, and discusses reuse of existing heliports. DOI: https://doi.org/10.1016/j.tra.2026.104957
4. **Dubai integrated UAM optimization (May 2026).** It jointly optimizes vertiport locations, fleet size, routes, multi-trip schedules and partial charging, with speed-dependent battery discharge; the paper reports a 4–30% improvement against the heuristics it studies. DOI: https://doi.org/10.1016/j.trip.2026.101945
5. **Spatial vertiport planning in Salt Lake City (2026).** A parcel-level study integrates nearly 60 datasets across built environment, natural environment, regulatory and value dimensions. It finds fewer than 1% of examined parcels rank highly suitable. DOI: https://doi.org/10.1016/j.cities.2026.107255

These are not all directly matched baselines. Separate a literature-level feature comparison, a transparent local implementation of a published model class, and a faithful numerical reproduction with the exact data, model, coefficients, solver, constraints and compute budget pinned.

Only the last supports a direct numerical comparison to a published score.

## 3. New research gap to test

The new contribution is not a second MNL estimator, queueing library, Benders solver, capacity optimizer or bilevel planner.

It is the aviation-specific, replayable bridge between:
1. user-choice outcomes and time-dependent generalized cost;
2. exact resource-event waits from candidate schedules;
3. aircraft/airspace capacity and recovery constraints;
4. feedback from service quality to mode/route demand;
5. per-cohort affordability/access outcomes and broader public value.

The scheduler and generic multimodal optimizer keep their current ownership. This work defines the demand-to-service benchmark and its measurement outputs.

## 4. Ownership and adapters

| Responsibility | Owner | This work consumes |
|---|---|---|
| Multimodal demand/service cost and investment optimization | Symtropy #1601 | Choice/service-cost outcomes and public-value dimensions |
| Aircraft/airspace/vertiport candidate schedules | Symtropy #5186 | Exact schedule and resource-event receipts |
| Reachable occupancy and conflict constraints | Symtropy #5163/#2279/#2297 | Feasibility/admissibility outcomes; no new route solver |
| Medical cargo mission chain | Symtropy #5181 | Separate cargo service class; do not treat cargo demand as passenger choice |
| Recovery infrastructure | Symtropy #5224 | Site/energy/access outage constraints |
| Reciprocal aircraft recoverability | Symtropy #5207 | Aircraft option-loss and joint recoverability outcomes |
| Aircraft performance/applicability | Symthaea #6249 | Profile applicability only |
| Spatial/access context and rights | Sol-Atlas #189/#825 | Admitted ground access/site evidence |
| Identity/reservation provenance | Mycelix #5023 | Exact provenance receipts, not demand truth |

The first version may use a tiny local deterministic event-trace checker as an independent oracle. It must not become a generic production queue service or scheduler. Production queue/service timings should be derived from the same admitted event trace as #5186, not guessed separately.

## 5. Semantic contract

### 5.1 Potential demand and anonymous cohorts

Represent synthetic or admitted aggregate cohorts, not identifiable people. Each cohort binds origin/destination and time window, potential trip count and expansion weight, mode alternatives and outside option, declared utility/choice table or versioned choice-model identity, value-of-time and affordability boundaries, accessibility/service-floor membership where supported, and evidence/sample-frame/revision/applicability.

The tiny exact oracle uses a frozen finite choice table with deterministic tie semantics. MNL, stated-preference and revealed-preference adapters are separate research comparator classes. Do not invent real preference coefficients or treat cohort labels as sensitive personal attributes.

### 5.2 Generalized door-to-door cost

A qualified choice model may consume first/last-mile access and egress, in-vehicle ground and flight time, expected and tail waiting time, transfer/handoff time, fares and other user charges under a declared value-of-time model, reliability/cancellation/missed-connection penalties, and accessibility requirements when the corresponding service property is evidenced.

Keep fare/revenue, private user generalized cost and real social resource cost separate. Do not double-count fare as both a user payment and a net societal resource cost. Affordability floors remain explicit and may not be averaged away.

### 5.3 Resource-event queue semantics

The event trace distinguishes landing/take-off pad service, passenger boarding/alighting and processing, stand/gate/turnaround, energy/charging service, transfer/ground access, and any other explicitly modelled server.

Capacity units, compatibility, service duration, service discipline, time basis and validity are explicit. A pad is not automatically a charger; an aircraft stand is not automatically a passenger terminal.

For exact tiny fixtures, queue wait is calculated from a finite event list using deterministic service duration and a declared discipline (initially FCFS). Larger comparators may use multi-server queueing formulas or event simulation, but must report their assumptions and compare on matched arrival/service traces.

### 5.4 Demand-service feedback

For demand vector D, service-cost vector C, and a declared mode-choice operator Choice, the bounded update is:

    D_(k+1) = Choice(C(ServiceTrace(D_k)))

The service trace is generated by the exact candidate schedule/resource manifest. The evaluator records the initial demand vector, every iterate, exact service-trace digest, chosen alternative by cohort, queue outcomes, convergence residual/fixed point, any repeated state/cycle, and budget exhaustion.

Do not silently stop at a convenient iterate. If multiple fixed points exist, report them. If an iteration cycles, report the cycle and initialization dependence. Damping or a different equilibrium solver is a separately declared policy, not a hidden repair.

The tiny reference fixture deliberately includes a fixed point and a two-cycle under one-server capacity. A two-server variant has a different stable outcome. These are counterexamples for algorithm correctness, not realistic demand forecasts.

### 5.5 Public value versus operator value

Keep separate operator investment/revenue/operating cost/private net benefit, user generalized cost and affordability, public/network externalities only when declared and supported by the synthetic or admitted model, whole-life cost, access distribution and service-floor outcomes.

Include cases where positive external benefits create private underinvestment and counterexamples where a subsidy, fare cap or capacity mandate creates excess cost or worsens access. No policy is presumed good because one paper found underinvestment under its own assumptions.

## 6. Tiny synthetic feedback fixture

The machine-readable fixture is fixtures/aeroweift-service-quality/tiny-demand-queue-v0.json.

It contains:
- three anonymous one-trip cohorts with different maximum acceptable UAM waits;
- a ground alternative and UAM alternative;
- one deterministic 5-minute per-pad service duration;
- one-server and two-server pad configurations;
- a fixed finite demand choice table and an exact mean-queue-wait table;
- a demand update rule and expected fixed-point/cycle results;
- an exogenous-demand comparator;
- no real aircraft, fares, travel survey or operational site.

The one-server case has:
- fixed point at UAM demand 2;
- a separate 1 ↔ 3 update cycle under the declared simultaneous cohort update;
- explicit initialization dependence.

The two-server case has:
- stable UAM demand 3;
- mean queue wait of 5/3 minutes at that fixed point.

The result demonstrates that a naïve best-response iteration can cycle even in a tiny model. Do not select an equilibrium without stating the selection/initialization rule.

## 7. Matched baselines

Use identical potential demand, cohort alternatives, candidate sites, time horizon, schedules, aircraft applicability, airspace restrictions, prices, resource capacities and compute budgets:

1. Exogenous fixed passenger demand with FIFO/first-fit resources.
2. Deterministic finite choice without queue feedback.
3. MNL choice with multi-server waiting-time estimates, matching the Lyu & Feng 2026 model class.
4. Risk-averse two-stage demand scenario and recourse, matching Gottschalg et al. 2026 at the model-class level.
5. Bilevel capacity/commuter-choice planning, matching Qin et al. 2026 at the model-class level.
6. Existing #1601 optimizer with fixed demand.
7. Existing #1601 optimizer supplied with the AeroWeft service-quality/demand feedback outputs.
8. Faithful reproduction of published work only after exact data, parameter, algorithm, solver and rights audit.

A local model-class implementation is not a faithful reproduction. Do not copy reported profit, adoption or loss-probability improvements into our targets.

## 8. Metrics

Report separately.

**Choice and demand:** mode share and outside-option share by cohort; potential, chosen, served, switched, deferred, denied and unresolved trips; sensitivity to fare, access, waiting and reliability; calibration, coefficient uncertainty and population/sample-frame limitations.

**Queue and operation:** p50/p90/p95/p99 and maximum wait; queue length, abandonment, missed connection and unused capacity; pad/stand/charger utilization; delay, cancellations, schedule churn; resource bottleneck and spillback attribution.

**Equity and public value:** door-to-door generalized cost by cohort; affordability/access service floors; worst-group and max-min service; operator net benefit versus user benefit versus public/network externality; origin/destination coverage and outside-option rate.

**Equilibrium and computation:** convergence/fixed-point rate, iterate count and residual; cycle detection, all fixed points found and initialization dependence; p50/p95/p99 runtime, memory and budget exhaustion; deterministic replay and exact source/model/input manifests.

No aggregate score may hide a breached affordability/access floor or a cohort with zero usable service.

## 9. Falsification and metamorphic controls

- Fixed exogenous demand overstates served aerial demand when a cohort's wait threshold is exceeded.
- Adding a pad server cannot increase deterministic mean wait for an otherwise identical arrival/service trace.
- Increasing a cohort's access/fare/wait cost cannot improve its utility under a declared monotone choice rule.
- A finite update cycle is reported as a cycle and not as convergence.
- A fixed point is independently verified by applying the update operator and obtaining the same demand vector.
- Multiple fixed points are retained, not collapsed.
- Every potential unit is accounted for as served, switched, deferred, denied, outside-option or unresolved.
- Unsupported preference parameters are not called calibrated.
- A queue estimate cannot soften a hard #5186 resource/airspace constraint.
- A higher operator profit cannot compensate for a breached public service floor.
- Input-order and ID permutation preserve semantic outcomes.
- Exact replay binds schedule, event trace, choice table, queue rule, cohort manifest and toolchain.

## 10. Acceptance targets

- 100% exact agreement with an independently written finite choice/event-queue oracle on the frozen tiny corpus.
- Deterministic tie-breaking and input/ID permutation invariance.
- Complete demand accounting with zero silently dropped cohorts.
- Zero unsupported choice parameters promoted to calibrated evidence.
- Every fixed point is an exact fixed point under the declared operator.
- Every repeated state is reported as a cycle.
- Adding a pad server does not increase waiting time under a fixed trace.
- All cohort affordability/access floors reported separately.
- No scheduler hard gate is softened because demand falls.
- Deterministic output from pinned input manifests.

These are finite synthetic targets, not real passenger forecast accuracy or market-adoption claims.

## 11. Execution order

1. Freeze a small deterministic finite-choice + event-queue golden fixture.
2. Implement an independent exact oracle for choice, queue outcomes and the expected feedback fixed point/cycle.
3. Define the adapter from #5186's exact candidate schedule/resource event receipts.
4. Evaluate finite-choice, MNL/queue, two-stage risk-averse and bilevel model classes with matched estimands.
5. Feed qualified service-cost/demand outcomes into #1601; keep investment/optimization ownership there.
6. Admit stated/revealed preference data only after rights, consent, sampling frame, representativeness and whole-group split review.

**Core invariant:** a demand model is not immutable truth, and increased UAM adoption is not by itself evidence of improved public value. Service quality changes demand, but the resulting access, affordability, reliability and externality distribution must be measured explicitly.
