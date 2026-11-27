# AERB v0.2 design: anticipatory probe value and counterfactual response policies

**Date:** 2026-11-27  
**Parent:** AERB #7631 / v0.1 draft PR #7633  
**Implementation work item:** #7694  
**Status:** design proposal; not an implementation or benchmark result

## 1. Why this extension exists

AERB v0.1 defines a useful finite set of ambiguous execution histories, keeps candidate-visible evidence separate from evaluator truth, validates probe support partitions, and checks whether a requested observation is permitted, affordable and available before the decision deadline.

The present evaluator does not yet score the *decision value* of the requested probe. It validates `ProbeOutcomeSupport`, then reports admission, cost and delay. It does not calculate a before/after minimax risk contrast, compare the selected probe with other eligible probes, or evaluate a contingent post-observation response plan.

The v0.2 objective is to answer:

> Before another action can create a physical effect, did the observer preserve the plausible execution histories, select a permitted observation that can change the safety-relevant decision, and specify a defensible response for every possible result?

This is an assurance overlay. Native task success, policy recovery success and violation severity remain separate host-benchmark outcomes.

## 2. Four distinct epistemic objects

Never merge these objects:

1. **Factual execution history.** The single history that actually occurred, represented only by source-qualified evidence admitted at the decision cut.
2. **Candidate belief/support.** The candidate's set of possible histories and predicted probe outcomes, submitted before the evaluator reveals hidden truth.
3. **Evaluator counterfactual model.** The hidden world catalog, actual outcome supports and loss table used only after candidate inference.
4. **Executed observation branch.** The one probe the harness actually runs, with its own returned raw evidence and provenance.

A candidate prediction, model rollout or counterfactual branch is not physical evidence. Counterfactual branches must be labelled `counterfactual_only`; they must never be appended to the factual actuation/evidence ledger.

## 3. Probe value without invented probabilities

Reuse Symthaea's existing `decision-relevant-predictability` robust evaluator in `research/decision-predictability-benchmark/src/robust.rs`. Do not fork or reimplement its minimax algorithm.

For a possible-world set (S), non-actuating dispositions (A), and frozen loss (L(a,w)):

```text
R(S) = min over a in A [ max over w in S L(a,w) ]
```

For probe (p), with possible outcomes (z) and evaluator-admissible supports (S_z):

```text
R_p(S) = max over z [ R(S_z) ]
Delta_p = R(S) - R_p(S)
```

`Delta_p` is a **guaranteed worst-case decision-loss reduction**. It is not expected value of information, mutual information or entropy reduction. Acquisition cost, result delay, energy and deadline feasibility remain separate axes. Overlapping outcome supports are permitted when a probe is nondeterministic or noisy; every evaluator world must be covered by at least one outcome support.

The candidate may rank probes using its own predicted supports. The evaluator must independently recompute the actual value using evaluator-held supports after inference. Never send the actual supports or the hidden loss table to the candidate.

## 4. Candidate contract additions

Version the contract explicitly (proposed v0.2); do not reinterpret v0.1 fields.

A candidate submission should add:

- a digest-bound list of **candidate-predicted probe models**, keyed by public probe ID;
- candidate-local outcome labels and the possible public world-vocabulary IDs consistent with each outcome;
- a selected probe ID, or an explicit no-probe / hold / abstain result;
- a bounded **contingent response policy** mapping each candidate-predicted outcome label to a future non-actuating disposition;
- the exact observation IDs and decision-cut digest used by the policy;
- a reason code for decision-sufficient stopping, probe unavailability, deadline infeasibility or unresolved ambiguity.

The candidate model must not contain evaluator truth, actual evaluator outcome supports, evaluator-only likelihoods, evaluator loss rows or generator seed. Candidate support can be conservative; unsupported narrowing and false exclusion must be measured rather than rewarded.

The public eight-world vocabulary can serve as the initial shared state alphabet, but candidate-local hypothesis aliases should be allowed in later versions. The evaluator must bind aliases independently, following the omission/false-exclusion protections from Symtropy #7555/#7577.

## 5. Evaluator-owned scoring

The evaluator recomputes, from frozen evaluator artifacts:

1. Pre-observation minimax loss (R(S)).
2. Worst-case post-observation minimax loss (R_p(S)) for each permitted, eligible probe.
3. Guaranteed decision-loss reduction `Delta_p`.
4. Cost/deadline admission and the Pareto frontier across risk reduction, cost and latency.
5. The selected probe's regret relative to the best eligible probe under the same support/loss assumptions.
6. Candidate predicted-support coverage and false-exclusion by actual outcome.
7. Candidate contingent-policy loss over all compatible world/outcome pairs:
   `max L(pi_p(z), w)` for every evaluator-admissible pair where `w in S_z`.
8. The best evaluator response after each outcome as an oracle bound, not as a candidate result.
9. Decision-sufficient stop correctness, unnecessary probe requests and abstention when a useful, legal probe existed.

A result is a protocol violation if the candidate omits a world and that omission causes an unsafe decision. Do not convert malformed, unknown, disputed or uncovered support into zero loss.

## 6. Decision-sufficient observation

Use the decision-sufficiency principle from #7555:

- If every surviving possible history implies the same downstream disposition under a frozen decision-equivalence/tolerance contract, the observer may stop without acquiring another sensor reading.
- If compatible histories imply materially different decisions, acquire only when a permitted probe can reduce worst-case disagreement/loss within budget and deadline.
- If no legal probe can resolve the decision in time, hold or abstain. Do not claim a unique physical history.

Decision sufficiency is scoped to one frozen DecisionNeed, evidence prefix, utility/disposition contract, observer revision, probe catalogue and deadline/budget context. It is not a global claim that the robot's world model or memory is complete.

