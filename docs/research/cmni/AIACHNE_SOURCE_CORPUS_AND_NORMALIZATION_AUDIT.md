# AIACHNE PFNS corpus and shared-reference normalization audit

**Status:** source-byte and structural audit; scientific reproduction remains outstanding  
**Pinned source:** `lanl/sparse_bias@190a4e97ac21de4f33b42e49733e62c45d515e75`  
**CMNI branch:** `research/cmni-symtropy-v0`

## 1. Corpus identity and byte verification

The public LANL directory `AIACHNE/Data_LA-UR-25-31736/` contains 22 files: 21 experiment JSON records and one pointwise (^{252}\mathrm{Cf}) reference-evaluation file. The total source size is 1,508,795 bytes.

The manifest `research/cmni/source-manifests/lanl-aiachne-pfns-public-corpus-v1.json` records:
- the immutable upstream commit;
- each relative path and byte length;
- each Git blob SHA-1 object identity;
- a SHA-256 digest for each file.

All 22 text payloads were retrieved at the pinned commit. For every file, its UTF-8 byte length matched GitHub's reported size and a locally recomputed Git blob SHA-1 (the `blob <size>\0<bytes>` object preimage) matched the upstream object ID. SHA-256 was computed over the same bytes.

This is exact source-byte identity verification. The bytes were retrieved and hashed in-memory through the GitHub connector; this is not a claim that they were copied into the Symtropy repository or a local filesystem. It also does not establish scientific validity, completeness of experimental provenance, or eligibility for a benchmark. The LANL README states BSD-3 licensing; preserve original copyright/attribution and verify redistribution terms before republishing source bytes.

## 2. Structural audit of the 21 experiment records

The audit parsed `data.values`, `data.uncertainties`, `data.correlations`, and `data.structure[1].limits` from every experiment record.

| Experiment file | Observations | Minimum unpivoted Cholesky pivot |
|---|---:|---:|
| PFNS_Bao1989.json | 33 | 0.0391482 |
| PFNS_Blain2017highEout.json | 12 | 0.0005470089 |
| PFNS_Blain2017lowEout.json | 13 | 0.0285906 |
| PFNS_Blinov1973.json | 73 | 0.00647697 |
| PFNS_Blinov1980002.json | 67 | 0.290811 |
| PFNS_Blinov1980003.json | 49 | 0.389919 |
| PFNS_Blinov1980004.json | 76 | 0.353803 |
| PFNS_Blinov1980005.json | 81 | 0.622406 |
| PFNS_Boettger1990.json | 60 | 0.01144997 |
| PFNS_Boldeman1986highEout.json | 59 | 0.0000188666 |
| PFNS_Boldeman1986lowEout.json | 28 | 0.00621279 |
| PFNS_Boytsov1983003.json | 24 | 0.0445277 |
| PFNS_Boytsov1983004.json | 24 | 0.00596214 |
| PFNS_Boytsov1983ant.json | 55 | 0.00276827 |
| PFNS_Chalupka1990.json | 28 | 0.345596 |
| PFNS_Knitter1973003.json | 90 | 0.0388139 |
| PFNS_Kornilov2015.json | 90 | 0.0134859 |
| PFNS_Lajtai1990.json | 70 | 0.102323 |
| PFNS_Maerten1990_0deg.json | 23 | 0.0201662 |
| PFNS_Maerten1990_60deg.json | 23 | 0.0238484 |
| PFNS_Poenitz1982.json | 52 | 0.0788631 |
| **Total** | **1,030** | — |

Observed structural facts across these 21 files:
- 1,030 values and 1,030 relative-uncertainty entries.
- 1,030 energy-limit values in `data.structure[1].limits`.
- 63,646 flattened correlation entries, exactly (sum_e n_e^2).
- Every correlation array reshapes to (n_e\times n_e).
- Every correlation matrix is symmetric to floating-point precision; maximum observed absolute asymmetry is below (4.5\times10^{-16}).
- All 21 matrices passed an ordinary unpivoted Cholesky factorization with strictly positive pivots.
- No zero-valued observation bins were found in these records.
- No non-finite numeric values were found in the audited numeric arrays.
- Energy-limit arrays were monotonically non-decreasing.

