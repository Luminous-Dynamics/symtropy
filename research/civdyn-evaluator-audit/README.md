# CivDyn Evaluator Audit v0.1

A deterministic Rust reference for preserving plural adjudication and measuring the calibration of model-based evaluators against an independently recorded panel.

## What it does not assume

- No single rater, majority label or LLM judge is presumed to be ground truth.
- Missing ratings are not imputed as dissent or agreement.
- Self-reported confidence is not treated as a probability of correctness.
- Agreement does not prove that a rubric is normatively legitimate or culturally true.
- A panel's empirical distribution is an observed adjudication distribution, not an oracle for moral truth.

## Inputs

- Stable rubric ID/version and category labels.
- Opaque case ID, context ID and explicitly declared cohort/slice.
- Two or more independent panel ratings per scored case; different cases may have different panel sizes.
- Optional panel self-confidence.
- One or more judge predictions with a full category probability distribution, selected category and optional self-reported confidence.

A judge ID may not appear as an independent panel-rater ID anywhere in the dataset. Duplicate rater/case and judge/case records fail closed. Category order is stable and rubric-versioned.

## Metrics

**Panel agreement**
- observed pairwise agreement and support counts;
- nominal Krippendorff alpha, which supports variable panel sizes and missing ratings;
- per-case disagreement entropy and plurality support;
- rater count range and unanimous-case count.

**Judge evaluation**
- multiclass Brier score against each case's soft panel distribution;
- clipped cross-entropy/log loss against that distribution;
- plurality match rate, reported separately from soft-distribution fit;
- support of the panel for the judge's selected category;
- reported confidence versus panel support gap and five-bin expected calibration error.

**Strata**
- all panel and judge metrics are reported overall and separately by declared context/cohort combinations.

A high alpha means raters applied the declared rubric consistently in this sample. It does not show that the rubric is unbiased, fair, complete or culturally legitimate. High majority-match accuracy can coexist with poor probability calibration and erased disagreement.

## Synthetic control

The executable uses 12 synthetic cases, three panel ratings per case and four response categories. Four cases are unanimous; eight have 2:1 splits. It compares:
- a distribution-aware judge that reproduces the soft panel distribution;
- an overconfident majority judge that selects the panel plurality but reports 0.99 confidence on every case.

This fixture demonstrates why the selected label, the panel distribution and the judge's confidence must be scored separately. It is not a human-subject study or a reproduction of LiveCultureBench or the EMNLP long-horizon activity benchmark.

## Run

```sh
cargo fmt --manifest-path research/civdyn-evaluator-audit/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-evaluator-audit/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-evaluator-audit/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-evaluator-audit/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-evaluator-audit/Cargo.toml --bin evaluator-audit-sweep
```

## Claim ceiling and data stewardship

The fixture uses no real participants, cultural communities, video, audio, demographic inference or sensitive labels. Real-data evaluation requires source rights, privacy and ethics review, consent where applicable, community governance, stable rubric provenance, independent/qualified raters, appeal routes, and explicit support counts. Raters' disagreement must be preserved instead of erased by a judge or majority.
