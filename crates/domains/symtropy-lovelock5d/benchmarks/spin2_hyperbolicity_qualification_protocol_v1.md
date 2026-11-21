# Physical hyperbolicity qualification protocol for 5D EGB rotating solutions

**Status:** prospective qualification protocol. No characteristic polynomial, hyperbolicity region, or strong-hyperbolicity result for the current spin-squared EGB branch is claimed.

## 1. Keep three notions separate

The programme must not conflate:

1. **Horizon regularity of the radial ODE/BVP.** The scaled ordinary differential equations extend analytically through the non-extremal horizon.
2. **Well-posedness of the stationary boundary-value problem.** This requires the reduced ODE operator, endpoint mode counts, and boundary-data map to be analyzed.
3. **Hyperbolicity of the full gravitational PDE.** This concerns the principal symbol of the time-dependent EGB field equations and whether a gauge-fixed evolution problem admits a well-posed initial-value formulation.

Passing either of the first two does not imply the third. A stationary solution can be regular as an ODE profile and still lie in a region where the gravitational initial-value equations are not strongly hyperbolic.

## 2. Mathematical object to qualify

For a candidate background \(g_{ab}(\lambda,\epsilon)\), compute the principal symbol of the linearized EGB equations. In geometric optics, use a perturbation
\[
h_{ab}=a_{ab}\exp(i\theta/\varepsilon),\qquad
\xi_a=\nabla_a\theta,
\]
and retain the highest-derivative terms as \(\varepsilon\to0\):
\[
\delta E_{ab}\big|_{\mathrm{principal}}
=\mathcal P_{ab}{}^{cd}(\xi;g,R,\alpha)\,a_{cd}.
\]

The physical characteristic set is obtained only after accounting for diffeomorphism gauge directions and constraints. Do not infer it from the determinant of the unreduced tensor symbol, which contains gauge degeneracies.

Where equal-angular-momentum symmetry gives a valid decomposition into tensor-, vector-, and scalar-type perturbations, compute each sector separately and verify that the decomposition remains complete at symmetry axes, the horizon, and any sector-degeneracy points. Otherwise analyze the coupled symbol.

## 3. Gauge and strong-hyperbolicity test

A strong-hyperbolicity claim requires a specified gauge-fixed, first-order-in-time formulation and a specified reduction. For every point in the exterior and every nonzero spatial covector:

- the principal evolution symbol has real characteristic speeds;
- it has a complete set of eigenvectors, or a uniformly positive symmetrizer;
- the eigenbasis/symmetrizer remains uniformly controlled over the declared spatial-covector directions and background domain;
- constraint and gauge modes are separated from physical polarizations;
- the result is stable under a regular change of frame and regular horizon-crossing coordinates.

Real roots alone are insufficient. A defective eigenbasis, an unbounded condition number, or a vanishing symmetrizer margin is a failure or an inconclusive result, not a pass.

The gauge must be stated explicitly. Standard harmonic gauge cannot be assumed strongly hyperbolic for generic Lovelock backgrounds. Modified harmonic formulations provide relevant alternatives in weak-coupling regimes, but their assumptions and coupling domain must be checked for each rotating background.

## 4. Domain and parameter protocol

Begin with exact and limiting backgrounds:

- \(\alpha=0\): equal-spin Myers–Perry / Einstein-gravity control;
- zero spin: static Einstein-branch Boulware–Deser control;
- the non-extremal exterior of each candidate spin-squared solution.

Then sweep a frozen coupling grid, initially
\[
\lambda\in\{0,0.1,0.7,2,3,10\},
\]
and a spin grid that includes the slow-rotation regime and later full nonlinear solutions. The coupling grid is a test design, not a claim that every point admits a rotating solution.

Use horizon-regular frames and compactified asymptotic coordinates, with overlapping charts. Sample radial and angular/covector directions adaptively near the horizon, at extrema of curvature invariants, and near apparent characteristic degeneracies. A finite point sample is discovery evidence, not a proof over the exterior.

