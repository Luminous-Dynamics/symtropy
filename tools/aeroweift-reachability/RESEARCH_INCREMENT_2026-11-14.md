# AeroWeft reachability research increment — 14 November 2026

This note binds the current reachability workspace revision to the expanded benchmark programme.

## Independent analytic-extrema integration oracle

The integration test at tests/independent_analytic_extrema_oracle.rs checks the entire continuous occupancy hull for two contiguous constant-velocity slices on all three axes. It derives the endpoint extrema analytically from the declared interval bounds, rather than relying only on a sampled grid. The fixture includes positive, negative and sign-changing velocity intervals.

The test only qualifies the bounded constant-velocity integration arithmetic. It does not validate aircraft equations, bank-angle dynamics, airspeed, wind evolution, a physical aircraft, or any real operating envelope.

## Benchmark programme expansion

The current benchmark specification also includes:
- an aviation-specific reciprocal recoverability measurement layer (#5207);
- an exhaustive tiny finite oracle for reciprocal options (draft PR #5225);
- an aviation-specific recovery-network/common-cause benchmark (#5224);
- matched references to 2026 UAM integrated, demand-uncertain, and demand-supply co-optimization studies;
- the distinction between paper-level comparisons, transparent algorithm-class baselines, and faithful reproductions.

## Verification status

The dedicated Cargo workflow is the authority for exact-head format, Clippy and test results. Source and tests may be authored before the workflow runs, but no pass is claimed until the exact commit SHA completes all required jobs. A pending, queued, skipped, cancelled or absent workflow is not a pass.
