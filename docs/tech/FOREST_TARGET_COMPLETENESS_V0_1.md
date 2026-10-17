# Forest Target-Completeness Benchmark v0.1

**Status:** standalone, deterministic synthetic audit. Rust execution and exact-head CI are required before calling the implementation validated. This is not calibrated tree geometry, a DBH estimator, field inventory evidence, or a hardware safety case.

## The paradigm shift

A planner can expose many map cells while failing to collect the particular evidence a forestry task needs. This benchmark shifts the objective from *where have we looked?* toward *which required target features have we observed well enough to answer a declared question?*

It introduces five separable questions:

1. **Coverage:** how many unique grid cells were observed?
2. **Target evidence:** how many required feature samples were visible?
3. **Task completeness:** were *all* required samples for each target observed?
4. **Feasibility:** did the original planner preserve its route, return and reserve contract?
5. **Evidence integrity:** can the score be reproduced from the exact planner receipt, corpus bytes, map digest, configuration digest, code revision and target seed?

A high feature average must not hide an incomplete target. Receipts therefore include per-target results, total feature visibility, complete-target rate, held-out aggregate and worst-case feature visibility.

## What v0.1 actually tests

The audit independently recomputes line-of-sight, field of view and range against four existing 5×5 occupancy templates: open grid, occluded clearing, blocked corridor and unknown barrier. It re-scores the viewpoint sequences produced by three current methods:

- Greedy marginal cell gain.
- Two-step receding-horizon lookahead.
- Exhaustive oracle for the **cell-coverage objective**.

The cell-coverage oracle is not a target-completeness oracle; its target score remains a comparator. The target-objective oracle described below enumerates a separate feasibility-bounded objective before reporting regret against it.

The seeded target generator creates localized two-sample trunk-edge tasks and four-sample log-end-contour tasks. A feature receives credit only if at least one selected viewpoint observes its cell within range and the 180° FOV, with conservative obstacle/unknown-cell occlusion. Standing in the same cell as a required feature does not count as observing it. A target is complete only when every required feature sample is visible across the selected viewpoints.

The corpus freezes four development seeds and six held-out seeds, disjoint by construction. Each of the six planner scenarios is crossed with all ten seeds, for 24 development evaluations and 36 held-out evaluations (60 total, three targets per case). Results are diagnostic: these seed splits vary target layouts over frozen occupancy templates. **They do not yet provide unseen map topology or a realistic 3D forest generator.** That limitation is explicit so held-out target layouts are not misrepresented as a generalization test across arbitrary forests.

## Independent checks and receipts

The audit is a separate Rust crate and reimplements visibility instead of importing the planner’s scoring function. It fails closed when:

- training and held-out seeds overlap, duplicate, or contain zero;
- the planner receipt's source SHA does not match the exact checked-out head;
- scenario IDs differ from the frozen bindings;
- a source plan is budget-infeasible;
- the planner's map/config digests do not match the independently reconstructed scenario binding;
- a feature is outside the clear-cell domain or a score is inconsistent with its required-feature count.

Both the planner receipt and target corpus are byte-hashed in the final receipt. The hosted workflow tests both Rust crates, asserts the expected evaluation/seed counts and score bounds, and uploads the two exact-input receipts. A queued workflow is not a pass.

## Target-objective oracle extension

The target audit now contains a second, independently specified exhaustive objective. It enumerates every feasible sequence in its bounded candidate space (at most three viewpoints and no more than 64 required feature samples) and scores the union of visible required features.

Its lexicographic objective is versioned in the emitted receipt:

1. maximize the number of complete targets;
2. maximize the number of required feature samples observed;
3. minimize movement + heading-change + shortest return + reserve cost;
4. minimize the number of viewpoints;
5. break remaining ties by deterministic candidate order.

Only viewpoints that reveal at least one required feature are enumerated. A viewpoint that adds no new required feature cannot improve either primary objective and is not needed to optimize the final cost/fewer-viewpoints tie-break under this simplified metric model. All candidates are constrained to the traversable component reachable from the start. The result is still exact only for this declared discrete candidate set and simple step/turn costs.

