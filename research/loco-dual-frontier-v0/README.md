# LOCO Dual Frontier V0

Dependency-free Rust reference for separating physical recoverability assurance from empirical learning-frontier decisions.

Research parent: [LOCO-DUAL-FRONTIER-001 / #4122](https://github.com/Luminous-Dynamics/symtropy/issues/4122).

## The distinction

Two ideas are both important and neither substitutes for the other.

- **Physical frontier:** a model-conditional set-based claim about whether all declared physical outcomes preserve viability/recovery, plus a valid evidence envelope at action time.
- **Learning frontier:** an empirical finite-horizon signal about whether a curriculum expansion produces useful learning, preserves the previous policy and does not collapse into mostly failed rollouts.

HORIZON (Bai et al., arXiv:2606.05143v1, June 2026) already develops recoverability-governed physical-domain curriculum expansion with checkpoint preservation and rollback. This package does not claim that concept as new. HORIZON explicitly describes its gate as a finite-horizon training signal, not an exact viability kernel. The Symtropy hypothesis is to keep that empirical frontier separate from the independently supplied physical certificate, and make their relationship auditable.

## Implemented

- Typed simulation versus hardware execution target.
- Independent physical-frontier status: Certified, Unknown, OutOfEnvelope.
- Independent assurance status: Valid, Unknown, Invalid.
- HORIZON-style empirical learning gate with fixed-point fields:
  - episode fraction;
  - tracking error;
  - checkpoint tracking ratio;
  - checkpoint reward ratio.
- Default empirical thresholds mirror HORIZON's reported values: 0.85 episode fraction, tracking error 0.5 in the caller-declared unit, checkpoint tracking ratio 1.05, checkpoint reward ratio 0.95.
- Hardware permission requires all three conditions: certified physical frontier with a non-empty certificate reference, valid assurance with a non-empty receipt reference, and a passing empirical learning gate.
- Simulation-only curriculum may explore Unknown or OutOfEnvelope physical regions, but the result remains explicitly labelled simulation-only.
- Unknown learning evidence defers; a known learning failure rejects.
- Deterministic finding order and a complete dual-frontier decision record.
- Nineteen deterministic tests covering gate precedence, evidence missingness, exact thresholds, checkpoint preservation, simulation-only exploration and fail-closed proposal identity.

## Important semantics

A passing learning gate cannot create physical viability. A valid physical certificate cannot establish that a domain is useful for training. A simulation-only commit is not permission to run the same domain on hardware.

This V0 uses a caller-supplied physical status and evidence references. It does not compute the viability kernel, validate the underlying certificate, run a curriculum, update a policy, estimate gradients or calculate statistical confidence intervals. The supplied learning thresholds are operational proxies, not a universal learning-success definition.

## Commands

    cargo fmt --manifest-path research/loco-dual-frontier-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-dual-frontier-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-dual-frontier-v0/Cargo.toml --locked --all-targets -- -D warnings

A dedicated exact-head workflow is defined at .github/workflows/loco-dual-frontier.yml. No verification pass is claimed until formatting, all tests and strict Clippy succeed on the exact PR head.

## Qualification ladder

1. Qualify this finite decision matrix against independent known-answer fixtures.
2. Reproduce the HORIZON length/tracking/checkpoint gate with matched thresholds and frozen seeds.
3. Compare learning-only, physical-only and dual-frontier decisions under the same curriculum proposals.
4. Add policy-checkpoint digests, randomization-domain schema, experiment receipts and independent outcome evaluator.
5. Connect the physical status to #4083/#4106 and certificate validity to #4119; never accept caller labels as a substitute for those source results.
6. Report physical coverage, false-safe count, learning progress, rollback, compute/sample cost and held-out OOD separately.

## Claim ceiling

Passing tests qualify only this deterministic finite decision function. The HORIZON source is an arXiv preprint and its reported results are specific to its simulator, quadruped task and Go2 hardware interventions. This package does not demonstrate better policy learning, physical safety, transfer or performance than HORIZON or other SOTA.

## References

- Bai et al. (2026), HORIZON: Recoverability-Governed Curriculum for Physical-Domain Scaling: https://arxiv.org/abs/2606.05143
- Bai et al. (2026), full HTML: https://arxiv.org/html/2606.05143v1
- Madabushi et al. (2026), hybrid invariant sets for legged robots: https://arxiv.org/abs/2604.05108
- Jia et al. (2026), contact sensing survey: https://doi.org/10.1007/s10462-026-11644-0
