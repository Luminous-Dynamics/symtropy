# symtropy-animal-observation

A source-neutral Rust validator for animal-observation reference manifests and leakage-resistant evaluation splits.

## Current scope

- Versioned source/dataset snapshots, optional SHA-256 payload digest with explicit requirement.
- Taxonomy source/release and **multiple taxon IDs per pack**.
- Measurement units, coordinate frames, temporal semantics and sampling interval checks.
- Operation-scoped permission receipts for metadata access, local analysis, transformations, evaluation, training, redistribution, publication and sensitive location output.
- Privacy gates that require a declared export-resolution ceiling for sensitive wildlife locations.
- Explicit provenance class and a conservative measurement-modality / claim compatibility matrix.
- Split qualification for unseen subject, study, site, taxon, future-time prediction and within-subject interpolation.
- Stable, order-independent canonical manifest digest.
- Frozen synthetic oracle with 66 expected-outcome cases, plus unit tests for scope, split and digest invariants.

The crate checks supplied permission receipts; it is not the authoritative data-rights system. It fetches no external payloads, grants no permission, and makes no claim of biological accuracy.

## Important semantic boundaries

A source can support one claim without supporting another. In particular, still 2D keypoints cannot by themselves establish 3D kinematics, contact force, physiology or causal behavior. Multi-animal tracking on unseen categories is a distinct visual-association task, not evidence of validated animal cognition or species physiology.

Train/test leakage is checked according to a declared generalization target. Track IDs are study-scoped; subject, site, duplicate-group, and taxon IDs are canonical within the pack. Namespace raw study-local subject IDs by study unless evidence-backed alias resolution links the same animal across studies. Future-time evaluation may reuse a track only when time windows are strictly separated by the declared gap. Within-subject interpolation requires the same study/subject/track to span roles, while overlapping windows across roles are rejected.

Registry/manifest eligibility is not empirical model validation.

## Focused qualification

From the workspace root:

```sh
cargo fmt --check --manifest-path crates/domains/symtropy-animal-observation/Cargo.toml
cargo test --locked --manifest-path crates/domains/symtropy-animal-observation/Cargo.toml
cargo clippy --locked --manifest-path crates/domains/symtropy-animal-observation/Cargo.toml --all-targets -- -D warnings
```

A passing claim must be bound to the exact code and corpus head. Queued, skipped, stale, missing, or wrong-head checks do not count as PASS.