For the spin-squared approximation, retain an explicit truncation model:
\[
g_{ab}=g^{(0)}_{ab}+\epsilon g^{(1)}_{ab}+\epsilon^2g^{(2)}_{ab}+O(\epsilon^3)
\]
with parity restrictions as appropriate. If the hyperbolicity margin is comparable to the uncontrolled higher-order remainder, classify the result as unresolved. A second-order background alone cannot certify a finite-spin nonlinear branch.

## 5. Required outputs

For every background/coupling/spin point, record:

1. action, convention, gauge, coordinate-chart, and background digests;
2. principal-symbol construction and physical polarization decomposition;
3. characteristic speeds/cones by polarization;
4. minimum real-root separation and minimum eigenbasis/symmetrizer margin;
5. maximum eigenbasis condition estimate;
6. horizon and asymptotic chart consistency;
7. constraint/gauge characteristic separation;
8. parameter intervals where the symbol is degenerate or hyperbolicity is lost;
9. interval bounds or a clear sampled-only designation;
10. exact code, precision, compiler, and replay receipt.

Report both metric-cone comparisons and polarization-specific characteristic cones. Do not describe a cone outside the metric cone as acausal without specifying the relevant causal structure and background assumptions.

## 6. Controls and failure injection

Required controls:

- Einstein limit: recover the GR principal symbol and its physical polarization count.
- Static Boulware–Deser: reproduce known characteristic-sector structure where available.
- Gauge mutation: verify that pure-gauge amplitudes are not counted as physical modes.
- Constraint mutation: detect a mode that violates the linearized constraints.
- Coordinate mutation: compare regular horizon-crossing charts and reject coordinate-induced false degeneracies.
- Sign and normalization mutation: flip the Gauss–Bonnet convention or coupling sign and ensure the fixture changes or fails.
- Polarization mutation: deliberately omit a sector and ensure the total mode count detects it.
- Near-degeneracy: test a defective/ill-conditioned symbol and require fail or inconclusive, never automatic acceptance.

A negative control that is merely logged but does not change the qualification outcome is not a functioning control.

## 7. Acceptance ladder

- **H0 — construction:** principal-symbol expressions independently derived from the EGB action.
- **H1 — pointwise numerical:** physical characteristic roots and eigenstructure computed at named points.
- **H2 — domain survey:** adaptive exterior/covector sampling with all controls.
- **H3 — validated local:** interval-enclosed roots and symmetrizer/eigenbasis bounds on declared spacetime boxes.
- **H4 — validated exterior:** overlapping validated boxes cover the full declared exterior and parameter interval.
- **H5 — formulation-level well-posedness:** the chosen gauge-fixed evolution system is shown strongly hyperbolic on the declared domain.

Do not promote H1/H2 to a theorem. A physical admissibility claim must identify its exact level and domain.

## 8. Literature and methodological boundaries

- Reall, Tanahashi and Way, *Causality and Hyperbolicity of Lovelock Theories*, Classical and Quantum Gravity (2014), https://arxiv.org/abs/1406.3379, DOI https://doi.org/10.1088/0264-9381/31/20/205005. This analyzes Lovelock characteristic structure and examples of hyperbolicity failure.
- Papallo and Reall, *On the local well-posedness of Lovelock and Horndeski theories*, Physical Review D (2017), https://arxiv.org/abs/1705.04370, DOI https://doi.org/10.1103/PhysRevD.96.044019. Standard harmonic-gauge formulations need not be strongly hyperbolic for generic Lovelock backgrounds.
- Kovács and Reall, *Well-posed formulation of Lovelock and Horndeski theories*, Physical Review D (2020), https://arxiv.org/abs/2003.08398, DOI https://doi.org/10.1103/PhysRevD.101.124003. Modified harmonic formulations provide a relevant route in a controlled weak-coupling regime.

These papers motivate the test architecture; none supplies a hyperbolicity conclusion for the rotating 5D EGB family in this repository.

## 9. Claim ceiling

This protocol specifies how to qualify physical hyperbolicity. It does not calculate the EGB principal symbol, prove strong hyperbolicity, identify a loss-of-hyperbolicity threshold, or establish stability/causality for the current spin-squared branch. Those remain research tasks independent of radial ODE regularity and BVP well-posedness.
