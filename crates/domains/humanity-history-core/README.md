# humanity-history-core

Renderer-independent Rust records and structural validation for evidence-aware historical reconstruction.

This is a deliberately small first prototype. It checks whether records are internally reviewable; it does **not** judge whether a historical claim is true, whether a source is credible, or whether a cited passage entails a claim.

## Run

From the Symtropy workspace root:

```sh
# Structural validation
cargo test -p humanity-history-core
cargo run -p humanity-history-core --bin humanity-history-validate -- crates/domains/humanity-history-core/fixtures/synthetic-validation.json

# Calculate the declared point-vs-interval sensitivity probes
cargo run -p humanity-history-core --bin humanity-history-validate -- analyze-sensitivity crates/domains/humanity-history-core/fixtures/antikythera-research-seed.json
```

The CLI prints a JSON report. Exit codes are:
- `0`: structurally valid dataset;
- `1`: dataset parsed but failed validation;
- `2`: input/read/JSON/CLI error.

## Initial invariants

- IDs are non-empty and globally unique across record kinds.
- Attested claims and evidence-based reconstructions need at least one source-linked evidence locator explicitly marked as supporting; a challenge-only citation cannot satisfy the positive-support gate. Challenging evidence is preserved rather than erased.
- Hypotheses and counterfactuals require an explicit interpretation note instead of being silently treated as attested.
- Documented knowledge-graph relations need evidence. Unknown claim/source references are rejected.
- Reversed date ranges are rejected. Certain temporal contradictions are detected only when two intervals use the same named calendar and the intervals decisively conflict. Overlapping/uncertain dates are not silently converted to exact timestamps.
- Rights declarations preserve an exact license identifier or permission basis when required. Public-domain assertions require a recorded determination basis. Distributable assets fail closed for restricted, unknown, or community-protocol rights.
- Asset records carry the SHA-256 digest of the precise asset bytes; this crate validates digest shape, not asset bytes, so the ingestion/build pipeline must calculate and independently verify the digest.
- Validation issues have stable sort order, intended for repeatable local checks.

## Historical research seed

`fixtures/antikythera-research-seed.json` is a metadata-only research seed, not an adjudicated reconstruction. It keeps published 365-hole and 354-hole calendar interpretations as separate claims and links the latter to evidence that challenges the former. It includes bibliographic citations and claim-level locators but no source paper content, imagery, or external model assets.

See [the pilot case-study note](../../../docs/research/case-studies/ANTIKYTHERA_MECHANISM_PILOT.md) for source choices, caveats, expected checks, and adversarial mutation controls.

See [the posterior replication protocol](../../../docs/research/case-studies/ANTIKYTHERA_POSTERIOR_REPLICATION_PROTOCOL.md) for the next-stage plan to retrieve and hash the measurement files, implement the source likelihood in Rust, validate it on synthetic data, and attempt an independently benchmarked posterior reproduction. This European/Mediterranean case is an epistemic stress test, not a globally representative selection of historical significance.

## Quantitative estimates and model alternatives

Datasets may include `estimates`, `reconstruction_models`, and `model_comparisons`.

- Quantitative values and uncertainty endpoints are serialized as decimal strings to avoid rewriting the source's displayed precision. Calculated deltas and distances also use exact base-10 arithmetic and serialize as decimal strings, rather than leaking binary floating-point artifacts. The prototype accepts plain decimal notation, not exponent notation.
- Each estimate declares a quantity, unit, method, assumptions, source evidence, and (when applicable) an interval kind, coverage percentage, and conditioning note.
- Credible and confidence intervals require an explicit coverage percentage. Bounds must be finite and ordered. The validator does not recompute the source analysis.
- Reconstruction records name the claim IDs and estimate IDs they depend on, and declare whether they describe a published interpretation, a published candidate design, or a hypothesis.
- A comparison group must reference at least two distinct models and list explicit evaluation axes and a declared decision rule. Per-model/per-axis assessments distinguish `source_reported`, `independently_reproduced`, `hypothesis`, and `not_evaluated`. Independently reproduced results must include a versioned validation artifact; a paper's report cannot be relabeled as our reproduction. The schema does not infer a single winner or aggregate incomparable evidence into a score.

