# habitat-scheduling-benchmark

A reproducible, ten-case scorecard that runs two separate implementations over the same frozen synthetic scheduling fixture:
- the bounded exhaustive micro-oracle, which supplies exact reference outcomes only for tiny instances;
- the deterministic hybrid planner, which first tries five priority rules, then uses bounded topological-order recovery if all five miss.

The report labels the scorecard scope as the nominal-duration scenario. It reports the fast priority-only baseline's feasible-case coverage separately from the recovery-enhanced planner's coverage, counts recovered candidates and fallback search states, compares validated candidates with exact makespans, and records any false infeasibility claims or conflicts against the exact reference. An instance missed by the priority-only rules is not counted as infeasible. The scorecard pins the original input digest and canonical digests of the oracle and planner reports.

On the ten-case micro-suite, the frozen expectations are:
- 6 cases are feasible under the exact reference.
- Five priority rules alone produce 5 candidates (5/6 feasible-case coverage) and miss the adversarial sixth feasible case.
- Bounded topological-order recovery explores 326 states and recovers the missed candidate.
- The recovery-enhanced planner produces validated candidates for all 6 feasible cases (6/6 coverage).
- The relative makespan gap is 0 ppm among the candidates found; no global-optimality claims are made.
- There are zero false infeasibility claims and zero candidate conflicts with the exact reference.

These are small synthetic functional outcomes, not a runtime benchmark or a general resource-constrained project scheduling performance claim.

From the repository root:

- cargo fmt --manifest-path tools/habitat-scheduling-benchmark/Cargo.toml -- --check
- cargo test --locked --manifest-path tools/habitat-scheduling-benchmark/Cargo.toml
- cargo run --locked --manifest-path tools/habitat-scheduling-benchmark/Cargo.toml -- docs/research/fixtures/HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json

All costs, durations, resources, and gates are synthetic. No reported schedule means work was executed; no scorecard establishes safety, permits, code compliance, or a construction estimate.
