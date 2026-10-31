# Prediction interval calibration and OOD stress protocol v0.1

## Purpose and evidence ceiling

This protocol evaluates already-produced predictive intervals on a sealed blind-confirmation set. It does not fit calibration parameters, provide a future coverage guarantee, or authenticate source observations. It separates calibration data from blind evaluation and preserves geometry-family, flow-regime and domain-status strata.

The key benchmark is not merely whether a model reports confidence. It is whether stated nominal interval coverage matches observed coverage on unseen cases, how wide those intervals must be, and whether coverage degrades under out-of-domain conditions.

## Required sample record

Each sample/nominal-level row carries task, sample and model IDs; geometry family and flow regime; dataset split (must be blind_confirmation for this audit); in-domain or out-of-domain status; nominal central interval coverage; lower and upper prediction bounds; point prediction and observed value.

Every sample must have a unique model/task/sample/stratum/nominal-level record. The point prediction and observed value must remain identical across its interval levels. Higher nominal coverage intervals must nest around lower nominal coverage intervals. Invalid, missing, inverted, duplicate or non-nested intervals fail closed.

## Metrics

For nominal coverage 1 - alpha and interval [L, U]:
- empirical coverage = count of observations in [L, U] / sample count;
- Wilson 95% confidence interval for the empirical coverage proportion;
- coverage gap = empirical coverage minus nominal coverage;
- interval width = U - L;
- mean interval score:

IS_alpha = (U - L) + (2/alpha) * (L - y) * I(y < L) + (2/alpha) * (y - U) * I(y > U)

The interval score is proper: it penalizes both unnecessarily wide intervals and misses, with a penalty scaled by how far an observation falls outside the interval. Compare intervals at common nominal levels and publish coverage and width together; coverage alone can be made trivially high by returning huge intervals.

Also report point MAE, RMSE and signed bias to ensure widening intervals is not confused with improving the point predictor. Lower-tail and upper-tail miss counts remain separate so asymmetric failure is visible.

All metrics are reported by model, geometry family, flow regime, domain status and nominal coverage. Never use pooled in-domain coverage to hide OOD undercoverage.

## Leakage and distribution shift rules

1. Fit the model and any interval calibrator only on training and calibration partitions.
2. Freeze both before touching the blind confirmation partition.
3. The audit rejects any row not explicitly labelled blind_confirmation.
4. Geometry families and flow regimes in confirmation must be held out at the group level when the claim is generalization to those groups; random row splitting alone does not establish this.
5. Report in-domain and out-of-domain results separately, including sample counts and Wilson intervals.
6. State the distribution-shift assumptions. Split conformal coverage relies on exchangeability; it is not an unconditional guarantee under arbitrary distribution shift.
7. Where exact conditional coverage is sought, state the target subgroup family and sample-size limitations. Universal exact conditional coverage is not available in finite samples without restrictions.

## Synthetic known-answer and adversarial control

The CLI creates 60 synthetic observations per model/domain stratum and nominal interval levels 80%, 90% and 95%. The nominal model keeps in-domain widths under OOD shift and therefore undercovers OOD data. The robust model doubles interval widths under OOD shift and restores the designed coverage at higher uncertainty cost. The independent standard-library Python oracle recomputes all 12 summaries, including Wilson intervals, interval scores and point-error metrics.

This controlled example establishes only calculation correctness and the coverage/width tradeoff. It is not real aerodynamic data or evidence that wider intervals improve model physics.

## Research comparisons and stretch targets

Compare at least:
- raw model uncertainty intervals (e.g. ensemble or surrogate intervals);
- split conformal intervals with calibration data frozen;
- locally/group-calibrated or weighted conformal variants where their assumptions apply;
- physics/mesh/turbulence-model ensembles, with model-form variation kept distinct from measurement noise;
- an abstaining policy that declines to rank/approve candidates outside its validated domain.

Compare at equal nominal levels and held-out geometry/flow regimes. Report empirical coverage with confidence intervals, mean width and interval score; point MAE/RMSE; false-feasible rates where design constraints are present; and the fraction of OOD cases on which the model abstains. A method that improves coverage only by making intervals unbounded or rejecting every candidate is not useful.

Proposed stretch target: achieve empirical 90% and 95% coverage whose confidence intervals are consistent with nominal coverage across all predeclared confirmation groups, while minimizing interval score and retaining useful feasible-design yield. This is a research target, not an achieved result.

## References

- Gneiting and Raftery (2007), proper scoring rules and prediction intervals: https://doi.org/10.1198/016214506000001437
- NASA, distribution-free uncertainty quantification using conformal prediction: https://www.nasa.gov/wp-content/uploads/2023/06/60-distribution-free-uncertainty-for-cers.pdf
- Kaiser and Herzog (2025), conformal prediction assumptions and distribution shift: https://journals.sagepub.com/doi/10.1177/25152459251380452
- Gibbs, Cherian and Candès (2025), conditional guarantees and limits of universal exact conditional coverage: https://academic.oup.com/jrsssb/article/87/4/1100/8058684
