# LOCO Contact Evidence V0

Dependency-free Rust reference for graded, provenance-preserving foot-ground reliability evidence using D1/D2/D3 sensing channels.

Research parent: [LOCO-CONTACT-EVIDENCE-001 / #4143](https://github.com/Luminous-Dynamics/symtropy/issues/4143).

## Evidence targets must remain distinct

A source reports a reliability interval for one declared target:
- contact presence;
- validity of a no-slip foot-ground kinematic constraint;
- traction feasibility.

These cannot be collapsed into one contact-confidence scalar. Contact presence does not establish a valid no-slip kinematic constraint; a no-slip kinematic constraint does not establish that a planned tangential force is inside the friction cone.

## Channels

- D1 InterfaceFields: pressure/shear and contact geometry.
- D2 NearFootWrenchVibroAcoustic: near-foot wrench and vibro-acoustic transients.
- D3 ProprioceptiveResidual: joint/body-dynamics inference.

Nominal gait phase is not a sensing channel and cannot independently satisfy the evidence gate.

## Implemented

- Exact per-foot and per-contact-event identity.
- Declared target, constraint digest, operating-domain digest and per-source calibration digests.
- Per-source reliability interval in integer parts per million.
- Per-source health and out-of-distribution flags.
- Freshness, future timestamp and inter-source skew checks.
- Explicit Unknown result for uncertain clock or inadequate eligible source count.
- Deterministic source-by-source assessments and ordering.
- Dependence-safe aggregation by convex hull, not multiplication or intersection.
- Common-intersection diagnostic to flag source disagreement while preserving the full hull.
- Eighteen deterministic tests covering D1/D2/D3 fusion, stale/invalid sources, identity/calibration mismatch, timing, disagreement, thresholds and replay.

## Why convex hull, not naïve Bayesian multiplication

The D1/D2/D3 channels may share mechanics, environment, calibration, compute or training data. Their errors cannot be assumed conditionally independent by default. Multiplying likelihoods can create unjustified confidence.

For eligible intervals I_i = [l_i, u_i], V0 reports:

    hull(I_1, ..., I_n) = [min_i(l_i), max_i(u_i)]

The common intersection is diagnostic only. It is never substituted for the hull. Disjoint intervals are explicitly flagged as disagreement. The hull can be wide; that is intentional.

The supplied bounds remain estimator outputs. This package does not prove their statistical coverage or calibration. A calibration digest identifies the source protocol; it is not itself evidence that the protocol is correct.

## Reproducing the current contact-estimation baseline

Zhao et al. (Sensors 2026, 26(19), 6041) provide a continuous-contact-reliability neural residual Kalman-filter baseline. The paper reports 14.32 ms mean temporal-estimator inference, 26.46 ms mean full-pipeline latency, 36.24 ms maximum and a roughly 50 ms update period on its experimental system. It reports reduced state RMSE versus Binary Contact KF on flat and mildly uneven simulation, plus indoor walking/turning hardware results. Those figures are specific to that paper's system and conditions.

- DOI: https://doi.org/10.3390/s26196041
- PubMed: https://pubmed.ncbi.nlm.nih.gov/42884241/

The broader sensing-channel taxonomy is from the 2026 contact-value survey: https://doi.org/10.1007/s10462-026-11644-0.

## Commands

    cargo fmt --manifest-path research/loco-contact-evidence-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-contact-evidence-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-contact-evidence-v0/Cargo.toml --locked --all-targets -- -D warnings

A dedicated exact-head workflow is defined at .github/workflows/loco-contact-evidence.yml.

## Claim ceiling

This package aggregates supplied calibrated bounds. It does not train or validate a neural estimator, prove probability coverage, estimate force, enforce Coulomb friction, run a Kalman filter, establish sensor independence, compute physical viability or authorize a robot action. It is an evidence-contract primitive only.