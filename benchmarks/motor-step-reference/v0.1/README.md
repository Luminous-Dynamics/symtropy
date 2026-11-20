# Motor Step Semantics Reference v0.1

**Tracking issue:** [MOTOR-STEP-REFERENCE-001 #6403](https://github.com/Luminous-Dynamics/symtropy/issues/6403)  
**Parent engine defect:** [ACTUATOR-STEP-SEMANTICS-001 #6144](https://github.com/Luminous-Dynamics/symtropy/issues/6144)  
**Related hinge semantics:** [PHYS-HINGE-MOTOR-001 #4516](https://github.com/Luminous-Dynamics/symtropy/issues/4516)

**Status:** M1 scalar analytical reference with synthetic known-answer and regression tests. Not a production solver.

## Defect being isolated

The current world calls velocity constraints inside the solver-iteration loop and passes the same full frame dt on each invocation. The current hinge and prismatic motor paths calculate and apply effort inside that call. Consequently, actuator command application and work callbacks can be repeated once per solver iteration.

The engine fix needs to preserve iterative coupling with contacts and passive constraints without granting the motor a fresh full-timestep impulse budget on every iteration.

## Scalar reference contract

For a one-dimensional generalized coordinate with target velocity \(v^*\), current relative velocity \(v\), and effective inverse mass/inertia \(K>0\):

\[
\Delta\lambda_{desired} = \frac{v^*-v}{K}
\]

Project the new accumulated generalized impulse into the single per-step effort budget:

\[
\lambda_{new} =
\operatorname{clamp}\left(
\lambda_{old}+\Delta\lambda_{desired},
-u_{max}\Delta t,\;
+u_{max}\Delta t
\right)
\]

Apply only the incremental difference:

\[
\Delta\lambda = \lambda_{new}-\lambda_{old}, \qquad
v_{new}=v+K\Delta\lambda
\]

The scalar generalized mechanical work of this incremental impulse is:

\[
\Delta W = \Delta\lambda\frac{v+v_{new}}{2}
\]

For constant effective inverse mass this equals the exact generalized kinetic-energy change:

\[
\Delta E_k = \frac{1}{2K}(v_{new}^2-v^2)
\]

This reveals a source-level accounting issue: \(\Delta\lambda\,v\) uses only the pre-impulse velocity and omits the \(\frac12K(\Delta\lambda)^2\) term. It is not the exact discrete mechanical energy change for the isolated scalar impulse.

For two endpoints:

\[
K=m_A^{-1}+m_B^{-1}
\]

Static and kinematic endpoints contribute zero inverse mass. If A is fixed and B is dynamic, \(K=m_B^{-1}\); the dynamic body must not receive an arbitrary half response.

## Receipt semantics

Create a fresh MotorStepBudget once per elapsed world step and motorized generalized coordinate. Every solver iteration updates its accumulated impulse and returns only the incremental impulse. Each iteration's midpoint mechanical work is accumulated. After all iterations, consume the budget with finish() to produce one aggregate step receipt.

This prevents repeated full-dt effort budgets and per-iteration duplicate ledger events. It does not say that the total result is independent of coupled constraint order or solver convergence; the invariant is that motor impulse cannot exceed the one-step cap.

## Scope and limitations

- Scalar velocity-target projection only; it does not implement a production PD controller.
- A production position/velocity controller may supply the target velocity, but coordinate extraction and position mode remain under #4516.
- The reference uses one scalar effective inverse mass/inertia. It is not a general hinge inertia mapping, multi-axis rigid-body operator or contact solver.
- Work is at the generalized mechanical port. It is not bus energy, copper loss, battery draw or electrical regeneration.
- A negative mechanical work value is absorption at that port, not proof that a drive can return the energy to a battery.
- The production engine still needs passive-contact coupling, fixed/kinematic endpoint tests, timestep convergence, actual applied-impulse receipts and energy balance.

## Regressions

The crate includes tests for:
- iteration counts 1/2/4/8 with the same total impulse and work;
- total impulse bounded by maximum effort multiplied by dt;
- passive-constraint disturbance followed by motor reprojection using only remaining budget;
- static endpoint effective inverse mass;
- positive and negative work exactly matching kinetic-energy changes;
- one aggregate receipt after all solver iterations;
- invalid timestep, effort, non-finite and inverse-mass inputs.

The JSON fixture file records machine-readable known-answer cases.

## Reproduction

From the repository root, run package-scoped rustfmt, test and Clippy against the Cargo manifest:
- cargo fmt --manifest-path benchmarks/motor-step-reference/v0.1/Cargo.toml -- --check
- cargo test --manifest-path benchmarks/motor-step-reference/v0.1/Cargo.toml --locked
- cargo clippy --manifest-path benchmarks/motor-step-reference/v0.1/Cargo.toml --all-targets --locked -- -D warnings

## Claim ceiling

This reference demonstrates a scalar mathematical contract with synthetic values. It does not establish that the current engine meets the contract, fix #6144, verify #4516 hinge inertia, or authorize physics, robotics or hardware claims.
