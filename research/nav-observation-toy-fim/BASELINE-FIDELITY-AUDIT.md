# Navigation baseline fidelity audit v0.1

**Scope:** equation and statistical-convention audit of the adaptive PID and Student-t protection-level comparators.
**Parent implementation:** https://github.com/Luminous-Dynamics/symtropy/pull/6737
**Follow-up:** https://github.com/Luminous-Dynamics/symtropy/pull/6745 is being replayed on a branch whose base is the #6737 head; this file documents the additional checks.

## 1. Derivative filter

Maharmeh et al. (2026), Eq. (34)-(35), prints:

`alpha = min(1, 2*pi*f_c*dt)`

`D[k] = alpha * (e[k]-e[k-1])/dt + (1-alpha) * D[k-1]`

The #6737 code had used the exponential coefficient `exp(-2*pi*f_c*dt)` as the previous-state weight and described it as the paper's exact equation. It is not the printed equation. The controller now offers two explicitly named discretizations:

- `PaperLinear` — paper equation; default for paper reproduction.
- `ExactExponential` — `alpha = 1-exp(-2*pi*f_c*dt)` as the current-derivative weight; a separately labelled continuous-time first-order alternative.

A benchmark receipt must record the mode. Published reproduction uses `PaperLinear`; a sensitivity study may compare both.

## 2. Student-t TIR convention

For a bivariate Student-t with degrees of freedom `nu > 2`, covariance `P` (not scale matrix), and a total two-dimensional radial tail probability `TIR`, the covariance-normalized radial multiplier is:

`K_TIR = sqrt(TIR^(-2/nu) - 1)`

This must not be substituted for a univariate normal quantile or a one-axis/two-sided marginal probability. The original paper's precise TIR convention must be reconciled before a route-level numerical comparison.

## 3. Directional Student-t equation

The paper's displayed directional expression is retained literally:

`PL_i = K_TIR * sqrt((nu-2) * max(lambda_1*|v_1,i|, lambda_2*|v_2,i|))`

A separate mathematical comparator uses:

`var_i = e_i^T P e_i = sum_j lambda_j * v_(j,i)^2`

`PL_i_cov = K_TIR * sqrt((nu-2) * var_i)`

For `P = [[1.5, 0.5], [0.5, 1.5]]`, whose eigenvalues are 2 and 1 at 45 degrees, the covariance projection is 1.5 while the printed maximum term is sqrt(2). With the same `nu` and `K_TIR`, the printed PL is about 2.90% lower. This exposes a formula-convention difference for author clarification; the two outputs must remain separate in result tables.

## 4. LiDAR residual ambiguity

The article's prose describes an RMS point-to-map residual, while the displayed formula appears to average unsquared Euclidean norms before taking a square root. These are dimensionally different. The controller accepts a precomputed residual and does not resolve the ambiguity. The frontend reproduction must document the source-code/equation interpretation used.

## 5. Evidence boundary

These checks qualify equation-level implementations only. They do not reproduce ES-EKF estimation, raw sensor frontends, time/frame alignment, UrbanNav route outputs, calibrated integrity risk, or physical safety. The paper-specific output, the mathematical alternative, and every software implementation choice must be separately identified in the experiment manifest.
