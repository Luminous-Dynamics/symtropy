# Forest Flux Practical Identifiability v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Structural layer:** [#5349 — exact flux identifiability](https://github.com/Luminous-Dynamics/symtropy/issues/5349)  
**Practical layer:** [#5360 — noise-aware practical flux identifiability](https://github.com/Luminous-Dynamics/symtropy/issues/5360)  
**Empirical target:** [#5352 — ForestPaths benchmark reproduction](https://github.com/Luminous-Dynamics/symtropy/issues/5352)  
**Date:** 15 November 2026  
**Status:** synthetic linear-Gaussian information study; no field error model admitted

## 1. Why structural rank is not enough

The exact structural-identifiability reference (#5349) shows that two live/deadwood stock-increment observations constrain six unknown flows with rank 2 and nullity 4. Direct process observations can raise the matrix to full rank.

But a full-rank design can still be practically useless. Measurement noise, correlated observations, shared plot support and common calibration error can leave the information matrix poorly conditioned. This is the distinction between *uniqueness in an ideal noiseless system* and *usable information under a declared observation model*.

The 2026 review by Malchow & Hartig recommends identifiability analysis alongside sensitivity, uncertainty and calibration, including equifinality, parameter sloppiness and synthetic-data recovery: https://doi.org/10.1111/ele.70375

The ForestPaths nine-model benchmark independently demonstrates why this matters for forests: mature carbon stocks can agree while growth and mortality compensate, and the authors call for higher-temporal-resolution, process-relevant site data spanning carbon pools, fluxes and structure: https://doi.org/10.1111/nph.70643

## 2. Linear-Gaussian reference

Let the unknown flux vector be:

`x = [growth, recruitment, mortality, live_export, decomposition, deadwood_export]^T`

The observation model is:

`y = A x + \epsilon, \qquad \epsilon \sim \mathcal{N}(0, \Sigma)`

The covariance-aware whitened operator is:

`A_w = L^{-1} A, \qquad \Sigma = L L^T`

and the Fisher information matrix for this linear-Gaussian model is:

`F = A^T \Sigma^{-1} A = A_w^T A_w`

The benchmark reports three separate design criteria:

- **D-optimal:** maximize `log(det(F))`, reducing the generalized confidence-ellipsoid volume.
- **A-optimal:** minimize `trace(F^{-1})`, reducing total marginal variance under the declared linear-Gaussian model.
- **E-optimal:** maximize the smallest eigenvalue of `F`, strengthening the weakest identifiable direction.

It also reports the Fisher eigenvalue spectrum, condition number, and generalized least-squares estimator covariance diagonal where numerically available. There is no prior in this v0.1 construction. These values are not posterior uncertainty from a Bayesian ecological model.

## 3. Executable implementation

New artifacts:

- `validator/src/practical_identifiability_kernel.rs`: covariance validation, Cholesky whitening, Fisher information, symmetric eigenvalues, GLS covariance and D/A/E design ranking; twelve authored unit tests.
- `cases/forest-flux-practical-identifiability-synthetic-v0.1.json`: six unknown flows, two stock observations, six direct measurement options, three declared synthetic correlation pairs and 64 design subsets.
- `validator/src/lib.rs`: exact structural-rank reuse from #5349, top-level corpus dispatch and SHA-256 receipt test.
- `validator/src/main.rs`: CLI accepts `PASS_SYNTHETIC_PRACTICAL_IDENTIFIABILITY`.

Run from the repository root:

```bash
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-flux-practical-identifiability-synthetic-v0.1.json
```

The evaluator rejects zero/unknown standard deviations, invalid correlation identities, duplicate correlation pairs and non-positive-definite covariance matrices. It separately computes exact structural rank using the rational rank implementation in #5349, so structural status is not inferred from floating-point eigenvalues.

## 4. Synthetic result

The fixed six-flux fixture has 64 measurement subsets; 18 are full structural rank. The minimum synthetic cost to achieve structural full rank is 14 effort units, with two tied four-channel designs:

- growth + recruitment + mortality + decomposition;
- growth + recruitment + mortality + deadwood export.

Under the declared covariance and the explicit practical-information thresholds (minimum Fisher eigenvalue 0.01; maximum condition number 30), both cheapest structural designs are classified `PRACTICALLY_WEAK`.

| Design | Cost | Minimum Fisher eigenvalue | Condition number | Status |
|---|---:|---:|---:|---|
| Growth + recruitment + mortality + decomposition | 14 | 0.004969 | 71.82 | `PRACTICALLY_WEAK` |
| Growth + recruitment + mortality + deadwood export | 14 | 0.004953 | 72.02 | `PRACTICALLY_WEAK` |
| All six direct flux channels | 25 | 0.020987 | 17.01 | `INFORMATION_SUFFICIENT_UNDER_DECLARED_SYNTHETIC_THRESHOLDS` |

For the all-six design, all three criteria (D, A and E) select the full channel set in this particular fixture. Its log determinant is -15.03719 and trace GLS covariance is 105.77208 synthetic units under the stated noise scale.

This is the key lesson: a design can be structurally identifiable yet still have a weak information spectrum. The more expensive design improves the smallest eigenvalue by about 4.23× and reduces trace GLS covariance from 363.8 to 105.77 for the first minimum-cost design. This is an analytic toy result, not a measured forest survey trade-off.

A second adversarial unit test constructs an exactly full-rank but nearly collinear observation operator and confirms that the practical diagnostic labels it weak. A covariance negative control also rejects a matrix whose individual pairwise correlations lie within (-1, 1) but whose full joint correlation matrix is not positive definite.

## 5. What this does and does not prove

It proves only the finite mathematical results for the declared two-pool linear observation operator and declared synthetic covariance. It does not establish:
- actual instrument standard deviations or cross-sensor covariance;
- independence of plots, trees, pixels, campaigns or repeated measurements;
- identifiability of a nonlinear FATES, LPJ-GUESS or other VDM parameterization;
- parameter recovery under model discrepancy or structural error;
- an economically optimal field campaign or a conservation intervention.

The minimum-eigenvalue and condition-number thresholds are synthetic fixture cut-offs. They are not universal scientific acceptance criteria. For real models, thresholds must be related to target parameter scales, measurement uncertainty, acceptable predictive uncertainty and decision loss.

## 6. Next empirical step

The ForestPaths data and model-output archives are the nearest public comparison opportunity. The paper's authors state that its processed observations and model outputs are deposited on Zenodo 14415143. The archive still requires exact file-byte retrieval, checksum verification, licence/attribution review, inventory and model-specific schema mapping. It is not admitted merely because its landing page is public.

For an admitted plot/campaign dataset:
1. estimate observation error from repeat measurements, intercomparison or documented instrument calibration;
2. estimate correlated errors at the actual plot/campaign support, not by assuming every stem or pixel is independent;
3. preserve the distinction between chronosequence and repeated same-plot longitudinal evidence;
4. compose the practical information analysis with #5223 expected value of sample information, #2171 active sensing and #5198 model-set adequacy;
5. test synthetic-data recovery on the actual model response surface, with holdout observations untouched by design selection.

The target is not maximum entropy reduction in isolation. It is the minimum permitted, support-valid, rights-cleared measurement design that reduces uncertainty enough to change a declared ecological or conservation decision.

## 7. Claim ceiling

A passing receipt means exact structural rank, covariance validity, numerical information metrics and design-oracle arithmetic match the public synthetic fixture. The synthetic noise and thresholds are not empirically calibrated. Field measurement precision, practical ecological identifiability, survey optimality, forecast performance and SOTA superiority remain `NOT_ESTABLISHED`.
