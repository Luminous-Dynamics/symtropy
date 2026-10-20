# Physical Mechanics Metamorphic Test Protocol v0.1

**Tracking:** [PHYS-TRANSFORM-001 #2209](https://github.com/Luminous-Dynamics/symtropy/issues/2209)  
**Implementation:** [reference kernel PR #2238](https://github.com/Luminous-Dynamics/symtropy/pull/2238), `research/physical-transformation-reference/tests/metamorphic.rs`  
**Status:** software-level invariance checks; not physical validation or an executed test result.

## Why add metamorphic tests?

Known-answer fixtures test selected parameter points. Metamorphic tests instead ask whether responses preserve the invariants implied by an already-declared model when its inputs are transformed. They are useful for finding exponent errors, sign mistakes, accidental parameter coupling, and unit-normalization mistakes without requiring a measured test for every generated case.

They are not an independent physical oracle by themselves: related outputs can all be consistently wrong. Keep frozen known-answer cases and independent integration expectations as separate gates.

## Declared SI API contract

The reference functions accept SI-valued numeric inputs, not quantity types or unit strings. The source-unit cases in this test file convert kN, mm, mm², mm⁴, and GPa to SI before calling those functions. They check caller-side normalization equivalence only; they do **not** test or imply an automatic unit parser, dimensional type system, or schema-level unit conversion.

## Metamorphic relations

For the current idealized, linear-elastic models, the tests pin these relations:

| Model and transformation | Expected response |
|---|---|
| Axial bar: force (F\rightarrow kF), other inputs fixed | Stress, strain, and extension multiply by (k). |
| Axial bar: geometrically similar scaling (L\rightarrow kL, A\rightarrow k^2A), force/material fixed | Stress and strain divide by (k^2); extension divides by (k). |
| Cantilever tip load: (F\rightarrow kF) | Signed deflection and stress magnitude multiply by (k). |
| Cantilever: (L\rightarrow kL), other inputs fixed | Tip deflection scales as (k^3); maximum stress scales as (k). |
| Cantilever: (E\rightarrow kE, I\rightarrow kI) | Deflection divides by (k^2); stress magnitude divides by (k). |
| Simply supported beam with full-span uniform load: (q\rightarrow kq) | Deflection and maximum stress multiply by (k). |
| Simply supported beam: (L\rightarrow kL), other inputs fixed | Midspan deflection scales as (k^4); maximum stress scales as (k^2). |
| Simply supported beam: (E\rightarrow kE), other inputs fixed | Deflection divides by (k); stress remains unchanged for this model. |
| Equivalent source-unit encodings converted to SI | Corresponding result fields agree within stated floating-point tolerances. |

The tests use an explicitly declared comparison rule: relative tolerance (10^{-10}) with an absolute floor (10^{-14}) in the output's SI units. This is a software-test tolerance, not a measurement uncertainty or a universal physical accuracy guarantee. Near-zero and ill-conditioned results require separate absolute-error handling before expanding the domain.

## Qualification sequence

1. Keep exact expected-value integration tests for the bar and both beam families.
2. Run the metamorphic suite with the same pinned Rust toolchain, formatting, Clippy, and locked dependencies as the reference kernel.
3. Add seeded parameter sweeps only after defining safe numeric ranges and reporting the seed and every failure; reject non-finite values and overflow instead of filtering them out.
4. Add mutation checks for intentionally wrong exponents or omitted parameters, and require the suite to detect those mutations before treating the metamorphic layer as strong regression evidence.
5. Once a unit-aware boundary exists, test conversion there directly with dimensional types and invalid-dimension negative controls; do not retrofit the current SI-only functions as if they already implement this capability.
6. Keep physical validation separate: compare a qualified solver against independently measured, rights-cleared test cases with pinned uncertainty, blind holdouts, and a named baseline.

## Acceptance and claim limits

- Every listed relation passes on the pinned test parameters.
- Invalid/non-finite domains continue to fail closed.
- Frozen known-answer tests remain present; metamorphic equality cannot replace them.
- A future seeded sweep publishes its seed, tested ranges, failure count and exact revision.
- No measured-data accuracy, solver superiority, structural safety, building-code compliance, or demolition authorization is inferred from these software tests.

The implementation adds metamorphic tests for axial-bar force/geometric scaling, cantilever force/span/stiffness scaling, simply-supported-beam load/span/modulus scaling, and caller-side source-unit normalization equivalence. It also adds a fixed-seed sweep of 128 bounded cases per run across deterministic SI parameter ranges, checking selected scaling laws. The sweep uses an explicitly implemented 64-bit linear-congruential sequence; it is for reproducible test generation, not security or statistical random sampling. Failures include the case index and must be reproduced at the pinned commit.

The current sweep is intentionally small and does not establish broad stochastic coverage. Next add mutation-sensitivity evidence: demonstrate that test variants detect deliberately altered exponents/omitted factors, without shipping the mutated formula to production. Test execution and CI qualification must be reported against the exact current commit; a queued workflow is not a pass.
