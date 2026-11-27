# Knowledge-indexed terminal evaluation (KITE): information-set replay and decision-time regret v0.1

Companion to [MCSC-KITE-001 #7775](https://github.com/Luminous-Dynamics/symtropy/issues/7775), under [MCSC #7686](https://github.com/Luminous-Dynamics/symtropy/issues/7686) and [MCSC-RESILIENCE-REGRET-001 #7747](https://github.com/Luminous-Dynamics/symtropy/issues/7747).

## 1. Why this benchmark exists

An online recovery policy cannot act on evidence that has not reached an authorized decision-maker. A hindsight planner can. Comparing the policy directly with a clairvoyant plan mixes at least two distinct effects:

1. **Information gap:** the event, dependency, forecast, revocation or inspection result was not yet observable or admissible at the decision cutoff.
2. **Decision gap:** the information was available, but the policy selected a worse admissible action than another policy could have selected from the same information.

A third gap—**execution/authorization delay**—exists between a plan being computed, published, authorized and actually started. It must not be disguised as algorithmic decision quality.

KITE adds an explicit information-time contract to MCSC. It is not another planner or evidence store. It consumes immutable owner receipts and timestamped evidence from Mycelix/Symtropy owners, then evaluates decisions against the evidence vintage that was genuinely available at each historical decision.

## 2. Paradigm shift: from event-time replay to knowledge-time replay

A physical event time is not the same as the time its evidence became available. Every evidence item must preserve at least:

| Clock | Meaning |
|---|---|
| `event_time` | When the described physical or administrative event occurred, as asserted by its source |
| `source_observed_at` | When the source first observed or recorded it |
| `first_available_at` | Earliest time the evidence was available to the named decision authority |
| `ingested_at` | When the receiving system ingested the evidence |
| `validated_at` | When integrity, schema, authority, freshness and rights checks completed |
| `decision_cutoff_at` | Frozen cutoff for a particular policy decision |
| `plan_published_at` | When a candidate plan became available for review |
| `authorized_at` | When the relevant authority authorized the action |
| `action_started_at` | When execution actually began |
| `service_milestone_at` | Inspection, receiver acceptance and on-time service event times |

These values must not be collapsed into a single timestamp. A source can report at 14:00 that an outage began at 11:00; an 12:00 decision did not know about that outage merely because the physical event time was 11:00.

For decision epoch \(t\), define the admissible knowledge set:

\[
\mathcal{F}_t = \{e : first\_available\_at(e) \le t
\land ingested\_at(e) \le t
\land validated\_at(e) \le t
\land authority(e,t)=valid
\land freshness(e,t)=valid\}.
\]

The exact owner-provided rights and authority predicates take precedence; this expression is a benchmark definition, not a replacement authorization engine.

A policy action \(a_t\) must be measurable only with respect to \(\mathcal{F}_t\). In ordinary terms: the policy can depend on what it could actually have known at the time, not on the later corrected database.

### 2.1 Late evidence and historical truth

- Late evidence can change the current world model and present admissibility.
- Late evidence must not silently alter the historical knowledge set used to reproduce an earlier decision.
- A corrected record creates a new evidence vintage. Preserve the earlier version, its digest and its first-available time.
- A revocation discovered after cutoff changes current authority. It does not rewrite whether an earlier action was authorized on the evidence actually available then; assess current and historical authorization separately.
- Unknown, censored, disputed and missing observations remain typed UNKNOWN/INDETERMINATE. They are never coerced to zero, false or independent.

## 3. Three distinct references, not one impossible baseline

Every scenario and decision epoch should be evaluated against three references.

### R0 — perfect-information / clairvoyant bound

The reference knows the realized scenario, including events that had not yet occurred or become observable at the decision cutoff. It obeys the same physical, quality, dependency, capacity and service constraints.

Use it only as an information upper bound / lower-cost bound. It is not a deployable policy, not a fair sole comparator for online algorithms, and not evidence that the gap could have been eliminated operationally.

### R1 — information-set-constrained decision oracle

The reference sees exactly \(\mathcal{F}_t\) at every cutoff and obeys the same hard constraints, action set, owner evidence and authority rules as the policy under test. It may be solved offline to an optimality certificate and can therefore be computationally idealized.

R1 asks: *given the information that really existed at that moment, what was the best admissible decision?* This isolates decision quality from the absence of future information. Its compute time must still be reported; do not imply it could have responded inside a production deadline if the solver took hours.

### R2 — operational, compute-budget-matched policies

These are deployable baselines and candidates. All receive the same evidence stream and cutoff schedule, and are subject to the same wall-clock, core, memory and authorization constraints.

At minimum compare:
- existing/no-new-investment or no-action policy;
- deterministic / FCFS where applicable;
- bounded disruption management;
- full rescheduling;
- proactive–reactive hybrid;
- two-stage rolling-horizon methods;
- robust or adversarial scenario methods;
- event-triggered MPC / rolling control where the problem mapping is valid;
- a learned recovery policy only when training data, prediction horizon, rights and inference budget are disclosed.

Keep R0, R1 and R2 separate in every scorecard. The R1 idealized solver is not a substitute for an R2 compute-matched algorithm.

## 4. Regret decomposition

For an outcome dimension \(m\), preserve the direction of improvement. For minimization metrics (e.g. cost, delay, emissions), lower is better; for service quantities, higher is better.

For compatible scalar dimensions, show the descriptive decomposition:

\[
R_{total}^{R0}(m) =
R_{information}^{R0\rightarrow R1}(m)
+
R_{decision}^{R1\rightarrow R2}(m).
\]

Where:
- \(R_{information}\) is the gap between the clairvoyant and same-information oracle;
- \(R_{decision}\) is the gap between the same-information oracle and the tested policy.

Execution and authorization delay remain separately measured because they can change the realized service outcome. This equation is a bookkeeping identity only when all three outcomes share an exact scenario, decision cutoff, denominator, constraints and metric definition. For the general multi-objective case, report the component-wise vector and scenario-specific decomposition; do not scalarize or infer causal attribution from arithmetic alone.

A policy cannot improve its apparent regret by dropping demand, changing the service denominator, relaxing quality, treating an unreserved offer as reserved capacity, or using future data.

## 5. Replay contract

Every replay bundle must freeze:

1. source event IDs and evidence version/digest;
2. event, first-available, ingestion and validation timestamps;
3. source authority, rights and freshness dispositions;
4. scenario, instance, service, policy and model versions;
5. decision cutoffs and the information set visible at each cutoff;
6. physical/dependency/resource constraints and their provenance;
7. policy inputs, code/configuration digest, random seed and solver parameters;
8. compute budget, hardware/runtime envelope, timeout and fallback rules;
9. candidate plan publication, authority decision and action-start receipts;
10. inspection, receiver acceptance, unresolved and censored outcome records.

Replay must be deterministic for identical frozen inputs and code/configuration versions, or the nondeterministic elements and their seeds must be exposed. Preserve the exact input ordering only where the owner contract defines order as semantically meaningful.

### 5.1 No-lookahead negative controls

The executable corpus must demonstrate all of the following:

- **Late-source control:** an event occurred before cutoff but was first available after it; the historical policy must not see it.
- **Late-ingestion control:** source observation existed, but receiving system ingested it after cutoff; the policy must not see it.
- **Late-validation control:** evidence was ingested but was not validated/authorized at cutoff; it must not become admissible.
- **Backfill control:** a record inserted later with an old event timestamp remains invisible to earlier decisions.
- **Correction control:** a revised record after cutoff cannot replace the earlier evidence vintage in historical replay.
- **Revocation control:** a revocation available before cutoff blocks current action; one discovered after cutoff affects current state but does not rewrite the historical replay.
- **Clock-confusion control:** changing event_time without changing first_available_at cannot make evidence arrive earlier.
- **Future-feature control:** a feature computed using post-cutoff data is detected and rejected.
- **Unknown/censoring control:** a missing receiver observation is not counted as rejection or success without an owner-defined disposition.
- **Duplicate/conflict control:** duplicate evidence does not multiply service; conflicting owner claims produce the canonical conflict/unknown state.
- **Deadline control:** an admissible plan published after the action cutoff is recorded as late even if its objective is better.
- **Authorization control:** a computed plan is not executable until the required authority receipt is valid.

The companion [`information-set-replay-v0.tsv`](fixtures/information-set-replay-v0.tsv) is a synthetic case manifest for these controls. It is a test-design corpus, not a real event history or a probability sample.

## 6. Benchmark scorecard

Publish separate scorecards rather than a single leaderboard number.

### Service and recovery
- expected, inspection-qualified, receiver-accepted, on-time and unresolved units;
- protected service-floor violations and duration below floor;
- service-deficit unit-hours;
- deadline misses and accepted-but-late units;
- time to minimum viable service and full restoration;
- cancellation, substitution, rework and partial recovery.

### Decision and compute
- regret vectors against R0 and R1;
- feasible / infeasible / unknown plan disposition;
- event-to-first-admissible-plan latency;
- event-to-authorized-action and event-to-action-start latency;
- p50, p95, p99 and worst observed latency;
- replanning invocations, plan churn and objective-bound gap;
- CPU/GPU time, peak memory, timeout rate and fallback/abstention rate.

### Evidence and governance
- first-available-to-ingestion and ingestion-to-validation delay;
- stale, expired, revoked and wrong-configuration admission counts (target: zero);
- false retention and false invalidation rates for requalification;
- source coverage, unknown dependency remainder and cross-owner receipt lag;
- historical replay mismatch count (target: zero);
- invalid-authority action count (target: zero).

### Reliability and uncertainty
- report median and tail service times, not mean alone;
- report variation (e.g. coefficient of variation) only with clearly defined population, censoring and segmentation;
- when forecasts are used, report calibration, interval coverage, sharpness and proper scoring rules separately from decision utility;
- use probability-weighted expected loss or CVaR only when event frequencies and sampling assumptions are sourced and disclosed;
- keep unknown frequency separate from observed zero incidence.

## 7. Evaluation design

### 7.1 Same-information matched episodes

Each policy receives byte-identical evidence vintages and identical decision cutoffs. Enforce non-anticipativity at every replanning epoch, not just at the initial plan. Each policy receives the same action set and constraints; record any policy-specific preprocessing and compute resource use.

### 7.2 Temporal and structural holdouts

A valid generalization study must include:
- rolling-origin temporal evaluation;
- complete port/facility holdouts;
- event-family holdouts (feeder outage, supplier/material failure, workforce, calibration/qualification, corridor, cyber/configuration, weather);
- supplier, utility, corridor and owner holdouts where IDs can be safely generalized;
- separate vessel/cargo/service classes where the operational mechanics differ.

Random row splitting alone is insufficient because correlated events, shared vendors and repeated port calls can leak across train and test partitions.

### 7.3 Operational baselines and external alignment

The benchmark matrix should preserve conventional port measures and add reliability/service measures without claiming to replace them.

- The 2026 two-stage rolling port-scheduling study at Huanghua Coal Port uses historical vessel patterns, Monte Carlo disruption cases and hybrid rolling strategies; it compares its tailored algorithm with GA, FCFS and CPLEX. Reproduce only with aligned problem constraints, source access and compute budgets. [Wang et al., *Computers & Industrial Engineering* (2026), DOI 10.1016/j.cie.2026.112035](https://doi.org/10.1016/j.cie.2026.112035).
- The 2026 AIS-derived liner recovery study identifies behaviours including port-skipping, port-swapping and speed-up across 3,559 container vessels and 700+ ports (2018–2023), then predicts strategy choices. Treat observed behaviour and predicted suitability as distinct from counterfactual operational benefit. [Zhang et al., *Transportation Research Part E* (2026), DOI 10.1016/j.tre.2026.105009](https://doi.org/10.1016/j.tre.2026.105009).
- The 2026 systematic review of 45 disrupted seaside-planning studies identifies disruption management, rescheduling and hybrid proactive–reactive strategies, while noting limited propagation, integrated-model and broad-metric coverage. [Review, DOI 10.1080/00207543.2026.2719841](https://doi.org/10.1080/00207543.2026.2719841).
- A 13 November 2026 study operationalizes port-time variability as the coefficient of variation across 401 ports in 119 countries and argues that mean turnaround suppresses schedule reliability information. Keep mean, tail and variability measures distinct, with cargo/port segmentation and clear censoring rules. [Chhetri et al., *European Transport Research Review* (2026)](https://doi.org/10.1186/s12544-026-00862-w).
- Digital-twin benchmarking across seven European port/logistics initiatives reports gaps in data availability, model transparency, operational-performance reporting and synchronization detail. KITE therefore measures event/vintage synchronization and decision latency, not merely whether two systems are connected. [*Digital twin architectures for smart port operations* (2026)](https://doi.org/10.1016/j.rineng.2026.110485).

These papers provide different datasets, models and outcome definitions. No published percentage, accuracy, cost saving or runtime transfers to MCSC without a matched reproduction. The first goal is reproducing at least one strong public baseline faithfully, not selecting a flattering headline result.

## 8. Proposed qualification targets

Targets for the next executable version; not achieved claims:

- 100% of replayed actions use only the as-of information set at the decision cutoff.
- Zero future-information leakage under every negative control in section 5.1.
- Zero historical replay changes when only post-cutoff evidence is appended.
- Zero stale, expired, revoked, wrong-configuration or unauthorized capability admission.
- 100% deterministic receipt reproduction for identical frozen inputs, code/configuration, seed and policy versions.
- Identical evidence vintage, service denominator, constraints and decision cutoffs across compared policies.
- P50/P95/P99 plan and authorization latencies reported; missed deadlines never removed from the denominator.
- Separate R0, R1 and R2 scorecards with exact bound semantics and compute disclosure.
- Generalization results on held-out time, facility, supplier and event-family blocks.
- No probability-weighted claim without a documented, source-grounded sampling frame.

## 9. Claim ceiling and implementation order

A synthetic replay suite proves that the evaluator enforces the declared information-time rules. It does not prove that the upstream timestamps are truthful, a physical twin is synchronized, a port will recover, or an intervention caused better service.

Recommended sequence:

1. Freeze event-vintage schema and the case manifest.
2. Extract the passing-or-pending conformance parser from `tests/information_set_replay.rs` into a reusable Rust module, then implement deterministic as-of selection over append-only evidence vintages.
3. Implement no-lookahead negative controls before adding a recovery optimizer.
4. Consume canonical Mycelix/Symtropy receipts by exact digest, authority, rights, freshness and version.
5. Add the R0/R1/R2 benchmark harness and compute-budget capture.
6. Reproduce a public rolling-horizon or AIS-based baseline under matched constraints.
7. Only then run held-out port/facility experiments and evaluate operational outcomes.

The distinctive MCSC proposition is not “a better schedule.” It is **a replayable, authority-aware answer to what could legitimately have been decided at each point in time, what service that decision preserved, and how much avoidable regret remained under the same information.**
