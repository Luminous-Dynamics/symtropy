# Wood Robotics Platform Audit

Standalone Rust checker for the frozen synthetic portfolio corpus. It checks evidence and mission contract consistency, not robot performance or physical safety.

## Run locally

```sh
cargo fmt --manifest-path tools/wood-robot-platform-audit/Cargo.toml -- --check
cargo test --manifest-path tools/wood-robot-platform-audit/Cargo.toml
cargo run --quiet --manifest-path tools/wood-robot-platform-audit/Cargo.toml -- \
  docs/tech/wood-robot-platform-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)" > wood-robot-platform-audit-receipt.json
```

The source commit must be a 40-character hexadecimal Git SHA. The receipt binds the exact corpus bytes via SHA-256, source commit, scenario counts, decisions and contract findings.

## Contract covered

- Corpus is explicitly synthetic and `plan_only_no_actuation`.
- Scenario IDs are unique; expected scenario count matches actual inputs.
- Every scenario has a known decision, non-empty reason codes and `expected_actuation=false`.
- An eligible candidate cannot ignore generic task blockers such as missing approval, expired calibration, fixture mismatch, localization loss, required extraction/interlock failures, or insufficient return reserve.
- The aerial Scout cannot be considered plan-eligible without verified site/flight permissions, clear airspace/geofence, declared weather and wildlife policies, no detected disturbance, configured privacy handling, valid recovery, stand-off compliance, no contact or payload release, current localization, human approval and sufficient return reserve.
- Adversarial tests mutate the frozen positive fixture to prove that key aerial gates fail closed.

## Claim ceiling

A passing result establishes only that the declared synthetic corpus conforms to this checker at the identified revision. It is not evidence of field accuracy, regulatory authorization, absence of wildlife disturbance, aircraft safety, machine guarding, timber grade, structural approval, or production readiness.