# Matched-comparison addendum: Symtropy spin-2 versus Akribeia

**Status:** protocol correction and benchmark design. This file reports published comparator facts and fixes a problem-class mismatch in the current matched-SOTA protocol. It contains no Symtropy-versus-Akribeia benchmark result.

## 1. What Akribeia has already demonstrated

The primary comparator is now a peer-reviewed / accepted publication, not merely a proposed method:

- Agurto-Sepúlveda et al., *Neural-Spectral Discovery of Rotating Black Holes Beyond General Relativity*, accepted in **Physical Review D on 12 November 2026**, DOI [10.1103/3kz2-6dvv](https://doi.org/10.1103/3kz2-6dvv); preprint [arXiv:2607.07418](https://arxiv.org/abs/2607.07418).
- Akribeia is a two-stage pipeline: a PINN discovers a continuous approximate family from random initialization; a Chebyshev pseudo-spectral Gauss–Newton stage refines a selected parameter point with extended precision.
- The paper reports 5D EGB profiles at fixed \(r_H=1\), \(\Omega_H=0.33\), with \(\alpha\in[0,3]\) in steps of \(0.01\), and another family with fixed conserved charges. It also reports a separate \(\Omega_H=0.68\) family over a reduced coupling interval.
- For the 5D EGB configuration, the reported neural architecture is four fully connected networks of shape \([2,20,20,20,1]\). The appendix reports 60,000 Adam-stage steps, 6,000 initial collocation points, 5,000 L-BFGS iterations with 16,000 points, and a Chebyshev refinement sequence \(N=20,20,50,100,200\) at 300-digit working precision.
- Reported machine-specific costs are approximately 15–120 minutes for neural construction of the 5D EGB family and about one minute per solution for the final pseudo-spectral refinement on a single CPU core.
- The paper reports a relative \(L^2\) profile error of \(10^{-85}\) for the Myers–Perry recovery in the Einstein limit using 300-digit arithmetic.

These are important baselines. The \(10^{-85}\) number is specifically a relative profile error on the Myers–Perry control; it is not a universal finite-coupling field-equation residual target. The paper's use of “certified” is tied in its exposition to continuous profiles with residuals verified below a quantified threshold. That should be distinguished from a separate interval-Newton / radii-polynomial proof of a nearby exact solution.

## 2. The current Symtropy problem is not yet a direct full-spin match

The current Symtropy branch derives the linear, inhomogeneous \(O(a^2)\) slow-rotation system around the static Einstein-branch EGB background. Its even metric fields have the form
\[
b=F+a^2B_2+O(a^4),\qquad
f=F+a^2F_2+O(a^4),\qquad
h=r^2+a^2H_2+O(a^4),
\]
and \(w=aW_1+O(a^3)\).

Akribeia's published \(r_H=1,\Omega_H=0.33\) profiles solve the **full nonlinear stationary rotating problem**, not the \(O(a^2)\) truncated equations. Comparing the current perturbative profiles directly with the full \(\Omega_H=0.33\) solution would mix numerical error with the unknown \(O(a^4)\) truncation error.

This distinction does not invalidate the Akribeia comparator. It changes the correct matching experiment.

## 3. Two valid comparison tracks

### Track A — perturbative coefficient extraction

For each frozen coupling \(\lambda=\alpha/r_H^2\), obtain full Akribeia solutions at several sufficiently small horizon angular velocities, at fixed \(r_H\), coupling convention, radial gauge, and horizon normalization. The paper's public \(\Omega_H=0.33\) family is a useful full-spin reference, but it is not by itself the small-spin coefficient test.

In the Symtropy convention \(w(r_H)=\Omega_H=a/r_H^2\), hence \(a=r_H^2\Omega_H\). From full-spin fields extract
\[
B_2^{\rm ext}(r;\Omega_H)
=\frac{b_{\rm full}(r;\Omega_H)-F(r)}{a^2},
\]
\[
F_2^{\rm ext}(r;\Omega_H)
=\frac{f_{\rm full}(r;\Omega_H)-F(r)}{a^2},
\qquad
H_2^{\rm ext}(r;\Omega_H)
=\frac{h_{\rm full}(r;\Omega_H)-r^2}{a^2},
\]
and
\[
W_1^{\rm ext}(r;\Omega_H)=\frac{w_{\rm full}(r;\Omega_H)}{a}.
\]

Then test convergence as \(\Omega_H\to0\), using at least three decreasing spin values and an even-power fit. The even-field extraction should approach a common \(O(a^2)\) coefficient with a leading discrepancy consistent with \(O(a^2)\) in the divided coefficient (equivalently \(O(a^4)\) in the unscaled field). The odd field should approach \(W_1\) with the corresponding \(O(a^2)\) correction after division by \(a\).

Do not subtract profiles or compare coefficients until the mass/charge ensemble, horizon gauge, radial coordinate, normalization, and \(\alpha\)-dictionary are aligned.

### Track B — full nonlinear extension

Build a separate full stationary Symtropy solver and compare directly against Akribeia at its published finite-spin points, including \((r_H,\Omega_H)=(1,0.33)\) and the additional \(\Omega_H=0.68\) family where conventions are matched.

This is a larger programme. It is not a prerequisite for validating the existing slow-rotation equations, and it must not be reported as achieved by the current two-field \(O(a^2)\) solver.

## 4. Freeze the convention dictionary before either track

The Akribeia paper defines its EGB coupling as \(\alpha=4\alpha_2/\alpha_1\). The Symtropy equations use \(\lambda=\alpha/r_H^2\) after dimensionless reduction. Treat this as a candidate dictionary and verify it at the action level and against the static Boulware–Deser control before matching numerical values.

Record separately:
- action normalization and sign of the Gauss–Bonnet term;
- dimensionful \(\alpha\) and dimensionless \(\lambda=\alpha/r_H^2\);
- \(r_H\), \(\Omega_H\), and \(a=r_H^2\Omega_H\);
- fixed-\((r_H,\Omega_H)\) versus fixed-\((E,J)\) ensemble;
- radial coordinate, compactification, and metric-field normalization.

The paper's two EGB figures use different ensembles. A point on the fixed-\((E,J)\) family must not be compared with a fixed-\((r_H,\Omega_H)\) Symtropy run without the corresponding parameter-family transformation.

## 5. Use the public implementation, not only a lookalike baseline

The paper points to these tagged reproducibility repositories:

- [Akribeia v1.0.0](https://github.com/moyarzoca/Akribeia/tree/v1.0.0)
- [Explicit rotating Lovelock equations v1.0.0](https://github.com/moyarzoca/rotating-equal-momenta-lovelock/tree/v1.0.0)
- [DiffLearning v2.1.1](https://github.com/moyarzoca/difflearning/tree/v2.1.1)
- [Deterministic Gauss–Newton solver v1.1.1](https://github.com/moyarzoca/deterministic-gauss-newton-wl/tree/v1.1.1)

Pin immutable commit digests for the actual run. The existing “Akribeia-style baseline” remains useful as a method ablation, but it is not a reproduction of the authors' implementation.

## 6. Revised benchmark scorecard

Keep the two leaderboards independent.

| Discovery / refinement | Qualification / proof |
|---|---|
| Time to first candidate | Exact action/equation provenance |
| Time to source-matched residual | Independent original-equation residuals |
| Profile and observable error | Noether and radial-constraint defect |
| Success rate over fixed initializations | Horizon and infinity mode counts |
| Work-precision and memory curves | Boundary-map rank / Fredholm index |
| Coupling and spin coverage | Full-domain interval residual and tail |
| Branch/fold/disconnected-branch coverage | Radii-polynomial existence and local uniqueness |
| Failed starts, false candidates, timeouts | Validated continuation and event enclosures |
| Energy use, hardware, wall time | Independent PDE hyperbolicity status |

For fair discovery comparisons, freeze:
1. exact equations, conventions, boundary conditions, and ensemble;
2. hardware, arithmetic, wall time, memory, and initialization budget;
3. collocation data, refinement schedule, stopping rules, and rejected-run policy;
4. independent held-out radial nodes and all original field equations.

Report the Akribeia settings above as published reference points, not universal targets. In particular, do not compare a 300-digit final result with binary64 work and call the difference an algorithmic win; report precision and resource budgets together.

## 7. How Symtropy can make a stronger contribution

The defensible differentiation is not simply “more digits” or “faster neural training”:

1. **Equation-aware reduction:** use the exact algebraic \(E_f\) pivot and Noether identity to reduce the solve dimension while retaining all original equations as independent residual/constraint checks.
2. **Endpoint structure:** use exact horizon row valuations, principal rank, indicial exponents, and coupled Frobenius recurrences to remove endpoint stiffness rather than letting a network learn it implicitly.
3. **Global branch topology:** combine pseudo-arclength continuation and deflation with horizon/infinity mode counts and explicit fold/bifurcation events. BifurcationKit documents pseudo-arclength continuation and deflated continuation for disconnected diagrams; these are strong classical baselines, not claims of novelty.
4. **Validated answers:** target a full-domain interval residual, spectral-tail and inverse bounds, then a radii-polynomial existence/local-uniqueness certificate. High precision and tiny residuals are necessary evidence, not substitutes for this proof tier.
5. **Physical admissibility:** independently qualify the full EGB principal symbol and a stated gauge-fixed evolution formulation. Neither a stationary BVP pass nor a validated radial profile implies strong hyperbolicity.

## 8. Acceptance gates

- **M0 — convention lock:** exact action dictionary and MP / Boulware–Deser controls.
- **M1 — source reproduction:** reproduce an Akribeia published case using its tagged code and identical physical ensemble.
- **M2 — perturbative match:** extract the \(O(a^2)\) coefficient from at least three decreasing-spin full solutions and verify the expected truncation scaling.
- **M3 — independent refinement:** match the published solver accuracy under declared precision and resource budgets; report every original equation separately.
- **M4 — branch coverage:** continuation/deflation with explicit, independently checked fold and disconnected-branch records.
- **M5 — validated solution:** full-domain interval enclosure and radii-polynomial or equivalent proof.
- **M6 — physical admissibility:** full-principal-symbol and strong-hyperbolicity qualification over a declared background/domain/gauge.

Passing M0–M4 does not imply M5 or M6. A complete branch catalogue additionally requires a completeness argument; successful continuation and deflation alone only establish discovered branches.

## Sources

- Agurto-Sepúlveda et al., *Neural-Spectral Discovery of Rotating Black Holes Beyond General Relativity*, accepted PRD 12 Nov 2026, [DOI](https://doi.org/10.1103/3kz2-6dvv), [arXiv:2607.07418](https://arxiv.org/abs/2607.07418). The main text and appendix document the model family, training/refinement schedule, Myers–Perry accuracy, machine-specific costs, and source repositories.
- Lessard and Reinhardt, *Rigorous Numerics for Nonlinear Differential Equations Using Chebyshev Series*, SIAM J. Numer. Anal. 52 (2014), [DOI](https://doi.org/10.1137/13090883X). The paper formulates Chebyshev solutions as fixed points in a Banach space and uses radii polynomials to establish local existence and uniqueness.
- [BifurcationKit pseudo-arclength continuation](https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/PALC/) and [deflated continuation](https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/DeflatedContinuation/).
- Papallo and Reall, *On the local well-posedness of Lovelock and Horndeski theories*, [DOI](https://doi.org/10.1103/PhysRevD.96.044019); Kovács and Reall, *Well-posed formulation of Lovelock and Horndeski theories*, [DOI](https://doi.org/10.1103/PhysRevD.101.124003).