# AER-01 virtual-sensing joint health-state oracle (Tier 0)

This crate is a deliberately small, exact synthetic benchmark for the joint structural-health / sensor-health problem. It is a child implementation for #4176 and composes with placement #4259, structural observability #4244 and measurement-operator contract #4215 / #4236.

**Claim ceiling:** synthetic joint-state hypotheses, witness gating, raw-data preservation and abstention only. It is not an SHM detector, NDE system, aircraft model, POD assessment, airworthiness analysis or flight-safety function.

## Paradigm shift: correction is not diagnosis

A measured anomaly can arise from a structural change, a sensor fault, an environmental/operational change, an invalid measurement chain, or multiple causes. A residual on one channel does not establish which cause occurred.

This oracle applies four hard rules:

1. Evaluate structure and sensor state jointly: HH, HF, DH and DF (healthy/damaged structure × healthy/faulty sensors).
2. A single-channel residual is only a candidate explanation. It does not prove sensor failure. A proposed sensor fault requires an independent, channel-matched health witness with an explicit evidence digest.
3. Multiple plausible explanations, unqualified operators, expired calibration, insufficient observations and out-of-model responses abstain.
4. Raw observations are immutable. An optional model-imputed value is emitted in a separate field with status CONDITIONAL_MODEL_ESTIMATE_NOT_MEASUREMENT. It is never written back as a measurement and is not independent evidence.

This is intentionally more conservative than “reconstruct first, classify later.” A localized defect or unmodeled boundary condition can generate the same one-channel residual pattern as a faulty sensor. Without independent sensor-health evidence, the correct outcome is uncertainty, not automatic correction.

## Frozen synthetic corpus

Three registered channels, two structural templates, a normalized static response and a one-channel residual fault model. The eight cases cover:
- HH: healthy structure / healthy sensors;
- HF: healthy structure / faulty sensor, with independent synthetic built-in-self-test witness;
- DH: damaged structure / healthy sensors;
- DF: damaged structure / faulty sensor, with independent synthetic witness;
- ambiguous structural-damage versus sensor-fault explanations;
- out-of-model environmental/unknown shift;
- insufficient observations;
- a single-channel localized anomaly with no independent sensor-fault witness.

The HF and DF witness digests are synthetic fixture values. They are not records from actual hardware. The operator digests, channel health, structural templates, thresholds and signal values are also synthetic.

## Output contract

Every case report preserves:
- original observed_values exactly as supplied;
- candidate structural state and disposition, or an explicit abstention;
- faulty_sensor_ids only when the channel-matched independent witness is present;
- model_imputed_values separately from observed_values;
- reconstructed_sensor_ids and the literal status CONDITIONAL_MODEL_ESTIMATE_NOT_MEASUREMENT;
- post-correction structural state, which is required to remain the pre-reconstruction structural hypothesis;
- damage_preserved as a synthetic invariant, not physical damage-detection evidence.

For a case with no sensor fault, model_imputed_values is null. For an abstained case, no model imputation is emitted. Unwitnessed single-channel residuals return ABSTAIN_SENSOR_FAULT_UNCONFIRMED. If the healthy/damaged template hypotheses are both consistent, the result is ABSTAIN_AMBIGUOUS_HYPOTHESES.

## Implementation limits

This v0.1 model is deliberately tiny:
- three channels, two fixed structural templates and a static normalized response;
- a single arbitrary channel residual as the only injected sensor-fault pattern;
- a deterministic absolute tolerance, not a learned or calibrated probability;
- synthetic built-in self-test witnesses, not a real diagnostic architecture;
- no temporal response, dynamic transfer functions, covariance estimation, uncertainty propagation, environmental causal model, damage morphology, fatigue, fracture, POD or field calibration.

It does not reproduce either current research frontier:
- Di Fiore et al., *Real-time sensing digital twin for aerospace structural health monitoring* (8 June 2026), combines physics-informed sensing twin, joint sensor/structure classifier, healthy-manifold reconstruction and probabilistic sensor reliability; its demonstration is a numerical composite panel with simulated fault/damage conditions. https://doi.org/10.1007/s00158-026-04324-8
- Chen & Liu, *Causal Bayesian optimization for sensor placement and deconfounding in structural health monitoring* (1 October 2026), uses a structural causal model, do-calculus, counterfactual uncertainty and multi-objective sensor placement, with numerical, experimental and full-scale bridge validation. https://doi.org/10.1016/j.ymssp.2026.114895

