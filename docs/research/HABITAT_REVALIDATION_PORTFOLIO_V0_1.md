# Habitat Joint-Target Revalidation Portfolio Benchmark v0.1

**Status:** proposed synthetic planner contract; no real engineering, permit, safety or approval claim. All gate IDs, task costs, and durations are invented.

## Paradigm shift

Move from repairing one evidence claim at a time to **planning a joint set of related outcomes as a shared dependency graph**. A material revision may affect commissioning and maintenance handover; the planner should schedule the shared material review and as-built inspection once, then restore each target only after its own prerequisites and gate are satisfied.

A bounded search has an explicit epistemic status. If the work budget is exhausted, the result is `SEARCH_INCOMPLETE` even when a feasible partial frontier was found. It must not be reported as `OPTIMAL_EXACT` or `NO_VALID_PLAN_PROVEN`. This complements the small single-target planner in `tools/habitat-revalidation-planner`; it is a separate portfolio experiment, not yet a scalable production solver.

## Frozen scenario

Fixture: `docs/research/fixtures/HABITAT_REVALIDATION_PORTFOLIO_V0_1.json`

- Seven evidence nodes cover source, concept, structural analysis, material order, as-built receipt, commissioning receipt and maintenance handover.
- Both commissioning and maintenance handover are target claims.
- Five tasks include two alternative material-review producers and three shared/downstream tasks.
- The search budget is 10,000 expanded states.
- Five abstract gates model procurement review, optional independent QA, inspection signoff, commissioning authority and operations handover.

Expected full-gate Pareto frontier:

| Path | Material review | Shared as-built review | Target tasks | Cost | Duration |
|---|---|---|---|---:|---:|
| Low-cost | low-cost route | once | commissioning + handover | 14 | 9 |
| Faster | QA route | once | commissioning + handover | 17 | 6 |

Costs and durations are sequential arbitrary units, not measured estimates. The shared material review and as-built reinspection must appear once per portfolio; one cannot charge two visits simply because two target claims depend on the same evidence, nor can one silently omit the shared prerequisite.

## Search semantics

The standalone Rust package is `tools/habitat-revalidation-portfolio`.

Statuses:
- `TARGET_ALREADY_VALID`: every target is already valid and the dependency validator accepts the evidence graph.
- `OPTIMAL_EXACT`: search completed inside the budget; the reported Pareto frontier is complete for this finite model.
- `NO_VALID_PLAN_PROVEN`: search completed and found no valid portfolio.
- `SEARCH_INCOMPLETE`: the state budget was exhausted. Any returned plans are provisional, and frontier completeness is false.

The verifier pins exact fixture bytes, rejects unknown fields, missing/duplicate targets, undeclared gates, invalid statuses, cycles, missing prerequisites and tasks whose evidence dependencies do not match the produced artifact's direct dependencies. Every plan step carries the task version, output artifact, required evidence, gates, costs and duration. A plan is always marked `PROPOSED_NOT_EXECUTED`; it does not restore evidence.

## Differential benchmarks

The authored suite includes an independent subset-based oracle with its own nondominance filtering, all 32 combinations of five abstract gates, a budget-exhaustion negative control before finding a plan and a second budget control after finding one feasible plan, shared-prerequisite count controls, ordering invariance, unknown fields and an invalid-dependency control. The exhaustive oracle does not use the production Pareto-frontier function.

Acceptance targets:
- 100% agreement with the independent subset oracle on all 32 gate subsets and the frozen scenario.
- 0 plans with a missing declared prerequisite or unavailable mandatory gate.
- 0 interrupted searches reported as complete or impossible.
- 100% of returned portfolio plans contain every target exactly once at the portfolio level and each shared evidence-producing task at most once.
- 100% deterministic plan/frontier output under semantically irrelevant input ordering.
- 0 claims restored until task outputs actually exist as validated, versioned evidence.

## Next research

Increase to multiple site/design/operations targets, shared prerequisite subgraphs, more than one producer per artifact, and dependency-aware parallel duration bounds. Differential-test any optimized search against exhaustive subset/sequence oracles on seeded small instances, then profile at 100, 1,000 and 10,000 artifacts. Report p50/p95/p99, peak memory, explored states and Pareto-frontier size with the exact fixture digest, machine, and toolchain. Keep exhaustive correctness cases small; if the search limit is hit, return `SEARCH_INCOMPLETE`.

## Research anchors

NASA distinguishes product verification (proof against specified requirements) from validation (whether the product meets its intended use), and recommends explicit verification/validation matrices and change traceability: https://www.nasa.gov/reference/2-0-fundamentals-of-systems-engineering/ and https://www.nasa.gov/reference/system-engineering-handbook-appendix/

NIST's 2026 digital-twins workshop report highlights interoperability, verification/validation/uncertainty quantification and cybersecurity as open challenges in trustworthy, scalable lifecycle models: https://www.nist.gov/publications/digital-twins-workshops-summary-report

These sources motivate traceability and explicit search confidence; they do not validate this invented portfolio or supply real-world planning costs.


## Extended differential stress case

The Rust test suite builds an in-memory third target that shares the existing as-built evidence and adds one abstract gate. It compares all 64 subsets of six gates against the independent subset oracle. The full-gate frontier is expected at cost/duration (16,12) and (19,9). An independent JavaScript calculation matched all 64 subsets; this is not a Rust test execution result.
