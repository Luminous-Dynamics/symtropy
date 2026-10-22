# ACCESS-HOME-NETWORK-001 — resident task-network contract v0.2

**Status:** research RFC and synthetic data contract. Not a building model, compliance engine, or real-home qualification.

## Why the model changes

The v0.1 evaluator classifies evidence for a task's required capabilities. That is a useful triage baseline, but a flat list cannot adequately answer: *if the preferred path fails, does another path achieve the same resident-defined outcome without violating their constraints?*

Model the home experience as a **resident-owned task network**: nodes are actions, decisions, outcomes or handoffs; edges express sequence, dependency, alternatives, fallback and recovery. Paths are evaluated separately under explicit operating conditions and name both their ordered node IDs and edge IDs so their dependencies and fallback triggers can be audited. A fallback is not considered available merely because a designer has named it.

This fits the built-environment scope of ISO 21542 (access, circulation, egress and evacuation) while leaving geometric and life-safety determinations to qualified reviewers. The ISO status page currently lists ISO 21542:2021 as published and records the September 2026 review milestone; confirm its current status and local legal requirements for every live project. WHO's housing guidance considers real-world outcomes including injury, well-being, independence and social participation. South Africa's Universal Design and Access framework supplies important local policy context, not an approval certificate.

## Artifacts

- accessible-home-task-network-v0.2.schema.json: JSON Schema Draft 2020-12 contract for a synthetic task network.
- accessible-home-task-network-v0.2.example.json: one synthetic goal with a primary network-dependent path and a locally operated alternative for a declared network-unavailable condition.

## Core entities

- **Resident goal:** stable identity, resident-defined outcome and explicit constraints. A category or diagnosis must not substitute for the resident's goal.
- **Node:** action, decision, outcome or handoff. Required nodes carry declared-scope evidence, a source reference, a reviewer role and a note.
- **Edge:** sequence, dependency, alternative, fallback or recovery. Fallback/recovery edges require an explicit trigger and their own evidence state.
- **Path:** an ordered candidate for reaching the *same goal* under a named operating condition. Each path retains its own assessment.
- **Failure mode:** an explicit condition that activates a degraded-mode path, such as network unavailability. The condition and its alternative must not be assumed to be tested in a real house.

## Fail-closed path semantics

These semantics are proposed for the eventual Rust evaluator; the schema validates shape but does not itself evaluate feasibility or validate all cross-references.

1. A path is blocked_by_declared_barrier if a required node/edge on that path has an explicit known barrier.
2. Otherwise, a path is not_evaluable if required evidence is unknown, proposed, needs site measurement or needs professional review.
3. Only when all required path evidence is explicitly verified_for_declared_scope may the result be evidence_complete_for_fixture. That label is synthetic-fixture-only.
4. Preserve assessments of *all* alternatives. A blocked primary path remains visible even if another path is complete.
5. The goal-level view must not collapse to one unqualified score. It should expose at least: the resident goal, all path assessments, the active condition, constraints satisfied/unresolved/conflicted, and the evidence needed to resolve uncertainty.
6. A declared fallback counts only if it targets the same resident-defined outcome, honors hard resident-selected constraints and has its own required steps/evidence. A merely named fallback is not resilience.
7. The synthetic contract must never be accepted as proof of real-world dimensions, usability, accessibility, code compliance, fire/egress safety or suitability.

## Failure injection and benchmark ladder

### Contract and evaluator properties

- 100% of valid schema examples parse; malformed enum values and missing required fields fail.
- 100% of duplicate node/path/edge IDs and dangling from/to/node references are rejected by the semantic validator (JSON Schema alone cannot guarantee ID uniqueness across arrays or target existence).
- Exhaustively test the finite state matrix for required versus advisory evidence: a required known barrier blocks that path; unknown/proposed/measurement/review-pending never becomes fixture-complete; optional advisory barriers do not silently become hard constraints.
- Deleting a required edge or node, or changing a required verified state to unknown, must never improve the path assessment.
- A failure-mode fixture must activate the declared alternative and preserve the failed primary path's result.
- Identical input bytes and a pinned Rust build must produce the same receipt digest and canonicalized report.
- Zero real-home or statutory-compliance claims from synthetic results.

### Human-centered measures, only after participant co-design

Do not preselect one universal pass percentage. Co-design tasks and thresholds with compensated participants whose access needs overlap in varied ways. Report a vector of outcomes, by task and adequately sampled cohort: task completion; path/fallback availability; unrecovered blockage; time and recovery time; errors or near-misses; resident-reported effort, autonomy and confidence; and requested assistance. Publish uncertainty and adverse outcomes. Do not let averages hide a subgroup or individual for whom every path fails.

### Staged evidence

1. **Schema/contract:** shape validation, cross-reference and uniqueness validator, malformed-data negative controls.
2. **Deterministic evaluator:** exhaustive evidence-state matrix, path-local assessment, conditions, precedence and receipts.
3. **Mutation corpus:** remove dependencies; flip evidence to unknown/barrier; delete the fallback; activate network/power/device/support-unavailable conditions; introduce conflicting resident constraints.
4. **Resident co-design:** validate that goal/outcome and constraints reflect participants' own language and choices; test alternatives in safe mock-ups.
5. **Site trials and professional review:** actual measurements, local jurisdiction, accessibility specialists and life-safety authority where relevant. The tool does not replace these.

## Integration boundary

This contract composes with issue #1687 (resident-led planner), PR #1688 (Leptos requirement capture), PR #1596 (housing evidence/jurisdiction work), and issue/PR #1695/#1696 (v0.1 evidence triage). v0.2 describes path structure and failure conditions; the v0.1 evaluator continues to own its narrow synthetic evidence-state rules until a separately reviewed implementation is merged.

## Research sources

- ISO 21542:2021, Building construction — Accessibility and usability of the built environment: https://www.iso.org/standard/71860.html
- WHO, Housing and health guidelines, including housing accessibility and functional impairment outcomes: https://www.who.int/publications/i/item/9789241550376
- South African Government, National Strategic Framework on Universal Design and Access: https://www.gov.za/ss/node/809572

These are context sources, not a claim of compliance. Verify current standards and local law before applying this contract to a real project.
