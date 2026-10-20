# EV factory paired intervention effect-of-effects v0.1

**Issue:** [EV-FACTORY-INTERVENTION-BENCH-002 #2421](https://github.com/Luminous-Dynamics/symtropy/issues/2421)  
**Implementation branch:** `research/ev-factory-paired-intervention-effects-v0-1`  
**Status:** inference over synthetic discrete-event simulation replications

## Problem corrected

The earlier ranking evaluator screened the top two station interventions by asking whether their individual 95% confidence intervals overlapped. That is only descriptive; it is not the right uncertainty calculation for comparing two interventions whose outcomes share the same baseline and common random inputs.

## Paired estimator

For each seed (s) and intervention (i), the simulator emits a row with the scenario ID, intervention station, good units/hour, same-seed baseline rate and paired gain:

`g_i(s) = Y_i(s) - Y_0(s)`

The evaluator validates that each seed has one baseline and all five interventions, the baseline rate is identical across rows for that seed, and each stored gain matches its output-minus-baseline arithmetic. Missing/duplicate seeds, scenario rows, station mismatches or mismatched deltas are rejected.

For intervention pair (A,B), the comparison uses:

`d_{A,B}(s) = g_A(s) - g_B(s)`

The report summarizes these **within-seed difference-of-differences** values across the declared 20 seed replications, including sample standard deviation, empirical quantiles and a two-sided 95% Student-`t` interval for the mean paired difference. It does not estimate this interval by subtracting the lower/upper bounds from two marginal confidence intervals.

The probability-labelled fields in the JSON are deliberately defined as the empirical share of seeds for which the paired effect exceeds the practical margin. They are not Bayesian posterior probabilities and do not make assumptions about future real-world frequencies.

## Decision rule and practical-equivalence margin

The initial evaluator uses a fixed, internal `0.05 good units/hour` practical-equivalence margin for the synthetic example:

- A is meaningfully better when the full 95% paired interval for (d_{A,B}) is above +0.05.
- B is meaningfully better when the full interval is below -0.05.
- Interventions are practically equivalent under this rule only when the interval lies wholly inside [-0.05, +0.05].
- Otherwise, the pair remains inconclusive.

A candidate is reported as robust-best only if it is meaningfully better than every other candidate under the pairwise interval rule. If even one pair is inconclusive or equivalent, the evaluator returns no unique robust best. The margin is a declared benchmark setting—not an industry standard—and must be sensitivity-tested or pre-registered before any future measured-data evaluation.

## Reproducibility and guards

- The source simulation report is hashed from the exact raw bytes by the ranking executable.
- Per-seed outcomes are checked for one unique `(seed, scenario_id)` combination.
- Seed lists are explicit, positive, unique and complete.
- Each stored per-seed intervention gain is cross-checked against the same-seed baseline and output.
- Repeated fixed inputs should yield byte-identical report output.
- Release status remains false regardless of a narrow interval or a robust-best result.

## Interpretation

The intervals quantify only output variation under the current toy model and fixed synthetic seed catalog. They omit model-form uncertainty, parameter calibration uncertainty, sensor uncertainty, structural misspecification and changes in a physical factory. A robust-best station therefore means only "separated under these simulation assumptions and this predeclared margin." It does not mean best site, best factory layout, positive ROI, safe machinery or an authorized purchase.

The next robustness step is a **scenario-family frontier**: vary product mix, shift calendar, supplier availability, process-cycle uncertainty, buffer capacity, changeover duration, repair behavior and utilities, then find interventions whose advantage persists across plausible regimes instead of just across random seeds for one fixture. Separate random variability from scenario/model uncertainty.

No real manufacturing records were collected, no suppliers/plants/integrators/landlords/agencies were contacted, and no quotes were requested.
