# Robotics Fault-Response Dynamics Auditor

Standalone Rust contract checker for paired no-fault/challenge records. It measures event timing and capability retention while keeping hard safety failures separate from false abstention.

## Run

```sh
cargo test --manifest-path tools/robotics-fault-recovery-audit/Cargo.toml
cargo run --quiet --manifest-path tools/robotics-fault-recovery-audit/Cargo.toml -- \
  fixtures/robotics/fault_response_pairs_v0_1.json \
  --source-commit "$(git rev-parse HEAD)" > fault-response-receipt.json
```

The frozen fixture contains synthetic records, not robot measurements. Its exact bytes and the evaluator source commit are included in the output receipt.

## Fail-closed invariants

- Every pair embeds matched control and challenge outcomes under the same policy/family/seed/evidence-class metadata.
- Control episodes cannot claim a physical fault or fault alert.
- Physical fault events need ordered, non-negative timestamps. Detected/reached/cleared/restored booleans must agree with their corresponding timestamps.
- Per-scenario detection and safe-state limits are checked when declared. Missing timing is never treated as zero.
- Any safety violation or required-safe-response deadline miss fails the safety gate.
- A service cannot be restored before fault clearance and explicit recovery authorization.
- A sham alert that causes an unnecessary abort is reported as false abstention rather than hidden in a composite score.

## Claim ceiling

The audit verifies this synthetic response contract only. It does not validate the physical system, sensor clock synchronization, fault model, or simulator.
