# Group-wise finite-sample calibration of positive NLOS bias

**Status:** deterministic calibration kernel and synthetic tests added; no physical calibration cohort has been admitted or scored.

**Tracker:** [issue #6869](https://github.com/Luminous-Dynamics/symtropy/issues/6869).

## 1. Why group calibration is necessary

Thousands of adjacent radio packets from one experiment do not constitute thousands of independent calibration trials. A row-wise quantile can appear extremely precise while the whole environment, obstacle material, anchor configuration or device has a common bias.

The calibration method therefore takes one predeclared independent calibration unit per environment/experiment/campaign and collapses all residuals in that unit to its maximum positive bias. Each unit contributes one value to the order statistic.

## 2. Mathematical rule

For group (g), compute:

[
s_g = max_{iin g} b_i,qquad b_i = r_i^{measured}-r_i^{truth}ge 0
]

For (n) exchangeable independent groups and miscoverage (alpha), sort:

[
s_{(1)} le cdots le s_{(n)}
]

Select:

[
k=leftlceil (n+1)(1-alpha)ightceil
]

The controller-facing positive-bias interval is:

[
b_{NLOS}in [0,s_{(k)}]
]

When (k>n), the method returns `InsufficientIndependentGroups` and emits no bound. For example, alpha=0.05 requires at least 19 independent calibration units; 18 units cannot support a finite 95% one-sided bound by this rule.

The implementation calculates (k) using integer parts-per-million arithmetic, not floating-point rounding.

## 3. What is guaranteed—and what is not

The finite-sample interpretation is group-level marginal coverage under exchangeability of the predeclared independent calibration units. It is not:
- conditional coverage for every distance/material/anchor;
- a guarantee under arbitrary domain shift;
- proof that a group definition is independent;
- a universal NLOS bias model;
- an integrity or safety certificate.

The units must be defined before seeing residuals. Leave-one-material, environment and anchor-configuration tests are required to investigate whether the exchangeability scope is credible.

Negative or unknown-sign residuals are rejected; this method must only receive residuals already qualified as positive excess-path bias. LOS and arbitrary signed errors need separate modelling.

## 4. Controller/evaluator boundary

Input `PositiveNlosBiasCalibrationUnit` contains evaluator-derived residuals and a predeclared unit ID. Those residuals remain calibration/evaluator-side.

The output receipt records:
- alpha and selected order-statistic rank;
- independent group count and total residual observations;
- upper-bound value and maximum group score;
- calibration-domain ID;
- SHA-256 dataset and split-manifest commitments.

The receipt can export a `PositiveNlosBiasBound { minimum_bias_m: 0, maximum_bias_m: ... }` for controller-side range interval composition. It does not expose raw residual rows or truth labels.

A production pipeline must keep calibration execution isolated from the controller and preserve the dataset/split commitments. A nonzero digest supplied by a caller is not itself proof that the underlying file was actually frozen; admission checks must verify it.

## 5. Tests

Eight synthetic tests cover:
1. Exact 95% group-level order-statistic rank with 19 units.
2. Group maximum aggregation despite highly unequal row counts.
3. Rejection when 18 units cannot support the requested 95% rank.
4. A lower-coverage target where the rank is available.
5. Duplicate unit identity rejection.
6. Negative and nonfinite residual rejection.
7. Dataset/split commitment and calibration-domain validation.
8. Invalid alpha, empty calibration and empty-unit rejection.

These tests validate the rank and input contract only. No Bregar or MILUV residuals have been used.

## 6. Integration path

1. Freeze an admitted source payload and calibration split manifest.
2. Select independent experiment/environment groups before examining residuals.
3. Derive positive NLOS residuals using evaluator-only truth, after the source/controller stream is frozen.
4. Apply the group-wise calibrator and preserve its receipt.
5. Test coverage and width on held-out materials, anchors, devices and environments.
6. Only export the bound inside its declared calibration domain; outside it, abstain or use a separately qualified model.
7. Feed the resulting bound to interval-valued DS-TWR; keep clock, antenna, power and NLOS uncertainty as separate terms.

## Claim boundary

This is a reusable group-level calibration kernel. No empirical coverage, NLOS-bias performance or physical navigation result is claimed. Group independence/exchangeability, dataset rights, payload hashes, truth lineage and held-out domain coverage remain to be established.
