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


## 7. Literature and qualification update — 22 November 2026

This addendum records details checked against the full current papers, not only abstracts.

### Akribeia v2: separate the accuracy quantities and test branch topology

The 19 November v2 reports the following distinct measures:

- The Myers–Perry Einstein-limit control uses an aggregate \(\Delta_{\rm Total}\) made from relative \(L^2\) profile errors of the metric functions, evaluated at approximately \(10^3\) out-of-collocation points. The table reports values varying by dimension and \(\Omega_H\); for example, its 5D row spans approximately \(10^{-104}\) to \(10^{-92}\) over \(\Omega_H=0.1,\ldots,0.6\). This is a profile-error control, not a finite-coupling equation residual.
- The spectral method reports residuals around \(10^{-80}\) at \(N=200\), 300-digit arithmetic, and roughly one minute per solution. The 7D cubic Lovelock example is then extended from \(N=200\) to \(N=500\), with total four-row \(L^2\) residual of order \(10^{-80}\). Keep dimension, configuration, \(N\), arithmetic and residual definition attached to each figure.
- In the 5D \(\Omega_H=0.68\) family, the paper describes non-monotonic profile evolution as \(\alpha\) increases and notes that a given profile point \((r,f(r))\) may be crossed by more than one coupling value. This is not by itself a proof of a fold or a complete branch classification, but it makes branch identity, injectivity of the parameter-to-profile projection, pseudo-arclength and deflation important matched tests.

This suggests a stronger benchmark than residual alone: **branch projection diagnostics**. For every parameterized family, record whether selected profile/observable projections remain one-to-one, identify self-overlaps without conflating them with duplicate solutions, and verify any suspected fold/intersection through the full solution-space Jacobian and transversality conditions.

