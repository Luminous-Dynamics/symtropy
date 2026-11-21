# Robotics Benchmark Robustness Reference v0.1

**Tracking issue:** [BENCH-ROBUSTNESS-001 #6509](https://github.com/Luminous-Dynamics/symtropy/issues/6509)  
**Related full benchmark contract:** [LADDER M0 #6478](https://github.com/Luminous-Dynamics/symtropy/pull/6478)

**Status:** M1 statistical scorecard reference. It computes metrics from already-paired outcomes; it does not run a robot or create causal validity by itself.

## Why this exists

A top-line manipulation success score is not enough to establish robust capability. Shortcuts, data leakage, overfitting and weak statistical power can make improvements look stronger than they are. Different benchmark scores should not be averaged or ranked together unless their task definition, episode denominator, embodiment, training/tuning and evaluation protocol are matched.

## Matched input contract

Each episode record is represented as a baseline/candidate pair with the **same** episode ID, inside a task cluster. Each task cluster declares:
- stable task ID;
- task category;
- semantic task horizon;
- explicit shift family such as `in_distribution`, `held_out_object`, `held_out_layout`, `lighting`, `material`, `camera` or `compositional`;
- all matched episode pairs for that task.

The benchmark identity binds the scorecard to a benchmark ID, exact 40-character benchmark commit, task/scene protocol digest, embodiment, evaluation split and tuning-on-benchmark declaration.

## Metrics

- Baseline and candidate pooled success with Wilson 95% intervals.
- Pooled absolute candidate-minus-baseline success delta.
- Paired task-cluster bootstrap 95% interval. Resampling is done over whole task clusters, keeping all paired episodes within a sampled task together.
- Equal-weight task macro success delta.
- Candidate/baseline rates by semantic horizon and shift family.
- `positive_improvement_supported` is true only when the lower task-cluster bootstrap bound is above zero.

The interval is a percentile cluster bootstrap, not an exact randomization test. Use a predeclared replicate count and seed. For formal claims, publish per-task counts and a preregistered power/significance plan as well.

## Required external diagnostics (the Rust scorecard cannot perform these)

Run matched ablations and held-out interventions:
- instruction removed / semantically neutralized;
- image masked or matched neutral substitute;
- image swapped from another matched scene/task family;
- instruction swapped on a matched scene;
- held-out object identities and categories;
- held-out layouts and object-goal compositions;
- lighting, material and camera perturbations;
- recovery from missed grasp, slip/drop, partial placement and ambiguous execution outcome;
- no-language/no-image heuristic probe to detect shortcut solvability.

These are independent diagnostic columns. Never subtract them from the official benchmark score or use them to redefine official success.

## Statistical rules

- Pair baseline and candidate on identical episode IDs / seeds.
- Bootstrap whole task clusters, not frames or low-level control steps.
- Keep task clusters as the unit when scenes/task families are dependent.
- Never discard failed episodes or select the best rerun.
- Report horizon, category, embodiment and shift family separately.
- Do not mix full and lite subsets, privileged and non-privileged policies, simulators and hardware, or benchmark suites with different definitions.
- A SOTA claim requires matched pinned protocol and independent reproducibility; no number here is a SOTA claim.

## Reproduction

```sh
cargo fmt --manifest-path benchmarks/robotics-benchmark-robustness-reference/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/robotics-benchmark-robustness-reference/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/robotics-benchmark-robustness-reference/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

## Claim boundary

The scorecard validates internal arithmetic and input completeness only. It cannot establish that the chosen scene split is genuinely independent, that a negative control is causally valid, or that a policy generalizes beyond the evaluated distribution. Those require protocol review, independent assets and actual evaluation runs.
