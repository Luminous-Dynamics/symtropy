# Construction Task Compiler — v0.1

**Status:** normalized design/method input schemas, deterministic method-template instantiation and structural graph validation. Not an IFC parser, method authoring tool, robot controller, safety case or construction acceptance authority.

## Purpose

Compile approved design/product facts plus an approved construction-method template into a typed, robot-neutral work graph. Preserve design revision, method-step derivation, transformation-rule identity, parameter source/unit, dependencies, capability requirements, preconditions, postconditions and independent evidence obligations.

This v0.1 package begins at a normalized design manifest boundary. It does not parse IFC directly or infer a construction method from geometry. An upstream IFC/AAS adapter must provide the normalized inputs with explicit source paths, revisions, units and authority references.

## Paradigm shift

A task graph is not merely a sequence of robot actions. It is a traceable, reviewable construction work proposal with a derivation chain:

`approved design element + approved method step + versioned transformation rule -> task node`

Every node binds:
- source element/component identity, task family and design revision;
- approved method template and method step;
- transformation rule ID, version digest and compiler/rule-set digest;
- typed parameter bindings with source references and units;
- preconditions, postconditions and dependency edges;
- capability and resource obligations;
- required physical measurement and independent acceptance evidence.

A generated graph never grants motion authority. A behavior tree or adapter may consume only a graph whose relevant obligations have been resolved by the downstream authorities.

## Research baseline

- McClymonds, Asadi & Leicht (2026) describe the fragmented BIM-to-robot information exchange stack and distinguish planning, decomposition and execution. Their article notes that not every stage is experimentally validated in its case study: [paper](https://www.mdpi.com/3042-4720/2/4/12).
- Brinkhoff, Esser & Borrmann (ISARC 2026) derive task structures from a BIM product graph and parameterized method templates using graph transformations, then map them to Behavior Trees: [paper](https://www.iaarc.org/publications/2026_proceedings_of_the_43rd_isarc_singapore/an_integrated_product-process_representation_for_deriving_robotic_task_structures_using_graph_transformations.html).
- Tan et al. (ISARC 2026) describe a hybrid design/execution digital twin using Rhino/Grasshopper, HAL Robotics and ROS 2: [paper](https://www.iaarc.org/publications/2026_proceedings_of_the_43rd_isarc_singapore/unifying_design_and_execution-a-hybrid_digital_twin_for_robotic_fabrication_in_construction.html).
- AAS / OPC UA virtual commissioning work adds execution-time semantics such as motion commands, status and trajectory logs, but is a software-in-the-loop proof of concept rather than physical acceptance evidence: DOI 10.1016/j.autcon.2026.107095.

These are design/architecture comparators, not a claim that this package is already better than or conformant with them.

## Ownership and composition

- #1834 remains authoritative for robot-to-task capability matching.
- #3561 / #3569 remain authoritative for input frame/revision admission and control-domain coverage.
- #3566 remains authoritative for permission-to-evidence chain composition and safety boundaries.
- #3605 remains authoritative for accepted functional units, inspection dispositions and production denominators.
- #3644 remains authoritative for vendor-neutral interop and mixed-fleet message semantics.
- #3521 / #3522 remain authoritative for physical trial receipts and evidence admissibility.
- Mycelix remains the source of custody/provenance and accountable approval references where applicable.

This package owns only normalized work-graph structure, deterministic dependency validation and compile-time diagnostics. It does not create a second BIM parser, CRS engine, capability matcher, safety evaluator, resource authority, scheduler, fleet manager, evidence ledger or acceptance authority.

## V0.1 method-template compiler and graph validator

The dependency-free Rust crate now includes a bounded Stage 1 compiler over normalized design elements and an approved method-template contract. It instantiates one task node per design-element/method-step pair, preserves source element and transformation-rule lineage, binds typed properties, resolves declared same-element method dependencies, and feeds the resulting graph into the independent structural validator.

The compiler requires an approved method reference, exact design revision/frame/site epoch, explicit task-family applicability, property names/values/units/source references and versioned transformation rule digests. If source and target units differ, a supplied conversion output and external conversion-evidence reference are required; this module does not perform or authorize conversions. Elements and steps are canonicalized by stable IDs before output, and task identities use length-prefixed element/step pairs to avoid delimiter collisions.

The implementation remains a normalized adapter boundary: arbitrary IFC/AAS parsing, geometry processing, capability matching and Behavior Tree lowering are not part of this stage.

The dependency-free Rust crate checks:
- non-empty project/site/work-package/model/revision/frame/site-epoch/method/compiler/rule-set identities;
- an explicit method approval reference;
- unique task-node IDs and unique per-node source identities;
- exact source-element, method-step and transformation-rule lineage;
- non-empty functional-unit class, preconditions, postconditions, capability and evidence obligations;
- typed parameter bindings with units and source references;
- dependency references, self-edges and duplicate edges;
- DAG acyclicity with a deterministic topological order.

The validator reports structural completeness only. It does not authenticate approvals, parse model geometry, prove that a method is constructible, solve robot capability matching, assess safety, or measure physical work.

## Benchmark corpus and target

The structural corpus is in `fixtures/construction-task-compiler-cases-v0.1.json` and defines 45 positive, negative and mutation scenarios. The method-template instantiation corpus is in `fixtures/construction-task-compiler-transform-cases-v0.1.json` and defines 25 cases. The Rust source currently contains 43 tests: 24 graph-validator tests and 19 method-instantiation tests. Fixture expected outcomes are reference targets, not empirical results.

Prospective targets:
- zero false executable graph admissions on frozen hard-negative cases;
- 100% derivation lineage from generated node to source element, approved method step and versioned rule;
- 100% exact agreement with an independent graph oracle on the frozen corpus;
- zero silent inference of units, frames, materials, tolerances, capabilities or approvals;
- 100% mandatory evidence/acceptance-obligation preservation;
- deterministic topological output under input-order permutations;
- zero cycles unless a separately versioned bounded repeat/feedback construct is introduced.

## Deterministic validator implementation

The Rust library validates graph headers, task source/method/rule lineage, parameter unit/source binding, required capability and evidence lists, dependency references, duplicate/self edges, and DAG acyclicity. It returns sorted diagnostics and a lexicographically tie-broken topological order so input-vector permutations do not change the scheduling order. A unit conversion between different source/target units requires an explicit conversion-evidence reference.

The validator deliberately does not calculate geometry, approve transformations, infer a construction method or produce a cryptographic graph digest. A canonical serializer/digest layer and graph rewrite engine are later stages.

## Build and test

```sh
cargo test --manifest-path research/construction-task-compiler-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-task-compiler-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

GitHub Actions pins Rust 1.96 and validates formatting, tests and strict Clippy on Ubuntu, Windows and macOS. The crate has no external dependencies and a committed package lockfile.

## Staging and claim ceiling

V0.1 proves only that declared normalized graph structure and lineage constraints are checked against a deterministic synthetic corpus. It does not parse arbitrary IFC files, generate behavior trees, command robots, establish physical feasibility, approve construction methods, certify safety or accept work.

Later stages require independent IFC/AAS adapters, method-template review, differential graph/behavior-tree semantics, virtual commissioning and finally project-authorized physical trials under the existing safety/metrology/acceptance authorities.
