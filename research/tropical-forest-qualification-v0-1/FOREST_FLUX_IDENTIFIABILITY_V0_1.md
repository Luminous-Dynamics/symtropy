# Forest Flux Identifiability and Measurement Design v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Process trajectory:** [#5340](https://github.com/Luminous-Dynamics/symtropy/issues/5340)  
**Child:** [FOREST-FLUX-IDENTIFIABILITY-001 #5349](https://github.com/Luminous-Dynamics/symtropy/issues/5349)  
**Empirical reproduction:** [FORESTPATHS-EMPIRICAL-REPRO-001 #5352](https://github.com/Luminous-Dynamics/symtropy/issues/5352)  
**Branch:** `research/tropical-forest-qualification-v0-1`  
**Date:** 15 November 2026  
**Status:** exact structural synthetic benchmark; no empirical flux data ingested

## 1. The question before calibration

Before fitting an ecological model, ask whether the observations can identify the process variables being calibrated.

Eckes-Shephard et al.'s nine-model vegetation-demography comparison found that mature carbon stocks could agree even while growth was overestimated and mortality underestimated. The authors emphasize that process-level validation needs higher-temporal-resolution site data that cover carbon pools, fluxes and forest structure. They also explain why chronosequences do not identify a single plot's time-to-recovery, and why spatial and temporal observation support must match the model support:

- [Eckes-Shephard et al. (2025), New Phytologist](https://doi.org/10.1111/nph.70643)
- [Cambridge repository record](https://www.repository.cam.ac.uk/items/3aeb935f-b733-41ce-bd8a-739da3d031e6)
- [Zenodo analysis, observations and model-output archive](https://zenodo.org/records/14415143)

The 2026 Ecology Letters review by Malchow & Hartig recommends identifiability analysis alongside sensitivity, uncertainty analysis and calibration. It discusses equifinality, parameter non-identifiability, model sloppiness, synthetic-data recovery tests, prior sensitivity and exploring the full response surface rather than trusting one optimizer optimum:

- [Malchow & Hartig (2026), Calibration, Sensitivity and Uncertainty Analysis of Complex Ecological Models](https://doi.org/10.1111/ele.70375)

The benchmark turns that concern into an executable pre-calibration gate.

## 2. Structural rank for the two-pool process

Use the bounded process vector:

`x = [G, R, M, X_L, Q, X_D]^T`

where:
- `G`: growth into live biomass;
- `R`: recruitment into live biomass;
- `M`: mortality transfer from live biomass to deadwood;
- `X_L`: live-pool export;
- `Q`: deadwood decomposition;
- `X_D`: deadwood export.

Observed pool increments produce:

`ΔL = G + R - M - X_L`

`ΔD = M - Q - X_D`

The corresponding observation matrix is:

`A = [[1, 1, -1, -1, 0, 0], [0, 0, 1, 0, -1, -1]]`

It has exact rational rank 2 across six unknowns. The nullity is 4. Therefore, stock increments alone cannot distinguish all six fluxes.

Four public integer vectors form an explicit basis of the nullspace in this fixture. Each vector changes the flux combination while leaving both observed stock increments unchanged. The validator checks both `A v = 0` for every witness and that the four witnesses are linearly independent.

This is structural non-identifiability under the declared linear equations. It is not a claim about posterior distributions or real-world parameter estimates.

## 3. Implemented exact-rational measurement tournament

New artifacts:

- `validator/src/flux_identifiability_kernel.rs`: bounded exact-rational row reduction, nullspace witness verification, exhaustive measurement-subset enumeration and synthetic-effort optimization.
- `cases/forest-flux-identifiability-synthetic-v0.1.json`: two stock-balance rows, six optional direct-measurement rows, four nullspace witnesses and frozen expected design results.
- `validator/src/lib.rs`: `FOREST-FLUX-IDENTIFIABILITY-001` dispatch and a top-level digest-receipt test.
- `validator/src/main.rs`: accepts `PASS_SYNTHETIC_FLUX_IDENTIFIABILITY`.

No floating-point threshold is used for the structural rank decision. The matrix coefficients and witness vectors are small bounded integers, transformed through checked rational arithmetic. Overflow, malformed widths, duplicate variable/measurement identities and invalid coefficients fail closed.

Run from repository root:

```bash
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-flux-identifiability-synthetic-v0.1.json
```

The canonical fixture path is `research/tropical-forest-qualification-v0-1/cases/forest-flux-identifiability-synthetic-v0.1.json`; use that exact path if copying the command.

### Frozen structural results

Six optional direct measurements are enumerated, with synthetic effort costs:

| Measurement | Synthetic effort |
|---|---:|
| Growth | 2 |
| Recruitment | 4 |
| Mortality | 3 |
| Live export | 6 |
| Decomposition | 5 |
| Deadwood export | 5 |

All 64 subsets are evaluated.

| Result | Frozen synthetic oracle |
|---|---:|
| Stock-only observation rank | 2 |
| Unknown fluxes | 6 |
| Stock-only nullity | 4 |
| Independent nullspace witnesses | 4 |
| All optional measurement subsets | 64 |
| Full-rank designs | 18 |
| Minimum optional measurement count | 4 |
| Minimum synthetic effort cost | 14 |

Two tied minimum-cost designs achieve rank 6:

1. growth + recruitment + mortality + decomposition;
2. growth + recruitment + mortality + deadwood export.

These choices arise from the declared linear operator and synthetic costs. They are not a universal forest survey prescription. The optimum changes with measurement-error covariance, process support, practical feasibility, season, destructive-sampling restrictions, spatial/campaign dependence and real cost.

## 4. A deeper interpretation: rank is necessary, not sufficient

Structural identifiability only answers whether distinct flux vectors can produce the same ideal noiseless observations under this declared linear model.

It does **not** answer:
- whether actual field instruments can distinguish the fluxes;
- whether measurement uncertainty makes the inverse problem ill-conditioned;
- whether the flux channels share correlated or systematic errors;
- whether the sample spans rare disturbance events;
- whether the process equations omit an important pool or pathway;
- whether a hierarchical site/size-class model is identifiable;
- whether field costs and access conditions make a chosen design worthwhile.

The next practical-identifiability layer should include:
- declared observation-error covariance and bias;
- singular values / condition number of the weighted sensitivity or Jacobian matrix;
- posterior or likelihood ridges and parameter correlations;
- synthetic-parameter recovery and prior sensitivity;
- spatial/campaign block structure;
- robustness under omitted fluxes and alternative process structures.

Only after this layer is justified should we compare Bayesian, SMC, ensemble or optimization-based calibration strategies. The 2026 review treats full Bayesian calibration as a useful gold-standard framework for model inversion, while also emphasizing prior sensitivity, synthetic recovery and independent prediction checks. The current kernel does not implement Bayesian inversion.

## 5. Research tournament: from identifiable to useful

A mature version should compare:

1. stock-only calibration;
2. stock + size-class structure;
3. stock + direct growth/mortality observations;
4. stock + recruitment and decomposition/exports;
5. support- and uncertainty-aware full observation design;
6. optimal expected decision value after measurement error and field cost are admitted.

Metrics should remain separate:
- structural rank and nullity;
- smallest singular value / condition number (future noise-aware extension);
- synthetic-parameter recovery error;
- posterior width and parameter correlation (where a probabilistic inversion is used);
- flux-specific bias and RMSE;
- state and pool budget residual;
- design effort / field time / destructive samples;
- expected reduction in model-set disagreement;
- expected decision value, not uncertainty reduction alone.

The stretch objective is not “maximum measurement count.” It is the smallest feasible set of independently supported observations that resolves the process ambiguity needed for a decision, under explicit costs, errors and permissions.

## 6. Empirical bridge: ForestPaths D-BEN archive

The ForestPaths benchmark is a concrete next empirical target. The published paper says analysis scripts, benchmark data and model outputs are deposited on Zenodo 14415143. The record describes processed benchmark observations as CSV under `observations/2_processed` and model outputs as mostly netCDF under `model_outputs`, generally using D-BEN variables. It lists:
- `model_outputs.zip`: 5.7 MB; record MD5 `0bde6b925bf5ca5e0aec604fc50abd58`;
- `observations.zip`: 430.6 kB; record MD5 `75c53d963e30f1089d23b1752765d822`;
- processing code repository: [DBEN_Eckes_Shephard_et_al](https://github.com/teatree1212/DBEN_Eckes_Shephard_et_al).

The journal article also lists the model versions / code sources, including LPJ-GUESS v4.1 and its exact branch, FATES, CABLE-POP, ORCHIDEE, EDv3, BiomeEP, BiomeE and SEIB-DGVM. JULES-RED access has an application gate. These exact versions and configurations need to be frozen before reproduction.

The Zenodo landing page retrieved for this audit exposes file names, sizes and MD5 values, but its visible rights/license field is not populated. Verify the exact record and embedded file rights/attribution before derivative use or redistribution; do not infer the dataset's license from the paper's publication license.

**Retrieval status:** the record and metadata are accessible through web research, but binary download attempts from the current execution container failed because it cannot resolve `zenodo.org`. No bytes were downloaded or ingested. This is tracked in [#5352](https://github.com/Luminous-Dynamics/symtropy/issues/5352).

## 7. Proposed targets, not achieved results

- Every benchmark matrix has a verified rank/nullity and independent nullspace witnesses.
- Every full-rank measurement design is enumerated for bounded candidate sets; ties are preserved.
- Field measurement costs, uncertainties and covariance are admitted only from a defensible source.
- A chosen empirical design improves flux recovery on held-out sites versus the stock-only and conventional direct-measurement baselines.
- Calibration and evaluation use independent site/campaign blocks.
- The empirical VDM reproduction is matched on model version, forcing, initial states, site/time windows, observation support and scoring rules.
- No test or hidden challenge outcome selects the final observation design after the holdout is opened.

## 8. Claim ceiling

A successful `PASS_SYNTHETIC_FLUX_IDENTIFIABILITY` receipt establishes exact structural rank, witness and finite design-enumeration arithmetic for the declared two-pool matrix. It does not establish practical identifiability, parameter recovery under noise, forest field efficacy, the cost of any real measurement, a recommended survey, or SOTA superiority. The empirical ForestPaths archive remains a candidate for reproduction, not an ingested or reproduced dataset.


## 9. Next stage: practical identifiability under measurement error

The practical follow-on [#5360](https://github.com/Luminous-Dynamics/symtropy/issues/5360) now has an executable synthetic linear-Gaussian reference documented in [FOREST_FLUX_PRACTICAL_IDENTIFIABILITY_V0_1.md](FOREST_FLUX_PRACTICAL_IDENTIFIABILITY_V0_1.md). Its covariance assumptions are synthetic; it is not a field error model.


## 10. Target-specific estimands: recover the question even when the full flux vector is not identifiable

A new exact row-space layer distinguishes identifiability of a scientific target from identifiability of every primitive flux. A linear estimand (c^T x) is identifiable from an observation operator (A) exactly when adding (c^T) as a row does not increase the row rank. Equivalently, it lies in the row space of the observations and does not vary along any nullspace direction.

The fixture declares five target estimands:

- net live-pool increment: identifiable from the live stock balance;
- net deadwood-pool increment: identifiable from the deadwood stock balance;
- total-pool increment: identifiable as the sum of the two stock balances;
- gross mortality transfer: not identifiable from the stock balances alone;
- gross live inputs (growth + recruitment): not identifiable from the stock balances alone.

The 64-design tournament now reports estimand identifiability for every measurement subset and a minimum measurement design for each target. Under the declared synthetic costs:

| Target estimand | Subsets that identify it | Minimum additional cost | Minimum-cost measurements |
|---|---:|---:|---|
| Net live-pool increment | 64/64 | 0 | None |
| Net deadwood-pool increment | 64/64 | 0 | None |
| Total-pool increment | 64/64 | 0 | None |
| Mortality transfer | 43/64 | 3 | Mortality |
| Gross live inputs | 31/64 | 6 | Growth + recruitment |

Gross live inputs can also be identified by mortality + live export (two measurements, but higher cost 9 in this fixture). These costs and subsets follow only from the declared linear equations and synthetic effort values.

This changes the benchmark question from “can we identify all six fluxes?” to “which decision-relevant ecological quantities are supported by the current evidence, and what is the cheapest additional measurement that identifies the quantity we actually need?” A model may be adequate for estimating net carbon change while remaining inadequate for attributing that change to mortality, recruitment or turnover.

The row-space diagnostic is structural and noiseless. Practical uncertainty still depends on the covariance model, field support, bias and model discrepancy handled separately by [#5360](https://github.com/Luminous-Dynamics/symtropy/issues/5360).