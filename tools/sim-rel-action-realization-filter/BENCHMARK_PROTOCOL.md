# SIM-REL downstream decision-value benchmark protocol

Status: protocol specification; no benchmark result is claimed by this document.

This benchmark compares sequential experiment-selection systems on a common
interface: the same public observations, action-realization kernels, experiment
costs, terminal loss tensor, finite horizon, risk budgets and compute allowance.
The primary outcome is downstream decision quality under model shift, not raw
information gain.

## 1. Evaluated methods

Run these methods as separate baselines and preserve each method's native
objective and tuning procedure:

1. **Stop / fixed design:** terminal Bayes decision with no additional
   experiment; fixed command order where the task supports it.
2. **Random feasible:** random admissible command selection under the same
   resource and safety constraints.
3. **Myopic EIG:** one-step expected information gain; also report cost-normalized
   EIG. EIG is a diagnostic acquisition baseline, not the primary objective.
4. **EGIG / inferential-error penalty / model-error penalty (Callahan &
   Catanach, AISTATS 2026):** implement the paper's distinct robust utility
   criteria where their inferential and model-misspecification assumptions can
   be matched. Keep them as separate baselines and ablations; do not collapse
   them into one generic "robust EIG" score. Score every selected design using
   the shared downstream decision-loss endpoint.
5. **SIM-REL-027 nominal DP:** exact finite-horizon downstream decision-value
   optimizer using one nominal complete kernel. Replay its policy in every
   shifted test world without allowing it to see world identity.
6. **SIM-REL-028 finite-family minimax:** exact shared-policy planner minimizing
   worst-case cost plus terminal loss over the declared worlds.
7. **SIM-REL-028 TV ambiguity:** exact total-variation robust expectation over
   nominal categorical world weights, with worst-case hazard and abstention
   constraints retained over the full declared family. Calibrate the TV radius
   on a separate calibration split and preregister it.
8. **Robust Bayesian Decision Making under Adversarial Uncertainty (UAI 2026):**
   implement or obtain the authors' sequential adversarial decision-aware
   acquisition baseline. Match its adversarial-variable family where possible,
   then score its selected policy under the common downstream loss. This is the
   closest current decision-stability comparator and must not be represented as
   an EIG-only baseline.
9. **Maximin robust Bayesian experimental design (2026):** include the
   adversarial Sibson-alpha mutual-information/PAC-Bayes policy baseline where
   its inference and model-family assumptions can be matched. Retain its own
   robust information objective as a secondary diagnostic.
10. **COPEx:** constrained multi-step Bayesian experimental design with online
   scenario-tree lookahead. Preserve its original informative-design objective;
   evaluate its chosen sequence using our common downstream loss and risk
   metrics.
11. **R-IDeA:** include informativeness-only, representativeness-only,
   informativeness-plus-representativeness, and full
   representativeness/informativeness/error-de-amplification ablations.
12. **Generalized total-variation DRO (Fontem & Ji, EJOR 2026):** compare
   a sample-size-, support-, confidence- and cost-sensitive ambiguity set
   against the simple finite-support TV ball. Re-derive its guarantees for our
   sequential per-world decision-loss setting; do not transplant its newsvendor
   or portfolio guarantee without proving the assumptions and mapping.
13. **Robust sequential experimental design:** use its published worst-case
   estimation-risk objective as a comparator where the task supports its
   assumptions. Also report a downstream-decision evaluation of its selected
   designs.
14. **Sequential conformal calibration under recursive selection: compare a
   fresh-fold stagewise calibration policy, including its prefix-coverage and
   abstention tradeoffs, against naive survivor reuse and Bonferroni-style
   allocation. Only claim its finite-sample coverage when the paper's
   exchangeability, predictable-refitting and fresh-fold assumptions are met.

Do not silently retune external baselines on our sealed test worlds. Record
baseline source revision, parameters, training data, tool versions and compute.

## 2. Task families and split design

### A. Exact finite-state oracle suite

Use fully specified small kernels where exhaustive policy-tree enumeration is
tractable. Vary:
- number of mechanism hypotheses and latent process/sensor/apparatus states;
- observation overlap and unequal likelihoods between persistent worlds;
- measurement inversion, sensor drift and missing/readback telemetry;
- full, partial and failed action realization;
- apparatus degradation and state-dependent hazard;
- experiment costs, decision-loss asymmetry and abstention penalties;
- horizon and the number of plausible complete model worlds.

For every task, compare the optimized frontier against a non-memoized exhaustive
shared-policy oracle. Independently replay all returned policies from raw path
mass. Keep the oracle domain small enough that the full policy space is
enumerated rather than sampled.

### B. Semi-synthetic shift suite

Start from documented process families and generate controlled shifts not used
by policy selection:
- sensor calibration offsets and sign inversion;
- heteroscedastic and state-dependent measurement noise;
- partial execution and state-dependent dropped actions;
- stale, delayed or contradictory readbacks;
- persistent apparatus backlash, wear and degradation;
- transition-model misspecification and rare response modes;
- multi-factor combinations, including shifts that only emerge after several
  interventions.

Split by generator family and apparatus signature, never by random rows alone.
A shift family used to design the falsifier or calibrate the TV radius cannot
also serve as a sealed test family.

### C. Real workflow / agent transfer

