# Magnetic active metrology (v0.1)

A zero-dependency Rust reference for deterministic, decision-directed selection of magnetic field measurement locations.

## Research thesis

Uniform grids are useful baselines, but a measurement has the greatest engineering value where qualified models disagree and that disagreement is consequential to a named functional requirement.

For each candidate point, the planner consumes:
- a model-set disagreement bound in tesla;
- an absolute sensitivity of the named functional margin in radians per tesla;
- a simultaneous absolute measurement-error bound in tesla;
- acquisition cost in seconds;
- exact frame, operating-condition, probe-calibration, qualified-model-set and requirement identities.

The transparent screening metric is:

    resolvable_field_separation = max(0, model_disagreement - 2 * measurement_error)
    resolvable_decision_impact = resolvable_field_separation * abs(d_margin / dB)
    priority = resolvable_decision_impact / acquisition_cost

The factor of two reflects the two-sided absolute measurement-error bound: two model predictions separated by no more than twice the measurement bound are not guaranteed to be distinguishable by this observation. This is a deterministic screening heuristic, not expected information gain or a probability.

The full unadjusted decision-impact bound is also reported. Model disagreement, measurement uncertainty, sensitivity and acquisition cost remain individually visible; no opaque weighted composite is used.

## Evidence separation

Only `AdaptiveExploration` points may be selected. Model-calibration, blind-holdout and post-selection-qualification candidates remain protected and are not ranked. The planner cannot consume the blind validation map or move its points into the adaptive scan.

Each candidate is bound to exact frame, operating condition, probe calibration, qualified model-set and functional-requirement identities. Mismatches fail closed. Previously measured points are excluded unless an explicit policy permits revisit.

## Scan constraints

Selection is deterministic and greedy over the supplied candidate list:
- descending priority;
- stable candidate ID for exact score ties;
- input order as a final tie-break;
- minimum Euclidean 3D spacing;
- total acquisition-time budget;
- maximum number of points;
- minimum declared priority floor.

The planner reports selected order and explicit exclusion/not-rankable reasons for every candidate. It does not invent locations or modify measurement inputs.

## Benchmark programme

Compare against a uniform grid, seeded random sampling, maximum field-variance sampling, maximum solver-disagreement sampling, sensitivity-only sampling and a full-map oracle. Use identical probe calibration, coordinate registration, standoff, condition, cost model and blind holdout map.

Report observations and scan time to stabilize the engineering decision, blind-holdout functional error, false-accept/false-reject rate, missed risk regions, calibration overhead and planner determinism. Do not claim sample-efficiency improvement until these matched trials exist.

## Claim ceiling

This crate ranks only the supplied candidate list and supplied source-bound values. It does not verify the physical correctness of model disagreement or sensitivity, estimate a probability, establish field-map calibration, replace blind validation, infer manufacturing yield or qualify a component.
