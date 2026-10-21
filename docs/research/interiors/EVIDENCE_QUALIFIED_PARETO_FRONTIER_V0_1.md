# Evidence-qualified Pareto frontier v0.1

**Status:** first Rust utility slice; synthetic/API-level logic only. Tracked by [INTERIOR-FRONTIER-001](https://github.com/Luminous-Dynamics/symtropy/issues/2508).

## Paradigm shift

A layout generator should not choose one design by adding price, style, movement, daylight, and task coverage into an unexplained weighted score. Such weights can let an attractive objective offset a failed mandatory requirement and can encode preferences the resident never supplied.

Instead, use a two-stage decision contract:

1. **Feasibility first:** only candidates marked Verified by the independent hard-constraint verifier are eligible. Blocked and Indeterminate candidates are excluded regardless of their objective values.
2. **Pareto comparison second:** among eligible candidates with comparable, policy-admitted objective observations, retain every non-dominated alternative. Do not silently scalarize different goals into one score.

This utility is downstream of verification. It does not implement or replace the independent geometry / functional-envelope oracle.

## Contract

- Every verified candidate must carry SHA-256-shaped identifiers for the exact design revision and the independent verifier receipt. This module validates their shape and passes the identifiers through; it **does not authenticate or recompute either digest**.
- Each objective names a stable dimension, explicit unit or normalized scale, minimize/maximize direction, finite numeric value, and evidence-admission state. Dimension, unit, and direction must match exactly across the ranking set; this utility performs no currency conversion or hidden normalization.
- Only objective observations explicitly marked AcceptedByPolicy are rankable. Provisional, Unknown, Conflicting, and Blocked observations exclude that candidate from ranking and cause the report to state that the frontier is incomplete.
- Every rankable candidate must use the same objective dimensions, units/scales, and directions. A mismatch fails closed; the program does not guess conversions or pretend the alternatives are comparable.
- Comparisons use the supplied finite floating-point values directly. There are no hidden epsilon tolerances, normalization rules, utility weights, or tie-breaks that erase a non-dominated alternative.
- Reports are sorted deterministically by candidate ID; objective order and request order do not affect the frontier.
- The resident retains final choice. Pareto membership is not a statement that an alternative is universally best.

## Relevant research and industry baseline

- [SceneEval (WACV 2026)](https://github.com/3dlg-hcvc/SceneEval) evaluates explicit requirements and implicit physical plausibility, including collision, navigability, and opening clearance.
- [Function2Scene (2026 preprint)](https://arxiv.org/abs/2605.30819) frames layout generation around functional specifications and iterative evaluation/refinement rather than object placement alone.
- Consumer tools such as [IKEA Kreativ](https://www.ikea.com/us/en/customer-service/knowledge/articles/43460e1e-dd01-4fe8-86ac-74efecd7743f.html) already provide room capture/entry and editable catalog-based furniture arrangements. This project does not claim novelty for visual placement or editable 3D rooms.

The differentiated hypothesis is a traceable boundary between hard feasibility, evidence admission, and resident-controlled multi-objective choice. It remains a hypothesis until verified against baselines and real resident workflows.

## Test and benchmark gates

The unit tests cover:
- non-dominated cost/coverage trade-offs without scalar weights;
- exclusion of blocked and indeterminate candidates;
- unknown objective evidence causing explicit incompleteness;
- invariance to candidate/objective input order;
- mismatched objective schemas;
- equal alternatives retained on the frontier;
- malformed digest shapes and non-finite values;
- duplicate/invalid candidate IDs; and
- empty input / no verified candidate cases.

Proposed acceptance targets (not measured performance claims):
- zero blocked or indeterminate candidates on the frontier;
- zero non-admitted objective observations silently ranked;
- 100% determinism for identical inputs regardless of input ordering;
- fail-closed behavior for incomparable objective schemas;
- exact-head formatting, unit tests, and strict Clippy passing.

The frontier utility needs its own eventual runtime / memory measurements at 25, 100, 1,000, and 10,000 alternatives. Existing collision-oracle timings are not measurements of this module. End-to-end evaluation must later compare the same verified candidate set against transparent rules, human-curated trade-offs, and commercial workflows, using identical objective definitions and input snapshots.

## Claim ceiling

A frontier result is only a deterministic comparison of caller-supplied objective observations among candidates labelled verified by a caller. It does not establish source accuracy, authenticity of the verifier receipt, global optimality, real-room fit, accessibility, building-code compliance, environmental benefit, user preference, or resident outcomes. Those require independent evidence and specific evaluation protocols.
