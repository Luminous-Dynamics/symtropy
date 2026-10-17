# Symtropy formal mathematics

This is a separate Lean 4 package for machine-checkable mathematical contracts
supporting the public Symtropy engine. It is not a Rust production dependency.

## Scope of the first slice

- **HousingReferences.lean** defines five primary closed-form response equations plus supporting equations for section area, extreme-fiber distance, stress, bending moment, and reactions. Exact-rational fixture theorems cover all 28 expected output fields in the shared five-vector corpus; the additional named theorem declarations are not verified until Lean CI passes.
- **Dimensions.lean** defines seven-SI-base dimension vectors, proves twelve
  dimensional-homogeneity identities and five dimensionless-group invariants,
  and includes a negative control that rejects a malformed cantilever expression.
  The catalogue maps every declared SI unit to an exponent vector and a matching
  Lean theorem anchor.
- **contracts.json** maps stable contract IDs to assumptions, input/output SI
  units, theorem IDs, candidate housing fixture links, dimension status, and
  negative controls. It also maps every expected output field in the pinned
  Rust contract-vector corpus to a named Lean theorem.
- **Shared contract vectors** are authored on open housing-evidence PR #1653.
  Lean CI fetches that candidate PR, verifies pinned commit/blob identities,
  and reconciles each vector ID, contract ID, output key, and theorem name.
  This is traceability and exact-contract coverage, not an automated proof that
  the Rust implementation refines the real-valued Lean functions.

The current numerical definitions accept untyped real numbers. The dimensional
proofs validate equations in a separate exponent model; they do not yet make
invalid Rust calls unrepresentable, enforce units in the runtime, or prove the
Rust implementation computes these functions.

The project pins Lean 4.35.0 and an immutable Mathlib commit. CI validates
the theorem-to-contract mapping, builds the package, audits the SymtropyFormal
namespace for unapproved axioms/placeholders, and compiler-checks every mapped
theorem name from a generated temporary Lean module. No theorem counts as
verified until that exact revision passes.

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
