# Shipyard Distortion Co-Design Benchmark v0.1

Tracking: [SHIPYARD-DISTORTION-CO-DESIGN-001 #4633](https://github.com/Luminous-Dynamics/symtropy/issues/4633)

## Purpose

Benchmark the joint choice of weld-line order, fixture architecture/restraint state, staged release and measurement plan, with production constraints. Score the **final free-state geometry after full cooling and restraint removal**, not only the in-fixture shape.

This is a planning/evidence contract. It does not compute a physical weld-distortion field, choose a qualified welding procedure or authorize fixture/welding changes.

## Ownership

- #2048 owns shipyard DFM, resources, production schedule and accepted-output economics.
- #4614 owns physical welding model VVUQ and source admission.
- #3784 owns generic sensitivity, identifiability, calibration and experiment design.
- #4433 owns exact pass/workpiece/equipment/procedure process traces.
- #4443/#4500 own inspection, NDT, defect/repair lineage and provider/method scope.
- #3331 owns component identity and as-built lifecycle evidence.

No duplicate solver, generic optimizer, model-calibration engine, NDT authority or production release authority is introduced here.

## Research grounding

Salman et al. (2026), *Investigation of altering multi-pass welding sequences and mechanical constraints on stiffened panel distortion*, reports a DH36 panel study with six weld lines / 18 passes, straight-forward, back-step and center-out sequences, free and template-constrained cases. The published abstract reports thermal RMSE 3.97–5.89 °C, mechanical RMSE 0.53–2.11 mm and a study-specific maximum displacement change from 42 mm to 16.5 mm for a combined straightforward/template-constrained case. Measurements were reported after full cooling and restraint removal. These are published abstract results, **not** raw data, a reusable dataset license, a directly reproduced result or a universal benchmark threshold. Exact source data, model setup, measurement definition and rights must be admitted before numerical comparison.

DOI: https://doi.org/10.1016/j.marstruc.2026.104146

Mancini & Skriko's open-access review (published 7 November 2026) identifies large-assembly transfer, calibration, evolving restraints, data availability and non-comparable validation methods as persistent gaps; it calls for common benchmark structures and open reference datasets. The review focuses on distortion, not residual-stress prediction. DOI: https://doi.org/10.1007/s40194-026-02695-6

## Paradigm: restraint and release are time-varying process states

A fixture is not a static clamped/unclamped Boolean. Preserve fixture identity/revision, locating datum, contact/constraint map, clamp state and measurement provenance, installation/engagement/tack/partial-release/full-release events, assumed stiffness/contact/friction and uncertainty, access/collision/handling restrictions, and exact measurement milestones.

The planning objective is the free-state geometry after full cooling and full release. In-fixture geometry is useful diagnostic evidence but cannot substitute for springback/release response. Fixture reaction/load and residual stress are additional objective/constraint dimensions only where qualified models and measurements exist.

## v0.1 synthetic contract

Each case binds panel identity/configuration, weld-line sequence/precedence, fixture identity/revision and reaction limit, ordered release events, final free-state measurement bound to the same panel revision/fixture/datum/coordinate frame, model identity/revision/scope/status, online information boundary, resource feasibility, objective vector and no-writeback boundary.

The first evaluator only checks the contract and declared synthetic toy-envelope facts. Its units are **symbolic synthetic units**, not millimetres, MPa, clamp force or physical measurements.

## Independent evaluator rules

A case is valid only if:
1. all weld tasks are present and unique;
2. every precedence edge is respected;
3. declared fixture reaction does not exceed the synthetic contract limit;
4. release history has strictly increasing event order and a final state after cooling with all restraints removed;
5. final measurement is at final free state, uses an explicit datum and carries uncertainty metadata;
6. model is explicitly synthetic-contract-only, in declared scope and makes no physical qualification claim;
7. schedule uses no future information and contains no resource conflict;
8. objective vector preserves final geometry, production time, fixture cost and worst-case robustness separately;
9. evaluator does not write synthetic state to canonical as-built records.

This is not a full mechanical optimizer. It is the fail-closed admission gate before any optimizer or qualified physics model is connected.

## Baselines and future experiment

The next stage compares fixed straight-forward, back-step and center-out heuristics; fixture-free and fixed-template policies; sequence-only, fixture-only and joint co-design; and robust scenario-based plans. Exhaustive enumeration is the exact oracle for small toy cases. All methods get identical inputs, model outputs, constraints, observed prefixes, compute budget and evaluation set.

For larger planning cases, integrate resource scheduling from #2048. Use #3784 for generic sensitivity/identifiability/DOE and #4614 for any physically qualified predictions. Do not implement a second generic DOE or solver.

## Metrics

- Exact feasibility and optimality agreement on enumerable toy cases.
- Sequence, fixture and release-state constraint violations.
- Final post-cooling/post-release geometry error once a measured case is admitted.
- In-fixture vs final free-state discrepancy.
- Fixture reaction and release transient where measured/qualified.
- Worst-stratum and robust feasibility under frozen uncertainty scenarios.
- Accepted panels/joints per labour-hour and cell-hour.
- Fit-up-to-final-release inspection time; setup, welding, cooling, release, metrology, rework and reinspection time.
- First-pass/final yield and cost per accepted output with identical inspection denominators.

Aspirational target: at least 5% improvement on one preregistered held-out quality-adjusted production or free-state geometry measure at matched resource/compute budget, with no protected constraint violation or deterioration in other predeclared critical outcomes. This is a research target, not an achieved result.

## Frozen controls

The 31-case corpus contains two positive contract controls and 29 negative controls covering invalid sequences, precedence errors, fixture over-limit, missing cooling/release, in-fixture substitution for final measurement, missing datum/uncertainty, OOD or unqualified physical model claims, future leakage, resource conflict, objective collapse/omission, and synthetic-to-as-built writeback.

## Exact enumerable oracle added — 2026-11-10

A separate symbolic oracle file covers four tiny problems with 2–4 weld tasks, precedence constraints, fixture alternatives and release alternatives. The Rust CLI enumerates every feasible task permutation × approved/load-feasible fixture × fully cooled/full-release policy, computes four separate integer objectives, and independently reconstructs the Pareto set:

- final free-state geometry units;
- production time units;
- fixture lifecycle-cost units;
- robustness-risk units.

The frozen expected results are 4 cases, 37 feasible plans and 15 Pareto plans. A fourth case has a precedence cycle and correctly yields zero feasible plans. The oracle is a known-answer software test only: its integer quantities are symbolic, not mm, MPa, weld distortion, clamp force or industrial production measurements.

Run the independent oracle with:

```sh
cargo run --locked -p shipyard-distortion-co-design -- --oracle
```

The focused CI workflow runs this command separately from the 31-case evidence-contract verifier. The oracle tests exact enumeration and Pareto-set reproduction; larger optimization cases will still require matched baselines, valid physics outputs and measured production data.

## Claim ceiling

A PASS establishes only conformance to the declared synthetic planning/evidence contract. It does not predict physical distortion, validate a fixture, qualify a welding process, accept an inspected component, establish residual-stress acceptability or authorize production.


Lockfile provenance: the standalone Cargo.lock was generated by pinned Cargo 1.96.0 on the branch. The focused workflow retains `--locked` for tests, Clippy, both corpus verifiers and the exhaustive oracle.