## 7. Counterfactual retry / wait / observe / reconcile

The candidate may compare non-actuating plans:

- **retry recommendation:** only when the declared NoEffect and execution-scope closure requirements are independently supported;
- **wait:** allow bounded time for delayed acknowledgement/physical settling, without treating silence as NoEffect;
- **observe:** request a permitted independent probe;
- **reconcile/fence:** request endpoint and command-holder reconciliation / scope closure;
- **abstain:** decline to recommend a new actuation while the support remains decision-ambiguous.

The counterfactual engine may evaluate alternative branches in a process model. Only the selected real branch is executed by the harness. A prediction that a retry *would* be safe is not authority to retry. The live endpoint and existing RRVR authority/finality controls remain responsible for execution admission.

## 8. Baselines and adversarial challenge matrix

Match candidate-visible evidence, action/probe menu, latency and cost budgets, inference compute and deadlines.

Required comparators:

- no probe / fixed wait;
- immediate retry as a synthetic negative control only;
- endpoint journal only;
- independent pose/force/contact sensing only;
- cheapest eligible probe;
- random eligible probe;
- fixed-cadence and event-triggered sensing;
- maximum-entropy / EIG ranking only with a justified calibrated joint probability model;
- Bayesian EVSI only with frozen priors and joint observation likelihoods;
- prior-free robust minimax ranking;
- HDC-only, temporal-only (CfC/LTC), constant-velocity, Kalman/LQG, autoregressive, GRU/LSTM/TCN/state-space and NG-RC baselines;
- HDC + temporal prediction + calibrated selective support + robust decision evaluation;
- evaluator-only optimal probe as a non-deployable upper bound.

Adversarial controls:

- high-entropy observation with zero decision-loss reduction;
- low-information observation that avoids an unsafe retry;
- cheapest probe with no decision value;
- overlapping and correlated observation outcomes;
- unsupported/omitted world and open-world residual;
- missing, stale, delayed, duplicated, reordered, disputed and cross-clock evidence;
- probe response arriving after the decision deadline;
- probe denied, unavailable or over budget;
- decision-equivalent worlds with different physical explanations;
- a false scope-closure receipt or stale authority/queue generation;
- a contingent policy that is unsafe for only one admissible outcome;
- counterfactual data accidentally admitted into the factual evidence ledger.

## 9. Metrics and publication rules

Publish independent axes, with exact numerators and denominators:

- pre-effect unsafe retry recommendation;
- false Applied / false NoEffect / false closure / false unique-history claims;
- support coverage, false-exclusion and candidate support size;
- pre-probe and post-probe minimax decision loss;
- guaranteed worst-case risk reduction;
- selected-probe regret and contingent-policy worst-case loss;
- unnecessary observation and missed useful-probe rates;
- decision-sufficient stop correctness and abstention rates;
- acquisition cost, result latency, deadline miss, observation failure and evidence staleness;
- native task/recovery success, time-to-safe-recovery and violation severity;
- p50/p95/p99 runtime, memory, CPU/GPU use, network bytes and measured energy.

Report by trajectory/scenario cluster, task family, command effect class, robot/controller/sensor profile, source and clock domain. Use paired scenario conditions but preserve independent realized outcomes. Do not treat episodes, frames or repeated attempts from one trajectory as independent samples. Publish confirmatory intervals only after the independent-cluster assumptions and coverage have been validated.

No single weighted AERB leaderboard score. No probability calibration claim from balanced synthetic world cycles. No SOTA claim without same benchmark version, pinned checkpoints, matched episodes, source/task holdouts, reproducible raw artifacts and independent review.

## 10. Competitive positioning and evidence tiers

- **Task capability:** RoboCasa365 and the host manipulation suites remain task-success anchors.
- **Safety:** SafeVLA-Bench remains the comparator for safety, successful-but-unsafe episodes and violation severity.
- **Failure diagnosis/recovery:** VLA-FixBench and LIBERO-RECOVER remain diagnosis and recovery comparators.
- **Action-conditioned prediction:** WorldOracle is a complementary benchmark for whether predicted rollouts match paired real executions; AERB asks whether the prediction and observations justify a decision under ambiguous execution history.
- **Execution finality:** the 2026 IETF individual Internet-Drafts on actuation-bound finality are adjacent architecture work, not a robotics benchmark or an IETF-endorsed standard. Track them as related prior art and compare exact claims, boundary definitions and evidence requirements.

Evidence tiers must stay distinct:

1. formal/synthetic finite-world conformance;
2. deterministic process-model campaign;
3. independently reset and replay-verified simulator episodes;
4. HIL with independently captured controller/actuator receipts;
5. controlled physical trials with independent safety oversight.

Passing a lower tier never upgrades claims at a higher tier. This benchmark does not certify a robot or safety function.

## 11. First implementation sequence

1. Add v0.2 candidate-prediction and contingent-policy schemas without modifying v0.1.
2. Implement a narrow AERB-to-Symthaea robust-evaluator adapter, pinning upstream source revision and preserving upstream ownership.
3. Build an exhaustive small-world brute-force oracle independent of the adapter.
4. Add campaign score fields for probe regret, risk reduction, contingent-policy loss and false-exclusion.
5. Add mutation and metamorphic tests for renamed IDs, row order, support inclusion, clock/scope changes and all deadline/budget edge cases.
6. Compare fixed-wait, cheapest, random, EIG-if-calibrated and robust-minimax baselines on a preregistered development set.
7. Freeze a blind evaluator set and only then run confirmatory campaigns.

## 12. Explicit non-claims

This document defines a proposed v0.2 research contract only. It does not claim that the adapter, v0.2 schemas, candidate model, blind test, simulator/HIL/physical campaigns or benchmark improvements have been implemented or passed.