# Robotics Persistent Mission-Series Auditor

Standalone Rust evaluator for a sequence of missions in which energy and declared state carry over between missions. This is a research contract check, not a simulator or physical-robot validator.

## Run

\`\`\`sh
cargo fmt --manifest-path tools/robotics-mission-series-audit/Cargo.toml -- --check
cargo test --manifest-path tools/robotics-mission-series-audit/Cargo.toml
cargo run --quiet --manifest-path tools/robotics-mission-series-audit/Cargo.toml -- \\
  fixtures/robotics/mission_series_v0_1.json \\
  --source-commit "$(git rev-parse HEAD)" > mission-series-receipt.json
\`\`\`

## Invariants

- Each comparison group has one conservative baseline and one candidate with identical scenario family, seed, initial state and mission schedule.
- Mission indices are contiguous; state tokens form an explicit prior/current chain.
- Battery transitions satisfy start + charged - consumed = end within the declared tolerance.
- Battery state remains within capacity; unexplained state-epoch changes fail integrity.
- Any reset must be explicitly authorized, carry a reason, and increment the epoch.
- Declared safety violations and unauthorized resets fail the safety gate.
- Metrics stay disaggregated; no composite score can trade away a hard-gate failure.
- Receipt binds exact input bytes via SHA-256 and records the evaluator source commit.

Opaque state tokens are only continuity markers. They are not signatures, trusted hardware attestations or proof of physical provenance.
