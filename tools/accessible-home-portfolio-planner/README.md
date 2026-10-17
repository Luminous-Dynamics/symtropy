# Accessible Home Portfolio Planner

Standalone Rust prototype for **synthetic-only** staged intervention portfolios. It enumerates subsets of up to 20 interventions, checks dependency closure and permission gates, retains cost ranges and unknown costs, and computes a deterministic Pareto frontier without converting hard constraints into weights.

## Run

From the repository root:

```sh
cargo fmt --manifest-path tools/accessible-home-portfolio-planner/Cargo.toml -- --check
cargo test --manifest-path tools/accessible-home-portfolio-planner/Cargo.toml
cargo run --manifest-path tools/accessible-home-portfolio-planner/Cargo.toml -- \
  --input docs/built-environment/accessible-home-portfolio-v0.1.example.json \
  --output /tmp/accessible-home-portfolio-report.json
```

## Model semantics

- A hard constraint is either already declared satisfied **for the synthetic fixture** or needs at least one selected intervention that declares it as a target. That mapping is planning coverage, not proof of remediation.
- An unresolved hard constraint with no possible declared remediation invalidates the input. A candidate that lacks coverage never enters the budget frontier.
- All dependency graphs must be acyclic; selected plans must be dependency-closed. Independent actions can share a stage. The upper lead-time metric is a critical-path bound, not an implementation promise.
- Capital and annual-maintenance costs are lower/upper ranges in integer minor currency units. The capital budget gate uses the upper bound; ranges that straddle the cap remain separately counted as budget-uncertain. Unknown cost is never substituted with zero and never enters the budget frontier.
- A denied or pending access/permission gate excludes that subset from budget-feasible plans and is counted separately.
- Preferences are optimization criteria, not hard gates. Explicit conflicts and unmet preferences are separately surfaced; their count is used only as one Pareto dimension, not as a universal resident value scale.
- The Pareto frontier minimizes high-end capital cost, high-end annual maintenance, dependency-aware upper lead time, relative disruption points and preference penalty. A plan is removed only when another plan is no worse on every dimension and strictly better on at least one.
- Implementation-state values (proposed, quote received, funded, installed, measured, professionally reviewed) are carried into the report. They are not collapsed into a single "accessible" badge.

## Initial benchmark gates

Proposed acceptance thresholds (not measured here):

- 100% of frontier plans cover every unresolved/conflicted hard constraint with a declared remediation path.
- 100% of plans with pending/denied gates are excluded from the budget frontier.
- 100% of candidates containing unknown cost are excluded until costs are supplied; no missing cost is treated as zero.
- 100% of dependency cycles and unknown references fail closed.
- 0 strictly dominated plans in the Pareto frontier.
- Identical input bytes produce identical output under a pinned build.
- Public fixture contains invented IDs and synthetic costs only; no actual household evidence or prices.

The current exhaustive search cap of 20 interventions means at most 1,048,576 subsets. This is intentionally auditable for the first prototype. Benchmark 5, 10, 15 and 20 interventions before raising the limit or adopting more complex optimization. Report median/p95, memory, compiler/build, input digest and exact environment.

## Limits

All numbers in the committed fixture are fabricated scenario values, not quotes, market prices or recommendations. "Covers a constraint" means the intervention was mapped to that constraint by the synthetic input, not that it resolves it in reality. This prototype does not certify building safety, code compliance, physical accessibility, affordability, funding, resident suitability or clinical efficacy. Real use requires resident direction, current attributable prices, site measurement, tenancy/permission verification and qualified local professional review.
