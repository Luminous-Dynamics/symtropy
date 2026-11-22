# 2026 state-of-the-art landscape and competitive programme for Symtropy spin-2

**Snapshot date:** 22 November 2026  
**Purpose:** source-grounded comparison and a fair roadmap. This is not a leaderboard result and does not assert Symtropy currently exceeds these methods.

## 1. Three relevant state-of-the-art tracks

### A. Akribeia: full nonlinear stationary higher-dimensional Lovelock solutions

Agurto-Sepúlveda et al., *Neural-Spectral Discovery of Rotating Black Holes Beyond General Relativity*, arXiv:2607.07418v2, revised 19 November 2026 and accepted by Physical Review D on 12 November 2026 ([paper](https://arxiv.org/abs/2607.07418v2), [accepted PRD DOI](https://doi.org/10.1103/3kz2-6dvv)).

Scope and reported evidence:

- 5D equal-angular-momenta EGB and 7D cubic Lovelock stationary solutions, with non-perturbative dependence on the curvature couplings in the solved ODE systems.
- Discovery by a PINN over compactified radius and coupling parameters, followed by high-precision pseudo-spectral Gauss–Newton refinement.
- The paper reports a total residual of order \(10^{-80}\) for its 7D example after increasing the refinement order to \(N=500\), with extended-precision arithmetic.
- It reports a relative \(L^2\) profile error of \(10^{-85}\) for exact Myers–Perry recovery in the Einstein limit, using 300-digit arithmetic. This is an exact-control profile error, not the finite-coupling equation residual.
- The 5D discovery stage takes roughly 15–120 minutes on commercial laptop hardware depending on refinement; the reported spectral refinement at \(N=200\) is about one minute per solution on a single CPU core.

These are serious baselines. The reported total residual is a sum of row \(L^2\) residuals; a fair independent benchmark should additionally publish each unscaled row separately, \(L^\infty\), held-out radial nodes, constraint drift, and observable convergence.

### B. Arbitrary-spin spectral metrics in four-dimensional EFT

Lam, Chung and Yunes, *Analytic and accurate approximate metrics for black holes with arbitrary rotation in beyond-Einstein gravity using spectral methods*, Phys. Rev. D 113, 024030 (15 January 2026), [DOI](https://doi.org/10.1103/txxb-z8bx), [arXiv:2510.05208](https://arxiv.org/abs/2510.05208).

Scope and reported evidence:

- 4D Kerr deformations in quadratic EFTs, including dynamical Chern–Simons, scalar–Gauss–Bonnet and axidilaton gravity.
- Leading order in the coupling constants, but generic spin rather than a slow-rotation expansion.
- The authors report errors below machine precision for \(a\leq0.9\), and \(\lesssim10^{-8}\) for \(a\leq0.999\).
- They explicitly compare the loss of accuracy of existing slow-rotation approximations as spin increases.

This is a crucial warning: an \(O(a^2)\) result cannot be promoted to moderate or near-extremal spin without a measured truncation-error envelope. This 4D Kerr/EFT result is not a like-for-like comparison to 5D equal-spin EGB, but it sets the bar for a separate arbitrary-spin programme.

### C. Generic-spin analytic metric reconstruction at leading coupling

Li, Hu and Yunes, *One Method to Find Them All: Analytic Black Holes beyond Einstein*, arXiv:2610.37515 (27 October 2026), [paper](https://arxiv.org/abs/2610.37515).

The method solves sourced stationary Teukolsky equations and reconstructs metric perturbations through a universal Newman–Penrose hierarchy. The paper reports closed-form leading-coupling results for six four-dimensional purely metric interactions of cubic, quartic and quintic curvature order at generic subextremal spin, eliminating the mode decomposition in the new construction.

This changes the frontier for analytic perturbative metrics: generic spin and multiple interaction families can be handled by exploiting the structure of curvature sources and reconstruction, rather than directly solving every tensor component. Its claim is leading order in couplings around Kerr, not a finite-coupling non-perturbative EGB branch.

## 2. Apples-to-apples comparison matrix

| System | Dimension / symmetry | Coupling regime | Spin regime | Main strength | What is not established by the cited result |
|---|---|---|---|---|---|
| Akribeia | 5D/7D, equal angular momenta | Full nonlinear stationary equations in curvature couplings | Finite rotation in its solved ansätze | Discovery + high-precision spectral refinement | Interval existence/uniqueness and full branch completeness are not the reported residual metric |
| Lam–Chung–Yunes | 4D Kerr, stationary axisymmetric | Leading coupling | Generic, including near-extremal survey | Spectral arbitrary-spin metrics and observable predictions | Not non-perturbative in the coupling |
| Li–Hu–Yunes | 4D Kerr, stationary axisymmetric | Leading coupling | Generic subextremal spin | Closed-form reconstruction across six interactions | Not a finite-coupling nonlinear Lovelock branch |
| Symtropy current increment | 5D, equal-angular-momenta EGB | Exact at \(\lambda=0\), plus exact first derivative at \(\lambda=0\) | Slow rotation \(O(a^2)\) | Action-first global boundary map, exact branch sensitivity, all four order-\(\lambda a^2\) rows | No finite-\(\lambda\) solution, no arbitrary-spin solution, no interval theorem yet |

The present Symtropy result is a *complementary mathematical increment*, not evidence of overall superiority. It closes the Einstein-limit global map and obtains the first coupling derivative of the asymptotically flat 5D perturbative branch. That is a more specific result than a generic statement that one solver is better.

## 3. Strategic paradigm shift: sensitivity-carrying, proof-producing continuation

The next step should not be “PINN versus spectral solver”. It should be a common branch object carrying both the solution and its parameter sensitivities.

For a discretized or functional BVP
\[
\mathcal F(u,\lambda)=0,\qquad \mathcal B(u,\lambda)=0,
\]
a smooth branch satisfies
\[
\begin{pmatrix}
D_u\mathcal F & D_u\mathcal B
\end{pmatrix}
\partial_\lambda u
=-
\begin{pmatrix}
\partial_\lambda\mathcal F\\
\partial_\lambda\mathcal B
\end{pmatrix},
\]
with the endpoint normalization included in \(\mathcal B\). For ODE shooting/collocation, this is an inhomogeneous sensitivity BVP; for an interval proof, the base solution and sensitivity can be enclosed in a coupled operator.

This enables four things:

1. **Predictor quality:** the exact \(\lambda=0\) sensitivity predicts the first finite-coupling correction rather than relying on an arbitrary seed.
2. **Branch conditioning:** the norm of \(\partial_\lambda u\), the smallest singular value of the global matching Jacobian, and the observable sensitivities warn when continuation approaches a fold or loss of regularity.
3. **Adaptive work:** collocation density and arithmetic precision can be allocated where sensitivities and residual bounds grow, not uniformly.
4. **Proof propagation:** interval enclosures can validate a base solution and its tangent on the same continuation tube.

The new exact \(O(\lambda)\) correction gives
\[
b_1'(0)=106/9,\qquad h_0'(0)=-14/3.
\]
It is the tangent of the selected asymptotically flat branch. It is not yet the derivative of the entire two-by-two boundary-map Jacobian; the two independent homogeneous boundary-map columns still need their own sensitivity solves.

## 4. Concrete benchmark programme

Freeze separate tracks so problem-class differences are never hidden.

### Track 1 — 5D EGB equal-spin, slow-spin coefficient problem

Reference solutions: exact Myers–Perry at \(\lambda=0\), exact static Boulware–Deser at zero spin, and the action-derived \(O(a^2)\) system.

Targets:
- Exact rational recovery of all known limits and all original equation rows.
- First and second coupling sensitivities at \(\lambda=0\), independently checked against finite differences of small-\(\lambda\) numerical solutions.
- At least three nonzero coupling values and three spin amplitudes for truncation-error measurement.
- Report the \(O(a^4)\) contamination explicitly; do not extrapolate the \(O(a^2)\) solution beyond its measured regime.
- Boundary-map condition number, branch tangent, observable tangents, and coupling radius of practical convergence.

### Track 2 — 5D EGB equal-spin, full nonlinear stationary BVP

This is the direct Akribeia comparison.

Targets:
- Reproduce the pinned Akribeia version and a matched parameter point first.
- Match \(r_H\), \(\Omega_H\), mass/energy, angular momentum, asymptotic normalization, action convention and equation rows.
- Compare each original unscaled residual, \(L^\infty\), \(L^2\), held-out nodes, spectral tails, observable differences and wall time.
- Extend the same solutions with pseudo-arclength continuation and deflation to find additional branches/folds.
- Qualify at least one nontrivial finite-coupling solution with interval residual/tail/inverse bounds and a radii-polynomial or interval-Newton existence/uniqueness theorem.

### Track 3 — arbitrary-spin 4D quadratic EFT and analytic reconstruction

This is a different theory/dimension track for fair comparison to Lam–Chung–Yunes and Li–Hu–Yunes.

Targets:
- Compare exact symbolic/analytic reconstructions on shared interaction subsets.
- Reproduce the published spin-error envelope at \(a\leq0.9\), \(0.999\), and the generic subextremal points used by the reconstruction paper.
- Add source/response boundary map, exact horizon/ADM charges, independent field-equation residuals and interval enclosures.
- Where the equations permit it, connect the exact reconstruction to a branch sensitivity atlas in coupling space.

Do not claim 5D equal-spin EGB coverage from 4D Kerr EFT results.

### Track 4 — physical principal-symbol qualification

For every finite-coupling background, separately derive the EGB principal symbol, remove gauge/constraint modes, and test characteristic completeness and uniform diagonalizability over declared exterior and covector domains. The \(O(a^4)\) uncertainty must be propagated; a hyperbolicity margin smaller than that uncertainty is inconclusive.

## 5. Numerical targets that are ambitious but meaningful

These are proposed targets, not current results.

| Measure | Reproduction target | Research stretch target |
|---|---:|---:|
| Original-row residual, per row | Match source reference; publish \(L^2,L^\infty\) | \(<10^{-40}\) high-precision residual on held-out points |
| Spectral convergence | 3 increasing resolutions | Demonstrated exponential tail and validated tail enclosure |
| Observable error | \(<10^{-8}\) across resolutions | \(<10^{-12}\), then interval-bounded observable |
| Coupling sensitivity | Exact first derivative + finite-difference convergence | Exact higher sensitivities and validated sensitivity tube |
| Global branch | Numerical pseudo-arclength | At least one interval-validated branch segment |
| Existence claim | Refined candidate | Radii-polynomial/interval-Newton local existence and uniqueness |
| Branch events | Sampled fold detection | Interval event boxes and validated local branch topology |
| Hyperbolicity | Adaptive pointwise survey | Interval coverage of the declared physical exterior |

Akribeia's \(10^{-80}\) total residual and \(10^{-85}\) Myers–Perry relative profile error are high-precision reference results, but the metrics are different. Symtropy must report per-equation unscaled residuals, not only a summed \(L^2\) norm, and must keep exact-profile error, equation residual, observable error and interval theorem as separate columns.

## 6. How to exceed the current frontier

The strongest publication target is not merely “a smaller residual”. It is:

1. A nontrivial finite-\(\lambda\), finite-spin 5D EGB solution with matched Akribeia parameters.
2. A replayable source-level action/equation derivation and independent original-row residual evaluator.
3. A boundary-data map and its first parameter derivatives, with conditioning and branch events tracked.
4. A full-domain interval enclosure and local existence/uniqueness theorem.
5. At least one validated continuation segment, including sensitivity enclosures.
6. A separate physical hyperbolicity qualification over a stated domain.

If achieved, this would combine Akribeia-style nonlinear discovery/refinement with a stronger mathematical qualification and branch-topology layer. That is a credible route to a distinct state-of-the-art result.

## References

- Agurto-Sepúlveda et al., [Neural-Spectral Discovery of Rotating Black Holes Beyond General Relativity](https://arxiv.org/abs/2607.07418v2), arXiv:2607.07418v2, revised 19 Nov 2026; [accepted PRD DOI](https://doi.org/10.1103/3kz2-6dvv).
- Lam, Chung & Yunes, [Analytic and accurate approximate metrics for black holes with arbitrary rotation in beyond-Einstein gravity using spectral methods](https://arxiv.org/abs/2510.05208), Phys. Rev. D 113, 024030 (2026), [DOI](https://doi.org/10.1103/txxb-z8bx).
- Li, Hu & Yunes, [One Method to Find Them All: Analytic Black Holes beyond Einstein](https://arxiv.org/abs/2610.37515), arXiv:2610.37515 (2026).
- Lessard & Reinhardt, [Rigorous Numerics for Nonlinear Differential Equations Using Chebyshev Series](https://doi.org/10.1137/13090883X), SIAM J. Numer. Anal. 52 (2014).
- Papallo & Reall, [On the local well-posedness of Lovelock and Horndeski theories](https://doi.org/10.1103/PhysRevD.96.044019), Phys. Rev. D 96 (2017).
