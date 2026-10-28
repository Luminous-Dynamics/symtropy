# Nano-Quadrotor failure-aware rollout evaluation v0.1

**Status:** Protocol and Rust evaluator draft. The exact-head cargo test/check gate has not yet passed. No real-flight model result or safety claim is made by the tooling change.

## Motivation

Open-loop learned dynamics can diverge on difficult starts. Calculating RMSE only over finite rollouts can make a model appear better when failures disappear from the metric. A recent NanoBench fresh-seed audit explicitly retains divergent starts in a failure-aware position RMSE and analyzes paired model differences across complete recordings and training seeds. It reports a 2 m position-error divergence/penalty threshold for its specific protocol; this is not a universal threshold and must not be carried to a different platform without pre-registration.

Source: [Evaluation-Recording Contamination in Learned Nano-Quadrotor Dynamics: A Fresh-Seed Audit, July 2026](https://arxiv.org/abs/2607.18482), especially sections 6.1–6.3. Its primary result was inconclusive under its predeclared 95% interval despite a 12.1% point-estimate gap. This is a useful reminder not to equate a promising point estimate with a confirmed effect.

## Primary endpoint

For a predeclared position-error limit L, each registered rollout start contributes at every horizon:

- If the rollout is valid and the horizon error is finite and at most L, use its error.
- If it is marked non-finite, missing, timed out, violates a declared constraint, or the position error exceeds L, use penalty L.
- The same start remains in the denominator for every model. Once a rollout has diverged at an earlier horizon, later horizons must remain marked failed; the evaluator rejects silent recovery.

The primary metric is:

\[
\mathrm{RMSE}_{FA}(h)=\sqrt{\frac{1}{N}\sum_{i=1}^{N}\tilde e_{i,h}^{\,2}},
\qquad \tilde e_{i,h}=\begin{cases}
e_{i,h},& \text{finite, valid, and }e_{i,h}\le L,\\
L,& \text{otherwise}.
\end{cases}
\]

Report conditional finite RMSE and divergence rate as secondary diagnostics only. Do not replace a failed rollout with a missing row.

## Input and split contract

The evaluator takes one paired CSV with both methods, the shared truth error, all registered rollout starts, both training seeds, and all preregistered horizons. Required columns:

    flight_id,rollout_start_id,regime,training_seed,horizon_s,baseline_status,baseline_position_error_m,candidate_status,candidate_position_error_m

Status values are "ok", "non_finite", "missing_prediction", "constraint_violation", or "timeout". If a status is "ok", the corresponding position error must be finite, nonnegative, and present. A non-ok status remains a failure even when a finite diagnostic error is present.

The starts manifest is a CSV with flight_id, rollout_start_id, regime. Each registered start must appear exactly once; starts must be balanced across evaluation recordings. The evaluator requires exactly one row for every registered start × training seed × horizon combination, and rejects unregistered rows, missing rows, duplicates, training/evaluation flight overlap, regime mismatch, duplicate seed IDs, and ambiguous horizon specifications.

Once a model crosses the divergence limit at any supplied horizon, all later horizons must remain flagged as failed. If the caller only records endpoint errors, it must still mark later horizons failed if an intermediate simulation step diverged. The status must reflect failures seen at any intermediate step; the evaluator cannot infer an unreported between-horizon failure from endpoint error alone.

The JSON receipt hashes the exact bytes of the scored CSV and starts/train manifests, and records dataset receipt, paired truth/conditioning receipt, both model source commits, both model artifact-manifest digests, seed list, horizons, penalty threshold, and bootstrap seed. Model artifact digests should identify the complete per-seed checkpoint/config/scaler manifests, not a single arbitrary checkpoint.

## Crossed inference

For every horizon, report baseline and candidate:

- failure-aware position RMSE, divergent rollout count/rate, and conditional finite RMSE;
- candidate-minus-baseline RMSE and divergence-rate differences;
- percentile intervals from a paired crossed bootstrap.

A bootstrap replicate samples the complete flight IDs with replacement and training seeds independently with replacement, then applies the same sampled flight and seed indices to both methods. All starts from a sampled flight-seed cell stay together. This estimates uncertainty over these two sampled factors for the fixed evaluation design; it does not certify distribution-wide generalization. Fewer than ten recordings or ten seeds triggers a warning, not a validity guarantee.

The output also reports descriptive regime × horizon summaries. These summaries do not carry individual confidence intervals and cannot rescue a failed primary result. Predeclare the primary horizon, regime split, penalty threshold, seeds, starts, and claim rule before opening confirmatory scores.

## Run

The nanodrone_failure_aware_rollout example is wired into the existing source-check gate. From the configured Symtropy checkout plus its companion Symthaea checkout:

    STFR_CHECK_NANODRONE_RESIDUAL_SPECTRUM=1 ./scripts/test-real-symthaea-in-symtropy.sh

The example expects a paired score file already produced by the model rollouts. Example CLI:

    cargo run --manifest-path integration/symthaea-physics-loop/Cargo.toml --example nanodrone_failure_aware_rollout -- \
      --input rollout_scores.csv \
      --starts-manifest evaluation_starts.csv --train-flights manifests/train.txt \
      --dataset-receipt-sha256 <64-hex> --paired-target-receipt-sha256 <64-hex> \
      --baseline-artifact-sha256 <64-hex> --candidate-artifact-sha256 <64-hex> \
      --baseline-method physics-residual --candidate-method hdc-ltc \
      --baseline-source-commit <40-hex> --candidate-source-commit <40-hex> \
      --training-seeds 101,102,103,104 --horizons-s 0.5,1.0,2.0 \
      --channel position --divergence-limit-m 2.0 --evidence-class synthetic \
      --bootstrap 2000 --seed 17 --output failure-aware-report.json

Choose the divergence limit and horizon grid appropriate to the dataset before evaluation; 2.0 m is shown only to mirror the cited NanoBench audit's protocol. Do not reuse it uncritically on the smaller brushless Nano-Drone benchmark.

## Research targets (proposed, not results)

1. Reproduce the upstream baseline and match its exact registered starts, state conventions, motor/control inputs, and horizons.
2. Compare physics, residual MLP, physics-plus-residual, recurrent, explicit-history, ESN/online-uncertainty, and persistent HDC/LTC models using the same paired flight × seed starts.
3. Preregister one primary horizon and failure-aware RMSE endpoint. Require the paired crossed 95% interval for candidate-minus-baseline to exclude zero and a practically meaningful minimum effect (initial target: 10% relative RMSE reduction). Treat other horizons and regimes as secondary unless multiplicity is addressed.
4. Report divergence rate, conditional finite RMSE, regime-specific values, model size, per-rollout latency, p95/p99 latency, and hard deadline misses.
5. Only after an offline rollout gate passes, test whether the model improves closed-loop tracking and safety in paired simulator episodes; a lower open-loop error alone is not controller or flight-safety evidence.

Related research:
- [Decision-validity evaluator PR #2789](https://github.com/Luminous-Dynamics/symtropy/pull/2789)
- [Paired warning bootstrap PR #2792](https://github.com/Luminous-Dynamics/symtropy/pull/2792)
- [Recording contamination issue #2793](https://github.com/Luminous-Dynamics/symtropy/issues/2793)
- [Nano-Drone benchmark](https://doi.org/10.1016/j.conengprac.2026.106871)
- [NanoBench benchmark](https://arxiv.org/abs/2603.09908)

## Claim ceiling

This evaluator consumes precomputed rollout summaries. It does not train the models, independently prove the split, recompute physical-state errors from raw predictions, or assess closed-loop control. The paired crossed bootstrap only supports inference for the declared recording-by-seed design. No Rust test, real-flight score, SOTA gain, or safety claim is asserted until measured.
