# Exact interval certification of Coulomb root-count stability

**Status:** test-only bounded-family verifier; exact-head Rust validation pending.  
**Date:** 2026-10-31  
**Parent:** [#3175 exact Sturm root-count oracle](https://github.com/Luminous-Dynamics/symtropy/pull/3175).  
**Research contract:** [#3176 PHYS-CONTACT-ROBUST-ROOT-001](https://github.com/Luminous-Dynamics/symtropy/issues/3176).  
**Qualification authority:** [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Paradigm shift: certify a family, not a sample

The exact Sturm verifier in #3175 proves the distinct real-root count for one declared exact binary64 input tuple (W, q, mu). Physical parameters, geometry and measured velocities are uncertain. Sampling a hundred parameter values is useful falsification, but does not establish what happens between samples.

This increment adds an exact rational interval certificate over a bounded rectangular family of symmetric 3x3 Delassus matrices, free velocities and positive friction coefficients.

For a quartic polynomial P(t) = a*t^4 + b*t^3 + c*t^2 + d*t + e, its finite distinct real-root count cannot change continuously while both the leading coefficient and discriminant remain nonzero. A root can escape through the projective point t = infinity only if the leading coefficient reaches zero; finite roots can collide or change real/complex character only when the discriminant reaches zero.

The verifier therefore certifies a whole connected input box only when it can prove all of the following:

1. Every matrix represented by the symmetric Delassus intervals is positive definite.
2. The interval enclosure of the quartic leading coefficient excludes zero.
3. The interval enclosure of the quartic discriminant excludes zero.
4. An exact rational Sturm count succeeds at the exact midpoint representative.

The leading coefficient and discriminant are enclosed using rational interval arithmetic with exact endpoints and operations. Dependency overestimation may cause conservative refusal; it cannot justify excluding a value that is actually possible.

## Independent interval coefficient construction

The interval quartic is expanded from the same half-angle vector equation as the exact-input point oracle, but with interval rational operands. Its coefficient enclosure is checked against 128 deterministic exact-dyadic samples from a frozen input box.

The quartic discriminant is evaluated by an interval expansion of the closed-form quartic discriminant. Its point-interval formula is cross-checked against an exact determinant of the 7x7 Sylvester matrix for (P, P'). In the seeded interval containment panel, the exact resultant discriminant of each exact sample must lie inside the interval discriminant enclosure.

No floating-point interval endpoints are used by the authority path: input endpoints are decoded as exact binary64 dyadic rationals, and every bound operation remains in BigRational.

## Positive-definite family guard

For every matrix in a symmetric interval box, the verifier computes the conservative Gershgorin lower bound

min_i (lower(W_ii) - sum_{j != i} max_abs(W_ij)).

A strictly positive result proves all represented symmetric matrices are strictly diagonally dominant with positive diagonals, hence positive definite. This is a sufficient rather than necessary condition. If it fails, the result is NeedsSubdivision; it is not a claim that the matrix is indefinite.

## Outcomes and deterministic subdivision

The family verifier distinguishes:

- Certified { finite_real_root_count }: the exact root count is invariant throughout the input box.
- NeedsSubdivision(reason): interval dependency, possible projective transition, possible discriminant-zero event, or failed SPD enclosure prevents certification.
- InvalidInput(reason): the declared friction interval includes negative values.

A deterministic subdivision routine bisects the input interval with the greatest relative width. It traverses left child before right child, uses exact rational midpoints, and obeys both a maximum depth and a maximum visited-box budget. Its family summary is:

- UniformlyCertified: every leaf was certified and all leaves have the same root count.
- RootCountVaries: fully certified leaves have different root counts.
- Inconclusive: one or more leaves remain unresolved or the resource budget is exhausted.
- InvalidInput: at least one leaf contains an invalid friction domain.

A box crossing a root fold or projective boundary is never reported as uniformly certified. Subboxes that touch the unresolved boundary remain inconclusive unless a stronger exact enclosure can separate them.

## Frozen tests

- A narrow regular SPD uncertainty box is certified with an exact representative Sturm root count.
- A box whose quartic leading coefficient can pass through zero is not certified.
- A box without a positive-definiteness proof is not certified.
- A negative friction range is invalid; a range including zero is unresolved.
- A wide box whose discriminant enclosure contains zero is not certified.
- 128 seeded exact-dyadic points must be contained by every interval quartic coefficient and the interval discriminant.
- The closed-form interval discriminant is checked against an independent exact Sylvester resultant at a point polynomial.
- Repeated invocation of deterministic subdivision produces the same summary and respects the box budget.

## What is and is not proven

This is a root-count-invariance certificate for the angular quartic over a declared bounded family of exact dyadic (W, q, mu) values. It does not prove that:
- W was correctly assembled from the real bodies, inertias and contact Jacobians;
- uncertainty ranges cover all real-world parameter error;
- an angular root yields an admissible impulse for every point in the box;
- sticking/sliding/separating candidate multiplicity is constant under all physical admissibility conditions;
- the multi-contact island, integrator or full trajectory is accurate;
- Symtropy is faster or generally superior to any engine.

Admissibility of the normal impulse, tangent KKT multiplier, original contact residuals and energy change remains a subsequent branch-wise interval-verification obligation. All eventual campaign receipts remain under #460.

## Promotion targets

These are proposed gates, not achieved results:

| Gate | Target |
|---|---:|
| False family certification over exact adversarial boxes | 0 |
| Interval coefficient/discriminant containment on frozen exact points | 100% |
| SPD family proofs sound against independent exact matrices | 100% |
| Root-count invariance for admitted boxes | 100% |
| Boxes crossing leading/discriminant zero falsely certified | 0 |
| Subdivision replay divergence | 0 |
| Resource exhaustion converted to Inconclusive | 100% |
| Exact arithmetic in the runtime solver | Not required |

The next stronger step is interval continuation of each admissible impulse branch, followed by interval uncertainty in the physical assembly of W itself. This PR deliberately stops at root-count invariance.
