# Shipyard space-time baseline (V0)

A dependency-free, deterministic first-fit/bottom-left-style baseline for the isolated verifier in [space-time PR #2548](https://github.com/Luminous-Dynamics/symtropy/pull/2548). This is an initial reference policy for benchmarking, not a solver or production scheduler.

## Run

```sh
cargo fmt --all --manifest-path tools/shipyard-space-time-baselines/Cargo.toml -- --check
cargo test --locked --offline --manifest-path tools/shipyard-space-time-baselines/Cargo.toml
cargo clippy --locked --offline --manifest-path tools/shipyard-space-time-baselines/Cargo.toml --all-targets -- -D warnings
```

The nested Cargo crate has no external dependencies and uses the sibling space-time verifier as a local path dependency.

## Policy contract

Caller-order first-fit processes requests in supplied order. Decreasing-area first-fit sorts by descending area, maximum side, width and height, then ascending reservation ID; area multiplication uses i128 to avoid overflow. A six-policy heuristic portfolio runs caller order, decreasing area, decreasing maximum side, decreasing height, decreasing width, and decreasing perimeter. It sends each completed layout through the independent verifier and chooses the lowest time-integrated bounding-envelope area, with canonical reservation coordinates as the deterministic tie-break. The API `first_fit_heuristic_portfolio` returns `AllPoliciesFailed` when all six policies fail; that is heuristic exhaustion, not proof that the instance is infeasible. The combined API `first_fit_portfolio` then tries a complete integer-grid fallback only for at most four requests in a yard no larger than 5×5 units, with a 100,000-candidate budget. That fallback either emits an independently verified plan, returns `ExactSearchProvenInfeasible` after complete enumeration, or returns `ExactSearchBudgetExceeded` without claiming impossibility. Bigger cases that defeat the heuristics remain `AllPoliciesFailed`. Each individual policy derives a deterministic candidate set from yard and obstacle boundaries and tests candidates in ascending (y, x) order. Reservations may reuse a floor region only when their half-open occupancy intervals do not overlap. Active rectangular exclusion zones block placement. Any invalid input, unsupported exclusion geometry, or unsatisfied request rejects the entire call; no partial plan is returned.

Every output scenario is passed to the independent space-time verifier. The baseline cannot override a violation or mark the result feasible itself.

## Tests

Unit cases cover concurrent non-overlapping placements, temporal reuse of the same space, exclusion-zone avoidance, exhausted-yard rejection, deterministic replay, decreasing-area input-order invariance, portfolio input-order invariance, an independent tick-sweep check of the portfolio's area-time score, and all three exact-search outcomes: verified rescue/proof of infeasibility/budget exhaustion. A separate test verifies that heuristic exhaustion outside the exact-search domain remains `AllPoliciesFailed`, not a proof. Empty input and unsupported exclusion geometry fail closed. A separate integration test independently enumerates all legal integer placements in 216 tiny three-block cases using cell-set collision checks, then compares exact feasibility with both policies. Frozen expected counts are 154 exact-feasible cases and 62 infeasible cases. Caller-order first-fit solves 150 feasible cases, misses four (4/154 ≈ 2.6%), and rejects the 62 infeasible cases. Decreasing-area first-fit is expected to solve all 154 feasible cases, miss zero, and reject the same 62 infeasible cases, with zero false-feasible outputs for both. One known feasible packing that caller-order first-fit misses is explicitly included and is expected to be solved by the decreasing-area policy. These are authored expectations until exact-head CI passes; the 3-block integer-lattice family is deliberately too small to imply general or industrial superiority.

## Generated four-block stress family

The integration test `tests/generated_stress_oracle.rs` uses a pinned xorshift seed (`0x5eed_c0de`) to generate 512 four-block requests on a 5×5 integer yard. An independent 25-bit cell-occupancy recursion decides exact feasibility; it does not use production rectangle overlap or candidate-coordinate generation.

Pinned expected outputs:
- exactly feasible: 205; infeasible: 307;
- caller-order first-fit: 179 solved, 26 feasible cases missed, 307 infeasible cases rejected, zero false-feasible outputs;
- decreasing-area first-fit: 197 solved, 8 feasible cases missed, 307 infeasible cases rejected, zero false-feasible outputs.

The six-policy heuristic portfolio is expected to find 203/205 exact-feasible cases, miss two, and reject all 307 infeasible cases, with zero false-feasible outputs. The two heuristic misses are cases 48 (3×1, 2×2, 2×3, 2×4) and 244 (1×4, 4×1, 3×1, 2×4); exact enumeration confirms both have feasible packings. The combined `first_fit_portfolio` is expected to repair these two with exact search and classify all 307 infeasible cases by complete enumeration within the 100,000-node cap: 205/205 feasible layouts returned, zero false-feasible layouts, and zero budget exhaustions. These are pinned expectations awaiting exact-head Rust CI, and cover only four axis-aligned rectangles on a 5×5 grid with no disruptions. They are not an industrial or SOTA comparison.

## Generated temporal-reuse and active-exclusion stress family

The integration test `tests/generated_temporal_oracle.rs` uses seed `0x51a7_2026` to generate 512 four-block cases on a 4×4 yard. Each reservation uses one of five half-open time windows, and each case has one rectangular exclusion zone with an active interval. The exact oracle recursively enumerates legal cell masks and checks temporal overlap independently of the production geometry helper.

Pinned expected counts:
- exactly feasible: 228; infeasible: 284;
- caller-order first-fit: 168 solved, 60 feasible cases missed, 284 infeasible cases rejected, zero false-feasible plans;
- decreasing-area first-fit: 192 solved, 36 misses, 284 infeasible cases rejected, zero false-feasible plans;
- six-policy portfolio: 216 solved, 12 misses, 284 infeasible cases rejected, zero false-feasible plans.

In this synthetic family, the six-policy heuristic portfolio is expected to find 216/228 = 94.7% of exact-feasible layouts, versus 168/228 = 73.7% for caller-order first-fit. The bounded exact fallback is expected to recover the remaining 12 and prove the other 284 layouts infeasible, yielding 228/228 exact-feasible layouts returned, zero false-feasible layouts and zero budget exhaustions. These are pinned targets awaiting exact-head CI confirmation, not observed industrial results. The family models only rectangles, four placements and one time-active exclusion zone; it omits transfer routes, handling phases, crane/lift physics and irregular polygons.

## Claim boundary

This is a transparent baseline, not a globally optimal bottom-left solver. It does not support irregular polygons, rotation, block transfer routes, cranes, lifting/rigging, multi-capacity vehicles, vertical clearance, stacking, floor bearing, or operational safety. A passing synthetic case does not establish industrial throughput, cost savings or superiority to a published solver.

## Benchmark role

The six-policy heuristic portfolio is a reproducible comparison baseline. For tiny supported cases where all heuristics fail, `first_fit_portfolio` uses a bounded complete search to distinguish a verified rescue, a proven infeasible input, and an exhausted search budget. This does not imply objective optimality: the exact fallback returns the first verified feasible layout within the finite search order. The score still omits transfer access, irregular polygons and unplanned future blocks. The static and temporal generated test targets are described above and remain expected results until exact-head Rust CI passes.

For a directly relevant published comparison, see [PUBLISHED-BASELINES.md](PUBLISHED-BASELINES.md). Wang et al. report 0.775 yard-utilisation score, 308.0 seconds algorithm time, and 2,339 minutes vehicle time for Case9_50_4 with their hybrid ABC/ALNS/recovery algorithm. Our rectangle-only verifier/heuristics do not reproduce that irregular-polygon problem; the numbers are a pinned target to reconstruct, not a head-to-head result.

Use the baseline with the frozen finite-yard corpus, then add versioned generated instances and exact tiny-instance solutions. Report feasibility, usable area-time utilization, re-handles, vehicle travel, makespan/tardiness, recovery time, schedule churn, runtime and memory separately. Compare against exact or bounded solvers and published methods only when the same model assumptions, objective definitions, input data and stopping budgets are reproducible.
