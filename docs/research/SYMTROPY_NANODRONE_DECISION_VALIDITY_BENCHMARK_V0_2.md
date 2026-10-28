# Symtropy x Symthaea: Nano-Drone decision-validity benchmark v0.2

**Status:** Research evaluator draft. The Rust build is not yet verified and no real-flight warning result or safety claim is asserted.

## Research question

Can causal warning scores detect severe future prediction errors early enough to change a planning decision, while keeping false interventions and compute cost acceptably low? Persistent Symthaea HDC/LTC memory must beat strong low-cost baselines, not merely produce a novel latent state.

## Evidence and comparison set

The 2026 Nano-Drone dataset includes about 75,000 samples from a Crazyflie 2.1 Brushless across four aggressive trajectories and multi-horizon prediction out to 0.5 seconds. Its paper identifies slow, low-frequency actuation effects as an unresolved rotational-dynamics problem: https://doi.org/10.1016/j.conengprac.2026.106871

NanoBench provides a separate set of 172 recordings from a 27 g brushed Crazyflie, including Vicon ground truth, IMU, EKF, controller internals, motor PWM and battery telemetry. Keep this a separate scoreboard because the platform and protocol differ: https://arxiv.org/abs/2603.09908

Compare residual MLP, physics-plus-residual, LSTM, explicit-history MLP, ESN, RBF/ELM, online-GP uncertainty, input novelty, model disagreement, recent-error heuristic, residual-spectrum warning, and HDC/LTC persistence. Relevant published baselines include the 2025 online uncertainty comparison (https://doi.org/10.1007/s10846-025-02305-5) and the 2026 uncertainty-aware residual MPC study (https://doi.org/10.1016/j.cja.2026.104424). The latter's reported simulated gains are replication targets, not direct Nano-Drone comparisons.

## Input schema

One CSV is one warning method, physical channel and fixed target horizon. Required columns:

    flight_id,decision_time_s,outcome_time_s,risk_score,feature_max_time_s,error_value

The score must be finite in [0,1]; larger means higher risk, not necessarily a calibrated probability. Outcome time must be later than decision time. The feature-max timestamp must not exceed the decision time. Error must be nonnegative and use a declared channel/unit and geometry convention. The horizon must stay fixed within 1 microsecond. Per-flight decision times must be strictly increasing in file order. Missing, non-finite or invalid rows fail closed. Timestamp checks are only tripwires; they do not prove the upstream feature pipeline is free of hidden leakage.

## Split and receipt requirements

Supply newline-separated flight IDs in train, calibration and test manifests. The manifests must be unique, nonempty, and pairwise disjoint. Training flights cannot appear in scored CSVs; every calibration/test ID must have rows. Never split overlapping windows from one flight across splits. Freeze the model and thresholds before scoring test flights.

The JSON report hashes the exact input CSV and raw split-manifest bytes and includes the source commit, dataset receipt, paired truth/conditioning receipt, feature provenance receipt, channel, unit, error threshold, horizon, event gap, alert-merge gap, seed and evidence class. Include checkpoint/config/scaler identity within the model/feature provenance receipt. A hash is an integrity identifier, not a signature or proof of causality.

## Metrics

For each nominal alert budget, fit a threshold using calibration scores only, freeze it, then report calibration and test realized alert rates. Score ties may cause overshoot; do not retune against test labels. Report window recall/precision, AUROC, average precision (AUPRC), flight-clustered bootstrap intervals, event recall, median best lead per detected event, false alert episodes per observed flight-minute, and within-flight score-permutation control distributions. Undefined AUROC/AUPRC must stay null rather than silently becoming zero. Bootstrap whole flights, not overlapping windows.

The evaluator is for precomputed scores. It does not train a warning model, prove feature causality, quantify counterfactual controller benefit or attest flight safety.

## Cross-method comparison

The companion example nanodrone_decision_validity_compare takes baseline and candidate JSON reports and rejects comparisons unless dataset receipt, paired truth/conditioning receipt, calibration/test manifests, channel, unit, error threshold, horizon, event gap, alert-merge gap, evidence class and nominal alert budgets match. It reports candidate-minus-baseline deltas without making a unit-mixed composite score. When realized test alert rates differ by more than the declared tolerance, it explicitly labels that operating point as not matched-rate; frozen thresholds are never tuned using test labels. The deltas are descriptive only: the reports do not contain paired per-flight scores, so this comparator does not establish statistical significance or closed-loop controller benefit.

## Proposed targets, not achieved results

- Held-out flight AUPRC should beat simple baselines; report warning quality at frozen thresholds and actual alert rates.
- At a preregistered false-alert episode budget, detect more severe events with positive lead than disagreement-only, input-novelty-only, residual-spectrum-only, ESN and online-GP baselines.
- HDC/LTC persistence must improve the accuracy/safety/compute frontier over explicit-history MLP and ESN to justify added state.
- Retain same-target/same-conditioning prediction comparisons. Initial target: at least 10% lower error than the strongest reproduced baseline in 3/4 headline channels at preregistered horizons, without >5% regression in any headline channel/run.
- Only after offline gates, use paired simulator seeds for nominal dynamics, actuator lag, battery shift, sensor delay/dropout and bounded wind. Initial target: at least 20% fewer boundary violations than the strongest matched baseline and no more than 5% tracking-error regression; report hard deadline misses and p50/p95/p99/max latency.

These are proposed research targets, not industry standards. Offline warnings are not controller evidence. Closed-loop claims require simulator experiments; hardware claims require separate HIL/flight evidence, exact data receipts and per-flight failure analysis.

## Run source checks

From the Symtropy integration checkout with its required sibling Symthaea checkout configured, set STFR_CHECK_NANODRONE_RESIDUAL_SPECTRUM=1 when invoking scripts/test-real-symthaea-in-symtropy.sh. That gate runs cargo test and cargo check for both the residual-spectrum and decision-validity examples. The evaluation CLI is documented in the PR description and example usage output.


## Paired warning-method deltas

The companion `nanodrone_paired_warning_bootstrap` consumes a **single shared CSV** containing each flight/window once, with both methods' scores and the common future error. This is preferable to joining aggregate JSON: pairing is explicit at the row level and the target label is generated only once.

Required columns:

    flight_id,decision_time_s,outcome_time_s,error_value,baseline_risk_score,candidate_risk_score,baseline_feature_max_time_s,candidate_feature_max_time_s

Both models' latest feature timestamps must be no later than the decision time. Flight/time keys must be unique, times increasing within each flight, target horizon fixed, and calibration/test flights disjoint from train. Pass hashes for the dataset, shared paired truth/conditioning windows, and each model's feature-provenance receipt, plus source commits for both methods.

For every nominal calibration alert budget, the example independently fits baseline and candidate thresholds from calibration only and freezes them for test. It reports each model's actual calibration/test alert rate, AUROC/AUPRC, event recall, warning lead, false alert episodes per observed flight-minute, and candidate-minus-baseline metric deltas. It resamples **whole test flights** with the same sampled flight IDs for both methods and rebuilds event groups inside each bootstrap replicate. Percentile intervals are reported for delta AUROC, AUPRC, event recall and false-alert episode rate when defined. If realized alert rates differ by more than one percentage point, the report explicitly warns that the operating points are not matched-rate.

This is paired cohort inference, not proof of causality, controller benefit or flight safety. A fixed held-out cohort does not establish distribution-wide calibration. Pre-register severe-error threshold, horizon, event gap, alert-merge gap, flight split, warning features and bootstrap seed. The bootstrap samples are flight clusters, not overlapping windows.

Source check gate: `STFR_CHECK_NANODRONE_RESIDUAL_SPECTRUM=1 ./scripts/test-real-symthaea-in-symtropy.sh` also runs cargo test/check for the paired bootstrap example. Evaluating paired output requires frozen precomputed scores from both models and the same target windows; no real-flight score is claimed by the tooling change itself.
