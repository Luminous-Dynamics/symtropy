# Rotating black-hole discovery and independent certification protocol

**Status:** proposed research design; no neural solver or rotating solution is implemented by this document.  
**Parent:** [BH-THEORY-TRIAD-001](README.md) / [PR #5814](https://github.com/Luminous-Dynamics/symtropy/pull/5814)

## Why this exists

Akribeia demonstrates a timely methodological pattern: a physics-informed neural field searches for rotating solutions, then a pseudo-spectral Chebyshev solver with extended precision refines the candidate. Its 2026 accepted paper demonstrates this in higher-dimensional Lovelock gravity, particularly cohomogeneity-one equal-angular-momenta sectors, not in our four-dimensional static Weyl problem. It reports recovery of the Myers–Perry limit at an extraordinary relative L2 error using 300-digit arithmetic; the result is an author-reported claim until independently reproduced.

We should adopt the *separation of discovery from certification*, not copy its physics ansatz or assume its accuracy transfers to another theory.

## Core rule

**A candidate generator must never be its own verifier.**

The discovery engine may be a neural field, continuation method, spectral ansatz, shooting method, or a hybrid. It may propose a field configuration and branch. Qualification is performed by independently implemented equations, boundary operators, numerical refinement and validation points that were not used to train or tune the candidate.

## Pipeline

1. **Freeze theory identity.** Hash the action, equations, coupling conventions and dimensions, matter sector, gauge, coordinate ansatz, boundary conditions, horizon regularity conditions and solution branch definition.
2. **Establish analytic controls.** Recover exact GR limits (Schwarzschild, Kerr, or Myers–Perry as appropriate), known static solutions, zero-coupling limits, slow-rotation limits and any known extremal limits.
3. **Discover, do not certify.** Run multistart global discovery. Candidate methods may include PINNs, neural operators, spectral basis continuation or randomized initial profiles. Record failed seeds and all candidate branches.
4. **Refine independently.** Transfer the candidate to a conventional pseudo-spectral or finite-element representation. Solve with a separately implemented residual and Jacobian. Increase domain resolution and arithmetic precision independently.
5. **Evaluate off-grid.** Compute the complete field-equation residual at unseen radial/angular points, including near-horizon, intermediate and asymptotic regions. The residual evaluator must not reuse the discovery loss implementation.
6. **Check regularity and boundary conditions.** Evaluate horizon regularity in a horizon-regular chart; asymptotic conditions in the declared asymptotic chart; verify gauge constraints, constraints propagated by Bianchi identities, and conserved charges.
7. **Track branches.** Vary couplings, spin and horizon data. Detect folds, bifurcations, extremal limits, branch crossings and loss of ellipticity / hyperbolicity. Use pseudo-arclength continuation where ordinary parameter continuation fails.
8. **Test stability of the numerical answer.** Repeat at multiple resolutions, precisions, collocation grids, initializations and domain compactifications. Distinguish small residual from small solution error using conditioning, Jacobian singular values and invariant-based comparisons.
9. **Cross-check independent formulations.** Where possible compare a second gauge, an alternative coordinate chart, a distinct discretization, and an independent symbolic or tensor implementation.
10. **Publish a certificate, including failures.** Record the exact source/data digests, action hash, source commit, solver versions, precision, grid, residual norms, convergence sequence, conditioning and all rejected branches.

## Qualification gates

| Gate | Requirement | Failure interpretation |
|---|---|---|
| D0 Theory contract | Action, equations, gauge and boundary data pinned | Not a well-defined candidate |
| D1 Analytic limits | Exact GR / static / slow-rotation controls recovered | Method or convention error |
| D2 Discovery diversity | Multistart candidates and branch identities recorded | Discovery coverage unknown |
| D3 Independent refinement | Candidate converges in a separate solver | Candidate not numerically qualified |
| D4 Off-grid residual | Full independent equations small over held-out domain | Training residual overfit |
| D5 Regularity | Horizon, axis and asymptotic regularity checked in regular charts | Candidate is singular or chart artifact |
| D6 Constraint consistency | Differential identities and constraints close | Inconsistent discretization or equations |
| D7 Convergence | Resolution and precision refinement stabilize functions and invariants | Reported digits unsupported |
| D8 Conditioning | Small residual translates to controlled solution error | Ill-conditioned result; report uncertainty |
| D9 Branch completeness | Multistart / continuation records branch coverage and folds | No completeness claim |
| D10 Independent reproduction | Different implementation reproduces invariants and residuals | Provisional only |

A low residual at training collocation points alone passes none of D3–D10.

## Metrics to publish

- Full independent equation residual: L2, L-infinity, weighted norms and spatial maps.
- Boundary and regularity residuals separately from bulk residuals.
- Relative solution error against exact solutions when available.
- Convergence order versus spectral order / grid refinement.
- Invariant error: horizon area, angular velocity, ADM/Komar charges where well-defined, curvature scalars and thermodynamic quantities.
- Conditioning: smallest singular values of the linearized operator / Jacobian, condition estimates and precision sensitivity.
- Discovery cost: success probability per random initialization, time to first candidate, number of candidate branches.
- Refinement cost: time, peak memory and precision needed to reach qualified accuracy.
- Branch coverage: parameter-volume sampled, folds detected and repeatability across seeds.
- Failure transparency: non-convergence, singular branches and excluded domains.

Do not use a single average residual as the headline metric. A tiny average can hide a large near-horizon error.

## Required null tests

### Geometry / field equations
- Schwarzschild / Kerr / Myers–Perry exact solution recovers the known equations and invariants.
- Zero higher-curvature coupling returns the GR equations and GR solution branch.
- A deliberately off-shell perturbation produces a nonzero field-equation residual.
- Constraint identities and Bianchi consequences close independently.
- A change of coordinates or gauge leaves scalar invariants and physical charges unchanged within numerical uncertainty.

### Perturbation / QNM
- Redundant operators or field-redefinition terms that vanish on shell must not create a spurious first-order vacuum QNM shift.
- When distinct operator bases are equivalent on the chosen background, their calculated pole shifts must agree.
- Compute the same shifts using two independent methods (for example eigenvalue perturbation and generalized continued fractions).
- Compare complex pole locations and residues across multipoles and overtones; do not report frequency shifts alone.

These null tests follow the methodological pattern in Aly et al., *Modified black-hole perturbation formalism: Null testing and numerical benchmarking*, Phys. Rev. D 114, 024077 (2026), DOI 10.1103/w112-1pnj.

## Scope warning for applying Akribeia-like methods

The accepted 2026 Akribeia result is in higher-dimensional Lovelock gravity with a symmetry-enhanced equal-angular-momenta ansatz, reducing the stationary field equations to cohomogeneity-one ODEs. A four-dimensional generic rotating black-hole ansatz is generally a harder two-dimensional boundary-value problem, often with gauge and coordinate complications. Neural-field candidate generation may help, but there is no automatic transfer of the cited precision or existence result.

The first Symtropy benchmark should be an exact known branch with a reproducible spectral solution. Only after a blind recovery test should a new beyond-GR branch be attempted.

## References

- Agurto-Sepúlveda et al., *Neural-spectral discovery of rotating black holes beyond general relativity*, Phys. Rev. D accepted 12 Nov 2026, DOI [10.1103/3kz2-6dvv](https://doi.org/10.1103/3kz2-6dvv); preprint [arXiv:2607.07418](https://arxiv.org/abs/2607.07418).
- Aly et al., *Modified black-hole perturbation formalism: Null testing and numerical benchmarking*, Phys. Rev. D 114, 024077 (2026), DOI [10.1103/w112-1pnj](https://doi.org/10.1103/w112-1pnj).

## Claim ceiling

This protocol does not demonstrate that a PINN, neural operator, spectral refiner or any current Symtropy solver can discover rotating Weyl or scalar-conformal black holes. It specifies how any such candidate must be independently checked before being called a solution.
