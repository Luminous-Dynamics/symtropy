# Selective prediction risk/coverage and false-feasible protocol v0.1

## Purpose

A design-support model should be allowed to abstain when its prediction is unreliable. This protocol evaluates whether its uncertainty ranking helps remove risky candidates before they enter a design shortlist. It complements interval calibration: interval calibration measures coverage/sharpness of stated prediction ranges; selective risk/coverage measures the utility of refusing to rank the most uncertain samples.

## Required blind sample fields

Each blind sample carries:
- task, sample, model, geometry-family and flow-regime IDs;
- a predeclared constraint identifier;
- blind_confirmation split label;
- in-domain/out-of-domain status;
- non-negative uncertainty score;
- point prediction and observed value;
- predicted feasibility and observed feasibility under the same constraint definition.

The audit rejects non-blind rows, duplicate model/sample rows, negative or non-finite uncertainty, non-finite values, inconsistent observed truth across models, and unpaired sample sets across compared models. Grouped model comparison is performed separately by geometry family, flow regime, constraint and domain status.

## Curve construction

Within each model/stratum, samples are sorted by uncertainty ascending. All samples tied at a threshold are accepted together. The remaining high-uncertainty samples are interpreted as abstentions. For each distinct threshold report:
- accepted sample count and retained coverage fraction;
- selective point MAE and RMSE among accepted samples;
- false-feasible count: predicted feasible while observed infeasible;
- false-feasible rate among all accepted samples;
- false-feasible rate among accepted samples predicted feasible;
- missed-feasible count: predicted infeasible while observed feasible.

Two areas are computed by trapezoidal integration from zero to full coverage:
- AURC-MAE: area under selective MAE versus coverage;
- AURC-FF: area under false-feasible rate versus coverage.

Lower areas indicate that low-uncertainty samples are safer/more accurate to retain. Always publish the full curve: a low area obtained by rejecting nearly everything is not a successful policy. Compare curves alongside retained coverage at fixed risk limits and the full-coverage baseline.

## Leakage and comparability requirements

1. Only blind_confirmation rows are admitted.
2. All compared models must cover exactly the same sample IDs within each task/geometry/flow/constraint/domain stratum.
3. Observed numerical values and observed feasibility labels must match for the same physical sample across models.
4. Predicted feasibility may differ across models; the observed constraint truth may not.
5. Report in-domain and out-of-domain curves separately.
6. Constraint definitions and thresholds must be fixed before evaluation and encoded in constraint_id.
7. Never describe the area metrics as failure probabilities under arbitrary distribution shift.

## Synthetic known-answer validation

The synthetic fixture creates two uncertainty rankings over the same 20 physical samples, in two domain strata. One ranking is proportional to known prediction error; the other reverses it. One false-feasible candidate is assigned the largest true error. The known-answer target is that the error-ranked uncertainty has lower AURC-MAE and AURC-FF in both in-domain and OOD strata. An independent stdlib Python implementation recomputes all curve points and areas.

This proves calculation and ranking-audit behavior only; it does not validate a model, aerodynamic feasibility constraint or safety decision.

## Benchmark targets

- 100% same-sample pairing across compared models and domain strata;
- 100% traceability from each curve point to the blind sample records;
- zero accepted rows from calibration/training splits;
- no model can improve its score by silently dropping a sample;
- lower AURC-MAE and AURC-FF at a useful retained coverage;
- predeclare a maximum false-feasible rate at operationally relevant coverage levels;
- report in-domain and OOD performance separately, including all sample counts.

## References

- ShapeBench 2026: multi-task optimizer rankings vary by task, reinforcing matched benchmarks rather than one-case claims: https://arxiv.org/abs/2605.20763
- Gneiting and Raftery (2007), proper scoring rules: https://doi.org/10.1198/016214506000001437
- Conformal prediction guarantees and distribution-shift caveats: https://journals.sagepub.com/doi/10.1177/25152459251380452
