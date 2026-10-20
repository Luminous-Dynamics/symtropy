# Analytical Interval Enclosure Method v0.1

**Tracking:** [PHYS-UQ-001 #2292](https://github.com/Luminous-Dynamics/symtropy/issues/2292)  
**Implementation:** [draft PR #2295](https://github.com/Luminous-Dynamics/symtropy/pull/2295)  
**Status:** bounded arithmetic API and synthetic tests; request/receipt schema integration is not yet implemented.

## Definition

For a supported analytical case, each input is represented by a nominal scalar and a finite interval [lower, upper]. The nominal scalar must lie inside its interval. The method returns deterministic enclosures for outputs consistent with all represented input values under the named closed-form equation.

These ranges are not probability distributions, standard deviations, confidence intervals or coverage probabilities. They cover only the provided parameter ranges under the selected formula; they do not automatically include measurement error, model-form uncertainty, uncertain boundary conditions, omitted physics, or uncertainty in solver implementation.

## Arithmetic rules

- Every interval must have finite endpoints and satisfy lower <= upper.
- Positive geometry and material denominator terms must have strictly positive lower bounds. A range containing zero for area, modulus, inertia or other denominator is rejected rather than divided through.
- Multiplication and division evaluate interval corner combinations, then expand the result outward using adjacent representable floating-point values. A non-finite operation is an error.
- Positive integer powers are built from repeated outward interval multiplication, rather than relying on a platform power routine with unspecified rounding accuracy.
- Absolute-value quantities such as bending-stress magnitude treat a signed load interval crossing zero specially: the lower magnitude bound is exactly zero.
- Bound maps must contain exactly the inputs declared for the requested formula. Missing/unknown inputs and nominal values outside intervals fail closed.
- Bounds can over-approximate when intermediate expressions share parameters. Over-approximation is acceptable; under-coverage is not.

## Frozen test targets

The UQ module tests all three qualified formula families, checks that independent nominal formula values lie inside the returned intervals, enumerates input-box corner combinations for each family, tests load intervals crossing zero, and rejects reversed/non-finite bounds, out-of-range nominal values, unknown keys, nonpositive denominator bounds, overflow and unsupported cases.

These are software soundness checks over the named analytic formulas, not validation against physical specimens. Before putting interval output into the public JSON request/receipt contract, the API must pass exact-head formatting, tests and strict Clippy, and review must confirm that every declared output interval remains finite and ordered.

## Next step

Add a versioned request field for input bounds with unit/provenance fields and extend the result receipt with lower/nominal/upper output values. Keep the existing point-only contract valid and do not invent input uncertainty when none was supplied. Use calibrated measurements where available to choose bounds; otherwise the result remains conditional on user-declared ranges.
