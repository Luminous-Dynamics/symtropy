# Locomotor Kinematic Invariant Reference V0

Standalone, dependency-free Rust reference metrics for [LOCO-INVARIANT-001 / #3958](https://github.com/Luminous-Dynamics/symtropy/issues/3958).

## Purpose and claim boundary

This crate tests a narrow measurement hypothesis: whether a distal-to-proximal gradient in movement velocity can be expressed in a dimensionless way that remains comparable under explicit body-size and cycle-time scaling.

It is a **measurement/normalization oracle only**. It does not infer:
- a universal locomotor law;
- anatomical homology or joint correspondence;
- shared neural dynamics or a universal CPG;
- a species-general controller;
- biological causality or controller-transfer performance.

The candidate gradient is a descriptive statistic motivated by Hosseini et al. (PNAS, 2026), who report species-specific locomotor strategies alongside a common distal-to-proximal gradient in joint movement velocities across their studied fly, mouse and human data. Reproduction of their analysis requires source-pinned data, preprocessing and metric definitions; this crate does not claim that reproduction.

## Data model

Each trial binds:
- species, individual and session identifiers;
- body length and gait-cycle duration with explicit SI units;
- native measurement quantity (joint angle in radians or segment displacement in metres);
- a normalized proximal coordinate (0 = distal, 1 = proximal) defined for that body's declared measurement chain;
- raw timestamped observations for each channel.

The proximal coordinate is a measurement ordering, **not a homology claim**. A mapping between species must be justified by an independently versioned anatomical or functional correspondence record.

Every declared channel has an observation record. `samples: None` means the channel was unavailable. It is not filled with zeros. Missing channels remain visible in the output and are excluded from the regression only as explicit missing observations.

## Time-weighted velocity and normalization

For irregular timestamps, each interval has velocity:

    v_i = (x[i+1] - x[i]) / (t[i+1] - t[i])

The time-weighted RMS velocity is:

    RMS(v) = sqrt(sum(v_i^2 * dt_i) / sum(dt_i))

The normalized channel values are quantity-specific:

    joint angle:       nu = cycle_duration * RMS(theta_dot)
    linear displacement: nu = cycle_duration / body_length * RMS(x_dot)

Angular and linear observables are never pooled into one gradient. The original timestamps are used directly; this crate does not resample cycles to a fixed bin count or discard the original time warp.

For one quantity family, the descriptive gradient is the least-squares slope:

    nu_j = intercept + slope * proximal_coordinate_j + residual_j

A positive slope means greater normalized RMS velocity toward the proximal end under the declared coordinate convention. The estimator requires at least three observed channels and reports the number used and the number missing.

## What is tested

Nine deterministic tests cover:
- invariance of angular normalized velocity under cycle-time scaling when the within-cycle movement is unchanged;
- invariance of linear normalized velocity under coupled body-size and time scaling;
- time-weighted velocity under irregular timestamps;
- exact known-answer distal-to-proximal gradient recovery;
- quantity-family separation;
- explicit missing-channel handling without zero imputation;
- non-monotonic timestamp rejection;
- duplicate channel identity rejection;
- missing observation record rejection.

These tests validate the formula and guards on synthetic fixtures only.

## Required next increments

1. Add measurement uncertainty and tracking-confidence propagation instead of treating landmark trajectories as exact.
2. Add anti-aliasing / filtering sensitivity studies with filters fitted and selected only on training data.
3. Add multiple cycles and animal-clustered aggregation; never treat frames as independent biological replicates.
4. Add explicit source adapters and versioned anatomical/functional correspondence tables.
5. Run native-time and cycle-normalized analyses side by side, preserving cycle duration and resampling maps.
6. Add leave-one-species-out and leave-one-study/site-out splits, plus shuffled-correspondence and normalization-artifact negative controls.
7. Compare against pinned AutoGaitA outputs as an external descriptive-analysis baseline after license and provenance review.
8. Keep descriptive invariance, held-out predictive transfer, mechanistic invariance and controller transfer as four separate claims.

## Run

    cargo fmt --manifest-path research/loco-invariant-kinematics-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-invariant-kinematics-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-invariant-kinematics-v0/Cargo.toml --locked --all-targets -- -D warnings

## References

- Hosseini et al. (2026). *Cross-species identification of conserved and divergent locomotor kinematic strategies using AutoGaitA*. PNAS 123(35):e2534093123. https://doi.org/10.1073/pnas.2534093123
- AutoGaitA source: https://github.com/mahan-hosseini/AutoGaitA (GPL-3.0; external baseline only).
- Symtropy LOCO-HIER-001: https://github.com/Luminous-Dynamics/symtropy/issues/3955
