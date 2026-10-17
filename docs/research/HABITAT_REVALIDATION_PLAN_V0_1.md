# Habitat Dependency-Safe Revalidation Planner v0.1

**Status:** Proposed synthetic planning benchmark. Task costs, durations, gates and artifacts are invented. No physical, regulatory or professional approval claim is made.

## Paradigm shift

Move from listing invalidated evidence to planning minimum evidence-repair work for a requested claim. The planner preserves every declared direct dependency, respects hard review gates, and distinguishes a proposed task from a validated output. A blocked or incomplete plan cannot be labeled success.

This direction is motivated by systems-engineering change-impact analysis and digital-twin validation practice. NASA's systems-engineering handbook describes verification/validation matrices and traceability; NASA software-engineering change guidance calls for evaluating downstream changes and revalidating affected work products; NIST identifies validation as foundational to trustworthy digital twins. These motivate the contract but do not qualify this synthetic planner:
- https://www.nasa.gov/reference/system-engineering-handbook-appendix/
- https://swehb.nasa.gov/spaces/7150/pages/16449679/SWE-053+Manage+Requirements+Changes
- https://www.nist.gov/digital-twins/validating-and-advancement

## Frozen scenario

Fixture: `docs/research/fixtures/HABITAT_REVALIDATION_PLAN_V0_1.json`

The graph has ten evidence artifacts. Six upstream items remain valid; an illustrative material substitution invalidates material/procurement evidence plus dependent as-built and commissioning receipts. The target is `D-COMMISSIONING-RECEIPT`.

A four-task library offers two ways to review the substituted material: a slower, lower-cost route and a faster, higher-cost route. Both require procurement review; the faster alternative also requires independent QA. Both then require as-built reinspection and recommissioning. Structural analysis is reused only because this fixture explicitly says it remains valid and it is a declared prerequisite.

All cost and duration values are arbitrary test units. Gate identifiers are synthetic tokens, not actual professional permission or approval.

## Executable planner

Package: `tools/habitat-revalidation-planner`

- Pins exact fixture bytes before parsing.
- Rejects unknown fields/statuses, undeclared gates, missing artifacts, dependency cycles, and task prerequisites that do not match the produced artifact's declared direct dependencies.
- Exhaustively enumerates feasible task sequences, then keeps nondominated options on cost and duration instead of hiding preferences behind one score.
- Returns explicit blockers when a required gate is unavailable; no unavailable task is assumed completed.
- Emits per-step task version, produced artifact, required evidence, required gates, cost/duration units and a synthetic-only claim ceiling.
- A separate exhaustive sequence oracle checks the frozen Pareto frontier and all 16 combinations of the four declared abstract gates.

## Expected synthetic Pareto frontier

| Path | Task sequence | Cost units | Duration units |
|---|---|---:|---:|
| Low-cost | Material review → As-built reinspection → Recommission | 12 | 8 |
| Fast | Independent-QA material review → As-built reinspection → Recommission | 15 | 5 |

Neither dominates the other: one costs less, one is faster. The planner returns both until an authorized decision-maker expresses a preference. The plan is only a proposal; an output claim is not restored until the corresponding task is completed and its new evidence independently validated.

## Acceptance targets

- 100% agreement with an independent exhaustive oracle on the frozen finite case.
- 0 plans bypassing a declared direct dependency or hard gate.
- 0 blocked/no-plan cases mislabeled as complete.
- 100% deterministic, input-order-invariant Pareto output.
- 100% output provenance for fixture digest, target claim, task versions, prerequisites, gate assumptions, cost/duration units and claim ceiling.
- 0 synthetic score or gate token interpreted as real-world authorization, safe design, or permit approval.

Next: multiple targets and shared prerequisites; alternative evidence-producing task schemas; uncertainty ranges for task duration; performance tests at 100/1,000/10,000 artifacts; then measured revalidation effort. Real task libraries and gate policies require independent domain review.


## Alternative-producer stress case

The authored suite also augments the fixture in memory with two extra material-review tasks that produce the same evidence node but offer different synthetic cost/duration pairs. The four nondominated paths are independently expected at (11, 10), (12, 8), (15, 5) and (16, 4). A separate sequence oracle checks the planner's frontier. This is an important early stress case for alternative producers and trade-offs; it is not yet a large-graph scaling benchmark, and the Rust test is awaiting CI execution.
