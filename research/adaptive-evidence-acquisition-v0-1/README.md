# Adaptive Evidence Acquisition v0.1

Parent: Symtropy #5857  
Portfolio: #5700 robotics evidence qualification  
Status: standalone, dependency-free Rust reference planner; synthetic-only.

## Paradigm

A conventional coverage planner asks: "Which route covers the area?"

An evidence-first planner asks: "Given what is already independently qualified, which next observation closes the most remaining evidence deficit within the available time and energy budget?"

This crate implements a small, deterministic planning primitive for that second question. It can be composed with the PV, AUV, manipulation, or surveying evaluators without granting any of them additional authority.

## Model

- **Evidence targets** declare the number of required independent passes, minimum view-angle separation, and relative importance.
- **Existing passes** represent evidence already independently qualified by an upstream evaluator.
- **Candidates** represent possible next acquisitions. A candidate's covered-target list must come from upstream geometry evaluation; its quality must come from upstream sensor/evidence preflight.
- **Candidate qualification is fail-closed**: only `Qualified` candidates may be selected. `NotQualified`, `Unknown`, and `InvalidEvidence` are counted and excluded.
- **Independence** requires a new pass identity and the minimum angular separation from every already-qualified pass for that target.
- **Greedy objective** selects the candidate with greatest weighted marginal evidence gain per time-equivalent cost. The cost includes estimated travel time, acquisition time, explicit risk penalty, and an explicit Wh-to-seconds utility weight.
- **Budgets** constrain actual estimated elapsed time and energy independently. The planner stops with Partial/Blocked rather than relaxing evidence requirements.

The candidate list is not a motion plan. Travel time is Euclidean distance divided by a supplied nominal speed; no obstacles, dynamics, acceleration, wind, collision avoidance, geofencing, or flight authorization are modeled.

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

The synthetic example runs all five policies on the same inputs and prints selected candidates, target completion, elapsed time and energy. The current unit suite includes policy-comparison and cross-policy gate-preservation tests. These checks are not a public benchmark leaderboard result.
