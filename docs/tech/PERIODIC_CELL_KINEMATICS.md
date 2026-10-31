# Time-Dependent Periodic Cell Kinematics

**Status:** standalone kinematics primitive; no particle integrator or PhysicsWorld integration.  
**Tracking:** [PHYS-TRICLINIC-004 #3037](https://github.com/Luminous-Dynamics/symtropy/issues/3037).  
**Dependency:** [PeriodicLattice geometry PR #3010](https://github.com/Luminous-Dynamics/symtropy/pull/3010).

## 1. Separate the coordinate frame from the material state

At one physical sample time, describe the cell by origin o, origin velocity o_dot, basis B, basis rate B_dot, and a periodic-axis mask. Basis columns are the lattice vectors. The mask is expressed in fractional basis coordinates.

For material/fractional position s and its physical-time derivative s_dot:

```text
x = o + B s
v = o_dot + B_dot s + B s_dot
```

The affine velocity term B_dot s is mandatory. Dropping it changes the Cartesian velocity when the cell is sheared, compressed, expanded, or otherwise deformed.

Inverse conversion:

```text
s     = inverse(B) (x - o)
s_dot = inverse(B) (v - o_dot - B_dot s)
```

The first implementation validates full-rank D=2/3 bases, positive orientation, finite frame data, a Frobenius condition proxy <= 1e8, and a scale-independent inverse reconstruction residual <= 1e-8. These are explicit numeric-envelope policy values, not a formal floating-point proof.

## 2. Two operations that must not be conflated

### Physical affine deformation

`affine_deform_to` holds material s and s_dot fixed while expressing them in another same-sample-time cell frame. It reports:

- deformation gradient F = B_new inverse(B_old);
- volume ratio det(B_new)/det(B_old);
- resulting Cartesian position and velocity;
- affine position and velocity deltas.

This is a kinematic mapping only. It does not integrate over time, compute barostat work, conserve energy, or determine how forces should change.

### Pure integer-unimodular basis rebase

`basis_rebase_to` represents the same physical lattice using B_new = B_old U, where U is an exactly verified integer matrix with determinant +1. It also requires Bdot_new = Bdot_old U and unchanged origin/origin velocity.

Fractional state is transformed by the exact integer inverse:

```text
s_new     = inverse(U) s_old
s_dot_new = inverse(U) s_dot_old
```

This preserves Cartesian position and physical velocity. U and inverse(U) are checked with integer arithmetic, and integer coefficients outside the exact f64 conversion envelope are rejected. U may not mix active periodic basis coordinates with inactive nonperiodic coordinates.

The current version intentionally rejects origin shifts and origin-velocity changes during a basis-only rebase. Those require a separate affine chart-offset contract.

## 3. Explicit non-goals

- No integration of s or s_dot through time.
- No NPT/barostat or stress/virial/work calculation.
- No force or energy conservation claim under physical deformation.
- No periodic boundary wrapping of body state.
- No collision, CCD, contact-cache or PhysicsWorld integration.
- No time-dependent nonlinear lattice or rank-changing periodic subspace.

Different physical sample times are rejected by the same-time mapping API. An integrator must explicitly evolve material state between times.

## 4. Correctness corpus

- Identity conversion and Cartesian/fractional round trip.
- Nonzero B_dot s contribution to velocity.
- Pure deformation with analytic F and determinant/volume ratio.
- Same-time old/new/old deformation round trip.
- Pure integer basis rebase preserving Cartesian x and v.
- 65 deterministic 2D integer shears (including negative shear) with invariant physical state.
- 3D upper-triangular unimodular basis transform.
- Non-unimodular transform rejection and active/inactive mask-mixing rejection.
- Sample-time/mask mismatch, singular, left-handed, ill-conditioned and non-finite frame rejection.
- Future corpus: general random well-conditioned 2D/3D bases, orthogonal rotation covariance, unit scaling, and adversarial near-condition-envelope cells.

Every tolerance is explicit. Preserve failures with basis, basis rate, mask, integer transform, state and exact seed.

## 5. Benchmark

Criterion target: `periodic_cell_kinematics`. It measures frame construction, fractional-to-Cartesian conversion, Cartesian-to-fractional conversion, pure basis rebase, physical affine deformation, and a 4,096-state batch transform in 2D/3D. It reports timing only; correctness residuals remain separate hard gates.

Run:

```bash
cargo bench -p symtropy-physics --bench periodic_cell_kinematics
```

Record exact commit SHA, Cargo.lock digest, Rust/Cargo version, CPU, build profile, raw Criterion output, and frame condition proxy. Compare at equal error and refusal envelope; do not report transform throughput alone.

## 6. External context and comparison

GROMACS and LAMMPS demonstrate that triclinic cells, tilt and equivalent box-vector representations are established simulation-engine concerns. GROMACS canonicalizes triclinic box representations through lattice-vector operations, while LAMMPS documents box flipping to keep tilt factors in efficient ranges. This work is not a replacement for those molecular-dynamics engines; its narrower contribution is an explicit, testable kinematic contract separating a pure basis representation change from a physical affine deformation.

References:

- GROMACS periodic boundary conditions and triclinic box representation: https://manual.gromacs.org/current/reference-manual/algorithms/periodic-boundary-conditions.html
- LAMMPS triclinic boxes, tilt factors and box flips: https://docs.lammps.org/Howto_triclinic.html
- LAMMPS boundary conditions and periodic shifts in tilted boxes: https://docs.lammps.org/boundary.html

## 7. Integration gates

- [ ] Static PeriodicLattice geometry and closest-image tests pass on the exact dependency head.
- [ ] This module passes exact-head formatting, 2D/3D unit tests, and the full math crate suite.
- [ ] Pure basis rebases preserve Cartesian position and physical velocity for a frozen random/unimodular corpus.
- [ ] Physical deformation reports F, volume ratio and B_dot-induced velocity with independent analytic checks.
- [ ] A separate integrator specifies state evolution between physical sample times.
- [ ] Work/virial/energy semantics are specified before coupling deformation into a dynamics solver.
- [ ] Periodic collision/CCD frame revisions and contact identities are qualified before any PhysicsWorld integration.

## Claim ceiling

This is a bounded affine kinematics and representation-rebase utility for full-rank 2D/3D Euclidean periodic cells. It does not implement an NPT molecular-dynamics engine, barostat, arbitrary spacetime metric, or physical pocket dimensions.