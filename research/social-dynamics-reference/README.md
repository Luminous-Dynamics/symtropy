# Social Dynamics Reference Model (research prototype)

This is a small, dependency-free Rust reference implementation for the synthetic scenario proposed in ../social-dynamics-v0-1. It exists to exercise the scenario mechanics and make assumptions inspectable. It is **not** a validated model of real people or a claim that any treatment is superior.

## Run

From the repository root:

    cargo test --manifest-path research/social-dynamics-reference/Cargo.toml
    cargo run --release --manifest-path research/social-dynamics-reference/Cargo.toml
    cargo run --release --manifest-path research/social-dynamics-reference/Cargo.toml -- --runs 256 --runs-csv social-dynamics-runs.csv

The executable prints one summary CSV row per scenario and treatment, including paired relative reduction in unmet need and a deterministic 95% paired-bootstrap interval versus the no-commitments baseline. By default it runs 32 paired seeds for a 128-agent ring-lattice community under a temporary shared-water capacity shock. Use --runs N to change the seed count (minimum 20), --sweep to run the baseline plus eight one-factor sensitivity cases, and --runs-csv PATH to write one raw row per scenario/treatment/seed for independent auditing. The Rust scenario defaults are documented in Scenario::default; the JSON fixture is an accompanying machine-readable scenario sketch and has not yet become the executable input format.

## Treatment arms

- no_commitments: local willingness and shared-shock urgency only;
- static_trust: fixed initial tie-trust estimates and urgency;
- reciprocity: previous-neighbor contribution rate and fixed trust;
- evidence_based_trust: noisy observations with an explicit signal-tampering rate, trust updating, and explicit observation overhead.

The rules are deliberately simple reference policies, not behaviorally calibrated mechanisms. Their ordering is an empirical question. Do not change assumptions after seeing the held-out scores and then describe the result as a preregistered prediction.

## Modeling contract

- The network is a deterministic degree-four ring lattice.
- The canonical module exposes ring_lattice, voter_update (one asynchronous voter event), and degroot_step (one synchronous DeGroot averaging step). Its tests cover graph validity, absorbing consensus, local-update semantics, convex-hull preservation, and mean preservation for a regular undirected ring. These are formal-mechanism sanity checks, not yet an independent replication of a published empirical benchmark.
- Randomness uses an in-crate SplitMix64 generator to avoid an external RNG version dependency.
- The model pre-draws action/noise/tampering channels each tick so treatment branches do not consume different amounts of randomness. Initial states and random channels are paired by seed.
- Resource reserve bounds contribution feasibility; shared service capacity sets the main unmet-need outcome.
- Recovery is counted only if capacity reaches 98% of baseline within the declared shock window.
- The benchmark reports unmet need, peak unmet need, the Gini coefficient of cumulative unmet need, total contributions, recovery rate/time, observation overhead, and repair progress. The optional sweep varies observation noise (0.00/0.20), observation cost (0.00/0.35), signal-tampering rate (0.00/0.20), and shock capacity (0.25/0.65) one factor at a time around the default scenario. It does not test interactions between factors or network-density changes.
- Tests cover deterministic replay, invalid-parameter rejection, finite/bounded metrics, a zero-duration shock, Gini calculations, and paired-bootstrap behavior on known synthetic values (eighteen tests total across the library and binary).

For the next empirically grounded benchmark, see [the external-validation plan](../../docs/research/social-dynamics-v0-1/EXTERNAL_VALIDATION_PLAN.md). It requires license verification and study-level held-out evaluation before any claims about human behavior.

## Known limitations

This prototype has no empirical calibration, no spatial heterogeneity, no institutional-capture mechanism; the only adversarial behavior represented is signal flipping, no real Mycelix or Symthaea integration, and no Sol-Atlas input. The network is a synthetic degree-four ring lattice and fixed during each run. The model omits strategic adaptation, household movement, service queues, equity weights, and confidence intervals around forecasts. These are research limitations, not hidden capabilities.

Do not use outputs to make claims about a real city, real households, individual trustworthiness, or policy effectiveness. The paired-bootstrap interval quantifies variation across this toy model's synthetic seeds; it is not a confidence interval for real people and is not evidence of empirical prediction. The next valid step is to test canonical models and assess whether the mechanism-specific treatment effects survive ablation, parameter sensitivity, and held-out validation.
