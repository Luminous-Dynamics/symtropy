# EHD-PIV-STRUCTURE-001 — Weak-structure preservation benchmark

**Status:** prospective V0 manufactured 2D scorecard. No real-flow or physical EHD claim.

**Issue:** [#7209](https://github.com/Luminous-Dynamics/symtropy/issues/7209)

## Why global metrics are insufficient

Relative velocity-field error and a vector-validity classifier answer different questions. A classifier can agree with expert vector labels while downstream filtering, gap filling or smoothing changes the physical structures used in an engineering analysis.

Ju (2026) reports reconstruction L2, dynamic range and correlation measures for SVGP-KAN, LSE, SAMM-RR and Kalman methods on a synthetic periodic impinging jet. The study reports approximately 0.047 relative L2 and 0.996 correlation for SVGP-KAN/SAMM-RR at 8.1% PIV sampling. That cohort is deterministic and synthetic; its result is not a turbulent-flow or EHD benchmark. The paper's structure metrics make a natural comparator, but the next step is to explicitly test weak-feature deletion and amplitude distortion. [Source](https://doi.org/10.1016/j.aitf.2026.100034).

PIVPostProcess (Ghazi Nezami, Johnson & Vantassel, 2026) reports test F1 0.992 / ROC-AUC 0.998 and mean F1 0.990 across six unseen experimental datasets. Its task is classifying vectors relative to manual analyst labels. It is a strong post-processing comparator, but F1 is not an end-to-end physical structure-preservation metric. [Source](https://doi.org/10.1007/s00348-026-04319-7).

The key hypothesis is falsifiable: a filter can improve label agreement or global error while deleting a weak coherent structure. We need a scorecard that makes this loss visible.

## V0 metric kernel

The crate `tools/ehd-piv-structure-preservation` evaluates declared reference and reconstructed 2D vector fields on an explicit mask.

### Vorticity and stencil

For coordinates increasing in the positive x/y directions:

`omega = dv/dx - du/dy`

V0 uses a centered five-point stencil. A derivative is admitted only if the center, west, east, south and north grid cells all lie in the evaluation domain. This avoids differentiating across masked boundaries. Grid spacing is an explicit input and must be finite and positive.

The reference vector field must be present on every cell in the declared evaluation domain. Reconstruction absence is retained as missingness, never converted to a zero vector.

### Frozen weak-structure estimand

The benchmark defines weak-structure cells from the reference field before evaluating a method:

`weak_min <= abs(omega_reference) <= weak_max`

A weak cell is retained only when a reconstructed derivative is available, the vorticity sign matches, and the predicted magnitude is at least the configured fraction of the reference magnitude.

Reference-background cells satisfy `abs(omega_reference) < weak_min`. A reconstructed vorticity above the same threshold in those cells counts as false support. Stronger reference structures outside the weak band are not misclassified as background.

This is a benchmark estimand, not a universal physical vortex detector. Thresholds must be frozen by the benchmark owner and justified against the characteristic scales and measurement resolution.

### Separate metrics

The scorecard reports:
- vector coverage and missing fraction;
- relative L2 / normalized RMSE on available vectors;
- relative vorticity error and derivative coverage;
- weak-structure recall, precision and Dice;
- weak-structure sign agreement;
- signed amplitude ratio and bias;
- false vorticity support within reference-defined background.

Missing vectors count against coverage and structure retention, even though reconstruction error on available vectors is reported separately. This prevents a candidate from improving error by deleting difficult regions without exposing the resulting information loss.

## Manufactured controls

V0 includes exact reconstruction, weak-vortex deletion, attenuation, sign reversal, an invented vortex, concentrated missingness, irregular masks, reference holes, non-finite values and insufficient-coverage cases. It is a deterministic metric-kernel test suite; no image-processing model is included.

## SOTA tournament

After data admission and exact baseline reproduction, compare:
- no-removal reference;
- local/global median and universal outlier detection;
- adaptive Gaussian window;
- PIVPostProcess on matched labels;
- interpolation/imputation of removed vectors;
- POD, LSE and SAMM-RR;
- SVGP-KAN;
- covariance-aware GP / reduced-rank GP;
- HDC/CfC as a matched challenger.

Compare at both matched relative-L2 and matched valid-vector retention. Report classification and reconstruction operation classes separately. Use acquisition-cluster bootstrap intervals for empirical data; pixels and vectors are not independent replicates.

## Prospective targets

- Manufactured controls must match their analytical/constructed oracle outcomes.
- Target weak-structure recall >= 0.95 at false-support rate <= 0.05.
- Target signed weak-structure amplitude bias within +/-5% where the reference support and derivative resolution are adequate.
- Report peak/centroid displacement and circulation error as later metrics when their contour/support definitions are frozen.
- Never accept a high-confidence zero where the reference contains a weak structure without exposing it as a structure-retention failure.
- No aggregated F1 or relative-L2 score may replace coverage and weak-structure metrics.

These are prospective targets, not achieved results.

## Dataset admission

Use analytic manufactured fields first. DNS/Eulerian-Lagrangian data can validate observation operators and numerical response but is not experimental PIV truth. The INRAE experimental cylinder-wake dataset remains outside admitted payloads until exact reuse rights and reference semantics are clarified. SVGP-KAN source data/code must be obtained for a faithful reproduction. Physical EHD remains a separate later validation lane.

## References

- Ju (2026), [SVGP-KAN flow reconstruction](https://doi.org/10.1016/j.aitf.2026.100034).
- Ghazi Nezami, Johnson & Vantassel (2026), [PIVPostProcess](https://doi.org/10.1007/s00348-026-04319-7).
