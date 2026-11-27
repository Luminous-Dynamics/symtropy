# Preservation Intervention Portfolio Reference

A zero-dependency Rust reference planner that selects evidence-qualified, non-executing intervention portfolios under hard cost, energy, duration and downtime limits.

## Safety and authority boundary

The planner:
- consumes unresolved alert IDs from Preservation Watch;
- requires verified action-bound upper-bound estimates and available prerequisite evidence;
- checks action dependency closure, budgets, currency and source retention;
- returns a deterministic Pareto frontier and the approvals/postconditions needed for each plan.

It does **not** authenticate bounds receipts, measure cost/energy/time, authorize interventions, execute actions, write or erase media, release keys, or clear alerts. "Addresses alert" is only a prospective planning relation. A separate independent remediation receipt must be consumed by Preservation Watch before an alert clears.

All generated plans set `auto_execute=false` and preserve original source copies until an independently accepted destination exists. Destructive/source-discarding candidates are excluded.

## Exact v0.1 search

Candidate set is capped at 16 actions. The reference implementation enumerates all non-empty subsets, enforces complete alert coverage, dependency closure, available prerequisites, action-count and four resource budgets, then returns the exact non-dominated portfolios.

Objectives minimized:
1. cost upper bound;
2. energy upper bound;
3. elapsed-duration upper bound;
4. downtime upper bound;
5. action count as a tie-breaking efficiency dimension.

The planner does not emit a weighted scalar score. It sums duration and downtime upper bounds conservatively; no concurrency or schedule optimization is claimed. A future parallel scheduler must be a separately qualified implementation that reproduces the frozen exact frontier when concurrency is disabled.

## Evidence model

Each candidate must carry a non-empty bounds receipt ID and SHA-256-shaped digest, plus externally supplied provenance status. The planner trusts the external verifier's Verified/Unverified/Invalid classification; it does not authenticate the receipt itself.

- Verified bounds may be considered.
- Unverified bounds are excluded and can produce EvidenceIncomplete.
- Invalid bounds are excluded and, when no feasible plan remains, produce InvalidEvidence.
- Missing prerequisite evidence excludes the candidate and can produce EvidenceIncomplete.

All values are synthetic in the initial fixture. They are not measured estimates or a service quote.

## Local commands

~~~bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
~~~

The authoring environment has no Cargo, rustc or rustfmt, so the authored Rust tests have not yet been compiled or executed.

## Files

- `../PRESERVATION_INTERVENTION_PORTFOLIO_001.md`: protocol, semantics and evidence boundary.
- `../preservation_intervention_portfolio_cases_v0_1.schema.json`: frozen case schema.
- `../fixtures/preservation_intervention_portfolio_cases_v0_1.json`: synthetic known-answer corpus.
