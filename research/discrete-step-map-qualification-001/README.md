# Discrete Step-Map Qualification Seed

This independent Rust crate establishes a reproducible analytic benchmark for the discrete-step-map research lane in Symtropy issue #3975.

## What it implements

- A validated one-dimensional linear mass-spring-damper model in SI units.
- The fused implicit discrete update using the positive effective metric:
  - m_hat = m + h*c + h^2*k
  - m_hat*a = -k*q - (c+h*k)*v
  - v_next = v + h*a
  - q_next = q + h*v_next
- A semi-implicit Euler baseline.
- An independent closed-form continuous-time solution for underdamped, critically damped and overdamped cases.
- The analytic Jacobian of the complete discrete transition.
- An exact discrete energy-change identity.
- Dimensionless stiffness and damping parameters.

The discrete acceleration is the finite-difference step (v_next-v)/h; it is not the instantaneous continuous acceleration.

## Qualification properties

The tests cover:

1. Full-step Jacobian agreement against central finite differences.
2. Bounded energy for a stiff case beyond the semi-implicit Euler stability interval.
3. A negative control in which semi-implicit Euler grows rapidly outside its stable range.
4. The exact energy dissipation identity for nonnegative damping and stiffness.
5. First-order convergence toward the independent closed-form trajectory.
6. Unit-invariant dimensionless parameters.
7. Fail-closed validation of invalid models, states and timesteps.

## Scope boundary

This seed is not a MuJoCo implementation, not a contact solver and not a rigid-body simulator. Its purpose is to establish a small independently checkable discrete-step oracle before adding unilateral contact, friction, articulated constraints or a backend adapter. The next contact lane must bind the contact law, constraint regularization, event ordering and solver semantics explicitly.

No SOTA superiority, production solver validation or hardware result is claimed.
