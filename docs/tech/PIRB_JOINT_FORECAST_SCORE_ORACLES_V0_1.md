---
title: PIRB Joint Forecast Score Formula Oracles
version: 0.1
status: research-stage, synthetic-only
authority: MeasurementOnly
---

# PIRB joint forecast score formula oracles v0.1

## Why this layer exists

PIRB now has two separate upstream contracts: joint sample coherence (#2191 / PR #2246) and score admissibility (#2254 / PR #2255). These check whether samples satisfy declared constraints and whether the registered metric-comparison family is allowed; they do not calculate multivariate scores.

This increment implements deterministic **energy score** and **variogram score** for a fixed multivariate observed vector, comparing unreconciled/base samples and reconciled samples separately. The primary theoretical design reference is Panagiotelis et al. (2023), *Probabilistic forecast reconciliation: Properties, evaluation and score optimisation*, which develops probabilistic reconciliation and evaluates it with multivariate scoring rules. The paper proves that log score is improper for the relevant comparison of reconciled and unreconciled distributions, and instead studies energy and variogram scores: https://doi.org/10.1016/j.ejor.2022.07.040.

The 2026 review by Waghmare and Ziegel discusses the broader foundations and families of proper scoring rules: https://doi.org/10.1146/annurev-statistics-042424-050626. A formula being implemented correctly does not by itself prove the formula is proper for every estimand and comparison; PIRB retains the separate score-admissibility gate.

## Supported contract

Schema: `docs/schemas/pirb-joint-score-registration-v0.1.schema.json`.

Fixtures:
- `docs/fixtures/pirb-joint-score-four-draw-synthetic-v0.1.json` — two-dimensional, four-draw oracle.
- `docs/fixtures/pirb-joint-score-three-draw-synthetic-v0.1.json` — independent two-dimensional, three-draw oracle.

Only aggregate-only synthetic metadata and invented values are accepted in this version. The registration pins draw IDs, target period, historical fit cutoff, hierarchy, score vector ordering, observation vector, base and reconciled sample values, method digest, score parameters and numerical tolerance.

Every registered forecast node must have exactly one sample row for every declared draw. Duplicate, unknown or missing draw/node cells fail closed. Reconciliation fit cutoff must precede forecast origin, and target period must follow forecast origin.

## Energy score

For joint draws (X_1,\ldots,X_M\), observed vector (y\), Euclidean norm, and the registered finite-ensemble V-statistic convention:

[
ES = \frac{1}{M}\sum_{m=1}^M \|X_m-y\|_2
-\frac{1}{2M^2}\sum_{m=1}^M\sum_{n=1}^M\|X_m-X_n\|_2.
]

All ordered pairs are included, including zero diagonal pairs. Lower is better for this registered score. The convention is explicitly pinned rather than left to the implementation default.

## Variogram score

For exponent (p=1\), unit weights for all coordinate pairs, and the same observation/sample vector:

[
VS_1=\sum_{i<j}\left(|y_i-y_j|-\frac{1}{M}\sum_{m=1}^M|X_{mi}-X_{mj}|\right)^2.
]

Lower is better. The exponent and unit-weight policy are fixed in v0.1; no metric parameters are tuned on the test observations.

## Double-counting prevention

The observed target vector must be in the exact order specified by `scored_node_ids`. It may not include a registered aggregate parent together with one or more of that parent's children in a declared aggregation constraint. Including both would count overlapping information twice and change the geometry of the score. The validator rejects that vector instead of silently reweighting or deduplicating it.

This check does not independently prove that the external hierarchy is authentic. Hierarchy, child-set and outcome provenance still require source authority and independent validation for real data.

## Independent golden values

### Four draws

Observed vector: (y=(0,0)\). Base draws are ((0,0),(1,0),(0,1),(1,1)\). Reconciled draws are ((0,0),(2,0),(0,2),(2,2)\). With the registered finite-ensemble convention:

- Base energy score: ((2+\sqrt{2})/8\) (approximately 0.4267766953).
- Base variogram score: (1/4\).
- Reconciled energy score: ((6+3\sqrt{2})/8\) (approximately 1.2803300859).
- Reconciled variogram score: (1\).

This fixture is intentionally constructed so that the reconciled draws are structurally coherent but score worse against the invented observation. It tests the important non-implication: **coherence does not imply forecast accuracy**.

### Independent three-draw oracle

Observed vector: (y=(0,0)\). Base draws are ((1,0),(0,1),(1,1)\). The expected scores are:
- Energy: (5(2+\sqrt{2})/18\).
- Variogram: (4/9\).

Rust tests compare the computed values against these closed-form expectations at an absolute tolerance of (10^{-12}\). The CI independently checks the four-draw receipt's score and coherence summaries.

## Receipt contents

The `pirb-joint-score-eval` command emits a deterministic JSON receipt binding:
- input registration and schema hashes;
- source commit and score implementation hash;
- method, hierarchy, forecast artifact and outcome snapshot digests;
- base and reconciled energy/variogram scores and paired numeric differences;
- per-draw hierarchy residuals and coherence status;
- total and maximum absolute sample adjustments;
- explicit non-claims for calibration, real-world superiority, operational utility, causal impact and live authority.

The numerical difference between scores is descriptive for this synthetic fixture. One observed vector and four invented draws cannot support a general superiority claim; no ranking is emitted.

## Acceptance criteria

- Both independent hand-calculated fixtures match within (10^{-12}\).
- All missing/duplicate sample cells and target-vector ordering mismatches fail closed.
- An overlapping parent-plus-child score vector is rejected.
- Sample-row ordering does not change score values, while the full input digest still changes.
- Coherence diagnostics, score values and adjustment magnitude are reported separately.
- Public CI remains synthetic-only and aggregate-only.
- Real-world comparisons require an admissible score registration, common eligible rows, mature frozen outcomes, authorized data, preregistered estimands and dependence-aware uncertainty.

## Limits

The scorer does not fit reconciliation parameters, produce predictive distributions, authenticate that rows share genuine joint dependence, estimate marginal or joint calibration, test causal effects, or establish an SOTA advantage. Matching draw labels do not authenticate the original statistical coupling. The fixtures only verify formula implementation under the stated finite-ensemble convention.

## Dependencies

Follows the joint-sample coherence contract (#2191 / PR #2246), score-comparison admissibility (#2254 / PR #2255), outcome-vintage handling (#2109 / #2118) and the benchmark ladder (#2159).
