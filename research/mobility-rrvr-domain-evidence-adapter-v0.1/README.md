# RRVR Domain-Qualified Evidence Adapter v0.1

This adapter projects deterministic process-fault harness traces into the RRVR evidence-capsule validator. It is the first executable integration between the harness (#7564) and the evidence validator (#7515).

## Run

- Format: `cargo fmt --manifest-path research/mobility-rrvr-domain-evidence-adapter-v0.1/Cargo.toml -- --check`
- Execute: `cargo run --locked --manifest-path research/mobility-rrvr-domain-evidence-adapter-v0.1/Cargo.toml --bin rrvr_domain_evidence_adapter`
- Tests: `cargo test --locked --manifest-path research/mobility-rrvr-domain-evidence-adapter-v0.1/Cargo.toml`
- Clippy: `cargo clippy --locked --manifest-path research/mobility-rrvr-domain-evidence-adapter-v0.1/Cargo.toml --all-targets -- -D warnings`

## What it emits

For each of the six matched scenarios and both arms, the adapter emits:
- The full process-model outcome and trace.
- A trace digest that binds the ordered logical events.
- A nested `EvidenceBundle` explicitly marked `evidence_domain: process_model`.
- Model-oracle endpoint-fence and temporal-closure receipts, with parent relationships and logical tick intervals.
- An assessment produced by calling the actual `mobility-rrvr-evidence-capsule::evaluate_json` implementation.

The adapter uses the public `seal_bundle` API. It does not duplicate capsule-digest or claim-snapshot logic.

## Domain restrictions

Every evidence source in a process-model bundle is `model_oracle`, with one shared principal and one shared independence group. This deliberately expresses a single correlated model source; it is not independent physical corroboration.

The evidence validator only allows `EndpointFenced` and `TemporalClosure` claims in the process-model domain. Physical observation, fault-delivery, `Applied`, and `NoEffect` claims are rejected. The model's effect count, reported disposition, retry decision and other outcomes remain in the outer process-model result and are not relabeled as physical observations.

The model's abstract command graph contains a controller holder, an actuator holder and one forwarding edge. These are model entities projected from the deterministic reference harness. They are not an inventory of a real robot's command paths.

## Claim ceiling

This output qualifies the model's own endpoint/closure assertions under its declared synthetic scenario. It is not simulator evidence, HIL, physical observation, proof of real-world source independence, a robot safety result, PL/SIL, certification, field reliability or SOTA superiority.

The next qualification tier requires a simulator adapter with actual simulator state and command/effect instrumentation. HIL and physical adapters require independently instrumented hardware and domain-specific trust/calibration profiles; they must use separate schemas and cannot upgrade this process-model envelope by changing a string.
