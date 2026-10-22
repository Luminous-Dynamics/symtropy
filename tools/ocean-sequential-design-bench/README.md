# Ocean Sequential Design Benchmark

Synthetic Rust benchmark for adaptive observation under imperfect detection.

## Run

From the repository root:

    cargo fmt --manifest-path tools/ocean-sequential-design-bench/Cargo.toml --check
    cargo test -p ocean-sequential-design-bench --locked
    cargo run --quiet -p ocean-sequential-design-bench --locked -- --corpus docs/tech/OCEAN_SEQUENTIAL_DESIGN_CORPUS_V0_1.json --source-commit "$(git rev-parse HEAD)" --receipt target/ocean-sequential-design-receipt.json

The receipt binds the exact corpus bytes, Git source commit, schema/suite/tool versions and random seed. The focused workflow uploads a machine-readable receipt.

## Observation model

Each scenario has a synthetic latent target probability. For each candidate, the model separately records:

- sensitivity: P(positive observation | target present);
- false-positive rate: P(positive observation | target absent);
- minimum detectable signal and expected signal;
- effort, rights review, explicit use permission, observation mode and separate exposure channels.

Bayesian updates keep latent state and observation outcomes separate. A non-detection lowers posterior probability under the stated likelihood; it never becomes a categorical absence label. A candidate is not eligible when the detector profile is missing, expected signal is below the declared minimum, rights review is incomplete, use is not explicitly permitted, mode is contact, or exposure is unknown.

Repeated observations within the same declared correlation group are treated as perfectly correlated in this synthetic V0 and may not be selected twice. Distinct windows/groups are separate observations under the stated conditional-independence assumption. These are explicit toy-model assumptions—not real sensor calibration or a validated ecological model.

## Policies and oracle

The benchmark compares fixed-seed random, largest coverage gap, repeat-the-dominant-stratum, batch-priority information gain, adaptive greedy information gain, and an exhaustive adaptive decision-tree oracle for tiny bounded cases. The oracle and heuristics use the same likelihoods, candidate set, horizon, effort budget and independent sound/light/sediment caps.

Information gain is measured as expected reduction in Bernoulli entropy under the synthetic prior/likelihoods. This is a method-comparison metric, not a biological value or environmental-safety score.

## Boundaries

All probabilities, signal strengths, strata and action records are synthetic. The tool is not a field-calibrated occupancy model, abundance estimator, environmental-impact assessment, physical ocean/plume model, navigation product, contact authorization, mining-site selector or extraction optimizer. No real CCZ biodiversity observations are bundled or inferred.

Next scientific steps are correlated-noise models validated against independent simulation, time-varying latent states, explicit false-positive/false-negative calibration evidence, held-out Brier/log-loss evaluation, sensitivity analysis, and transparent Pareto reports.
