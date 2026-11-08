# Scan-to-BIM Replay Contract v0.1

**Status:** P0.3 schema/receipt contract; still not an external dataset replay. The current manifest schema is v0.2 and records candidate execution identity, per-scene frame/units, tolerance/sampling/metric profiles, candidate outputs and local byte verification.  
**Issue:** #4116 — SCAN-BIM-REPLAY-001.  
**Package:** `tools/scan-bim-replay-contract/`.

## What this tranche does

The Rust CLI validates a replay manifest and verifies its declared local artifacts before any evaluator is run. Schema v0.2 checks:

- dataset use class and explicit prohibition on committing source data or embedding source bytes in CI artifacts;
- separate typed candidate-input and evaluator-artifact namespaces;
- SHA-256-shaped artifact identities, recomputed against local bytes under separate candidate/evaluator roots;
- disjoint development/evaluation group IDs;
- building- and capture-lineage split consistency;
- the ISPRS TUB1/TUB2 same-building and distinct-capture grouping rule;
- explicit surface visibility and observation-status classifications;
- finite positive surface areas and fail-closed unknown JSON fields.

It emits deterministic JSON for the exact input manifest bytes, including a manifest SHA-256, per-scene surface-area accounting and counts of candidate/evaluator files whose bytes were actually verified.

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

The CLI rejects shared artifact IDs and candidate/evaluator content digests. Runtime, input and output artefacts are verified under the candidate root; reference/ground-truth artefacts are verified under the evaluator root. Each file carries a root-relative path; the verifier canonicalizes distinct, non-overlapping roots, rejects absolute paths, dot/parent components and observed symlink components, confirms regular files remain beneath the selected root, and recomputes SHA-256 over actual bytes. The manifest contains relative paths and hashes, not data bytes or absolute local paths. The committed fixtures are synthetic text placeholders only; the IFC-placeholder bytes are not valid IFC models.

This is **local integrity verification**, not process isolation. Candidate and evaluator roots are both readable by this one verifier process. Standard-library path checks are not race-free against a concurrently malicious process; OS-level descriptor-relative no-follow opens/mounts, separate processes/identities and independent custody remain separate gates. Rights flags are still manifest assertions; this CLI does not inspect Git history or arbitrary CI artifact archives.

## Schema v0.2 execution receipt

The v0.2 manifest requires a candidate pipeline/source commit, toolchain and command; runtime artefact IDs for executable/configuration/optional model weights; candidate outputs bound to declared scene IDs (including at least one IFC-role output per scene); coordinate-frame metadata per scene; and SI-metre tolerance/sampling/metric profiles. The official ISPRS wall profile requires an explicit metric-definition authority: `official_results_page` pairs with reference-to-candidate, `final_results_paper_2021` pairs with candidate-to-reference, and `bidirectional_diagnostic` permits both directed distributions without claiming official equivalence. This reflects a real contradiction between the current results page and the 2021 final-results paper; no canonical historical direction is silently assumed. This binds an experiment declaration to bytes, not to a validated physical CRS transform or actual candidate process execution.

See [SCAN_BIM_EXECUTION_RECEIPT_AND_BENCHMARK_TARGETS_V0_1.md](SCAN_BIM_EXECUTION_RECEIPT_AND_BENCHMARK_TARGETS_V0_1.md) for competitive targets and research comparators.

## Deliberate non-goals

- No point-cloud parsing, registration, reconstruction, meshing, IFC authoring or ray-cast visibility derivation.
- No implementation of official ISPRS completeness/correctness/accuracy.
- No official ISPRS geometric scorer yet. The official results page defines accuracy as reference-surface samples to the closest submitted source-model surface; the v0.2 metric profile binds that direction, but source scorer sampling/tessellation/area details and historical reproduction remain P1 work.
- No reproduction of Ersöz–Bosché coverage/distribution/distance indices.
- No statistical generalization claim from the small ISPRS building count.
- No safety, code-compliance, certification or surveyor-approval claim.

## Run

```sh
cargo fmt --manifest-path tools/scan-bim-replay-contract/Cargo.toml -- --check
cargo test --locked --manifest-path tools/scan-bim-replay-contract/Cargo.toml
cargo clippy --locked --manifest-path tools/scan-bim-replay-contract/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/scan-bim-replay-contract/Cargo.toml -- \
  --manifest tools/scan-bim-replay-contract/fixtures/synthetic-observability.json \
  --candidate-root tools/scan-bim-replay-contract/fixtures/data/candidate \
  --evaluator-root tools/scan-bim-replay-contract/fixtures/data/evaluator
```

The fixture is synthetic and contains no ISPRS/BIMNet source data.
