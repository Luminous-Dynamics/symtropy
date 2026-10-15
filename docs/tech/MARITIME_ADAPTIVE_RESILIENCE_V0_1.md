# Maritime Adaptive Resilience Contract v0.1

**Status:** research-backed design contract and synthetic example fixture. Not an engine implementation, calibrated model, operational procedure, safety case, or live integration.  
**Stack:** follows the frozen scenario/evidence contract in [PR #1654](https://github.com/Luminous-Dynamics/symtropy/pull/1654); must not bypass its forecast-applicability or exact-byte receipt gates.  
**Fixture:** [MARITIME_ADAPTIVE_RESILIENCE_V0_1.json](./MARITIME_ADAPTIVE_RESILIENCE_V0_1.json). All values are deliberately invented and all authority flags remain false.

## 1. The next paradigm shift

A static plan-by-scenario matrix is a necessary first step, but it is not yet a model of resilience. Resilience is temporal, networked and adaptive: the system must distinguish absorbing a disruption from recovering, observe when losses spread beyond the initiating node, and explain which new evidence could change the next decision.

This increment defines a contract for five shifts:

1. **Static plan → conditional policy.** A policy maps observed evidence/dependency states to bounded *simulation-only* responses. It can request evidence, trigger reassessment, or keep an alternative unconfirmed. It cannot dispatch a vessel, reserve a berth, authorize navigation or assert that a real-world action occurred.
2. **Snapshot → recovery trajectory.** Report service delivered and unmet demand over time, onset and duration of degradation, recovery criteria and whether recovery was observed before the horizon ends. A single end-state hides whether losses were short and severe or persistent and spreading.
3. **Single asset → explicit dependency graph.** Model ports and terminals in relation to energy, marine access, information exchange and land-side logistics. A graph edge expresses a declared dependency, not a validated causal law. Cascading effects are quantified only when a versioned, evidenced propagation model supports them.
4. **Opaque resilience score → metric vector plus non-compensable constraints.** Preserve separate service classes, hard constraints, service loss, time-to-recovery and spillover. An increase in one service cannot automatically compensate for a breached hard constraint or a collapse in another service class.
5. **Numerical value-of-information guess → decision-relevance test.** First ask whether resolving an evidence gap could change the feasible candidate set or the policy branch. Do not compute expected value of sample information (EVSI) without defensible probabilities, consequence functions and a declared decision model.

The result should not merely tell an operator what one model predicts. It should show **how a conclusion changes when evidence changes, what remains unknown, and how much of the recovery narrative is actually supported by the model.**

## 2. Research and standards implications

### Maritime resilience is a trajectory and network problem

UNCTAD's port resilience guidance separates preparation, response and recovery, and treats resilience as the ability to sustain service, recover, and adapt to changing conditions. The new contract operationalizes that framing as explicit scenario families, observed service trajectories and an out-of-horizon state rather than a forced “recovered” label.  
Sources: [UNCTAD Port Resilience Guidebook](https://unctad.org/publication/building-capacity-manage-risks-and-enhance-resilience-guidebook-ports), [step-by-step resilience method](https://resilientmaritimelogistics.unctad.org/guidebook/4-building-port-resilience-step-step-approach).

Research published in *Transportation Research Part E* in January 2025 models cascading failures in the global container shipping network by combining network topology and flow redistribution. A 2025 *Reliability Engineering & System Safety* paper proposes dynamic port-resilience assessment with time-varying allocation, congestion and recovery metrics. These are strong reasons to measure temporal loss and network spillovers rather than only the status of the initiating port. They do **not** establish that their assumptions, data or parameters are valid for this repository.  
Sources: Cao et al., [“Data-driven resilience analysis of the global container shipping network against two cascading failures”](https://doi.org/10.1016/j.tre.2024.103857); Zhang et al., [“Dynamic port resilience assessment in the maritime network: modeling flow evolution and cascading failure”](https://doi.org/10.1016/j.ress.2025.111927).

### Interoperability is a product requirement, not an afterthought

The IMO Facilitation Committee approved its Strategy on Maritime Digitalization at FAL 50 in March 2026, emphasizing interoperability, standardization, data sharing and data governance. The IMO notice states that the strategy is to undergo further committee review before submission for Assembly adoption in 2027; do not call it an Assembly-adopted instrument yet. The contract should therefore bind external evidence to source identity, product/dataset identity, revision, time semantics, use restrictions and exact bytes, rather than treating a URL or provider label as sufficient provenance.  
Sources: [IMO FAL 50 strategy notice, 31 March 2026](https://www.imo.org/en/mediacentre/pressbriefings/pages/facilitation-committee-approves-digitalization-strategy-cyber-security-measures.aspx), [IMO Maritime Single Window and data-exchange guidance](https://www.imo.org/en/ourwork/facilitation/pages/maritimesinglewindow-default.aspx).

### Refresh the hydrographic/port-system standards map

The IHO's current [Standards and Specifications catalogue](https://iho.int/standards-and-specifications) lists S-127 Marine Traffic Management Edition 2.0.0 and S-131 Marine Harbour Infrastructure Edition 2.0.0, both dated September 2026. Its separate [S-100 Standards in Force page](https://iho.int/en/s-100-standards-in-force-0) lists S-104 water-level information, S-111 surface currents and S-129 under-keel-clearance management as Edition 2.0.0 in force from 1 January 2026. The catalogues are not interchangeable: an implementation must verify the precise status, applicable edition, distribution channel, data availability, terms and field semantics for the intended deployment. A standard's existence does not mean that a relevant live dataset has been captured or that this project implements it.

**Design consequence:** dependency and evidence adapters should map to standard/product-specific semantics, but the resilience contract should not hard-code a single version into model truth. The adapter pins the revision and policy actually used in a run; any change produces a new run identity.

## 3. Formal semantics

### 3.1 Conditional policy

A policy is a versioned mapping:

`(observation_state, evidence_state, declared_preconditions) → simulated_action, next_rehearsal_state`

The v0.1 action vocabulary is deliberately bounded:

- `request_authoritative_evidence`
- `recompute_candidate_consequences`
- `retain_alternate_as_unconfirmed_option`
- `await_reassessment`
- `emit_indeterminate`

These are records about an analysis, not commands to outside systems. The v0.1 contract excludes tool execution, port booking, vessel dispatch and physical control.

When required evidence is unknown, missing, stale or conflicting, the policy must preserve that state and record what could resolve it. It must not infer that the dependency is available, unavailable, harmless or recoverable. If a forecast is invalidated, the prior dependent result cannot remain current merely because no new result was calculated.

### 3.2 Dependency graph

Each node represents a declared entity or service (for example, a terminal, energy supply, marine-access condition, data-exchange service or landside link). Each edge must bind:

- stable edge ID and endpoint IDs;
- typed relation (for example, `supports`, `requires`, `enables` or `coordinates`);
- evidence references and temporal applicability;
- the particular propagation model and version, or `not_declared`;
- whether the relation is direct, inferred or merely hypothesized.

No edge means “causal effect automatically known.” If a shared event affects two nodes, encode the common-cause membership as an explicit scenario declaration. Do not turn that declaration into an incident probability. If propagation has not been calibrated or admitted, report downstream quantitative effects as `UNSUPPORTED` and retain qualitative dependency context separately.

The graph must also represent uncertainty in links. An unknown edge, missing node or unresolved mapping cannot silently become a disconnected network, because that would falsely erase spillovers.

### 3.3 Service trajectory and units

A trajectory is a sequence of piecewise-constant interval measurements, with the interval semantics, units, demand basis and time origin bound to the run. A sample is not an observation unless it has source evidence; a model output is not an observation of the physical port.

For service class (s), interval (t), planned demand rate (D_{s,t}), delivered service rate (Q_{s,t}), and interval duration (Delta t):

- When (D_{s,t} > 0), service ratio is (R_{s,t}=Q_{s,t}/D_{s,t}).
- When (D_{s,t}=0), the ratio is `NOT_DEFINED`; do not silently divide by zero or claim 100% service.
- Integrated unmet service is (sum_t max(D_{s,t}-Q_{s,t},0)Delta t), reported **separately per service class** with its units.
- Recovery requires the service ratio to meet its declared floor for a declared number of consecutive, fully observed intervals **after the first observed interval below the floor**. Report the instant at the end of the qualifying run.
- If that run never occurs before the simulation horizon ends, report `NOT_OBSERVED_WITHIN_HORIZON`; do not extrapolate a recovery time.

The toy fixture uses a two-interval recovery rule and a service floor of 0.8 solely to exercise arithmetic. Neither value is a recommended maritime threshold. Real thresholds must be set and approved by the relevant domain stakeholders; they must not be inferred from the fixture.

Do not conflate these metrics:
- minimum service and duration below the declared floor;
- integrated unmet service by class;
- time to the declared recovery condition;
- whether a recovery plateau is stable or oscillating;
- affected nodes and direct versus propagated effects;
- resource/cost/emissions trade-offs, where supported by actual inputs.

These form a vector, not a magic weighted “resilience score.” A named governance policy may compare cost or service objectives, but it cannot compensate away hard safety/operational constraints.

### 3.4 Scenario coverage, not invented probabilities

A required case family should include single disruptions, shared dependencies/common-cause cases, delayed recovery, stale or conflicting data, alternative-resource uncertainty, and unknown/unsupported propagation. The report describes which dimensions have and have not been covered. Case inclusion is not a probability estimate.

Where probabilities are unavailable, compare the feasible set across all required declared cases and the vector of outcomes per case. Do not call a result an expected loss, confidence interval, “X% safe,” or probabilistically robust unless distributions and dependence assumptions are grounded and the uncertainty computation is itself validated.

### 3.5 Decision-relevance before value-of-information

For each evidence gap, identify:
1. which result or policy branch it affects;
2. the minimum fields needed to resolve the gap;
3. whether a hypothetical resolution could change a hard-constraint disposition or the candidate set;
4. the time horizon within which the evidence could still change the analysis;
5. issuer, provenance, licence, version and freshness requirements for a later real adapter.

With no justified probability model, this is a **decision-relevance test**, not a numeric ranking of expected information value. A simple deterministic counterfactual may show that evidence matters to a branch; it may not assert the probability that the evidence will arrive, that it is correct, or that the resulting plan will be feasible.

## 4. What the synthetic fixture demonstrates

The companion JSON file is a **contract illustration only**, and is not an input to PR #1654's evaluator or to a physical simulator. It contains:

- five toy nodes and three dependency edges; edge propagation models are explicitly undeclared;
- three required abstract cases, including an explicitly grouped common-cause case with no probability;
- a bounded evidence-first policy with no operational side effects;
- toy interval traces for two distinct service classes;
- examples of integrated unmet-service arithmetic and time-to-recovery;
- an unknown marine-access case for which no quantitative service trajectory is supplied.

The expected metrics are purposely simple enough for an independent test oracle. For instance, the power-loss essential-supplies series has a synthetic integrated unmet-service total of 100 normalized service units and reaches the toy recovery condition at hour 5; the common-cause essential-supplies series never meets the two-consecutive-interval recovery rule within the six-hour horizon. These are arithmetic examples—not port performance estimates, simulated output, or claims about real incidents.

## 5. Independent verification and falsification requirements

Before using this contract in software, add an independent oracle which does not call production metric, timestamp, policy or dependency-propagation functions. Freeze positive, negative and metamorphic controls.

Required tests include:

1. Identical pinned inputs and deterministic configuration reproduce identical canonical traces and digests.
2. Reordering JSON keys cannot change a semantic result, while changing any bound input byte changes the run binding.
3. A missing, stale, conflicting or unknown required dependency cannot improve a dependent service claim.
4. An unmodelled propagation edge never creates a numeric downstream effect; it produces an explicit unsupported result.
5. Duplicating correlated evidence cannot count as independent corroboration.
6. A common-cause case must impact all declared linked factors in the case definition; it must not acquire an implicit probability.
7. Adding a required adverse case cannot improve the robustly-feasible candidate set.
8. A hard-constraint violation cannot be compensated by lower service loss, cost or delay elsewhere.
9. Service ratio with zero demand is undefined, and negative demand or service is rejected unless a separately justified schema explicitly supports it.
10. Integrated loss uses duration, consistent units and each class's own demand basis.
11. Recovery requires a prior observed floor violation, consecutive fully observed recovery intervals and exact end-of-interval timestamp semantics.
12. A time series that fails to recover within the horizon cannot be assigned a made-up recovery time.
13. Service-class metrics are not silently collapsed into a weighted total.
14. Unconfirmed alternate-terminal capacity cannot become confirmed capacity, a reservation or a port-call event.
15. An evidence update produces a new assessment/run identity; it never rewrites an old receipt.
16. A policy output is only a simulated intent and cannot serialize into Mycelix's actual reservation, custody or delivery types.
17. A visualization, filter, legend, colour or selection change cannot mutate the canonical trajectory, receipt or admission state.
18. Any authority escalation fails verification even if all numeric metrics are correct.

For metamorphic claims, test the exact variable that is changed. Do not assume every additional service-capacity value or every new observation should improve the answer; it may legitimately reveal infeasibility. The invariant is that unsupported or missing evidence must never be converted into unjustified certainty.

## 6. Project boundaries

| Project | Contract responsibility | Non-responsibility |
|---|---|---|
| **Symtropy** | Scenario schedule, admitted model/capability profile, deterministic simulation, trace and run receipt | Inventing physics, asserting source truth, operational approval |
| **Symthaea** | Compare supported candidate policies, explain trade-offs and unresolved dependencies, identify decision-relevant evidence | Filling missing model physics or authorizing execution |
| **Mycelix** | Preserve `simulated_intent` / `proposed_allocation` as separate record types; later actual events only from their authoritative issuers | Turning a policy branch into a booking, custody or delivery event |
| **Sol Atlas** | Read-only temporal and dependency-lineage projection of the same versioned result bundle | Changing run state or implying navigation/engineering authority |

Reuse the existing receipt, source-provenance and scenario structures. Do not build another simulator, trust score, ontology, scheduler or runtime to represent these semantics.

## 7. Delivery sequence and honest gates

1. Keep PR #1654's evaluator and forecast-applicability parent as hard gates. Exact-head Rust checks being queued is not a pass.
2. Review and freeze this contract/fixture only as a prospective interface, including independent verification of all example arithmetic.
3. Inspect the current deterministic engine capability census and identify the minimum supported trajectory/event interfaces before writing an adapter. If those capabilities are absent, report the specific unsupported effects instead of implementing invented physics.
4. Add a versioned schema and pure trajectory reducer with an independent oracle and negative/metamorphic corpus. Bind exact configuration, event schedule, model/capability profile and input receipts.
5. Add common-cause and network propagation only when the propagation model, assumptions and calibration evidence are independently reviewed. Until then, output `UNSUPPORTED` for quantitative spillover.
6. Add policy rehearsal as a deterministic, read-only evaluator. No external side effects.
7. Validate against at least one carefully sourced historical disruption case using pinned data, and reserve a separate holdout case to avoid grading the model on the scenario used to design it. Include a domain expert review of assumptions and time semantics.
8. Only then consider real, read-only adapters for relevant official data products. Capture exact bytes, revision, retrieval metadata and terms state; do not equate access to a standard with access to a valid, licensed and relevant live dataset.
9. Integrate with Symthaea, Mycelix and Sol Atlas via a versioned result contract and explicit authority separation.

**Completion ceiling for this increment:** research-aligned contract + synthetic JSON example only. It does not implement a trajectory engine, demonstrate a network cascade, establish any maritime threshold, execute a real policy, or confirm exact-head CI. Future code must pass the independent oracle and exact-head workflow before its claims can be elevated.
