# CIVDYN policy-dependent observability reference v0.1

This isolated, dependency-free Rust crate implements the first executable slice of [CIVDYN-OBSERVATION-POLICY-001 #4750](https://github.com/Luminous-Dynamics/symtropy/issues/4750).

It tests a failure that ordinary model and policy evaluation can miss:

**A policy changes which outcomes are recorded, and the observed records then make the policy appear better or worse than it is.**

## Why this is a separate benchmark

The existing [closed-loop policy benchmark #4688 / PR #4737](https://github.com/Luminous-Dynamics/symtropy/pull/4737) compares action-conditioned policies against true service trajectories. This extension qualifies the evidence channel that would be used to estimate policy value when not every outcome is observed.

The key distinction is:

- a measured zero is an observed outcome;
- a missing outcome is unknown, not zero;
- an observed-case mean describes the observed subset, not automatically the target population;
- a policy can alter the selection mechanism, so missingness can be post-policy and outcome-dependent.

The crate deliberately separates the estimator input (CandidateRecord) from evaluator-only latent outcomes and the true missingness mechanism (EvaluatorRecord).

## Frozen finite-population fixture

Outcomes are rewards/service-value units on [0, 100]. Every policy has four independent units in two beneficiary strata (two high-need, two general). The evaluator knows all outcomes; candidate estimators do not.

### Selective absence of adverse outcomes

| Policy | Latent outcomes | Observed outcomes | True mean | Complete-case mean |
|---|---|---|---:|---:|
| masked_failures | 100, 0, 0, 0 | 100 | 25 | 100 |
| transparent_service | 60, 60, 60, 60 | all four | 60 | 60 |

Complete-case scoring ranks masked_failures first (100 versus 60), even though its true mean is lower (25 versus 60). Its observation coverage is only 25%. The bounds for masked_failures are [25, 100], so the policy ranking is unresolved rather than falsely certified.

### Selective absence of beneficial outcomes

| Policy | Latent outcomes | Observed outcomes | True mean | Missing-as-zero mean |
|---|---|---|---:|---:|
| hidden_successes | 100, 100, 100, 100 | 100 | 100 | 25 |
| steady_service | 60, 60, 60, 60 | all four | 60 | 60 |

Missing-as-zero scoring reverses the true ranking in this second fixture. This is a negative control, not a generally valid estimator.

### MAR positive control

A separate balanced finite fixture has values 20, 20, 80, 80. One unit in each stratum is observed, all units carry a declared positive observation propensity of 0.5, and the Horvitz-Thompson mean is 50, equal to the known finite-population mean. This verifies arithmetic only; it does not establish that a real missingness process is MAR.

### Mixed observation mechanism

A fourth finite fixture makes the population's observation process explicitly mixed: two units carry a declared MAR mechanism and two carry an MNAR mechanism. The evaluator reports the aggregate mechanism as `mixed`, while candidate estimators still receive only unit-level observation rows. The bounds remain [42.5, 92.5] around the true mean of 45; the benchmark does not pretend the MAR propensity values for two units identify the other two units' MNAR outcomes.

## Output contract

The sweep emits a deterministic 32-column CSV over six rows: both policy pairs for the two rank-reversal fixtures, one mixed-mechanism population, and one known-propensity positive control. It records overall and subgroup bounds/coverage, true subgroup means and inequality gap, reason-specific missingness counts, propensity coverage/range, interval comparison, and worst-case regret. The evaluator-only truth is reported solely for qualification and never passed to an estimator.

## Estimators and qualification

For outcome support [L, U], observed weighted sum S, total population weight W, and missing weight M, the no-assumption finite-population bounds are:

`[(S + M L) / W, (S + M U) / W]`

The implementation reports these bounds without replacing them with a midpoint. Strict interval dominance is required for a robust pairwise ranking. Overlapping intervals produce an unresolved comparison, not a forced winner.

The known-propensity estimator is an explicit positive-control path and rejects unknown/MNAR assumptions, absent propensities and non-positive observation support. It must not be used to make MNAR data appear identified.

## Included controls

- Missing and observed-zero states are disjoint in the type contract.
- Missingness reasons are explicit and retained in counts.
- Unit IDs are unique per policy; policy IDs cannot be mixed in one estimate.
- Every missing unit remains in the population denominator.
- Outcome support and population weights are validated.
- An observed value that disagrees with evaluator-only latent truth is rejected.
- Mixed unit-level observation mechanisms remain visible as a mixed population; they are not forced into a single global label.
- No hidden outcome or true-mechanism field is passed to candidate estimators.
- Complete-case and missing-as-zero rank-reversal tests run in opposite selection directions.
- All-missing policies return the full outcome-support interval and no complete-case estimate.
- Per-stratum true values, coverage and identified intervals are reported separately. The sweep also emits reason-specific missingness counts and known/unknown propensity counts and extrema; it does not silently pool away subgroup coverage.
- Pairwise robust regret is bounded from interval endpoints rather than point midpoints.

## Research and industry comparison

The closest current methodological comparator located for the sequential extension is Wei, Qu & Miao, **Off-Policy Evaluation for Missingness-Aware Policies in MDPs with Rewards Missing Not at Random**, ICML 2026 / PMLR 306. It studies finite-horizon MDP reward-dependent missingness, uses future states as shadow variables and a bridge-function/min-max estimator, and evaluates on simulated and MIMIC-III Sepsis data:

https://proceedings.mlr.press/v306/wei26v.html

This v0.1 crate does **not** reproduce that estimator. It provides a much smaller finite-population arithmetic/identification control against which a future sequence-level implementation can be tested.

A second, complementary planning comparator is Wendland et al., **Missingness-MDPs: Bridging the Theory of Missing Data and POMDPs**, IJCAI 2026 (pp. 6217–6226). Unlike reward-MNAR off-policy evaluation, this work treats missing state features as a state-dependent observation function and learns it under MCAR, simple-MAR and selected MNAR structural assumptions before solving the approximate POMDP. Its ICU and Predator benchmarks include identifiable and unidentifiable MNAR cases, dependent feature missingness, AMCAR/AsMAR/AIMI methods, and comparisons against SARSOP optimal/prior policies, POMCP and PPO with/without memory. The paper reports average/worst total variation of the learned missingness function and normalized true policy value over 20 independent runs. Its [public reference implementation](https://github.com/ai-fm/missingness-mdps) is MIT-licensed.

The ICML OPE track and IJCAI planning track are different estimands: reward observation versus state-feature observation. Do not merge their headline scores. Reproduce both paper protocols separately before adding CIVDYN's policy-induced mixed-mechanism and subgroup challenge families.

The ICML paper's [author implementation is ShadOPE](https://github.com/NAIVlab/ShadOPE), whose README lists ProxFQE, NaiveFQE, WeightedFQE, ImputeFQE, SCOPE and a neural bridge/AGMM route. The current requirements pin Python 3.11-era dependencies and PyTorch 2.9.1+CUDA 12.6. The root `LICENSE` path was not found during this inspection; do not vendor or redistribute that code until its terms are verified. Keep it as a separately pinned external reference runner.

Other relevant 2026 work includes Rubinstein et al. on bounds under an unknown mixture of informative and non-informative missingness (Journal of Causal Inference, 2026):

https://pmc.ncbi.nlm.nih.gov/articles/PMC13455597/

Daly-Grafstein & Gustafson on partial identification with categorical outcomes and nonignorable missingness (first published 26 January 2026):

https://doi.org/10.1002/cjs.70037

For population realism, SSDataBench (PNAS, 12 May 2026) evaluates marginal distributions, bivariate associations, multivariate outcomes, event sequences and sequence/covariate associations, and highlights heterogeneity compression:

https://doi.org/10.1073/pnas.2538145123

NIST community resilience guidance connects infrastructure performance to social/economic functions, service continuity and recovery:

https://www.nist.gov/programs-projects/development-and-implementation-community-resilience-planning-guides

These works are methodological reference points, not a directly comparable leaderboard. CIVDYN should compete on the rigor and completeness of its evaluation protocol, not claim to beat specialized causal estimators on a four-unit toy corpus.

## Next benchmark targets

1. Reproduce the IJCAI 2026 miss-MDP protocol (AMCAR/AsMAR/AIMI, ICU/Predator, sample-size curves, average/worst missingness-function TV and true policy value) in a separately pinned reference runner.
2. Reproduce the ICML 2026 ShadOPE protocol (its reward-MNAR model, bridge/min-max estimator, FQE variants and published simulation/MIMIC-III evaluations) without importing unlicensed source into this crate.
3. Add sequence-level CIVDYN trajectories with policy, state, action, reward, observation indicator and observation propensity on every time step.
4. Add MCAR, MAR, identifiable/unidentifiable MNAR, mixed-mechanism, dependent-feature outage, positivity failure and policy-induced mechanism-shift challenge families.
5. Compare shadow-variable/bridge estimators, AMCAR/AsMAR/AIMI, inverse-propensity baselines, POMCP/PPO planning, complete-case and zero-imputation controls, and assumption-free bounds on their respective tasks.
6. Use paired held-out worlds with measured policy value, regret, bound coverage, interval width, subgroup support, total-variation fit and compute.
7. Empirical South African water/energy/municipal evaluation only after the #4560/#4601 source, rights, measurement-equivalence and uncertainty gates are satisfied.

## Claim ceiling

This is deterministic synthetic protocol evidence only. It does not identify real-world causal effects, reproduce the ICML 2026 estimator, establish field policy effectiveness, validate South African service data, or demonstrate SOTA superiority. The exact expected result is a negative control: point estimators fail under selective observation, while honest bounds preserve uncertainty.
