# Physics differential fuzzing — V0.1 seed

This standalone, dependency-free Rust crate is the first executable slice of
[PHYS-DIFF-FUZZ-001](https://github.com/Luminous-Dynamics/symtropy/issues/4446).

It currently provides:

- deterministic SplitMix64 generation from an explicit seed;
- bounded, physically valid 1D point-impact cases in SI units;
- an analytic first-impact time / pre-impact velocity oracle;
- distinct rebound and zero-restitution resting-contact outcomes;
- a coherent length/time unit-rescaling metamorphic relation;
- deterministic and analytical regression tests.

## Deliberate limits

This is **not yet a cross-backend differential runner**. It does not invoke
Symtropy, Rapier, MuJoCo or other engines; it does not decide semantic
equivalence; and it does not publish scenario identities. The generated
`(seed, ordinal)` pair is generator provenance only. Portable scenario
identity belongs to #3060/#1172, semantic pair admission to #4366, independent
physics verification to #3749, Rapier execution to #443/#495 and the final
qualification corpus to #460.

Generated cases remain search artifacts. A later implementation tranche must
add admitted backend execution, discrepancy predicates and failure-preserving
minimization before any generated case can be promoted to the canonical corpus.

## Run

```bash
cargo +1.96.0 fmt --manifest-path research/physics-differential-fuzz-001/Cargo.toml -- --check
cargo +1.96.0 test --manifest-path research/physics-differential-fuzz-001/Cargo.toml --locked
cargo +1.96.0 clippy --manifest-path research/physics-differential-fuzz-001/Cargo.toml --all-targets --locked -- -D warnings
```