The Antikythera seed now contains ten quantitative records: the 365-hole and 354-hole reconstruction parameters, two 68% credible estimates from the 2024 analysis under different data treatments, and six source-reported discrete posterior probabilities for N=354/355/356 under full and trimmed data. It also includes three reconstruction-model records, one explicit comparison group, four point-vs-interval contrasts, and three paired full-vs-trimmed sensitivity deltas.

Run against this case study:

```sh
cargo run -p humanity-history-core --bin humanity-history-validate -- crates/domains/humanity-history-core/fixtures/antikythera-research-seed.json
```

The authored unit suite also parses and validates the seed via `include_str!`. This check has **not been executed in the current environment**; do not treat its expected clean result as a passing test until a Rust toolchain runs the suite.

## Auditable analysis-run manifests

The `analysis_runs` field records experiment lifecycle rather than treating a citation as a run. Statuses are:
- `planned`: protocol only; no execution claim.
- `inputs_verified`: input artifacts and a content-addressed input manifest have been recorded.
- `executed`: command, code revision, environment fingerprint, inputs, and output digests are recorded.
- `reproduced`: the executed record also references the source record and target estimates being reproduced.
- `failed`: the command/environment/input context and failure summary remain auditable.

Executed/reproduced runs require SHA-256 strings for the input manifest and each named artifact. This crate validates digest shape but does not itself read the files and calculate the hashes; the ingest/runner must hash actual bytes. A run's limitations are mandatory. A `reproduced` status is still a recorded assertion, not a proof of scientific equivalence; acceptance requires reviewing the output artifacts and the declared tolerance protocol.

The Antikythera seed now contains one `planned` reproduction run with no inputs or outputs. That is intentionally **not** labeled executed: the source measurement files and their exact license are not yet verified.

## Descriptive sensitivity diagnostic

`analyze-sensitivity` evaluates the declared `estimate_contrasts` and `estimate_deltas`. It reports point-vs-interval position/distance and paired baseline-versus-variant numeric deltas. The latter makes the effect of the source's full-versus-trimmed preprocessing explicit.

For the Antikythera research seed, the expected descriptive result is:
- 365 compared with the all-measurement 68% interval [353.88, 356.63]: above the interval by 8.37 holes.
- 365 compared with the fracture-excluded 68% interval [352.67, 355.55]: above by 9.45 holes.
- 354 lies within each of those two reported intervals.

These are arithmetic locations relative to intervals reported by a paper. They are **not** posterior model probabilities, Bayes factors, or a formal adjudication of the calendar-ring interpretation. Woan and Bayley's two intervals are alternative analyses of related measurements, not independent replications. The 354 and 365 values are published reconstruction parameters; comparing one to an interval does not itself prove which historical model is correct.

The same seed records Table 2's source-reported discrete posterior probabilities:
- N=354: 0.1917 (all data) to 0.2714 (trimmed), a change of +0.0797.
- N=355: 0.2823 to 0.2251, a change of -0.0572.
- N=356: 0.2479 to 0.1173, a change of -0.1306.

The deltas are simple differences between table values rounded to four decimals. The trimmed dataset excludes short segments S5/S6 and the first/last holes of S1/S2/S3/S7 (holes 2, 23, 24, 32, 33, 69, 76, and 81). These outputs demonstrate sensitivity to data handling within the same paper; they are not independent observations, new posterior calculations, or proof that one calendar interpretation is true.

## Synthetic fixture

`fixtures/synthetic-validation.json` is test data only. It deliberately contains no real historical assertion or media. Its asset digest is the standard SHA-256 digest for an empty byte sequence, used to demonstrate the required 64-hex-character field.

## Limitations and next steps

This first implementation does not:
- ingest or crawl sources;
- establish source independence or scholarly credibility;
- prove that evidence semantically supports a claim;
- transform dates between calendars;
- detect every graph contradiction or causal error;
- verify external content hashes or package assets;
- claim full CIDOC CRM / PROV-O conformance;
- integrate with Bevy, Sol-Atlas, Symthaea, or Mycelix yet.

Next gates: run tests in a Rust-enabled local environment; add invalid JSON fixtures/negative controls; calculate the content digest from actual input bytes in an ingestion command; add migration tests; then implement a source-reviewed historical case study. Do not use CI queue state as a pass.
