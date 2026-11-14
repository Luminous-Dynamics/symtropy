# LOCO Viability Reserve V0

Dependency-free Rust reference utility for reporting axiswise uncertainty-ladder brackets from independently computed robust viability/recovery outcomes.

Research parent: [LOCO-VIABILITY-RESERVE-001 / #4130](https://github.com/Luminous-Dynamics/symtropy/issues/4130); implementation issue: [#4132](https://github.com/Luminous-Dynamics/symtropy/issues/4132).

## Purpose

This package consumes a sequence of oracle outcomes for one declared uncertainty axis. It reports the largest tested uncertainty level that remains viable and the smallest tested level that loses viability. It does not compute the viability kernel, construct the uncertainty sets, generate transitions or validate the physical model.

The uncertainty level is an integer expressed in a caller-declared unit (for example, milli-coefficient of friction or millinewtons). No implicit unit conversion, floating-point interpolation or inferred physical threshold is performed.

## Nested-family requirement

A formal threshold bracket is produced only if the caller declares that the uncertainty family is nested and the model, safe set, action semantics and oracle assumptions remain fixed.

Under that contract, increasing uncertainty should only add admissible successor outcomes. A viable -> nonviable transition is expected; a later nonviable -> viable rebound is a monotonicity violation. The utility preserves the original samples, reports all rebound indices and suppresses the formal bracket. It does not smooth, sort or repair a non-monotone result.

The caller's declaration is not independently verified. A false nested-family declaration invalidates the physical interpretation even if this utility reports monotonicity.

## Output

- Qualified bounded bracket: largest tested viable level and smallest tested nonviable level.
- Right-censored result: all tested levels remain viable.
- Left-censored result: all tested levels are nonviable.
- Unqualified result: no nested-family declaration or observed monotonicity violation.
- Separate bracket for guaranteed recovery to a declared target, distinct from indefinite controlled invariance.
- Full input identity and original sample sequence retained in the report.

No interpolation is emitted between samples. Coupled uncertainty axes require separate multidimensional sweeps/slices; one-axis margins are not added or multiplied into one scalar.

## Tests

Sixteen deterministic tests cover exact bracket construction, left/right censoring, rebound detection, non-nested input, strict level ordering, missing identity, separate recovery-target results, recovery-only monotonicity violation and deterministic replay.

## Commands

    cargo fmt --manifest-path research/loco-viability-reserve-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-viability-reserve-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-viability-reserve-v0/Cargo.toml --locked --all-targets -- -D warnings

A dedicated exact-head workflow is defined at .github/workflows/loco-viability-reserve.yml. No pass is claimed until formatting, tests and strict Clippy succeed at the exact PR head.

## Claim ceiling

This utility only brackets supplied finite outcome samples. It does not establish that uncertainty sets are physically nested, compute robust viability, validate the hybrid abstraction, verify the caller's oracle, or certify a robot. Those obligations remain with the sound abstraction, physical model and independent viability evidence process.

## Related work

- #4083 / #4105: set-valued hybrid abstraction contract and benchmark protocol.
- #4107: exact rational interval and event-order reference.
- #4106 / #4127: finite robust viability and recovery oracle.
- #4119 / #4129: certificate validity under sensor/actuator/timing changes.
- #4130: axiswise viability reserve definition.