Paper sections checked: spectral refinement and computational cost, Myers–Perry error definition/table, 5D \(\Omega_H=0.68\) discussion, and 7D residual definition. The source is [arXiv:2607.07418v2](https://arxiv.org/abs/2607.07418v2), revised 19 Nov 2026 and accepted by PRD on 12 Nov 2026.

### Li–Hu–Yunes: source-to-metric reconstruction is a distinct frontier

The 27 October 2026 paper [arXiv:2610.37515](https://arxiv.org/abs/2610.37515) should be treated as a methodological comparator for a separate 4D Kerr/EFT track, not as a direct 5D equal-spin EGB benchmark.

It constructs leading-coupling metric deformations for six four-dimensional purely metric interactions (cubic, quartic and quintic) at generic subextremal spin. Its common generating-kernel construction uses sourced Teukolsky/curvature equations and a Newman–Penrose metric-reconstruction hierarchy, removing the mode decomposition used in the authors' earlier mode-by-mode solution.

A reported cross-check at \(a/M=0.9999\) compares \(\Psi_0^{(1)}\) at five polar angles and 200 logarithmically spaced radii from \(r_++10^{-6}M\) to \(100M\), against mode sums truncated at \(\ell_{\max}=100\). The maximum relative discrepancy is below \(1.4\times10^{-22}\). This is a high-accuracy sampled comparison against a distinct construction; it is not a full-domain interval bound, a generic metric error theorem, or a runtime benchmark. The paper also checks horizon-thermodynamic quantities against independent analytic predictions where available.

The paradigm opportunity for Symtropy is a **source-to-solution compiler** with multiple interchangeable mathematical backends:
- direct action-derived tensor ODE/PDE reduction;
- curvature/Teukolsky source generation;
- metric reconstruction by quadratures or exact kernels;
- independent metric and original-field-equation qualification;
- common asymptotic charge, horizon observable and branch-sensitivity interface.

The reconstructed metric must still pass the full field equations and all gauge/regularity/charge conditions. The new method is not a shortcut around verification.

### Hyperbolicity literature: keep theory identity explicit

Rossi, Gualtieri and Sotiriou, *Minimum mass, maximum charge, and hyperbolicity in scalar Gauss-Bonnet gravity*, Phys. Rev. D 114, 064042 (published 9 September 2026), [DOI](https://doi.org/10.1103/2qq5-cqzx), studies perturbation-equation hyperbolicity in scalar Gauss–Bonnet black-hole solutions. It reports a minimum-mass threshold below which perturbation equations become elliptic for the studied class of coupling functions.

This is scalar-tensor Gauss–Bonnet, not pure metric Einstein–Gauss–Bonnet. Its numerical threshold must not be transferred to the 5D EGB system. Its relevance is methodological: physical admissibility can impose a branch boundary distinct from stationary BVP existence, and that boundary should be mapped rather than inferred from curvature invariants or residuals.

## 8. Normalization-aware boundary-map metrics

The exact Einstein-limit raw Jacobian is \(J_0=\operatorname{diag}(1/2,2)\), with determinant \(1\) and raw Euclidean condition number \(4\) in the published coordinates. The new exact-rational metamorphic check establishes that under nonsingular basis changes \(\widetilde J=RJS^{-1}\), rank/nullity are invariant but determinant magnitude and Euclidean singular values are not. Output rescaling can change the same map's raw Euclidean condition number to \(1\) or \(16\).

Therefore, a branch-degeneracy detector must not rely on an unqualified Euclidean condition number. Every atlas node must report the coordinate convention and a frozen pair of positive-definite endpoint metrics (or explicitly state that no canonical metric has yet been established). Generalized singular values should be computed in those metrics, while rank/invertibility is reported separately. If a coupling-dependent endpoint basis is used, its derivative must be included in the reported \(J_1\).

See [the boundary-map normalization protocol](spin2_boundary_map_normalization_v1.md) and its exact-rational replay receipt.

## 9. Updated challenge set

Add the following to the frozen comparator suite:

1. Akribeia MP control across the published \(\Omega_H\) values, reporting profile error separately from every equation residual.
2. Akribeia 5D \(\Omega_H=0.68\) family as a branch-projection/multi-crossing diagnostic; do not label a crossing a fold until the full solution Jacobian verifies it.
3. Akribeia 7D \(N=200\) to \(N=500\) refinement as a work-precision case, with all four residual rows and the exact total norm definition.
4. Li–Hu–Yunes near-extremal \(\Psi_0\) comparison as a 4D reconstruction cross-check, with its sampled grid and mode-sum truncation preserved.
5. Scalar-GB hyperbolicity threshold as an external theory-specific control; never reuse its parameter thresholds for pure EGB.
6. Boundary-map coordinate metamorphisms: row/column rescaling must alter raw condition numbers but preserve rank, and metric-aware generalized singular values must remain invariant when the metrics are transformed consistently.

All comparison entries remain problem-specific. No Symtropy superiority or finite-coupling existence claim follows from this literature update.


## 10. Architecture shift: theory-aware solver portfolio, shared independent qualification

### Akribeia changes the seed-generation benchmark

The current arXiv v2 (19 November 2026) explicitly identifies the main practical obstacle as constructing an initial approximation close enough for Newton-type high-precision solvers to converge. It pairs parameterized PINN discovery with pseudo-spectral Gauss--Newton refinement and reports continuous global neural fields with residuals checked to extreme precision. This means the honest competitor to Akribeia is not a standalone spectral solver; it is a seed-generation and branch-discovery strategy that can robustly find branches in the absence of hand-built perturbative seeds.

Symtropy's opportunity is to exploit exact action reductions, the known \(\lambda=0\) branch and exact sensitivities as one seed family, then combine it with deflation, neural fields or data-driven predictors only as additional discovery proposals. All candidates must pass the same original-equation evaluator, fixed boundary convention, charge normalization and branch-identity contract.

Reference: [Agurto-Sepúlveda et al., arXiv:2607.07418v2](https://arxiv.org/abs/2607.07418v2); accepted PRD DOI [10.1103/3kz2-6dvv](https://doi.org/10.1103/3kz2-6dvv).

### Source-to-metric reconstruction is a second, separate backend

Li, Hu and Yunes (arXiv:2610.37515, 27 October 2026) show a distinct approach for *perturbative four-dimensional Kerr deformations*: solve the sourced stationary Teukolsky equation for a Weyl scalar, then use a universal Newman--Penrose metric-reconstruction hierarchy, source quadratures and charge/gauge completion. The paper's reconstruction has the structure \(h_{ab}^{(1)}=\mathcal R_{ab}[\mathcal P]+\mathcal Q_{ab}[S]+h_{ab}^{\rm comp}\), with a radial curvature potential \(\mathcal P(r,\theta)=\frac16\int_r^\infty (u-r)^3\Psi_0^{(1)}(u,\theta)\,du\). It demonstrates six cubic, quartic and quintic purely metric interactions at generic subextremal spin, including a construction without mode decomposition.

This is not a direct comparison to five-dimensional finite-coupling EGB: it is linear in a small perturbation/coupling around 4D Kerr. Its strategic value is architectural. Symtropy should implement multiple theory-class backends behind common contracts:
- 5D nonlinear EGB: action-derived cohomogeneity-one BVP, global matching, and pseudo-arclength/deflation.
- 4D perturbative Kerr EFT: conserved source -> stationary Teukolsky curvature -> universal metric reconstruction -> original linearized-equation verification.
- Future higher-dimensional/stationary PDE cases: spectral or neural discovery feeding the same independent qualification layer.

The reconstruction backend must independently check source conservation, tetrad conventions, homogeneous charge completion, full original field equations and horizon/asymptotic observables. It must not certify its own metric.

Reference: [Li, Hu & Yunes, arXiv:2610.37515](https://arxiv.org/abs/2610.37515); technical derivation in [the full paper](https://borowiecki.dev/pdf/2610.37515).

### Hyperbolicity is formulation- and regime-dependent

Do not treat “EGB/Lovelock is hyperbolic” as a property established by the existence of a stationary background alone. Papallo--Reall's analysis finds that generalized harmonic gauge is not strongly hyperbolic on a generic weak-field Lovelock background; subsequent modified-harmonic formulations establish strong hyperbolicity in a controlled weak-coupling regime. These results are compatible because hyperbolicity depends on the chosen formulation, gauge and background regime.

Therefore the rotating-EGB principal-symbol programme must:
1. state the exact gauge-fixed first-order or second-order formulation;
2. separate gauge, constraint and physical characteristic sectors;
3. test completeness and uniform diagonalizability of the principal symbol over the declared exterior/covector domain;
4. bound the smallest eigenvalue/eigenvector condition margin with interval or validated numerics where possible;
5. compare more than one admissible formulation rather than transfer a theorem from a different gauge;
6. abstain if the hyperbolicity margin is smaller than background/perturbative truncation uncertainty.

References: [Papallo & Reall, Phys. Rev. D 96, 044019 (2017)](https://doi.org/10.1103/PhysRevD.96.044019); [Kovács & Reall, modified harmonic formulations for Lovelock/Horndeski](https://arxiv.org/abs/2003.08398) (verify exact theorem hypotheses before reuse).

### Updated programme consequence

The route to a distinct state-of-the-art result is the conjunction of:
- Akribeia-class discovery breadth and finite-coupling nonlinear solutions;
- action-first equations and exact perturbative sensitivities;
- normalization-aware global boundary maps and branch topology;
- independent full-domain residual, tail and inverse bounds;
- local existence/uniqueness certificates for selected finite-coupling solutions;
- formulation-specific physical hyperbolicity/EFT validity;
- a second source-to-metric backend for 4D Kerr perturbative theories.

A smaller residual alone does not establish superiority. Compare identical equations, couplings, gauge, boundary conditions, resolution and arithmetic; then report per-row residuals, held-out residuals, observable errors, branch coverage, conditioning, wall time and memory. Separately report whether the candidate has only a small residual, a validated enclosure, or a theorem-backed local existence result.


## 11. 22 November 2026 literature audit: matched comparators and metric boundaries

### Akribeia is the direct nonlinear 5D EGB comparator

The latest public record is v2 revised 19 November 2026; the paper was accepted by Physical Review D on 12 November 2026 (DOI 10.1103/3kz2-6dvv). The supplementary material gives useful matched details:

- Its 5D EGB demonstration fixes \(r_H=1\), \(\Omega_H=0.33\), and trains a parametric family across \(\alpha\in[0,3]\) in increments of 0.01. It also reproduces earlier 5D EGB data at \(\alpha=2,\Omega_H=0.68\), with a coupling sweep at that spin.
- The discovery phase is a PINN from random initialization; the refinement phase is pseudo-spectral Gauss--Newton with Chebyshev representation and high-precision residual evaluation.
- The Myers--Perry GR-limit control is separate: the paper reports a relative \(L^2\) error of \(10^{-85}\) using 300-digit arithmetic. Its Table I separately lists dimension/spin-dependent MP control errors for three random initializations.
- The \(O(10^{-80})\) total residual after \(N=500\) collocation points is reported for the more involved **7D cubic Lovelock** example, not as the metric for the 5D EGB family. It must not be placed in a single column with the MP relative \(L^2\) profile error or the 5D EGB pointwise equation residual.
- Akribeia reports a non-monotonic 5D EGB profile at \(\Omega_H=0.68\): the same \((r,f(r))\) can be crossed for multiple coupling values. This motivates the Symtropy branch-identity rule: compare the complete solution vector and continuation tangent, not a single radial profile.

The fair Symtropy target is therefore not simply a smaller nominal residual. It is a matched reproduction of the \((r_H,\Omega_H,\alpha)\) family, plus robust branch identity, equation-row/held-out residuals, independent sensitivity validation, and eventually a validated existence tube. The seed/discovery algorithms should be compared at equal compute budgets and repeated random seeds.

Primary records:
- [Akribeia arXiv v2](https://arxiv.org/abs/2607.07418v2)
- [Physical Review D accepted paper, accepted 12 November 2026](https://doi.org/10.1103/3kz2-6dvv)

### Ma--Lü is a direct first-coupling 5D Myers--Perry overlap, but not automatically the same action

Ma and Lü, *Quadratic curvature correction to 5D Myers--Perry metric*, JHEP 05 (2026) 081, arXiv:2512.23797, study \(R+\alpha_R R_{\mu\nu\rho\sigma}R^{\mu\nu\rho\sigma}\) at linear coupling about the general two-spin 5D MP background. They expand in \(\chi_a=a/\sqrt{\mu}\), \(\chi_b=b/\sqrt{\mu}\) through tenth order and compare thermodynamic corrections against Reall--Santos.

This is an unusually valuable independent overlap for Symtropy's \(\lambda a^2\) tangent. The Symtropy reduced action uses \(R+\alpha_{GB}\mathcal L_{GB}/4\). On Ricci-flat MP, the Ricci-squared and scalar-squared pieces in \(\mathcal L_{GB}=Riemann^2-4Ricci^2+R^2\) vanish as background tensors, but the equality of the *linearized source tensors* must still be proved, including the double-derivative terms and conventions. The candidate normalization \(\alpha_R=\alpha_{GB}/4\) is therefore a hypothesis to derive, not a settled conversion.

For equal spins, the background horizon relation gives \(\mu=(r_H^2+a^2)^2/r_H^2\), hence \(\chi=a r_H/(r_H^2+a^2)=\Omega_H r_H\). Under Symtropy's \(\Omega_H=a_S/r_H^2\) convention, the dimensionless slow-spin parameter is \(a_S/r_H=\chi\). The exact map must additionally transform Ma--Lü's cohomogeneity-two coordinates and conformally-flat orbit-space gauge to Symtropy's radial gauge before component comparison.

A dedicated matched-comparison task is open as [#6915](https://github.com/Luminous-Dynamics/symtropy/issues/6915). No equality of the published coefficients is claimed until that audit passes.

Primary records:
- [Ma--Lü arXiv:2512.23797](https://arxiv.org/abs/2512.23797)
- [JHEP version of record, 8 May 2026](https://doi.org/10.1007/JHEP05(2026)081)

### Teukolsky reconstruction benchmark: strong method, carefully bounded reported comparison

Li, Hu and Yunes, arXiv:2610.37515 (27 October 2026), provide analytic leading-coupling deformations for six higher-curvature interactions at generic subextremal Kerr spin without mode decomposition. Their mode-free \(\Psi_0\) is compared with a separate mode-sum construction at \(a/M=0.9999\): five polar angles and 200 logarithmically spaced radii, with the mode sum truncated at \(\ell_{\max}=100\), yielding a maximum relative discrepancy below \(1.4\times10^{-22}\). They also report exact agreement with available analytic first-order horizon thermodynamic predictions.

Interpretation: this is a strong matched sampled curvature and thermodynamic validation. It is not a runtime benchmark or a general full-domain error bound. The paper's auxiliary equation can be transformed to a flat-space Poisson operator in prolate coordinates, which is an important reusable mathematical reduction for the proposed 4D backend.

Primary record: [Li--Hu--Yunes, arXiv:2610.37515](https://arxiv.org/abs/2610.37515).

### Hyperbolicity is a separate benchmark axis

Static Lovelock characteristic cones, gauge-fixed strong hyperbolicity, linear stability and EFT validity are different properties. Kovács--Reall (2020) demonstrate a modified-harmonic formulation that is strongly hyperbolic at weak coupling under its hypotheses; this cannot be transferred without checking the exact formulation and background regime. Keep the existing rotating 5D principal-symbol work in [#6334](https://github.com/Luminous-Dynamics/symtropy/issues/6334) independent of the stationary BVP.

### Revised comparison rule

Every public SOTA table must identify, for each numeric result: theory/action, dimension, background/branch, spin and coupling, perturbative order, gauge/normalization, arithmetic, resolution, whether it is a pointwise residual, global norm, profile error, observable error or interval bound, and whether it is independently verified. Values from different metric classes must not be ranked as if they were the same score.


## 12. Ma--Lü parameter-map correction (23 November 2026)

The prior shorthand in Section 10 that wrote chi=a/sqrt(mu) and chi=Omega_H*r_h is superseded. The current paper's Eq. (16) literally prints chi=a/mu; Eqs. (4), (6) give for equal spins q=a/r_h, nu=Omega_H*r_h:
\[
\mu/r_h^2=(1+q^2)^2/2,\quad
\nu=q/(1+q^2),\quad
\chi r_h=2q/(1+q^2)^2.
\]
The small-root inverse is q=nu+nu^3+2nu^5+5nu^7+...; thus only the leading O(nu^2) spin-squared metric overlap uses q^2=nu^2+O(nu^4). The printed chi-series normalization is distinct and must be carried explicitly. The manuscript's Eq. (50) gives the equal-spin consistency expansion mu=r_h^2/2+r_h^4*chi^2/4+9*r_h^6*chi^4/32+...; the dimensional convention in Eq. (16) still warrants careful explanation.

The parameter-map script has now been fetched byte-for-byte from the public branch and locally executed with Python 3.13.5 / SymPy 1.14.0. Public Git blob SHA-1 5d17da9c52e0668d699d591c608a5bc18eb98efc; local SHA-256 dd04510d0cbe3d79003a57b54e4362b0bda98bb1e8652772cef83d0d454c6ac3. All exact symbolic checks pass. See issue #6915 and the parameter-map artifacts in the benchmark directory.
