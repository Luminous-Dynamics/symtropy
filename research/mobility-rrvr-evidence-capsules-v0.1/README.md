# RRVR Evidence Capsules v0.1

This isolated Rust crate validates the provenance structure of RRVR evidence and checks whether supplied evidence is sufficient for a small set of explicit assurance claims. It complements, but does not replace, the raw-record scorecard in `research/mobility-rrvr-benchmark-v0.2/evaluator/`.

## Run

- Format: `cargo fmt --manifest-path research/mobility-rrvr-evidence-capsules-v0.1/Cargo.toml -- --check`
- Validate: `cargo run --locked --manifest-path research/mobility-rrvr-evidence-capsules-v0.1/Cargo.toml --bin rrvr_evidence_validate -- research/mobility-rrvr-evidence-capsules-v0.1/fixtures/evidence_bundle_v0_1.json`
- Test: `cargo test --locked --manifest-path research/mobility-rrvr-evidence-capsules-v0.1/Cargo.toml`
- Clippy: `cargo clippy --locked --manifest-path research/mobility-rrvr-evidence-capsules-v0.1/Cargo.toml --all-targets -- -D warnings`

## Evidence model

Each capsule binds a stable evidence ID and episode/attempt/command scope to a declared source, parent evidence IDs, interval-valued event and observation times, an ingest sequence, a typed payload, and a SHA-256 digest. Parents must exist, remain within the same attempt scope, and have a strictly earlier ingest sequence. This monotonic order rules out provenance cycles.

The digest is computed over the deterministic Serde JSON representation of the capsule with `content_sha256` set to the empty string. The claim snapshot digest is SHA-256 over a lexicographically sorted array of `[evidence_id, content_sha256]` pairs. Struct field order is part of schema v0.1; changing it requires a schema-version change.

The crate checks five claim classes:
- Fault delivered: injector execution receipt plus a physical observation from a different declared independence group.
- Effect applied / no effect: a physical observation for the effect identity. An absence claim additionally requires complete observation coverage over a declared interval.
- Endpoint fenced: endpoint receipt with accepted epoch, retired high-water, queue generation, no outstanding/in-flight work, and terminal actuator fencing.
- Temporal closure: a complete independent holder/edge inventory; a qualifying fence receipt for every holder; and old-epoch enqueue closure for every inventoried edge.

A claim's decision cutoff is an ingest sequence. Evidence ingested later and not named in the claim snapshot is counted as excluded late evidence; it cannot silently rewrite the earlier claim. A new claim or superseding assessment is required.

## Trust and claim limits

A digest detects content changes relative to the declared digest; it does not prove that a statement is true. This crate does not validate digital signatures, key custody, physical sensor calibration, observer independence in the real world, or the completeness of an externally supplied command-holder inventory. Independence groups and source roles are declarations that must be justified by the campaign's trust model.

A physical `NoEffect` conclusion is only as strong as the declared continuous observation window and measurement design. The validator checks supplied coverage metadata; it cannot prove that the sensor actually observed the whole physical domain.

The synthetic fixture qualifies schema, hashing, provenance and evaluator behavior only. It is not a robot campaign, fault-delivery result, physical safety result, HIL result, PL/SIL result, certification or field-reliability claim.
