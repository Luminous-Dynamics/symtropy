# Magpylib Material Response 0.15.0 — solver qualification addendum

**Release date:** 25 November 2026  
**Source commit:** `492b37459a47f4220d7e44380668c2d9ab720784`  
**Source:** https://github.com/magpylib/magpylib-material-response/commit/492b37459a47f4220d7e44380668c2d9ab720784

## Verified upstream changes

The 0.15.0 source changelog and release commit add the following capabilities and behaviour changes:

- Browser/WebAssembly field kernels for face-charge sheets and Magpylib cuboid, cylinder, circle, straight-wire, sphere and dipole sources.
- The release describes the kernels as checked in Rust and Python against shared 50-digit reference files.
- The Python face-charge kernel supports JAX and PyTorch, including `jax.jit` and gradients.
- NumPy minimum version increases to 2.1.
- `H_ext` and deprecated aliases are removed; this is a breaking API change.
- A ChargeSheet face with zero area is dropped rather than propagating NaN through the field.
- ChargeSheet setters preserve originally supplied faces so a previously degenerate face can become valid again when vertices change.

Source claims above are upstream release claims. They are not independent reproductions by Symtropy.

## Implications for the Symtropy tournament

Field-value agreement is no longer a sufficient comparison for inverse design. The tournament should qualify field values, derivatives and numerical stability separately.

1. Pin the exact wheel/sdist artifact, SHA-256, provenance attestation, source commit, Python, NumPy and backend versions.
2. Reuse published 50-digit field fixtures where licensing and fixture provenance permit; independently regenerate analytic cuboid, cylinder and dipole known answers.
3. Compare field kernels for cuboid/cylinder/dipole and face-charge sheet separately. They are different formulations and do not imply mutual demagnetization or nonlinear B-H support.
4. For the JAX/PyTorch face-charge kernel, compare first derivatives with analytic derivatives where available and high-precision, step-sensitivity-controlled finite differences in smooth regions.
5. Test derivative covariance under rotation, translation, scaling and source superposition. Test mesh and panel refinement independently.
6. Treat near-axis, far-field, close-surface and near-degenerate face cases as dedicated conditioning strata. Value accuracy and gradient accuracy may differ materially.
7. Compare eager, JIT, PyTorch and WASM at matched field and derivative tolerances. Runtime alone is not a valid score.
8. Record unsupported/non-smooth regions explicitly; do not expose gradients to the inverse optimizer when the model or derivative applicability is unknown.

## Relationship to follow-on work

- [MAG-SOLVER-ADJOINT-001 #7536](https://github.com/Luminous-Dynamics/symtropy/issues/7536) defines the value-and-gradient qualification benchmark.
- [Solver qualification PR #7445](https://github.com/Luminous-Dynamics/symtropy/pull/7445) remains the evidence-contract owner.
- The Magnetic Function Compiler should use differentiable kernels for candidate search, then independently verify finalists with higher-fidelity solvers and physical metrology.

## Claim boundary

This addendum does not assert Symtropy field parity, derivative parity, speed superiority, inverse-design superiority or component qualification. Magpylib Material Response is experimental; preserve exact version and breaking API changes in any result.