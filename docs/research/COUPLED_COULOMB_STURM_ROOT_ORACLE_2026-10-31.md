# Independent exact Sturm root-count oracle for coupled Coulomb quartics

**Status:** test-only exact-arithmetic reference; Rust exact-head validation pending.  
**Date:** 2026-10-31  
**Parent:** [#3168 coupled 3D Signorini-Coulomb quartic reference](https://github.com/Luminous-Dynamics/symtropy/pull/3168).  
**Qualification authority:** all eventual results remain part of [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Why a second root solver is necessary

The coupled contact solver's production quartic routine recursively partitions the real line at floating-point derivative roots and bisects sign-changing intervals. It is fast and useful as a candidate generator, but by itself it does not prove that every real root was found.

The particularly dangerous case is a pair of closely spaced simple roots. At the local stationary point between them, the polynomial can be very small without being zero. Treating a small value as an even-multiplicity root merges two distinct roots into one candidate and may allow UniqueWithinTolerance to be reported incorrectly.

The safe response to a stationary value that is small but nonzero at the configured uncertainty scale is Inconclusive. It is not safe to coerce it to a root.

## Independent verifier

The new test-only verifier is implemented in coupled_friction_sturm.rs. It does not reuse the production derivative-partition algorithm.

1. Each finite binary64 coefficient is decoded from its IEEE-754 sign, exponent and significand into its **exact dyadic rational**. There is no decimal parse or floating conversion in this step.
2. It builds a polynomial over BigRational.
3. It constructs a classical Sturm chain using exact rational polynomial remainders. Only positive rescalings are applied, so sign variations are preserved.
4. At rational endpoints, one-sided signs are obtained from the first nonzero derivative. This handles a root exactly on a bisection midpoint without dropping it or double-counting it.
5. A strict Cauchy bound encloses every finite real root. Exact Sturm variation differences count distinct roots in open rational intervals.
6. Rational bisection yields disjoint intervals; each emitted interval has an independently verified Sturm count of exactly one. Exactly representable rational roots are retained as point intervals.

The test compares the production root list with the exact distinct-root count and checks that every production root is close to one certified interval.

### Claim boundary

This proves the number and isolating intervals of the distinct real roots of the **binary64 coefficient polynomial actually supplied to the oracle**. It does not prove that the coefficient calculation from physical inputs was performed in exact arithmetic, nor that an impulse satisfies the original physical model beyond the existing independent residual checks. It is an independent root-completeness check for the rounded quartic, not a complete formal proof of the entire contact solve.

The verifier is test-only. num-bigint, num-rational and num-traits are development dependencies, not runtime dependencies of Symtropy's physics crate.

## Changes to production ambiguity handling

The production enumerator no longer turns a small stationary value into an approximate repeated root. If a stationary evaluation falls inside the configured ambiguity envelope—including an f64 evaluation that rounds to exactly zero—root enumeration returns an error. A bisection midpoint that evaluates to zero is also unresolved because a rounded floating zero is not an exact rational sign certificate. Close numerical candidates are never merged silently.

Ordinary simple roots remain isolated by sign-changing monotone intervals. Repeated-root cases are therefore handled by the exact test oracle, while the production candidate generator refuses to assert their multiplicity without a separate exact count. This is a deliberate trade-off: a small fraction of numerically ambiguous contacts should be refused rather than silently assigned the wrong multiplicity.

## Frozen tests

- Exact binary64 decoding round-trips signed zero, 0.1, the smallest subnormal, the smallest normal, and the largest finite values.
- Four distinct simple roots at known integer locations.
- Repeated factors; Sturm count is the number of **distinct** real roots, not algebraic multiplicity.
- A repeated root and a double root exactly at the initial rational bisection midpoint; exact Sturm resolves the distinct roots while production refuses to guess multiplicity.
- A close pair of distinct dyadic roots; exact Sturm resolves four roots while production refuses the ambiguous stationary point.
- No-real-root and degree-reduced polynomials.
- Positive/negative binary-power scaling invariance.
- A deterministic 128-case panel of SPD 3x3 contact matrices; each generated sliding quartic is checked by both algorithms. At least 120 cases must be resolved by the production method; unresolved cases are separately counted rather than treated as successes.

## Promotion targets

These are gates, not achieved measurements:

| Gate | Target |
|---|---:|
| Exact rational Sturm count on frozen polynomial corpus | 100% |
| Every emitted rational interval contains exactly one distinct real root | 100% |
| Production roots matched to independent intervals when classified resolved | 100% |
| Close-root ambiguity converted to Inconclusive | 100% |
| Silent merge of two distinct roots | 0 |
| False UniqueWithinTolerance on unresolved root panels | 0 |
| Exact arithmetic in release/runtime solver | Not required |

The next stronger step is interval arithmetic on the coefficient construction itself, which would enclose rounding from W, q, mu through the quartic coefficients. That is a distinct proof obligation and should not be conflated with the exact Sturm count for the supplied binary64 polynomial.

## Industry and research comparison

Most production physics engines expose solver settings and residual/performance behaviour, not a certified root count for a single-contact Coulomb quartic. This verifier therefore targets a different comparison layer from MuJoCo, PhysX, Rapier, Chrono and Newton:

- **Mathematical references:** exact Sturm count and interval isolation versus production floating candidates.
- **Contact law:** KKT, complementarity, friction-disk and kinetic-energy residuals versus independent high-precision solves.
- **Engine integration:** matched scenarios through [#443](https://github.com/Luminous-Dynamics/symtropy/issues/443) only after contact conventions are mapped.
- **Qualification:** all receipts feed [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

The defensible differentiator is not merely that Symtropy computes impulses quickly. It is that it can state how many candidate roots the rounded equation has, exhibit their isolating intervals, show which candidates pass the physical contact law, and refuse a uniqueness claim when the numerical evidence is insufficient.

No runtime speedup, production-contact or SOTA claim is made by this test-only oracle.
