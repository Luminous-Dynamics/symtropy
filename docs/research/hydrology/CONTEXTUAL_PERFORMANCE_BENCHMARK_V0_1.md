# Catchment-Conditioned Hydrology Performance Benchmark v0.1

**Status:** Research protocol / schema proposal; not a qualified benchmark implementation  
**Date:** 2026-11-15  
**Parent:** Symtropy #5338  
**Existing validation authority:** Symtropy HYDRO-VALID-05 #461

## 1. Purpose

This protocol adds a defensible performance-comparison layer to the existing hydrology qualification corpus. It answers a question the ordinary leaderboard does not:

> Given this physical process, catchment, observation record, forcing resolution, evaluation horizon and intended decision, what performance is attainable, what is independently demonstrated, and what additional value does a more complex model provide?

The protocol does not replace the existing physical validity gates. A model must still conserve water, obey its declared numerical scheme, pass its analytical/reference tests and preserve cross-fidelity transfer semantics. A high prediction score cannot compensate for physical invalidity.

## 2. Research basis

### 2.1 Context-conditioned attainable bounds

Seibert, Vis and Pool (2026), *Setting the bar: benchmarks for model performances in large-sample hydrology*, Hydrology and Earth System Sciences 30, 5857–5871, DOI [10.5194/hess-30-5857-2026](https://doi.org/10.5194/hess-30-5857-2026), evaluated lower and upper performance references across 13 CAMELS datasets. Their study found wide catchment variation and linked attainable skill to factors including runoff ratio, aridity, high/mean flows and low-precipitation frequency.

For their HBV-light experiment, a random-parameter lower-benchmark ensemble stabilized at around 1,000 members. Their upper benchmark was estimated using repeated calibration; it is an optimistic calibrated reference, not an independently demonstrated future forecast. The paper also distinguishes the score of an ensemble-mean hydrograph from the median score of individual ensemble members. The ensemble mean preserves water balance but may not correspond to a physically realizable parameter vector.

**Transfer limit:** the 1,000-member result is a useful replication point for a comparable HBV-style experiment. It is not a universal required sample count for all solvers, parameter spaces or physical processes. Our evaluator must measure convergence and cost for the actual admitted profile.

### 2.2 Structural uncertainty and model complexity

Mangukiya et al. (2026), *Benchmarking and Selecting Optimal Hydrological Models for Large-Sample Applications Considering Complexity and Uncertainty*, Water Resources Research, DOI [10.1029/2026WR044753](https://doi.org/10.1029/2026WR044753), compared 47 conceptual rainfall-runoff structures across 159 watersheds in Peninsular India. The study reported wide differences across catchments and structural equifinality: distinct model structures could achieve near-similar scores.

**Transfer limit:** this is evidence that structure and complexity must be assessed alongside performance. It is not a global ranking of hydrology engines and does not establish that a model with a higher score is physically superior.

## 3. Evaluation object and evidence classes

Every result is bound to the tuple:

`(process, spatial unit, temporal horizon, intended decision, observation population, model profile)`

Do not mix catchments, processes, temporal resolutions, model domains or evidence classes into a single score without an explicit and defensible aggregation contract.

Retain the evidence classes already established by #461:

- structural conservation / invariant evidence;
- analytical-solution agreement;
- reference-model differential evidence;
- empirical calibration evidence;
- independent temporal/spatial predictive evidence;
- physical test-cell evidence;
- presentation/gameplay plausibility (not a scientific accuracy class).

One evidence class does not silently promote another. A reference-model match does not prove Earth realism; a calibrated historical fit does not prove future predictive skill; a field correlation does not validate an untested flood regime.

## 4. Baseline ladder

Every admitted benchmark task should report the applicable baseline levels separately.

### B0 — no-skill, persistence and data-only baseline

Examples, only where they fit the process:
- zero-flow/no-input control;
- previous-step persistence;
- seasonal climatology using interannual daily means and medians;
- simple recession/storage or area-ratio baseline with a declared domain and assumptions.

For streamflow forecasting, compute both the interannual daily mean and median from the training portion. Select or compare them using a validation procedure fixed before final evaluation. Do not select the stronger one after inspecting the final holdout.

For a flood event, use an event-matched operational or simple hydrologic reference, not a seasonal climatology that cannot represent the event task.

### B1 — uncalibrated model-prior baseline

Run the model under a versioned parameter prior/range with reproducible seeds. Retain:
- the full prior source, bounds and distributions;
- invalid/unstable run counts and exclusion reasons;
- individual-member trajectories and scores;
- ensemble-mean trajectory and score;
- median-member score;
- water-balance residual per member;
- runtime, memory and compute budget.

Estimate convergence of the ensemble statistic. A suggested screening sequence is 100, 250, 500 and 1,000 members, with larger ensembles only where stability has not been reached. The stopping rule must use a predeclared tolerance and repeated independent seeds. Record the cost of reaching it.

Do not silently discard physically invalid members to improve the score distribution. Report them as invalid under the physical gate.

### B2 — calibrated attainable reference

Use repeated calibration on a declared calibration period. Freeze:
- model structure and parameterization;
- parameter bounds and priors;
- optimizer, objective functions and stopping criteria;
- number of starts and total compute budget;
- warm-up and initial-state procedure.

Report the whole distribution across starts and the best calibration result. This is an attainable/in-sample reference; it must not be described as holdout performance or a prediction guarantee.

### B3 — temporal / event holdout

Evaluate the frozen model and decisions on chronological future periods and fully held-out events. Keep complete storm, flood, drought, snowmelt or operational episodes within a single split. Report event counts and regime coverage, not only time-step counts.

### B4 — spatial / population transfer

Hold out entire catchments, sub-basins or networks. Group nested/upstream-downstream catchments where routed signals or shared forcing make leakage possible. Report donor-to-recipient parameter transfer and applicability separately from local calibration.

### B5 — external reference / physical evidence

Use analytical solutions, independently configured reference tools, laboratory/test-cell measurements or qualified field observations within their actual domain. Pin tool versions, settings, source data and uncertainty. Reference output is a comparison target, not a substitute for empirical truth.

## 5. Context vector

Each catchment/process result records available contextual attributes. Unsupported or unknown fields remain null with a reason; do not fabricate completeness.

### Hydroclimatic context

- aridity / water-limitation index and its exact derivation;
- runoff ratio and baseflow contribution;
- mean and high-flow statistics, including Qmean/Q95 where defined;
- precipitation intermittency, dry-spell frequency and event concentration;
- seasonality, rain/snow phase and melt regime where applicable;
- evapotranspiration relevance and forcing source.

### Physiographic and anthropogenic context

- catchment area, elevation range, relief and slope distribution;
- soil / geology / permeability class and source;
- land cover, imperviousness and urban fraction;
- lakes, wetlands, reservoirs, dams, abstractions, diversions and return flows;
- channel geometry and network connectivity where applicable;
- groundwater abstraction / pumping influence where applicable.

### Observation and forcing quality

- observation source, release and rights;
- spatial and temporal resolution;
- missingness, imputation and quality flags;
- gauge density, measurement method and uncertainty;
- rating-curve vintage / stability where known;
- input forcing source, bias correction and resolution;
- clock, calendar, time zone and timestamp semantics;
- calibration/evaluation period lengths and independent event counts.

A performance result without sufficient context may still be recorded, but its comparison class must be downgraded rather than guessed.

## 6. Metric families

Keep metrics process-specific. Do not average fundamentally different quantities into a single unqualified realism score.

### Rainfall-runoff and streamflow

- NSE, KGE and NPE where mathematically applicable and fully specified;
- log-flow / low-flow performance;
- cumulative volume bias and water-balance residual;
- peak-flow magnitude and quantile error;
- peak timing, event hydrograph shape and recession;
- probabilistic interval coverage, width and calibration;
- error stratified by flow regime, season and event type.

### Groundwater

- head and storage error/bias;
- recharge and baseflow timing/volume;
- hydraulic gradient and boundary behavior;
- drawdown/recovery behavior;
- groundwater/surface-water exchange closure.

### Channel, floodplain and inundation

- discharge, stage and depth error;
- peak magnitude, travel time and attenuation;
- inundation extent intersection/union and boundary-distance error;
- duration-above-threshold error;
- wetting/drying and stability violations;
- volume exchange and interface reconciliation residuals.

### Wetlands and reservoirs

- stage-volume relationship error;
- inflow/outflow and release error;
- peak attenuation and lag;
- groundwater exchange and recession;
- inundation duration and physically grounded ecological observables.

All metrics must bind units, reference, valid time, aggregation interval, treatment of missing observations, direction of improvement and uncertainty method.

## 7. Contextual attainment measures

The primary report must show raw metric values, B0–B5 results and per-context stratum distributions.

A normalized attainment indicator may be computed only for a single metric, task and matched evaluation period where the baseline gap is stable:

`A = (S_model - S_lower) / (S_upper - S_lower)`

where metric direction has first been normalized so that higher is better.

Rules:
- preserve and publish `S_model`, `S_lower`, and `S_upper` alongside `A`;
- do not clip values to [0,1]; values below 0 or above 1 are diagnostically meaningful;
- abstain if the denominator is zero, near-zero under a declared numerical tolerance, or the upper/lower bounds are not comparable;
- do not calculate this indicator across different tasks, data populations or evidence classes;
- never use it as a substitute for water conservation, uncertainty calibration, tail-risk or operational decision metrics.

A model's improvement over the lower reference is not automatically decision value. Report whether the improvement changes a real decision and whether the change is independently supportable.

## 8. Structural uncertainty and complexity

For each process/catchment, report:
- model structure IDs and exact versions;
- parameter count, prior/bounds and calibration budget;
- distribution of scores across structures and starts;
- count of models within predeclared near-equivalence margins;
- sensitivity and identifiability evidence where available;
- runtime, peak memory, energy/compute cost where measured;
- data requirements, preprocessing burden and maintenance complexity;
- decision-specific benefit over the simplest adequate baseline.

Model selection should be Pareto-aware. A model that gains a small amount of pooled NSE but worsens flood timing, low-flow reliability, transferability or computational cost may not be the better engineering choice.

## 9. Split and leakage protocol

1. Freeze source identities, versions, licences, catchment IDs, units and time semantics before creating splits.
2. Define temporal, event and spatial groups before calibration or feature engineering.
3. Keep each complete event in one split; no windows from the same event may straddle train and test.
4. Keep hydrologically nested or routed catchments grouped unless the benchmark explicitly measures that transfer.
5. Fit climatology, normalization, imputation, feature selection and calibration only on allowed training data.
6. Select configurations on validation data; touch the final test split once under a frozen protocol.
7. Preserve native measurement cadence; do not manufacture high-frequency information from hourly or daily records.
8. Use event/block/season resampling appropriate to autocorrelation and dependence. IID time-step bootstrap is not presumed valid for every hydrologic series.
9. Report effective independent event counts, uncertainty intervals and missingness.
10. Bind every receipt to exact dataset manifest, split manifest, model/configuration, toolchain and code revision.

## 10. Required adversarial negative controls

A benchmark harness must reject or flag:
- climatology computed using final test observations;
- normalization or gap-filling fit using future observations;
- a storm event split between train and test;
- upstream/downstream nested catchments leaking routed hydrographs;
- parameter calibration using held-out flood peaks;
- model selection after inspecting final test scores;
- synthetic training score presented as utility/field evidence;
- missing gauge data interpreted as zero flow;
- unresolved dam release / abstraction treated as known natural flow;
- time-zone/calendar mismatch hidden by resampling;
- score improvement achieved by violating water balance;
- invalid or unstable ensemble members omitted without disposition.

The observed-data cases and synthetic attacks must remain distinguishable.

## 11. Acceptance gates

- Existing #461 conservation, analytical and differential evidence classes remain intact.
- One frozen reference profile demonstrates B0/B1/B2/B3 over the same task and population.
- The lower-ensemble convergence rule is measured on the actual profile; 1,000 members are not assumed universally.
- The calibrated reference is labelled in-sample/attainable and never passed off as independent validation.
- Per-catchment and per-regime distributions are reported with the pooled summary.
- Physical validity and water conservation are hard gates outside performance aggregation.
- Model equifinality and complexity are reported.
- All leakage negative controls fail closed.
- Exact data/configuration/model/toolchain receipts are reproducible.
- No SOTA, field superiority or operational suitability claim is made without matched independent evidence.

## 12. South African / Highveld application profile

Johannesburg/Highveld is a candidate regional application profile, not a dataset already admitted by this document.

The first admission must identify a permitted catchment and obtain source-specific rights for rainfall, streamflow, reservoir/abstraction operations, land cover, soils/geology and any groundwater observations. A catchment with sparse, uncertain or discontinuous observations can still be studied, but the attainable envelope and claim ceiling must reflect those limitations.

Do not treat a synthetic Highveld forcing profile as measured local hydrology. Do not infer groundwater recharge, flood risk or municipal water availability from regional rainfall alone.

## 13. Exit artifacts

- Versioned run manifest schema.
- Baseline ladder report template.
- Dataset and split manifest requirements.
- Adversarial split-leakage vectors.
- One reference profile with B0/B1/B2/B3 calculations.
- Exact-head evaluator and reproducible receipts.
- Review linking the benchmark to #461 without duplicating numerical solvers.

## 14. References

- Seibert, J., Vis, M. & Pool, S. (2026). *Setting the bar: benchmarks for model performances in large-sample hydrology*. HESS 30, 5857–5871. https://doi.org/10.5194/hess-30-5857-2026
- Mangukiya, N.K. et al. (2026). *Benchmarking and Selecting Optimal Hydrological Models for Large-Sample Applications Considering Complexity and Uncertainty*. Water Resources Research. https://doi.org/10.1029/2026WR044753
- Symtropy HYDRO-VALID-05 #461 and HYDRO-LAB-01 #445.


## South African / Highveld source admission

Candidate local source populations are tracked separately in [HIGHVELD-HYDRO-DATA-ADMISSION-001 #5350](https://github.com/Luminous-Dynamics/symtropy/issues/5350) and the metadata-only [Highveld source admission register v0.1](highveld-source-admission-register-v0.1.json).

The register distinguishes:
- DWS audited surface-flow records from unaudited operational displays/forecasts;
- DWS NGA point/geosite records from actual groundwater time series;
- DWS RQIS/WMS raw surface-water samples from dashboard snapshots;
- SAWS station observations from gridded forcing;
- CHIRPS, ERA5-Land and GPM IMERG as separate product populations;
- Johannesburg Water notice assertions from independently measured service telemetry.

As of this register revision, **no source bytes are acquired and no usage rights are admitted**. SHA-256 fields remain null. Catchment selection is intentionally deferred until station coverage, audited flow, event coverage, forcing quality, anthropogenic boundary records and source terms can be compared. Synthetic or gridded forcing is not local field validation.


## 13. Executable V0 auditor — opened 15 November 2026

The first deterministic evaluator is now implemented in the separate Rust package `tools/hydro-contextual-t0` and frozen corpus:
- `research/hydrology/hydro-contextual-t0-v0.1.json`
- `research/hydrology/hydro-contextual-manifest-v0.1.json`
- `research/hydrology/hydro-contextual-oracle-v0.1.json`
- `research/hydrology/hydro-contextual-t0.schema.json`

The 27-case synthetic suite covers:
- B0 persistence / training-only seasonal-climatology metric recomputation;
- B1 parameter-prior member reporting, including invalid/non-conservative members;
- B2 repeated train-only calibration-reference summary;
- B3 frozen temporal-holdout metric recomputation;
- NSE, KGE, volume bias and MAE with explicit undefined-denominator semantics;
- contextual attainment with raw references, no clipping, and `NOT_COMPARABLE` / `UNDEFINED` states;
- split leakage controls for event, nested catchment, future preprocessing, held-out calibration, missing-as-zero, anthropogenic boundary unknowns, timebase mismatch, physical gates, invalid-member hiding and synthetic-to-field claim promotion;
- boundary attacks for duplicate timestamps and overlapping same-event windows.

The tool reads frozen arrays and run metadata. It does **not** create, calibrate, run, rank or replace a hydrology model. It consumes the existing physical gate as a declared input; #461/#445 and model-specific solver owners remain authoritative.

The separate input/manifest/oracle identity is enforced in Rust. The CLI produces a receipt with SHA-256 digests for inputs, manifest, oracle and Cargo.lock, plus exact implementation revision. The workflow is `.github/workflows/hydro-contextual-t0.yml`.

**Claim ceiling:** synthetic metric recomputation and split-integrity semantics only. No catchment calibration, South African data acquisition, physical hydrology qualification, predictive skill or field claim follows. Hosted formatting/tests/Clippy/exact-head results will be recorded only after the workflow completes for the exact PR head.

The 27-case corpus explicitly includes HCB-NC-06: final test data cannot select the winner of a model tournament and then be reported as an unbiased final test score. It also includes the remaining 11 named controls and two timestamp/event-window boundary attacks.


### Executable process-specific metrics — 15 November 2026

The contextual T0 suite has been extended from 21 to 27 cases to cover the metric gaps identified in the protocol:
- **Event peaks:** peak magnitude error (relative to observed peak), signed and absolute peak timing error, event NSE and event volume bias on a named, frozen event window with a declared fixed step. Earliest maximum wins ties.
- **Low-flow skill:** mean absolute and signed log10-flow error for observations at or below a predeclared threshold. Observed and predicted values must both be strictly positive; zero is not silently replaced with a pseudocount. A zero in the selected low-flow population returns Indeterminate.
- **Probabilistic intervals:** empirical coverage against a declared nominal coverage, coverage error, and mean interval width. Lower>upper is rejected.

The new known-answer cases are 022–026; input, manifest, oracle and schema all declare 27 cases. These are metric arithmetic and data-contract tests only. A small synthetic interval fixture does not prove calibration; report sample count, independent event count, event strata and time-horizon coverage before making probabilistic-skill claims.

The event-peak contract also freezes the tie rule: equal maxima select the earliest time index. A separate case tests this deterministically.


## 14. Event / low-flow / interval metric implementation — 15 November 2026

The executable evaluator adds six synthetic known-answer cases (022–027) to the existing 21-case baseline/split suite. The metrics are deliberately process-specific:
- peak magnitude percentage error and signed/absolute peak timing on named event windows;
- event-window NSE and volume bias, with raw event denominator preserved;
- low-flow log10 error under a frozen observed-flow threshold, with no pseudocount and explicit abstention if any selected observation or prediction is non-positive;
- empirical interval coverage, nominal coverage error and mean interval width; crossed lower/upper bounds reject;
- deterministic earliest-index tie handling for equal event peaks.

Input, manifest, schema and independent oracle carry 27 exact case IDs. Six new cases were independently replayed against the separate oracle at 1e-9 relative tolerance. Rust test/format/Clippy and exact-head workflow status must still be reported separately from this source-level arithmetic replay.

### External benchmark update — lower and upper references are essential

Seibert, Vis & Pool (16 September 2026) computed lower and upper model-performance benchmarks over 13 large-sample hydrology datasets. They show that uncalibrated bucket-model NSE can be 0.8 or higher in some humid and snow-dominated catchments, so a fixed universal NSE threshold is not defensible. For random-parameter lower-reference ensembles, their results recommend at least 1,000 parameter samples to reach a performance plateau with lower uncertainty; transferred calibrated parameter sets can yield slightly higher lower-reference performance. This benchmark should therefore report ensemble-size convergence and both random and transferred-prior populations instead of relying on a tiny, cherry-picked prior set.

- Seibert, J., Vis, M. & Pool, S. (2026), *Setting the bar: benchmarks for model performances in large-sample hydrology*, HESS 30, 5857–5871. https://doi.org/10.5194/hess-30-5857-2026

A complementary 3 July 2026 study benchmarked 47 conceptual rainfall–runoff models across 159 Peninsular Indian watersheds and selected a compact model set by jointly considering performance, model complexity and uncertainty. This supports a three-dimensional comparison: skill, structural/parameter uncertainty and computational/operational complexity, all stratified by hydroclimatic context.

- *Benchmarking and Selecting Optimal Hydrological Models for Large-Sample Applications Considering Complexity and Uncertainty* (2026), Water Resources Research. https://doi.org/10.1029/2026WR044753

### Proposed next benchmark acceptance gates (targets, not achieved results)

1. **B1 lower reference:** 1,000 random parameter samples per model/catchment as the initial reference target, with 100/250/500/1,000/2,000 convergence curves and a predeclared stability criterion; report parameter-range sensitivity and any prior truncation.
2. **Transferred-prior comparator:** add a separately labelled regional/nearby-catchment parameter-prior ensemble. Never merge it with the random-prior baseline or use held-out target catchments to define it.
3. **B2 attainable reference:** repeated train-only calibration from multiple starts, with calibration effort/compute budget fixed and all failed runs visible.
4. **B3 untouched evaluation:** spatial and temporal/event holdouts, with final test used once after model selection; record event and regime denominators.
5. **Catchment-conditioned interpretation:** report model-minus-lower-reference and upper-reference gap per metric/catchment; no universal NSE/KGE pass cutoff.
6. **Operational relevance:** score threshold exceedance/forecast lead time for flood/low-flow decisions separately from continuous-flow metrics. A high NSE cannot compensate for missed critical events.
7. **Complexity/uncertainty:** report parameter and structural spread, runtime/energy, memory, data burden and calibration effort alongside each metric; publish Pareto sets, not one weighted score.

The existing 27-case corpus is only a metric/split software contract. It does not yet execute 1,000 model simulations or establish lower/upper catchment benchmarks. That is the next model-run tournament after source data and the B1 prior definitions are qualified.
