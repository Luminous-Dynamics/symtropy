# MuonColliderSoft Metrics Admission

This Rust tool independently verifies the integrity and provenance of public MuonColliderSoft reconstruction metric artifacts. It does not recalculate or judge the physics metrics.

## Run

\`\`\`sh
cargo test --manifest-path tools/mucoll-metrics-admission/Cargo.toml
cargo fmt --manifest-path tools/mucoll-metrics-admission/Cargo.toml -- --check
cargo clippy --manifest-path tools/mucoll-metrics-admission/Cargo.toml --all-targets -- -D warnings

cargo run --manifest-path tools/mucoll-metrics-admission/Cargo.toml -- \\
  --report metrics.json \\
  --policy docs/research/muon-collider/mucoll-metrics-admission.v0.1.json \\
  --input reco.edm4hep.root \\
  --fragments-dir metrics-fragments \\
  --submission submission.json \\
  --output metrics-admission-receipt.json
\`\`\`

The fragment directory must contain the exact JSON files named in the aggregate report.

## What it verifies

- Exact RECO input byte count and SHA-256.
- Exact submission.json SHA-256 and equality between raw and embedded production metadata.
- Locked MuonColliderSoft benchmark and Spack source revisions.
- Clean benchmark checkout, stack version, immutable container image ID, platform and geometry.
- GitHub Actions run ID, attempt, commit and ref agreement across the aggregate and submission records.
- Required tracks, seeds, hits and photons fragments.
- Fragment names, file SHA-256, study identity, producer identity, configuration, metrics and event-count agreement.
- JSON numeric finiteness. Explicit null metrics remain unavailable, not zero.

## Claim boundary

A successful receipt means **artifact integrity and execution provenance admitted**. It does not mean that tracking efficiency, fake rate, occupancy, photon response or any physics result passed a tolerance.

The current upstream benchmark documentation states that physics pass/fail tolerances have not yet been applied. Accordingly this tool always emits:

- \`physics_comparison = not_evaluated\`
- \`qualification_decision = abstain\`

No arbitrary physics limits are included. A future reference gate must be a separate, source-pinned object with admitted reference samples, replicate statistics, statistical coverage, systematic uncertainty and predeclared tolerances.
