# Symtropy × Symthaea — Nano-Quadrotor system-identification adapter v0.1

**Status:** research protocol and adapter contract only. No external dataset has been imported into CI; no benchmark score, leaderboard result, or physical-flight claim is made by this document.

## Why this benchmark

The internal Symtropy loop is useful for causal feature, schema, and integration tests, but its spherical-body simulator receives aggregate thrust and body moments. The IDSIA [Nano-Quadrotor System Identification Benchmark](https://github.com/idsia-robotics/nanodrone-sysid-benchmark) uses real Crazyflie 2.1 Brushless flight data with four motor angular velocities as inputs. Those are different control interfaces and different dynamics; internal RMSE must not be compared numerically with the public benchmark.

The public benchmark README describes roughly 75,000 synchronized real-flight samples and physics, neural, hybrid physics-plus-residual, and LSTM reference models. Its documented protocol trains on Square, Random, and Chirp trajectories and reserves Melon for trajectory-level testing. The reference LSTM test script uses a 50-step horizon at 100 Hz (0.5 seconds) and test runs 1–3 of Melon. The repository's metric utilities report position, velocity, angular-rate, and rotation errors at each horizon and a sum-over-horizons summary.

## Exact data contract

### Inputs and outputs

Read the released trajectory files and preserve their timestamps and split identities. The reference dataset loader selects the four motor-speed columns:

- \`m1_rads\`, \`m2_rads\`, \`m3_rads\`, \`m4_rads\`

It forms the measured state from world-frame position, world-frame velocity, orientation, and body-frame angular velocity. The raw measurements include quaternion orientation (13 state channels); the reference loader defaults to a 12-dimensional model state by mapping quaternion orientation to an SO(3) log/rotation vector. The Rust adapter must pin one explicit convention and document every conversion; it may not mix a four-component quaternion with three-component rotation-vector labels.

Before training, validate monotonic timestamps, missing/duplicate rows, finite values, quaternion normalization/sign handling, motor-speed units, and sample intervals. Keep a manifest with dataset archive hash, file hashes, trajectory/run IDs, row counts, data schema version, source revision, and split membership.

### Split and normalization

- Training trajectories: Square, Random, Chirp.
- External held-out test trajectory: Melon; do not use it for feature selection, scaling, early stopping, ridge selection, calibration, or model architecture decisions.
- Any tuning and uncertainty-calibration split must be carved from training trajectories by contiguous run/trajectory blocks. Do not randomly split overlapping sequence windows.
- Fit every normalizer on training data only; reuse the exact fitted transform for validation/test. Persist the fit source IDs and scaler digest.
- Reproduce the official sequence alignment: start state \(x_t\), motor inputs \(u_{t:t+H-1}\), and ground-truth future states \(x_{t+1:t+H}\). Never let an overlapping window cross a trajectory/run boundary.

The upstream reference test script contains interactive model selection. A reproducible Symtropy comparison must replace that interaction with pinned model/config IDs and emit a machine-readable receipt; do not copy an interactive run as if it were a reproducible evaluation.

## Implemented first gate: Rust CSV integrity validator

The integration package now includes `examples/nanodrone_dataset_validate.rs`. It is intentionally a data-ingress gate, not a predictor. It accepts one official-format CSV at a time, takes the trajectory-level split and run ID as explicit arguments, and emits a JSON receipt with the raw-file SHA-256.

Run it from the Symtropy checkout after cloning the public benchmark separately and setting its path:

```bash
export NANODRONE_PATH=/path/to/nanodrone-sysid-benchmark
cargo run --manifest-path integration/symthaea-physics-loop/Cargo.toml \
  --example nanodrone_dataset_validate -- \
  "$NANODRONE_PATH/data/test/melon_20251017_run1.csv" test melon run1
```

For training files, pass `train` and one of `square`, `random`, or `chirp` with `run1`–`run3`; pass `validation` with those same trajectories and `run4`; for held-out evaluation, pass `test melon` with `run1`–`run3`. The validator rejects a split/trajectory/run mismatch, a file outside the expected `data/train` or `data/test` directory, a filename whose first trajectory token or final run token does not match exactly, missing or duplicate required headers, malformed/non-finite required values, non-monotonic or out-of-tolerance sample periods, quaternions whose norms fall outside the declared acceptance range, and negative motor angular velocities. Header order is resolved by name and covered by a reversed-header regression test.

The official split is enforced at both trajectory and run level: Square/Random/Chirp run1–run3 are training, run4 is validation, and Melon run1–run3 is the held-out test set. This matches the upstream training protocol's `train_runs=[1,2,3]`, `valid_runs=[4]` and its Melon test runs 1–3. The validator preserves the source-file digest and does not normalize or rewrite data. It pins the public input order (`m1_rads` through `m4_rads`), raw `qx,qy,qz,qw` quaternion convention, body-frame angular velocity, and 12D SO(3)-log model-state convention in its receipt. Do not concatenate trajectories before validation: the unit of integrity and the bootstrap unit is an independent trajectory/run, not a sliding window.

This gate proves only the integrity properties it checks. It does **not** establish parity with the upstream loader, reproduce the official multi-step metrics, prove model quality, or certify safety. The next gate remains parity against a pinned upstream baseline with frozen normalizers and exact horizon alignment. CI runs the validator's unit tests and compile check without needing to download the dataset; a real dataset receipt still requires the public CSV files.

## Implemented second gate: run-isolated multi-horizon metric receipt

The integration package now includes `examples/nanodrone_metric_receipt.rs`. It consumes three **separate per-run prediction CSVs**, requires exactly one each for Melon run1, run2, and run3, and reports horizon 1–50 position/velocity/rotation/angular-rate metrics and the cumulative simulation-error sums. It fails closed on missing/non-finite required predictions, duplicate headers, invalid timing, invalid quaternion norms, and mismatched run/orientation schemas.

Example, for per-run result CSVs that expose the truth columns and each model's `*_pred_h1` through `*_pred_h50` columns:

```bash
cargo run --manifest-path integration/symthaea-physics-loop/Cargo.toml \
  --example nanodrone_metric_receipt -- \
  results/melon_20251017_run1.csv \
  results/melon_20251017_run2.csv \
  results/melon_20251017_run3.csv
```

The CLI supports the upstream table's 12D SO(3)-log representation (`rx,ry,rz`) or a complete XYZW quaternion representation, and implements the public metric helper's vector-norm errors and quaternion relative-rotation formula. It also reports a separate sign-invariant SO(3) distance: the public helper's raw `2*atan2(||v||,w)` formula does not canonicalize quaternion sign, so antipodal representations of the same rotation can produce a nonzero (in the identity-versus-negative-identity case, `2π`) reference-formula error. Both metrics are preserved rather than silently changing the published score. It emits per-run scores, a macro-mean over the three independent runs, and sample counts at every horizon. It never shifts truth across run boundaries. If all per-horizon truth columns `{state}_true_h{h}` are provided, it additionally reports an all-available-window score; it will not infer those targets from the one-step truth column.

### Evaluation alignment audit

Source review of the benchmark's [test script](https://github.com/idsia-robotics/nanodrone-sysid-benchmark/blob/main/test/test_lstm.py), [dataset loader](https://github.com/idsia-robotics/nanodrone-sysid-benchmark/blob/main/dataset/dataset.py), and [official metric helper](https://github.com/MaartenSchoukens/nonlinear_benchmarks/blob/master/nonlinear_benchmarks/nanodrone_error_metrics.py) surfaced an alignment hazard to account for in reproduction:

- The loader constructs each window with the full target sequence `x_{t+1:t+H}`, but the reference test script stores only `trues[:, 0, :]` in the CSV.
- The metric helper then applies a `shift(-h+1)` to that one-step-ahead truth series for each horizon. That matches the intended horizon value for its retained rows, but omits `h-1` otherwise available prediction windows at horizon `h`.
- The test script builds a `ConcatDataset` from Melon runs 1–3 before creating the output table. A global shift on the concatenated table can therefore cross a run boundary for the last `h-1` rows before each run boundary, because the output table does not retain run IDs.

The public orientation helper also computes `2*atan2(norm(vector), scalar)` on the relative quaternion without sign canonicalization. Since `q` and `-q` encode the same orientation, this formula can penalize sign-equivalent representations; the metric receipt reports a sign-invariant SO(3) diagnostic alongside the exact source formula.

This is an implementation-level audit finding, not a claim that published scores are invalid. For **leaderboard parity**, preserve a separately labeled source-compatible score and disclose its exact row semantics. For a scientifically cleaner primary metric, score each run independently and export per-horizon ground-truth targets so every available window can be evaluated. Do not quietly replace the reference number with the corrected metric; report both, with the alignment and sample counts attached.

The receipt deliberately does not claim an official leaderboard submission or a numerical improvement. It provides the run isolation, metric implementation and explicit sample accounting needed before that comparison can be trusted.

## Implemented third gate: attach complete future-state windows

The integration package now includes `examples/nanodrone_truth_window_join.rs`. It attaches the actual raw trajectory state at row `i+h` to prediction window row `i` for every horizon `h=1..50`. It supports both independent per-run prediction files and the **pooled CSV emitted by the current upstream test script**.

Because the upstream `test_lstm.py` currently concatenates Melon runs 1–3 and writes `melon_multistep.csv`, the pooled mode partitions its rows by each raw trajectory's exact `N-50` window count, then joins and validates each run separately:

```bash
cargo run --manifest-path integration/symthaea-physics-loop/Cargo.toml \
  --example nanodrone_truth_window_join -- \
  "$NANODRONE_PATH/out/predictions/lstm_random_square_chirp_model_multistep/melon_multistep.csv" \
  "$NANODRONE_PATH/data/test/melon_20251017_run1.csv" \
  "$NANODRONE_PATH/data/test/melon_20251017_run2.csv" \
  "$NANODRONE_PATH/data/test/melon_20251017_run3.csv" \
  results/windowed --pooled --state-units-si
```

The output directory must be a fresh path that does not already exist; the tool stages all three files and atomically renames the completed directory only after every run passes. The output directory contains one joined CSV for each Melon run. Pass those files to `nanodrone_metric_receipt` in run1/run2/run3 order. For already separated prediction tables, the CLI also supports one raw run + one matching prediction CSV + one output path.

Before writing each output, the joiner checks exact run identity, raw schema and quaternion norms, finite values, timestamp increments, each prediction horizon, and the upstream `N-50` row-count rule. The current explicit thresholds are ±2 ms about the 10 ms sample period, quaternion norm 0.95–1.05, absolute tolerance 0.005 for base position/velocity/angular-rate agreement, and 0.01 rad for orientation agreement; the real published files still need to confirm these thresholds are appropriate. It then verifies that the prediction CSV's one-step truth agrees with raw row `i+1` for position, velocity and angular rate, and compares the orientation under a sign-invariant SO(3) tolerance. Those checks catch off-by-one alignment, normalized-value mistakes and incorrect row slices before they can produce a score.

The joiner appends complete `{state}_true_h{h}` columns, writes each output atomically, and emits a batch receipt with source hashes, per-run row offsets, counts, orientation convention and output hashes. The required `--state-units-si` flag explicitly asserts that prediction state values have been denormalized into physical units; the receipt records this assertion but cannot prove the scaler history from CSV contents alone.

Synthetic tests cover exact h=1/h=50 target attachment, deliberate one-step mismatch rejection, and pooled splitting when the three raw runs have unequal lengths. These are regression tests, not evidence yet that the published CSVs pass the declared timestamp, quaternion and alignment tolerances. The result is an alignment receipt, not model-performance or safety evidence.

## Model arms

Compare on identical rows, scalers, motor input sequences, horizons, and hardware budgets:

1. Released physics-based reference model.
2. Released neural model.
3. Released physics-plus-residual model.
4. Released LSTM model.
5. Symthaea instantaneous HDC encoder + trained readout.
6. Symthaea HDC/LTC reservoir + trained readout.
7. Symthaea HDC/LTC with end-to-end-trained residual dynamics.
8. Explicit finite-history model with matched state/action history length.
9. Compact current-state and history MLPs.
10. Negative controls: shuffled training targets and time-shifted/misaligned histories.

All learned arms must train only on training trajectories. Use a fixed, declared tuning budget and publish parameter count, trainable parameter count, training wall time, peak memory, model size, CPU/accelerator, p50/p95/p99 and worst-observed inference latency, and energy/power where measurable. A model that wins only with a much larger training or inference budget must be presented as a budget trade-off, not an unqualified architecture win.

## Metrics and reporting

Reproduce the upstream per-horizon errors and extend the receipt with paired uncertainty:

- Position error: Euclidean norm in metres.
- Velocity error: Euclidean norm in metres/second.
- Body angular-rate error: Euclidean norm in radians/second.
- Orientation error: rotation-geodesic angle using a clearly pinned quaternion convention, reported in radians/degrees.
- Per-horizon errors at 1, 5, 10, 20, and 50 steps; publish the full 1–50 horizon curves as well.
- The upstream sum-over-horizons summary may be shown, but never use it as the only metric: it can hide instability at long horizons.
- Per-trajectory/run metrics, mean, median, p95, worst run, and empirical CVaR90 across independent runs.
- Paired model deltas with confidence intervals bootstrapped by trajectory/run, not overlapping windows.
- Invalid predictions, quaternion normalization failures, divergence, and horizon-specific catastrophic errors must remain in the denominator/receipt; never silently drop them.

For uncertainty-aware variants, report interval coverage and width by trajectory/run, but Melon is deliberately a trajectory shift from training. Empirical coverage on Melon is a shift diagnostic, not a distribution-free guarantee. Do not convert prediction intervals into a safety claim.

## Physical-interface compatibility gate

Before comparing with the official leaderboard, the Symtropy model must accept the released four motor-speed inputs and use the official state/time conventions. If control outputs are needed, add and validate an actuator model (motor lag, command-to-speed dynamics, saturation, and a pinned motor mixer/inertia convention) before mapping motor speeds into thrust/body moments. Do not synthesize missing motor signals from aggregate commands and call that a matched evaluation.

The first adapter milestone is **open-loop system identification only**: read the official train/test data, reproduce a reference model metric on the held-out trajectory, and prove the receipt records the expected trajectory/run IDs, row counts, normalizer digest, and exact horizon alignment. Next add the explicit-history and HDC/LTC arms. Only after this pass should any model feed MPC or be assessed in SITL/HITL.

## Proposed go/no-go targets (not achieved results)

These are engineering hypotheses to preregister, not published standards:

- Data parity: reproduce the released reference evaluation and trajectory-level Melon split before reporting a Symthaea score.
- Generalization: for an HDC/LTC temporal-representation claim, require at least 10% lower paired multi-step error than both the released LSTM/physics-plus-residual references and the explicit-history MLP on predeclared horizons, with powered run-level confidence intervals. Report every horizon, even if some regress.
- Long-horizon behavior: no material degradation at horizons 20 and 50; specify thresholds before viewing test results.
- Compute: meet a target-hardware control deadline at p99 with zero invalid predictions and zero unreported deadline misses. Publish the exact hardware and profiling method.
- Robustness: add separately versioned tests for actuator lag, sensor bias/dropouts, timestamp jitter and altered payload only after exact public-data parity; do not contaminate the official held-out score with these internal stress tests.
- Integrity: source/data/scaler/config/model digests, trajectory/run IDs, seed sequence, code revision, units and state-convention version must be emitted in the result receipt.

## Industry/control path after system identification

The next stage is a controller benchmark, not another predictor-only leaderboard. Compare tuned PID/INDI/LQR where assumptions apply, nominal MPC/tube-MPC, residual mean compensation, and uncertainty-aware constraint tightening under the same model, actuator and disturbance. Report trajectory tracking error, safety-boundary violations, recovery time, empirical CVaR90/worst run and real-time deadline misses. Only then move through higher-fidelity simulation and PX4 Software-in-the-Loop; Hardware-in-the-Loop should run actual flight-controller firmware against simulated vehicle physics before any controlled physical flight.

Relevant references:

- IDSIA public dataset and model/evaluation scripts: https://github.com/idsia-robotics/nanodrone-sysid-benchmark
- Published benchmark: https://doi.org/10.1016/j.conengprac.2026.106871
- Official PX4 HITL/SITL distinction: https://docs.px4.io/main/en/simulation/hitl.html


## Paired model-comparison receipt v0.1

The metric receipt computes `ground_truth_window_sha256` per run when complete `*_true_h1` through `*_true_h50` columns are present. It separately computes `conditioning_window_sha256` only when the full conditioning schema is present. The conditioning schema includes `x0_t`, every `x0_*` state channel, timestamps `input_time_t0..t49`, and the four motor-speed channels for all 50 input steps (`m1_rads_input_t0..t49` through `m4_rads_input_t0..t49`). The joiner derives `x0` from raw row i, appends timestamps and the full motor sequence from rows i..i+49, and joins future target h to raw state row i+h. This matches the released `QuadDataset` contract (initial state plus a 50-step motor-input sequence); a single scalar motor sample is not enough to certify equivalent conditioning.

Both digests domain-separate their formats and hash row counts and parsed IEEE-754 `f64` bits in little-endian order in fixed field order; the target digest covers timestamps and every future target h=1..50, while the conditioning digest covers `x0_t`, the initial state, every motor-sample timestamp, and every motor input in the sequence. Numeric hashing makes `1` and `1.0` match while any changed target or input invalidates pairing. Orientation representation is part of the pairing contract: quaternion receipts and rotation-vector receipts are not silently mixed. Both digests are unkeyed reproducibility identifiers, not signatures; the comparator proves receipt-to-receipt equality of the encoded targets and conditioning values, but does not authenticate how models were trained or normalized. Preserve source prediction CSVs, raw data, and join receipts for independent review. The digest domain-separates this format and hashes (1) the row count, (2) each row timestamp, and (3) every target field for horizons 1–50 in fixed field/horizon order, using parsed IEEE-754 `f64` bits in little-endian order. This deliberately hashes numeric values rather than CSV text spellings, so `1` and `1.0` pair identically while a changed timestamp or target invalidates the pairing. Orientation representation is part of the pairing contract: quaternion receipts and rotation-vector receipts are not silently mixed. The digest is an unkeyed reproducibility identifier, not a signature; the comparator establishes receipt-to-receipt target identity, but does not authenticate how either model was trained or normalized. Preserve the source prediction CSVs and join receipts for independent review.

For each model output, first join it to the exact same raw Melon run files so the receipt can bind the model's full input sequence as well as its targets. Example for the released LSTM output (use a fresh output directory):

```bash
cargo run --manifest-path integration/symthaea-physics-loop/Cargo.toml \
  --example nanodrone_truth_window_join -- \
  "$NANODRONE_PATH/out/predictions/lstm_random_square_chirp_model_multistep/melon_multistep.csv" \
  "$NANODRONE_PATH/data/test/melon_20251017_run1.csv" \
  "$NANODRONE_PATH/data/test/melon_20251017_run2.csv" \
  "$NANODRONE_PATH/data/test/melon_20251017_run3.csv" \
  results/lstm-joined --pooled --state-units-si

cargo run --manifest-path integration/symthaea-physics-loop/Cargo.toml \
  --example nanodrone_metric_receipt -- \
  results/lstm-joined/melon_20251017_run1.csv \
  results/lstm-joined/melon_20251017_run2.csv \
  results/lstm-joined/melon_20251017_run3.csv \
  > results/lstm-metric-receipt.json
```

Repeat with the candidate model's pooled output and a different fresh join directory, then provide the two JSON receipts to the comparator below. Confirm the predictions are denormalized SI before passing `--state-units-si`; the flag records a caller assertion, not an independently proved scaler receipt. Preserve both joined outputs, JSON receipts, raw files, and model/scaler provenance. Do not change the held-out test protocol after looking at the results.

The new Rust example `nanodrone_model_compare` consumes two metric JSON receipts (reference baseline first, candidate second) and a **new** output path:

```bash
cargo run --manifest-path integration/symthaea-physics-loop/Cargo.toml \
  --example nanodrone_model_compare -- \
  results/lstm-metric-receipt.json \
  results/symthaea-metric-receipt.json \
  results/lstm-vs-symthaea-comparison.json
```

It refuses to compare receipts unless both use the same benchmark version, 100 Hz sample period, held-out Melon runs 1–3, 50 horizons, orientation representation, row count, and exact per-run ground-truth and full conditioning-window digests. It requires complete per-horizon target windows; the legacy shifted-truth metric alone is not accepted. The comparison emits all 50 horizons, selected h=1/5/10/20/50 summaries, per-run paired deltas, and separate channels for position, linear velocity, sign-invariant SO(3) orientation, the source-compatible orientation formula, and body angular rate. It does not invent a unit-mixed aggregate score. A negative `relative_error_change_pct` means the candidate has lower error.

**Statistical ceiling:** three Melon runs are three fixed held-out trajectories, not hundreds or thousands of independent trials. Per-timestep windows overlap and must not be treated as independent samples for a confidence interval. This comparison receipt does not claim significance, broad generalization, controller quality, or flight safety. To make a strong SOTA claim, expand evaluation to independently collected trajectories / flight sessions and pre-register the analysis before looking at candidate results.

## Proposed comparison gates (targets, not current results)

- **Parity gate:** first reproduce each official baseline using the same raw trajectory split, state representation, normalizer fit scope, full-window alignment, units, and source metric. Preserve an explicitly labelled source-compatible score alongside the sign-invariant orientation diagnostic.
- **Accuracy gate:** pre-register per-channel results at all 50 horizons, with headline reporting at 1, 5, 10, 20 and 50 steps. A candidate target is at least 10% lower macro error than the strongest reproduced baseline in at least three of the four core physical channels (position, velocity, orientation, angular rate), while avoiding a regression above 5% in any predeclared headline channel/horizon or on any held-out run. These are proposed thresholds, not measured performance.
- **Evidence gate:** after enough independent flights exist, report paired effect sizes and uncertainty clustered at the flight/trajectory level; never bootstrap individual overlapping windows as if they were independent. With only the three released Melon runs, report paired run deltas without calling a win statistically significant.
- **Systems gate:** separately report inference p50/p95/p99, peak memory, model size, and deadline-miss rate on a declared target device. An accuracy gain is not a practical win if it misses the control deadline or exceeds the memory envelope.
- **Transfer gate:** evaluate unseen trajectories, actuator lag, battery-voltage variation, payload changes, and sensor/noise shifts only after the held-out benchmark protocol is frozen. Report each shift as a separate condition and include worst-condition results; do not average away safety-relevant failures.
- **Safety gate:** prediction error is not closed-loop safety evidence. Before physical use, evaluate constraint violations, tracking error, recovery time, loss-of-control events, and predeclared tail-risk limits in simulation and hardware-in-the-loop; physical testing requires separate risk controls.

Baseline families to reproduce include the benchmark's physics-based, neural, hybrid physics-plus-residual, and LSTM models. For Symthaea, compare persistent HDC/LTC against current-state and explicit-history learned models, plus a reset-each-step ablation. The persistent-state claim only survives if its gains persist on held-out data under matched parameter/runtime budgets.
