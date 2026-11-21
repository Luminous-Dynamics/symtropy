# Robust decision-valued sensing — prototype v0.1

**Focused issue:** [NAV-ROBUST-VOI-001 #6663](https://github.com/Luminous-Dynamics/symtropy/issues/6663)  
**Parent:** [NAV-DECISION-001 #6645](https://github.com/Luminous-Dynamics/symtropy/issues/6645)  
**Status:** isolated finite-scenario research implementation; no physical qualification.

## Why this exists

A nominal value-of-information policy assumes its observation and acquisition models are correct. In navigation, those assumptions can fail because of changing occlusion, NLOS propagation, sensor aging, environmental shift, stale calibration or incomplete common-mode inventories.

The direct 2026 comparator is *Value-Based Active Sensing for Improving Sampling Efficiency*, IEEE Transactions on Systems, Man, and Cybernetics: Systems, early access 30 October 2026, DOI 10.1109/TSMC.2026.3745855. It evaluates observe-then-plan sensing with opportunity-loss value, a risk premium for variability across belief states and an ignorance penalty for decaying model confidence. Its temperature-alert and CO2-aware office deployments are not navigation tests. We must treat that paper as a direct method baseline; value-based sensing itself is not our novelty claim.

## Implemented calculation

For each candidate action, a finite set of frozen scenarios supplies:
- outcome likelihoods under each latent state;
- state-dependent acquisition success/failure probabilities;
- the same action identity and resource costs.

Each scenario is evaluated using the exact one-step Bayesian decision oracle, including informative or uninformative missingness and the selected asymmetric loss.

The prototype then reports:

[
\bar V = \frac{1}{N}\sum_i V_i
]

[
D^- = \sqrt{\frac{1}{N}\sum_i \max(\bar V - V_i,0)^2}
]

[
P_{risk}=\lambda D^-
]

[
P_{ignorance}=\kappa(1-c)R_0
]

[
V_{adjusted}=\bar V-P_{risk}-P_{ignorance}
]

where:
- \(V_i\) is the exact one-step decision value under scenario \(i\);
- \(\lambda\) is an explicit risk-aversion coefficient;
- \(c\) is a separately supplied model-confidence value in [0,1];
- \(\kappa\) is an ignorance-penalty scale;
- \(R_0\) is the baseline Bayes risk under the chosen loss.

The finite-scenario mean is a descriptive equal-weight average, not a calibrated Bayesian model average. The risk premium and ignorance penalty are separately reported. The API also reports minimum scenario value, downside deviation and all scenario-level values.

## Fail-closed properties

- Scenario IDs must be unique and nonempty.
- Every scenario must describe the same action ID and resource cost.
- Candidate actions must be evaluated against identical scenario-ID sets.
- Non-finite/negative risk aversion or ignorance scale is rejected.
- Model confidence outside [0,1] is rejected.
- Infeasible actions cannot be selected.
- Scenario ordering does not alter the result.

## Limitations

This is a finite scenario adjustment, not a continuous uncertainty bound, confidence region or safety certificate. A model-confidence value must come from a distinct calibration/monitoring process; this code does not calculate it and must not obtain it from evaluator-only truth. The scenario family must be frozen before evaluation. An omitted failure mode is not bounded by this method.

The downside-deviation penalty is a transparent navigation-specific research hypothesis, not a reproduction of the IEEE comparator's exact algorithm. A paper-faithful comparator implementation needs its full method, equations, artifacts, tuning protocol and matched evaluation conditions.

## Next evaluation

Compare periodic, random, greedy mutual information, greedy FIM, nominal decision value, finite-scenario minimum, risk-premium/ignorance-adjusted decision value, receding-horizon planning and a privileged oracle. Freeze initial beliefs, candidate geometry, acquisition models, model-scenario family, resource budgets, stopping rules and truth manifests. Report decision regret, false-safe/false-degraded exposure, correct abstention, acquisition yield, latency, energy and compute separately.
