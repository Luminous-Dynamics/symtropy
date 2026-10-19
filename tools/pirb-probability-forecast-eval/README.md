# PIRB probability forecast evaluator

A standalone Rust evaluator for archived **probabilities**. It is intentionally separate from the binary warning/response replay harness: forecast probability, local warning issued, acknowledgement, dispatch, independently verified service, and observed outcome are different facts.

## Run locally

From the repository root:

    cargo test --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml
    cargo run --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-probability-forecast-v0.1.json
    cargo run --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml -- --input docs/fixtures/pirb-probability-forecast-v0.1.json --source-commit "$(git rev-parse HEAD)" --rustc-version "$(rustc --version)" --cargo-version "$(cargo --version)"

A local run without `--source-commit` is marked `UNPINNED_LOCAL_RUN` and is not an exact-head receipt. The checked-in input is synthetic evaluator plumbing, not a real VIEWS or UCDP extract.

## Normalized input contract

The Draft 2020-12 schema is deliberately one outcome definition and one analysis unit per dataset. A normalized adapter for an external provider must state the exact forecast release, model/version, probability variable, target definition, outcome release, source-file digests, attribution, and license reference. Every row carries its forecast release, opaque unit, forecast issue month, last input-data month, target month, lead, probability, outcome label, and label-availability month.

Month fields use a continuous integer index: `month_index = year * 12 + (month - 1)`. It preserves temporal differences across year boundaries (so January and December are adjacent in the right order). The normalized contract checks `lead_months = target_month - input_cutoff_month`, and the target must be strictly after the forecast issue month and input cutoff.

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
- Duplicate release/unit/target/horizon forecast keys are rejected.
- Repeated forecasts for the same unit/target month must agree on the frozen outcome label and its availability month. A changed outcome version is a new dataset/evaluation, not a hidden overwrite.
- Known outcomes must be available after the target month and by the declared evaluation-as-of month. Unknown labels are counted and never mapped to no-event.
- The input schema rejects unrecognized fields, including person-level fields. Data records are aggregate, using opaque unit IDs.

## Privacy, source licensing, and claim ceiling

An `authorized_historical` dataset must declare non-synthetic source and license references, but strings in a JSON document cannot authenticate that authorization. The responsible data controller must check purpose limitation, contractual permission, provenance, retention, disclosure risks, and outcome versioning. A public source may still contain sensitive granularity; public accessibility is not a guarantee that every derived report is safe to publish.

Public CI accepts only the synthetic aggregate fixture. Do not put protected forecasts/outcomes or their raw hashes into public artifacts; a digest of a small or guessable dataset can itself be a confirmation oracle. Small-unit test metrics are suppressed below the dataset's declared threshold, but the threshold must be governed and is not by itself a privacy guarantee.

A passing receipt supports only descriptive scores for the declared forecast release, target definition, unit, outcome vintage and evaluation window. It does not establish data truth, representativeness, consent, authority legitimacy, intervention effectiveness, causal impact, reduced violence, or deployment safety.