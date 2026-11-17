# Human-systems evidence-boundary benchmark runner v0.1

**Status:** executable synthetic software benchmark; not a human model, clinical system, or hostile-code sandbox.  
**Issue:** [HUMAN-EVIDENCE-BOUNDARY-RUNNER-001 / #5730](https://github.com/Luminous-Dynamics/symtropy/issues/5730)  
**Parent:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625)

## Why this is a paradigm shift

A model should be evaluated against the evidence it actually received, not against simulator state that happens to be nearby in memory. This runner makes the first practical separation:

1. the parent generates the synthetic world and retains hidden truth;
2. the parent serializes only the estimator-facing observation events;
3. a separate candidate process receives that payload on standard input;
4. the candidate returns keyed predictions or explicit abstentions;
5. only after the child exits does the evaluator join returned IDs to scorer-only truth.

The parent launches the child with a cleared environment and a temporary working directory. The input contains scenario identity, sensor/channel identity, sensor-clock timestamp, availability time, clock-domain ID, optional measured value, status and duplicate-delivery flag. It does **not** contain the generator seed, sample ordinal, physical event time, latent truth, scorer identity map or alternate world.

This is a process/data-flow boundary, not a security sandbox. It does not prevent a hostile candidate from reading user-owned files, opening network connections, consuming unbounded resources or attacking the host. Do not run untrusted third-party models with this runner as the only isolation. A future external-candidate qualification needs a separately verified gVisor/Firecracker or equivalent boundary, no host mounts/secrets, network denial, resource limits, bounded execution time, and an auditable image/runtime digest.

## Run

From the repository root:

~~~bash
cargo fmt --manifest-path crates/domains/symtropy-human-systems/Cargo.toml -- --check
cargo test --manifest-path crates/domains/symtropy-human-systems/Cargo.toml --locked
cargo clippy --manifest-path crates/domains/symtropy-human-systems/Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path crates/domains/symtropy-human-systems/Cargo.toml --bin human-systems-benchmark
~~~

The command compiles one binary. In parent mode it launches itself twelve times in candidate mode, once per HOBS scenario. Candidate mode is a deliberately simple, deterministic last-observation baseline: it uses a valid observed/quantized reading, carries forward the last valid reading on missing/clipped events, and abstains when it has no prior value. It is an interface smoke-test baseline, not an SOTA estimator.

## Output contract and metrics

The stdout stream is line-oriented for easy capture and hashing:

- HOBS: latent truth-point count, unique target capture points visible to the candidate, capture coverage, predictions, abstentions, response coverage, MAE and RMSE.
- IDENTIFIABILITY: the HOBS-11 candidate stream is passed once and scored against both scorer-only latent worlds.
- RATE: canonical sample-rate, Nyquist, ideal task-band information retention, sampling regime and sample count.
- HIE: time-gated delivery/exposure observations and scorer-only physical-exposure outcome.
- COMPLETE: total candidate subprocess count and explicit human/clinical non-assessment.

Capture coverage is the fraction of latent truth sample indices represented by at least one target-channel event delivered to the candidate. Response coverage is the fraction of those visible target events for which the candidate predicted rather than abstained. MAE/RMSE are calculated only over non-abstained predictions; they must always be read together with both coverage figures. They are not a complete uncertainty/calibration assessment.

HOBS-11 is the core non-identifiability control. One composite observation stream is compatible with two different latent channel assignments. The candidate is invoked once; the evaluator reuses that exact candidate response against each hidden world. A result that fits one world does not demonstrate that the candidate identified which world was real.

The runner reports the canonical sampling-rate retention oracle using the analytic ideal brick-wall reference. It intentionally does not pretend that this is a deployable FIR/polyphase resampler. The intervention lane checks event availability at 60-second and 120-second decision cutoffs; it does not estimate an intervention effect or synthesize a physiological response.

## Comparison with the field

The 2026 WHAR Arena study standardizes a large wearable-HAR comparison across 30 datasets and 17 architectures, and measures on-device latency, memory and model size. It reports a tightly grouped predictive frontier, with small models and classical baselines remaining practically competitive. FlowState (ICML 2026) is a relevant sampling-rate-equivariant time-series comparator. A November 10, 2026 preprint proposes separate frequency-sensitivity, forecasting-degradation and classification-degradation measures for MOMENT, Chronos-2, Moirai-MoE and TimesFM.

This project should not claim to beat those results until their splits, preprocessing, checkpoints, data rights and hardware measurements are reproduced. The intended advantage is a stronger evaluation contract around information availability, observation faults, domain transfer, abstention and non-identifiability—not a claim that the current baseline outperforms published models.

References:
- WHAR Arena: https://arxiv.org/abs/2606.13194
- FlowState (ICML 2026): https://proceedings.mlr.press/v306/graf26a.html
- Sampling-frequency invariance preprint (2026-11-10): https://arxiv.org/abs/2611.14425

## What is qualified here

- An actual estimator subprocess receives a wire payload rather than a Rust reference to parent memory.
- The environment is scrubbed for the child and its working directory is moved to the system temporary directory.
- The candidate response protocol is checked for duplicate IDs, malformed records, unknown abstention reasons and non-finite predictions.
- The scorer validates that every visible target event has exactly one candidate response.
- The observational-equivalence control reuses one candidate invocation against two scorer-only worlds.
- The six canonical sampling-rate retention values are checked against the exact analytic reference.
- HIE availability cutoffs are checked against scorer-only event truth.

## What is not qualified

- A hostile-process sandbox, bounded runtime, resource isolation, or filesystem/network confinement.
- A Symthaea observation/FEP adapter or any production estimator.
- A production resampler or TSFM adapter.
- Reproduction of WHAR Arena, FlowState or the sampling-invariance preprint.
- Real-person wearable, sleep, circadian, physiological, behavioural or clinical validity.
- Causal effects, treatment efficacy, human performance, diagnosis or intervention safety.

## Next research increments

1. Replace the smoke-test baseline with pinned classical probabilistic/state-space estimators and report uncertainty, log score, CRPS, calibration and selective risk.
2. Add the shared Symthaea observation envelope from [Symthaea #12407](https://github.com/Luminous-Dynamics/symthaea/issues/12407) only through an explicit adapter; preserve masks, precision, modality, event time, receive time and late-event semantics.
3. Implement a real FIR/polyphase anti-aliasing lane and compare it with the ideal oracle, Neural CDE/ODE-RNN and FlowState.
4. Reproduce WHAR Arena splits and device measurements; add leave-device, leave-placement and leave-dataset tracks with adaptation regimes reported separately.
5. Add external model processes only after a gVisor/Firecracker-style execution boundary and pinned runtime/model digests are independently qualified.
6. Keep every metric axis visible. Publish Pareto frontiers for error, calibration, abstention, worst-domain performance, latency, memory, energy and adaptation cost; do not hide trade-offs in a single weighted score.
