# Scan-to-BIM Replay Contract v0.1

**Status:** executable P0 metadata and visibility-accounting prototype; not an external dataset replay. The focused workflow is defined in the base branch for this stacked pull request.  
**Issue:** #4116 — SCAN-BIM-REPLAY-001.  
**Package:** `tools/scan-bim-replay-contract/`.

## What this tranche does

The Rust CLI validates a replay manifest before any evaluator is run. It checks:

- dataset use class and explicit prohibition on committing source data or embedding source bytes in CI artifacts;
- separate typed candidate-input and evaluator-artifact namespaces;
- SHA-256-shaped artifact identities (format validation only; this CLI does not open or hash dataset files);
- disjoint development/evaluation group IDs;
- building- and capture-lineage split consistency;
- the ISPRS TUB1/TUB2 same-building and distinct-capture grouping rule;
- explicit surface visibility and observation-status classifications;
- finite positive surface areas and fail-closed unknown JSON fields.

It emits deterministic JSON for the exact input manifest bytes, including a manifest SHA-256 and per-scene surface-area accounting.

## Observability contract

Visibility and observation are separate axes.

| Visibility class | Meaning | Eligible for theoretically-visible denominator? |
|---|---|---:|
| `theoretically_visible` | The target surface should be visible to the declared capture configuration | Yes |
| `self_occluded` | Hidden by another surface of the target element or asset | No |
| `externally_occluded` | Hidden by an external object such as furniture, blinds or temporary equipment | No |
| `outside_sensor_coverage` | Outside the declared field of view or capture envelope | No |
| `unknown` | Visibility cannot be established from the available evidence | No; retained as unknown |

Observation status is independently recorded as `captured`, `not_captured`, `unknown` or `contradictory`.

The audit reports:
- theoretically visible area;
- visible area marked captured;
- visible area not captured;
- unresolved visible area;
- self-occluded, externally occluded, outside-coverage and unknown areas;
- contradictory area;
- captured fraction of theoretically-visible area, only when its denominator is non-zero.

**This fraction is an acquisition/observability accounting measure, not an ISPRS completeness score, geometric matching confidence score, or proof that a BIM surface is correct.** Surface strata must be prepared by an upstream visibility evaluator; the present prototype does not ray-cast or infer visibility.

## Candidate/evaluator separation

Candidate inputs have a closed role vocabulary: point-cloud observation, sensor metadata, prior asset model, registration metadata, and calibration artifact.

Evaluator artifacts have a separate closed vocabulary: reference geometry, ground-truth labels, holdout assignment, official score table and independent checkpoints.

The CLI rejects shared artifact IDs. It does not provide filesystem sandboxing, process isolation, access-control enforcement, organizational independence or cryptographic authentication of the artifact producer. Those are separate gates. The manifest contains identifiers and hashes, not data bytes or local paths.

## Deliberate non-goals

- No point-cloud loading, registration, reconstruction, meshing or IFC authoring.
- No implementation of official ISPRS completeness/correctness/accuracy.
- No choice of the disputed canonical ISPRS accuracy direction. The two directed distance distributions should remain diagnostics until the canonical evaluator implementation is resolved.
- No reproduction of Ersöz–Bosché coverage/distribution/distance indices.
- No statistical generalization claim from the small ISPRS building count.
- No safety, code-compliance, certification or surveyor-approval claim.

## Run

```sh
cargo fmt --manifest-path tools/scan-bim-replay-contract/Cargo.toml -- --check
cargo test --locked --manifest-path tools/scan-bim-replay-contract/Cargo.toml
cargo clippy --locked --manifest-path tools/scan-bim-replay-contract/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/scan-bim-replay-contract/Cargo.toml -- \
  --manifest tools/scan-bim-replay-contract/fixtures/synthetic-observability.json
```

The fixture is synthetic and contains no ISPRS/BIMNet source data.
