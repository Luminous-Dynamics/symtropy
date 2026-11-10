# Black-hole response evidence manifest

This crate defines a machine-readable evidence contract for the causal response
pilot in issue #4154 and the canonical Green-function work in issue #4109.

## Purpose

A paper reproduction, a numerically stable run, an independent cross-check and
a converged physical response are different evidence states. The manifest
records the model and convention identity, source and observer, exact code
revision, dependency-lock and input digests, reproduction command, environment,
solver and precision, method identities, component-wise error budget, causal
response windows, refinement sequences and categorized negative controls.

The JSON Schema is published in the schema directory. Rust validation adds
cross-field qualification rules.

## Qualification ladder

| Qualification | Minimum meaning |
|---|---|
| unresolved | A run/manifest exists; physical reproduction is not qualified. |
| analytic_oracle | A versioned analytic or published oracle is pinned. |
| numerically_reproduced | At least one numerical method and named input digests are recorded against the oracle. |
| independently_cross_checked | At least two methods from distinct implementation digests are recorded. |
| converged | Independent cross-check, accepted response windows, at least three monotone refinement levels, all seven error-budget categories resolved, and all twelve Pöschl–Teller negative controls passed. |

These are evidence gates, not claims that an equation or reference is correct.
Changing the qualification enum cannot bypass the corresponding evidence.

## Error-budget categories required for converged

- formula_transcription
- frequency_root_residue
- time_domain_discretization
- spectral_truncation
- contour_quadrature
- finite_precision
- source_observer_sampling

Unknown error blocks convergence. Not-applicable is allowed only with a
rationale. Error components remain separate; aggregate mismatch must not hide
cancellation between errors.

## Required negative controls for converged

1. wrong_fourier_sign
2. swapped_boundary_conditions
3. wrong_wronskian_orientation
4. wrong_matsubara_spacing
5. qnm_sum_outside_convergence_region
6. matsubara_sum_outside_convergence_region
7. omitted_branch_cut_or_direct_term
8. pre_source_response
9. source_change_mislabeled_as_geometry
10. unmatched_state_scaling
11. source_overlap_change_must_not_move_poles
12. precision_loss_near_cancellation

A control passes only when the observed failure category equals the preregistered
expected category. Something failing for the wrong reason is not a pass.

## Use

~~~rust
use bh_response_manifest::{parse_and_validate_json, sha256_hex};

let bytes = std::fs::read("initial-data.bin")?;
let input_digest = sha256_hex(&bytes);
let text = std::fs::read_to_string("response-manifest.json")?;
let manifest = parse_and_validate_json(&text)?;
~~~

~~~sh
cargo fmt --manifest-path tools/bh-response-manifest/Cargo.toml -- --check
cargo clippy --manifest-path tools/bh-response-manifest/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/bh-response-manifest/Cargo.toml --all-targets
~~~

## Scientific and security claim boundary

- The validator checks digest format, not digest truth. Recompute hashes from
  preserved bytes before accepting a replication receipt.
- It does not execute solvers, validate boundary conditions, prove PDE
  convergence, authenticate a reference or establish physical interpretation.
- Distinct implementation digests are necessary, but not sufficient, for
  methodological independence; independent numerical analysis and review are
  still required.
- The monotone-refinement rule is a conservative minimum. Non-monotone
  high-order convergence, cancellation-dominated observables and adaptive
  methods need a separately justified extension, not a weakened gate.
- A passing validator test suite is a schema/control result, not a black-hole
  physical reproduction.

## Paired-comparison contract — schema v0.2.0

Schema v0.2.0 adds a comparison record that binds the *claim of independent agreement* to the actual method pair and a named response window.

Every method records:
- its implementation digest;
- the digest of the canonical input bundle it actually consumed;
- the response artifact digest.

Every paired comparison records:
- left and right method IDs;
- the response-window ID and norm;
- the digest of the aligned time/frequency sample grid and weights;
- the digest of the comparison artifact (aligned values plus metric calculation);
- observed error, acceptance limit and a derived acceptance flag.

For `independently_cross_checked` and `converged`, every declared response window must have an accepted paired comparison. A valid pair must use distinct implementation digests and the same input-set digest. The window summary must match the comparison's norm, measured error and limit.

This closes an important metadata gap in v0.1: two implementation hashes and a free-standing error value no longer suffice to declare a cross-check. A verifier can follow the exact evidence chain from methods to aligned samples to comparison artifact to causal response window.

### Trust boundary

The v0.2 validator checks references, digest shape, same-input identity, method independence by implementation digest, norm/window consistency, and error/limit consistency. It does **not** recompute digests or the metric from bytes. The independent evidence consumer must retrieve the preserved response and comparison artifacts, recompute their digests, and reproduce the declared alignment and norm before accepting the scientific result.

Schema v0.1.0 is retained as a historical contract. New paired-comparison receipts use `bh-response-manifest-0.2.0.schema.json` and crate version `0.2.0`.

The paired comparison also pins two **procedure digests**:
- `alignment_definition_sha256` identifies the exact time/frequency sample alignment, interpolation, cropping and weighting procedure/configuration;
- `metric_definition_sha256` identifies the exact norm/metric formula and its configuration.

These are distinct from `aligned_samples_sha256` (the resulting aligned sample set) and `comparison_artifact_sha256` (the resulting values and metric record). The four identities make it possible to detect the same arrays being scored under a changed alignment or metric definition.

For qualification, **every listed method must participate in at least one accepted paired comparison**; an unreferenced method cannot be counted toward independent evidence. Each response-window ID has one pairwise comparison record. For three or more representations, create one pair-specific response-window record per comparison (the `causal_region` may be the same, but each `window_id` is unique), so each pair has its own error, norm and limit.

## v0.2 example

`examples/paired-comparison-v0.2.json` is a complete serialization fixture for an independently cross-checked direct-PDE / frequency-Green comparison. Its hash strings and numerical error are illustrative placeholders, not execution receipts or scientific measurements. The Rust unit suite parses and validates this fixture end-to-end; real run manifests must replace every placeholder with digests recomputed from preserved bytes.

The manifest provenance now carries `canonical_input_set_sha256`. Every method's `input_set_sha256` must match it exactly. This prevents a pair of methods from agreeing with each other on an input digest that is not bound to the study's own provenance record. Individual source/input files remain separately listed and hashed under `input_digests`.

