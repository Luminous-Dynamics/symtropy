# Adaptive Evidence Acquisition v0.1

Parent: Symtropy #5857  
Portfolio: #5700 robotics evidence qualification  
Status: standalone, dependency-free Rust reference planner; synthetic-only.

## Paradigm

The generic evidence-request decision is owned upstream by Symthaea #12046. Its request planner determines which evidence would resolve a decision-relevant uncertainty and supplies the requirement and priority.

This crate starts **after** that decision: given an approved finite set of robotics evidence requirements and an upstream-qualified set of candidate viewpoints/actions, which physically feasible sequence best satisfies those requirements within the declared time and energy budgets?

It is a robot-domain route/action ordering adapter. It does not decide which institutional claim needs evidence, generate new evidence requests, collect data, authorize disclosure, accept mission stages, or independently qualify a measurement. It composes with the PV, AUV, manipulator and surveying evaluators without granting any of them additional authority.

## Model

- **Evidence targets** declare the number of required independent passes, minimum view-angle separation, and an upstream-supplied priority weight. The weight is consumed as provided; this crate does not estimate decision value or change the request priority.
- **Existing passes** represent evidence already independently qualified by an upstream evaluator.
- **Candidates** represent possible next acquisitions. A candidate's covered-target list must come from upstream geometry evaluation; its quality must come from upstream sensor/evidence preflight.
- **Candidate qualification is fail-closed**: only `Qualified` candidates may be selected. `NotQualified`, `Unknown`, and `InvalidEvidence` are counted and excluded.
- **Independence** requires a new pass identity and the minimum angular separation from every already-qualified pass for that target.
- **Greedy objective** orders the approved target set by normalized marginal completion gain per time-equivalent cost. Each qualifying pass contributes upstream-supplied target weight divided by that target's required pass count, so targets requiring more independent views are not overweighted merely because they need more passes. The cost includes estimated travel time, acquisition time, explicit risk penalty, and an explicit Wh-to-seconds utility weight.
- **Budgets** constrain actual estimated elapsed time and energy independently. The planner stops with Partial/Blocked rather than relaxing evidence requirements.

The candidate list is not a motion plan. Travel time is Euclidean distance divided by a supplied nominal speed; no obstacles, dynamics, acceleration, wind, collision avoidance, geofencing, or flight authorization are modeled. This package does not replace Symthaea #12046 request selection or Symtropy #5657 mission-stage evidence transitions.

## Benchmark design

The example exposes five selection policies over the exact same candidate pool, evidence requirements, and budgets:

1. **Evidence gain per cost** — weighted marginal evidence divided by travel/acquisition time, risk penalty, and energy penalty.
2. **Nearest feasible** — shortest travel distance among candidates that pass the same quality, independence and budget gates.
3. **Maximum marginal evidence** — largest weighted evidence gain, without normalizing by cost.
4. **Input order / fixed route** — stable first-feasible ordering; supply raster/waypoint order to reproduce a fixed-route baseline.
5. **Seeded hash order** — deterministic pseudo-random candidate ordering keyed by a published seed.

Every policy uses identical upstream quality, independence and resource-budget gates. The benchmark must never disable safety/evidence gates to make a baseline weaker; this isolates selection quality, not permission to accept bad evidence.

Report:
- weighted evidence deficit closed per elapsed minute and per Wh;
- proportion of targets complete under the declared independent-pass/view-angle rules;
- selected NotQualified / Unknown / InvalidEvidence candidates (must be 0);
- duplicate pass credits (must be 0);
- minimum-angle violations (must be 0);
- elapsed-time and energy budget violations (must be 0);
- planning latency and deterministic replay agreement;
- mean, standard deviation, median and worst-case performance across held-out layouts and deterministic seeds.

Keep target distributions, candidate-generation method, evaluation budget, geometry/sensor preflight and compute budget identical across baselines. Publish per-episode records, not only aggregate scores. Include adversarial cases that make a naive maximum-coverage planner look good while selecting stale, repeated, low-quality or uneconomic observations.

## Claim boundary

This package is not a sensor model, image processor, coverage geometry engine, photogrammetry implementation, trajectory generator, obstacle planner, robot controller or safety authority. It does not diagnose defects, qualify construction, prove inspection conformity, or establish physical robot performance. A candidate's coverage and quality are accepted only as inputs from a separate evaluator. The output is a proposed acquisition ordering, not permission to execute it.

## Run

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo run
```

No third-party dependencies are required.

The current example/benchmark uses five policies over identical candidate pools, target requirements and budgets. Each episode contains 12 targets and 72 candidate viewpoints.

Candidate coverage is now generated from spatial coordinates and an oriented rectangular footprint, rather than assigned by a target-ID formula. The six acquisition lanes have distinct pass identities and view angles; multiple frames from the same lane cannot count as independent passes for one target. Four synthetic layout families are defined: regular rectangular, wider, compact and staggered-row layouts.

The runner executes **48 development episodes** across the first three layout families and **16 held-out episodes** from the staggered-row family, with deterministic seed ranges. It reports mean, lower-decile and worst-case weighted completion, weighted evidence units per minute and Wh, elapsed time, energy, selected steps and independently re-audited gate-violation counts. Candidate order is serpentine for the fixed-route baseline; every policy receives identical candidate records, quality dispositions and budgets.

The candidate footprint is an analytic synthetic geometry oracle, not image formation, photogrammetry, an occlusion model, a sensor model or a physical camera. Sensor/qualification states are deterministic benchmark dispositions, not an evaluated perception model. Route travel is still straight-line at a nominal speed and omits obstacles, turn radius, acceleration, wind, communications and real battery dynamics. The held-out split tests generalization to one withheld synthetic layout family; it is not external, physical or industry holdout evidence.

The current unit suite has 13 tests, including policy-comparison, normalized multi-pass weighting and cross-policy gate-preservation tests. The benchmark is a reproducible engineering harness, not a public leaderboard submission or evidence that any policy is superior. Report paired per-seed deltas, uncertainty intervals and the full per-episode output before making a policy claim.
