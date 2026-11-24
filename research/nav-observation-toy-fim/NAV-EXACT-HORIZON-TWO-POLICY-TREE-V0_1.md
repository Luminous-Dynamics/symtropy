# Exact horizon-two active-sensing policy tree v0.1

**Status:** isolated offline research kernel, proposed as the exact finite oracle for NAV-SEQUENTIAL-VOI-001 (#6689). It is not a navigation estimator, sensor driver, runtime controller, integrity monitor or safety case.

## Research question

Can a contingent two-step sensing policy reduce terminal navigation decision loss relative to a greedy first observation, when the first result changes both the belief and the best second observation?

A fixed action ranking misses this opportunity. The same second sensor need not be best after every first-stage outcome. The planner must be non-anticipative: the next action is selected from only the observation branch that has actually become controller-visible.

## Model boundary

The v0.1 reference model contains:

- two latent states: inside or outside a declared navigation service envelope;
- a binary terminal choice: declare safe or declare degraded;
- asymmetric false-safe and false-degraded losses;
- a finite candidate action set, capped at 16 for this exhaustive reference;
- three mutually exclusive outcomes per action: qualified observation 0, qualified observation 1, and no observation;
- state-conditional outcome likelihoods, including state-dependent acquisition dropout;
- hard latency, energy and bandwidth budgets;
- a horizon of at most two unique actions.

All probability masses are integer parts-per-billion (PPB). Each action's three outcomes must sum exactly to one billion under each latent state. The evaluator therefore computes an exact policy for the supplied discrete likelihood table, without floating-point posterior arithmetic. This does not imply that the likelihoods are calibrated, complete, independent of future conditions, or physically valid.

The no-observation branch is not silently assigned zero value or treated as missing-at-random. When dropout differs between latent states, observing no result changes the exact posterior. This is a critical negative control for sensing systems that otherwise update beliefs only when a measurement arrives.

## Objective

Let the current unnormalized state masses be \(m_I,m_O\), the false-safe loss be \(L_{FS}\), and the false-degraded loss be \(L_{FD}\). The optimal terminal decision risk is

\[
R(m_I,m_O)=\min(m_O L_{FS},\,m_I L_{FD}).
\]

For each action \(a\), the state-conditional outcome likelihoods are \(\ell_{a,o|I}\) and \(\ell_{a,o|O}\). Branch masses are multiplied by the exact PPB likelihood:

\[
m'_{I,o}=m_I\ell_{a,o|I},\qquad
m'_{O,o}=m_O\ell_{a,o|O}.
\]

At the first stage, the oracle enumerates every feasible first action. For every nonzero outcome branch it evaluates stopping and every feasible remaining second action, subject to the residual resource budget. The second action may differ by branch. The exact selector chooses the first action minimizing the sum of terminal branch losses. A stop decision wins exact ties, so sensing with zero decision value is not rewarded.

The fixed denominator for a two-step policy is \(10^{27}\), corresponding to the initial PPB prior mass and two PPB outcome factors. Stopping early multiplies the unexpanded branch risk by the number of unused probability scales, preserving exact comparability across policy trees. All products and sums are checked in u128 arithmetic.

The comparator is one-step greedy: it selects the first action minimizing immediate terminal decision risk, then receives the same best feasible branch-specific second-step continuation as the exact oracle. This is a deliberately strong comparator. The exact oracle must never have greater expected terminal loss than this greedy-first policy under the same model, candidate set and resource budget.

## Implemented contract

The Rust module navigation_policy_tree.rs provides:

- exact PPB state-conditional likelihood validation;
- exact posterior ratios for every visible first-stage branch;
- explicit state-dependent no-observation updates;
- exact false-safe / false-degraded terminal decision selection;
- exhaustive horizon-two policy-tree enumeration;
- a matched greedy-first comparator;
- unique-action enforcement and candidate-order invariance;
- hard per-dimension resource budgets;
- deterministic action and outcome ordering;
- explicit abstention and stop-on-tie behavior;
- a permanent non-authority marker on every plan.

The synthetic regression fixture is a constructed greedy counterexample. The greedy policy chooses action 1, while the contingent oracle chooses action 3 and then chooses no second action after a no-observation branch, action 1 after observed-zero, and action 2 after observed-one. This is an exact discrete model result, not a physical sensing result.

For the committed fixture (prior outside probability 0.48, false-safe loss 2, false-degraded loss 5), the no-sensing expected loss is 0.96. The greedy-first policy has exact expected loss 0.486828, while the contingent oracle has 0.470988 under the common denominator 10^27. That is a 3.2537% reduction relative to this greedy comparator in this one constructed case. These values are regression-fixture outputs only; they are not measured navigation performance or a general superiority result.

The tests also cover posterior change under state-dependent dropout, impossible branches, asymmetric terminal losses, budget limits, action non-reuse, malformed likelihoods and explicit abstention. These tests are authored in source; only an exact-head CI run can establish that the current repository commit passes them.

## What this does not prove

This kernel does not model:

- more than two latent states or continuous pose;
- shared latent common-cause regimes or arbitrary cross-action dependence;
- changing vehicle/environment state during the sensing horizon;
- stale or delayed out-of-sequence measurements;
- sensor calibration, physical reachability or hardware scheduling;
- uncertainty over the likelihood model itself;
- interval-robust impact weights or outcome-law uncertainty inside the tree;
- a certified protection-level or integrity-risk update;
- a navigation mode change or physical movement.

The first extension is the separate common-cause model in #6708. Do not fake dependence by adding pairwise correlation coefficients to this kernel. The joint state should explicitly include the shared fault regime and must be scored against the independent-sensor approximation as a negative control.

## Matched benchmark ladder

Use the same frozen initial belief, action set, likelihood table, budget, loss matrix and stop rule for every policy:

1. No sensing / stop immediately.
2. Periodic or fixed-order sensing.
3. Random feasible first action.
4. Mutual-information greedy.
5. One-step decision-risk greedy.
6. Greedy first action with branch-specific second-step recourse.
7. Exact horizon-two contingent policy tree (this module).
8. Privileged oracle with evaluator truth, reported as an upper-bound diagnostic only and never exposed to the controller.

The exact finite oracle is the ground-truth comparator for this tiny model, not an algorithm expected to scale to real action spaces. Use it to measure regret and detect approximation errors in larger planners. Publish the full policy tree, branch probabilities, posterior ratios, per-path resource use, terminal decisions and risk numerator/denominator.

## Required measurements

Report:

- expected terminal Bayes decision loss and regret against the exact policy tree;
- false-safe and false-degraded probability/cost separately;
- first-action selection disagreement with the exact tree;
- per-branch second-action selection and stop probability;
- no-observation, invalid and timeout rates when those are represented in later model versions;
- action count, latency, energy, bandwidth and policy-computation time;
- abstention and infeasibility rates;
- performance under state-dependent dropout and asymmetric terminal loss;
- sensitivity to PPB quantization and model misspecification;
- exact replay equality under candidate/outcome permutation.

For larger models, also measure the exact-oracle optimality gap on tractable subsets and lower/upper regret bounds. Do not use aggregate mean error as a substitute for false-safe exposure or tail behavior.

## Research comparators and field datasets

### Active sensing

The 30 October 2026 IEEE early-access paper *Value-Based Active Sensing for Improving Sampling Efficiency* formulates observe-then-plan sensing around expected opportunity loss, with a risk premium for belief-state variability and an ignorance penalty as predictive confidence decays. It reports temperature-alert and CO2-aware office deployments, but evaluates a fixed one-step downstream decision. It is a direct comparator, not a novelty claim. The Symtropy research extension is the exact non-anticipative multi-step policy-tree oracle, with explicit no-observation Bayesian updates, hard multidimensional budgets and action non-reuse.

Krishna, Hu and Jayaraman's ICLR 2025 work *The Value of Sensory Information to a Robot* measures regret when observations are withheld during robot execution. Its finding that sensing is not always task-critical motivates observation-ablation and sensor-dropout campaigns: the goal is not to maximize the number of measurements but to spend sensing budget where it changes the decision.

### UWB environment transfer

Kovaci et al.'s open-access UWB survey was published on 23 November 2026. It emphasizes reproducible protocols, environment/hardware transfer, fusion under measurement loss, and separate reporting of ranging and position tasks. It summarizes results where rejecting NLOS measurements improved positioning in one cross-scenario setting while learned range correction increased error. The empirical campaign must therefore compare NLOS rejection, learned correction, reliability weighting and robust estimation on held-out environments rather than assuming that correction is always superior.

### Candidate datasets

- **MILUV:** 217 minutes across 36 experiments and three quadcopters, with raw UWB timestamps/CIR, stereo and monocular vision, IMUs, magnetometer, height and motion-capture ground truth. Keep UWB-only, UWB+IMU and UWB+vision privilege tracks separate. Dataset payload rights and exact payload hashes remain distinct from the MIT-licensed devkit.
- **TWR-CLOUD-Bern:** released 17 October 2026; 40 DWM3001C nodes, 12 scans across more than 500 m², ranging/reception/protocol statistics, JSON scenarios and an official scorer. Eleven scans support training and one independent scan is held out. Use the official scorer and freeze the exact release version: v6 corrected three training device-to-position assignments, changing 44,280 rows and 191,514 values while retaining the evaluation archive and 148,260 evaluation scenarios.
- **HYMN:** a 2026 benchmark paper evaluates fused UWB/BLE/Wi-Fi range-based positioning and reports that random splits hide a large spatial-generalization penalty. Use reference-point-held-out evaluation, not random sample splits, when comparing learned correction or fusion.

Payload admission must record release/version, source URL, file manifest and cryptographic hashes, rights, schema, coordinate frame, calibration, clock mapping, experiment/robot identity, and train/validation/test split. A parser test is not evidence that a dataset payload has been downloaded, admitted or scored.

## Proposed gates, not achieved results

- 100% agreement with exhaustive enumeration for every tiny finite fixture.
- Exact parity with an independent rational-arithmetic reference implementation.
- Exact-policy risk never worse than greedy-first risk under identical inputs.
- 100% rejection of malformed probability masses, duplicate identities and infeasible paths.
- 100% deterministic replay under candidate and outcome permutations.
- Zero action reuse along any policy path.
- No-observation posterior changes exactly when dropout is state-dependent.
- Explicit abstention where no action improves the terminal decision under the available budgets.
- For field campaigns: report held-out P50/P95/P99 position error, coverage, false-safe exposure, NLOS detection/rejection, environment transfer, observation yield, latency, energy and compute separately.

The field targets must be preregistered after reproducing the reference baselines. Synthetic exactness is not physical navigation integrity, rare-event probability or safety qualification.

## References

- NAV-SEQUENTIAL-VOI-001, issue #6689: https://github.com/Luminous-Dynamics/symtropy/issues/6689
- NAV-COMMON-CAUSE-VOI-001, issue #6708: https://github.com/Luminous-Dynamics/symtropy/issues/6708
- Value-Based Active Sensing for Improving Sampling Efficiency (IEEE, 30 October 2026): https://doi.org/10.1109/TSMC.2026.3745855
- The Value of Sensory Information to a Robot (ICLR 2025): https://mlanthology.org/iclr/2025/krishna2025iclr-value/
- UWB indoor positioning critical survey (23 November 2026): https://doi.org/10.1186/s13638-026-02703-9
- MILUV dataset paper (2026): https://pmc.ncbi.nlm.nih.gov/articles/PMC13552925/
- TWR-CLOUD-Bern dataset, release v6 (17 October 2026): https://zenodo.org/records/23434390
- HYMN multi-technology benchmark (22 September 2026): https://arxiv.org/abs/2609.25835
