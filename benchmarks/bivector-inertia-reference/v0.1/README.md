# N-D Bivector Inertia Reference v0.1

**Tracking issue:** [PHYS-BIVECTOR-INERTIA-001 #6414](https://github.com/Luminous-Dynamics/symtropy/issues/6414)  
**Related production limitations:** [#4516](https://github.com/Luminous-Dynamics/symtropy/issues/4516), [#6144](https://github.com/Luminous-Dynamics/symtropy/issues/6144) and [PR #6411](https://github.com/Luminous-Dynamics/symtropy/pull/6411)

**Status:** M1 analytical reference with synthetic tests. No production engine replacement or physical body measurement.

## Source contract and limitation

The current RigidBody stores inertia and inverse inertia as one scalar per coordinate axis (SVector<f64,D>), while angular velocity and torque accumulator are bivectors. The current integrator and center-of-mass angular impulse helper use the arithmetic mean of inverse inertia for every bivector plane.

The integrator explicitly documents that this makes all rotations isotropic and is only exact for spheres. It does not transform principal inertia through body orientation. This reference addresses the mathematics under one explicit assumption: the supplied I_i are principal moments about the local body axes.

## Derivation

Let the diagonal mass second-moment tensor in the body principal frame be:

\[
C = \int r r^T\,dm = \operatorname{diag}(C_0,\ldots,C_{D-1}).
\]

The principal moment about axis i is the sum of the second moments on all orthogonal axes:

\[
I_i = \operatorname{tr}(C)-C_i.
\]

For D >= 2, this yields:

\[
\operatorname{tr}(C)=\frac{\sum_i I_i}{D-1},
\qquad
C_i=\frac{\sum_k I_k}{D-1}-I_i.
\]

For an antisymmetric angular-velocity generator Omega, rotational kinetic energy is:

\[
T=\frac12\operatorname{tr}(\Omega^T\Omega C)
=\frac12\sum_{i<j}(C_i+C_j)\Omega_{ij}^2.
\]

Therefore the inertia operator on bivectors is:

\[
\mathcal I_C(\Omega)=C\Omega+\Omega C,
\qquad
J_{ij}=C_i+C_j.
\]

When C is diagonal, every body-frame plane is an eigenplane with its own scalar inertia J_ij. In 3D, J_ij equals the principal moment about the complementary axis, as required by conventional rigid-body mechanics.

For a body-to-world rotation matrix R:

\[
\Omega_{body}=R^T\Omega_{world}R,
\quad
L_{body}=R^T L_{world}R,
\]

\[
\Delta\Omega_{body}=\mathcal I_C^{-1}(L_{body}),
\quad
\Delta\Omega_{world}=R\Delta\Omega_{body}R^T.
\]

The implementation checks that R is a proper orthogonal matrix before applying this adjoint transformation.

## Realizability and assumptions

The reconstructed C_i must be nonnegative. Consequently the principal moments must satisfy the generalized triangle inequalities I_i <= sum(other I_k). In addition, every represented plane inertia must be positive for an invertible response.

This is only a principal-axis diagonal model. It does not represent nonzero products of inertia in the local body frame, changing deformation, mass redistribution, articulated internal motion, or a full spatial inertia with translational/rotational coupling.

The production RigidBody inertia field is currently used by constructors and call sites without one explicit documented statement establishing that it always contains principal moments in local coordinates. Do not feed arbitrary values into this reference as if that contract had been proven. The common 3D sphere constructor's 0.4*m*r^2 value is a 3D sphere formula; it is not a general N-dimensional ball inertia formula.

The formal 2D and 4D fixtures validate the algebraic extension only. They do not establish the physical meaning of the current 2D/4D engine inertia vector.

## Implemented API

- PrincipalInertia::new(principal_moments) reconstructs C and the positive plane-inertia spectrum, rejecting non-realizable inputs.
- apply_inverse_body applies the diagonal body-frame operator.
- apply_inverse_world rotates a world-frame impulse into the principal frame, applies the inverse plane operator and rotates the response back.
- kinetic_energy_world evaluates rotational energy using the same frame convention.
- midpoint_mechanical_work computes discrete impulse work at the generalized angular port.

The crate is dependency-free and is a research/reference implementation, not hot-loop production code.

## Regressions

The Rust tests cover isotropic 3D sphere behavior, asymmetric 3D complementary-axis moments, 2D one-plane and 4D multi-plane known answers, rotated anisotropic response against an independently assembled direct matrix reference, midpoint work equality with exact rotational kinetic-energy change, and invalid/non-realizable moments, invalid rotations and dimension mismatches.

The JSON fixture records the derivation and known answers.

## Reproduction

From the repository root:

\`\`\`sh
cargo fmt --manifest-path benchmarks/bivector-inertia-reference/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/bivector-inertia-reference/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/bivector-inertia-reference/v0.1/Cargo.toml --all-targets --locked -- -D warnings
\`\`\`

## Promotion path

- **M0:** confirm the meaning and frame of RigidBody::inertia at every constructor and deserialization boundary.
- **M1:** verify this analytic operator and energy identity with independent direct-matrix regressions (this reference).
- **M2:** integrate plane-specific body inertia in the engine with local/world transformation and explicit serialization migration.
- **M3:** compare free rotation, applied torques and collision impulses for spheres, rods, boxes and compounds against independently derived or trusted 3D/ND reference solutions.
- **M4:** update hinge effective mass, motor response and contact lever-arm operators to share one qualified inertia implementation.

## Claim ceiling

Passing the software tests establishes the algebraic reference under the stated principal-axis assumption. It does not prove that current production inertia inputs satisfy that assumption, does not repair the existing engine, and is not a physical qualification or human-contact safety result.
