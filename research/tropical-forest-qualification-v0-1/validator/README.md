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

# Public forest decision-value development corpus (uses synthetic arithmetic only)
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-decision-value-development-v0.1.json

```

Exit status is 0 for a successful fixture-specific verification (`PASS`, `PASS_SYNTHETIC_RULES`, `PASS_SYNTHETIC_GEOMETRY`, `PASS_SYNTHETIC_SUPPORT`, `PASS_SYNTHETIC_AGGREGATION`, `PASS_SYNTHETIC_CONSILIENCE`, `PASS_SYNTHETIC_MODEL_SET_DIAGNOSTICS`, `PASS_SYNTHETIC_FORECAST_SCORING`, `PASS_SYNTHETIC_BRIER`, `PASS_SYNTHETIC_BLOCK_CONFORMAL` or `MANIFEST_VALID`), 1 for a semantic `FAIL`, and 2 for malformed JSON, missing required fields, or invalid invocation. The receipt includes the exact input-byte SHA-256 and individual checks.

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

The validator currently contains 137 Rust unit tests: 49 in `lib.rs`, 8 in the headless canopy kernel, 20 in the rectangular support/registration kernel, 17 in the variable-aware aggregation kernel, 8 in the known-truth model-set kernel, 22 in the probabilistic/event-forecast kernel and 13 in the block-conformal kernel. These tests are authored but must still pass exact-head Cargo execution. The validator is a separate research workspace to keep dependency and compile scope small. It does not change production `symtropy-basin` APIs or canonical Living World ownership.
