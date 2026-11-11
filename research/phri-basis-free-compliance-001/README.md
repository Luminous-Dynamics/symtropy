# PHRI-BASIS-FREE-COMPLIANCE-001 — matrix-coordinate passivity reference

Status: small 3×3 synthetic constraint evaluator. No optimizer, robot controller, continuous-time theorem proof, sampled-data proof, physical measurement or human-contact qualification is claimed.

## Candidate formulation

For scalar task-space inertia scaling epsilon, configuration-dependent task inertia Lambda(q), symmetric stiffness K, symmetric damping D, task error e and error velocity v, the candidate storage function is:

    S = 0.5 * epsilon * vᵀ Lambda(q) v + 0.5 * eᵀ K e

Under the same task-space dynamics and skew-symmetry compensation used in the cited compliance derivation, a sufficient task-space passivity condition is:

    0.5 * epsilon_dot * vᵀ Lambda(q) v
      + 0.5 * eᵀ K_dot e
      - vᵀ D v <= 0

Candidate parameter barriers are:

    epsilon_dot >= -kappa_epsilon * (epsilon - epsilon_min)
    K_dot + kappa_K * K >= 0 in the Loewner/PSD order

V0 implements an evaluator for a proposed rate pair, not the optimization. It validates the SPD task inertia and stiffness, PSD damping, scalar inertia-rate barrier, PSD stiffness-rate barrier and passivity residual.

## Why basis-free

The 2026 Dinc et al. full-compliance extension represents K and task inertia through a generalized eigenbasis. Its derivation assumes simple, separated generalized eigenvalues and continuously tracked eigenvectors, estimates the basis derivative, and includes an additional Gamma term in the passivity condition. V0 instead evaluates the candidate matrix-coordinate storage derivative directly. Repeated eigenvalues require no selected eigenvector basis in this reference.

This is a candidate research direction only. A basis-free evaluator does not, by itself, prove that a full controller can realize arbitrary matrix-rate updates, or that sampled integration preserves the continuous-time inequality. It also does not solve the associated convex semidefinite optimization problem.

## Numerical reference

Symmetric 3×3 matrices use six stored coordinates. PSD is checked by all principal minors after scaling by the largest absolute entry; SPD uses Sylvester's leading principal minors. The tolerance is dimensionless and declared by the caller. This is a numerical screening method, not a symbolic proof or an independent validated eigensolver.

## Regression coverage

Nine authored deterministic tests cover:
- PSD/SPD known answers;
- repeated eigenvalues without eigenvector tracking;
- off-diagonal stiffness-rate quadratic terms;
- passivity and scalar/matrix decay barriers;
- invalid inertia, stiffness, damping, non-finite state and tolerance.

These tests are authored, not yet compiled or executed.

## Required next steps

1. Derive the basis-free task-space dynamics and storage inequality independently from the chosen robot model.
2. Implement the weighted matrix-rate QP/SDP with Loewner and application-specific rate bounds.
3. Compare it against the Dinc generalized-eigenbasis QP on repeated/near-degenerate eigenvalue sequences and rotating eigenspaces.
4. Quantify continuous-time analytical and discrete-time sampled passivity separately.
5. Measure conditioning, feasibility, solver p99 latency and numerical residual under matched compute and plant conditions.
6. Analyze null-space passivity separately; task-space passivity does not imply null-space passivity.
7. Only then proceed from synthetic to phantom, bench, HIL and appropriately approved human-subject work.

## References

- Dinc et al. (2026), *Passivity-Guaranteed Variable Impedance and Compliance Control via Online Parameter Optimization*, arXiv:2611.10778: https://arxiv.org/abs/2611.10778

## Claim boundary

The reference does not synchronize clocks, estimate force, implement passivity control, command actuators, validate a tissue pressure envelope or authorize human contact.