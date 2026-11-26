# Magnetic active metrology (v0.1)

A zero-dependency Rust reference for deterministic, decision-directed selection of magnetic field measurement locations.

## Research thesis

Uniform grids are useful baselines, but an informative measurement should be located where qualified model predictions are provably separated and the downstream functional requirement is sensitive to the measured field.

For each candidate point, the planner consumes:
- a **lower bound** on the separation between at least two qualified model-prediction intervals, in tesla;
- a signed interval enclosing the derivative of the named functional margin over the complete model-disagreement segment, in radians per tesla;
- a simultaneous absolute measurement-error bound, in tesla;
- acquisition cost in seconds;
- exact frame, operating-condition, probe-calibration, model-set and requirement identities.

The signed sensitivity interval must lie wholly above or below zero. If its interval crosses zero, a positive decision-impact lower bound cannot be established and the point is not ranked.

The transparent screening metric is:

    resolvable_model_separation_lower_bound =
        max(0, model_disagreement_lower_bound - 2 * measurement_error_bound)

    minimum_absolute_sensitivity =
        min(abs(sensitivity_interval.lower), abs(sensitivity_interval.upper))

    decision_impact_lower_bound =
        resolvable_model_separation_lower_bound * minimum_absolute_sensitivity

    priority = decision_impact_lower_bound / acquisition_cost

The factor of two reflects the two-sided absolute measurement-error bound: model-prediction intervals separated by less than this cannot be guaranteed distinguishable by this measurement. The product is a conservative lower bound only when the supplied model-separation and signed sensitivity intervals are themselves valid over the declared point and condition.

All source record IDs and component values remain separately visible. A missing disagreement lower bound or sensitivity interval is not replaced with a guessed value.

## Evidence separation

Only `AdaptiveExploration` points may be selected. Model-calibration, blind-holdout and post-selection-qualification candidates remain protected and are not ranked. The planner cannot consume the blind validation map or move its points into the adaptive scan.

Each candidate is bound to exact frame, operating condition, probe calibration, qualified model-set and functional-requirement identities. Mismatches fail closed. Previously measured points are excluded unless the policy explicitly permits revisiting.

## Scan constraints

Selection is deterministic and greedy over the supplied candidate list:
- descending lower-bound decision impact per second;
- stable candidate ID for exact score ties;
- input order as a final tie-break;
- minimum Euclidean 3D spacing;
- total acquisition-time budget;
- maximum number of points;
- minimum declared priority floor.

The planner reports selected order and explicit exclusion/not-rankable reasons for every candidate. It does not invent locations or modify measurement inputs. A declared 3D scan-region envelope and standoff interval are enforced before candidate ranking.

The crate currently contains 16 authored unit tests covering impact-per-cost order, missing sensitivity/disagreement, sensitivity intervals crossing zero, positive and negative derivative intervals, measurement-error masking, protected evidence roles, 3D separation, budget, deterministic ties, provenance mismatch, duplicate IDs, invalid inputs and explicit revisit policy. Authored tests are not a test pass.

## Benchmark programme

Compare against a uniform grid, seeded random sampling, maximum field-variance sampling, maximum cross-solver disagreement sampling, sensitivity-only sampling, decision-directed sampling and a full-map oracle. Use identical probe calibration, coordinate registration, standoff, condition, cost model and blind holdout map.

Report observations and scan time to stabilize the engineering decision, blind-holdout functional error, false-accept/false-reject rate, missed risk regions, calibration overhead and planner determinism. Do not claim sample-efficiency improvement until these matched trials exist.

## Claim ceiling

This crate ranks only the supplied candidate list and supplied source-bound values. It does not verify the physical correctness of the model-separation or sensitivity records, estimate a probability, establish field-map calibration, replace blind validation, infer manufacturing yield or qualify a component.
