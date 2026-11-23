# Orbital Truss Inspectability Benchmark v0.1

**Benchmark:** ORBITAL-TRUSS-INSPECTABILITY-001  
**Evidence class:** synthetic  
**Claim ceiling:** synthetic 2D geometric observability only

This crate evaluates whether declared 2D point features have geometrically admissible inspection viewpoints under a frozen, explicit set of constraints. Each case names one common `coordinate_frame_id`; every feature point, view origin, occluder endpoint, and pose-translation perturbation is expressed in that right-handed planar frame. Coordinates and translations are metres, headings and heading perturbations are radians measured counter-clockwise from +X, and feature normals are unit outward vectors. The frame ID binds the assumption but does not establish registration to an external or physical datum. It is a first executable slice of Symtropy issue #6845, not a structural or NDE qualification system.

## What v0.1 evaluates

For each feature/view pair:

- Euclidean sensor-to-feature standoff against declared minimum and maximum distances;
- angular field of view against the declared view heading and half-angle;
- incidence angle between the feature's unit outward normal and the feature-to-sensor direction;
- 2D line of sight against closed occluder segments;
- nominal visibility and visibility under every declared finite sensor-pose perturbation;
- declared failure-domain diversity, counted by distinct failure_domain_id values.

A view is **robustly visible** only when it passes nominally and under every listed perturbation. This means only the finite enumerated perturbation set (1–256 samples), not every point in a continuous uncertainty region or a probability distribution. A feature is **robustly covered** only when the required number of distinct declared failure-domain labels remain robustly visible.

Occluder endpoint contact is conservatively treated as blocked. The evaluator does not silently infer an unlisted view, access path, sensor capability, defect type, or measurement result.

## Outcomes

- **FEASIBLE:** every declared point feature meets its nominal and finite-perturbation coverage contract.
- **INFEASIBLE:** at least one feature lacks nominal coverage or the required number of distinct declared failure-domain labels.
- **INDETERMINATE:** nominal coverage is sufficient, but the declared perturbation set does not preserve the required diversity.

These are plan-screening outcomes for the declared synthetic 2D geometry only. The count of distinct failure-domain labels is an input-level diversity check; it does **not** prove statistical independence, calibration independence, or separate physical failure modes.

## Reproduction

    cargo fmt --manifest-path tools/orbital-truss-inspectability-bench/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/orbital-truss-inspectability-bench/Cargo.toml
    cargo clippy --locked --manifest-path tools/orbital-truss-inspectability-bench/Cargo.toml --all-targets -- -D warnings

    cargo run --quiet --locked --manifest-path tools/orbital-truss-inspectability-bench/Cargo.toml --bin orbital-truss-inspectability-bench -- docs/research/orbital-truss/fixtures/inspectability-v0_1.json <40-hex-source-commit>

    python3 tools/orbital-truss-inspectability-bench/tests/inspectability_reference_oracle.py docs/research/orbital-truss/fixtures/inspectability-v0_1.json

The CI workflow replays the Rust receipt twice, independently evaluates the frozen corpus in standard-library Python, compares all case-level dispositions/reasons/visibility flags/domain counts, and retains exact-head manifests and artifacts.

## Paradigm shift: optimize for evidence, not just geometry

The longer-term problem is not merely “can a sensor see a point?” It is whether the design makes its critical properties independently observable throughout manufacture, assembly, inspection, repair and later service.

The next versions should compose, rather than duplicate, existing owners:

- #6811 / #6841: as-built geometry and external datum registration;
- #6845: orbital-structure inspectability and design trade-offs;
- #6846: separate 3D datum registration and covariance-aware metrology decisions;
- #4165: aerospace NDE/SHM methods and sensing observability;
- Mycelix #6390: adjudicated AP242/QIF interoperability benchmark and evidence identity;
- Symtropy solver/physics work: qualified structural response; never infer stiffness, strength or stability from this visibility evaluator.

The intended assurance thread is:

    design identity -> inspection requirement -> candidate access/view plan
      -> observed feature -> method-specific measurement
      -> uncertainty-aware decision -> article disposition

A visible feature is not necessarily measurable. A measurable feature is not necessarily defect-detectable. A defect-detectable configuration needs method-specific physical evidence before any probability-of-detection claim.

