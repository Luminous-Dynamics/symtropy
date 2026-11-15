# Physics evidence composition reference v0.1

First executable slice of [PHYS-EVIDENCE-COMPOSITION-001 #5412](https://github.com/Luminous-Dynamics/symtropy/issues/5412).

## The composition problem

An independently valid receipt is not automatically applicable to another receipt. A mass-metric result, operator spectrum, KKT residual and nonlinear sensitivity bound can each pass in isolation while referring to different model revisions, fixture bytes, initial states, units, integrators, timesteps, solver profiles, coordinate frames, constraints or observable mappings.

This crate checks the binding and scope composition of already verified receipts. It does not repeat their mathematical or physical calculations.

## Claim tiers

- Physical-step equivalence requires mathematical symmetry qualification, coordinate covariance, operator spectrum, constraint/Delassus qualification, KKT residual qualification, nonlinear sensitivity evidence and a discrete-step receipt.
- Task equivalence requires all physical-step evidence plus a task-semantics receipt binding the candidate domain, objective, feasibility conditions, initial/boundary conditions, observables and answer interpretation.

The two outcomes are reported separately. A passing physical step does not imply that the scientific task is invariant.

## Fail-closed rules

- All evidence must bind to the same immutable physical-problem binding.
- The task-semantics digest is optional for physical-step claims and required for task-equivalence claims.
- Mathematical symmetry evidence is explicitly source-frame bound; physical operator/constraint/KKT/sensitivity/step evidence is target-frame bound; task semantics are representation-independent.
- The coordinate-covariance receipt must bind the exact source/target frame pair, both metric digests and the basis-transform digest.
- Each required receipt must match an explicit expected verifier ID, version and checker commit SHA.
- The nonlinear receipt's declared validity radius must cover the requested perturbation radius.
- The mathematical evidence class must meet the request's minimum class.
- Missing or inconclusive evidence cannot become PASS.
- A failed or mismatched receipt rejects composition.
- Duplicate evidence kinds reject composition rather than selecting the most convenient copy.
- Scientific-search hard-pruning authority is always false in v0.1.

## Important trust boundary

This reference validates digest shape and equality of declared identities. It does not compute SHA-256 over external artifacts, fetch commits, verify signatures, or re-run the owning checkers. The supplied verification status and verifier identities are treated as upstream claims that must already have been independently established. A production receiver must resolve exact bytes and recompute artifact digests before accepting this composition receipt.

The composer also does not convert numerical Lie evidence into an exact group proof, does not infer global group topology from a generator basis, and does not infer full-task invariance from physical-step covariance.

## Reproduction

    cargo fmt --manifest-path research/physics-evidence-composition-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/physics-evidence-composition-001/Cargo.toml
    cargo clippy --locked --manifest-path research/physics-evidence-composition-001/Cargo.toml --all-targets -- -D warnings

## v0.1 claim ceiling

The crate is a bounded binding/scope composer over typed evidence records. It is not a theorem prover, solver, artifact-fetching service, cryptographic verifier, continuous symmetry checker, or scientific-search pruning engine.
