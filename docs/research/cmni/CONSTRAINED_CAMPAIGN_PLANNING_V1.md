# CMNI constrained evidence-campaign planning v1

**Status:** exact finite scenario-tree planner authored on `research/cmni-campaign-planning-v1`; fresh CI and external review pending.  
**Tracking:** issue #6889; implementation draft PR #6893.

## 1. Problem

The finite EVSI policy selects one acquisition using its immediate expected value of sample information minus cost. Scientific campaigns can have complementarities: an acquisition can be valuable mainly because it creates a posterior state in which a later acquisition becomes useful. A myopic policy can miss that conditional value.

Guo et al. (2026), *Constrained Bayesian Experimental Design via Online Planning*, ICML 2026, PMLR 306, combine amortized policies and posterior networks with online multi-step lookahead planning over scenario trees to handle dynamic budgets and physical constraints: https://proceedings.mlr.press/v306/guo26g.html

CMNI's first step is not an amortized policy. It is an exact finite oracle against which approximate planners can be measured.

## 2. Implemented contract

`campaign_planning.rs` enumerates a finite scenario tree:
- states have a declared prior probability;
- decision actions have state-dependent utility;
- acquisition options have conditional likelihoods over discrete outcomes and fixed costs;
- each option may be used at most once;
- after every outcome, the posterior over the declared expanded state is updated and the remaining campaign is re-optimized;
- each root-to-leaf path obeys a hard resource budget;
- at every node, the planner compares further acquisition against immediate action selection or abstention;
- if the node-expansion budget is exhausted, the planner returns an error rather than a partial plan presented as optimal.

The one-step finite EVSI policy is the myopic baseline. Setting maximum depth to one produces a direct exact one-step design oracle.

## 3. Critical source-dependence rule

All qualified acquisition options in one campaign must use the same campaign-scoped joint-model receipt. The receipt must cover the full conditional likelihood/factorization model across the campaign. Merely using different source IDs is not sufficient.

This interface factorizes future outcomes conditionally on the supplied expanded state. That state must include relevant common-cause latent variables. If later likelihoods depend on history in a way not captured by that state, this factorized option table is not applicable; a branch-specific joint transition model is required.

Unqualified options are skipped. A campaign with mismatched receipt scopes fails closed.

Receipt IDs are references for external verification, not self-authenticating proofs.

## 4. Exact synthetic lookahead benchmark

The machine-readable fixture is `research/cmni/fixtures/conditional-third-test-lookahead-v1.json`.

Two hypotheses are equally likely. The decision-maker can choose hypothesis A or B, receiving utility +1 when correct and -1 when wrong. Each available test has 0.8 accuracy and costs 0.05 utility units.

A one-step policy sees expected decision utility 0.6, pays 0.05, and obtains net utility 0.55.

A three-stage lookahead planner chooses the first two tests. If they agree, it stops and selects the supported hypothesis. If they disagree, it acquires the third test because the posterior remains ambiguous. The third test is acquired on 32% of paths.

The scenario-tree calculation gives:
- raw expected decision utility: 0.792;
- expected acquisition cost: 0.116;
- net expected utility: 0.676;
- maximum pathwise cost: 0.15.

Thus, the lookahead plan gains 0.126 expected utility over the one-step policy in this constructed case. The key effect is conditional value: the third test is useful only after the ambiguous branch. This fixture is synthetic, and no physical independence claim is made.

## 5. Benchmark ladder

| Benchmark | Baseline | Metric |
|---|---|---|
| One-step | finite EVSI policy | exact agreement at depth 1 |
| Small campaigns | exhaustive scenario tree | exact optimality and node count |
| Myopic traps | greedy receding horizon | regret against exact tree |
| Dynamic budgets | budget-feasible tree | zero pathwise budget violations |
| Source dependence | shared campaign receipt vs mismatch | zero qualified result from mismatched scopes |
| Scaling | exhaustive tree vs beam search / MCTS / amortized policy | regret, expansions, wall time, memory |
| Model shift | held-out likelihood/cost/constraint combinations | regret and calibration under shift |

Proposed targets (not achieved):
- exact agreement with exhaustive dynamic programming on frozen small instances;
- zero pathwise resource violations;
- zero unqualified or mismatched-receipt campaigns reported as qualified;
- median regret ≤1% and 95th percentile ≤5% against exact small-instance optima on a frozen larger suite;
- publish node expansions, tree depth, wall time, memory, acquisition counts, expected cost and regret separately.

## 6. Limits and next work

This is a small finite-state exact oracle. It is not yet:
- a full continuous-state POMDP;
- a general physical constraint / apparatus scheduler;
- an amortized policy;
- MCTS, beam search or differentiable scenario-tree planning;
- a nonlinear posterior update engine;
- a physical likelihood or detector model.

Next: add explicit campaign resources (time, apparatus, sample inventory), source-evidence state transitions, a branch-specific joint likelihood interface for history-dependent dependence, and approximate planners benchmarked against this oracle.

Do not claim superiority over the ICML 2026 online-planning method until a matched reproduction and held-out comparison exists.


### Cross-check against the one-step EVSI implementation

The depth-one scenario-tree regression now invokes the public `recommend_finite_decision_step` function from the parent finite decision module using the same prior, utilities, single-option likelihood, cost conversion, and budget. It asserts that the one-step campaign value matches that independently implemented finite EVSI net benefit to (10^{-12}). This makes the myopic baseline an executable cross-module parity check rather than only a shared hard-coded expected value.


## 7. Shared-calibration sequential negative control

A second campaign regression expands the hidden state to include both the physical hypothesis and a shared calibration state. Two repeated tests are conditionally independent only given this expanded state; after marginalizing the calibration state, their outcomes are dependent.

The synthetic prior is 0.35 / 0.35 for physical A/B under a good calibration and 0.15 / 0.15 under a bad calibration. Each test has 0.99 accuracy with good calibration and 0.01 accuracy with bad calibration, and costs 0.002 utility units. The same campaign-scoped receipt covers the conditional likelihood model for all three options.

The exact scenario-tree oracle returns net utility 0.395722. It uses three tests only on the branch where the first two disagree, whose probability is 0.0198. The expected number of acquisitions is 2.0198; the remaining paths stop after two observations. This provides a concrete shared-cause example: repeated measurements do not supply fresh independent calibration information, and the planner must preserve the shared latent state when calculating the joint outcome law.

The complete synthetic input and target values are frozen in research/cmni/fixtures/repeated-tests-shared-calibration-v1.json.

The test does not establish any real calibration probability or detector likelihood. It tests the mathematics of the declared finite model and the requirement to include common-cause variables in the expanded state.