Use public laboratory benchmarks only through their documented interfaces.
Autonomous Science Bench evaluates the complete model-agent-tools-harness loop
and separately tests discovery and learning on six pilot tasks; its first
leaderboard has three trials per task, so those rankings are pilot context,
not statistical proof. EmbodiedLab contributes apparatus interaction,
instrument-reading evidence and final bench-state evaluation. SafeLab-style
trajectory-level safety metrics should be used where action safety is part of
the task. A simulator or policy-kernel result is not a substitute for this
real-workflow evaluation.

## 3. Metrics

### Primary endpoints

- Mean downstream terminal decision loss at the campaign horizon.
- Worst-case expected cost plus terminal loss over declared test worlds.
- Held-out shift-family worst-case loss and task-family CVaR.
- Regret to the exact per-world oracle where a per-world oracle is available;
  report the oracle as a non-deployable diagnostic, not a fair policy baseline.

### Safety and decision integrity

- Campaign hazard probability and unsafe-success frequency.
- False-decision / false-claim frequency under the task's declared loss.
- Abstention and escalation frequency.
- Unsupported observation / out-of-family trace detection rate.
- Worst subgroup hazard, loss and abstention by sensor and apparatus regime.

### Model and execution diagnostics

- Posterior predictive log score and Brier score where ground truth is known.
- Calibration coverage/error overall and by apparatus, sensor and shift group.
- Command delivery, partial execution, readback agreement and missing-telemetry
  rates.
- Final physical/bench state, episode-grounded evidence and protocol violations.

### Efficiency

- Experiment count and measured resource cost.
- Wall-clock time, peak memory, model/query budget and generated policy nodes.
- Planner/frontier size and exact-oracle resource use.
- Cost per successful decision and cost per unit reduction in held-out loss.

EIG, entropy reduction and posterior concentration remain useful diagnostics;
none is allowed to replace the downstream decision endpoint.

## 4. Statistical protocol

- Freeze task generators, terminal losses, baseline versions, costs, risk
  budgets, compute caps and analysis code before opening sealed test results.
- Use paired task instances/seeds across all feasible methods.
- Split by task family, source, ligand/material family or apparatus signature
  wherever those groupings are scientifically meaningful.
- Use task-family bootstrap or paired randomization intervals; do not treat
  correlated trials from one family as independent samples.
- Publish every seed, failed run, timeout, unsupported trace and safety event.
  No post-hoc seed deletion or best-of-many reporting.
- Report all method configurations, total compute and elapsed time.
- Select a trial count using a pre-study power calculation for a preregistered
  minimum detectable effect; the 6-task/3-trial pilot leaderboard is too small
  to settle close comparisons.
- Calibrate the TV radius only on a dedicated calibration split. Report
  calibration coverage by subgroup and sensitivity to the radius; do not select
  the radius by maximizing sealed-test performance.
- For adaptive multi-stage calibration, do not reuse observations that informed
  the selection/refitting step as though they were exchangeable fresh scores.
  Use disjoint fresh folds per stage or a separately justified valid reuse
  scheme. Report prefix coverage and the price of freshness (calibration
  budget, abstention / infinite thresholds and achieved risk). Sequential
  conformal's fresh-fold guarantee is not a general protection against arbitrary
  nonstationary physical shifts; its sampling assumptions must be stated.
- Evaluate the falsifier on its own training-generator suite and a disjoint
  sealed generator suite to measure adversarial-search overfit.

## 5. Preregistered research targets

These are targets for future experiments, not achieved results:

- At least 10% lower held-out worst-case downstream loss than nominal planning
  at matched experiment cost and worst-case hazard/abstention budgets.
- No reduction in subgroup coverage or increase in unsafe-success frequency.
- Exact agreement with exhaustive shared-policy frontiers on every qualifying
  tiny-state task.
- Exact equality between radius-zero TV selection and nominal mixture loss,
  and between radius-one TV selection and finite-family minimax loss.
- Independent raw-mass replay equality for every reported policy receipt.
- Explicit detection/abstention on traces outside the declared finite family;
  no automatic normalization or silent model switching.

A result is a supported improvement only when the paired held-out interval,
risk constraints and compute accounting support it. A nominal score advantage
alone is not evidence of robustness or scientific progress.

## 6. External reference baselines

- On the Misinformation in a Statistical Experiment, AISTATS 2026:
  https://proceedings.mlr.press/v300/callahan26a.html
- COPEx, ICML 2026: https://proceedings.mlr.press/v306/guo26g.html
- Robust Bayesian Decision Making under Adversarial Uncertainty, UAI 2026:
  https://proceedings.mlr.press/v337/harikumar26a.html
- Maximin Robust Bayesian Experimental Design, 2026:
  https://arxiv.org/abs/2603.14094
- Distributionally Robust Optimization with Generalized Total Variation
  Ambiguity Sets, EJOR 2026:
  https://doi.org/10.1016/j.ejor.2025.09.024
- Sequential Conformal Calibration under Recursive Selection, 2026:
  https://arxiv.org/abs/2610.12937
- R-IDeA, AISTATS 2026: https://proceedings.mlr.press/v300/tang26d.html
- Robust Sequential Experimental Design for A/B Testing, ICML 2026:
  https://proceedings.mlr.press/v306/wen26i.html
- Autonomous Science Bench 0.1: https://www.autonomous-science-bench.ai/
- EmbodiedLab: https://arxiv.org/abs/2610.35385
- SafeLab, ICML 2026: https://proceedings.mlr.press/v306/bai26e.html

These works optimize different objectives and operate at different levels. The
fair comparison is not “our score versus their paper score”; it is a matched
task-level evaluation under common evidence, costs, safety constraints and
evaluation splits, with each method's native objective recorded.
