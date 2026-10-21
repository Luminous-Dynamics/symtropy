# Landscape forecast verification benchmark v0.1

**Status:** synthetic executable contract benchmark; not a field validation result.

## Why this is a distinct benchmark

Landscape tools already solve different, valuable problems. The Landscape Architecture Foundation (LAF) publishes more than 100 candidate metrics across 33 benefit categories and a large library of quantified case studies. SITES supplies a sustainability rating and certification workflow. USDA Forest Service i-Tree estimates urban-forest structure and ecosystem-service benefits. GIS/CAD tools such as QGIS, SketchUp and Vectorworks cover spatial analysis, terrain and authoring workflows. These are complementary baselines, not one interchangeable leaderboard.

The missing comparison we target here is narrower: **when Symtropy emits a numerical landscape forecast, how well does that forecast perform against an independently produced reference, and does its interval express uncertainty honestly?** Rendering speed, geometry integrity, model correctness, field effectiveness, and forecast accuracy remain separate qualifications.

Research anchors:

- LAF performance metrics and methods: https://www.landscapeperformance.org/guide-to-evaluate-performance
- SITES certification / rating-system overview: https://support.usgbc.org/hc/en-us/articles/4425464783891-SITES-certification-and-sustainability-for-landscapes
- USDA Forest Service i-Tree methods and limitations: https://research.fs.usda.gov/treesearch/68438
- EPA stormwater performance-monitoring guidance: https://nepis.epa.gov/Exe/ZyPURL.cgi?Dockey=P100170R.TXT
- Johannesburg remote-sensing study (2025): https://link.springer.com/article/10.1007/s44274-025-00264-0

The 2025 Johannesburg study reports a stronger association between its moisture index (NDMI) and its urban-heat-island measure than between its vegetation index (MSAVI2) and that heat measure. These are remote-sensing associations at the study's spatial/temporal support, not proof of causal cooling, pedestrian-level air-temperature reduction, or parcel-scale prediction accuracy. The benchmark must preserve that distinction.

## v0.1 scope

This standalone, Rust-first tool accepts only a frozen `synthetic_fixture`. It exists to validate metric arithmetic, grouping, exclusion rules and report shape while the project's real source, observation, evaluation-plan and scenario contracts remain separate workstreams:

- Sol-Atlas source and scale-suitability gate: https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/206
- Sol-Atlas provenance-bound observations: https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/207
- Sol-Atlas preregistered Johannesburg/Highveld field pilot: https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/264
- Symtropy pinned landscape scenario admission: https://github.com/Luminous-Dynamics/symtropy/issues/2298
- Symthaea read-only evidence-bound alternatives review: https://github.com/Luminous-Dynamics/symthaea/issues/8196
- Cross-repository artifact-chain conformance: https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/244

No live source is downloaded. No field site is named. The fixture's numbers and source digests are synthetic test data, not local measurements, model outputs, or evidence that a design works. The source digest strings in the fixture are identifiers for synthetic reference payload families; the exact whole-fixture bytes are also hashed by the verifier.

## Contract and calculations

Each case binds a forecast and an independent reference to an explicit metric ID, unit, spatial scope, temporal stratum, independent evaluation unit, and split (`training`, `calibration`, or `held_out`). Only held-out cases contribute to scoring; other partitions receive `not_held_out` exclusions. The forecast includes a point estimate and a declared prediction interval. A baseline forecast is required so error skill is not reported in isolation. Summaries report both row count and distinct independent-unit count; repeated records from one property must not be represented as independent properties.

For each metric/scope/stratum/model group, the report computes:

- Signed bias = forecast minus reference.
- Mean absolute error (MAE) and root mean squared error (RMSE).
- Baseline MAE and MAE skill score: `1 - candidate_MAE / baseline_MAE`. A zero-error baseline makes the score not computable rather than infinite.
- Point-reference coverage inside the declared prediction interval, mean interval width, and the central prediction-interval (Winkler) score. The interval score penalizes both width and misses; coverage alone can be made arbitrarily high with unhelpfully broad intervals. Lower score is better and remains stratified by metric, unit, scope, time stratum, model, and nominal interval level.

The report also emits both case-weighted metrics and **independent-unit macro metrics**. Case-weighted MAE/RMSE answer “how large is the error across all rows?” and can over-weight an often-sampled site. The `unit_macro_*` metrics first summarize rows within each declared independent unit, then give each unit equal weight; for example, unit-macro RMSE is the mean of per-unit RMSEs, not the pooled square-root of all squared errors. The frozen fixture intentionally has one row per unit, so both MAEs coincide. An adversarial test adds an extreme repeat from one site and checks that it raises case-weighted MAE to 3.025 while equal-unit macro MAE is 2.333333333. For field evaluation, use a truly independent unit appropriate to the metric (often a site, event, or cohort) and hold out entire sites/seasons; macro metrics do not cure a wrongly chosen unit or correlated leakage.

