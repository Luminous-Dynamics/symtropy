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


## 10. Completed coupling-sensitivity baseline — 22 November 2026

The exact first derivative of the full two-column boundary map at the Einstein limit is now available in the fixed published endpoint coordinates:
\[
J_0=\begin{pmatrix}1/2&0\\0&2\end{pmatrix},\qquad
J_1=\begin{pmatrix}1/2&19-2\pi^2\\0&67-6\pi^2\end{pmatrix}.
\]
The known physical-root tangent gives the independent affine-offset derivative \((-53/9,28/3)^T\). The determinant slope is \(69/2-3\pi^2\).

This supplies a new matched sensitivity benchmark:

\[
E_{J_1}(\lambda)=
\left\|
\frac{J_{\mathrm{BVP}}(\lambda)-J_0}{\lambda}-J_1
\right\|_F.
\]

Protocol:
1. Solve the finite-positive-\(\lambda\) BVP for two independent horizon-data directions, plus at least one independent affine/particular solution.
2. Reconstruct the full matrix \(J_{\mathrm{BVP}}(\lambda)\) from the forbidden asymptotic amplitudes.
3. Evaluate each matrix entry and \(E_{J_1}\) at at least three decreasing positive couplings.
4. Verify entrywise convergence and the predicted determinant slope. Do not report only a fitted aggregate slope.
5. Freeze the horizon/asymptotic coordinate bases and endpoint metrics before the sweep. If either basis is transported with coupling, include its derivative in the reported matrix sensitivity.
6. Preserve failed or non-asymptotic BVP points and disclose precision, residuals, spectral tails, and conditioning.

The expected asymptotic behaviour for a regular second-order expansion is \(E_{J_1}(\lambda)=O(\lambda)\), but this must be measured; it is not assumed evidence of finite-coupling analyticity. The exact \(J_1\) is a formal derivative at zero, not a finite-\(\lambda\) existence or invertibility certificate.

The result and full derivation are recorded in the public increment index and the dedicated action-row / Green--Lagrange artifacts. The exact-source local replay is not a GitHub Actions/CI pass.


### Endpoint singular-frame transport

For the boundary-map sensitivity benchmark, also compare the left/right singular-vector rotations, not only determinant and singular values. With the fixed raw Einstein-limit basis and \(b=19-2\pi^2\), the exact first-order slopes are:
\[
v_{\min}'(0)=-(2b/15)e_h,\qquad
u_{\min}'(0)=-(8b/15)e_H,
\]
with the complementary maximum-singular-vector directions rotating oppositely. Numerically the minimum right-frame component slope is \(0.0985611736238288\), and the minimum left-frame component slope is \(0.3942446944953152\). These values are prospective finite-difference regression targets in the frozen raw basis; they are not invariant under endpoint rescaling. When using metric-aware singular vectors, transform both endpoint metrics and track the same continuous orientation convention.


## Independent endpoint-projection cross-check (22 November 2026)

The exact Green--Lagrange endpoint amplitudes now have a second, implementation-independent numerical check. A separate mpmath tanh-sinh quadrature (without SymPy, the action-row fixture, or the symbolic verifier) reproduces \(C_H=67-6\pi^2\) and \(C_B=19-2\pi^2\) at requested precisions 40/60/80 digits. The largest reported discrepancies are below \(1.5\times10^{-46}\) and \(4.8\times10^{-47}\), respectively.

This validates the integral evaluation numerically, not the action variation, not the finite-coupling BVP, and not an existence theorem. It is a second oracle to catch integration/endpoint algebra mistakes before proceeding to finite-positive-coupling corrections.


## 12. Updated fair comparison with Akribeia and analytic reconstruction (22 November 2026)

### Akribeia reference envelope

