# habitat-scheduling-planner

A deterministic heuristic resource-constrained scheduling baseline over the synthetic habitat schedule contract. This is intentionally separate from the exhaustive micro-oracle, which remains the small-case correctness reference.

The planner tries five deterministic topological priority rules (lexicographic, critical-path, earliest-latest-start, longest-duration, shortest-duration). For each, it builds an earliest-feasible nonpreemptive schedule under precedence, resource capacity, task windows, hard gates, and optional hard deadline constraints. Every returned candidate is passed through a separate schedule feasibility validator that recomputes durations, precedence, windows, gates, resource occupancy, and deadline compliance.

Status semantics are deliberately conservative:
- `HEURISTIC_FEASIBLE_NOT_PROVEN_OPTIMAL`: one candidate was found and independently validated; optimality is **not** claimed.
- `PROVEN_INFEASIBLE_PRECHECK`: a direct hard-constraint/lower-bound proof rules out feasibility.
- `NO_CANDIDATE_FOUND_NOT_PROOF_OF_INFEASIBILITY`: the tried priority rules found no candidate; this is not a proof that none exists.
- `INTERNAL_VALIDATOR_DISAGREEMENT`: treat as a correctness failure; do not use the candidate.

Robustness is separately evaluated against the declared upper-duration scenario. It does not mean probabilistic reliability or physical safety certification. The executable currently consumes the nine-case synthetic micro-oracle fixture, and the fixture's expected results are not used by the scheduling algorithm.

From the repository root:

- cargo fmt --manifest-path tools/habitat-scheduling-planner/Cargo.toml -- --check
- cargo test --locked --manifest-path tools/habitat-scheduling-planner/Cargo.toml
- cargo run --locked --manifest-path tools/habitat-scheduling-planner/Cargo.toml -- docs/research/fixtures/HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json

The model uses integer time and half-open intervals. This is an experimental scheduling baseline, not a global optimizer or construction planning authority.
