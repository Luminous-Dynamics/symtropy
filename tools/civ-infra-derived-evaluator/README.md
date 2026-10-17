# CIV-INFRA Primitive-Derived Evaluator v0.1

Standalone Rust evaluator that derives synthetic civil-corridor findings from **primitive design and scenario inputs**, not caller-supplied booleans or derived metric maps.

## Run

\`\`\`sh
cargo fmt --manifest-path tools/civ-infra-derived-evaluator/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civ-infra-derived-evaluator/Cargo.toml
cargo run --locked --manifest-path tools/civ-infra-derived-evaluator/Cargo.toml -- tools/civ-infra-derived-evaluator/fixtures/derived-v0.1.json
\`\`\`

## Derived synthetic rules

- \`grade_limit\`: candidate grade <= versioned maximum grade.
- \`earthworks_budget\`: candidate earthworks <= versioned budget.
- \`hydraulic_capacity\`: declared synthetic drainage capacity >= base demand × scenario multiplier.
- \`utility_clearance\`: baseline vertical clearance − scenario encroachment >= versioned minimum clearance.

The evaluator emits per-state findings with observed quantity, comparator, threshold, primitive input references, and equation. It binds the exact fixture bytes and serialized ruleset with SHA-256. Unknown fields, wrong units, unsupported rule versions, missing/duplicate state combinations and non-finite derived arithmetic are rejected.

The four state combinations are exhaustively checked from two finite supports (storm multipliers 1.0/1.5; synthetic utility encroachment 0.0/0.2 m). R0-style interval uncertainty, correlated state sets and probability weighting are not supported here.

## Expected synthetic fixture outcomes

| Candidate | Disposition | Explanation |
|---|---|---|
| A-shortest | Robust-infeasible for declared synthetic set | Grade exceeds the rule limit in every state. |
| B-balanced | Robust-feasible for declared synthetic set | All four derived constraints pass in all states. |
| C-fragile-clearance | Conditional | Clearance passes under low encroachment and fails under high encroachment. |
| D-resilient | Robust-feasible for declared synthetic set | All four derived constraints pass in all states. |
| E-under-capacity | Robust-infeasible for declared synthetic set | Drainage capacity is below even the base declared demand. |

The expected Pareto set is B-balanced and D-resilient: B has shorter length and lower earthworks, while D has more drainage and clearance margin.

The test suite runs 625 deterministic primitive-candidate variants (12,500 candidate-state evaluations) against an independent test-only rules oracle and Pareto calculation. In CI, `CIV_INFRA_DERIVED_CORPUS_OUT` enables a machine-readable manifest containing the full generated input JSON, per-input SHA-256, generator seed/algorithm/mutator, evaluator version, oracle/evaluated dispositions and Pareto sets. The workflow uploads it as a 14-day artifact. The snapshots are retained in the artifact rather than checked into source control.

## Claim ceiling

All data and rules are synthetic. The evaluator is not a terrain, utility-detection, hydraulic, geotechnical or civil-design solver. It does not establish model calibration, field validity, real-world safety, standards conformance, permitting, cost savings or construction authority. EPA SWMM should be used for real drainage simulation, with model provenance and diagnostics separately qualified; its manual says a successful run can still have excessive continuity error and warns that some numerical flow instabilities are not detected automatically.

Research references:
- [NIST, risk-based uncertainty quantification / model credibility (2026-10-05)](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing)
- [EPA SWMM 5.2 User's Manual](https://nepis.epa.gov/Exe/ZyPURL.cgi?Dockey=P10145M6.TXT)
- [CIV-INFRA-ORACLE-002 #1777](https://github.com/Luminous-Dynamics/symtropy/issues/1777)
