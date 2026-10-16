# Garden Portfolio Comparator — Benchmark Contract v1

**Status:** acceptance targets and synthetic verification plan; no benchmark results are claimed.  
**Implementation:** `src/portfolio.rs`, exposed as `symtropy_garden_sim::portfolio`.  
**Dependency:** stacked on [PR #1637](https://github.com/Luminous-Dynamics/symtropy/pull/1637), the exact resource/nutrient accounting contract.

## Paradigm

Do not compress ecological, resource, supply, affordability, diversity and other objectives into a single opaque score. The comparator returns all nondominated feasible candidates under explicitly declared objective directions, units and integer scales. The community or an authorized decision process can choose among the trade-offs; the library does not assert a universal optimum.

The comparer accepts **already-assessed** candidates. It does not authenticate evidence, calculate yields, determine food safety, authorize land, evaluate licenses, establish consent or calculate nutrient adequacy. Those are independent upstream contracts and hard gates. A feasibility enum is only as trustworthy as the upstream evidence and integration that supplied it.

## Current authored synthetic coverage

The current draft contains a hand-calculated two-objective oracle plus 100 seeded, reproducible four-objective scenarios checked against a separate exhaustive reference implementation. It also checks 1,000 deterministic input-order permutations and the irreflexive/asymmetric/transitive laws of strict dominance. These are authored tests; they are not reported as passing until exact-head CI succeeds. Before release, freeze a versioned input/output corpus and literal expected reports for an additional representative set so results can be independently replayed without relying on the test generator.

## Exact correctness gates

1. **Hand-calculated oracle:** every frozen small case must match its manually enumerated feasible set, frontier, dominated set and dominance witnesses exactly.
2. **No hard-gate bypass:** zero infeasible or unassessed candidates may enter the frontier. Non-feasible candidates need blocker references and cannot carry scoreable objective values. A malformed request blocks the entire comparison rather than returning a partial frontier.
3. **No silent dimension loss:** missing/duplicate/unknown objectives and mismatched units, scales, scope or period must block comparison; no implicit unit conversion occurs here.
4. **Exact arithmetic:** objective values and scales use integer types; large values serialize as decimal strings. No floating-point sum or weighted score is introduced.
5. **Dominance laws:** strict Pareto dominance is irreflexive and asymmetric, and is transitive for a fixed set of directions. Equal vectors do not dominate each other; equivalent candidate plans remain distinct alternatives.
6. **Input-order invariance:** 1,000 seeded permutations of candidate rows, objective definitions and per-candidate objective values must produce byte-equivalent semantic reports. Any future input collection whose order carries meaning must explicitly say so.
7. **Independent oracle growth:** add at least 100 frozen synthetic cases before calling the comparator release-ready; include ties, all-infeasible sets, one feasible candidate, mixed directions, negative signed objectives where allowed, duplicate and malformed IDs, no-candidate input, high-score infeasible candidates, scope/period mismatches and wide integers.
8. **Independent checking:** the reference oracle must not call `compare_portfolios_v1` or reuse its dominance helper. Freeze expected output literals for a representative subset.

## Provisional performance targets

These are budgets to measure—not claims about current performance. First record the reference hardware, operating system, Rust compiler version, exact release profile/flags, source revision, input generator seed and input/output digests.

| Workload | Target |
|---|---:|
| 1,000 feasible candidates × 12 objectives, exact pairwise frontier | p95 ≤ 500 ms |
| 100 independent cases × 100 candidates × 8 objectives | total ≤ 1 s |
| Semantic output under 1,000 seeded input permutations | 100% identical |
| Oracle agreement on frozen small cases | 100% exact |
| Hard-gate false accepts in adversarial synthetic corpus | 0 |

Timing protocol: at least 30 repetitions after warm-up in an optimized build; publish p50/p95, median frontier size, candidate/objective counts, peak memory, machine/toolchain, raw measurements and corpus digest. Report cold-start separately if it matters to the deployment. Keep noisy absolute timing out of correctness gates unless runs use stable dedicated hardware; correctness and hard-gate checks must always run.

The current implementation uses pairwise dominance and is therefore approximately O(N²M) in candidate count N and objective count M. The target workload makes this explicit; do not claim suitability for very large portfolios without measurement. If a future algorithm changes, it must match the exact oracle and preserve deterministic reports before performance improvements are accepted.

## Real-world validation is separate

Passing synthetic tests proves only that the comparator follows its declared contract for those cases. It does not prove local crop suitability, yield accuracy, food safety, nutritional adequacy, fair distribution or community benefit. Those require licensed/reviewed source data, correctly matched food state and geography, independent agronomic observations, and explicit permission/consent. For local yield modeling, pre-register the sample plan and compare to simple baselines on genuinely held-out sites/seasons; do not randomly split correlated rows and treat that as independent validation.

Research context:
- FAO crop-model review: build for a declared purpose, account for data gaps, calibrate and test appropriately, and avoid over-parameterization/over-fitting: https://www.fao.org/family-farming/detail/en/c/1646589/
- FAO/INFOODS standards and guidance for food matching and composition data quality: https://www.fao.org/infoods/infoods/standards-guidelines/en/
- Multi-environment testing/stability across farms, villages and seasons: https://www.fao.org/4/v5330e/V5330e0G.htm
- Multi-objective optimization preserves nondominated trade-offs rather than implying a single best answer: https://docs.rs/pare/latest/pare/

The thresholds above remain proposed acceptance criteria until measured and independently reviewed.
