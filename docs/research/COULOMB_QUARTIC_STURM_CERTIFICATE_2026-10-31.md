# Exact Sturm certificate for the coupled Coulomb quartic

**Status:** independent test-only exact-arithmetic verifier; exact-head Rust validation pending.  
**Parent:** [#3168 — coupled 3D Signorini–Coulomb point contact](https://github.com/Luminous-Dynamics/symtropy/pull/3168).  
**Research issue:** [#3169 — certify contact quartic root completeness](https://github.com/Luminous-Dynamics/symtropy/issues/3169).  
**Qualification authority:** [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Why this increment exists

The parent solver reduces the sliding branch to a quartic and validates every candidate against the original contact equations. Its current root enumerator is a floating-point recursive derivative-partition method. That is useful for a bounded reference solver, but it cannot itself prove that every real root has been found.

This increment adds an independent oracle using exact rational arithmetic. It is test-only and deliberately does not change the runtime result enum or claim that every runtime call has a formal uniqueness proof.

## Exact input semantics

A finite binary64 value is interpreted as the exact dyadic rational represented by its IEEE-754 bit pattern. The verifier reconstructs the sliding quartic coefficients over the rationals from those exact input values. It therefore audits the exact polynomial associated with the declared floating-point inputs, rather than an arbitrary higher-precision decimal reinterpretation.

The quartic is reconstructed independently from the angular equation and is not fetched from the runtime solver's coefficient helper.

## Root-count and interval algorithm

1. Trim exact zero high-order coefficients.
2. Compute the polynomial greatest common divisor with its derivative by exact Euclidean division, then divide out repeated factors.
3. Construct the Sturm chain of the square-free polynomial.
4. Count distinct finite real roots exactly as the sign-variation difference at negative and positive infinity.
5. Use an exact Cauchy bound and rational non-root subdivision points to construct disjoint isolating intervals.
6. Verify that each interval endpoint is not a root and its exact Sturm count is one.
7. Check the projective direction theta = pi separately. In the half-angle chart t = tan(theta/2), this point is t = infinity; it is represented by the vanishing quartic leading coefficient and must not disappear during degree reduction.

Repeated roots are counted once as distinct angular directions. Nearby but distinct roots receive separate disjoint intervals. An identically zero polynomial or an exhausted isolation limit returns an error to the test harness, interpreted as INCONCLUSIVE rather than uniqueness.

## Falsification coverage

The test oracle covers:

- exact binary64-to-dyadic conversion, including subnormals and signed zero;
- repeated roots, near-coalescing roots, no-real-root polynomials and degree reduction;
- the projective theta = pi root;
- both frozen multiple-solution witnesses from #3168;
- a deterministic 64-case SPD Delassus panel comparing exact Sturm intervals with runtime candidate angles.

The parent tests continue to check Signorini complementarity, tangential KKT stationarity, Coulomb-disk feasibility, non-positive contact kinetic-energy change, frame covariance, and fail-closed invalid metrics. Exact root completeness is a different observable from candidate residual correctness.

## Acceptance and claim ceiling

A passing exact-head workflow would show that the runtime floating-point enumerator agrees with an independent exact Sturm oracle on this frozen, deterministic corpus. It would not be a formal proof for every possible runtime input. The runtime label remains UniqueWithinTolerance; it is not upgraded to unconditional mathematical uniqueness.

The verifier also does not extend the parent existence argument beyond its specific one-contact impulse law with a finite friction coefficient and SPD 3x3 Delassus matrix. Multi-contact islands, singular Delassus operators, compliant contact, acceleration-level contact, and full engine behavior remain separate research problems.
