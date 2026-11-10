# Scan-to-BIM decision-capture adapter P0 v0.1

**Status:** executable adapter proposed; synthetic corpus only.  
**Parent:** [#4207 SCAN-BIM-DECISION-CAPTURE-001](https://github.com/Luminous-Dynamics/symtropy/issues/4207).  
**Build dependency:** [#1849 spatial-resilience qualifier](https://github.com/Luminous-Dynamics/symtropy/pull/1849).  
**Composes:** #1892 decision-impact; #1922 adaptive policies; #1607 survey/pose/frame; #4292 geometric visibility; #4589 measurement observability; #4086 admission; #3331 component identity; #1226 blind receipts.

## Purpose

Connect building-specific capture evidence to the already-existing finite-state decision-impact engine. Do not fork the generic solver.

The adapter requires each proposed capture action to bind:
- building, scene, model and survey revisions;
- sensor profile identity and digest;
- pose revision, pose evidence and reachability qualification;
- coordinate frame and frame revision;
- evaluator visibility receipt and visibility disposition;
- measurement qualification for a named task profile;
- exact decision profile and requirement scope.

A capture action is eligible only when all required fields refer to the same building decision and current survey/frame revisions. Occluded, inaccessible, unknown, stale or task-unqualified capture actions cannot produce decision effects.

## Executable boundary

The Rust package in tools/scan-bim-decision-capture-adapter:

1. Validates the adapter envelope against a closed Draft 2020-12 schema.
2. Enforces exactly one evidence binding per delegated decision action.
3. Checks sensor, pose, frame, revision, visibility, measurement and task-profile bindings.
4. Reconstructs the existing #1892 decision-impact corpus envelope and calls validate_decision_impact_bytes from spatial-resilience-qualifier.
5. Does not implement candidate feasibility, Pareto, action ranking or adaptive policy logic itself.
6. Fails closed if the delegated engine recommends a capture action whose capture-evidence gate is unqualified.

## Frozen corpus

The P0 corpus wraps the eight known-answer #1892 finite-state decision cases and provides eleven action-to-capture bindings.

The synthetic baseline expects ten capture bindings to be internally qualified and one stale-source action to be unqualified. The existing #1892 oracle remains authoritative for:
- mutually exclusive and exhaustive outcome partitions;
- action revision/evidence state;
- critical unknown resolution;
- robust feasibility and Pareto changes;
- action recommendation order.

Adapter negative controls cover external occlusion, stale frame revision, unreachable/unqualified pose, missing action binding and unknown fields.

## Benchmarks and acceptance targets

P0 acceptance:
- 100% agreement with the existing #1892 eight-case decision oracle.
- Exactly one capture binding for each generic action (11/11 in the fixture).
- Zero recommended actions with unqualified source/frame/pose/visibility/measurement/task evidence.
- 100% rejection of the frozen capture-evidence negative controls.
- Deterministic receipts bound to input, schema, source and generated Cargo lock.
- No probabilities, expected savings or real cost claims.

P1 matched building experiments compare capture-action eligibility and resulting decision correctness against fixed-grid, coverage-greedy, uncertainty-led (Active3D), Bayesian task-specific NBV (Zhu et al.) and experienced surveyor/operator workflows. These are different baseline families: probabilistic utility and deterministic worst-case decision impact must not be collapsed into one score.

Report per building and per action: remaining hypotheses, robust-feasible/Pareto set changes, critical unknowns resolved, invalid/unreachable action rate, visibility/measurement failure reasons, capture/setup/travel/processing/review effort, and final independently accepted decision. Use building-level holdouts, not random point-level splits.

## Research and industry context

- Active3D (AAAI 2026) selects views through hierarchical uncertainty and Expected Hybrid Information Gain. [AAAI paper](https://doi.org/10.1609/aaai.v40i8.37585).
- Zhu, Sellán and Terenin (SIGGRAPH 2026) use Bayesian decision theory for task-specific NBV over uncertain geometry. Their probabilistic approach requires defensible priors/posteriors and is a comparator, not the same method as finite-state worst-case reasoning. [Paper](https://arxiv.org/abs/2605.05095).
- Leica Cyclone 3DR 2026 already supports per-sub-element BIM-vs-scan tolerance analysis, coverage thresholds, operator review, status resets after parameter edits, and PDF/BCF output. Basic tolerance coloring is not a novelty claim. [Official workflow](https://rcdocs.leica-geosystems.com/cyclone-3dr/2026.0/Ex_ToleranceChecking).
- IFC 4.3.2.0 remains the official published IFC version; IFC 4.4 and IFC 5 are development work. [buildingSMART version table](https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/).
- IDS 1.0 is the approved machine-readable information-requirements standard. It does not replace geometry, topology, metrology or physical state qualification. [buildingSMART IDS](https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/).

## Claim ceiling

The corpus's building IDs, geometry states, sensor profiles, poses, receipts and costs are synthetic. Passing P0 only qualifies the adapter's deterministic evidence-binding and delegation behaviour. It does not authenticate referenced artifacts, prove physical reachability, validate a sensor or metrology model, establish field cost savings, qualify a building for construction or establish SOTA superiority.

P1 requires independent TLS/survey control, exact sensor and pose evidence, physical access validation, held-out buildings, qualified task profiles and professional review.
