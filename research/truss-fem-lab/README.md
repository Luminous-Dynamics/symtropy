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

The damage module also provides a bounded two-member-removal campaign. It selects evenly spaced ranks from the complete lexicographic pair space, caps execution at 1,024 scenarios, and reports total pairs, attempted scenarios and every omitted pair. The deterministic sample is a screening campaign—not exhaustive analysis and not a probability estimate. For any safety-relevant use, identify physically justified failure combinations and perform mission-specific progressive-damage analysis.

## Member stability screen

The `stability` module adds per-member:
- axial yield utilization using the supplied yield strength;
- weak-axis radius of gyration and effective slenderness;
- ideal Euler critical load using an explicit effective-length factor;
- elastic Euler utilization for compressive members only when the Euler stress is below the supplied yield strength;
- explicit classification for the nominal elastic-Euler range versus yield/inelastic regime.

Each member receives declared section properties (E, A, weak-axis I, yield strength and K factor). The CLI uses an illustrative uniform section solely to exercise the screen.

This is not a design-code check. It excludes global frame buckling, beam-column interaction, bending/shear/torsion, joint restraint/compliance, initial crookedness, residual stress, plasticity, local buckling, load interaction, fatigue, fracture and safety factors. The K factor and section properties must be substantiated for any real design.

## Joint-aware axial compliance

The interfaces module provides a second solver path with one axial translational spring at each end of each member. Its element compliance is:

`L/(E*A) + 1/k_from + 1/k_to`

It reports member-bar extension/stress separately from end-connector slip and partitions strain energy into bar and joint contributions. Tests verify:
- very stiff end springs approach the ideal rigid-joint result;
- finite connector stiffness increases total extension while bar stress remains tied to bar strain;
- tension/compression sign consistency;
- invalid stiffness and property-count mismatches fail closed.

This is only axial spring compliance. It does not model rotational stiffness, preload, friction, bearing/contact, clearance take-up, backlash, bolt slip, nonlinear connector behavior, multi-axis coupling or joint fatigue.

## Mass-normalized candidate benchmark

The benchmark module derives one uniform solid-round member section from a declared member-only mass budget and material density. It then runs the same load/support conditions through:
- linear-elastic axial truss response;
- yield and ideal Euler member screening;
- exhaustive single-member removal;
- bounded two-member removal.

The receipt reports computed member mass, area/inertia, maximum displacement/stress, equilibrium residual, yield/Euler utilization, and damage-screen counts.

**Comparability rule:** callers must use the same structural envelope, supports, load cases, material model, density, mass accounting boundary, evaluation budgets and numerical tolerances for all candidate designs. The current mass budget excludes joints, nodes, fasteners, services, manufacturing waste, inspection, assembly and lifecycle costs. It is not a whole-system mass comparison.

## Matched topology fixture

The benchmark module also builds a reproducible pair:
- a regular tetrahedral frame with six outer members;
- the same external tetrahedron with one internal hub and four additional spokes.

Both candidates share the same external envelope, outer support coordinates and applied load. The evaluator can assign the same total straight-member mass budget and derive a different uniform circular-member area for each candidate. This is a small benchmark harness fixture, not evidence that the interior-hub design is better. Its purpose is to ensure that added connectivity is compared under equal member mass rather than equal section area.

The comparison still excludes hub/connector/fastener mass and does not represent a full modular structure lifecycle.

The two-member removal campaign has two limits: a hard 1,024-scenario ceiling and an approximate dense-solve work budget proportional to free_dofs^3. The applied scenario count is the minimum of the request, available pairs, the hard ceiling and the work budget. The receipt reports requested/applied counts and omitted combinations. This keeps the search bounded as model dimension grows; the work budget is an execution guard, not a physical metric.

## Independent global verification audit

The verification module separately checks:
- total applied forces plus constrained support reactions;
- total applied moments plus constrained support-reaction moments about the global origin;
- strain energy against one-half external work for the zero-prescribed-displacement linear static problem;
- the existing maximum free-DOF residual.

This is an equilibrium/energy consistency audit of the numerical result. It does not validate the structural idealization, boundary conditions, omitted physics or material/connector properties. It assumes nodal forces and reactions act at node coordinates and does not include applied nodal couples.

## Reproducible benchmark input identity

Each mass-normalized receipt includes an FNV-1a digest over the exact model geometry, member connectivity, support flags, nodal force vectors and benchmark specification. The field is explicitly named non-security: it supports same-input regression/replay identity but is not a cryptographic signature or tamper-evident evidence seal. For externally auditable evidence, export the complete input and output files and hash them with a modern cryptographic digest in the evidence pipeline.

## 3D beam element kernel (pre-frame-solver)

The beam_element module now provides a 12-DOF 3D Euler-Bernoulli element with:
- axial EA/L stiffness;
- torsional GJ/L stiffness;
- bending about both local principal axes;
- right-handed local-frame construction from an orientation hint;
- local-to-global transformation and a global element stiffness matrix.

Its tests include matrix symmetry/finite checks, a cantilever tip deflection and rotation against the closed-form Euler-Bernoulli solution, local-frame orthonormality, transformation orthogonality/recovery, and rejection of parallel/ill-conditioned orientation hints.

This is only an element kernel. It does not assemble a frame, solve global displacements, include shear deformation, geometric stiffness, global buckling eigenvalues, imperfections, nonlinear behavior or joint rotational springs. Euler-Bernoulli assumptions are inappropriate for sufficiently deep/short members; a Timoshenko element is a later requirement where shear deformation matters.

## Frozen comparison protocol

The benchmark campaign rules are versioned at research/ffmfe-benchmark/PROTOCOL_V1.md. The protocol freezes geometry, supports, loads, materials, solver fidelity, mass accounting, search budget, hold-out families and reporting metrics before candidate selection. B0 matched-topology reproduction is tracked in issue #5808.

The beam-element fixtures additionally check both principal bending planes and all six rigid-body modes (zero element strain energy). These fixtures are authored but remain unexecuted until exact-head Rust CI is available.

## 3D beam/frame assembly solver (research reference)

The new frame_analysis module assembles the 12-DOF Euler-Bernoulli beam members into a global six-DOF-per-node stiffness system. It supports:
- translational and rotational nodal DOFs;
- nodal forces and couples;
- prescribed zero-displacement/rotation supports;
- global static stiffness assembly and dense solution;
- member-local end action recovery and strain-energy calculation;
- support reaction, global force/moment and strain-energy / half-external-work audit.

The reference treats members meeting at the same node as rigidly connected in all six DOFs. It assumes small-displacement linear Euler-Bernoulli beam behavior. The model does not yet include Timoshenko shear, releases, flexible six-DOF connector matrices, distributed loads, geometric stiffness, global eigenvalue buckling, initial imperfections, contact, nonlinear/post-buckling, dynamics or fatigue/fracture. This is not a production FE solver or design-code check.

Authored fixtures cover closed-form cantilever response in both principal bending planes, global force/moment/energy closure, a two-member elbow frame with a shared six-DOF joint, and singular detection for a completely unsupported frame. These tests remain unexecuted pending current-head Rust CI.

The frame solver's shared-node model imposes rigid continuity of all six DOFs at a common node. It does not imply that a real modular connector has that stiffness. Use the separate connector-compliance prototype or a future six-DOF connector matrix for explicit joint flexibility; calibrate those properties from coupons before treating them as physical.
