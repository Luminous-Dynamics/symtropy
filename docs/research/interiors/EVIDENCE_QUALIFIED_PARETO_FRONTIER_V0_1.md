# Evidence-qualified Pareto frontier v0.1

**Status:** first Rust utility slice; synthetic/API-level logic only. Tracked by [INTERIOR-FRONTIER-001](https://github.com/Luminous-Dynamics/symtropy/issues/2508).

## Paradigm shift

A layout generator should not choose one design by adding price, style, movement, daylight, and task coverage into an unexplained weighted score. Such weights can let an attractive objective offset a failed mandatory requirement and can encode preferences the resident never supplied.

Instead, use a two-stage decision contract:

1. **Feasibility first:** only candidates marked Verified by the independent hard-constraint verifier are eligible. Blocked and Indeterminate candidates are excluded regardless of their objective values.
2. **Pareto comparison second:** among eligible candidates with comparable, policy-admitted objective observations, retain every non-dominated alternative. Do not silently scalarize different goals into one score.

This utility is downstream of verification. It does not implement or replace the independent geometry / functional-envelope oracle.

## Contract

- Every verified candidate must carry SHA-256-shaped identifiers for the exact design revision, the independent verifier receipt, and the revision digest that the receipt claims to cover. The revision digest on the candidate must equal the receipt-bound revision digest, or the candidate is excluded and the report is incomplete. This prevents accidental reuse of a receipt after editing a layout, but the module only checks caller-provided binding values: it **does not authenticate or recompute the receipt or either digest**.
- Each objective names a stable dimension, explicit unit or normalized scale, minimize/maximize direction, finite numeric value, and evidence-admission state. Dimension, unit, and direction must match exactly across the ranking set; this utility performs no currency conversion or hidden normalization.
- Only objective observations explicitly marked AcceptedByPolicy are rankable. Provisional, Unknown, Conflicting, and Blocked observations exclude that candidate from ranking and cause the report to state that the frontier is incomplete.
- Every rankable candidate must use the same objective dimensions, units/scales, and directions. A mismatch fails closed; the program does not guess conversions or pretend the alternatives are comparable.
- Comparisons use the supplied finite floating-point values directly. There are no hidden epsilon tolerances, normalization rules, utility weights, or tie-breaks that erase a non-dominated alternative. Exactly two objectives use an O(n log n) sweep; exactly three objectives use an O(n log n) sweep with a y-prefix Fenwick tree and explicit equal-coordinate handling. Four or more objective dimensions retain the conservative O(n²) pairwise path.
- Reports are sorted deterministically by candidate ID; objective order and request order do not affect the frontier. Each dominated candidate stores one deterministic dominance witness rather than every dominator, keeping report memory linear in candidate count.
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
- malformed digest shapes, stale receipt-to-revision bindings, and non-finite values;
- duplicate/invalid candidate IDs; and
- empty input / no verified candidate cases.

Proposed acceptance targets (not measured performance claims):
- zero blocked or indeterminate candidates on the frontier;
- zero non-admitted objective observations silently ranked;
- 100% determinism for identical inputs regardless of input ordering;
- fail-closed behavior for incomparable objective schemas;
- exact-head formatting, unit tests, and strict Clippy passing.

The frontier utility has a dependency-free scaling probe at 25, 100, 1,000, and 10,000 alternatives. Candidate construction is outside the timed section; each profile asserts expected frontier cardinality and reports p50/p95/max duration. The two- and three-objective sweeps are O(n log n); the general four-or-more-dimensional fallback is O(n²). The harness does not measure peak memory. The five-sample 10,000-candidate profile is exploratory, not a stable p95 service-level target. Existing collision-oracle timings are not measurements of this module.

### Exact-head performance evidence

Two runs used the same benchmark harness, Ubuntu 24.04 and Rust 1.96.0, but separate hosted-runner executions; treat the ratio as an observed result, not a hardware-controlled guarantee.

| Alternatives | Profile | Samples | Pairwise p50 | Sweep p50 | Sweep p95 |
|---:|---|---:|---:|---:|---:|
| 25 | Tradeoff-heavy; all non-dominated | 50 | 0.017 ms | 0.014 ms | 0.031 ms |
| 25 | Dominated chain; frontier size 1 | 50 | 0.014 ms | 0.016 ms | 0.029 ms |
| 100 | Tradeoff-heavy | 30 | 0.106 ms | 0.054 ms | 0.071 ms |
| 100 | Dominated chain | 30 | 0.058 ms | 0.073 ms | 0.092 ms |
| 1,000 | Tradeoff-heavy | 10 | 6.025 ms | 0.582 ms | 0.708 ms |
| 1,000 | Dominated chain | 10 | 0.717 ms | 0.804 ms | 0.904 ms |
| 10,000 | Tradeoff-heavy | 5 | 765.216 ms | 6.542 ms | 8.010 ms |
| 10,000 | Dominated chain | 5 | 8.761 ms | 9.637 ms | 10.267 ms |

Source records: pairwise baseline exact head `f1bf1b56df568854714dddab834ba3e505464c0b`, run [#39587364782](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39587364782); sweep exact head `f6a26541b8ab55b543fb17713d58b170b8313c57`, run [#39588539113](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39588539113). At 10,000 tradeoff-heavy alternatives, the observed p50 fell from 765.216 ms to 6.542 ms (about 117× in these runs). The dominated-chain p50 increased from 8.761 ms to 9.637 ms (about 10%), exposing the small cost of the sorting sweep on an easy-to-dominate distribution. The 10,000-case p95 is based on only five samples and is exploratory; peak memory remains unmeasured.

A second exact-head run after the documentation/CI-wiring change (`046fca1de725f29ea20aed52bbdcfe313fffe9bc`, [run #39588862002](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39588862002)) re-ran the same harness on a separate hosted-runner execution. It measured tradeoff-heavy p50 values of 0.016 / 0.060 / 0.740 / 8.080 ms and dominated-chain p50 values of 0.019 / 0.084 / 1.018 / 11.840 ms at 25 / 100 / 1,000 / 10,000 alternatives. Latest-run p95 values were respectively 0.021 / 0.091 / 0.869 / 9.163 ms and 0.023 / 0.110 / 1.120 / 12.461 ms. This confirms the same scaling shape but also shows run-to-run variation; the exact-host baseline has not been controlled. The observed 10,000-tradeoff p50 reduction versus the earlier pairwise baseline is about 95× on this retest and about 117× on the first optimized run, not a guaranteed speedup factor.

This is an internal microbenchmark, not an external SOTA result. End-to-end evaluation must compare identical verified candidate sets against transparent rules, human-curated trade-offs, and commercial/research workflows using identical inputs, objective definitions, and an independent evaluator.

## Three-objective follow-up

A separate stacked draft [PR #2518](https://github.com/Luminous-Dynamics/symtropy/pull/2518) adds an exact three-objective sweep using a compressed y-coordinate index and Fenwick prefix-minimum witnesses. It defers updates across equal-x groups and resolves equal-y candidates separately so identical points do not dominate one another. The new generated-grid and adversarial tests compare its output against the brute-force dominance oracle. This remains unproven until exact-head CI completes; four or more dimensions still use the pairwise fallback. The benchmark matrix now includes 2D and 3D profiles at 25/100/1,000/10,000 candidates, plus 4D fallback cases through 1,000 candidates.

## Claim ceiling

A frontier result is only a deterministic comparison of caller-supplied objective observations among candidates labelled verified by a caller. It does not establish source accuracy, authenticity of the verifier receipt, global optimality, real-room fit, accessibility, building-code compliance, environmental benefit, user preference, or resident outcomes. Those require independent evidence and specific evaluation protocols.
