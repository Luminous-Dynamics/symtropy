# Sequential diagnostic measurement and stopping benchmark v0.1

**Status:** exact finite-horizon policy evaluator with synthetic priors, utilities, costs and likelihoods. It is not a physical action planner, POMDP solution for unbounded campaigns or operational authority.

## 1. Research motivation

NIST's Sequential Bayesian Experiment Design programme describes an adaptive measure-and-update workflow: posterior information is incorporated as observations arrive, and the next measurement is chosen to improve uncertainty while conserving expensive resources. Its published examples include particular simulations and laboratory estimation problems; their gains must not be transferred to mining without a matched evaluation.
https://www.nist.gov/programs-projects/sequential-bayesian-experiment-design

Cheng & Huan (2025; revised June 2026), *Optimal stopping for sequential Bayesian experimental design*, formalize experiment choice and stopping jointly as finite-horizon sequential decisions. Their evaluations are numerical simulations, not excavation trials.
https://arxiv.org/abs/2509.21734

The existing one-step EVSI contract in #3917 already requires a declared prior, decision utility, observation likelihood, full experiment cost and stopping rule. This benchmark extends that contract with adaptive outcome-specific branches. It does not replace EVSI with entropy reduction or raw information gain.

## 2. Paradigm shift

A fixed measurement sequence is not always optimal. The value of an observation depends on what has already been observed. A useful first measurement may resolve one branch completely but leave another branch ambiguous, requiring a different second measurement.

The policy must compare:
- STOP now and select the best terminal decision;
- perform an admissible passive measurement, pay its full cost and branch on its possible observations;
- select a different measurement after each outcome;
- stop at any later branch when no further measurement clears the declared net-value threshold.

The planner maximizes expected terminal decision utility net of measurement costs, under a finite horizon and budget.

## 3. Exact finite-horizon recursion

Let w(s) be unnormalized joint belief weights at a policy node and m = sum_s w(s). These weights are not normalized after every outcome; retaining joint mass keeps expected branch utility additive and avoids silently losing probability.

For terminal action a with utility U(a,s):

`V_stop(w) = max_a sum_s w(s) U(a,s)`

For experiment e with conditional likelihood L(z|s,e), cost C(e), and a finite remaining horizon:

`V_continue(w,e) = -m*C(e) + sum_z V(w(s)*L(z|s,e))`

The value at a node is the maximum of the stop value and every admissible continuation value, subject to:
- remaining budget;
- maximum measurements remaining;
- remaining horizon;
- declared minimum net value required to continue;
- no repeated measurement or dependence group along the same branch.

The continuation threshold is scaled by belief mass so it is applied to the conditional decision problem rather than changing with the unnormalized probability of reaching a branch.

The recursion is exact for the declared finite model. It is not an infinite-horizon solver, learned policy or general partially observable Markov decision process (POMDP).

## 4. Correlated measurements and dependence

Sequential likelihood multiplication is only valid under an appropriate conditional probability model. If the same sensor, physical principle, sample or data window is reused, observations may be correlated even after conditioning on the fault hypothesis.

v0.1 therefore requires:
- a case-level conditional-independence assumption reference;
- a dependence-evidence reference and independence-group ID for every measurement;
- no two measurements from the same declared group on one policy path;
- an explicit statement that distinct groups are conditionally independent given the hypothesis for this v0.1 calculation.

Different group labels do not prove statistical independence. They are a model declaration that must be reviewed against sensor dependencies, calibration chains, sample lineage and common causes. A later version should support history-conditioned likelihoods, covariance and dependence-model validation.

## 5. Search limits

To keep enumeration exact and reproducible, the v0.1 implementation caps:
- 8 hypothesis states;
- 8 terminal decision actions;
- 5 candidate measurements;
- 3 outcomes per measurement;
- horizon 3;
- 10,000 recursion nodes.

Oversized cases are rejected with SEARCH_SPACE_LIMIT. There is no silent heuristic fallback. Each experiment and each dependence group may be used at most once per policy path.

