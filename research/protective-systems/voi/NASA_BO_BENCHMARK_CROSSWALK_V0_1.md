# NASA Materials Bayesian Optimization Benchmark Crosswalk v0.1

**Owner:** PROTECT-VOI-001 (#5268)  
**Parent:** PROTECTIVE-SYSTEMS-001 (#5246)  
**Status:** reproducibility protocol; no NASA benchmark run or superiority claim performed  
**Prepared:** 15 November 2026

## 1. Why this benchmark is mandatory for B2

NASA/TM-20260001102, *Benchmarking Bayesian Optimization Frameworks and Acquisition Strategies for Materials Discovery and Autonomous Laboratories*, is a directly relevant public benchmark because it includes a hybrid categorical/continuous laminate-composite design problem coupled to micromechanics, in addition to a discrete electrolyte composition problem and a standard continuous analytic objective.

Official NTRS record: https://ntrs.nasa.gov/citations/20260001102  
DOI: https://doi.org/10.64631/ZKVT6687  
Public memorandum PDF: https://ntrs.nasa.gov/api/citations/20260001102/downloads/TM-20260001102_Rework.pdf

The NASA abstract reports the following protocol:
- Five surrogate/framework combinations: Ax Gaussian Processes; BayBE Gaussian Processes; BayBE Monte-Carlo Neural Networks; Lolopy Random Forest; Hyperopt Tree-structured Parzen Estimator (TPE).
- Three tasks: discrete solid-electrolyte composition; hybrid categorical/continuous laminate-composite design with micromechanics; continuous Ishigami analytic function.
- Three acquisition choices: posterior mean, probability of improvement (PI), and expected improvement (EI).
- 100 trials following randomized initial samples.
- Uniform random search as a control.
- Five random seeds per configuration.

The NASA abstract reports BayBE GP + EI reaching at least 95% of the known optimum in the fewest evaluations overall. It also reports RF as competitive or better on some categorical/mixed search spaces at higher computational cost, while posterior-mean-only acquisition can stagnate. These are source-reported results for the declared tasks and protocol; they are not universal BO rankings.

## 2. Baseline matrix to reproduce

| ID | Framework / surrogate | Acquisition | Required search-space coverage |
|---|---|---|---|
| NASA-A1 | Ax Gaussian Process | posterior mean | all reproduced task types supported by the released benchmark |
| NASA-A2 | Ax Gaussian Process | PI | same |
| NASA-A3 | Ax Gaussian Process | EI | same |
| NASA-B1 | BayBE Gaussian Process | posterior mean | categorical, mixed, continuous where original supports |
| NASA-B2 | BayBE Gaussian Process | PI | same |
| NASA-B3 | BayBE Gaussian Process | EI | same |
| NASA-B4 | BayBE Monte-Carlo Neural Network | posterior mean | exact supported task subset |
| NASA-B5 | BayBE Monte-Carlo Neural Network | PI | exact supported task subset |
| NASA-B6 | BayBE Monte-Carlo Neural Network | EI | exact supported task subset |
| NASA-C1 | Lolopy Random Forest | posterior mean | categorical and mixed tasks |
| NASA-C2 | Lolopy Random Forest | PI | categorical and mixed tasks |
| NASA-C3 | Lolopy Random Forest | EI | categorical and mixed tasks |
| NASA-D1 | Hyperopt TPE | source-supported acquisition configuration | original supported tasks; do not invent unavailable acquisition variants |
| NASA-R1 | Uniform random search | random | all tasks |
| NASA-R2 | Repeated fixed-seed initial-design baseline | no adaptive acquisition | diagnostic only; distinguish from random policy if initial designs differ |

The exact task implementation, objective direction, input variables, bounds/levels, initial design size, surrogate hyperparameters, acquisition configuration, optimization direction, seed list and termination condition must be copied from the primary memorandum/code before reproduction. Do not reconstruct missing experimental details from an abstract or this crosswalk.

## 3. Reproduction protocol

### Freeze before running

Pin:
- NASA memorandum revision and SHA-256;
- source-code repository URI/commit and licence;
- Python/Rust/toolchain and package environment actually used by the source implementation;
- exact task source code, input levels, constraints and objective functions;
- initial samples for every seed and method;
- all surrogate and acquisition defaults;
- number and meaning of evaluation calls;
- stopping rule and the exact known optimum/reference value;
- timeout and failed-evaluation policy.

The NTRS abstract says benchmark code is released but the specific code repository/commit and licence have not yet been pinned in this crosswalk. Resolving those is a prerequisite to claiming an exact reproduction. A paper-level protocol approximation must be labelled as such.

### Budget semantics

Use 100 objective-evaluation opportunities after the source-defined initial design, or the exact source budget if the released implementation clarifies a different interpretation. Count every evaluated candidate, including infeasible, failed and repeated evaluations according to the source's original convention. Report separately:
- initial-design evaluations;
- adaptive evaluations;
- total objective calls;
- surrogate fit/acquisition wall time;
- objective/micromechanics wall time;
- end-to-end campaign wall time.

Do not compare one method with more objective calls or more tuning attempts. Separate optimizer compute from experiment/simulation cost.

### Seeds and uncertainty

Use the five source seeds for faithful reproduction. Keep the seed identity paired across methods wherever the source protocol permits. Report every seed, median/mean, dispersion and confidence interval; five seeds are a source-faithful reproduction count, not necessarily enough to establish small superiority differences. Add a separate, larger preregistered replication only after the source result has been reproduced.

### Primary reproduction metrics

- Best observed objective after each evaluation and at termination.
- Evaluations needed to reach 95% of the source-defined known optimum.
- Simple regret at the fixed budget.
- Feasible-candidate yield if the source problem defines constraints.
- Wall time and objective/simulation cost separately.
- Variability across five seeds.

Do not convert the NASA abstract's "≥95% of known optimum in fewest evaluations" into a universal leaderboard claim. Reproduce the full per-task curves and outcomes.

## 4. Luminous extension: beyond the reproduction

Only after the NASA source protocol is reproduced should the protective-design programme add its own research axes.

### E1 — hard-feasibility and abstention

Introduce explicit hard constraints for declared physical/design limits. A candidate that violates a hard floor is infeasible even if its objective value is attractive. Add:
- known-safe and infeasible controls;
- no-feasible-candidate outcomes;
- unknown constraint state and OOD abstention;
- probability of feasible candidate;
- false-feasible rate;
- cost to first confirmed feasible candidate.

### E2 — multiple objectives

Add a frozen multi-objective problem derived from a rights-cleared protective-system task, with separately declared axes such as performance, mass, cost, repair burden and service life. Do not collapse safety floors into a scalar objective. Compare NSGA-II, an appropriate multi-objective BO implementation and Symthaea policies using fixed normalization/reference points and hypervolume/IGD+ only where assumptions are valid.

### E3 — fidelity and model discrepancy

Add low- and high-fidelity objective sources with declared noise, cost, bias and correlation. Include adversarial conditions where:
- low fidelity is informative;
- low fidelity is weakly informative;
- low fidelity is uninformative;
- low fidelity reverses high-fidelity rankings;
- model discrepancy changes across regions.

Compare single-fidelity methods against multi-fidelity BO under equal total information/compute/confirmation budgets. Do not assume additional fidelity layers improve the result.

### E4 — hypothesis discrimination and stopping

Include problems where an experiment can disprove a current favourite or distinguish competing causal hypotheses. Charge all experiments, setup, failed runs, compute, confirmation and delay against the same fully burdened budget. Include the no-experiment action and a calibrated STOP/ABSTAIN outcome.

### E5 — robustness and nonstationarity

Use PROTECT-ROBUST-001 (#5317) scenarios to evaluate fixed-distribution versus shifted-distribution decisions. Report nominal objective, worst-case expected loss, CVaR under declared conditions, hard-floor violations, robust feasibility and recovery separately.

## 5. Proposed B2 acceptance gates

**B2-NASA-R — faithful reproduction**
- The source task and supported method matrix are pinned.
- Reproduced curves and metrics are within a declared tolerance, or deviations are itemized.
- Same source evaluation budget and initial-data access.
- Five source seeds are reproduced, with per-seed outputs retained.

**B2-MATCHED — fair extension**
- Same inputs, objectives, constraints, scenario IDs, information, compute/evaluation and confirmation budgets.
- All methods receive the same tuning budget and held-out-task access.
- Random/stratified, DOE, NSGA-II, NASA BO baselines, and Symthaea policies are included where problem type permits.
- Paired repeated trials, uncertainty intervals and worst-task reporting.
- Failed runs, retries, abstentions, and all human/compute/experiment burden are included.

**B2-CLAIM — superiority**
- At least one preregistered primary endpoint improves materially with uncertainty intervals excluding a practically negligible difference.
- No increase in hard-constraint false-feasible rate or any safety-floor regression.
- Improvement is repeated across held-out problem families and is not explained by more information, compute, tuning, or confirmation.
- Publish negative results and ablations.

## 6. Claim ceiling

A successful NASA task reproduction qualifies the benchmark implementation. A synthetic extension qualifies the specified optimizer/decision policy on those tasks. Neither proves better materials, safe PPE, crashworthiness, building resilience, manufacturing readiness or SOTA physical protection. Those require independent evidence at the relevant physical article and configuration level.

## 7. Research reference

Stuckner, Toma, Goodin, Hearley & Xie, NASA/TM-20260001102 (2026), *Benchmarking Bayesian Optimization Frameworks and Acquisition Strategies for Materials Discovery and Autonomous Laboratories*, NASA Glenn Research Center. DOI: https://doi.org/10.64631/ZKVT6687
