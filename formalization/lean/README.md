# Symtropy formal mathematics

This is a separate Lean 4 package for machine-checkable mathematical contracts
supporting the public Symtropy engine. It is not a Rust production dependency.

## Scope of the first slice

- **HousingReferences.lean** defines five elementary closed-form reference
  equations and theorem declarations for positivity, exact scaling, and exact
  rational known-answer values.
- **Dimensions.lean** defines seven-SI-base dimension vectors, proves eight
  dimensional-homogeneity identities and five dimensionless-group invariants,
  and includes a negative control that rejects a malformed cantilever expression.
  The catalogue maps every declared SI unit to an exponent vector and a matching
  Lean theorem anchor.
- **contracts.json** maps stable contract IDs to assumptions, input/output SI
  units, theorem IDs, candidate housing fixture links, dimension status, and
  negative controls.

The current numerical definitions accept untyped real numbers. The dimensional
proofs validate equations in a separate exponent model; they do not yet make
invalid Rust calls unrepresentable, enforce units in the runtime, or prove the
Rust implementation computes these functions.

The project pins Lean 4.35.0 and an immutable Mathlib commit. CI validates
the theorem-to-contract mapping, builds the package, and audits the
SymtropyFormal namespace for unapproved axioms/placeholders. No theorem counts
as verified until that exact revision passes.

## Run locally

From the repository root:

    cd formalization/lean
    lake exe cache get
    lake build

## Research basis

The approach is informed by a published Lean formalization of dimensional
analysis and the existing ATOMSLab/LeanDimensionalAnalysis project. That
upstream package pins an older Lean release, so this prototype does not vendor
its code without a compatibility review. Rust's `dimensioned` crate is another
potential way to make unit errors unrepresentable in implementation types; it
should be evaluated against Symtropy's existing API before introducing a
production dependency.

## Trust boundary

A successful Lean build only establishes that these exact propositions typecheck
in the pinned Lean/Mathlib environment. It does not establish:
- that the proposition captures all of the intended engineering assumptions;
- that Rust values are bound to the same dimensions or the formulas are implemented correctly;
- that binary floating-point arithmetic follows real-number identities exactly;
- that a numerical solver converges or models actual materials correctly;
- that loads, boundary conditions, measurements, building-code provisions, or safety decisions are valid.

Those require separate implementation-conformance tests, numerical analysis,
solver verification, empirical validation, and qualified engineering review.
