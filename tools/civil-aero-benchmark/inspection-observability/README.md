# AER-01 inspection-observability oracle v0.1

**Tracking:** AERO-INSPECTION-OBSERVABILITY-001 / #4165  
**Related:** #3921 aircraft architecture; #4093 structural state bridge; #4139/#4142 NASTRAN source admission; #4236 instrument observation operators; #2890 experiment value-of-information.  
**Status:** synthetic Tier-0 exact oracle only.

## Purpose

This crate implements a small, deterministic sensor-subset benchmark for a finite-dimensional linearized local observation model. It verifies the arithmetic and claim boundaries required before an inspection/observability optimizer consumes real structural models.

It does not parse NASTRAN, solve a beam, predict strain, identify physical damage, estimate fatigue life, qualify an NDE procedure, or establish aircraft SHM capability.

## Model

For parameter vector theta and a selected set of sensor channels:

    y = H theta + epsilon
    epsilon covariance = R = diag(sensor variances)

Each parameter declares a positive characteristic scale. The evaluator uses the normalized sensitivity matrix:

    H_normalized = H diag(parameter_scales)
    F = H_normalized^T R^-1 H_normalized

The evaluator reports, without regularization:
- observation-matrix rank and nullity for each selected subset;
- Fisher information matrix;
- ascending eigenvalues and minimum eigenvalue;
- log determinant only for full-rank positive-definite subsets;
- exhaustive D-optimal (maximum log-determinant) and E-optimal (maximum minimum eigenvalue) selections;
- exact input SHA-256, numerical tolerances and claim ceiling.

Sensor IDs are sorted before enumeration, and ties are resolved lexicographically. The small-instance limit is 16 candidate sensors and 8 parameters; every subset of the requested cardinality is enumerated exactly.

## Frozen synthetic known answer

Four synthetic sensors observe two normalized parameters. Select two sensors.

The locked answer is:
- 6 subsets enumerated;
- all 6 are full rank for this fixture;
- D-optimal subset: S_A + S_C;
- E-optimal subset: S_A + S_C;
- Fisher information for S_A + S_C: [[2,1],[1,1]];
- minimum eigenvalue: (3 - sqrt(5))/2, approximately 0.38196601125;
- log determinant: approximately 0 (floating-point result is within the locked tolerance).

This fixture is constructed solely to test known-answer arithmetic and deterministic selection. Its sensor sensitivities, variances and parameter scales are not physical CRM or AER-01 values.

## Reproducible toolchain inputs

Direct Rust dependencies are pinned in Cargo.toml and the complete resolved dependency graph is committed in Cargo.lock. CI uses Cargo's --locked mode for tests and execution, so dependency resolution may not silently mutate the lockfile. The lockfile SHA-256 is included in the retained evidence bundle. The bundle also records the checked-in Rust toolchain file, rustc/cargo/Python versions, runner OS/architecture/image identifiers and kernel identity, with its own SHA-256 receipt.

## Deliberate v0.1 boundaries

- Only a synthetic local linear observation model is admitted.
- Measurement noise is independent across sensors and represented by one positive scalar variance per sensor. Correlated covariance, time dynamics, frequency response and nonlinear observability are not supported in v0.1.
- Parameter scaling is explicit because singular values and Fisher metrics depend on coordinate scaling.
- Rank and eigenvalue tolerances are fixed and reported. They are numerical policies, not physical detectability thresholds.
- D-optimal selection is unavailable when no full-rank subset exists.
- E-optimal results preserve rank and nullity; a best E-optimal subset can still be unobservable.
- No diagonal loading, ridge term or other regularization is applied to make a singular information matrix look observable.
- No probability-of-detection (POD) or false-alarm claim can be inferred from this synthetic model.

## Fault-tolerant follow-on v0.1

The new Rust binary fault_tolerant_observability.rs is a separate exact Tier-0 experiment built on the same synthetic linear observation contract. It enumerates every fixed-cardinality sensor subset and evaluates it against:
1. no failure;
2. each individual sensor dropped in turn;
3. each declared common-cause group dropped in turn.

