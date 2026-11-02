# ANIMA Intervention Discrimination Benchmark v0

This standalone, dependency-free Rust crate implements the first executable part of Symtropy #3345: **choose experiments that distinguish competing mechanisms, then evaluate those mechanisms on intervention-held-out outcomes**.

It is not a biological model, a connectome importer, or evidence that any circuit is correct. Tests use synthetic fixtures only.

## Commands

```bash
cargo fmt --manifest-path research/anima-intervention-bench-v0/Cargo.toml -- --check
cargo test --manifest-path research/anima-intervention-bench-v0/Cargo.toml --locked
cargo clippy --manifest-path research/anima-intervention-bench-v0/Cargo.toml --locked --all-targets -- -D warnings
```

## Implemented contract

### 1. Leakage-resistant splitting

`build_group_held_out_split` holds out entire intervention families, individuals, and/or connectome datasets. All rows from a held-out group remain outside training. It rejects duplicate sample IDs, missing holdout groups, empty partitions and mixed outcome metric/unit schemas.

The split is explicit and deterministic. It does not randomly distribute adjacent timesteps from the same individual between training and test.

### 2. Evidence-class claim ceiling

Every observation has an explicit origin class:
- peer-reviewed measured;
- preprint measured;
- in-silico generated;
- synthetic fixture.

`validate_evidence_claim` rejects attempts to promote generated fixtures to empirical replication. Cross-dataset generalization requires peer-reviewed measured observations from at least two connectome datasets and two individuals.

### 3. Probabilistic held-out scoring

Each model prediction is joined by exact `sample_id`, not row position. Duplicate, missing, unexpected or wrong-model predictions fail closed.

For a single declared scalar metric/unit per run, the scorecard reports:
- RMSE;
- mean Gaussian log predictive density (higher is better);
- empirical coverage of the central 90% predictive interval.

Do not pool different metrics or units into one score. Do not reinterpret one paper's TCS definition as another's.

### 4. Paired individual-cluster bootstrap

`paired_cluster_bootstrap_log_score` compares candidate and baseline predictions on exactly the same held-out observations. It resamples individuals as clusters, preserving all of an individual's trials per resample. It reports the paired mean log-score difference and a deterministic 95% percentile interval.

This prevents repeated steps from one animal from being counted as independent biological replicates.

### 5. Expected information gain for intervention selection

`estimate_expected_information_gain` estimates mutual information between model-family identity and a scalar outcome under equal model priors. It uses Monte Carlo posterior model probabilities across all competing Gaussian predictive distributions.

`rank_interventions` ranks feasible interventions by expected information gain per declared comparable cost unit. It rejects model-family-set drift, inconsistent metric/unit schemas, invalid costs and invalid predictive uncertainties. Infeasible interventions are excluded, not silently assigned a low score.

This is Bayesian model-discrimination design, not a pairwise-KL proxy. The v0 independence assumption is one scalar endpoint per experiment; correlated multivariate outcomes and non-equal model priors are future extensions.

## Required next integration

Before this is used on biological data, each run must bind:
- exact intervention protocol and feasibility/cost source;
- source paper/version and evidence class;
- connectome dataset ID, release/materialization and source digest;
- individual/genotype/specimen IDs and rights-cleared observation receipt;
- exact model-family and parameter/calibration digests;
- outcome metric ID, unit, extraction method and raw measurement digest;
- split policy and resulting sample-ID sets;
- Monte Carlo sample count, seed and ranking output;
- raw per-sample predictions, outcomes and diagnostics.

The model family that wins must be selected on held-out interventions. Any synthetic adapter fixture is strictly a software test, not animal evidence.

## Proposed model families

Keep these separate and versioned: Pugliese's source-grounded VNC rate network; Sapkal's preprint hypothesis; independent per-leg oscillators; coupled phenomenological CPG; connectome-constrained local circuits with central coupling disabled; hybrid neural + sensory/contact feedback; FlyGym/NeuroMechFly CPG, rule-based and hybrid controllers.

Do not assert that these model families have been reproduced until their source data, circuit identities and parameterization are pinned.

## Metrics and acceptance targets

The first run should report held-out predictive density, interval coverage, RMSE by metric, worst intervention/individual score, intervention cost, and model discrimination power. After a baseline variance and sample-size study, consider a preregistered target such as 10% relative held-out predictive-score improvement over the strongest non-connectome baseline, while retaining locomotion performance and reporting compute overhead.

No SOTA, causal-mechanism, or biological-generalization claim follows from this synthetic test harness.
