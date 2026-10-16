# Symtropy formal mathematics

This is a separate Lean 4 package for machine-checkable mathematical contracts
supporting the public Symtropy engine. It is not a Rust production dependency.

## Scope of the first slice

The module **SymtropyFormal.HousingReferences** defines five elementary closed-form
reference equations and contains twenty theorem declarations covering
positivity and exact geometric/load scaling. The versioned
**contracts.json** catalog maps each contract to its equation, assumptions,
SI-normalized input/output units, theorem identifiers, and candidate fixture links. Six exact rational known-answer theorems mirror the declared housing examples on PR #1653, but fixture ingestion and Rust conformance are not wired yet.

The initial project pins Lean 4.35.0 and an immutable Mathlib commit. CI builds
the package, validates theorem-to-contract mapping, and audits the
SymtropyFormal namespace for unapproved axioms/placeholders. A proof only
counts as verified after that exact commit's workflow passes.

## Run locally

From the repository root:

    cd formalization/lean
    lake exe cache get
    lake build

## Trust boundary

A successful Lean build means the stated propositions typecheck against the
pinned Lean/Mathlib environment. It does not establish:
- that a proposition fully captures the intended engineering assumptions;
- that the Rust implementation computes the formal function;
- that binary floating-point arithmetic follows real-number identities exactly;
- that a numerical solver converges or models actual materials correctly;
- that measurements, boundary conditions, loads, or regulatory decisions are valid.

These require separate Rust conformance tests, floating-point error analysis,
solver verification, empirical validation, and qualified engineering review.
