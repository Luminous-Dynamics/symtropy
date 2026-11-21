# Correlation uncertainty envelope: prototype v0.1

This module extends the known-common-mode model with an explicit finite family of pairwise correlation matrices. For each scenario, the observation covariance is:

`R_ii = sigma_i^2`
`R_ij = rho_ij sigma_i sigma_j`

Every unordered sensor pair must be declared. Omitted pairs are not treated as independent. The covariance matrix must pass Cholesky factorization (strict positive definiteness); invalid or singular scenarios fail closed.

## Why this matters

In a deployed localization network, source count does not identify the effective number of independent observations. The correlation coefficient may itself be uncertain because the exact dependence between clocks, antenna paths, correction services, power, calibration, anchor survey and map priors may not be measurable. A planner should therefore retain a family of plausible correlation hypotheses and optimize against its worst case, rather than choose one convenient `rho=0` assumption.

## Known-answer check

Two same-bearing observations with unit standard deviations have directional Fisher information:

`Jxx = 2/(1+rho)`

For rho = 0, 0.5 and 0.9, Jxx = 2, 1.3333 and 1.0526 respectively. The sensor count is always two.

For two orthogonal bearings, the D-optimal determinant actually increases with positive rho because one eigen-direction gains information while the other loses it. The envelope therefore also reports and minimizes the smallest FIM eigenvalue (worst-direction information): at rho = 0, 0.5 and 0.9 this is 1, 0.6667 and 0.5263 respectively. This avoids treating a larger determinant as uniformly safer geometry.

The finite envelope rejects a three-sensor correlation matrix that is not positive definite, as well as missing, duplicate and unknown sensor-pair declarations. Run `cargo run --manifest-path research/nav-observation-toy-fim/Cargo.toml --bin correlation_envelope_demo` for the reproducible known-answer output.

## Claim limits

- The finite scenario set is not a continuous interval guarantee.
- Pairwise correlations supplied in a scenario must jointly define a positive-definite covariance matrix; valid pairwise scalar bounds alone do not guarantee this.
- No source independence is inferred from modality or ID.
- This evaluates local linearized Fisher information, not posterior covariance, protection level or integrity.
- This module is not a provenance source. Mycelix owns the evidence establishing the allowed correlation hypotheses.