## Research and industry boundary

NASA Langley's Design for Inspection work already proposes a simulation-driven inspectability metric, parameter-space interrogation, defect-findability tests and sensitivity-to-variability tests. This benchmark does not reproduce or supersede NASA's metric. It implements a disclosed, smaller geometric contract that can later be compared against published methods:

- NASA NTRS 20240003953: https://ntrs.nasa.gov/citations/20240003953
- NASA NTRS 20240007593: https://ntrs.nasa.gov/citations/20240007593

For interoperability, use NIST's STEP File Analyzer and CAx-IF recommended practices as specialist references; do not replace their parsing, PMI analysis or conformance tooling. The actual CAx-IF downstream-use Test Cases 7 and 8 should be consumed through the adjudicated AP242/QIF work in Mycelix #6390 once source-package rights and exact artifacts are established.

For measurement decisions, map applicable profiles to ISO 14253-1:2017 and the JCGM GUM suite, including JCGM 100 Amendment 1:2026, JCGM 101 Monte Carlo propagation and JCGM 106 conformity assessment. None of those metrology uncertainty models is implemented here. For NDE, ASTM E2862-23 concerns hit/miss POD analysis for a defined examination configuration; this v0.1 geometry score is not POD data.

## Benchmark ladder

| Stage | Comparison | Evidence to publish |
|---|---|---|
| G0 | Frozen positive, negative, endpoint, diversity and robustness controls | Zero oracle mismatches; every missing/blocked feature remains visible |
| G1 | Naive fixed views vs post-hoc manual plan vs greedy next-best-view | Same geometry, feature set, sensor constraints and setup budget |
| G2 | Nominal vs finite-perturbation robust plans | Coverage retained, fragile-feature count, blind-zone geometry and sensitivity by perturbation family |
| G3 | Single-objective vs Pareto-aware inspectability design search | Full frontier for structural solver outputs, mass, manufacturability, access, setup count and inspection effort |
| G4 | AP242/QIF identity and datum hand-offs | Zero silent loss or misassociation of required feature/datum/PMI/result relations |
| G5 | 3D sensor geometry and covariance-aware registration | Analytic/Monte Carlo agreement, interval coverage and explicit abstention |
| G6 | Method-specific physical inspection studies | Traceable artifacts, independent reference truth, pre-registered POD/false-call analysis where applicable |

Comparisons must use matched candidates, constraints, sensor models, failure scenarios and budgets. Report coverage vectors and denominators, not one opaque weighted score. Preserve unsupported/unknown states. Publish Pareto fronts rather than silently choosing trade-off weights.

## Explicit non-claims

v0.1 does **not** establish:

- 3D visibility, 6-DoF access, physical tool reach or fixture collision freedom;
- defect findability, sensitivity, probability of detection, false-call performance or NDE capability;
- instrument calibration, metrological traceability or measurement uncertainty;
- statistical independence from different failure-domain labels;
- structural stiffness, strength, fatigue, buckling, load-path adequacy or joint health;
- NASA metric equivalence, ISO/JCGM conformity, certification, flight readiness or release authority.

The fixture is synthetic, not a physical trial. Receipts set authority_granted=false, structural_acceptance_authorized=false, flight_authorized=false and physical_validation=NOT_ASSESSED.


## Frozen v0.1 input bounds

The corpus contains exactly 12 uniquely identified cases. Each case must supply a non-empty coordinate-frame identifier. Each view must supply 1–256 finite pose perturbations. Runtime validation enforces these bounds independently of the schema and CI preflight.


## v0.2 research increment: evidence-qualified viewpoint selection

planning.rs composes the existing geometry result into a small exact set-multicover planner. It accepts an explicit positive integer cost for every candidate view, an inspection-view count limit, and a total cost limit. It does not generate or assume viewpoints: candidate IDs and their nominal/robust feature coverage come from evaluate_case.

For up to 20 candidate views, the planner exhaustively enumerates all subsets and reports the exact robust-feasible optimum, a cost/count Pareto frontier, a marginal robust-coverage-per-cost greedy baseline, and a cheapest-first baseline. The exact objective is lexicographic: total cost units, then view count, then sorted view IDs. The greedy ratio uses integer cross-multiplication rather than floating-point comparison. Duplicate declared failure-domain labels never count as independent coverage.

