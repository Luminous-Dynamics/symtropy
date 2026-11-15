# Spatially Localized Forest Calibration v0.1

**Tracking:** [FOREST-SPATIAL-CONFORMAL-001 #5327](https://github.com/Luminous-Dynamics/symtropy/issues/5327)  
**Composes:** #5277 forecast scoring, #5285 block conformal, #5319 realized coverage inference, #5237 support conversion and #5257 consilience  
**Status:** bounded point-reference diagnostic kernel and public synthetic reference cases; no spatial validity guarantee

## 1. Scientific question

Pooled marginal coverage can conceal a spatial failure. A forest predictor may cover 90% of observations overall while covering none of the observations in a remote site, disturbance edge, dry-season pocket or distinct canopy regime.

The model should not receive a good reliability score merely because a large easy region overwhelms a small but decision-critical failing region.

The first benchmark therefore asks whether uncertainty intervals remain locally informative and whether local data support is sufficient to issue one at all.

## 2. Bounded point-reference kernel

The Rust library module is:

research/tropical-forest-qualification-v0-1/validator/src/spatial_localization_kernel.rs

It accepts one explicit projected metric coordinate-system identity, coordinates in metres, calibration residuals, test predictions/observations, a Gaussian bandwidth, alpha and a minimum effective sample size.

For test location x and calibration location x_i, the local similarity weight is:

w_i(x) = exp(-0.5 * (distance(x, x_i) / h)^2)

where h is the declared bandwidth in metres.

The local effective sample size is:

ESS(x) = (sum_i w_i(x))^2 / sum_i w_i(x)^2

The maximum normalized weight is also reported. These diagnostics expose a local sample dominated by one calibration observation even where the raw calibration count is large.

The weighted residual quantile uses an explicit test-point mass at infinity:

target_mass = (1 - alpha) * (sum_i w_i(x) + 1)

The smallest sorted residual whose cumulative local weight reaches target_mass becomes the radius. If local mass is insufficient, or ESS is below the declared floor, the kernel abstains and returns no finite interval.

This construction is included as an experimental diagnostic comparator only. The Gaussian weights are proximity similarities, not a proved calibration/deployment density ratio. They do not establish weighted exchangeability or finite-sample spatial coverage. The output status is:

EXPERIMENTAL_LOCALIZED_DIAGNOSTIC_NO_GUARANTEE

The module returns the ordinary pooled split-conformal radius over the same calibration residuals as a comparison point. That pooled comparator has its own exchangeability assumptions and is not a spatial guarantee.

The coordinate system must already be projected and metric. CRS transformation, geodesic distance, polygon/areal support, graph construction and registration uncertainty are outside this kernel; coordinates in longitude/latitude degrees must not be passed as metres.

## 3. The synthetic local-collapse counterexample

The public corpus is:

cases/forest-spatial-localization-synthetic-v0.1.json

The central fixture has two spatial regions. Both have ten calibration points with residual 0.5. At test time:

- Region A has 90 observations with residual 0.5.
- Region B has 10 observations with residual 8.0.

The pooled radius is 0.5. Pooled marginal coverage is 90%, but Region B coverage is 0%. Localized outputs at A and B each report their own radius, ESS and coverage; the system must not report only the 90% aggregate number.

Other fixtures cover:
- co-located calibration points and exact ESS;
- a geographically unsupported test point with zero numerical local mass;
- a narrow bandwidth that produces inadequate local weighted mass;
- a wider bandwidth that increases local weight mass;
- an explicit minimum-ESS abstention gate.

The JSON corpus is a public arithmetic specification. It is not loaded by a production receipt path in this slice. Eight Rust unit tests are authored in the library module; no Rust execution is claimed.

## 4. What is intentionally not claimed

This kernel is not an implementation of the full spconform method and must not be described as outperforming it. It does not yet implement:
- geostatistical conditional-validity theory;
- areal graph-neighbourhood weighting;
- spatially varying density-ratio estimation;
- boundary correction or spatial block-bootstrap uncertainty;
- Moran's I / residual variograms;
- calibration-set resampling stability;
- ERT conditional-coverage diagnostics;
- hierarchical site/region uncertainty.

Its role is to provide a small, reviewable baseline and a useful synthetic negative control before a matched external tournament.

## 5. Industry and research comparator protocol

Compare on the exact same held-out sites, spatial supports, predictions, calibration data and compute budgets:

1. Pooled split conformal.
2. Block-maximum split conformal, where block exchangeability is justified.
3. The present Gaussian local-weight diagnostic.
4. spconform geostatistical localized split conformal for point-referenced data.
5. spconform areal graph-neighbourhood conformal for lattice/polygon units.
6. Stable localized conformal / transductive comparators where the source-task and target-task assumptions fit.

The tournament must report:

- marginal observation coverage and simultaneous whole-block coverage separately;
- local conditional coverage by site, biome, disturbance regime, sensor, campaign and boundary-distance stratum;
- confidence intervals whose independent unit matches the sampling design;
- CRPS / WIS / Winkler interval score and width at matched realized coverage;
- local ESS, maximum normalized weight and abstention/unbounded-interval rate;
- calibration-set interval-size stability under repeated block-level calibration resampling;
- spatial residual autocorrelation, Moran's I and boundary effects;
- ERT conditional-coverage diagnostics;
- worst-site and worst-region result, not only pooled averages;
- runtime, memory, data volume and field cost.

Do not optimize bandwidth on the test set. Freeze the kernel, bandwidth-selection rule, minimum ESS, support definition and scoring thresholds inside the training/calibration protocol. Keep the final regional holdout untouched.

## 6. Current research basis

- The CRAN spconform vignette documents point-referenced locally weighted split conformal and areal graph-neighbourhood conformal, with marginal coverage, WIS, spatial strata, boundary effects and Moran's I diagnostics: https://cran.r-project.org/web/packages/spconform/vignettes/spconform-intro.html
- Braun, Holzmüller, Jordan & Bach (ICML/PMLR 306, 2026), *Conditional Coverage Diagnostics for Conformal Prediction*, formulates conditional-coverage violation as a classification problem and introduces excess risk of target coverage (ERT). The paper reports improved power over simpler metrics in its experiments: https://proceedings.mlr.press/v306/braun26c.html
- Min, Peng & Zou (ICML/PMLR 306, 2026), *Stable Localized Conformal Prediction via Transduction*, studies variance of prediction-set size conditional on the calibration sample and uses labelled source-task plus unlabeled target data to improve stability: https://proceedings.mlr.press/v306/min26d.html
- Ramos, Graziadei & Cabezas (PMLR 329, 2026), *Conformal Prediction via Transported Beta Laws*, distinguishes calibration-conditional coverage variability, clustered calibration and test-side shift: https://proceedings.mlr.press/v329/r-ramos26a.html

These are complementary targets. ERT diagnoses conditional coverage, StCP addresses calibration-set size stability, and transported Beta laws characterize calibration-conditional coverage distributions. None should be collapsed into the single local weighted-radius diagnostic.

## 7. Proposed acceptance and claim ceiling

Proposed tournament gates, not current results:

- local undercoverage pockets must be detected at preregistered statistical power and controlled false-alarm rates;
- 90% and 95% local-coverage claims must include stratum sample size and matched uncertainty intervals;
- no region may be hidden by aggregate pooling;
- local intervals must be compared at matched coverage, with width and abstention cost shown;
- calibration-set stability must be reported by repeated resampling at the independent site/campaign level;
- any method's spatial dependence assumptions must be declared and checked against the sampling design;
- data-source rights, support, lineage and split leakage remain hard gates.

A synthetic pass would establish only the bounded arithmetic and fail-closed diagnostics. It does not establish conditional coverage, spatial exchangeability, transfer, ecology, or superiority to any external method.
