# Symtropy formal mathematics

This is a separate Lean 4 package for machine-checkable mathematical contracts
supporting the public Symtropy engine. It is not a Rust production dependency.

## Scope of the first slice

The module **SymtropyFormal.HousingReferences** encodes five elementary
analytical reference equations and proves geometry, positivity, and scaling
properties for them. The formulas and theorems are visible in source; this
package does not claim to verify the Rust implementation, a finite-element
solver, or a physical building.

The initial project pins Lean 4.35.0 and an immutable Mathlib commit. CI builds
the package and runs an axiom audit over the SymtropyFormal namespace. A proof
only counts as verified after that exact commit's workflow passes.

## Run locally

From the repository root:

    cd formalization/lean
    lake exe cache get
    lake build

## Trust boundary

A successful Lean build means the stated propositions typecheck against the
pinned Lean/Mathlib environment. It does not establish:
- that the proposition captures the intended engineering assumptions;
- that the Rust implementation computes the formal function;
- that binary floating-point arithmetic follows real-number identities exactly;
- that a numerical solver converges or models actual materials correctly;
- that measurements, boundary conditions, loads, or regulatory decisions are valid.

These require separate Rust conformance tests, floating-point error analysis,
solver verification, empirical validation, and qualified engineering review.
