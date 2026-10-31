# MFG-COMP-UQ-001 — Correlation-preserving uncertainty propagation across composed twins

**Version:** 0.1  
**Date:** 2026-10-31  
**Evidence tier:** T0 synthetic analytical  
**Implementation:** `tools/manufacturing-assurance/composition_uncertainty.rs`

## 1. Research question

Can a composed manufacturing digital twin preserve and audit correlations between subsystem uncertainties, especially when multiple twins share a common sensor calibration, material lot, ambient variable, clock, preprocessing pipeline or calibration reference?

A component-level uncertainty interval is not enough. A downstream output depends on the joint distribution of its inputs, not merely on each marginal variance.

## 2. Prior art and standards boundary

JCGM 102:2011 (GUM Supplement 2) explicitly propagates an input covariance matrix through a measurement model to obtain output covariance. The linear law has the form:

[
U_y = J U_x J^T,
]

where (J) is the sensitivity/Jacobian matrix and (U_x) includes covariance terms between inputs. This is established metrology practice; this project is not claiming to invent covariance propagation.

NIST's October 2026 report on harmonizing CMS UQ and metrology practices, ASME's VVUQ portfolio, and the approved ISO/AWI 23247-8 VVUQ work item provide the relevant assurance context. The ISO work item remains under development; this benchmark is not evidence of conformance.

The opportunity is to make **joint uncertainty and shared-cause provenance first-class across distributed twin boundaries**: do not serialize only a mean and standard deviation when the receiving model needs covariance/correlation metadata to form valid output uncertainty.

## 3. Analytic known-truth panel

Assume two subsystem estimates have a shared common-mode calibration bias (b), plus independent local errors:

[
X_A = mu_A + b + epsilon_A,quad
X_B = mu_B + b + epsilon_B.
]

Known input variances:

- (Var(b)=4) K²;
- (Var(epsilon_A)=1) K²;
- (Var(epsilon_B)=2.25) K²;
- (Cov(epsilon_A,epsilon_B)=0).

Therefore:

[
U_x =
egin{bmatrix}
5 & 4\
4 & 6.25
end{bmatrix}	ext{ K}^2.
]

Use a sum/difference transform:

[
A =
egin{bmatrix}
1 & 1\
1 & -1
end{bmatrix},quad
Y = AX,quad U_y=AU_xA^T.
]

Exact expected output:

[
U_y =
egin{bmatrix}
19.25 & -1.25\
-1.25 & 3.25
end{bmatrix}	ext{ K}^2.
]

The false independence control zeros the off-diagonal input covariance and reports 11.25 K² for both output variances. That underestimates sum variance and overestimates difference variance. This demonstrates why correlation handling must be output-aware.

## 4. Implementation scope

The Rust reference provides:
- a 2×2 covariance matrix type;
- finite-value, nonnegative-variance, symmetry and positive-semidefinite checks;
- a 2×2 linear map;
- explicit (A U A^T) propagation;
- map composition and an associativity regression test;
- correlation calculation with sign;
- a frozen analytical fixture and deterministic report;
- negative controls for malformed fixtures, non-finite data, asymmetric and non-PSD matrices, and non-finite transforms.

This is a deliberately small audit target. It is not a full metrology library.

## 5. Metrics

1. **Propagation error:** absolute/relative error from the analytical output covariance.
2. **Correlation preservation:** correlation coefficient before and after transformations.
3. **False-independence cost:** output variance and coverage error when covariance is incorrectly dropped.
4. **PSD integrity:** rejection rate for invalid covariance matrices and numerical tolerance behavior.
5. **Composition consistency:** difference between sequential propagation and propagation through the composed map.
6. **Calibration/coverage:** later report empirical coverage for 50%, 90%, 95% and 99% intervals on known-truth Monte Carlo panels.
7. **Decision impact:** frequency and magnitude of decisions changed by covariance omission, separated by consequence severity.
8. **Scaling:** runtime and memory versus dimension, sparsity, graph width and number of latent causes.

No composite score may hide covariance invalidity or undercoverage.

## 6. Target levels

| Axis | T0 target | Required before industrial claims |
|---|---|---|
| Analytical propagation | exact 2×2 result within (10^{-12}) for fixture | independent implementation and arbitrary-dimensional reference |
| PSD guards | reject frozen invalid cases | property-based and adversarial numerical testing |
| Correlation | signed coefficients preserved | shared-source provenance across real model boundaries |
| Coverage | not claimed in v0.1 | nominal empirical coverage with confidence intervals on held-out trials |
| Independence error | expose 41.56% variance understatement in fixture | measure decision-impact distribution on real workflows |
| Provenance | fixture-level source decomposition | artifact IDs, calibration lineage, validity intervals, digests and lifecycle invalidation |

These are proposed acceptance targets, not measured performance claims. The false independence panel is deterministic and analytical; it is not a Monte Carlo coverage study.

## 7. Comparison with current practice and SOTA

### Metrology baseline

Compare against JCGM 100/102 covariance propagation and JCGM 101 Monte Carlo propagation. Preserve the same input covariance and sensitivity model across implementations. Compare first-order linearization with Monte Carlo for nonlinear systems in later work.

### Engineering VVUQ baseline

Map the assurance argument to ASME VVUQ standards and the developing ISO 23247-8 framework. Report verification, validation, uncertainty, and risk-based decision evidence separately.

### Commercial simulation baseline

Evaluate version-pinned workflows in Siemens Simcenter, Ansys Twin Builder and Modelon Impact. First establish whether the relevant release exposes cross-model covariance or common latent uncertainty; do not infer absence from a marketing page. Record export/import format, solver, UQ method, covariance representation, assumptions about independence, and downstream interval output. Compare measured outputs rather than feature lists.

### Research baseline

Benchmark against:
- independent marginal propagation (intentional false baseline);
- covariance-aware linearization;
- Monte Carlo propagation with correlated draws;
- variance-based/global sensitivity methods for nonlinear models;
- sparse covariance/factor-graph methods for large compositions.

The right SOTA comparison is not the one that produces the narrowest uncertainty interval. It is the one that gives the best-calibrated interval and decision undercoverage/overcoverage at a comparable computational cost.

## 8. Provenance model to develop

An uncertainty contribution should eventually carry:
- source identity and immutable digest;
- quantity, SI unit and coordinate/reference frame;
- distribution or covariance parameterization;
- correlation-group/common-cause identifier;
- scope (asset, machine, lot, process, site);
- validity interval and calibration/qualification evidence;
- lineage linking the contribution to model inputs/outputs;
- declared assumptions and missing-covariance behavior.

An absent covariance is not evidence of independence. The receiving composition must distinguish explicit independence, unknown dependence, and measured/estimated dependence. Unknown dependence may require conservative bounds or abstention rather than silently setting covariance to zero.

## 9. Roadmap

1. T0: current two-variable analytic reference.
2. T1: arbitrary-dimensional dense and sparse covariance operations; factorized common-latent representation; independent cross-check implementation.
3. T1: deterministic and Monte Carlo comparison with correlated Gaussian and non-Gaussian nuisance factors.
4. T2: public measured data with traceable calibration, lot/environmental covariates and data licensing.
5. T3: independent laboratory replication with shared calibration and independent reference channels.
6. T4: shadow-mode industrial deployment; decision consequence and uncertainty coverage measured before any control authority.

## 10. Claim ceiling

The implementation establishes only algebraic propagation for a synthetic 2×2 linear covariance example. It does not prove distributional calibration, nonlinear UQ performance, GUM compliance, physical validity, or production decision fitness. T0 success does not establish SOTA superiority.
