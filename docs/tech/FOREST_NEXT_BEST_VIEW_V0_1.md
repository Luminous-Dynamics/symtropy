# Forest Next-Best-View and Exhaustive Tiny-Grid Oracle v0.1

**Status:** deterministic synthetic benchmark. Not field performance, a calibrated sensor model, a global approximation guarantee, or a physical safety case.

## Paradigm shift

The planner's question changes from "Which path is shortest?" to "Which feasible sensing viewpoints yield the most new, unobstructed evidence while preserving a return reserve?" This is an active-perception objective: a viewpoint has value because of the observations it can add, not only the distance it advances the robot.

Recent research gives clear reasons to test occlusion-aware, target-completeness-aware viewpoints and to evaluate information gain jointly with reachability and mobility budgets:

- OA-NBV integrates target-centric visibility with feasible robot poses; its reported human-centered results are not forestry results: https://arxiv.org/abs/2603.11072.
- A 2026 preprint explores distributed semantic multi-robot exploration subject to mobility budgets and information gain: https://arxiv.org/abs/2610.17562.
- Receding-horizon next-best-view work reports benefits from short viewpoint sequences under heavy occlusion versus myopic choices in its tested target-reconstruction settings: https://arxiv.org/abs/2610.19082.

These works motivate our benchmark design, not claims that this small contract duplicates their methods or results.

## v0.1 model

- Four 5×5 scenes: open grid, an occluded clearing with a top opening, a disconnected corridor, and a band of unknown cells with an opening.
- Candidate view is a traversable cell plus one of four cardinal sensor headings.
- Visibility: range 3 cells, fixed 180° horizontal field of view, ideal deterministic grid ray casting, conservative corner checks. Obstacles and unknown cells can be directly observed as endpoints but block cells behind them.
- Energy budget uses integer steps. One step is charged per grid movement and one step for a heading change. The shortest return distance plus a declared reserve must fit the budget.
- This v0.1 model lets the body rotate in place at a one-step cost; it omits real turn radius, yaw uncertainty, acceleration, slope-specific energy, slip, weather and sensor noise.
- A plan chooses up to the scenario's viewpoint limit. Coverage is the union of observed cells from chosen viewpoints only; transit-path sensing is not credited.
- The exhaustive oracle enumerates unique view candidates with the same costs and constraints. It maximizes unique visible-cell count, then prefers lower total budgeted steps and fewer viewpoints. It is exact only for this finite model and capped viewpoint count.

## Frozen synthetic results

| Scene | Greedy visible cells | Exhaustive oracle | Greedy/oracle |
|---|---:|---:|---:|
| Open grid, 3 views, budget 10 | 19/25 | 23/25 | 82.60% |
| Occluded clearing, 3 views, budget 12 | 19/25 | 19/25 | 100% |
| Disconnected corridor, 3 views, budget 10 | 13/25 | 13/25 | 100% |
| Unknown barrier, 3 views, budget 10 | 17/25 | 17/25 | 100% |
| Tight return reserve, 3-view maximum, budget 4 | 11/25 | 11/25 | 100% |
| Open grid, 1 view, budget 7 | 11/25 | 16/25 | 68.75% |

The values are frozen synthetic expected outcomes from a separate reference implementation and are then checked exactly by the Rust contract. Rust CI has not been claimed as passing until checked for the exact head.

**The greedy baseline misses its aspirational 90% target in two scenes.** That is intentional evidence, not a pass hidden by averaged scores. It shows why the oracle matters. The next improvement should target lookahead and better candidate utility, then re-run exactly the same frozen suite before adding held-out maps. Six toy scenes are not enough to generalize any approximation claim.

## Proposed benchmark gates

| Metric | Proposed gate | How to validate |
|---|---|---|
| Budget safety | 100% of output plans fit outbound + return + reserve within budget | Adversarial tiny maps and independently recomputed costs |
| Oracle consistency | 100% exact outputs against frozen exhaustive results | Fixed corpus bytes, source SHA and map/config hashes |
| Greedy efficiency | First prevent regressions; then target ≥90% of oracle on the frozen suite | Report per-scene ratios and worst-case regret, not just mean |
| Occlusion false-credit | 0 | Obstacles and unknown cells occlude all cells behind them in independent ray-cast controls |
| Repeatability | 100% deterministic semantic receipts | Repeat each map/config/planner version under the same source revision |
| Planner runtime | Report p50/p95 and candidate expansions | Tiny oracle first; do not compare oracle runtime against production-scale planners as if equivalent |
| Information efficiency | Newly visible cells per energy step and per meter | Same maps, start, candidate budget and return policy across baselines |
| Forest object completeness | Detection precision/recall and DBH bias/median/p90/p95 error | Independent annotated plots; stratify by species, diameter, occlusion, season, terrain and sensor |

Targets above are engineering goals, not forestry standards or demonstrated capabilities.

## Next work

1. Improve the greedy baseline with a one- or two-step lookahead; retain the exhaustive solver as the regression oracle for small scenes.
2. Expand the corpus using held-out generated maps with an independent seed split, plus symmetric scenes and adversarial obstacle arrangements.
3. Model target-centric observation quality: how much of a tree trunk, diameter cross-section, log end or defect is actually visible, not only how many cells are exposed.
4. Add a conservative pose-uncertainty envelope and calibrated modality-specific false-positive/false-negative rates from real measurements.
5. Add a separate multi-robot allocation/reconciliation benchmark, charging communication and mobility costs while retaining conflicting detections.
6. Only then evaluate against independent point clouds and field plots. Synthetic scenes must never be counted as field inventory.

Safety remains independent of information gain: no planner score can override a blocked mission, a missing permission record, failed physical safety controls, or a machine-specific risk assessment.
