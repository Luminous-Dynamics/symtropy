# RRVR Replay-Fidelity Audit v0.1

This isolated Rust crate audits three independently selected action-prefix replay attempts for each RoboRecover-style scenario. Each attempt has its own end-effector comparison, object comparison, and final-five-frame image comparison. All three attempts must pass the declared replay gate for the scenario to be marked valid.

## Separate the host thresholds from the metric implementation

The `RoboRecoverPublishedThresholdsV1` profile encodes the published acceptance criteria:
- 20 screening trials across four reference policies, five trials per policy;
- between 2 and 18 successful recoveries in those 20 trials, inclusive;
- three selected inference attempts, recorded with seed 42 and linked to the 20 screening attempt identities;
- end-effector and object position errors strictly below 2 mm for each of the three attempts;
- arithmetic mean SSIM strictly above 0.99 over exactly five final pre-recovery frame pairs for each selected attempt.

The profile does **not** claim that this crate reproduces the authors' exact SSIM implementation. The paper gives the threshold and aggregation but does not fully pin the image decoder, SSIM window/kernel, color handling or software revision. Our separate `RRVR-SSIM-GAUSSIAN11-SIGMA1_5-RGB8-V0_1` metric profile is an explicit, reproducible candidate: PNG decoded by `png 0.18.1`; 8-bit RGB samples; alpha composited over white; 11×11 Gaussian windows with sigma 1.5; K1=0.01 and K2=0.03; mean across valid window centers and RGB channels, then mean across the five frame scores. The samples are the decoder's 8-bit channel values without additional color management or gamma normalization.

The public RoboTwin OOD replay verifier has a separate exact camera-video comparator using frame-by-frame RGB pixel differences (default pixel tolerance zero). That verifier is useful related work, but it is not the same calculation as the paper's SSIM acceptance criterion. This crate reports the selected metric profile in every manifest/report so results are not mislabeled as exact author-implementation parity.

## Three-attempt selection provenance

Each of the 20 difficulty-screening trials carries a unique attempt ID. A manifest identifies three selected attempts, each of which must be found in that screening ledger. It also includes a selection-method identifier and a path/digest for the upstream selection receipt (seed, method ID and selected IDs). The auditor verifies the receipt bytes and the selected-attempt membership.

The paper reports seed 42, but the selection algorithm/library details are not sufficient to assert that this Rust auditor independently regenerates the authors' random draw. The audit therefore preserves the upstream method identifier/receipt; it does not claim to reproduce the RNG stream unless the upstream algorithm is independently established.

## Replay state and split integrity

- Original and replayed positions are compared in a shared, explicitly named coordinate frame, with coordinates in millimetres.
- Each of the three attempts has five ordered original/replayed PNG pairs. Raw bytes are SHA-256 checked before image decoding.
- Train/test leakage keys are checked across scenarios within each platform.
- Repeated trajectory-cluster IDs within a split are permitted and retained for cluster-aware analysis; cluster IDs are not scenario uniqueness keys.
- All three selected attempts must pass. An absent, invalid or unresolved attempt is not silently replaced.

The SSIM threshold applies to the mean of the five frame scores **within each selected attempt**, not to each frame independently. Scenario validity requires all three selected attempts to qualify.

## Run

- Format: `cargo fmt --manifest-path research/mobility-rrvr-replay-fidelity-audit-v0.1/Cargo.toml -- --check`
- Audit: `cargo run --locked --manifest-path research/mobility-rrvr-replay-fidelity-audit-v0.1/Cargo.toml --bin rrvr_replay_audit -- MANIFEST.json ARTIFACT_ROOT`
- Tests: `cargo test --locked --manifest-path research/mobility-rrvr-replay-fidelity-audit-v0.1/Cargo.toml`
- Clippy: `cargo clippy --locked --manifest-path research/mobility-rrvr-replay-fidelity-audit-v0.1/Cargo.toml --all-targets -- -D warnings`

Image paths must be relative to the supplied artifact root. Absolute paths, parent traversal and symlinks resolving outside the root are rejected. Raw benchmark datasets are not vendored; preserve source provenance and licensing outside this crate.

## Output and claim ceiling

Each selected attempt receives a separate Valid / Invalid / Unresolved result with pose checks, five per-frame SSIM values, a five-frame mean, artifact digests and typed reasons. The scenario is valid only when the difficulty screen, selection receipt, and all three spot-check attempts qualify.

This is an offline artifact auditor. It has not reproduced RoboRecover's full dataset, run RoboTwin/LIBERO, or qualified a simulator, HIL or physical-robot campaign. The SSIM metric profile is a reproducible RRVR candidate and requires parity testing against author/reference outputs before claiming exact benchmark reproduction. A replay-valid checkpoint is only a precondition for a matched recovery comparison; it does not establish policy success, physical safety, effect finality, command-graph closure, or leaderboard improvement.