The present contribution is not novelty in sensor-fault detection or reconstruction. It is a small, inspectable fail-closed contract that tests independent fault-witness gating, ambiguity abstention, immutability of observations and explicit separation of imputation from measurement.

## Reproducible execution

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/virtual-sensing-health-state/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civil-aero-benchmark/virtual-sensing-health-state/Cargo.toml
cargo run --locked --quiet --manifest-path tools/civil-aero-benchmark/virtual-sensing-health-state/Cargo.toml > /tmp/virtual-sensing-health-state.json
python3 tools/civil-aero-benchmark/virtual-sensing-health-state/verify_independent.py --rust-output /tmp/virtual-sensing-health-state.json
~~~

The independent Python verifier uses only the standard library and does not import the Rust implementation. It recomputes every case and compares the complete JSON report, then runs hostile controls including wrong-channel witnesses, missing/expired channel admission, ambiguous hypotheses, out-of-model signals, insufficient observations and post-reconstruction damage preservation. The exact-head GitHub Actions workflow asserts the PR subject SHA and retains source, fixture, toolchain/environment, output and SHA-256 receipts.

## Advancement ladder

**Tier 0 — this crate:** frozen synthetic HH/HF/DH/DF + abstention; independent fault witness; immutable raw data; independent Rust/Python replay.

**Tier 1 — temporal synthetic:** bias, drift, gain, saturation/clipping, dropout, stuck-at, detachment, timing skew, noise and correlated/common-cause failures. Preserve complex response, phase, transient peaks and spectral features. Add unknown/overlapping faults and missingness during the event of interest.

**Tier 2 — model mismatch and environment:** held-out stiffness, mass, boundary conditions, temperature, humidity, excitation and geometry; environmental-only cases; multiple damage morphologies; counterfactual tests with hidden confounding. Hold out complete specimens/configurations/excitation regimes, not random windows.

**Tier 3 — matched literature baselines:** nearest-neighbour interpolation; modal interpolation; POD/ROM projection; observers/Kalman methods; physics-informed ROM updating; sensing-digital-twin-style joint classification/reliability; causal MO-CBO deconfounding where reproducible. Same data, hidden truth, fault scenarios and total evaluation budget.

**Tier 4 — physical article:** selected representative article, controlled sensor injection and independent NDE/ground-truth damage records; calibrated environmental/operational campaign; blinded operators; independent specimen/configuration holdouts. No POD statement without an appropriate inspection reliability study.

## Metrics that matter

Do not optimize aggregate reconstruction RMSE alone. Report:
- joint HH/HF/DH/DF confusion, per-fault isolation precision/recall and unknown-fault abstention;
- false-clear and false-damage decisions, decision preservation and downstream structural decision regret;
- complex amplitude/phase error, resonance/damping/mode identity preservation and local peak-strain/load preservation;
- uncertainty calibration/coverage and out-of-domain abstention on grouped holdouts;
- rate of unconfirmed fault hypotheses, additional-measurement requests and witness integrity failures;
- compute/update latency and total cost, including instrumentation, calibration, data review, maintenance, extra tests and independent audit.

Suggested research targets after the dataset and independent specimen count are frozen: zero synthetic false-clear; 100% raw measurement immutability; 100% witness-to-channel identity enforcement; 100% independent replay agreement; no silent out-of-domain promotion; and report calibrated false-clear confidence bounds on physical holdouts. These are gates to measure, not claims already achieved.

## Safety and NDE boundary

SHM anomaly scores and virtual reconstructions do not replace qualified inspections. If the result affects structural acceptance or maintenance, the benchmark must retain NDE method/procedure, flaw truth, sample population, operator/equipment variability, hit/miss data, false calls and confidence bounds. Relevant references include MIL-HDBK-1823A (NDE system reliability guidance; guidance, not a contractual requirement) and ASTM E2862-23 (hit/miss POD statistical analysis). They do not certify this implementation.

- MIL-HDBK-1823 status: https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=206122
- ASTM E2862-23: https://store.astm.org/e2862-23.html
