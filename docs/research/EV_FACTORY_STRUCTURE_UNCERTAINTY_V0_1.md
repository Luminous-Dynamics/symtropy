# EV Factory Structural Model Uncertainty v0.1

## Purpose

This benchmark checks whether a factory investment recommendation survives changes to the *structure* of the response model. It is deliberately separate from:

- coefficient intervals within one structural model (EV Factory Model Uncertainty v0.1);
- statistical uncertainty in repeated simulator replications (ranking-and-selection / adaptive allocation);
- calibration of prediction residuals against an independent holdout;
- validation of event traces collected from a physical factory.

The evaluator is a finite action-by-model decision oracle. It does not run a discrete-event simulation and does not claim that its synthetic effects represent a real plant.

## Frozen example

The fixture contains five actions, six explicitly described model families, and all 30 action × family outcomes. Four resource constraints are checked independently: capital budget, staffed labor, active power and floor area. An action that violates a resource constraint is not eligible for either robust selector, regardless of its reported gain.

The effect values are synthetic and designed to force an understandable decision conflict:

- The reference model (independent equipment faults) prefers `add-eol-test`, with a gain of 2.2 good units/hour.
- The maximin selector chooses `extend-buffer`, whose worst-family gain is 0.8 good units/hour.
- The minimax-regret selector chooses `improve-yield-rework`, whose maximum regret across the six model families is 0.8 good units/hour.
- The model-specific winners change from end-of-line capacity to buffering and then to yield/rework improvement as the assumed mechanism changes.

These numbers validate the decision logic only. They are *not* outputs of Symtropy's factory DES, measurements, forecasts, calibrated intervals or investment advice. We intentionally do not assign probabilities to the six families and do not count them as six IID replications.

## Decision rules

For action (a), family (m), and gain (g(a,m)):

- **Reference-model optimum:** maximize (g(a,m_0)) under the explicitly named reference structure.
- **Maximin:** maximize (min_m g(a,m)).
- **Minimax regret:** minimize (max_m[max_{b in A}g(b,m)-g(a,m)]), with every inner and outer maximization restricted to resource-feasible actions.
- **Pairwise ranking reversal:** count a reversal when at least one declared family prefers action (a) to (b) and another strictly prefers (b) to (a).
- **Leave-one-family-out:** recompute both robust selectors after omitting each family one at a time. This is sensitivity to model-set composition, not a confidence interval.

All choices use deterministic lexical tie-breaking. The exact fixture bytes are SHA-256-bound in the report. Row permutations may change that provenance digest but must not change score tables or decisions. Unknown identifiers, duplicate or missing outcome cells, invalid numeric values and invalid resource definitions fail closed.

## Research context and industry comparison

There is no defensible universal leaderboard that directly compares a small synthetic action-by-model oracle with an industrial digital-twin platform. Comparisons should be staged and matched to the subproblem:

1. **Decision logic:** exact finite oracle, hand-calculated controls, greedy/best-single and nominal reference-model baselines on the same action catalogue.
2. **Simulation optimization:** equal simulator-call budgets, common random streams where appropriate, predeclared termination and correct-selection/regret metrics. The simulation-optimization literature distinguishes finite-alternative ranking-and-selection from high-dimensional search; those settings should not be conflated.
3. **Model credibility:** held-out trace/event-sequence agreement, performance-indicator residuals and uncertainty coverage, with model validation kept distinct from optimization success.
4. **Field performance:** only rights-cleared measured traces and independently evaluated outcomes can support plant accuracy, ROI or production claims.

Relevant research includes the 2025 review of large-scale simulation optimization, the 2026 lab-scale study of online discrete-event digital-twin validation/calibration through event sequences and performance indicators, and NIST's October 2026 risk-proportionate treatment of computational model credibility and VVUQ. These establish relevant methods and gaps, not scores directly comparable to this fixture.

- Fan et al. (2025), *Review of Large-Scale Simulation Optimization*: https://doi.org/10.1007/s40305-025-00599-8
- Lugaresi, Dhore & Matta (2026), *Joint online validation and parameter calibration within a digital twin framework*: https://doi.org/10.1016/j.procir.2026.03.284
- Srinivasan (2026), NIST AMS 100-85, *On Methods of Uncertainty Quantification in Computational Modeling and Simulation*: https://doi.org/10.6028/NIST.AMS.100-85
- Yoon & Bekker (2017), *Single- and Multi-Objective Ranking and Selection Procedures in Simulation*: https://doi.org/10.7166/28-2-1732

## Gates and next experiments

For this synthetic v0.1:

- 100% exact agreement with the hand-computed reference decisions;
- 100% coverage of expected action/model cells and rejection of defined malformed fixtures;
- hard resource constraints checked independently of the reported gains;
- deterministic decision tables under row permutations;
- explicit ranking reversals and leave-one-family-out sensitivity in the designed fixture;
- no implicit family probabilities, no IID-family inference and no deployment/release authorization.

The next meaningful extension is to generate each model family's outcome table from the same frozen exogenous disturbance streams in the actual DES, with independent simulator-verification checks. Compare point estimates and robust selectors on hidden structural families and adversarial held-out corners; record cost per evaluation, median / tail regret, ranking reversals, false-robust-winner rate and constraint violations. Tune neither family weights nor decision thresholds on the final holdout. If the robust policy fails, report the failure and its regime rather than relaxing gates.

This benchmark must not be described as industry SOTA or as evidence of a real factory improvement. Its current contribution is a reproducible, fail-closed test harness for exposing model-structure dependence in decisions.
