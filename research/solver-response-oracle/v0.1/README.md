# Solver Response Oracle v0.1

A dependency-free Rust comparator for measuring whether full-candidate and reduced-manifold solver runs produce similar body-level generalized responses and relative point-velocity changes at the same frozen full-patch contact probes.

This crate does not implement a physics solver. It consumes snapshots produced by solver adapters. Version 0.1 is a 3D rigid-body comparison primitive (three linear and three angular components); higher-dimensional generalized responses require a separately versioned extension.

## Why common-space response matters

Different contact sets have different multiplier-vector dimensions and may distribute impulses differently. Comparing raw per-contact multipliers as the primary metric is therefore misleading. This oracle compares:

- body-level linear and angular velocity changes;
- body-level generalized linear and angular impulses;
- relative point-velocity response at common contact probes, resolved into normal and tangent components.

Each contact probe references two body IDs and stores world-space lever arms from each centre of mass to the same probe point. The same probe list must be used for both solver outputs.

## Fair comparison contract

The full and reduced snapshots must match exactly on:

- scenario digest;
- initial-state digest;
- probe-set digest for the frozen common full-patch probes;
- physics-model digest;
- solver-configuration digest;
- work-unit schema digest;
- declared equal-work budget ceiling.

Each snapshot also records actual work consumed, solver iterations, submitted solver rows, and a typed termination disposition (converged, iteration limit, work-budget exhaustion, divergence, abort or unsupported). Consumed work may differ under the same budget ceiling; it is reported rather than hidden. A comparison with either solver not converged remains a measurement, not a promotion pass.

Candidate-set and warm-start digests are recorded separately and may differ. The adapter must define its work-budget unit (for example, constraint-row visits or a deterministic operation count). Iteration count alone is not necessarily equal work when row counts differ.

The comparison uses explicit positive scales:

- characteristic length L;
- linear velocity scale v_scale;
- linear impulse scale p_scale.

Rotational velocity is converted to a linear scale using L times the angular-velocity difference. Angular impulse is converted using angular-impulse difference divided by L. The output is dimensionless, and the scales are part of the benchmark input, not chosen after observing results. The response record repeats all three scale values and the exact probe-set digest.

## Run

```sh
cargo fmt --manifest-path research/solver-response-oracle/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/solver-response-oracle/v0.1/Cargo.toml --locked
cargo run --manifest-path research/solver-response-oracle/v0.1/Cargo.toml --example response_tournament --locked
cargo clippy --manifest-path research/solver-response-oracle/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Output

The report includes:

- maximum generalized velocity and impulse response errors;
- maximum common-probe normal and tangent relative-velocity errors;
- RMS common-probe relative-velocity error;
- stable body/contact witness IDs;
- probe-set digest, work-unit schema and all normalization scales.
- exact scenario, state, model, solver, candidate-set, warm-start and work-budget identities;
- each solver's actual work, iteration count, submitted row count and termination disposition.

The example is a synthetic smoke tournament. It does not measure production Symtropy, Bullet, Jolt, Rapier, Newton or MuJoCo.

## Claim boundaries

- This compares outputs; it does not solve the contact problem or certify solver convergence.
- The full-candidate result is a same-solver reference, not physical truth.
- It does not replace complementarity/friction residuals, energy/work ledgers, static wrench support, temporal transfer, or independent physical-task validation.
- Common contact probes must be generated from the frozen full candidate set, not selected after observing which result looks better.
- Backend adapters must preserve raw solver outputs, warm-start provenance, solver settings, row counts and failure/termination dispositions.

Related programme: #7195, #7155, #7157, #7130, #7127, #7134, #3775, #4366 and #460.
