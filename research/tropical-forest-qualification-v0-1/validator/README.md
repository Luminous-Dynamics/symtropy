# Tropical forest fixture qualifier

This is a small, offline Rust verifier for the synthetic qualification fixtures in the adjacent `cases/` directory. It dispatches by the manifest identity and emits a deterministic SHA-256-bound JSON receipt.

## Run

From the repository root:

```bash
cargo test --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml

# Synthetic biomass / canonical-stock accounting fixture
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-patch-accounting-synthetic-001.json

# Analytical canopy topology / synthetic light geometry fixture
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/canopy-gap-representation-synthetic-v0.1.json

# Observation-operator synthetic rule corpus (does not run a production adapter)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-observation-operator-negative-controls-v0.1.json

# Synthetic cross-modal evidence dependence (not statistical fusion)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-consilience-dependence-synthetic-v0.1.json

# Variable-aware aggregation (intensive, extensive, counts, categorical)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-variable-aware-aggregation-synthetic-v0.1.json

# Exact rectangular support intersection (synthetic, pre-transformed planar coordinates only)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-support-intersection-synthetic-v0.1.json

# Record-level observation admission envelope (synthetic; no production adapter)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-observation-admission-contract-synthetic-v0.1.json

# Block-aware split conformal calibration (spatial/campaign group units)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-block-conformal-synthetic-v0.1.json

# Binary event Brier / reliability scoring (disturbance event probabilities)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-binary-event-brier-synthetic-v0.1.json

# Probabilistic forest forecast scoring (CRPS, intervals, paired skill, abstention)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-probabilistic-forecast-scoring-synthetic-v0.1.json

# Executable known-truth model-set diagnostics (public synthetic; not blind)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-model-set-known-truth-diagnostics-synthetic-v0.1.json

# Public model-set adequacy development manifest (not a blind test)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-model-set-adequacy-development-v0.1.json

# Process-resolved two-pool forest trajectory and compensating-flux diagnostic
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-process-trajectory-synthetic-v0.1.json

# ForestPaths D-BEN source crosswalk admission (metadata only; does not read archive data)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forestpaths-dben-source-crosswalk-v0.1.json

# Exact structural flux identifiability and measurement subset tournament
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-flux-identifiability-synthetic-v0.1.json

# Public forest decision-value development corpus (uses synthetic arithmetic only)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-decision-value-development-v0.1.json

```

Exit status is 0 for a successful fixture-specific verification (`PASS`, `PASS_SYNTHETIC_RULES`, `PASS_SYNTHETIC_GEOMETRY`, `PASS_SYNTHETIC_SUPPORT`, `PASS_SYNTHETIC_AGGREGATION`, `PASS_SYNTHETIC_CONSILIENCE`, `PASS_SYNTHETIC_MODEL_SET_DIAGNOSTICS`, `PASS_SYNTHETIC_FORECAST_SCORING`, `PASS_SYNTHETIC_BRIER`, `PASS_SYNTHETIC_BLOCK_CONFORMAL`, `PASS_SYNTHETIC_BLOCK_COVERAGE_INFERENCE`, `PASS_SYNTHETIC_SPATIAL_LOCALIZATION`, `PASS_SYNTHETIC_PROCESS_TRAJECTORY`, `PASS_SYNTHETIC_FLUX_IDENTIFIABILITY`, `PASS_SYNTHETIC_FORESTPATHS_CROSSWALK` or `MANIFEST_VALID`), 1 for a semantic `FAIL`, and 2 for malformed JSON, missing required fields, or invalid invocation. The receipt includes the exact input-byte SHA-256 and individual checks.

## CI integration

The standalone package has its own Cargo workspace boundary, so root `cargo fmt --all`, `cargo clippy --workspace` and root workspace checks do not include it. The repository CI workflow now has explicit steps for this manifest: `cargo fmt --manifest-path ... -- --check`, `cargo clippy --manifest-path ... --all-targets --all-features -- -D warnings`, and `cargo test --manifest-path ... --all-targets --all-features`. A queued workflow is not evidence that these steps passed.

## Headless canopy kernel