Akribeia's 5D EGB matched demonstration uses \(r_H=1\), \(\Omega_H=0.33\), and \(\alpha\in[0,3]\) in steps of 0.01, with a random-initialized PINN discovery phase and pseudo-spectral Gauss--Newton refinement. It also reproduces 5D EGB data at \((\alpha,\Omega_H)=(2,0.68)\). The separate Myers--Perry control reports relative \(L^2\) profile error \(10^{-85}\) at 300-digit precision. The paper's \(O(10^{-80})\) refined total residual at \(N=500\) belongs to its **7D cubic Lovelock** example, not the 5D EGB family. A distinct 5D/7D table reports MP-control errors versus \(\Omega_H\) and initialization; do not conflate these with the EGB field-equation residual.

**Symtropy matched acceptance target:** reproduce the same 5D parameter slice, report continuation/branch identity, per-original-row and held-out residuals, independent thermodynamics, sensitivities, failed random starts and total cost. Then seek additional evidence Akribeia does not claim in the same metric: interval-validated existence/local uniqueness and a formulation-specific hyperbolicity classification.

### Ma--Lü direct first-coupling overlap

Ma--Lü's published 5D \(Riemann^2\) correction about MP, through tenth order in two spin parameters at linear coupling, is the next independent check for the Symtropy \(O(\lambda a^2)\) tangent. The likely coupling relation \(\alpha_R=\alpha_{GB}/4\) and equal-spin conversion \(\chi=\Omega_H r_H\) are *candidate mappings only*; derive the linearized equation identity and gauge/charge map before scoring this as a match. See issue #6915.

### Li--Hu--Yunes analytic Kerr reconstruction

Their no-mode-decomposition \(\Psi_0\) agrees with a separate mode-sum result below \(1.4\times10^{-22}\) at a sampled grid of five polar angles and 200 radii for \(a/M=0.9999\), with \(\ell_{\max}=100\); available horizon thermodynamic predictions are also matched exactly. That is a strong sampled cross-check but not a full-domain bound or runtime comparison. Symtropy's reproduction benchmark must distinguish pointwise sample discrepancy, full reconstructed linearized-EOM residual, observable agreement and certified enclosure.

### No aggregate “best residual” score

Use a vector scorecard: discovery hit rate; branch coverage; matched-parameter coverage; original-row residual by row; held-out residual; spectral-tail estimate; profile and observable errors; constraint/Noether defects; condition and hyperbolicity margins; validated existence status; and wall-time/memory. Report the arithmetic and resolution for every entry. A value of \(10^{-85}\) in one category does not dominate a \(10^{-22}\) result in another category.


## Independent endpoint integral: second quadrature family

A separately implemented composite Gauss--Legendre method with analytic endpoint-log subtraction now reproduces the Green amplitudes. At 40 dps the absolute discrepancies are \(1.86\times10^{-35}\) for \(C_H\) and \(6.21\times10^{-36}\) for \(C_B\); at 60--80 dps they are approximately \(1.05\times10^{-49}\) and \(3.49\times10^{-50}\). The local script was executed and its Git blob SHA-1 matches the public branch exactly.

The matched pair of numerical oracles is now:
- tanh--sinh with \(s=1-t^2\);
- composite Gauss--Legendre after analytic subtraction/restoration of thirteen endpoint-log terms.

They share the mathematical integral but use distinct quadrature algorithms. Both are numerical checks, not interval enclosures.


## Ma--Lü map correction and replay (23 November 2026)

The paper's Eq. (16) prints chi=a/mu, not a/sqrt(mu), and it is not chi=Omega_H*r_H. For equal spins, its Eqs. (4), (6), (16) imply mu/r_h^2=(1+q^2)^2/2, nu=q/(1+q^2), and chi*r_h=2q/(1+q^2)^2, with q=a/r_h, nu=Omega_H*r_h. Exact inversion gives q=nu+nu^3+2nu^5+... and chi*r_h=2nu-2nu^3-2nu^5+.... The parameter-map script was replayed locally from its exact public blob; all symbolic checks pass. Do not use the earlier shorthand conversion for higher-spin comparisons.
