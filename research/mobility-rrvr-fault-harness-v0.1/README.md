# RRVR Deterministic Crash-and-Replay Harness v0.1

This isolated, standard-library-only Rust crate models a small deterministic process-fault campaign. It is designed to exercise the ambiguity where an endpoint may have applied an effect but the coordinator lost the acknowledgement.

## Run

- Format: `cargo fmt --manifest-path research/mobility-rrvr-fault-harness-v0.1/Cargo.toml -- --check`
- Run known campaign: `cargo run --locked --manifest-path research/mobility-rrvr-fault-harness-v0.1/Cargo.toml --bin rrvr_fault_harness`
- Test: `cargo test --locked --manifest-path research/mobility-rrvr-fault-harness-v0.1/Cargo.toml`
- Clippy: `cargo clippy --locked --manifest-path research/mobility-rrvr-fault-harness-v0.1/Cargo.toml --all-targets -- -D warnings`

## Model boundary

The harness models coordinator, endpoint, dedupe state, authority epoch, queue generation, observer, inventory and delayed-message delivery. Its event scheduler uses logical ticks and stable insertion sequence numbers; it does not depend on wall-clock scheduling.

The `native_retry` arm is an intentionally simple baseline that interprets a missing acknowledgement as permission to issue a new command identity. It is not a claim about every production robot stack. The `rrvr` arm observes the original operation, advances authority/queue generation, requires graph closure, and admits a new attempt only for a fully observed NoEffect result.

## Important limits

- The "physical effect" is a simulated state transition inside this model.
- Observer independence, complete holder/edge inventory, transport enforcement and endpoint durability are scenario declarations, not facts proven by this harness.
- No simulator, Gazebo, ROS, HIL or physical robot integration is included.
- The output is a process-fault model report, not RRVR scorecard v0.1 raw schema or an evidence capsule accepted as independent physical observation.
- The fixed scenarios and tests are synthetic model cases. They do not establish safety, PL/SIL, certification, field reliability or SOTA superiority.

The next step is to adapt emitted model events into the evidence-capsule and scorecard schemas, then run the same fault protocol through an instrumented simulator endpoint.
