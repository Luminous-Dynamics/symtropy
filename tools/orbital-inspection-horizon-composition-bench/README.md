# Orbital inspection piecewise-horizon composition benchmark v0.1

This package composes separately checked one-segment chord-tube outcomes over a declared action horizon. It is an evidence-completeness adapter, not a second trajectory geometry implementation.

## Contract

Every smooth segment carries its own:
- endpoint positions and endpoint-error bound;
- second-derivative norm upper bound;
- exact start/end TAI interval and duration;
- action, target, keep-out, transform, frame, time-basis and dynamics-profile scope;
- start/end event identities and a typed start-boundary classification.

The composer requires exact temporal adjacency and a complete horizon from the declared start through the declared end. It rejects missing segments, gaps, overlaps, duplicated IDs, out-of-order inputs, reused event identities at distinct seams, scope changes, and unsupported boundaries.

A smooth join and an impulsive velocity change are both treated as position-continuous boundaries. The second segment uses its own curvature bound; no derivative bound is carried across the seam. The adjacent endpoint uncertainty balls must overlap. This is a necessary consistency check, not proof that the actual physical seam state is correct.

Each segment is independently passed to the base crate. The horizon can be COMPLETE_CONDITIONAL_CLEAR only when every segment is CONDITIONAL_CLEAR and every coverage, scope, event and seam obligation passes.

## Dispositions

- COMPLETE_CONDITIONAL_CLEAR: all segments are locally clear and the complete declared horizon composes.
- NOT_PROVEN: at least one segment is not strictly clear.
- INCOMPLETE_COVERAGE: missing time, gaps, overlaps, wrong first start or wrong final end.
- SCOPE_MISMATCH: segment action/target/keep-out/transform/frame/time/dynamics scope differs.
- BOUNDARY_MISMATCH: unknown or unsupported boundary, event mismatch or repeated seam event.
- SEAM_INCOHERENT: adjacent endpoint uncertainty balls are disjoint.
- INVALID_INPUT, DUPLICATE_SEGMENT and OVERFLOW remain explicit non-success results.

## Claim boundary

The composer does not derive or authenticate endpoint errors or acceleration bounds. It does not validate orbital dynamics, target geometry, frame transforms, clocks, issuer identity, bytes, terminal passive safety, or actuator realization. The receipt hard-codes claim_ready=false, source_bytes_authenticated=false, authority_granted=false and physical_validation=NOT_ASSESSED.

A complete synthetic horizon result is not a #7573 physical-safety proof. The upstream continuous-time bound provider and source evidence require their own independent qualification.

## Reproduce

    cargo fmt --manifest-path tools/orbital-inspection-horizon-composition-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/orbital-inspection-horizon-composition-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/orbital-inspection-horizon-composition-bench/Cargo.toml --all-targets -- -D warnings
    CORPUS=tools/orbital-inspection-horizon-composition-bench/fixtures/horizon-composition-corpus-v0_1.csv
    DIGEST=$(sha256sum "$CORPUS" | cut -d' ' -f1)
    cargo run --quiet --locked --manifest-path tools/orbital-inspection-horizon-composition-bench/Cargo.toml --bin orbital-inspection-horizon-composition-bench -- "$CORPUS" "$GIT_COMMIT" "$DIGEST"
    python3 tools/orbital-inspection-horizon-composition-bench/tests/reference_oracle.py "$CORPUS" "$GIT_COMMIT" "$DIGEST"

The independent Python challenger uses checked i128 emulation, exact integer arithmetic and a separately implemented segment and horizon evaluator.

## Research parent

See [#7683](https://github.com/Luminous-Dynamics/symtropy/issues/7683) and [#7657](https://github.com/Luminous-Dynamics/symtropy/issues/7657). The implementation composes the single-segment conditional geometry kernel from [PR #7659](https://github.com/Luminous-Dynamics/symtropy/pull/7659).
