# Planar Truss Mutation Qualification Protocol v0.1

**Tracking:** [PHYS-TRUSS-001 #2361](https://github.com/Luminous-Dynamics/symtropy/issues/2361)  
**Implementation:** [planar truss reference solver PR #2363](https://github.com/Luminous-Dynamics/symtropy/pull/2363)  
**Purpose:** test sensitivity of the truss reference tests to named numerical/formula defects. This is not physical validation.

## Metamorphic and known-answer roles

The symmetric two-bar V-truss is checked against an independently derived closed-form solution for apex displacement, signed axial forces, support reactions, and normalized equilibrium residuals. The solver also checks input-order invariance, load and stiffness scaling, mechanisms/singular matrices, invalid topology/geometry, overflow, and the invariance of the pivot tolerance under a joint scaling of stiffness and loads.

These tests give different kinds of evidence:
- Known-answer checks pin an independently derived result for one declared model.
- Metamorphic checks constrain how outputs must change under specified transformations.
- Negative controls verify that underconstrained or malformed inputs do not yield a PASS.
- Mutation checks establish that selected tests detect selected classes of implementation defects.

No one layer establishes that the 2D axial-bar model represents a real structure.

## Named mutation set

The CI script `research/physical-transformation-reference/scripts/verify_planar_truss_mutations.sh` first runs the unmutated `planar_truss::tests` control. It then copies the crate manifest, lockfile, source files and frozen input fixture into fresh temporary directories, applies one explicit mutation per copy, and runs the intended test by exact name.

| Mutation ID | Deliberate defect | Required detecting test |
|---|---|---|
| `PHYS-TRUSS-MUT-STIFFNESS-AREA-001` | Omit area from (EA/L) during stiffness assembly | `load_and_stiffness_scaling_obey_metamorphic_relations` |
| `PHYS-TRUSS-MUT-STIFFNESS-LENGTH-001` | Divide assembled axial stiffness by (L^2) instead of (L) | `fixed_seed_parameter_sweep_matches_symmetric_truss_closed_form` |
| `PHYS-TRUSS-MUT-MEMBER-FORCE-AREA-001` | Omit area from recovered axial force (N=EAepsilon) | `symmetric_two_bar_truss_matches_independent_closed_form_oracle` |
| `PHYS-TRUSS-MUT-REACTION-SIGN-001` | Reverse the sign of vertical support-reaction recovery | `symmetric_two_bar_truss_matches_independent_closed_form_oracle` |
| `PHYS-TRUSS-MUT-PIVOT-SCALE-001` | Reintroduce an absolute `max(1.0)` floor in the pivot tolerance | `uniform_stiffness_and_load_rescaling_does_not_change_displacement` |

A mutant counts as detected only when its named test appears as FAILED. A compile failure, unrelated Cargo failure, failed source transform, or surviving mutant invalidates the audit. After the isolated runs, the script re-checks the source hash. CI then checks the entire production source remains unchanged, re-runs formatting, the full crate suite and Clippy, and uploads the JSON mutation receipt with the reference receipt.

## Receipt fields and interpretation

The receipt binds the exact 40-character source commit, SHA-256 of `planar_truss.rs`, Rust/Cargo versions, control outcome, stable mutation IDs, intended test names, per-mutant disposition and counts. The summary is limited to these five deliberately selected mutations. The fifth mutation targets the new 128-case closed-form parameter sweep, whose bounds, fixed seed, and scalar-check count are published separately in `planar-truss-sweep-receipt-v0.1` by CI after the full suite passes. It must not be represented as a global mutation score or a physical-accuracy metric.

## Claim boundary

The solver handles only static small-displacement linear-elastic 2D pin-jointed axial bars under its declared input contract. It excludes member bending, buckling, geometric/material nonlinearity, contact, fracture, dynamics, and structural code/safety checks. Passing this audit shows only software-test sensitivity for the five named defects; it neither reproduces a proprietary catalog case nor establishes sim-to-real validity.
