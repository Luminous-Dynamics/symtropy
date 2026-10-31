# Port-Hamiltonian discrete-gradient reference

**Status:** experimental smooth-system reference; not a production rigid-body/contact integrator.
**Date:** 2026-10-31
**Branch:** `research/port-hamiltonian-discrete-gradient-2026-10-31`

## Motivation

The discrete work ledger makes missing energy channels visible, but it does not yet enforce a balance law across rotational dynamics, joints, motors, damping, or contact. A stronger research direction is to represent energy storage, external power ports, and dissipation as part of the system structure rather than reconstructing the balance after a real-time step.

Port-Hamiltonian systems express a state (x), stored energy (H(x)), skew interconnection (J), positive-semidefinite dissipation (R), input map (G), external effort (u), and conjugate output (y):

\[
\dot{x}=(J-R)\nabla H(x)+Gu, \qquad
y=G^T\nabla H(x).
\]

For (J=-J^T) and (R=R^T\succeq0), the continuous power balance is:

\[
\dot H=-\nabla H^T R\nabla H+y^Tu.
\]

A 2026 paper develops discrete-gradient methods for port-Hamiltonian differential-algebraic equations and demonstrates their application to multibody systems (Kinon, Morandin and Schulze, *Applied Numerical Mathematics* 223, May 2026, DOI 10.1016/j.apnum.2025.12.006). A separate goal-oriented time-adaptivity study shows how energy-balance error estimates can guide timestep refinement for port-Hamiltonian systems. These works motivate the research direction; they do not establish that this prototype is comparable to their full methods.

## Implemented reference

The new `QuadraticPortHamiltonian` module is deliberately narrower. It supports a quadratic Hamiltonian:

\[
H(x)=\frac12 x^TQx,
\]

where Q is symmetric positive-semidefinite. The discrete gradient is exactly the midpoint gradient:

\[
\bar\nabla H(x_0,x_1)=Q\frac{x_0+x_1}{2}.
\]

For constant input u over timestep h, the implicit step solves:

\[
x_1-x_0=h(J-R)Q\frac{x_0+x_1}{2}+hGu.
\]

This is a linear system for (x_1). The resulting energy identity is:

\[
H(x_1)-H(x_0)
=-h\bar\nabla H^TR\bar\nabla H
+h\,y_{1/2}^Tu,
\]

where (y_{1/2}=G^T\bar\nabla H).

The API validates:
- Q symmetric positive-semidefinite;
- J skew-symmetric;
- R symmetric positive-semidefinite;
- matrix/input dimensions and finite values;
- positive timestep and solvability of the implicit step.

Each accepted step returns a `PortHamiltonianStepReceipt` with initial/final state and energy, supplied work, dissipated energy, relative energy-balance residual, and relative linear-solve residual. These two residuals are separate so algebraic solve quality is not conflated with physical-model balance.

## Regression tests

The prototype contains:
1. an undamped Hamiltonian oscillator that preserves energy;
2. a dissipative oscillator whose energy loss equals the reported dissipation;
3. an externally driven port whose supplied work matches stored-energy change;
4. negative controls for non-skew interconnection and indefinite dissipation.

These are small algebraic checks. They do not establish a general high-order DAE solver, contact method, or advantage over industry engines.

## Microbenchmark

A Criterion group measures implicit steps at state dimensions 2, 16 and 64:

```sh
cargo bench -p symtropy-physics --bench delassus_reference
```

The model construction and validation occur outside the timed loop. The benchmark measures the reference step's dense linear solve and receipt cost. It is not a comparison against MuJoCo, Rapier, PhysX, Newton or Chrono, and no timing result is claimed until executed on pinned hardware.

## Explicit limitations

- Quadratic Hamiltonian only; no nonlinear discrete-gradient construction yet.
- Smooth finite-dimensional ODE only; no algebraic constraints, contact complementarity, impact reset, unilateral constraints or joints.
- Dense matrices and dense LU; not suitable for large sparse multibody systems.
- Constant input during each step.
- The model is not connected to `PhysicsWorld` and must not be described as its integrator.
- No gradient, sensitivity or adaptive timestep interface is provided.

## Next research stages

1. Extend the reference to nonlinear Hamiltonians with an independently tested discrete-gradient construction.
2. Add semi-explicit DAEs and constraint manifolds, with constraint residuals and energy balance reported together.
3. Model contact as a dissipative/jump port and prove the correct impact/friction energy inequality rather than assuming smooth dynamics.
4. Add sparse/block structure and compare with a high-accuracy independent DAE reference.
5. Build a multibody port-Hamiltonian model with translational and bivector rotational states and explicit joint/motor ports.
6. Only after those steps, compare a research-grade energy-structured mode with the current realtime TGS path at matched error, stability, and computational budget.

The intended architecture is a solver portfolio with separate validity contracts: realtime TGS/sequential impulses, coupled island solvers, and energy-structured research integration. A solver selector may recommend a mode, but it must not certify its own physical correctness.
