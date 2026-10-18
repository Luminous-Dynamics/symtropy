# humanity-history-core

Renderer-independent Rust records and structural validation for evidence-aware historical reconstruction.

This is a deliberately small first prototype. It checks whether records are internally reviewable; it does **not** judge whether a historical claim is true, whether a source is credible, or whether a cited passage entails a claim.

## Run

From the Symtropy workspace root:

```sh
cargo test -p humanity-history-core
cargo run -p humanity-history-core --bin humanity-history-validate -- crates/domains/humanity-history-core/fixtures/synthetic-validation.json
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

See [the pilot case-study note](../../../docs/research/case-studies/ANTIKYTHERA_MECHANISM_PILOT.md) for source choices, caveats, expected checks, and adversarial mutation controls. This European/Mediterranean case is an epistemic stress test, not a globally representative selection of historical significance.

## Quantitative estimates and model alternatives

Datasets may include `estimates`, `reconstruction_models`, and `model_comparisons`.

- Quantitative values and uncertainty endpoints are serialized as decimal strings to avoid rewriting the source's displayed precision.
- Each estimate declares a quantity, unit, method, assumptions, source evidence, and (when applicable) an interval kind, coverage percentage, and conditioning note.
- Credible and confidence intervals require an explicit coverage percentage. Bounds must be finite and ordered. The validator does not recompute the source analysis.
- Reconstruction records name the claim IDs and estimate IDs they depend on, and declare whether they describe a published interpretation, a published candidate design, or a hypothesis.
- A comparison group must reference at least two distinct models and list explicit evaluation axes and a decision rule. The schema does not infer a single winner or aggregate incomparable evidence into a score.

The Antikythera seed now contains four estimate records: the 365-hole parameter from a 2008 reconstruction, the 354-hole parameter from a later interpretation, and two 68% credible estimates from a 2024 analysis under different handling of fracture-adjacent measurements. The two 2024 estimates share a measurement basis and are sensitivity variants, not independent replications. It also includes three reconstruction-model records and one explicit comparison group.

Run against this case study:

```sh
cargo run -p humanity-history-core --bin humanity-history-validate -- crates/domains/humanity-history-core/fixtures/antikythera-research-seed.json
```

The authored unit suite also parses and validates the seed via `include_str!`. This check has **not been executed in the current environment**; do not treat its expected clean result as a passing test until a Rust toolchain runs the suite.

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
