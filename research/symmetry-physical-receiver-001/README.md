# Independent symmetry-to-physical receiver v0.1

This is the first executable receiver for [PHYS-SYMMETRY-RECEIVER-001 #5663](https://github.com/Luminous-Dynamics/symtropy/issues/5663), stacked on the schema contract in [#4784](https://github.com/Luminous-Dynamics/symtropy/pull/4784).

## Authority boundary

The receiver does not copy a mathematical source-side PASS into a physical PASS. It independently checks the source receipt's exact bytes, generator-set binding, physical fixture and task-semantic bindings, then recomputes the bounded 3D coordinate laws.

The receiver can PASS the checked re-description obligations while the overall physical result remains INCONCLUSIVE. In v0.1 we do **not** receive an independently replayable physical one-step input/output trace, physical-observable trace, or calibrated operator error envelope. Therefore:
- discrete-step equivariance: INCONCLUSIVE;
- physical observables: INCONCLUSIVE;
- numerical error envelope: INCONCLUSIVE;
- physical equivalence authority: NONE;
- scientific hard-pruning authority: permanently false.

## Exact bytes and canonical fields

The source receipt is hashed as its exact raw byte sequence with a domain separator. The generator matrices and physical fixture fields use explicit versioned binary encodings: fixed field order, field labels, little-endian lengths, and IEEE-754 binary64 bit patterns. Each digest uses a distinct domain label. Task semantic components are separately bound and then combined into the top-level task digest.

A SHA-256 match is a content-identity check, not proof of the truth of the source receipt or physics.

## Physical checks

For q = S y, the receiver independently recomputes:
- M_y = S^T M_q S;
- force and momentum covectors f_y = S^T f_q and p_y = S^T p_q;
- constraint Jacobian J_y = J_q S;
- generator conjugation A_y = S^-1 A_q S;
- infinitesimal metric isometry A^T M + M A = 0;
- canonical force/velocity and momentum/velocity pairings;
- tangent mass norms and dual cotangent norms in both frames;
- all fixture and full-task digest bindings.

The transform, metric, generator and all derived arrays are checked for finite values and declared conditioning/residual limits. No coordinate component infinity norm is used as a physical norm.

## Reproduction

    cargo fmt --manifest-path research/symmetry-physical-receiver-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/symmetry-physical-receiver-001/Cargo.toml
    cargo clippy --locked --manifest-path research/symmetry-physical-receiver-001/Cargo.toml --all-targets -- -D warnings
    cargo run --locked --manifest-path research/symmetry-physical-receiver-001/Cargo.toml --bin verify-symmetry-handoff -- fixtures/synthetic-receiver-package-v0.1.json

## Test fixture and limits

The unit-test package is synthetic and exists to exercise exact byte binding, independent transformation checks, wrong-law mutations, digest mutation and fail-closed outcomes. It is not a real source receipt. The checker currently supports one bounded 3D dense coordinate-re-description, one SPD metric, a small generator set and a two-row full-rank constraint map.

It does not qualify arbitrary manifolds, nonlinear physical steps, contact/friction, sparse articulated dynamics or scientific search pruning. The next evidence required for a physical step PASS is a source-bound, independently replayable one-step trace with declared inputs, outputs, solver profile and numerical error envelope.
