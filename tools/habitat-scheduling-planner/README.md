# habitat-scheduling-planner

A deterministic hybrid resource-constrained scheduling baseline over the synthetic habitat schedule contract. The exhaustive micro-oracle remains the tiny-instance correctness reference. This crate is not a production or globally optimizing scheduler.

The planner first tries five deterministic topological priority rules (lexicographic, critical-path, earliest-latest-start, longest-duration, shortest-duration). Every candidate is checked by an independent validator that recomputes durations, precedence, windows, hard gates, resource occupancy, and deadline compliance.

If all five priority rules fail to construct a schedule, the planner runs a **bounded topological-order recovery search**. It explores alternative prerequisite-valid task orders, applying the same earliest-feasible serial schedule generator, with a hard budget of 20,000 expanded partial orders. The report preserves the priority-only result separately from the fallback outcome. A found fallback candidate is still not a proof of global optimality.

Status semantics are deliberately conservative:
- `HEURISTIC_FEASIBLE_NOT_PROVEN_OPTIMAL`: a priority rule found a validated candidate; no optimum is claimed.
- `BOUNDED_FALLBACK_FEASIBLE_NOT_PROVEN_OPTIMAL`: the priority-only rules missed, but bounded recovery found and independently validated a candidate.
- `PROVEN_INFEASIBLE_PRECHECK`: a direct hard-constraint or valid lower-bound check proves infeasibility.
- `NO_CANDIDATE_FOUND_NOT_PROOF_OF_INFEASIBILITY`: the priority-only rules found no candidate; fallback then runs.
- `FALLBACK_SEARCH_INCOMPLETE_NO_CANDIDATE_NOT_PROOF_OF_INFEASIBILITY`: fallback exhausted its budget without a candidate.
- `FALLBACK_EXHAUSTED_NO_CANDIDATE_NOT_PROOF_OF_INFEASIBILITY`: all enumerated topological orders were checked without a candidate; still not presented as a general infeasibility proof.
- `INTERNAL_VALIDATOR_DISAGREEMENT`: treat as a correctness failure; never use the candidate.

The tenth synthetic micro-case is intentionally adversarial. All five fast priority rules miss a feasible five-task, single-crew schedule. The bounded fallback recovers the exact fixture witness at makespan 19 after 326 search states, but still labels it not proven optimal. The scorecard records both the five-rule baseline's 5/6 feasible-case coverage and the recovery-enhanced planner's 6/6 coverage.

Upper-duration robustness is evaluated separately from nominal durations. It does not imply probabilistic reliability or physical safety certification. The fixture's expected outcomes are not used by the scheduler algorithm.

From the repository root:

- cargo fmt --manifest-path tools/habitat-scheduling-planner/Cargo.toml -- --check
- cargo test --locked --manifest-path tools/habitat-scheduling-planner/Cargo.toml
- cargo run --locked --manifest-path tools/habitat-scheduling-planner/Cargo.toml -- docs/research/fixtures/HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json

This is an experimental hybrid baseline, not a global optimizer or construction planning authority.
