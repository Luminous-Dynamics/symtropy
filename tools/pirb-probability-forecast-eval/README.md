# PIRB probability forecast evaluator

A standalone Rust evaluator for archived **probabilities**. It is intentionally separate from the binary warning/response replay harness: forecast probability, local warning issued, acknowledgement, dispatch, independently verified service, and observed outcome are different facts.

## Run locally

From the repository root:

    cargo test --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml
    cargo run --bin pirb-probability-forecast-eval --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-probability-forecast-v0.1.json
    cargo run --bin pirb-outcome-vintage-sensitivity --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-probability-forecast-v0.1.json --input docs/fixtures/pirb-probability-forecast-vintage-alternate-v0.1.json
    cargo run --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-probability-forecast-v0.1.json --source-commit "$(git rev-parse HEAD)" --rustc-version "$(rustc --version)" --cargo-version "$(cargo --version)"
    cargo run --bin pirb-cross-provider-benchmark-validate --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-cross-provider-benchmark-incompatible-v0.1.json --source-commit "$(git rev-parse HEAD)"
    cargo run --bin pirb-hierarchy-coherence-audit --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-hierarchy-forecast-synthetic-v0.1.json --source-commit "$(git rev-parse HEAD)"
    cargo run --bin pirb-joint-sample-coherence-audit --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-joint-sample-forecast-synthetic-v0.1.json --source-commit "$(git rev-parse HEAD)"
    cargo run --bin pirb-policy-impact-admissibility --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-policy-impact-before-after-not-identified-synthetic-v0.1.json --source-commit "$(git rev-parse HEAD)"
    cargo run --bin pirb-score-admissibility --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-score-admissibility-log-score-reconciliation-synthetic-v0.1.json --source-commit "$(git rev-parse HEAD)"
    cargo run --bin pirb-joint-score-eval --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-joint-score-four-draw-synthetic-v0.1.json --source-commit "$(git rev-parse HEAD)"

A local run without `--source-commit` is marked `UNPINNED_LOCAL_RUN` and is not an exact-head receipt. The checked-in input is synthetic evaluator plumbing, not a real VIEWS or UCDP extract.

## Cross-provider benchmark registration

The `pirb-cross-provider-benchmark-validate` binary validates `docs/schemas/pirb-cross-provider-benchmark-v0.1.schema.json` plus cross-field invariants that JSON Schema cannot establish alone: unique and resolvable provider/comparison IDs, exact target/unit/time/horizon/output compatibility for direct comparisons, pinned and independently reviewed mappings for mapped comparisons, output-type-specific metric families, and governance gates for authorized historical data. It emits a deterministic registration-validation receipt with schema/input digests and the explicitly declared comparison states. It does **not** load forecast rows, score providers, establish predictive superiority, select actions, or estimate causal impact.

The checked-in VIEWS/ACLED CAST example deliberately exits successfully as a valid **not-comparable** registration while keeping both scoring lanes disabled. That is a successful contract test, not a successful model comparison. For an evaluation registration, source releases, data rights, outcome maturity, shared-row rules and mapping evidence must be pinned before held-out evaluation.

## Hierarchical forecast coherence

The `pirb-hierarchy-coherence-audit` binary measures parent/child consistency for count forecasts before and after reconciliation. It reports raw and reconciled residuals, adjustment magnitudes, completeness coverage, and explicit `not_assessable` constraints when children are missing or coverage is not verified. Every constraint pins expected child keys; a complete constraint rejects missing child rows and existing child rows silently omitted from the declared set, and the aggregation graph must be acyclic. This is not a forecast-accuracy score and the audit does not implement a reconciliation method such as MinTraceSparse; it checks the declared hierarchy constraints and records their coherence diagnostics.

The checked-in hierarchy fixture has invented aggregate counts. Its all-zero/all-one artifact digests are placeholders and do not identify captured provider data. The fixture includes two complete exact-sum constraints and one incomplete spatial constraint whose missing child must not be imputed as zero. See `docs/tech/PIRB_HIERARCHICAL_FORECAST_COHERENCE_V0_1.md` for the contract and claim limits.

## Joint predictive sample coherence

