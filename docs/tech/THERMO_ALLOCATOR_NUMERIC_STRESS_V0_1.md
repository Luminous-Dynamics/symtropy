# Thermodynamic allocator numeric stress — v0.1

**Status:** Test protocol and adversarial cases authored; Rust execution and timing are not yet verified.

## Scope

The allocator in `src/systems/thermodynamic_allocation.rs` uses IEEE-754 `f64`, while its policy description is naturally expressed over non-negative exact reals. These models are not interchangeable. This test extension widens the input distribution beyond ordinary decimal ranges so that safety properties are exercised across subnormal, normal, large, and near-maximum finite values.

The new deterministic unit test covers:

- explicit boundary values: zero, smallest positive subnormal, smallest positive normal, machine epsilon, (10^{-300}), (10^{-16}), (1), (10^{16}), (10^{300}), and `f64::MAX`.
- 4,096 seeded cases with recipient counts from 0 through 64 and non-negative finite values sampled across the binary64 exponent range;
- 4,096 quarter-unit cases differentially checked against an independent integer/rational water-filling oracle; all generated source values and caps are exactly representable in binary64, while only equal-share division may round;
- nine named exact boundary fixtures for empty demand, zero supply, zero caps, supply above capacity, repeated caps, exact saturation ties, constrained-plus-equal-share allocation, zero limits, and a second saturation threshold;
- stable-ID output ordering and exact output equality after reversing input order;
- finite, non-negative allocations bounded by each recipient's `min(headroom, per_recipient_limit)`;
- finite non-negative unallocated supply, total allocation not exceeding source supply, and conservation accumulation in canonical result order.

The property loops are deterministic and repeatable; they are stress/differential corpora, not a statistical proof or a performance benchmark. The nine explicit rational fixtures also match a separate Python `Fraction` oracle. A separate Python model experiment exercised the corresponding IEEE-754 allocator path against the rational oracle for the same quarter-unit distribution: 4,096 cases passed, with a maximum absolute allocation difference of approximately 5.7e-12. This is model-level evidence only, not execution of the new Rust test.

## Manual scaling harness

The source now includes an ignored benchmark test. After Rust tooling is available, run it from the repository root:

```sh
cargo test allocator_scaling_benchmark -- --ignored --nocapture
```

It reports p50/p95/p99 elapsed nanoseconds and the final allocated/unallocated totals for 0, 1, 2, 10, 1,000, 10,000, and 100,000 recipients. It uses two warm-up runs, then 50 samples for sizes below 10,000, 20 samples at 10,000, and 15 samples at 100,000. Workload generation occurs outside the timed region; each sample includes the allocator's own input collection, validation, sorting, and allocation.

There are intentionally no hard wall-clock assertions. Record the exact source SHA, `rustc -Vv`, profile/flags, CPU model, operating system, and power mode before comparing results. These sample counts support an initial p95/p99 diagnostic, not a statistically strong tail-latency claim. Run larger repeated batches for release qualification.

## Numeric assurance boundary

Subnormal supplies may not be divisible into equal, non-zero representable `f64` allocations. The safe response can be to leave a small residual unallocated rather than overspend. Therefore record **under-utilization** separately from conservation failure. The current test asserts hard safety and permutation invariance, not optimal utilization or exact-real max-min fairness.

The following remain open:

- refinement of the Rust `f64` implementation to the exact-rational Lean specification;
- a documented scale-aware numerical error bound;
- a shared corpus consumed by both Lean and Rust;
- a completeness argument for the waterline-finding procedure;
- live ECS integration and overlapping-well policy.

## Benchmark campaign

Run performance tests separately from ordinary unit tests to avoid fragile CI timing gates. Bind every report to the exact source SHA, compiler/toolchain, build profile, CPU, workload generator seed, and recipient distribution.

| Dimension | Target |
|---|---|
| Recipient counts | 0, 1, 2, 10, 1,000, 10,000, 100,000 |
| Dynamic range | subnormal through maximum finite; uniform and adversarial exponent distributions |
| Safety violations | 0 accepted cap violations; 0 source overspends; 0 non-finite outputs |
| Input-order invariance | 100% exact per-ID output equality for permutations |
| Independent exact fixtures | 100% agreement on shared rational cases where exact conversion is defined |
| Mutation adequacy | at least 95% targeted mutation score, with surviving mutations listed |
| Performance | p50/p95/p99 latency, throughput, and peak/allocated memory where reliably measurable |
| Utilization diagnostic | report unallocated supply even when eligible demand exists; never fold it into the safety-violation count |

The reference oracle should be implemented independently of the production allocator. Mutations must include budget overspend, recipient-cap bypass, duplicate-ID acceptance, input-order dependence, loss of tiny accepted allocations, non-finite acceptance, and accounting-field drift. A passing test suite must not be presented as proof of source refinement.

## Evidence status

This branch adds test code and this protocol only. The container used to author it does not provide `rustc` or `cargo`; consequently no Rust compilation or test execution is claimed. A queued GitHub check is pending evidence, not a pass.