For every subset and scenario it records surviving sensor identities, observation rank/nullity, the minimum eigenvalue of the two-parameter Fisher information matrix, and log determinant when full-rank. A subset is robustly feasible only when **every declared scenario remains full rank**. Among robustly feasible subsets, the exact maximin E-optimal policy chooses the subset maximizing:

    max_S min_f lambda_min(F(S minus failed(f)))

The finite fixture has six candidates, four selected sensors, six single-sensor outage scenarios, three common-cause group outages and the no-failure baseline: 15 exact subsets × 10 scenarios. Its known answer is deliberately adversarial:
- nominal D-optimum: S_A + S_B + S_C + S_D, with log determinant ln(100) approximately 4.60517018599;
- nominal E-optimum: S_A + S_B + S_C + S_D, with minimum eigenvalue 10.0;
- that nominal optimum is rejected because loss of PWR_X or PWR_Y leaves a rank-one observation;
- robust maximin D-optimum: S_A + S_C + S_E + S_F, with worst-case log determinant ln(25) approximately 3.21887582487;
- robust maximin E-optimum: S_A + S_C + S_E + S_F, with worst-case minimum eigenvalue approximately 4.4826666667;
- 12 of 15 candidate subsets satisfy all 10 full-rank constraints.

D-optimality maximizes information volume (log determinant); E-optimality maximizes the weakest normalized parameter direction (minimum eigenvalue). Agreement on this fixture is a property of this intentionally small symmetric corpus, not a theorem that the objectives are interchangeable.

The Python verifier independently recomputes every subset/scenario result and compares the complete report produced by Rust. Hostile controls include duplicate/unknown group members, duplicate sensor IDs, non-positive variance and a rank-deficient corpus for which no robust selection may be emitted.

### Exact scope and limitations

This is a deterministic, adversarial, single-failure contract, not a probabilistic reliability model. It assumes exactly one sensor OR one declared common-cause group fails at a time. It does not combine simultaneous faults, assign outage probabilities, model sensor bias/drift/stuck-at/saturation/detachment, distinguish sensor faults from damage signatures, represent spatially correlated noise, or account for sensor mass, wiring, power, calibration and inspection cost. Those must be introduced as explicit evidence-bearing inputs in later versions. A common-cause group is only as complete as its declared membership; unknown physical dependencies remain unknown.

The result is an observability and identifiability proxy under the synthetic local linear model. Full rank is necessary for the modeled two-parameter identification problem, but does not prove adequate physical sensitivity, defect POD, diagnosability, or safe aircraft operation.

## Local execution

From the repository root:

    cargo fmt --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml -- --check
    cargo test --locked --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml
    cargo run --locked --quiet --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml > /tmp/observability-result.json
    python3 tools/civil-aero-benchmark/inspection-observability/verify_independent.py --rust-output /tmp/observability-result.json
    cargo run --locked --quiet --manifest-path tools/civil-aero-benchmark/inspection-observability/Cargo.toml --bin fault_tolerant_observability > /tmp/fault-tolerant-result.json
    python3 tools/civil-aero-benchmark/inspection-observability/verify_fault_tolerant_independent.py --rust-output /tmp/fault-tolerant-result.json

The Python verifiers use only the standard library and independently recompute their respective finite corpora. They do not import Rust output-generation code.


## Correlated and fault-tolerant observation follow-on v0.1

The Rust binary src/bin/correlated_observability.rs combines the full channel covariance model with declared sensor-outage scenarios in one exact Tier-0 evaluator. It computes each selected/subset-and-scenario Fisher information matrix without explicitly forming the covariance inverse:

    F = H_normalized^T R_selected^-1 H_normalized

The implementation factors each selected covariance principal submatrix with Cholesky and solves the two right-hand sides. It does not add diagonal loading, jitter, or another regularizer to make a singular covariance pass.

### Identity and covariance contract

