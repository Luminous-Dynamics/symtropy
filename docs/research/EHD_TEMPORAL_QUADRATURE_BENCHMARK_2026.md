# EHD Temporal Quadrature Exactness for Elastic Collision Sources (2026)

**Benchmark:** EHD-QUADRATURE-001  
**Status:** analytic quadratic manufactured oracle and authored tests; Rust execution pending  
**As of:** 21 November 2026  
**Tracking:** [#6643](https://github.com/Luminous-Dynamics/symtropy/issues/6643)  
**Machine-readable form:** [ehd_temporal_quadrature_v0_1.json](../../data/research/ehd_temporal_quadrature_v0_1.json)

## 1. Why a separate quadrature benchmark is needed

The time-varying drag receipt in `drag_trajectory.rs` verifies that an event's reported impulse and elastic frictional heating agree with the same supplied space-time quadrature samples. That is an important consistency invariant, but it does not establish that the selected quadrature rule resolves the underlying continuous source.

For non-negative time-dependent collision-rate coefficient \(k(t)\) and relative drift \(\Delta\mathbf u(t)\):

\[
\mathbf I_n=\int_0^1 k(t)\Delta\mathbf u(t)\,dt,
\qquad
Q_{\rm fric}=\int_0^1 k(t)|\Delta\mathbf u(t)|^2\,dt.
\]

The heating integrand is quadratic in drift. It can have a higher polynomial degree than the impulse integrand. A quadrature can therefore pass an impulse check while failing the heating check.

The benchmark treats these as independent accuracy results.

## 2. Implemented reference oracle

The pure-Rust module is:

`tools/ehd-reference-audit/src/drag_quadrature.rs`

It uses manufactured quadratic polynomials on normalized time \(t\in[0,1]\):
- \(k(t)=k_0+k_1t+k_2t^2\), constrained to be non-negative throughout the interval;
- each of three relative-velocity components is independently quadratic.

It constructs the impulse polynomial \(k(t)\Delta u_i(t)\) and heating polynomial \(k(t)\sum_i\Delta u_i(t)^2\), computes exact antiderivative integrals, and compares them against the selected quadrature.

For this class:
- impulse degree is at most 4;
- frictional-heating degree is at most 6;
- four-point Gauss–Legendre is algebraically exact through degree 7 and provides the known-answer reference (up to floating-point arithmetic);
- three-point Gauss–Legendre is exact through degree 5, so selected degree-six heating fixtures must fail;
- two-point Gauss–Legendre is exact through degree 3, allowing a useful adversary where the degree-two impulse is exact but degree-four heating is not.

The module also includes Simpson's three-point rule for comparison.

## 3. Exactness and adversarial fixtures

The authored tests include:
1. Four-point Gauss–Legendre exactly integrating quadratic drag-rate and quadratic drift, including degree-six heating.
2. A Gauss–Legendre two-point case where impulse passes but heating fails.
3. A degree-six heating case that fails three-point Gauss–Legendre and passes four-point Gauss–Legendre.
4. A low-degree case exactly integrated by Simpson's rule.
5. A drag-rate polynomial that is positive at both endpoints but negative inside the interval, which must be rejected.
6. Non-finite polynomial coefficient rejection.
7. Invalid tolerance rejection.
8. Offline reference self-check.

These are authored Rust tests, not executed Rust tests.

## 4. Independent physical/numerical boundaries

This temporal oracle assumes the spatial integration of \(m_s n_s\nu_{sn}\) has already been collapsed into \(k(t)\), with units kg/s. It does not qualify spatial quadrature, the collision frequency model, the chemistry, or the thermal partition.

The quadrature residual must remain separate from:
- spatial mesh mapping and overlap error;
- moving-mesh GCL;
- TPE(1)/TPE(2) transport exactness;
- event-level equal-and-opposite impulse residual;
- species charge/mass/momentum/energy closure;
- physical force-balance and control-volume uncertainty.

## 5. Proposed production-solver qualification

The manufactured oracle is a lower-cost regression suite. A production solver adapter must additionally demonstrate:
- temporal order refinement for both impulse and heating;
- spatial quadrature refinement for both;
- joint space-time refinement;
- independent subcycling/partition invariance;
- steep collision-rate and drift-gradient stress cases;
- non-polynomial smooth histories;
- convergence or a defensible error bound on held-out source histories;
- a receipt recording quadrature rule, nodes/weights, time/space partition, coefficient provenance and separate impulse/heating refinement residuals.

The solver must not select a quadrature tolerance after observing a discrepancy. Case-specific tolerances and refinement ladders are frozen before held-out evaluation.

## 6. Competitive benchmark

Compare the same collision source history and source-space-time measure using:
- 1-, 2-, 3- and 4-point Gauss–Legendre;
- Simpson's rule;
- the solver's production adaptive or collocation rule;
- an independently refined high-order reference.

Report separately:
- impulse error;
- frictional-heating error;
- cost per event / memory / communication;
- observed convergence rate;
- robustness under sharp gradients and high collision rates;
- total coupled-solver cost at matched impulse **and** heating accuracy.

A fast rule that accurately transfers impulse but under-resolves heating is not a winner.

## 7. Claim ceiling

This is a temporal quadrature exactness oracle for spatially integrated quadratic manufactured sources. It does not implement spatial collision quadrature, certify arbitrary production solver histories, select an adaptive solver quadrature, solve plasma/neutral equations, establish complete collisional energy closure, validate physical thrust or demonstrate SOTA superiority.