Metrics are **never pooled across units, spatial scopes or time strata**. There is no universal sustainability score, ranking across unrelated outcomes, or causal inference. Reference uncertainty is surfaced per case; v0.1 does not silently treat it as zero or use it to widen the model's interval. Three synthetic held-out rows from three independent synthetic units per group are a known-answer software test, not enough samples to establish calibration. The extra calibration-partition case verifies that fitted data do not leak into the reported test metrics. The frozen interval-score answers are 3.366666667 for temperature change, 0.766666667 for runoff-volume change, and 22.666666667 for plant-survival change.

The verifier fails closed on unknown fields, duplicate case IDs, missing independent-unit IDs, a forecast scope key inconsistent with its declared scope/period, non-finite/out-of-bound values, malformed or missing source digests for valid references, invalid intervals, invalid reference/status combinations, and any attempt to label the fixture as real field data. Calibration/training cases are explicitly excluded so evaluation cannot accidentally score fitted observations. Missing, stale and contradictory references are excluded with distinct reasons. A valid measurement with an incompatible support key is excluded as `scope_mismatch`, not scored.

## Run

```sh
cargo fmt --manifest-path tools/landscape-forecast-verifier/Cargo.toml -- --check
cargo test --locked --manifest-path tools/landscape-forecast-verifier/Cargo.toml
cargo run --locked --manifest-path tools/landscape-forecast-verifier/Cargo.toml -- \
  docs/research/fixtures/LANDSCAPE_FORECAST_VERIFICATION_V0_1.json
```

The report includes the exact SHA-256 of the input bytes. Reordering semantically identical cases preserves sorted result groups but changes the input digest, making the byte-level identity visible.

## Benchmark ladder toward real-world qualification

1. **Structural/analytic conformance:** frozen synthetic cases, known answers, invalid interval/status controls, digest linkage and deterministic output.
2. **Independent model references:** for each supported solver, add hand-calculable or separately implemented physical oracles. Keep runoff, canopy/shade, thermal comfort and habitat-connectivity models separate.
3. **Historical holdout:** only after source identity, license/access, CRS/datum, spatial resolution, temporal coverage, quality masks and independent reference checks have passed. Split by site and season—not random pixels from the same scene. Establish baselines before inspecting holdout results.
4. **Prospective field study:** use the preregistered BACI or matched-control design where feasible; freeze project/scenario digests, primary metric, smallest decision-relevant effect, sampling plan, exclusions, methods and analysis version before collection/unblinding.
5. **External replication:** independent sites, operators and seasons. Report adverse and null results, missingness, subgroup failures and uncertainty.

Potential metrics differ by family: paired shade/temperature observations; event rainfall/runoff/infiltration; irrigation demand; plant establishment/survival at declared follow-up windows; access/maintenance and lifecycle-cost observations. Satellite land-surface temperature is not interchangeable with air temperature at pedestrian height. Regional rainfall products are not parcel gauges; biodiversity-priority maps are not field species inventories.

## Targets to pre-register before evaluating real data

- **Contract correctness:** 100% of known-answer cases match; zero invalid/missing/stale/contradictory or cross-scope cases are scored.
- **Forecast skill:** candidate MAE beats a declared seasonal/persistence or domain-appropriate baseline on held-out site-season groups, with a confidence interval on the skill difference whose lower bound is above zero before calling it an improvement. Set the smallest practically important effect and sample size using measurement precision and a power analysis, not after seeing results.
- **Interval honesty:** report empirical coverage for each nominal interval level (e.g. 80% and 95%) by metric family, season and spatial scope. Use site/season-blocked uncertainty analysis; do not treat nearby pixels, repeated measurements, or multiple sensors on one site as independent projects. Predeclare acceptable calibration error.
- **Generalization:** hold out entire sites or time periods. A calibration set cannot also be the confirmatory test set.
- **Decision quality:** compare alternatives with separate objective ledgers and Pareto trade-offs; test for constraint violations, rank instability and worst-case regret without collapsing ecology, comfort, water, cost and equity into a hidden weighted score.
- **Reproducibility:** pin exact input bytes, source/model versions, calibration protocol, code/toolchain and report digest. A queued CI workflow is not a pass, and successful synthetic tests do not qualify physical accuracy.

## Industry comparison and honest success criteria

Use specialized incumbents for the capability they actually provide: LAF metric/method protocols and built-project cases for landscape outcomes; SITES for its rating/certification framework; i-Tree for supported urban-tree ecosystem-service estimates; QGIS for geospatial analysis/interoperability; and SketchUp/Vectorworks for design-authoring workflows. Compare like-for-like tasks and matching data/version/scale. Do not claim this benchmark replaces their validated functionality.

The product-level differentiator to try to prove is a traceable path from a versioned design → admitted scenario → forecast → independent observation → calibrated review, with visible uncertainty and principled abstention. The present benchmark establishes arithmetic and fail-closed software behavior only. A claim of state-of-the-art physical performance remains open until held-out and independently replicated evidence exists.
