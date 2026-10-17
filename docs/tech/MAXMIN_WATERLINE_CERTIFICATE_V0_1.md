# Proof-carrying max-min allocation — v0.1

**Status:** Draft formal contract. Theorems are authored, not verified until the exact-head pinned Lean build passes.

## Paradigm shift: verify the answer, not just the optimizer

An optimizer should not be the sole authority on the allocation it returns. For this policy, it can emit an exact-rational waterline certificate \`λ\`; a small checker reconstructs every allocation as

\[
a_i = \min(c_i, \lambda)
\]

where \(c_i\) is the recipient's eligible headroom/rate cap. It then checks the aggregate accounting identity

\[
\sum_i a_i = \min(S, \sum_i c_i)
\]

for finite source supply \(S\), alongside non-negative supply, caps and waterline.

This makes the policy output **proof-carrying**: the expensive or heuristic optimizer can search for a waterline, while a much smaller checker determines whether the result satisfies the exact policy. A certificate checker is not a replacement for proving the optimizer complete, but it reduces the trusted computing base for *accepting* a proposed result.

## What the Lean contract covers

\`formalization/lean/SymtropyFormal/WaterlineAllocation.lean\` specifies the exact-rational allocation rule and currently declares theorem obligations for:

- non-negativity and per-recipient cap adherence;
- saturation of constrained recipients;
- equal treatment of unsaturated recipients at the waterline;
- corresponding permutation of outputs when inputs are permuted;
- aggregate supply and aggregate capacity limits for an accepted certificate;
- a positive control and an over-budget negative control.

The cap list is assumed to be aligned with a canonical stable-ID order by the calling layer. Stable-ID uniqueness and canonicalization are not part of this first abstract model.

## What remains unproved

- **Existence/completeness:** this draft does not yet prove that a suitable waterline certificate exists for every valid exact input.
- **Rust refinement:** no statement here proves the production \`f64\` allocator matches the exact \`ℚ\` policy.
- **Floating point:** per-operation and aggregate rounding, tiny residual budgets and subnormal values need an independent numeric analysis and adversarial tests.
- **Integration policy:** overlapping wells remain a caller-policy issue (joint pool versus declared priority) and this PR does not alter the live ECS loop.

A Lean theorem only closes the mathematical-contract dimension once checked in the pinned environment. It cannot close source refinement, runtime identity or physical validity by itself.

## Proposed benchmark campaign

These are gates to implement and measure—not reported results.

| Metric | Proposed gate |
|---|---:|
| Invalid certificate accepted | 0 |
| Budget overspend | 0 |
| Per-recipient cap violations | 0 |
| Shared exact fixtures matching the contract | 100% |
| Tested recipients | 0, 1, 2, 10, 1k, 10k |
| Mutation score on targeted budget/cap/order faults | ≥95%, with survivors listed |

Report p50/p95/p99, memory, accepted/rejected certificate counts, and independent-oracle mismatches. Include cap distributions with all-zero demand, one large recipient, many tiny recipients, one saturated recipient, supply far below demand, supply above capacity, and values across floating-point magnitude regimes.

The production allocator and exact verifier must be compared against an independently implemented rational oracle. The Rust comparison should report both exact cases and scale-aware floating-point residuals; a tolerance must not conceal a genuine overspend.

## References

- [THERMO-ALLOC-001 allocator contract](https://github.com/Luminous-Dynamics/symtropy/issues/1747)
- [Deterministic max-min allocator PR](https://github.com/Luminous-Dynamics/symtropy/pull/1748)
- [THERMO-PROOF-002 formalization follow-up](https://github.com/Luminous-Dynamics/symtropy/issues/1761)
- [Pinned Lean / formalization groundwork](https://github.com/Luminous-Dynamics/symtropy/pull/1691)
- [OpenAI public math repository](https://github.com/openai/math), used as process inspiration for separating manuscript, theorem artifacts, pinned environment and proof status—not as evidence for this allocation policy.