The minimum Cholesky pivot in the table is the minimum positive diagonal Schur pivot before taking its square root in the unpivoted factorization. It is **not** an eigenvalue or a condition number. The very small pivot for `PFNS_Boldeman1986highEout.json` (about (1.89\times10^{-5})) and the small pivot for the Blain high-energy record are flags for a separate conditioning/eigenvalue audit. Passing Cholesky is only a structural positive-definiteness check, not validation of the covariance model or the physical measurements.

## 3. A consequential preprocessing dependency: all experiments are normalized to one reference

The public `AIACHNE/ExecuteStudy.ipynb` loads the pointwise evaluation file:

`Eval_MvUncCf252PFNS_AfterAIACHNEKnowledgePointwise_Upd20250908.dat`

It reads `Eout`, `val`, and `rel_unc`, then normalizes each experiment before compiling the multi-experiment dataset. The notebook calls the public `get_scale_factor` helper with the experimental energy grid and values, and with the common evaluation's `Eout` and `val`. The helper linearly interpolates the reference values onto the experimental grid and computes the ratio of trapezoidal integrals. The observed spectrum is multiplied by that scalar.

Two details matter:
1. The notebook passes the pointwise reference values to the scale-factor function, but not the `rel_unc` field read from the reference-evaluation file.
2. The public `compile_data_dicts` helper combines experiment correlation matrices with a block-diagonal construction. It does not add cross-experiment covariance from a shared uncertain reference.

This is not by itself proof that the published scientific analysis is invalid: the notebook may be a conditional analysis that treats the reference evaluation as fixed. But it is a precise modeling boundary. If CMNI reports uncertainty unconditional on the shared evaluation, then reference uncertainty and the dependence it induces across normalized experiments must be represented or bounded. If the reference is intentionally treated as fixed, that conditioning assumption must be explicit and its impact tested.

Normalization also changes the covariance within each experiment. For observed vector (y), integration weights (w), common reference (r), and a projection (A) from the reference grid to the experiment grid:

[
p=w^Ty,\qquad q=w^TAr,\qquad z=\frac{q}{p}y.
]

The first-order Jacobians are:

[
J_y=\frac{q}{p}\left(I-\frac{yw^T}{p}\right),
\qquad
J_r=\frac{y(w^TA)}{p}.
]

Therefore:

[
C_z\approx J_yC_yJ_y^T+J_rC_rJ_r^T,
]

and for two otherwise independent experiments (j,k) normalized to the same uncertain reference:

[
C_{jk}\approx J_{r,j}C_rJ_{r,k}^T.
]

With a fixed reference integral, the normalized integral is constrained exactly, giving a null covariance direction along the integration weights. With an uncertain shared reference, the common-reference term restores variance in that direction and creates cross-experiment dependence.

## 3A. Reconstructed normalization factors

The pointwise evaluation grid contains 72 strictly increasing energies from 0.0005 to 25.01 MeV. All 21 experimental energy grids are strictly increasing and have no zero-valued observations. Exactly one experiment, `PFNS_Blinov1980002.json`, begins below the evaluation's lower energy boundary (0.000364 MeV). Its first two trapezoidal intervals have left endpoints outside the reference grid and are skipped by the upstream helper. No interval has an in-domain left endpoint followed by an out-of-domain right endpoint.

A candidate parity fixture now records the raw and reference overlap integrals, overlap interval counts, and computed scale factor for all 21 experiments:

`research/cmni/fixtures/aiachne-reference-normalization-v1.json`

The fixture records 1,030 observations, 1,007 included trapezoids, two skipped lower-bound trapezoids, and zero upper-endpoint NaN cases. Its scale factors were reconstructed in-memory using piecewise-linear interpolation and the upstream trapezoidal loop. They have **not** yet been executed against SciPy's `griddata` and the original CmdStan notebook. Treat them as candidate golden values until that independent comparison is performed.

