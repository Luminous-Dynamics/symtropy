# Spin-2 higher-curvature black-hole discovery and qualification scorecard

**Version:** 1.0  
**Status:** proposed benchmark protocol; no leaderboard result or superiority claim is implied.

## 1. Comparison principle

Do not compare one solver's residual number with another solver's profile error, nor compare a perturbative solution with a finite-spin nonlinear solution as if they solve the same problem.

Freeze four independent axes:

1. **Problem identity:** action normalization, sign conventions, dimension, ansatz, gauge, horizon normalization, coupling definition, spin parameter, ensemble, endpoint conditions, and equation rows.
2. **Discovery:** ability to find candidates from frozen initializations or parameter paths.
3. **Numerical refinement:** convergence, residuals, observables, precision, and computational cost.
4. **Mathematical/physical qualification:** independent equations, endpoint mode counts, boundary-map rank, validated existence, continuation, and PDE hyperbolicity.

Distinguish source-replayed derivations, numerical candidates, refined candidates, validated solutions, local existence theorems, validated branch segments, and physical admissibility.

## 2. Source-faithful Akribeia baseline

Primary comparator: Agurto-Sepúlveda et al., *Neural-Spectral Discovery of Rotating Black Holes Beyond General Relativity*, accepted by Physical Review D on 12 November 2026, DOI [10.1103/3kz2-6dvv](https://doi.org/10.1103/3kz2-6dvv), arXiv [2607.07418v2](https://arxiv.org/abs/2607.07418v2).

The paper defines its “certified neural-field solutions” as continuous, globally defined functions whose field-equation residuals are verified below a quantified threshold. It describes a PINN discovery stage followed by extended-precision pseudo-spectral refinement. This is residual-qualified continuous-field evidence; it is different from a radii-polynomial proof of existence and local uniqueness.

Published comparator facts:

- The paper reports PINN equation residuals around \(10^{-3}\) before spectral refinement.
- It reports a relative \(L^2\) profile error of \(10^{-85}\) for exact Myers–Perry recovery in the Einstein limit, using 300-digit extended precision.
- Its 5D EGB illustration presents \(r_H=1\), \(\Omega_H=0.33\), and a neural family over \(\alpha\in[0,3]\).
- The Akribeia v1.0.0 orchestration selects a 5D stage-2 seed for a fixed-coupling spectral refinement at \(\alpha=1\), \(\Omega_H=0.33\), with precision override 310. Do not infer every point in the neural coupling family received this same high-precision refinement.

Public code:
- [Akribeia v1.0.0](https://github.com/moyarzoca/Akribeia/tree/v1.0.0)
- [DiffLearning v2.1.1](https://github.com/moyarzoca/difflearning/tree/v2.1.1)
- [Deterministic Gauss–Newton refiner v1.1.1](https://github.com/moyarzoca/deterministic-gauss-newton-wl/tree/v1.1.1)

The paper references DiffLearning v2.1.1 and refiner v1.1.1, while the Akribeia v1.0.0 README clones DiffLearning v2.1.0 and refiner v1.0.0. Treat these as separate reproduction manifests. Do not mix versions silently. The companion audit records source files and blob identifiers.

## 3. Matched problem tracks

### Track P: perturbative coefficient comparison (current Symtropy problem)

Symtropy currently derives an \(O(a^2)\) slow-rotation system. Akribeia's headline rotating profiles solve the full nonlinear stationary equations. Comparing directly at \(\Omega_H=0.33\) would mix numerical error with perturbative truncation error.

Fair protocol:

1. Reproduce an Akribeia full nonlinear profile under matched dimension, action, coupling, horizon radius, rotation parameter, gauge, and ensemble.
2. Obtain profiles at at least three decreasing nonzero spin values, holding non-spin parameters fixed.
3. Extract \(B_2,F_2,H_2\) from the small-spin behavior. Fit the expected parity and truncation order; do not assume the remainder scaling without checking it.
4. Compare extracted coefficients against Symtropy's perturbative equations, original unscaled rows, Noether constraint, horizon compatibility, infinity conditions, and independent observables.
5. Publish the fit window, conditioning, spin values, and all rejected/non-asymptotic points.

### Track N: full nonlinear finite-spin comparison (future Symtropy milestone)

Direct finite-spin comparison requires a full nonlinear Symtropy solver. Match the same Akribeia parameter points and endpoint conditions.

### Track E: exact and perturbative controls

Mandatory controls include exact equal-spin Myers–Perry at \(\alpha=0\), static Einstein-branch Boulware–Deser at zero spin, slow-spin Einstein-limit coefficients, sign/normalization mutations, independent radial Noether identity, and gauge/chart controls where available.

A positive control must reproduce its known answer. A negative control must make the acceptance predicate fail; merely logging a mutation is insufficient.

## 4. Discovery and refinement leaderboard

Publish every attempted run, including failures, timeouts, false roots, and rejected candidates.

| Metric | Definition |
|---|---|
| Time to first candidate | Wall time from frozen start to predeclared discovery threshold |
| Time to refinement target | Wall time to predeclared per-row residual and observable tiers |
| Frozen-start success | Successful candidates divided by all preregistered starts |
| Branch coverage | Parameter interval and arclength discovered, separate from validated coverage |
| Branch-event recall | Fraction of known/injected folds or intersections detected |
| False-event rate | Reported events rejected by rank/transversality checks |
| Duplicate-root rate | Duplicates after gauge/normalization-aware equivalence checks |
| Work-precision | Residual and observable error versus collocation order, precision, and work |
| Resource use | Wall time, processor/GPU, thread count, peak memory, core-hours, energy if measurable |
| Robustness | Sensitivity to seed, chart, gauge, precision, tolerances, and held-out nodes |

Compare Akribeia PINN discovery, Akribeia spectral refinement, classical Chebyshev/Newton, Symtropy reduced DAE, pseudo-arclength/deflation, and hybrids separately.

Use identical information budgets. An equation-derived algebraic pivot may be used, but all original equations remain independent qualification checks.

## 5. Refinement metrics

For every original Euler–Lagrange row, report unscaled \(L^\infty\), ordinary \(L^2\), and declared weighted \(L^2\); worst radial location; horizon and infinity defects; radial constraint \(E_g\); Noether defect; spectral tail; held-out-node residual; cross-resolution difference; patch mismatch; observable mismatch; first-law defect where defined; approximate inverse/Jacobian condition; and arithmetic precision.

Use at least three spectral resolutions and multiple precisions. Report scaled and original unscaled residuals. A small aggregate residual must not hide a large individual-row or local residual.

Keep Akribeia's \(10^{-85}\) Myers–Perry relative profile error in a separately named control column. It is not a universal finite-coupling residual target.

## 6. Mathematical qualification leaderboard

| Gate | Required evidence | Claim ceiling |
|---|---|---|
| Q0 — exact controls | MP/BD recovery and mutation rejection | Exact limits |
| Q1 — source replay | Exact source/input/output hashes; clean-checkout generator | Source-replayed equations |
| Q2 — endpoint structure | Horizon/infinity mode counts, compatibility, pivot and Noether dependency | Endpoint-qualified local data |
| Q3 — refined BVP | Finite-coupling candidate, original rows, 3 resolutions, precision escalation | Numerically refined candidate |
| Q4 — global BVP structure | Boundary-map rank/nullity and Fredholm index for stated spaces | Global operator evidence |
| Q5 — validated solution | Full-domain interval residual, spectral tail, approximate-inverse bound | Validated local solution |
| Q6 — existence/uniqueness | Radii polynomial or equivalent fixed-point/interval-Newton theorem | Nearby exact solution, local uniqueness |
| Q7 — validated branch | Enclosed continuation tubes and event boxes | Certified branch segments |

### Proposed headline metrics

**Validated branch yield**
\[
Y_{\rm branch}=\frac{\text{validated branch arclength}}{\text{CPU core-hours}}.
\]
Report discovered arclength separately.

**Qualification conversion**
\[
Q_{\rm conversion}=\frac{\text{candidates passing independent qualification}}{\text{all candidates proposed}}.
\]
Publish both counts and failure reasons.

**Proof latency:** time from first candidate to first independently replayable mathematical certificate.

**Proof debt:** unresolved obligations per candidate, such as endpoint modes, boundary-map rank, interval tail, inverse bound, continuation, and hyperbolicity. Use it to prioritize work, not as a physical quality score.

**Claim coverage:** proportion of the advertised parameter/spacetime domain actually covered by the relevant certificate. Finite samples are not full-domain coverage.

These are proposed metrics, not achievements.

## 7. Branch discovery and rigorous continuation

Use pseudo-arclength continuation to pass ordinary folds and deflation to seek additional roots and disconnected components. These are discovery methods, not completeness proofs. See [BifurcationKit pseudo-arclength](https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/PALC/) and [deflated continuation](https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/DeflatedContinuation/).

A completeness statement needs a separate global argument: validated coverings, degree/homotopy bounds, or an a priori theorem excluding unsearched regions.

For local existence/uniqueness, the Chebyshev-series radii-polynomial framework of Lessard and Reinhardt is concrete: construct a fixed-point operator on a Banach space of decaying Chebyshev coefficients, bound residual and inverse, and verify a contraction/radii-polynomial inequality. See [SIAM J. Numer. Anal. 52 (2014)](https://doi.org/10.1137/13090883X).

## 8. Physical hyperbolicity is independent

Stationary horizon regularity and BVP solvability do not establish a well-posed time-dependent gravitational initial-value problem.

For each background, derive the full EGB principal symbol; quotient gauge directions and impose constraints before extracting physical polarizations; specify a first-order-in-time gauge-fixed formulation; report characteristic speeds, eigenbasis conditioning or symmetrizer margin, and uniformity over spatial covectors; use horizon-regular/asymptotic charts; distinguish sampled surveys from interval-covered domains; and propagate the explicit \(O(a^4)\) uncertainty of a slow-spin background. If that uncertainty is comparable to the hyperbolicity margin, classify the result as inconclusive.

The literature makes this independent gate important: generic Lovelock backgrounds need not be strongly hyperbolic in standard harmonic gauge, and some Lovelock black-hole backgrounds have hyperbolicity violations near horizons. Modified harmonic formulations offer a weak-coupling route, but hypotheses must be checked for the actual background. See [Papallo–Reall 2017](https://doi.org/10.1103/PhysRevD.96.044019), [Reall–Tanahashi–Way 2014](https://doi.org/10.1088/0264-9381/31/20/205005), and [Kovács–Reall 2020](https://doi.org/10.1103/PhysRevD.101.124003).

Use H0–H5: H0 action-level symbol; H1 pointwise physical roots/eigenstructure; H2 adaptive sampled survey; H3 local interval boxes; H4 full declared exterior interval coverage; H5 formulation-level strong-hyperbolicity theorem.

## 9. How Symtropy can exceed the current state of the art

Do not compete only on decimal depth. Build a **proof-producing, branch-aware discovery stack**:

1. Exact action variation, symbolic reduction and algebraic pivots shrink the solve state while preserving original equations as independent checks.
2. Horizon and infinity indicial data generate endpoint-adapted basis functions and boundary jets before numerical discovery.
3. Any candidate proposer (PINN, perturbation, classical seed, continuation, deflation) feeds one independent evaluator.
4. A patch solver joins horizon, interior and infinity charts with explicit overlap residuals.
5. Continuation/deflation discovers candidate topology; interval numerics qualifies segments.
6. The solution atlas stores each node, edge, event, certificate, source hash, and claim ceiling.
7. Full PDE hyperbolicity is assessed separately and attached as a physical-admissibility record.

The first publication-grade differentiator should be one nontrivial finite-coupling rotating solution with independent original-row residuals, full-domain interval enclosure, a radii-polynomial/interval-Newton existence and local-uniqueness certificate, and at least one validated continuation segment. A later result should qualify the physical principal symbol over a declared spacetime domain.

## 10. Benchmark freezes and anti-gaming rules

Before comparative results are inspected, freeze exact commits and dependency locks, physical problem and ensemble, parameter grid and held-out points, initial seeds, equation scaling, collocation nodes, basis and truncation order, solver tolerances, precision schedule, hardware/resource budget, stopping rules, and independent residual/certificate evaluators.

Keep tuning and test points disjoint. Count all failures and timeouts. Queued, skipped, missing status, timeout, or not-run are not passes. Every claim must identify domain, source digest, verifier, receipt, and claim ceiling.

## References

- Agurto-Sepúlveda et al., [Akribeia paper, arXiv:2607.07418v2](https://arxiv.org/abs/2607.07418v2), accepted PRD 12 Nov 2026, [DOI](https://doi.org/10.1103/3kz2-6dvv).
- Lessard & Reinhardt, [Rigorous Numerics for Nonlinear Differential Equations Using Chebyshev Series](https://doi.org/10.1137/13090883X), SIAM J. Numer. Anal. 52 (2014).
- [BifurcationKit pseudo-arclength](https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/PALC/) and [deflated continuation](https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/DeflatedContinuation/).
- Papallo & Reall, [On the local well-posedness of Lovelock and Horndeski theories](https://doi.org/10.1103/PhysRevD.96.044019), PRD 96 (2017).
- Reall, Tanahashi & Way, [Causality and hyperbolicity of Lovelock theories](https://doi.org/10.1088/0264-9381/31/20/205005), CQG 31 (2014).
- Kovács & Reall, [Well-posed formulation of Lovelock and Horndeski theories](https://doi.org/10.1103/PhysRevD.101.124003), PRD 101 (2020).
