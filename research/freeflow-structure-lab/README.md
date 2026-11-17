# Symtropy Free-Flow Modular Structure Lab

A standalone, standard-library-only Rust geometry/topology research crate for Free-Flow Modular Fractal Structures (FFMFS).

It currently contains two geometry grammars:

1. **Recursive branch/truss motif:** paired branches with sibling cross-braces. This is useful for studying free-form morphology, but graph connectivity and cycle rank do not establish 3D rigidity; one-point-attached substructures can retain hinge mechanisms.
2. **Face-attached tetrahedral bays:** a regular tetrahedral root with child tetrahedra attached across a complete, non-collinear triangular face. This gives a more defensible ideal pin-jointed kinematic construction. It is still not a strength or safety proof.

The face-attached assembly includes:
- stable node, member and cell identifiers;
- a declared three-point triangular attachment interface;
- indexed pin-end member interface;
- bounded recursion and node/port budgets;
- adaptive attachment pruning: a candidate face is not expanded if any shared hub would exceed its connector-port budget; blocked faces are counted;
- finite geometry, cell volume and attachment face area checks;
- member length, graph connectivity, duplicate-edge and cycle-rank audits;
- minimum angular separation between ports at a shared node;
- pin/bore worst-case clearance analysis;
- worst-case and root-sum-square tolerance stack estimates;
- a separate member-removal campaign in the companion truss solver to expose kinematic mechanisms and residual capacity;
- assembly parent/child ordering;
- deterministic same-build, non-cryptographic regression digest.

## Run

```bash
cargo test --manifest-path research/freeflow-structure-lab/Cargo.toml
cargo clippy --manifest-path research/freeflow-structure-lab/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path research/freeflow-structure-lab/Cargo.toml --bin freeflow-structure
```

## Companion truss analysis

The separate `research/truss-fem-lab` crate consumes this geometry and provides a narrow linear-elastic pin-jointed truss reference solver, with analytical single-bar tests and a singular-mechanism negative control. It is not part of this geometry crate.

## Claim boundary

This crate checks geometry and graph topology only. It does not calculate stress, strain, stiffness, joint compliance, buckling, fatigue, vibration, fracture, manufacturing tolerances, material defects, thermal behavior, human factors or flight qualification.

The tetrahedral grammar is port-budget-aware: it prunes an attachment before committing geometry if one of the three shared hubs would exceed its configured port count. The generated assembly can therefore contain fewer cells than the full binary recursion upper bound; blocked attachment faces are counted. This makes the connector limit a design constraint rather than a post-hoc warning.

The ideal kinematic observation is narrow: a regular tetrahedron has six pin-jointed bars, and each child cell shares three non-collinear vertices with an already assembled rigid complex. This removes the single-point attachment hinge in the ideal geometric construction. Real connectors, clearance, compliance, member bending, buckling, loads, imperfections and fatigue remain unmodeled.

Cycle rank, graph connectivity, port count and visual appearance are not structural strength or redundancy proof. The generated geometry is not yet shown to have a statistical fractal scaling regime.

The next stage is independent truss/beam analysis, realistic connector models, matched conventional-truss/spaceframe/TPMS/graded-lattice baselines, and physical coupon tests. Do not connect this output to safety-critical design decisions.