The `pirb-joint-sample-coherence-audit` binary checks whether each registered joint predictive draw satisfies the declared spatial and event-type aggregation constraints. It preserves the paired base and reconciled values, measures coherence by constraint and by complete draw, reports raw/reconciled residuals and adjustment magnitudes, and keeps incomplete constraints explicitly `not_assessable`. Every declared forecast node must have a row for every registered draw; missing values are rejected, never imputed.

The synthetic fixture has four invented joint draws: the raw samples have 6/8 coherent complete constraint-draw pairs, while the submitted reconciled samples have 8/8; only 2/4 raw draws satisfy all complete constraints simultaneously, versus 4/4 reconciled draws. One incomplete spatial constraint is not included in the coherence denominator. These are test-oracle values, not empirical results from VIEWS, ACLED CAST, or any production forecaster.

This command audits submitted samples—it does not implement MinTraceSparse or another reconciliation algorithm, and it does not calculate sample calibration, interval coverage, energy score, variogram score, count skill, decision utility or causal effect. Proper scoring rules have different validity conditions when comparing coherent and incoherent distributions. See `docs/tech/PIRB_JOINT_PREDICTIVE_SAMPLE_COHERENCE_V0_1.md`, [ACLED CAST methodology](https://acleddata.com/methodology/cast-methodology), and [Panagiotelis et al. on probabilistic reconciliation](https://doi.org/10.1016/j.ejor.2022.07.040).

## Forecast-to-action policy impact admissibility

The `pirb-policy-impact-admissibility` binary checks whether a policy-evaluation registration keeps forecast verification, action delivery, observed outcomes and causal evaluation as separate lanes. It requires a no-action baseline, fixed resource capacity, action offer/delivery/adherence records, a measurement and missingness policy, reporting-shift assessment, counterfactual/estimand and design references, and governance declarations. Historical registrations claiming independent-review eligibility must also satisfy declared design-review, outcome-maturity, spillover, source-authority and disclosure gates. The tool accepts no causal effect estimate and never sets impact established.

The checked-in fixture is a synthetic before/after negative control. It passes as `not_identified`, with no effect estimate, because a before/after association alone is not an identified causal effect. Even an eligible receipt means “eligible for independent review”, not that identification assumptions are true or an effect has been estimated. See `docs/tech/PIRB_POLICY_IMPACT_ADMISSIBILITY_V0_1.md` and [issue #2248](https://github.com/Luminous-Dynamics/symtropy/issues/2248).

## Scoring-rule admissibility

The `pirb-score-admissibility` binary validates whether a score family is admissible for the registered prediction representation, outcome type and comparison design; it does not calculate that score. Its closed v0.1 map supports Brier/log score for binary probabilities, pinball/interval score for quantiles, and energy/variogram score for multivariate joint samples. It rejects incompatible pairings, score-orientation mismatches, and explicitly rejects log score for reconciled-versus-unreconciled distribution comparisons according to the comparison-specific warning in Panagiotelis et al. (2023).

The synthetic fixture is intentionally not admissible: it registers log score for reconciled-vs-unreconciled joint samples. The receipt must return `not_admissible`, a reason code, `score_calculated=false`, and `ranking_permitted=false`. A compatible score registration may be admissible for future score computation, but this command never computes a score or establishes superiority. See `docs/tech/PIRB_SCORE_ADMISSIBILITY_V0_1.md`, [the scoring-theory reference](https://doi.org/10.1016/j.ejor.2022.07.040), and [issue #2254](https://github.com/Luminous-Dynamics/symtropy/issues/2254).

## Joint score formula oracles

The `pirb-joint-score-eval` binary computes empirical energy and variogram scores separately for the base/unreconciled and reconciled joint samples. The contract pins the Euclidean norm, finite-ensemble energy-score V-statistic, variogram exponent, unit weights, observed-vector order, draw IDs, fit cutoff, and numeric tolerance. It also reports hierarchy coherence/residuals and adjustment magnitude separately from score values.

The scored vector may not contain an aggregate parent together with its children, because that double-counts overlapping outcomes. Every declared node must be present for every declared draw; missing or duplicate draw-node pairs fail closed. The four-draw and three-draw fixtures have independent hand-calculated golden values. See `docs/tech/PIRB_JOINT_FORECAST_SCORE_ORACLES_V0_1.md` for formula conventions and limitations.

The receipt is a formula-audit artifact, not a superiority ranking. Coherent reconciled draws can score worse than base draws against an observation, and sample IDs alone do not authenticate real dependence. Public fixtures remain synthetic-only.

## Outcome-vintage sensitivity

The separate `pirb-outcome-vintage-sensitivity` binary compares 2–20 frozen datasets whose forecast archives and every issued prediction remain identical but whose outcome release IDs/digests differ. It rejects changed probabilities, prediction cutoffs/horizons, unit mappings, outcome source/definition, split, privacy, or resampling policy. It reports how many held-out labels or their publication-availability months changed against the designated first/reference release, each vintage's source/licensing/attribution references, and the min/max/spread of descriptive score values across releases. It does **not** decide which vintage is correct, average incompatible labels, or turn the observed score range into a confidence interval. For reproducibility, input order defines the reference outcome release.

Run it with repeated `--input PATH` arguments. Do not place protected data or raw release files in public CI; a sensitivity receipt includes input and outcome digests and should follow the data controller's publication/access policy.


## Frozen VIEWS archive import (offline only)

The \`pirb-views-offline-adapter\` binary maps locally saved response pages into the normalized dataset contract. It does not contact VIEWS or download data. The manifest pins each response to one model/versioned run, query URI, analysis unit, target month, partition, matching codebook, column mapping, outcome release and governed opaque spatial-block map.

Public CI contains a synthetic VIEWS-shaped response envelope and synthetic \`main_dich\` codebook. They are invented fixtures, not real VIEWS data. v0.1 accepts only the state-based country-month probability-of-at-least-25-BRDs target, exactly \`sc_cm_sb_dich_main\` with codebook key \`main_dich\`. PRIO-GRID, other violence types and fatality-count variables are rejected until their target-specific mappings are separately reviewed; bounded numeric counts are never reinterpreted as probabilities. The codebook's unit/range, threshold language, geography, response model metadata and target-month query filter must agree with the manifest.

Example from the repository root:

    cargo run --bin pirb-views-offline-adapter --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --manifest docs/fixtures/views-import-manifest-synthetic-v0.1.json --outcomes docs/fixtures/views-outcomes-synthetic-v0.1.json --block-map docs/fixtures/views-spatial-block-map-synthetic-v0.1.json --output pirb-views-normalized.json --receipt pirb-views-import-receipt.json --source-commit "$(git rev-parse HEAD)" --rustc-version "$(rustc --version)" --cargo-version "$(cargo --version)"

The adapter rejects unsafe relative source paths, missing or duplicate outcome keys, missing block mappings, incomplete response pagination, source/model mismatches, wrong horizons/query months, duplicate forecast observations, non-probability codebooks, and extra source files not declared by the manifest. The query URI must match the documented route shape `/{run}/{loa}/{violence_type}/{model}` (for example, `/fatalities003_2026_08_t01/cm/sb/sc_cm_sb_dich_main?month=...`). The upstream implementation does not use `eval`/`predict`/`calib` as a route segment. The receipt marks the model route as checked but keeps partition verification false, because the ordinary response envelope does not authenticate its analysis split. Real-data import requires trusted capture provenance and explicit split mapping. Receipts bind the manifest, outcome snapshot, block-map, codebook, response pages, forecast-only archive commitment, normalized dataset and dependency lock. The forecast archive digest commits to provider/archive ID, model/version, forecast variable, target/unit/violence type/threshold/lead, query URIs and response/codebook digests; it deliberately excludes outcome-vintage fields. A regression test proves an outcome-only label/availability revision leaves the forecast archive digest unchanged while changing the outcome and normalized-dataset digests.

A passing import receipt establishes mapping/conformance only. Source references and license strings are declarations to review, not cryptographic proof of permission. For authorized historical use, keep raw files and receipts in the controlled environment and check data terms, attribution, confidentiality, and disclosure risk before publishing any derived metrics.

## Normalized input contract

The Draft 2020-12 schema is deliberately one outcome definition, one analysis unit, and one forecast horizon per dataset. Score each horizon separately. A normalized adapter for an external provider must state the exact forecast release, model/version, probability variable, target definition, outcome release, source-file digests, attribution, and license reference. Every row carries its forecast release, opaque unit, forecast issue month, last input-data month, target month, lead, probability, outcome label, and label-availability month.

Month fields use a continuous integer index: `month_index = year * 12 + (month - 1)`. It preserves temporal differences across year boundaries (so January and December are adjacent in the right order). The normalized contract checks `lead_months = target_month - input_cutoff_month`; the target must be after the input cutoff and on or after the forecast-origin/issuance month. This accommodates a one-month forecast issued in January from input data through December.

For VIEWS, the FAQ currently distinguishes the probability of crossing the target threshold from predicted fatality counts: country-month probabilities concern at least 25 battle-related deaths, and PRIO-GRID-month probabilities concern at least one. They are different targets and cannot be mixed. Do not map a fatality-count variable into the probability field. Use a specific frozen release and its machine-readable codebook; never score a current forward-looking release as if its outcomes had already matured. References: [VIEWS data access](https://viewsforecasting.org/data/), [forecast targets and partitions](https://viewsforecasting.org/faq/), and [VIEWS API codebook documentation](https://github.com/prio-data/views_api/wiki/Codebook).

This evaluator expects an already-normalized JSON contract. It does not currently download VIEWS/UCDP data, infer mapping choices, or authorize data use.

## Scores and their interpretation

- **Brier score:** mean squared error of the probability against the binary outcome; lower is better.
- **Log loss:** logarithmic proper score using the declared epsilon only for numerical clipping at 0 and 1; lower is better.
- **ROC-AUC:** rank discrimination; null if the scored split contains only one outcome class.
- **Average precision:** rare-event precision/recall summary with equal-probability ties processed as a group; null when both classes are not present.
- **Reliability bins and ECE:** equal-width probability bins report count, mean probability, and observed event rate. ECE is a descriptive weighted absolute gap and depends on binning, so it is not a standalone guarantee of calibration.
- **Brier skill score:** compares the model with a naive climatology baseline using the *same records* where baseline probability is available. Skill is \(1 - \text{model Brier}/\text{baseline Brier}\); positive means better than that baseline, zero ties it, and negative is worse. It is null if the baseline has no error to improve on.
- **Past-only climatology:** estimates the event rate from unique earlier unit-month outcome labels that had actually become available by that forecast's input cutoff. It never includes a label released later than that cutoff. An explicit minimum history is required; insufficient histories are counted, not filled with a guessed baseline.

Training, calibration, and test partitions use forecast-issue months. Every known training label must have an availability month strictly before the calibration boundary, and every known calibration label strictly before the test boundary; labels timestamped in the boundary month are rejected as ambiguous. This prevents a long-lead calibration target whose outcome arrives years later from silently influencing a test-time tuning choice. The tool does not itself tune or calibrate model probabilities; `probability_calibration_fitted` is explicitly false.

## Validation and data vintage

- Forecast probabilities must be finite and within [0, 1].
- The target and forecast lead must agree with the declared input cutoff.
- The declared lead horizon must match every row; do not pool different horizons in one score.
- Duplicate unit/target/horizon forecasts are rejected even if their release IDs differ.
- Repeated forecasts for the same unit/target month must agree on the frozen outcome label and its availability month. A changed outcome version is a new dataset/evaluation, not a hidden overwrite.
- Known outcomes must be available after the target month and by the declared evaluation-as-of month. Unknown labels are counted and never mapped to no-event.
- The input schema rejects unrecognized fields, including person-level fields. Data records are aggregate, using opaque unit IDs.

## Privacy, source licensing, and claim ceiling

An `authorized_historical` dataset must declare non-synthetic source and license references, but strings in a JSON document cannot authenticate that authorization. The responsible data controller must check purpose limitation, contractual permission, provenance, retention, disclosure risks, and outcome versioning. A public source may still contain sensitive granularity; public accessibility is not a guarantee that every derived report is safe to publish.

Public CI accepts only the synthetic aggregate fixture. Do not put protected forecasts/outcomes or their raw hashes into public artifacts; a digest of a small or guessable dataset can itself be a confirmation oracle. Small-unit test metrics are suppressed below the dataset's declared threshold, but the threshold must be governed and is not by itself a privacy guarantee.

A passing receipt supports only descriptive scores for the declared forecast release, target definition, unit, outcome vintage and evaluation window. It does not establish data truth, representativeness, consent, authority legitimacy, intervention effectiveness, causal impact, reduced violence, or deployment safety.