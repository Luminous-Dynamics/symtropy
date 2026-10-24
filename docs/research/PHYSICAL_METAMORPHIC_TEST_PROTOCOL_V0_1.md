# Physical Mechanics Metamorphic Test Protocol v0.1

**Tracking:** [PHYS-TRANSFORM-001 #2209](https://github.com/Luminous-Dynamics/symtropy/issues/2209)  
**Implementation:** [reference kernel PR #2238](https://github.com/Luminous-Dynamics/symtropy/pull/2238), `research/physical-transformation-reference/tests/metamorphic.rs`  
**Status:** software-level invariance checks plus a CI mutation-sensitivity gate; neither is physical validation. Report pass/fail only for the exact commit that ran.

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

## Mutation-sensitivity gate

The CI script `research/physical-transformation-reference/scripts/verify_mechanics_mutations.sh` checks whether selected metamorphic cases detect six concrete formula defects. It first runs the unmodified metamorphic suite as a passing control, then makes a fresh temporary copy of the crate for each single source mutation:

| Mutation | Expected detecting test | Failure mechanism |
|---|---|---|
| Axial extension uses force times length-squared instead of force times length | `axial_bar_obeys_force_homogeneity_and_geometric_scaling` | Geometric-similarity extension no longer has the expected inverse length scaling. |
| Axial stress divides by area-squared instead of area | `axial_bar_obeys_force_homogeneity_and_geometric_scaling` | Stress no longer scales inversely with area under geometric similarity. |
| Cantilever tip deflection uses length-squared instead of length-cubed | `cantilever_obeys_force_span_and_stiffness_scaling` | Span response no longer scales cubically. |
| Cantilever deflection omits Young's modulus from the flexural denominator | `cantilever_obeys_force_span_and_stiffness_scaling` | Doubling modulus and second moment no longer reduces deflection by the expected factor. |
| Simply-supported beam deflection uses length-cubed instead of length-to-the-fourth | `simply_supported_beam_obeys_load_span_and_stiffness_scaling` | Span response no longer scales to the fourth power. |
| Simply-supported beam stress is incorrectly divided by Young's modulus | `simply_supported_beam_obeys_load_span_and_stiffness_scaling` | Stress changes when modulus changes, violating the model's stress/modulus independence relation. |

A mutation counts as detected only if the specifically named test is reported as failed. A passing mutant, compile failure, unrelated Cargo failure, or missing expected failure marker fails the audit. Every mutation is confined to a temporary copy; the unmodified working tree remains authoritative.

The script writes `mutation-sensitivity-receipt.json` with the exact source commit, SHA-256 hashes for the tested mechanics and metamorphic files, Rust/Cargo versions, passing baseline-control disposition, stable mutation IDs and individual dispositions. CI publishes it beside the existing synthetic reference receipt. The summary counters apply only to this six-mutant set: they are not a general mutation score and do not establish that all plausible defects are detectable. This layer remains software-test sensitivity, not physical validation; it does not replace frozen independent known-answer tests.


## Acceptance and claim limits

- Every listed relation passes on the pinned test parameters.
- Invalid/non-finite domains continue to fail closed.
- Frozen known-answer tests remain present; metamorphic equality cannot replace them.
- A future seeded sweep publishes its seed, tested ranges, failure count and exact revision.
- No measured-data accuracy, solver superiority, structural safety, building-code compliance, or demolition authorization is inferred from these software tests.

The implementation adds metamorphic tests for axial-bar force/geometric scaling, cantilever force/span/stiffness scaling, simply-supported-beam load/span/modulus scaling, and caller-side source-unit normalization equivalence. It also adds a fixed-seed sweep of 128 bounded cases per run across deterministic SI parameter ranges, checking selected scaling laws. The sweep uses an explicitly implemented 64-bit linear-congruential sequence; it is for reproducible test generation, not security or statistical random sampling. Failures include the case index and must be reproduced at the pinned commit.

The current sweep is intentionally small and does not establish broad stochastic coverage. Next add mutation-sensitivity evidence: demonstrate that test variants detect deliberately altered exponents/omitted factors, without shipping the mutated formula to production. Test execution and CI qualification must be reported against the exact current commit; a queued workflow is not a pass.
