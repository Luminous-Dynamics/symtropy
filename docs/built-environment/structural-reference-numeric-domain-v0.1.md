# Structural reference numeric-domain contract v0.1

- **Status:** draft contract; implementation and tests are authored on a stacked pull request, not yet CI-verified.
- **Scope:** the small, analytical Rust kernel for positive-magnitude, linear-static structural reference cases.
- **Not a claim:** this contract is not a floating-point error proof, FEM verification, physical validation, building-code compliance, or construction approval.

## Paradigm shift: numeric outputs need a declared domain

A finite `f64` is not automatically an acceptable engineering result. IEEE-style floating-point formats distinguish normal numbers from subnormal values; Rust documents `f64::MIN_POSITIVE` as the smallest positive normal value and values between zero and that boundary as subnormal ([Rust `f64` reference](https://doc.rust-lang.org/std/primitive.f64.html)). Subnormal values preserve gradual underflow, but carry reduced significand precision relative to normal values. For this narrow reference kernel, silently propagating such values risks presenting a numerically fragile result as an ordinary magnitude.

Computational-solid-mechanics guidance treats verification, validation and uncertainty quantification as connected but distinct credibility activities ([ASME V&V 10, 2019 reaffirmed in 2025](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics)). NASA's active [NASA-STD-7009B](https://standards.nasa.gov/standard/NASA/NASA-STD-7009) likewise requires acceptance criteria to be defined for the intended program/project and has a 2026 implementation handbook ([NASA-HDBK-7009B](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009)). A [NIST report published 5 October 2026](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing) argues for risk-scaled credibility and for harmonizing uncertainty quantification with metrology. These sources support keeping an explicit numeric domain and treating uncertainty/error bounds as their own obligation; they do not endorse this particular subnormal cutoff. These guards are a first, narrow domain rule—not a roundoff estimate.

## Normative candidate policy

All public SI-magnitude constructors and all derived calculation stages must fail closed unless values are finite, positive and normal:

- Non-finite input: `NonFiniteInput`.
- Zero or negative magnitude: `NonPositiveInput`.
- Positive subnormal input: `SubnormalInput`.
- Non-finite intermediate: `NonFiniteIntermediate`.
- Zero or negative intermediate: `NonPositiveIntermediate`.
- Positive subnormal intermediate: `SubnormalIntermediate`.
- Non-finite output: `NonFiniteOutput`.
- Zero or negative output: `NonPositiveOutput`.
- Positive subnormal output: `SubnormalOutput`.

The boundary `f64::MIN_POSITIVE` is normal and may enter a typed constructor. A calculation can still be rejected later if an operation with that input overflows, underflows to zero, or produces a subnormal intermediate. Constructors validate magnitudes that callers have already normalized to SI; they perform no unit conversion.

This is deliberately conservative for the elementary structural reference kernel, not a universal rule for all scientific computing. It also does **not** imply that a normal finite result is sufficiently accurate: cancellation, conditioning, accumulated rounding, and model-form error still require separate assessment.

## Adversarial controls authored

- Test all seven quantity constructors against the smallest positive subnormal value and at the minimum-normal boundary.
- Construct a rectangle whose positive area is subnormal, and require an explicit intermediate-domain rejection.
- Exercise cantilever point-load and simply-supported point/uniform-load span-power overflow and underflow.
- Preserve existing known-answer values, unit-category type checks, and dimensionless scale tests.

## Acceptance targets — proposed, not achieved

| Metric | Target |
|---|---:|
| Subnormal values admitted by the seven public quantity constructors | 0 |
| Seeded subnormal intermediate/output cases silently returned as valid values | 0 |
| Seeded overflow/underflow span-power cases returned as valid beam results | 0 |
| Known-answer analytical vectors lost due solely to the guard change | 0 |
| Claimed forward-error bounds from these guards alone | 0 |

The exact-head Rust CI must execute the tests before any pass can be claimed. This draft has not been compiled or run locally in this work session.

## Next numerical-verification gate

1. Derive per-output floating-point error budgets from the implemented operation graphs and the declared input envelope.
2. Compare results against an independently implemented higher-precision or interval-arithmetic oracle. Do not treat fixture agreement with the same formula family as independence.
3. Add near-boundary and randomized logarithmic sweeps over a predeclared admissible engineering range, recording rejected cases and output-specific relative/absolute error.
4. For a future FEM adapter, separately report equilibrium residuals, refinement/convergence, solver termination, independent-solver comparison, and validation against observations with uncertainty.

Passing the numeric-domain guard tests would establish only that the enumerated subnormal and overflow/underflow hazards are rejected. It would not establish any of the four later gates.
