# Symtropy Experiment Selection Benchmark

This is the first executable synthetic benchmark rung for SCI-ACTIVE-001 (#2895).

## What it compares

All methods receive the same digitally shifted Sobol initial design and the same total number of objective evaluations:

- seeded uniform random search;
- digitally shifted 2D Sobol;
- 2D Halton low-discrepancy sequence;
- randomized Latin hypercube;
- Gaussian-process upper-confidence-bound selection (RBF kernel, fixed length scale/noise/UCB beta).

The GP-UCB model is intentionally small and transparent. It is a reference baseline, not a claim to match the strongest production Bayesian-optimization packages. The experiment-selection interface is a CandidatePolicy trait so a Symthaea adapter can be added without changing objective, oracle, accounting, or scoring code.

## Synthetic problem families

- smooth bowl: easy unimodal optimum;
- multimodal: global and local peaks;
- narrow peak: small attraction basin;
- constrained bowl: optimum lies on the feasible boundary x + y <= 1.15;
- non-identifiable ridge: all y values are equally optimal at x = 0.7.

The objective is maximized on [0,1]^2. Observation noise is deterministic for a (trial seed, point) pair and mildly heteroskedastic, so paired methods see the same noisy observation if they evaluate the same point. The regret oracle uses the analytic noiseless objective and is never passed to a CandidatePolicy.

## Run

From the repository root:

    cargo test -p symtropy-experiment-selection
    cargo run --release -p symtropy-experiment-selection --bin experiment-selection-bench -- --source-sha "$(git rev-parse HEAD)" --seed-count 20 --budget 32 --initial-design 8

The executable prints:
- an EXPERIMENT_CAMPAIGN header;
- one EXPERIMENT_RECEIPT row per problem/policy/seed trial;
- one EXPERIMENT_SUMMARY row per problem/policy.

The source SHA is supplied by the caller; use the exact committed source SHA. Runtime nanoseconds are observational and will differ across hosts/replays. Candidate coordinates and synthetic observations are deterministic for a fixed source, seed, policy and configuration.

## Interpretation

Each observation consumes one unit of the synthetic high-fidelity evaluation budget, including infeasible proposals. The initial design is included in that budget. In this first rung there are no external solver failures, mesh costs, GPU costs, or expensive model-training costs. GP-UCB computation is included in per-trial elapsed wall time but is not yet charged in a separate CPU-cycle budget.

Report mean, median and p90 final feasible regret; infeasible calls; and runtime. Use paired seeds and report distributions, not only best outcomes. Do not average across problem families as though an easy bowl and a narrow multimodal peak were interchangeable.

## Claim ceiling and follow-on

This is a deterministic synthetic optimizer benchmark, not evidence of Symthaea superiority or physical-model correctness. It does not yet implement expected information gain, posterior calibration coverage, OOD abstention, confirmation-set protection, or a Symthaea candidate arm.

Next gates:
1. exact-head Rust tests and clippy;
2. preregister a larger paired-seed campaign and practical improvement threshold;
3. add negative controls and independent confidence-interval/reporting code;
4. add a Symthaea proposal adapter while keeping its access to the objective strictly through observations;
5. add synthetic heteroskedastic-noise calibration and OOD shift tests;
6. only then connect manufactured SPH cases, followed by the separately qualified external CFD runner.

Industry context: compare against standard random/quasi-random designs and a named Bayesian-optimization implementation under matched observations and compute. Do not call this stronger than BoTorch, Ax, SMAC, Optuna, or any other production optimizer until a like-for-like external comparison has been run.