The covariance contract binds:
- a covariance ID and explicit provenance status;
- an exact channel_ids vector that must match the sensor records and matrix row/column ordering;
- an explicit covariance unit;
- a finite, symmetric, positive-definite covariance matrix;
- canonical sorted sensor/channel IDs and exact input SHA-256;
- explicit common-cause failure-group membership and maximum simultaneous failure-domain count.

Unknown, duplicated, missing or reordered channel identities are rejected. Symmetry, positive definiteness, channel/group membership and dimensional consistency are validated before subset evaluation.

### Joint frozen synthetic known answer

Six channels and four selected sensors produce 15 exhaustive candidate subsets. Every subset is evaluated against 10 scenarios: no failure, each of six individual sensor dropouts, and each of three declared common-cause group outages.

The full-covariance nominal selections are:
- D-optimum: S_A + S_C + S_E + S_F;
- E-optimum: S_A + S_B + S_E + S_F.

The full-covariance fault-tolerant maximin selections are:
- robust D-optimum: S_A + S_C + S_E + S_F, worst-case log determinant approximately 3.22529639267;
- robust E-optimum: S_A + S_C + S_E + S_F, worst-case minimum eigenvalue approximately 2.90608817764;
- 12 of the 15 candidate layouts retain full rank across all 10 declared scenarios.

The diagonal-only assumption baseline deliberately discards the off-diagonal covariance entries:
- nominal diagonal-assumption D and E optima: S_A + S_B + S_C + S_D;
- robust diagonal-assumption D and E optima: S_A + S_C + S_E + S_F.

This is the desired negative control: the independence assumption changes the nominal recommendation. The joint failure scenarios then force both robust strategies to retain a more diverse sensor layout. These are fixture-specific synthetic results, not measured AER-01 values.

The independent Python verifier recomputes every subset from the selected covariance principal submatrix under every declared failure scenario, verifies full and diagonal-assumption D/E metrics and compares the entire Rust report. Hostile controls include covariance channel-order mismatch, nonsymmetry, non-positive-definiteness, dimension mismatch, unknown/duplicate failure-group membership and rank deficiency.

### Exact scope and limitations

This is a local linear, two-parameter, synthetic covariance and single-failure-domain oracle. It does not estimate covariance from physical data, address nonstationarity, temporal correlation, frequency-dependent noise, missingness, calibration uncertainty, covariance estimation error or covariance shift. The declared covariance provenance is deliberately limited to synthetic_fixture_known_answer; experimental provenance is rejected by v0.1. A future source-admitted covariance must bind acquisition campaign, channel calibration, preprocessing, sampling, units, time interval, estimator, sample count, missing-data policy and uncertainty in the covariance estimate.

The failure model assumes exactly one sensor OR one declared common-cause group fails at a time. It is not a probabilistic reliability model and does not enumerate simultaneous independent failures or unmodeled common causes. A declared common-cause group is only as complete as its membership evidence; unknown physical dependencies remain unknown.

Full-rank Fisher information still does not establish physical defect detectability, POD, sensor reliability, diagnosis accuracy or aircraft safety.
## Next qualification steps

1. Add full covariance only with exact channel identity/order, calibration, units and source-bound covariance provenance.
2. Compose #4236 so each sensor's spatial/temporal transfer, mask, calibration and timebase is applied before comparison.
3. Add declared sensor dropout, drift and common-cause power/data failures, retaining observability loss explicitly.
4. Only after #4139 admits exact NASA BDF bytes and terms, derive candidate structural DOFs from its topology. Do not infer physical stiffness, masses, loads, damage signatures or sensor noise from unknown BDF fields.
5. Compare uniform/engineering placement, effective independence, QR-pivot, D-optimal, E-optimal, modal/strain-energy and decision-value-guided policies under identical total cost.
6. Use physical NDE/POD evidence for defect-detection claims; synthetic observability is not physical inspection qualification.

## Claim ceiling

The original oracle's only positive claim is synthetic_local_observability_only. The fault-tolerant follow-on's claim ceiling is synthetic_single_sensor_dropout_and_single_common_cause_group_only. Neither output is structural adequacy, damage detection, damage tolerance, fatigue, flutter, airworthiness, SHM safety or maintenance-release evidence.
