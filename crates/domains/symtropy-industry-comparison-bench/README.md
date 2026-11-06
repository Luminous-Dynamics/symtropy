# Symtropy Industry Comparison Bench

Standalone, read-only evidence comparator for published manufacturer figures, operator/contractor field reports and peer-reviewed mining trials.

Implements [issue #3930](https://github.com/Luminous-Dynamics/symtropy/issues/3930). It does not estimate machine performance or create a pooled score.

## Run

    cargo test --manifest-path crates/domains/symtropy-industry-comparison-bench/Cargo.toml

## Comparison gate

Pairwise ranking is allowed only when all these fields match exactly:
- metric family and versioned metric definition;
- unit;
- numerator basis and denominator basis;
- operating boundary;
- task profile;
- geology/material cohort;
- geometry profile;
- task acceptance profile;
- declared comparison protocol;
- machine configuration class.

The exact machine model/configuration may differ because the comparator is intended to compare different machines. All other operating context must be matched. Unknown required context yields INSUFFICIENT_CONTEXT, never equality.

If matched, the evaluator computes conservative value-interval differences. It only declares a definite winner when the intervals are strictly separated in the declared higher/lower-is-better direction. Overlap is NOT_RESOLVED.

Two reports of the same physical trial are not independent competitors; an operator report and secondary article sharing one trial ID are flagged as SAME_TRIAL_NOT_INDEPENDENT.

## Public-figure fixture

The fixture records distinct MC51 published/operator figures and the South African undercutting/vacuum trial. They are deliberately not combined:
- >3.8 m/shift and >5.7 m/day have different time denominators;
- 20 m3/h cutting, 30 m3/h in-situ material handling and up-to-60 m3/h cut-material removal are different metric definitions;
- 237 MPa trial mean UCS is not definitionally the same as 250 MPa advertised rock-hardness capability;
- 25% downtime reduction is relative to a prior cutting-head baseline, not absolute availability.

A separate synthetic matched pair validates the arithmetic only. It does not represent a real mine or product.

## Non-claims

COMPARABLE means that declared definitions/context pass this contract. It does not mean independently replicated, physically qualified, or commercially superior. Evidence source class and reported-value semantics remain visible.