## 6. Evidence and cost contract

Every case declares:
- normalized prior distribution and provenance;
- complete terminal action × hypothesis utility table;
- complete outcome × hypothesis likelihood for every measurement;
- likelihood method/source and material/path domain;
- full measurement cost, unit, boundary and source;
- explicit stopping threshold, budget, maximum measurement count and horizon;
- decision owner, utility validation and human-review authority.

All costs use the same declared utility unit, unless a separately audited conversion is introduced by a future version. The likelihood table is P(outcome | state, measurement). v0.1 does not support likelihoods that depend on the complete preceding measurement history; conditional independence groups constrain use.

A MEASURED case containing any SIMULATION_ONLY prior or likelihood is rejected. A validated manifest does not by itself qualify probability calibration, costs or utilities; those remain subject to independent decision review.

## 7. Synthetic adaptive example

The fixture contains:
- 3 hypotheses (fragmentation-dominant, transport restriction, filter loading);
- 3 terminal decision actions;
- 2 passive candidate measurements;
- 2 outcomes for the first measurement and 3 outcomes for the second.

Structural replay evaluates 18 recursion nodes.

The exact policy is:
1. Perform `coarse-passive-observation`.
2. If `A-like`, stop and choose `address-A`.
3. If `B-or-C-like`, perform `B-vs-C-passive-sample`.
4. Select `address-A`, `address-B` or `address-C` from the second observation.

Synthetic values:
- current stopping value: -3.2 benefit points;
- adaptive policy value: 7.6178 benefit points.

These fabricated values demonstrate adaptive branch selection, not physical utility, economic value or a recommended experiment.

## 8. Software benchmark gates

- exact agreement with hand-calculated one-step EVSI for horizon one;
- exact agreement with golden adaptive policy trees;
- zero-information positive-cost measurement never selected;
- perfect-information upper bound never exceeded;
- measurement cost charged on every reachable branch;
- no repeat of a measurement or same dependence group on any path;
- zero-probability outcomes skipped without renormalizing remaining likelihood mass;
- positive prior × positive likelihood underflow rejected;
- action/experiment/outcome permutation leaves the value and deterministic policy unchanged;
- missing/duplicated likelihood cells, nonnormalized priors, utility/cost unit mismatch and incomplete provenance rejected;
- simulation-only likelihoods cannot produce decision-qualified recommendations;
- hard search limits enforced without approximate fallback.

## 9. Physical/model benchmark targets

After a qualified physical measurement study:
- compare adaptive policy to best fixed measurement sequence at equal campaign budget;
- compare net decision utility, total measurement count, campaign cost and time-to-resolution;
- report probability calibration, posterior predictive checks and decision sensitivity to prior/likelihood uncertainty;
- evaluate on held-out material cohorts, paths and campaigns;
- assess whether adaptive measurements reduce unresolved critical fault pairs without increasing nuisance inspections;
- quantify correlated measurement error and common-mode evidence dependencies.

No universal percentage improvement is claimed before independent measurement likelihoods, decision utility and campaign costs exist.

## 10. Integration

- #3917: one-step EVSI reference calculation and golden vectors.
- #3993/#4005: fault-pair observability and unresolved hypothesis pairs.
- #3991: measurement calibration, sampling, timebase and uncertainty qualification.
- #4020/#4026: coupled-fault hypotheses and interaction evidence.
- #4027: this exact finite-horizon adaptive measurement and stopping policy.
- #3927/#3910: stage-resolved material and particle morphology evidence.

## 11. Claim boundary

The planner can return a passive measurement candidate, STOP or HUMAN_REVIEW. The terminal action IDs represent declared decision alternatives; they are not commands. No machine actuation, pressure/airflow adjustment, blockage clearing, cutting-depth selection or worker direction is emitted. A real policy requires independent likelihood validation, an approved utility profile, complete cost accounting, domain qualification and competent engineering review.