The public Rust kernel now provides:
- `trapezoidal_integration_weights`;
- `linear_reference_projection` (strict in-domain interpolation);
- `aiache_compatible_overlap_geometry` (the notebook's lower-bound skip behavior, with upper-endpoint NaN cases rejected);
- first-order covariance propagation and shared-reference cross covariance.

The overlap geometry is important: simply extrapolating the low-energy Blinov bins or dropping them from the normalized output would not match the notebook. The scale factor is calculated over the valid overlap, then applied to every observation bin.

## 4. CMNI implementation increment

The standalone Rust kernel now contains `shared_reference_normalization.rs`, exported through `lib.rs`. It:
- accepts raw observation covariance, reference covariance, a projection matrix, and explicit integration weights;
- returns the normalized spectrum and scale factor;
- returns separate observation and reference Jacobians;
- returns the propagated within-experiment covariance components;
- computes cross-covariance between two experiments sharing the same reference.

The implementation is a first-order delta-method primitive. It does not silently assume a diagonal reference covariance, because the pointwise reference file supplies marginal relative uncertainties but not a full covariance matrix in the inspected format. A diagonal-reference scenario can be used as a declared sensitivity case, not treated as known truth. Monte Carlo propagation of the nonlinear ratio is a required independent comparison.

Authored Rust tests cover integral preservation, the fixed-reference normalization null direction, induced cross-experiment covariance, and invalid dimensions/non-finite inputs. These tests are not yet reported as executed; exact-head CI must compile, lint, and run them.

## 4A. Bias-basis coefficient ordering and energy-domain parity

The LANL notebook builds three Gaussian basis blocks in short/medium/long order:
- short: 100 centres, width 0.05 in log10 energy (`B_s`);
- medium: 50 centres, width 0.15 (`B_m`);
- long: 10 centres, width 0.3 (`B_l`).

The public Python helper sorts the supplied widths in ascending order before constructing these blocks. The CMNI flattened basis had previously stored the same 160 functions in long/medium/short order. The functions were present, but a flattened coefficient vector could not be treated as a drop-in AIACHNE vector. CMNI now stores short/medium/long order and has regression assertions for the 100/50/10 block boundaries.

The notebook also derives its log-energy basis domain from the pointwise evaluation grid:
- lower endpoint = minimum reference energy minus 0.0001 MeV;
- upper endpoint = maximum reference energy plus 5 MeV.

For the pinned 72-point evaluation grid (0.0005 to 25.01 MeV), this gives 0.0004 to 30.01 MeV. The Rust kernel now exposes `aiachne_bias_basis_energy_domain` to reproduce this recipe explicitly. It must be called with the pinned reference grid; using the observed-data min/max or the unextended reference endpoints would alter every basis centre.

The kernel therefore has separate, explicit reproduction helpers for the upstream energy-grid normalization and the 160-column bias basis. Both remain research code until the independent SciPy/Stan parity checks below pass.

## 4B. Grouped horseshoe and experiment-scale posterior target

The original selected Stan model has separate short-, medium-, and long-range global scales for each feature level, rather than one global scale for the entire 160-column block. CMNI now has a grouped raw-scale horseshoe posterior target with an explicit coefficient-to-global-scale index mapping. For each feature group, its flattened coefficient order is:
- 100 short-range (B_s) coefficients;
- 50 medium-range (B_m) coefficients;
- 10 long-range (B_l) coefficients.

The posterior target accepts one log-global-scale value per group, half-normal or half-Cauchy global priors, half-Cauchy local scales, and a fixed coefficient-scale multiplier corresponding to AIACHNE's `tau_scale`. The helper `aiachne_global_scale_index_by_coefficient` emits the group-major three-scale mapping. The one-group case is regression-tested against the existing scalar-global-scale target.

The target also accepts optional per-experiment scale parameters. When supplied, they:
- are represented in log coordinates;
- have the original AIACHNE Normal(1, 0.01) prior on the physical scale;
- are restricted to [0.5, 1.5];
- include the log-coordinate Jacobian;
- affect the predicted mean;
- leave the likelihood covariance anchored to the unscaled baseline mean, as in the inspected Stan likelihood.

The target rescales its internal relative covariance representation so that the raw-scale covariance remains invariant to the experiment scale. It uses the likelihood derivative with respect to log mean to compute the per-experiment scale gradient. Authored finite-difference tests cover coefficient, local-scale, global-scale, and experiment-scale gradients, plus a rejection test outside the original scale bounds.

**Remaining parity boundary:** the target still receives the baseline interpolation mean as a fixed vector. The original Stan model samples the positive interpolation-grid mean coefficients (the `beta_mean` / `sigma` vector) with its own prior. Their joint posterior and gradient—including the dependence of the relative-error covariance on that mean—are not yet implemented here. The current prepared HMC chain still has one global-scale coordinate and must not be used to sample the grouped target. Tracking issue: Symtropy #6644.

## 4C. Independent methodological evidence for shared-reference dependence

Fukutani et al., “When normalization induces correlation: shared and unstable references can create hidden dependencies,” *npj Systems Biology and Applications* 12, 90 (published 2026-04-21; DOI 10.1038/s41540-026-00722-2), analytically derives that subtracting the same uncertain reference from two otherwise independent variables induces covariance. Their expression for the induced correlation increases with the variance of the shared reference, and their simulations show that the artifact does not disappear merely by increasing sample size. Sequential ratio transformations can be still more unstable when denominators fluctuate.

This is a different application domain, but the statistical mechanism is directly relevant to AIACHNE's shared-reference normalization. It supports treating the evaluation reference as a common uncertainty source, not as independent per-experiment noise. It does not establish that the LANL/AIACHNE analysis is wrong: the original notebook can be interpreted conditionally on a fixed reference. CMNI's obligation is to make that conditioning explicit and quantify the difference when reference uncertainty is included.

CMNI now includes an authored regression test in which two otherwise independent spectra are independently normalized using the same uncertain reference; the cross-experiment covariance computed from the actual reference Jacobians must be nonzero. The test is not yet reported as executed.

Source:
- https://doi.org/10.1038/s41540-026-00722-2
- https://pmc.ncbi.nlm.nih.gov/articles/PMC13287743/

## 4D. Prepared grouped sampler and independent-chain qualification

The standalone kernel now has `PreparedGroupedRawScaleHorseshoeModel`. It constructs the observation covariance from the unscaled baseline mean and relative covariance, computes the Cholesky factor and log determinant once, and reuses both across repeated target evaluations. This is important because AIACHNE's experiment scales alter the mean but the original Stan likelihood anchors the residual covariance to the unscaled physical mean.

A regression test compares prepared and unprepared grouped evaluations for:
- log likelihood and log posterior;
- biased predictions;
- coefficient gradients;
- local/global horseshoe gradients;
- experiment-scale gradients.

The branch also has `run_grouped_horseshoe_hmc_chain` and `run_grouped_horseshoe_hmc_chains`. The phase-space vector includes coefficients, local log-scales, multiple global log-scales, and optional experiment log-scales. The multi-chain runner requires at least four chains and unique seeds, freezes the same HMC configuration across chains, and preserves each chain's draws, Hamiltonian energies, acceptance, divergence and seed receipts. `diagnose_grouped_hmc_quantity` projects a selected coefficient, local scale, global scale or experiment scale into the existing rank-normalized/folded R-hat, bulk/tail ESS, MCSE and E-BFMI diagnostics.

This is fixed-step HMC with no NUTS tree building, dual-averaging adaptation, or mass-matrix adaptation. Unique seeds do not guarantee overdispersed initial states; the initial-state dispersion policy must be preregistered and checked. The tests are authored and not yet reported as executed. No NUTS/BMC or posterior qualification claim is made.

## 5. Next parity experiments

1. Reconstruct the upstream notebook's exact trapezoidal weights and linear interpolation projection on each experimental energy grid.
2. Reproduce its scale factors exactly before fitting any bias model.
3. Compare the upstream conditional covariance path (original per-experiment correlation blocks, fixed reference) against:
   - first-order delta covariance with a declared reference covariance;
   - Monte Carlo normalization draws from the same reference uncertainty;
   - shared-reference cross-experiment covariance retained in the joint likelihood.
4. Run feature-attribution and leave-one-experiment-out tests under both conditional and propagated-reference models.
5. Quantify whether feature inclusion probabilities, bias signs, and the reported spread-reduction factor change under reference uncertainty and alternative normalization anchors.

The purpose is not to assume that the original choice is wrong. It is to expose the reference-conditioning assumption, measure its consequence, and stop CMNI from reporting conditional uncertainties as though they were unconditional.
