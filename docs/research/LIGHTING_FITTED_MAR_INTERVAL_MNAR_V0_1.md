# Lighting fitted-MAR and interval-aware MNAR sensitivity v0.1

**Status:** exploratory research implementation; no inferential method selected.  
**Repository:** `Luminous-Dynamics/symtropy`; stacked after draft PR #2827.  
**Claim ceiling:** synthetic statistical diagnostics only. No Radiance execution, measured buildings, calibrated photometry, product compliance, safety or industry-superiority claim is made.

## Motivation

The preceding lighting evidence work showed two unacceptable extremes: complete-case analysis can overstate savings under informative missingness, while bounded worst-case completion can be valid under declared finite bounds yet too conservative to support a decision. This step adds a fitted response model and interval-aware MNAR sensitivity rather than relying on oracle propensities and a point-estimate tipping delta.

## What is implemented

The standard-library-only script in `tools/lighting-study-plan/fitted_mar_interval_mnar.py` includes:

1. A fitted one-covariate logistic model for whether the outcome was observed, with convergence and separation checks.
2. An observed-case linear outcome model using pre-outcome scene difficulty.
3. Fitted stabilized Hájek inverse-probability weighting and augmented inverse-probability weighting (AIPW).
4. Positivity/overlap diagnostics: range of fitted observation probabilities, maximum inverse-probability weight, effective sample size and an eligibility flag that fails closed when checks do not support the MAR analysis.
5. Pattern-mixture delta sensitivity. Delta is the assumed missing-outcome mean shift relative to the fitted MAR conditional mean, in percentage points. Every delta reports an estimate, asymptotic one-sided 95% lower bound, two-sided 95% upper bound, and a grid-based tipping diagnostic.
6. A separate confirmation suite frozen independently from the development scenario grid, with different sample sizes, tail distributions, 40% MCAR and nonlinear MAR mechanisms.

The AIPW confidence limits use an asymptotic influence-function standard error. The simulation is specifically designed to evaluate when that approximation fails; it is not assumed to be finite-sample valid. A fitted model and good overlap do not prove missing-at-random. MNAR cannot be ruled out from observed outcomes alone.

## Local validation and results

On the source implementation mirrored into the local test package:

- **7/7 unit tests passed**.
- A completed frozen synthetic run covered **72 scenarios × 200 replications**, seed `20261028`. Results and exact source digest are recorded in `results/frozen-confirmation-200.json`.
- A separate deterministic constructed example has **60 independent families, 38 observed and 22 missing**. Under delta=0, the AIPW point estimate is **5.784 percentage points** and its one-sided lower bound is **5.122**, apparently clearing a 5-point margin. The nearest negative grid tipping point is **−1 percentage point**. The true constructed outcome-dependent missingness mechanism deliberately violates MAR, so this is a warning case, not a real-world estimate.

Worst null false-superiority rates across sample sizes/distributions in the 200-replication confirmation grid:

| Missingness mechanism | Maximum null false-superiority rate | Mean null estimate bias (pp) | Minimum 95% interval coverage |
|---|---:|---:|---:|
| Complete | 10.5% | +0.013 | 89.5% |
| MCAR, 40% missing | 18.0% | +0.002 | 83.5% |
| Nonlinear MAR difficulty | 16.0% | +0.014 | 85.5% |
| MNAR, low outcomes more likely missing | 90.5% | +0.814 | 14.0% |

These maximums are scenario-wise diagnostics from only 200 Monte Carlo replicates, not method rankings or simultaneous confidence statements. In particular, the very high MNAR failure is expected: it shows what happens when a MAR estimator is used despite outcome-dependent missingness. Even complete-data/skew and MCAR/MAR stress cases show false-positive inflation and undercoverage; **the current AIPW interval is not approved as a confirmatory decision rule**. Eligibility rates and failure reasons are separately reported so discarding difficult samples cannot create a false impression of reliability.

## Why this is a paradigm shift

The evaluator is no longer allowed to say “the optimizer saves energy” merely because the available cases look favorable. It must expose selection assumptions, overlap problems, uncertainty, and the range of MNAR deviations under which a conclusion changes. Feasibility failures, non-compliant photometry, solver timeouts and truly missing energy observations remain distinct outcomes.

## Next benchmark gates

1. Expand and preregister a held-out ADEMP simulation suite; use larger Monte Carlo counts and account for multiple scenario-wise screening rather than selecting a method on its development grid.
2. Require nominal type-I error and interval coverage across skew, heavy tails, missingness, positivity stress and scene-family shift. Non-eligibility is itself a reported failure, not a dropped replicate.
3. Compare candidate inference procedures using bias, RMSE, interval coverage, false-superiority rate, power, eligibility rate, compute cost and worst-case behavior; do not select AIPW from this run.
4. For lighting outcomes, pin Radiance versions and scene/material settings, compare matched compute budgets against conventional search and commercial design workflows such as DIALux, and report optical quality separately from energy.
5. Confirm on held-out real rooms with calibrated illuminance grids, power measurements and daylight/luminaire diversity. No commercial baseline or physical measurements are included here.

## References

- Morris, White & Crowther (2019), “Using simulation studies to evaluate statistical methods”: https://pmc.ncbi.nlm.nih.gov/articles/PMC6492164/
- Missing-data sensitivity analysis and pattern-mixture/selection models: https://www.ncbi.nlm.nih.gov/books/NBK209895/
- Positivity and weighting diagnostics: https://onlinelibrary.wiley.com/doi/10.1002/sim.70329
- Radiance `rtrace` reference manual: https://radsite.lbl.gov/radiance/man_html/rtrace.1.html
- DIALux official downloads/releases: https://www.dialux.com/en-GB/download

The exact-head GitHub Actions result is not asserted here; queued or absent CI is not a pass.