The exported `canopy_kernel` module provides a minimal spatial cohort-to-cell LAI representation, explicit one-way cohort mortality, derived Beer–Lambert transmittance, four-neighbour gap components and an optical understory eligibility mask. It is a synthetic analytical kernel only; it does not implement crown allometry, multiple scattering, species physiology, recruitment, succession or a calibrated radiation model. Its eight unit tests are included in the library test suite.

## Cross-modal consilience / dependence kernel

The synthetic lineage evaluator checks pairwise source dependence before any probabilistic fusion. It detects shared upstream assets, campaigns, calibration/training datasets, common causes, sites, sensor platforms and processing pipelines. Incomplete lineage or unassessed dependency dimensions produce `DEPENDENCE_UNKNOWN`; distinct product IDs do not imply independence. A fully declared pair with no shared identity receives only `DECLARED_INDEPENDENT_GIVEN_CONDITIONS`, not a proof of statistical independence.

The twelve-case CLI corpus returns `PASS_SYNTHETIC_CONSILIENCE` when the deterministic dispositions match. Three library tests cover corpus execution, shared-upstream mutation of an ostensibly independent pair, and unassessed-dimension mutation. The evaluator does not authenticate a source registry, estimate a correlation or dependence parameter, or perform evidence fusion. See [CONSILIENCE_DEPENDENCE_V0_1.md](../CONSILIENCE_DEPENDENCE_V0_1.md) and [issue #5257](https://github.com/Luminous-Dynamics/symtropy/issues/5257).

## Variable-aware aggregation kernel

The full scientific contract, benchmark targets and claim limits are documented in [VARIABLE_AWARE_AGGREGATION_V0_1.md](../VARIABLE_AWARE_AGGREGATION_V0_1.md).

The exported `aggregation_kernel` distinguishes four incompatible operations:

- **Intensive variables:** area-weighted mean over valid observed overlap only; target unit is declared separately from the expected result. It reports geometric/valid coverage and never extrapolates a partial observed mean to the unobserved target.
- **Extensive stock from density:** integrates `Mg/ha × overlap_m² / 10,000` into Mg over valid observed overlap. This is a unit conversion/integration under the source density's support assumption, not a whole-target imputation.
- **Sampled counts:** uses a Horvitz–Thompson total only for an explicitly declared probability sample with positive first-order inclusion probabilities and consistent frame/design identity. It deliberately does not report a standard error without second-order inclusion probabilities or another justified variance method.
- **Categorical variables:** preserves class IDs and reports area/fraction by class. It never averages category codes.

The twelve-case synthetic CLI corpus covers full and partial intensive means, all-masked data, unit mismatch, full/partial biomass density integration, probability-sample count estimation, invalid inclusion probability, non-probability sampling refusal, categorical fractions, masked categorical support and empty class refusal. Three corpus/mutation tests are added; the kernel adds seventeen unit tests. Receipt status is `PASS_SYNTHETIC_AGGREGATION`.

The bounded kernel does not validate the real sampling design, first/second-order inclusion probabilities, source units or source data; it does not implement arbitrary polygons, raster affine transforms, CRS conversion or survey-design variance. Its outputs remain synthetic research evidence.

## Rectangular observation support kernel

The exported `support_kernel` module qualifies exact axis-aligned rectangles in one already-declared planar coordinate system. It reports target area, geometric overlap area, geometric coverage fraction and valid-data coverage fraction separately. Cloud/no-data footprints may contribute geometric coverage but not valid-data coverage. Partial support is reported as partial; the kernel does not extrapolate. Overlapping source rectangles are rejected to prevent double counting, and CRS mismatches are rejected rather than silently transformed. The eighteen-case machine-readable corpus returns a SHA-256-bound `PASS_SYNTHETIC_SUPPORT` receipt only when the actual kernel agrees with the declared exact-overlap and registration-envelope cases.

Twenty kernel unit tests cover identity, containment, partial and zero overlap, tiled and fragmented coverage, invalid-data masks, mixed validity, registration shift, CRS mismatch, duplicate overlap, invalid geometry, edge-touching tiles, empty evidence, and six deterministic registration-uncertainty envelope controls. The registration helper bounds minimum/maximum overlap under a declared axis-aligned translation interval; four of the eighteen receipt cases exercise these bounds. It is not a probability distribution and assumes a rectangular x/y shift box. The module is a bounded analytical geometry kernel, not a production GIS adapter. It does not implement arbitrary polygons, rotated raster transforms, CRS conversion, geometry repair, probabilistic registration-error distributions, source asset authentication or field validation. Variable-specific intensive/extensive/count/categorical operations are handled by the separate `aggregation_kernel` module. The remaining geometry work is explicit in [#5237](https://github.com/Luminous-Dynamics/symtropy/issues/5237).

## Decision-value corpus

The public forest decision-value corpus defines twenty case families and checks two worked synthetic examples. The arithmetic oracle verifies that high information can have zero decision value when the loss matrix is state-invariant, and independently recomputes the positive-EVSI example (gross 8.8 synthetic loss units; net 5.8 after cost). This is not execution of the existing EVSI policy kernel or a field-survey recommendation.

## Block-aware split conformal calibration

The `conformal_kernel` calibrates at a declared spatial/campaign block unit using the maximum absolute residual within each equal-sized calibration block. It computes the finite-sample split-conformal rank `ceil((n+1)(1-alpha))`, returning an unbounded radius when the requested rank exceeds the number of available blocks. It compares this with a naïve pooled observation-level residual radius and reports observation coverage separately from whole-block coverage.

The nine-calibration-block synthetic corpus returns `PASS_SYNTHETIC_BLOCK_CONFORMAL`: pooled radius 1 / width 2 versus block-maximum radius 9 / width 18. Its held-out blocks yield 90% pooled point coverage, 93.33% block-method point coverage, but only one of three entire blocks is fully covered at the block radius. One shifted negative-control block is reported separately with zero whole-block coverage. This is an intentionally small dependence example, not forest calibration.

The kernel checks unique training/calibration/test identities, disjoint groups, equal block sizes, finite residuals and declared test-regime labels. The receipt labels exchangeability `DESIGNER_DECLARED_NOT_STATISTICALLY_VERIFIED`; group separation is not proof of exchangeability. Thirteen kernel tests and three library corpus/mutation tests cover finite-sample ranks, insufficient calibration, leakage, regime mutation and coverage oracles. See [FOREST_BLOCK_CONFORMAL_BENCHMARK_V0_1.md](../FOREST_BLOCK_CONFORMAL_BENCHMARK_V0_1.md) and [#5285](https://github.com/Luminous-Dynamics/symtropy/issues/5285).

## Binary event Brier scoring

The same `forecast_kernel` module includes Bernoulli-event Brier scoring, paired Brier skill and five fixed-width reliability bins. An abstention is a missing probability, not probability zero; it remains in the eligible-case coverage denominator. The six-case corpus returns `PASS_SYNTHETIC_BRIER` with five issued forecasts, one abstention, mean model Brier 0.222, paired baseline Brier 0.250 and Brier skill +0.112. Its reliability bins are small-sample descriptive diagnostics, not calibration evidence.

The corpus checks each case score, aggregate paired skill, all five bin counts/means/event rates and abstention coverage. Five unit tests cover Brier arithmetic, invalid probabilities, all-abstention behavior, duplicates and mixed horizons; three library tests cover corpus execution and score/reliability mutations. See [FOREST_PROBABILISTIC_FORECAST_BENCHMARK_V0_1.md](../FOREST_PROBABILISTIC_FORECAST_BENCHMARK_V0_1.md) and [#5277](https://github.com/Luminous-Dynamics/symtropy/issues/5277).

## Probabilistic forecast scoring

The `forecast_kernel` scores equally weighted finite ensembles with empirical CRPS, reports nearest-rank central 50/80/95% intervals, empirical WIS, interval coverage and width, ensemble-mean MAE, paired baseline CRPS/WIS skill, and forecast coverage including abstentions. The paired skill convention is `CRPSS = 1 - model_CRPS / baseline_CRPS`; positive is better. Model and baseline are scored on identical issued cases; baseline performance on all eligible cases is reported separately.

The six-case synthetic corpus returns `PASS_SYNTHETIC_FORECAST_SCORING`. Five cases issue forecasts and one abstains. Its expected CRPSS is negative (-0.225): the toy model is worse than the paired point-mass baseline. This is intentional conformance evidence, not a field result. The corpus checks each case-level CRPS/WIS score and the aggregate summary. Twenty-two unit tests exercise CRPS, WIS, quantiles, abstention, paired skill, Brier/reliability bins, confidence-ranked risk–coverage ordering and invalid/mixed cases; eight library tests check continuous score/skill/coverage/WIS/risk-curve oracles.

This v0.1 corpus is restricted to one variable, unit and horizon. It includes a separate six-case confidence-ranked risk–coverage curve; the declared confidence score is not yet calibrated. The corpus does not establish statistical calibration, PIT/reliability, field forecast skill or transfer. See [FOREST_PROBABILISTIC_FORECAST_BENCHMARK_V0_1.md](../FOREST_PROBABILISTIC_FORECAST_BENCHMARK_V0_1.md) and [issue #5277](https://github.com/Luminous-Dynamics/symtropy/issues/5277).

## Block-aware conformal calibration

The `conformal_kernel` compares naïve pooled residual calibration with a split-conformal quantile over **block-maximum residuals**. The score unit is the declared spatial/campaign block, and the block interval uses one common radius across that block. It reports point coverage, simultaneous all-in-block coverage, interval width and 90% Winkler interval score separately. Training, calibration and test group identities are checked for leakage; calibration/test block IDs must also be disjoint.

The nine-block synthetic corpus yields radius 1 for pooled per-observation residuals and radius 9 for block maxima. On its three test blocks, pooled calibration gives 90% point coverage but 0% whole-block coverage; block calibration gives 93.33% point coverage but 33.33% whole-block coverage. The block radius is wider (18 vs 2), which is why Winkler score is reported alongside coverage. The shift block is known by fixture construction; no prospective shift detector exists.

The split-conformal finite-sample guarantee applies only when the calibration and future **block scores are exchangeable**. Group-ID separation is not a statistical proof. With nine calibration blocks, the nominal 90% order statistic is the maximum block score; a zero-miscoverage request is unbounded. The receipt reports `DESIGNER_DECLARED_NOT_STATISTICALLY_VERIFIED`.

See [FOREST_BLOCK_CONFORMAL_BENCHMARK_V0_1.md](../FOREST_BLOCK_CONFORMAL_BENCHMARK_V0_1.md) and [issue #5285](https://github.com/Luminous-Dynamics/symtropy/issues/5285). The empirical tournament should compare block maxima with spatially localized conformal (`spconform`), appropriate temporal methods (EnbPI / SPCI), and online approaches such as UP-OCP / SAOCP—under their respective assumptions, and with coverage and interval efficiency both reported. Recent theory also motivates repeated block-level calibration resampling to measure calibration-conditional coverage variability.

## Executable known-truth model-set diagnostics

The separate `model_set_kernel` evaluates five public analytic counterexamples. It compares candidate historical aggregate fit, pairwise cohort-space forecast disagreement and residual to an explicit validation state. It distinguishes:
- structural disagreement despite aggregate historical fit;
- common structural omission where candidates agree but all miss validation;
- candidate history-fit failure;
- a matched control where no failure is detected.

The five-case CLI corpus returns `PASS_SYNTHETIC_MODEL_SET_DIAGNOSTICS` when the kernel matches the declared arithmetic and dispositions. Eight Rust unit tests exercise the pure kernel; two tests in `lib.rs` exercise corpus execution and a delayed-response truth mutation. This is a toy diagnostic, not a forest process model or a blind detector. The known truth is public and all quantities are synthetic. A `NO_FAILURE_DETECTED_ON_THIS_SYNTHETIC_CASE` disposition is not an adequacy certificate.

See [MODEL_SET_KNOWN_TRUTH_DIAGNOSTICS_V0_1.md](../MODEL_SET_KNOWN_TRUTH_DIAGNOSTICS_V0_1.md) and [#5198](https://github.com/Luminous-Dynamics/symtropy/issues/5198).

## Model-set adequacy

The public forest model-set corpus contains 30 development descriptors (three variants across each of ten families). It is a public case-family contract only: it deliberately exposes no hidden generator equations and returns `MANIFEST_VALID`, not a model-adequacy pass. A real blind tournament requires independent custody of generator identity/outcomes and a precommitted candidate run.

## Fixture-specific result meanings

| Fixture | Successful result | What that result means |
|---|---|---|
| Biomass accounting | `PASS` | The synthetic live/deadwood transitions, declared transfers, total stocks and disconnected-control invariants are internally consistent |
| Canopy geometry | `PASS_SYNTHETIC_GEOMETRY` | The 8×8 analytical grid matches its declared LAI, four-neighbour gap topology and uncalibrated synthetic transmittance equations; ten public challenge-case IDs and expected dispositions are valid. The challenge processes are not executed |
| Observation operators | `PASS_SYNTHETIC_RULES` | The nine synthetic input descriptors are run through the Rust rule engine and produce their declared missingness/support/semantics dispositions; no external data or production adapter is used |
| Rectangular support intersection | `PASS_SYNTHETIC_SUPPORT` | Eighteen exact axis-aligned rectangle cases report geometric and valid-data coverage separately, including four receipt-bound registration envelopes; CRS mismatch, overlap and degenerate geometry are refused |
| Observation admission envelope | `MANIFEST_VALID` | Eleven synthetic record descriptors exercise provenance, rights-purpose, support/time, missingness, qualified absence, domain, semantics and leakage gates; no real asset, production adapter or rights authority is executed |
| Model-set adequacy | `MANIFEST_VALID` | Thirty public development variants (three per family) have known family dispositions and unique IDs; no blind generator or independent custodian is provided |
| Known-truth model-set diagnostics | `PASS_SYNTHETIC_MODEL_SET_DIAGNOSTICS` | Five public toy counterexamples distinguish structural disagreement, common omission, history-fit failure and matched control; no ecological detector is claimed |
| Probabilistic forecast scoring | `PASS_SYNTHETIC_FORECAST_SCORING` | Six cases check empirical CRPS, WIS, paired baseline CRPS/WIS skill, central interval coverage/width and explicit abstention coverage; no empirical calibration is claimed |
| Binary event Brier scoring | `PASS_SYNTHETIC_BRIER` | Six cases check Bernoulli Brier scores, paired baseline skill, five reliability bins and explicit abstention coverage; no event calibration is claimed |
| Block-aware conformal calibration | `PASS_SYNTHETIC_BLOCK_CONFORMAL` | Nine calibration blocks compare pooled and block-maximum radii, report point and whole-block coverage separately, and expose a shifted negative control; exchangeability remains unverified |
| Decision value | `MANIFEST_VALID` | Twenty public development cases, the zero-EVSI counterexample, and the positive-EVSI worked arithmetic are coherent; the existing EVSI policy kernel is not executed by this validator |

## Dataset candidate register

See [DATASET_CANDIDATE_REGISTER_V0_1.json](../DATASET_CANDIDATE_REGISTER_V0_1.json) for five metadata-only source candidates: Amazon RAINFOR / EIDC (OGL), NASA / ORNL DAAC Brazil, DRC NFI, PNG NFI and Brazil NFI. It records spatial/temporal support, access state, permitted use and the next admission gate. No source data have been ingested by committing this register.

## Covered checks

- Fixture version, benchmark identity, synthetic claim ceiling, source scope and exact spatial IDs.
- Non-negative declared flows, parameter/effect consistency and explicit decomposition boundary.
- Live biomass, deadwood, total biomass, mortality transfer and biomass budget.
- Disconnected-control invariance using declared causal links, comparing canonical stocks separately from derived totals.
- Variable-aware aggregation tests for area-weighted intensive means, density-to-stock integration, probability-sample count totals and categorical area composition; synthetic consilience/dependence rules for shared upstream assets, campaign, calibration/training, common causes, site, sensor, pipeline and unknown lineage; record-level observation admission checks for source digest/operator/support completeness, declared rights-purpose, missingness, qualified absence, domain, variable semantics, repeated-census support and split leakage.
- Observation corpus rule execution for support mismatch, no acquisition, label-semantic mismatch, geographic out-of-domain, cloud/no-data, plot leakage, inaccessible DRC units, PNG security/logistics omissions and single-census longitudinal overclaim; production adapter and rights execution remain explicitly not implemented.
- Block-conformal quantile rank, pooled-vs-block calibration, Winkler width/coverage trade-off, exchangeability claim boundary, group leakage, cross-split block identity and known-shift negative control.
- Decision-value corpus coverage, public-not-blind boundary, zero-EVSI loss-invariance case and positive-EVSI arithmetic from the declared binary likelihood/loss model.
- Canopy coordinate bounds, duplicate/overlap/coverage, total and mean LAI, connected gap components, synthetic transmission, uniform-baseline calculations, and ten unique named challenge-family dispositions.
- Deterministic receipt generation and adversarial mutation tests.

## Claim limits

These results are deliberately narrow:

- Biomass fixture PASS does not validate ecological growth or mortality equations.
- Headless canopy-kernel tests validate the declared synthetic cohort-support operations only; they do not validate biological crown geometry, radiative transfer or recruitment.
- Canopy geometry PASS does not validate radiative transfer, crown allometry, tree physiology or recruitment; the ten challenge-case dispositions are contract checks, not executed canopy processes.
- Observation `PASS_SYNTHETIC_RULES` means only that the nine machine-readable synthetic input descriptors triggered the intended Rust rule dispositions; it does not mean a production observation adapter rejected real-world invalid claims.
- Decision-value manifest validity does not measure an actual policy's EVSI performance or authorize a field visit.
- None establishes a real forest forecast, field calibration, regional transfer, SOTA superiority or authority to intervene.

The original core suite contains 144 Rust unit tests: 52 in `lib.rs`, 8 in the headless canopy kernel, 20 in the rectangular support/registration kernel, 17 in the variable-aware aggregation kernel, 8 in the known-truth model-set kernel, 22 in the probabilistic/event-forecast kernel and 17 in the block-conformal kernel. Additional suites add 7 block-coverage kernel tests, 15 spatial-localization/Moran/k-neighbour tests, 7 prior CLI receipt-adapter tests, 12 process-trajectory tests, and 10 flux-identifiability tests (9 kernel + 1 top-level dispatch/receipt), for 206 authored Rust test functions. All remain subject to exact-head Cargo execution. The validator is a separate research workspace to keep dependency and compile scope small. It does not change production `symtropy-basin` APIs or canonical Living World ownership.


## Spatial localization and residual autocorrelation kernels

The block-coverage and spatial-localization modules are exported and wired into the fixture CLI receipt dispatcher. Their kernels are exercised by Rust unit tests and their public corpora emit SHA-256-bound synthetic receipts:

- `coverage_inference_kernel` computes an assumption-labelled Hoeffding interval over held-out block-coverage indicators. The number of blocks is the effective sample size; within-block pixel/stem count is never substituted. Independence is declared, not verified.
- `spatial_localization_kernel` computes a Gaussian distance-weighted residual quantile for one declared projected metric coordinate system, local effective sample size, maximum normalized weight and low-support abstention. Gaussian similarity is not a density ratio, so the output is explicitly diagnostic-only and has no finite-sample spatial coverage guarantee.
- `spatial_localization_kernel::global_morans_i` reports a descriptive global Moran's I statistic using symmetric binary distance-neighbour weights on signed held-out residuals. It provides no permutation p-value, boundary correction or inferential significance.

Research owners:
- [#5319 — realized block-coverage inference](https://github.com/Luminous-Dynamics/symtropy/issues/5319)
- [#5327 — spatial localization tournament](https://github.com/Luminous-Dynamics/symtropy/issues/5327)
- [#5328 — sealed model-set custody](https://github.com/Luminous-Dynamics/symtropy/issues/5328)

The spatial-localization and block-coverage corpora now produce digest-bound synthetic receipts through the CLI. The receipt adapters add mutation controls and explicit claim ceilings. They do not run Rust in the current runtime, ingest real data, or establish ecological validity.

## Block-coverage and spatial-localization CLI receipts

The new evaluation kernels are now wired into the same deterministic CLI receipt path as the earlier synthetic corpora:

```bash
# Realized whole-block coverage uncertainty; block count is the evaluation unit
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-block-coverage-inference-synthetic-v0.1.json

# Spatially localized residual intervals, local ESS, abstention and Moran's I
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-spatial-localization-synthetic-v0.1.json
```

The CLI emits source-manifest SHA-256, per-case checks and explicit claim ceilings. Success result names are `PASS_SYNTHETIC_BLOCK_COVERAGE_INFERENCE` and `PASS_SYNTHETIC_SPATIAL_LOCALIZATION`. These are synthetic arithmetic/diagnostic receipts only; neither claims ecological calibration, spatial conditional validity, independent block structure or field performance.

The `forest_corpus_receipts` module adds mutation tests for interval-oracle corruption, pseudo-replication acceptance, local undercoverage mutation and Moran reference mutation. Those tests are authored but have not been run in the current environment.

# Optional k-nearest-neighbour spatial diagnostic

The spatial-localization corpus now includes an optional `nearest_neighbor_count`. When present, the validator limits the Gaussian diagnostic to the nearest k calibration records and records the selected physical neighbourhood radius. This is a diagnostic control; it must not be described as implementing the statistical guarantees of spconform. The new frozen fixture demonstrates a pooled radius of 50 versus a k=12 local diagnostic radius of 0.5 over an 11 m neighbourhood.

The package now has 206 authored Rust test functions: 144 original core tests, 7 block-coverage kernel tests, 15 spatial localization/Moran/k-neighbour tests, 7 prior CLI receipt-adapter tests, 12 process-trajectory tests, 10 flux-identifiability tests (9 kernel + 1 top-level receipt dispatcher), and 11 ForestPaths crosswalk tests (10 kernel + 1 top-level receipt dispatcher). They remain authored but unexecuted until exact-head Rust verification.


## Process-resolved forest trajectory kernel

The exported `trajectory_budget_kernel` module evaluates a bounded two-pool reference (live biomass and deadwood) with distinct growth, recruitment, mortality transfer, live export, decomposition and deadwood export. It checks increasing period identities, within-trace stock continuity, non-negative declared fluxes, live/deadwood/total budget residuals, every census-boundary stock, and each primitive flux independently.

The four-case synthetic corpus returns `PASS_SYNTHETIC_PROCESS_TRAJECTORY` when the kernel detects: a full process match; identical stock trajectories with compensating growth/mortality/decomposition errors; a valid budget with a wrong stock trajectory; and a pool budget that does not close. In the compensating case, stock error is exactly zero while the largest primitive-flow error is 20 synthetic biomass units. Eleven kernel Rust unit tests are authored, including per-flow oracle mutation, missing/negative-flow, reordered-period, discontinuity and initial-census mismatch controls. One additional library test exercises top-level corpus dispatch and the SHA-256 receipt. At that stage the package inventory was 185 authored tests; the subsequent flux-identifiability increment raises the current total to 195. All remain unexecuted until exact-head Rust verification.

Run the CLI command shown above. The receipt binds the exact fixture SHA-256 and reports `ecological_validation: NOT_ESTABLISHED`. This is an arithmetic benchmark oracle, not a process model or real-forest validation. See [FOREST_PROCESS_TRAJECTORY_V0_1.md](../FOREST_PROCESS_TRAJECTORY_V0_1.md) and [#5340](https://github.com/Luminous-Dynamics/symtropy/issues/5340).


## Exact flux identifiability and measurement design

The exported `flux_identifiability_kernel` uses exact rational row reduction over bounded integer observation matrices. For the six-flux live/deadwood balance, stock increments alone have rank 2 and nullity 4. Four independent nullspace witnesses show which changes in growth, recruitment, mortality transfer, exports and decomposition cannot be distinguished from stock observations alone.

The kernel exhaustively enumerates all 64 subsets of six optional direct measurement channels and reports rank/nullity, measurement count, declared synthetic effort, and every tied minimum-cost full-rank design. The frozen synthetic oracle finds 18 full-rank subsets and two tied minimum-cost designs at 14 synthetic effort units, each with four measurements. These effort values are fixture inputs, not real-world costs or a field survey recommendation.

The corpus returns `PASS_SYNTHETIC_FLUX_IDENTIFIABILITY` and includes exact source-manifest SHA-256. Nine kernel tests cover rank/nullity, nullspace witnesses, design enumeration, tied optima, cost mutation and malformed designs; one top-level test checks dispatcher/receipt behavior. It does not model observation noise or establish practical identifiability. See [FOREST_FLUX_IDENTIFIABILITY_V0_1.md](../FOREST_FLUX_IDENTIFIABILITY_V0_1.md), [#5349](https://github.com/Luminous-Dynamics/symtropy/issues/5349), and the empirical ForestPaths archive follow-on [#5352](https://github.com/Luminous-Dynamics/symtropy/issues/5352).


## ForestPaths / D-BEN source-admission crosswalk

The ForestPaths analysis repository is not a universal D-BEN schema. Its model-specific adapter functions translate filenames, run identifiers, forcing IDs, variable aliases and unit transforms. The public source audit is recorded in [FORESTPATHS_D_BEN_ADMISSION_V0_1.md](../FORESTPATHS_D_BEN_ADMISSION_V0_1.md), and the machine-readable source crosswalk is in [forestpaths-dben-source-crosswalk-v0.1.json](../cases/forestpaths-dben-source-crosswalk-v0.1.json).

The crosswalk preserves nine exact model-helper labels, FIN/BIA/BCI, P0-P7 run-name mappings, forcing identifiers, 24 quantity definitions, four composite/turnover warnings, model-specific unit conversion paths and five distinct size-class families. It also records the visually selected model/site equilibrium windows from the published R Markdown. Its structural checks (9 model names, eight run mappings each, and three 9-wide equilibrium tables) were independently inspected from the JSON. This is source-code admission only; it does not validate NetCDF data or reproduce the paper.

The core traps are consequential: `cmort` includes mortality or other turnover, BiomeE `WDgrowTot` includes reproduction and migration, BiomeE `WDmortTot` includes density-related killing and disturbance removal, and size-class bins differ by model. Every transformation must bind raw units, source variable, model, run and output unit before cross-model scoring. See [#5357](https://github.com/Luminous-Dynamics/symtropy/issues/5357) and empirical reproduction gate [#5352](https://github.com/Luminous-Dynamics/symtropy/issues/5352).


## Executable ForestPaths / D-BEN crosswalk admission

The exported `forestpaths_crosswalk_kernel` validates the public source-code crosswalk before it is used to read or harmonize model outputs. It checks the nine model identities, P0-P7 run mapping coverage, site and visually selected equilibrium-window arrays, unique variable/unit declarations, source blob identity fields, per-model forcing IDs and unit-conversion paths, composite growth/mortality semantics, distinct model-specific size-class families, and separation of data/code/R Markdown DOI references.

A successful receipt returns `PASS_SYNTHETIC_FORESTPATHS_CROSSWALK`, binds the exact JSON bytes with SHA-256, and explicitly states `data_ingested=false`, `model_outputs_validated=false`, and `empirical_reproduction=NOT_RUN`. It does **not** authenticate the upstream GitHub source, inspect data archive bytes, or prove the declared variable semantics are scientifically correct. Ten kernel tests and one top-level dispatch/receipt test are authored. See [FORESTPATHS_D_BEN_ADMISSION_V0_1.md](../FORESTPATHS_D_BEN_ADMISSION_V0_1.md), [#5357](https://github.com/Luminous-Dynamics/symtropy/issues/5357), and the separate archive-reproduction owner [#5352](https://github.com/Luminous-Dynamics/symtropy/issues/5352).
