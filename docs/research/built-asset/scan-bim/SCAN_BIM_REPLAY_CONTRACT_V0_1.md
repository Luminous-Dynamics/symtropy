# Scan-to-BIM Replay Contract v0.2

**Status:** executable P0.3 replay-provenance, evaluation-profile, local artifact-integrity and visibility-accounting prototype; not an external dataset replay or geometric scorer.  
**Issue:** #4116 — SCAN-BIM-REPLAY-001.  
**Package:** `tools/scan-bim-replay-contract/`.

## What this tranche does

The Rust CLI validates a replay manifest before any evaluator is run. It checks:

- dataset use class and explicit prohibition on committing source data or embedding source bytes in CI artifacts;
- separate typed candidate-input, candidate-pipeline, candidate-output and evaluator-artifact namespaces;
- candidate run identity: source commit, toolchain, exact argv and content digests for build, model and configuration artifacts;
- per-scene IFC model and element-graph output artifacts, with actual local SHA-256 verification;
- explicit coordinate frame, CRS, axis order, linear unit and metres-per-unit conversion;
- versioned tolerance, sampling and metric profiles, including declared cutoff/buffer curves and reference-to-candidate accuracy direction;
- SHA-256-shaped artifact identities, recomputed against local bytes under separate candidate/evaluator roots;
- disjoint development/evaluation group IDs;
- building- and capture-lineage split consistency;
- the ISPRS TUB1/TUB2 same-building and distinct-capture grouping rule;
- explicit surface visibility and observation-status classifications;
- finite positive surface areas and fail-closed unknown JSON fields.

It emits deterministic JSON for the exact input manifest bytes, including a manifest SHA-256, candidate run and evaluation-profile identity, per-scene surface-area accounting, and typed counts of candidate input, pipeline, output and evaluator files whose bytes were actually verified. The command-argv digest is computed from the canonical JSON serialization of the stored argv array.

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

Candidate inputs have a closed role vocabulary: point-cloud observation, sensor metadata, prior asset model, registration metadata, and calibration artifact. Candidate pipeline files bind the build executable, model and configuration. Candidate outputs have a separate closed role vocabulary: IFC model, element graph, geometry mesh and metrics table. Every scene requires both an IFC model and an element graph output.

Evaluator artifacts have a separate closed vocabulary: reference geometry, ground-truth labels, holdout assignment, official score table and independent checkpoints.

The CLI rejects shared artifact IDs and content digests. Each artifact carries a root-relative path; the verifier canonicalizes distinct, non-overlapping candidate/evaluator roots, rejects absolute paths, dot/parent components and observed symlink components, confirms regular files remain beneath the selected root, and recomputes SHA-256 over the actual bytes. The manifest contains relative paths and hashes, not data bytes or absolute local paths. The committed fixture files are synthetic text bytes only.

The exact argv array is persisted for audit and its digest is emitted, but the verifier does not execute it. The candidate source commit is syntax-checked as a Git SHA-1; it is not fetched or proven to correspond to the supplied binary. Build/model/configuration and output digests are recomputed over local files. This is **local integrity verification**, not proof of candidate execution or process isolation. Candidate and evaluator roots are both readable by this one verifier process. Standard-library path checks are not race-free against a concurrently malicious process; OS-level descriptor-relative no-follow opens/mounts, separate processes/identities and independent custody remain separate gates. Rights flags are still manifest assertions; this CLI does not inspect Git history or arbitrary CI artifact archives.

## Deliberate non-goals

- No point-cloud parsing, registration, reconstruction, meshing, IFC authoring or ray-cast visibility derivation.
- No implementation of official ISPRS completeness/correctness/accuracy.
- The official ISPRS results page defines accuracy using reference-surface samples and their closest source-model surface. The manifest records this canonical reference-to-candidate direction. The official scorer, historical sample generation, surface construction, cutoff/buffer curves, exclusions and aggregation are not yet implemented or reproduced.
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

The fixture contains only synthetic text bytes. It binds 2 point-cloud input placeholders, 3 pipeline artifacts, 4 candidate output artifacts and 2 evaluator reference placeholders. No ISPRS/BIMNet source data, derived geometry or restricted labels are committed or uploaded to CI.

## Evaluation profile semantics

- Coordinate reference identifies the frame, CRS, axis order, native linear unit and exact metres-per-unit conversion. Tolerances and sampling parameters are explicitly normalized to metres.
- The linear-unit conversion must match the declared unit (metre, millimetre, international foot or US survey foot); a mismatched scale fails closed.
- Surface distance cutoff and intersection buffer must be finite positive metre values.
- Reference/candidate surface spacings and point-cloud voxel size must be finite positive metre values; the random seed is recorded.
- Metric profile records a suite ID/version, a zero-origin strictly increasing buffer curve and distance-cutoff curve, and the official reference-to-candidate accuracy direction.

This binds the parameters needed for a later faithful scorer. It does not execute or qualify that scorer, nor does it claim that the synthetic IFC/graph placeholders are valid geometric models.
