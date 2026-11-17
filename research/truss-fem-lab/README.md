# Symtropy Truss FEM Reference Lab

A small, standard-library-only reference solver for 3D linear-elastic pin-jointed trusses. It consumes nodes and bars from the free-flow structure geometry crate.

## Scope

- Uniform isotropic Young's modulus and circular/nominal cross-section area represented by one constant area.
- Axial-only two-node truss members.
- Small-displacement linear elasticity.
- Zero-displacement supports and nodal point loads.
- Dense global stiffness assembly and partial-pivot Gaussian elimination.
- Displacements, member axial forces/stresses, support reactions, strain energy and free-DOF equilibrium residual.
- Singular-stiffness detection for kinematic mechanisms and insufficient support conditions.

## Run

```bash
cargo test --manifest-path research/truss-fem-lab/Cargo.toml
cargo clippy --manifest-path research/truss-fem-lab/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path research/truss-fem-lab/Cargo.toml --bin truss-fem
```

## Important limitations

This is a verification/reference model, not a production finite-element solver. It does not include beam bending, shear, torsion, joint compliance, contact, geometric nonlinearity, buckling, fracture, fatigue, plasticity, thermal loads, material anisotropy, AM defects, stress concentrations or code factors of safety.

A solved model is not structural qualification. Results are valid only under the declared ideal pin-jointed, linear-elastic, small-displacement assumptions. Compare analytical fixtures and an independent solver before using results beyond research.

The key negative control is that a connected, cross-braced branching graph may still be kinematically unstable if substructures attach at one point. The face-attached tetrahedral grammar is tested separately because each child cell shares three non-collinear vertices with the existing rigid complex. This is still an ideal joint model.

## Damage sensitivity

The companion damage module runs the intact model and then removes every member once under the same boundary/load conditions. It classifies each scenario as stable, a singular kinematic mechanism, or another typed failure and records an envelope for stable responses. The CLI reports counts and the fraction of single-member removals that remain solvable.

That fraction is a deterministic stress-test scenario statistic, not a physical failure probability or reliability estimate. The campaign does not include fatigue, fracture, material scatter, joint failure distributions, progressive damage, dynamic impact or repair.

The next extension should be a bounded two-member cut-set search and explicit joint/connector removal, with the search budget and every omitted scenario reported.
