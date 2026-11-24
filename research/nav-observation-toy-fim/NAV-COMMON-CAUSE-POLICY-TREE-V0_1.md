# Persistent common-cause horizon-two policy tree v0.1

**Status:** offline exact finite research kernel for NAV-COMMON-CAUSE-VOI-001 (#6708), stacked on the independent-state policy-tree reference in PR #7250. No physical sensing or navigation authority is exposed.

## Why a separate joint-state model

A different modality is not an independent failure domain. UWB, camera, IMU, BLE, GNSS and map/clock services can share power, timing, geometry, mounting, obstruction, calibration, networking, compute, or common environmental conditions. Marginalizing a persistent shared regime induces dependence between sensor outcomes even when their likelihoods factorize conditional on the complete hidden state.

The model carries the joint latent state:
- navigation condition: inside or outside the declared service envelope;
- common-cause regime: nominal or shared fault.

The four state order is inside/nominal, outside/nominal, inside/shared-fault, outside/shared-fault. The shared regime is mission-persistent in v0.1. Each action supplies one normalized outcome row for each joint state; that row does not change between policy-tree stages.

This is a structural dependency model, not a claim that a real dependency has been discovered. A deployment must derive common-cause identities from qualified dependency evidence.

## Implemented contract

The Rust module navigation_common_cause_policy_tree.rs supplies an exact horizon-two reference with:

- a four-element joint prior normalized exactly to one billion PPB;
- complete three-outcome likelihoods per hidden joint state;
- exact updates of both navigation-state and shared-fault beliefs;
- an explicit no-observation/dropout outcome;
- asymmetric false-safe and false-degraded terminal loss;
- branch-specific second actions and stopping decisions;
- hard latency, energy and bandwidth budgets;
- unique action use, deterministic order, checked u128 arithmetic and abstention;
- a matched greedy-first policy with the same branch-specific second-stage recourse;
- a permanent non-authority marker on every plan.

The terminal navigation decision marginalizes over the shared regime. The posterior report retains the shared-regime probability separately. The probability law assumes actions are conditionally independent given the fixed four-state joint variable. Cross-action residual dependence after conditioning on the declared regime is outside v0.1.

## Exact induced-dependence regression

The test persistent_common_cause_regime_induces_marginal_sensor_dependence constructs a 50/50 persistent regime:

- under nominal conditions, each of two sensors reports outcome 1 with probability 0.1;
- under the shared-fault regime, each reports outcome 1 with probability 0.9.

Conditional independence holds within each regime. After marginalizing over the persistent regime:

\[
P(O_1=1,O_2=1)=0.5(0.1^2)+0.5(0.9^2)=0.41
\]

while the product-of-marginals approximation gives:

\[
P(O_1=1)P(O_2=1)=0.5^2=0.25.
\]

The independent approximation underestimates the probability of the joint outcome by 39% relative to the true 0.41 probability, or the true joint probability is 64% higher than the 0.25 approximation. The regression test checks this inequality with integer arithmetic. These are constructed probabilities, not empirical failure rates.

## Tests authored

Nine tests cover induced marginal dependence, posterior update after a measurement, posterior update after state-dependent dropout, exact-plan versus greedy-first dominance, candidate-order invariance, per-path resource ceilings and no action reuse, invalid prior/likelihood/duplicate-ID rejection, asymmetric terminal decisions, and explicit abstention.

Exact-head CI remains the qualification gate. No test pass is claimed merely because the source has been committed.

## Matched comparator

Freeze the same joint prior, joint-state likelihood table, action set, action budgets and terminal losses. Compare:
1. correct joint-state policy tree;
2. independent-sensor approximation that marginalizes the shared regime separately for each action;
3. one-step greedy under the correct joint model;
4. exhaustive exact horizon-two oracle under the correct joint model;
5. stop/no-sensing baseline.

Report joint outcome probability calibration and divergence from the independent approximation; expected terminal decision loss and exact-oracle regret; false-safe / false-degraded outcomes separately; posterior shared-fault belief calibration; selection and stopping frequencies by observation branch; resource use and abstention; held-out site, hardware, mission and environmental generalization.

The independent approximation is a deliberately adversarial diagnostic baseline, not a production candidate.

## Limitations and next steps

This v0.1 has one binary common-cause regime, two navigation states, fixed action likelihoods and horizon two. It does not cover:
- multiple overlapping or changing common causes;
- residual dependence after conditioning on the common-cause variable;
- persistent uncertain model identity with more than one plausible likelihood table;
- continuous pose, delayed measurements or moving dynamics;
- calibrated field probabilities, integrity risk or safety certification.

The follow-on should compose the persistent model identity in issue #7251 with this joint latent state. A mission-global model must remain the same over a policy tree. If a fault regime may change in time, an explicit transition model is needed rather than branchwise model switching.

## Research grounding

- The UWB critical survey (23 November 2026) emphasizes fusion under measurement loss and warns that learned ranging correction is not uniformly beneficial across scenarios: https://doi.org/10.1186/s13638-026-02703-9
- TWR-CLOUD-Bern v6 (17 October 2026) provides 40 DWM3001C nodes, 12 scans, an independent held-out scan and an official scorer. The held-out scan enables cooperative ranging comparison, but one held-out scan is not broad evidence of environmental generalization: https://zenodo.org/records/23434390
- HYMN has co-located UWB/BLE/Wi-Fi/5G/GNSS at 48 indoor/transition points; its measurement recordings are static at reference locations, not continuous trajectories, so it is suitable for static cross-technology/ranging evaluation but not closed-loop dynamic navigation: https://doi.org/10.1109/ieeedata.2026.3691044
- Non-rectangular robust MDP research identifies tractability and dynamic-consistency costs of persistent cross-state uncertainty: https://doi.org/10.1137/24M1631250 and https://papers.nips.cc/paper_files/paper/2025/hash/372cb7805eaccb2b7eed641271a30eec-Abstract-Conference.html
- ISO/IEC 21134:2026 provides a benchmark framework for integrated indoor localization/tracking with dead reckoning; it is not an integrity certificate: https://www.iso.org/standard/86651.html
