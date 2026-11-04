# WPT multi-fidelity transient benchmark v1

**Status:** research evaluator with synthetic known-truth data. It has not reproduced any cited paper or measured hardware trace.

## Purpose

Compare reduced-order circuit models and learned WPT predictors on matched transient traces without confusing them with the spatial inductance-matrix model or a frequency-dependent multiport network.

Families are explicitly distinct:
- full-order reference model;
- balanced truncation;
- data-driven Petrov–Galerkin / generalized Lyapunov balancing;
- port-Hamiltonian / structure-preserving reduced model;
- learned surrogate;
- other declared models.

The evaluator consumes time-aligned reference and candidate traces. It reports channel NRMSE, peak-magnitude error and separately integrated Wh for each declared receiver active-power channel. Model and training-data digests are carried into the output.

## Core scoring contract

A case fixes the reference trace, time grid, evidence class, power-flow direction, energy channels and acceptance limits. Each candidate must:
- use exactly the same strictly increasing time grid;
- expose exactly the same channels with exactly matching units;
- declare model family, optional state order, model digest and training-data digest;
- provide finite values for every sample;
- bind to the same execution-environment digest as the full-order reference;
- report median/P95 evaluation runtime and peak memory, measured under a declared repetition protocol.

Energy channels are active power in watts. V1 integrates each receiver independently using the trapezoidal rule and reports signed Wh under the case's declared power-flow direction. It never collapses receivers into a single total before checking individual channel error.

For each channel:

[
\mathrm{NRMSE}=\frac{\sqrt{\frac1N\sum_t(\hat y_t-y_t)^2}}
{\sqrt{\frac1N\sum_t y_t^2}}
]

Peak-magnitude relative error compares the maximum absolute reference and candidate magnitudes. All-zero reference channels are rejected because this NRMSE definition is undefined for them; a future schema may admit a separately defined absolute-error metric for such channels.

## Feasibility-first ranking

The evaluator does not use a weighted sum that allows a good average score to conceal a hard violation.

1. All declared NRMSE, peak-error, energy-error and absolute peak limits must pass.
2. Infeasible candidates rank after feasible candidates, ordered by maximum normalized violation.
3. Feasible candidates are ordered by maximum per-receiver energy error, then maximum channel NRMSE, then measured speedup against the full-order reference, then model state order and candidate ID.

A reference waveform that itself exceeds a declared absolute peak limit makes every candidate infeasible for that case. This prevents an unsafe scenario from being misreported as a model-only failure.

A low waveform error does not establish passivity, stability, causality or safe operation. Those are separate evidence gates (see the inductance-matrix validator and multiport positive-realness work). V1 does not perform frequency resampling, phase alignment, time warping or uncertainty-interval calibration; traces must be sampled on the same frozen time grid.


## Matched runtime and memory

Each case defines the full-order reference's median/P95 runtime and a digest for the execution environment (hardware, OS, compiler/runtime and relevant power/performance settings). Candidate records must use the same environment digest; mismatched hardware is a separate benchmark case, not an apples-to-oranges speedup. Each candidate reports median runtime, P95 runtime and peak memory. Use repeated warm/cold protocols consistently and record the raw run distribution in the source artifact; v1 stores summary statistics.

Runtime is a Pareto dimension, not a substitute for waveform accuracy. The ranking uses it only after feasibility, per-receiver energy error and channel NRMSE. The report includes the exact model and training-data digests so a fast score cannot float free of the model artifact being measured.

## Synthetic known truth

`synthetic-transient-comparison.json` contains a simple two-receiver load-step trace and candidate outputs for:
- an exact full-order reference;
- a four-state balanced-truncation candidate;
- a two-state port-Hamiltonian candidate;
- a learned surrogate with deliberately excessive transient error.

The fixture is entirely synthetic. Its values are software regression inputs, not a physical WPT result or a reconstruction of any publication.

Run:

```sh
cargo test --locked -p wpt-multifidelity-bench
cargo run --locked -p wpt-multifidelity-bench -- benchmarks/wpt-multifidelity/v1/synthetic-transient-comparison.json
```

## Current research baselines

1. Lee & Lee, *Scientific Reports* 16, 19957 (2026), DOI 10.1038/s41598-026-50781-y: heterogeneous fixed-layout 1T–3R electromagnetic/circuit twin, receiver-specific power constraints, mixed series/parallel compensation and steady-state optimization. It is a fixed-coupling multi-receiver design baseline, not a dynamic transient-ROM reference.
2. Ma et al., *Electronics* 14(18), 3668 (2025), DOI 10.3390/electronics14183668: data-driven Petrov–Galerkin projection and generalized Lyapunov balancing, reporting 2–4 state models from experimental data.
3. Zhou et al., *IEEE Transactions on Power Electronics* 41(4), 7053–7066 (2026), DOI 10.1109/TPEL.2025.3624876: balanced-truncation second-order model used for WPT pulse-skipping current-envelope analysis with experimental validation.
4. MLP–PSO WPT control/prediction prototype, *IEEE Transactions on Power Electronics* 40(10), 14196–14209 (2025), DOI 10.1109/TPEL.2025.3573712: reported 430 W / 85 kHz prototype, selected-condition prediction MAPE below 3.5% and maximum efficiency 93.5%. This is not an EV-class or route-efficiency comparison.
5. Samuel et al., IFAC 2012, DOI 10.3182/20120620-3-DK-2025.00119: passivity-preserving parametric model reduction is established prior art. The benchmark therefore does not claim that passive MOR itself is novel.

These are different types of result: a design twin, a dynamic circuit ROM, an experimental controller and a mathematical reduction framework. Their power and efficiency values must not be pooled into one league table.

## Proposed benchmark targets (not achieved)

For held-out transients, including load steps, startup/shutdown, handover, pulse skipping and changing misalignment:
- P95 channel NRMSE <=1%, with maximum <=2% for admitted traces;
- peak voltage/current stress prediction error <=2%;
- each receiver's integrated Wh error <=0.5% P95;
- zero false-feasible classifications against hard current/voltage/thermal/receiver constraints;
- zero passivity/stability false passes, adjudicated by separate verified gates;
- >=10x model-evaluation speedup against the matched full-order simulator, measured on declared hardware with memory and solver costs reported;
- report state order and accuracy–latency Pareto fronts, not a single aggregate score.

These are ambitious research targets, not standards requirements or current achievements. First validate the metric harness on synthetic fixtures, then add source-backed waveforms under their actual evidence class. Do not fabricate unavailable transient data from papers that publish only steady-state values.

## Next extensions

- uncertainty calibration: interval coverage, proper scoring rules and out-of-distribution detection;
- transient event metrics: overshoot, settling time, switching stress and energy dissipated during mode changes;
- separate per-receiver current/voltage/thermal constraint traces;
- bidirectional cases and direction-specific power/Wh sign conventions;
- stable/passive model checks from an independent state-space or port-Hamiltonian verifier;
- matched runtime, memory, compile and solver-call costs;
- frozen geometry/load/temperature/material holdouts and adverse-event denominators.

## Claim ceiling

This tool compares finite trace predictions against a declared reference. It does not verify reference correctness, model provenance signatures, passivity, stability, causality, electromagnetic leakage, foreign-object detection, thermal durability, vehicle-roadway interoperability or standards compliance. No energized hardware test or field deployment is authorized by this benchmark.