The planner separates four outcomes:

- FEASIBLE: a robust plan exists inside the declared view and cost budgets;
- INFEASIBLE: even the full candidate set lacks nominal coverage or declared domain diversity;
- INDETERMINATE: nominal full-candidate coverage exists but robust full-candidate coverage does not;
- BUDGET_INFEASIBLE: full-candidate robust coverage exists, but no robust plan fits the declared budgets.

The current unit tests include an adversarial greedy trap (exact plan: two views; greedy: three), duplicate-domain rejection, nominal-only abstention, and budget/geometric separation. This is an abstract coverage-matrix algorithm test, not an executed 3D inspection benchmark and not a claim of planner superiority over published methods.

The API problem_from_case / plan_case binds costs to an existing evaluated Case. A subsequent benchmark increment should freeze an external planning fixture, implement a separately authored Python oracle, and add candidate-generation, routing, reachability and uncertainty-family holdouts before any SOTA comparison.

All receipts remain synthetic-only and hard-code no authority, no structural acceptance, no flight authorization, and physical_validation=NOT_ASSESSED.


## Frozen planning corpus v0.1

The planner has a second, independent corpus: ORBITAL-TRUSS-PLANNING-002 with five synthetic coverage-matrix cases. The Rust CLI writes a deterministic planning receipt; a separate Python standard-library oracle independently enumerates the same subsets and reconstructs the exact optimum, Pareto frontier, greedy and cheapest-first baselines, budgets, dispositions and cost gaps.

Run:

    cargo run --quiet --locked --manifest-path tools/orbital-truss-inspectability-bench/Cargo.toml --bin orbital-truss-inspection-planning-bench -- docs/research/orbital-truss/fixtures/planning-v0_2.json <40-hex-source-commit>
    python3 tools/orbital-truss-inspectability-bench/tests/planning_reference_oracle_v0_2.py docs/research/orbital-truss/fixtures/planning-v0_2.json <40-hex-source-commit>

The corpus covers a greedy trap, budget infeasibility despite sufficient candidate geometry, duplicate declared failure-domain collapse, robust-coverage abstention, and a cost-versus-view-count Pareto frontier. These are small, synthetic algorithm controls, not realistic inspection tasks or physical evidence. The workflow replays the Rust CLI, compares the complete Rust and Python receipts, asserts the intended counterexamples, and retains both receipts with the fixture, schema and exact-head manifest.


## v0.2: keep k-view coverage and failure-domain diversity orthogonal

Planning corpus v0.2 adds two independent requirements per feature:

- \`required_view_count\`: the number of distinct candidate viewpoint IDs that must robustly observe the feature;
- \`required_failure_domains\`: the number of distinct declared failure-domain IDs represented among those observing views.

These constraints are not interchangeable. Two camera poses can satisfy k-view coverage while sharing a sensor/calibration dependency. Conversely, a feature may require multiple declared domains even when its view-count demand is one. The v0.2 receipt reports nominal and robust counts for both axes, for each selected plan and for the full candidate pool.

Most importantly, a requested k is never silently capped to the number of available candidates. If a feature requests three views but only two candidate views observe it, the full candidate pool is reported INFEASIBLE with an explicit 2/3 coverage record. Candidate generation/augmentation must address the deficit; the evaluator does not redefine the requirement.

The frozen v0.2 corpus has eight cases: the v0.1 five adversarial cases, k-view coverage within one declared domain, domain diversity with a one-view demand, and an under-provisioned candidate pool. The old v0.1 fixture/oracle remains as a historical contract snapshot. The new Rust receipt is compared in full with a separate standard-library Python oracle.

Run the new contract:

    cargo run --quiet --locked --manifest-path tools/orbital-truss-inspectability-bench/Cargo.toml --bin orbital-truss-inspection-planning-bench -- docs/research/orbital-truss/fixtures/planning-v0_2.json <40-hex-source-commit>
    python3 tools/orbital-truss-inspectability-bench/tests/planning_reference_oracle_v0_2.py docs/research/orbital-truss/fixtures/planning-v0_2.json <40-hex-source-commit>
