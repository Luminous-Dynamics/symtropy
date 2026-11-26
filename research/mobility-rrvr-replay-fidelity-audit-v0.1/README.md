# RRVR Replay-Fidelity Audit v0.1

This isolated Rust crate audits action-prefix replay fidelity for a pinned RoboRecover-compatible profile. It computes pose displacement from supplied simulator state snapshots and SSIM from the raw bytes of five original/replayed PNG frame pairs. It does not accept caller-supplied scalar SSIM or pose-error fields.

## RoboRecover profile

The named profile, `RoboRecoverReplayV1`, fixes:
- exactly 20 difficulty-screen trials from four named reference policies, five trials each;
- between 2 and 18 successful reference recoveries, inclusive;
- exactly three distinct replay spot-check attempt IDs, recorded under seed 42;
- end-effector and object positional deviations strictly below 2 mm;
- mean SSIM strictly above 0.99 across exactly five final pre-recovery frame pairs;
- same declared coordinate-frame identity for each original/replayed pose pair.

The image profile is pinned to PNG decode through png 0.18.1, 8-bit RGB sample space, alpha composited over white, 11×11 Gaussian SSIM windows (sigma 1.5), K1=0.01, K2=0.03, and arithmetic mean over valid window centers and RGB channels. Input samples are the decoded 8-bit sRGB-encoded values; no unversioned color-space conversion is applied. A change to decoder, alpha handling, window, sigma, constants or aggregation requires a profile version change.

## Run

- Format: `cargo fmt --manifest-path research/mobility-rrvr-replay-fidelity-audit-v0.1/Cargo.toml -- --check`
- Audit: `cargo run --locked --manifest-path research/mobility-rrvr-replay-fidelity-audit-v0.1/Cargo.toml --bin rrvr_replay_audit -- MANIFEST.json ARTIFACT_ROOT`
- Tests: `cargo test --locked --manifest-path research/mobility-rrvr-replay-fidelity-audit-v0.1/Cargo.toml`
- Clippy: `cargo clippy --locked --manifest-path research/mobility-rrvr-replay-fidelity-audit-v0.1/Cargo.toml --all-targets -- -D warnings`

Image paths must be relative to the supplied artifact root. Absolute paths, parent traversal and symlinks resolving outside the root are rejected. Raw benchmark datasets are not vendored; preserve source provenance and licensing outside this crate.

## Output

Each candidate receives Valid / Invalid / Unresolved with separate pose checks, five frame SSIM checks, artifact digests, and typed reason codes. Train/test leakage checks compare declared host-specific leakage keys. Repeated trajectory-cluster IDs are allowed within a split and retained for cluster-aware inference; cluster identity is not treated as a scenario uniqueness key.

A replay-valid checkpoint is a precondition for a matched RRVR recovery comparison. A replay-valid outcome does not mean the policy succeeded, a physical effect was safe, a command graph was closed, or the host benchmark score improved.

## Claim ceiling

This is an offline artifact auditor. It has not yet reproduced the RoboRecover dataset, run RoboTwin/LIBERO, or qualified a simulator, HIL or physical-robot campaign. Its computed SSIM is only as trustworthy as the source artifact, decoder/profile and provenance. This crate alone does not prove source authenticity or that the original and replay images came from the claimed simulator run.
