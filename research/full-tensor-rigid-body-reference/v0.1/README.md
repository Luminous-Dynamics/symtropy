# Superseded full-tensor reference scratch branch

This branch-local prototype has been retired to avoid duplicating the more complete reference and validation work already present in:

- [PR #4698 — anisotropic angular dynamics reference](https://github.com/Luminous-Dynamics/symtropy/pull/4698)
- [PR #6418 — compound mass properties](https://github.com/Luminous-Dynamics/symtropy/pull/6418)
- [PR #6445 — independent contact-Jacobian oracle](https://github.com/Luminous-Dynamics/symtropy/pull/6445)
- [Issue #6679 — full-inertia/SO(3) research](https://github.com/Luminous-Dynamics/symtropy/issues/6679)

Do not merge this scratch branch or use it as a qualification artifact. The prototype was not validated by an exact-head Rust test/Clippy run and is not the canonical implementation. Continue from the existing reference stack, then integrate only after the production kinematics/contact convention and mass-property contracts are qualified.
