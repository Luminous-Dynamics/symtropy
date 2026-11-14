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

## Additional 2026 research increment

The benchmark suite now has three aviation-specific measurement tracks beyond kinematic reachability:
- Track H: infrastructure recovery portfolios and common-cause exposure (#5224).
- Track I: reciprocal recoverability and option externalities (#5207).
- Track J: endogenous demand, queueing and service-quality feedback (#5248).

Current model-class comparisons include:
- Lyu & Feng (2026), multinomial-logit demand + multi-server queueing + vertiport/vertipad location and capacity planning (DOI 10.1016/j.tra.2026.105027).
- Qin et al. (2026), bilevel vertiport capacity and commuter choice (DOI 10.1016/j.tre.2026.105103).
- Gottschalg et al. (2026), risk-averse two-stage site/fleet recourse under demand uncertainty (DOI 10.1016/j.tra.2026.104957).

These papers establish that adoption, waiting time, capacity and infrastructure are already being jointly studied. AeroWeft's proposed contribution is the composable evidence adapter across exact candidate schedules, aircraft/airspace constraints, recovery resources, per-cohort service outcomes and whole-life multimodal value. The new service-quality oracle is a tiny deterministic semantics test only; it does not estimate real demand or reproduce the published algorithms.

The South African source-admission line also preserves BVLOS and BRLOS as separate visual-rule and radio-link dimensions; a conditional B-VLOS route is not a blanket authorization or a BRLOS permission.