The receipt now includes the target-oracle viewpoint sequence and its independently checked cost fields, per-target truth/visibility, per-case complete-target regret for greedy, lookahead, and the cell-coverage oracle, aggregate ratios where the denominator is nonzero, and worst-case per-case regret for each split. The ratio is serialized as null when the target oracle completes zero targets. The cell-coverage oracle stays separate and can score fewer useful forestry targets despite seeing more total cells.

This makes target-completeness regret measurable rather than inferred from raw cell coverage. It does not establish a production-grade target-aware planner: the current operational baselines remain unchanged, and the exact oracle is a tiny-instance reference only.
## Research basis

- [OA-NBV: Occlusion-Aware Next-Best-View Planning for Human-Centered Active Perception](https://arxiv.org/abs/2603.11072) studies target-centric visibility and completeness with feasible viewpoints. Its reported success and observation improvements are for its own human-centered experiments, not forestry.
- [Receding-Horizon Next-Best-View Planning for Highly Occluded Target Perception](https://arxiv.org/abs/2610.19082), posted 13 October 2026, reports benefits over tested single-step baselines under heavy occlusion, including on a physical robot. This motivates testing short-horizon plans; it does not transfer a performance claim to Symtropy.
- [Towards autonomous photogrammetric forest inventory using a lightweight under-canopy robotic drone](https://www.tandfonline.com/doi/full/10.1080/01431161.2025.2579803) reports DBH RMSE of 3.33–3.97 cm across its sample and 1.16–2.56 cm for trees below 30 cm DBH. Those are that study's results and sensor/setup; RMSE is not interchangeable with the median or percentile targets proposed below.
- [Building Forest Inventories With Autonomous Legged Robots—System, Lessons, and Challenges Ahead](https://ieeexplore.ieee.org/document/11053872) reports a 1-hectare survey in under 30 minutes and typical DBH accuracy around 2 cm in its own field system.

## Proposed benchmark ladder

These are targets to work toward, not achieved results or forestry standards.

| Stage | Gate | Evidence needed |
|---|---|---|
| Contract v0.1 | 100% plan feasibility; zero train/test seed overlap; zero impossible target-completeness credit; exact deterministic replay | Focused Rust tests and exact-head workflow receipt |
| Target-objective oracle | 100% feasible oracle plans; no feasible greedy/lookahead plan may beat the oracle's lexicographic score; publish held-out and worst-case regret | Exhaustive tiny-scene target-feature search, independent route-cost verification and receipt assertions |
| Target-aware planning | Build a practical target-aware receding-horizon selector; aim for ≥90% of the exact tiny-scene target-oracle complete-target score on held-out layouts and publish worst-case regret | Same feature definition, sensor budget, return reserve and seed split for every planner |
| Generalization | At least 1,000 procedurally generated scenes, distinct tuning/acceptance seeds, multiple occluder arrangements, narrow passages, unknown space and distractor objects | Held-out target and map-generation manifests, hashes, per-scenario results and mutation/negative controls |
| Field calibration | Report DBH bias, MAE, RMSE, p50/p90/p95 error, detection precision/recall and incomplete-target rate, stratified by diameter, species, occlusion, sensor and terrain | Independent annotated plots/point clouds, calibration records, uncertainty intervals and a preregistered protocol |
| Aspirational field target | For a defined platform and calibrated operating envelope, explore median absolute DBH error ≤2 cm for trees under 30 cm DBH and p90 absolute error ≤4 cm; separately report larger-tree strata | Repeated paired field plots and confidence intervals; do not substitute synthetic scores or one paper’s result |

## Next research and implementation slice

1. Replace the four-template occupancy model with a versioned procedural map-topology generator; keep that map seed split independent from target-layout seeds.
2. Use the new target-objective exhaustive oracle as the reference; implement a target-aware receding-horizon candidate policy that minimizes held-out regret while preserving existing greedy/lookahead baselines unchanged.
3. Add observation uncertainty, pose error and sensor-specific false positives/negatives only when those parameters can be declared and, later, calibrated.
4. Add 3D/cylindrical trunk and log-end geometry and compare predicted diameter/contour evidence to independent ground truth.
5. Only after those gates, consider target-specific active sensing in Symthaea; keep mission feasibility and machine safety hard constraints, never weighted away by information gain.

No synthetic target score should be promoted into a timber grade, field inventory, certification, work order, or permission to actuate machinery.
