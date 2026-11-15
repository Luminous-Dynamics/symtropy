# Transient Outlet-Resolved Junction Mixing Operator v0.1

**Programme:** WATER-MIXING-DYNAMIC-OPERATOR-001 / [#5274](https://github.com/Luminous-Dynamics/symtropy/issues/5274)  
**Parent qualification:** [WATER-MIXING-FIDELITY-QUAL-001 / #5216](https://github.com/Luminous-Dynamics/symtropy/issues/5216)  
**Design engine:** [#5259](https://github.com/Luminous-Dynamics/symtropy/issues/5259), draft [#5271](https://github.com/Luminous-Dynamics/symtropy/pull/5271)  
**Mass accounting:** [#5242](https://github.com/Luminous-Dynamics/symtropy/issues/5242)  
**Evidence ceiling:** research design contract; no transient physical mixing qualification

## 1. Problem definition

A steady junction transfer matrix describes how inlet tracer mass flux is divided among outlets under a declared geometry and operating regime. It does not automatically describe the time-resolved outlet concentration history when injection, flow split or flow direction changes.

For a conservative passive tracer, the scalar transport problem is linear conditional on a fixed velocity field and boundary state, but the velocity field itself changes with operating state. This suggests a family of local reduced-order operators:

- steady outlet-resolved transfer matrix for steady/quasi-steady conditions;
- causal outlet-resolved impulse-response kernel for a fixed operating regime with resolvable local delay/memory;
- time-varying or state-space local operator when the hydraulic regime changes during the experiment.

This is a model-selection question. The presence of an operator with memory must not be assumed. If the physical memory timescale is negligible relative to the operating event and the sensor resolution, the simpler steady model is preferred.

## 2. The exact boundary of the EPANET baseline

EPANET 2.2 already uses time-dependent Lagrangian pipe-segment transport. It advances parcel segments at water-quality time steps and reorders segments when pipe flow reverses. At a junction, it blends the mass and volume of the incoming parcels to compute one new node concentration, which is then used for outgoing links.

Therefore this programme does not claim that EPANET lacks time-dependent water-quality transport or flow-reversal handling. The research target is the local junction closure: whether the single node concentration is adequate where real geometry produces different transient concentration histories in different outlets.

Reference: https://usepa.github.io/EPANET2.2/3_network_model.html

## 3. Candidate operator family

Let inlet i have conservative tracer mass flux:

\[
m_i^{in}(t)=Q_i^{in}(t)C_i^{in}(t)
\]

and outlet j have:

\[
m_j^{out}(t)=Q_j^{out}(t)C_j^{out}(t)
\]

For a fixed declared geometry and operating regime x, a causal local response kernel may be written:

\[
m_j^{out}(t)=
\sum_i\int_0^\infty h_{ji}(\tau;x)m_i^{in}(t-\tau)\,d\tau
\]

Here h_ji is an outlet-specific response density over delay. For passive scalar transport it must be nonnegative within numerical/measurement tolerance. A physically meaningful kernel also retains the local accumulation/recovery boundary.

### Static model as a nested limiting case

The steady transfer matrix P is a zero-delay limiting case:

\[
h_{ji}(\tau;x)=P_{ji}(x)\delta(\tau)
\]

The dynamic model must contain the static model as a nested limiting case. This is important for fair model selection: added memory is justified only when it predicts held-out measurements better than the simpler steady operator.

### Conservative accounting

For a fully observed inlet pulse, complete outlet measurement and a sufficiently long recovery horizon:

\[
\sum_j\int_0^\infty h_{ji}(\tau;x)\,d\tau=1
\]

for every inlet i only when all tracer mass eventually exits through the declared outlet set and no residual inventory remains.

For a finite or incomplete measurement horizon, the observed kernel may be sub-stochastic. The missing portion remains a delayed-tail, known-inventory or unresolved-residual term under [#5242](https://github.com/Luminous-Dynamics/symtropy/issues/5242). Do not renormalize the measured kernel to unit area.

For changing flow states, investigate a time-varying kernel:

\[
h_{ji}(t,\tau\mid z_{t-\tau:t})
\]

or a compact positive state-space realization. z must contain measured hydraulic state, not an inferred label that silently embeds future observations. This is a separate model family requiring its own identifiability and validation.

## 4. Model hierarchy and refusal conditions

| Model | Supported question | Refusal / limitation |
|---|---|---|
| EPANET complete junction mixing | Does the standard flow-weighted node concentration reproduce the network-scale baseline? | Not evidence of real outlet-wise homogeneity |
| Steady transfer matrix P | Are outlet concentrations different but stable at a fixed geometry/flow condition? | No claim outside the calibrated geometry/flow envelope |
| Fixed-regime causal kernel | Do outlets have resolvable local delay, dispersion or storage response under fixed operating conditions? | Refuse if pulse bandwidth, recovery window or sensor response cannot resolve the kernel |
| Time-varying/state-space kernel | Does the outlet response change with measured hydraulic regime and recent state? | Refuse if cross-regime excitation or history is not identifiable |
| Transient geometry-specific CFD | Does a spatial flow/scalar model predict the measured outlet histories? | CFD is not ground truth; require mesh/time-step/turbulence/scalar-diffusivity sensitivity and independent tracer data |

A more complex dynamic model is not automatically better. If the static P model meets blind error and uncertainty targets, and the dynamic kernel does not produce a material improvement, prefer P.

## 5. Experiment design

A controlled non-potable junction rig should proceed in stages.

### Stage A — steady response

- establish a repeatable geometry and flow condition;
- independently measure every inlet flow and concentration;
- measure every outlet flow and concentration;
- establish a plateau response and outlet-specific steady transfer;
- repeat the same condition to quantify repeatability.

### Stage B — dynamic response at fixed hydraulics

- single-inlet pulse;
- single-inlet step;
- independently excited inlet pulses;
- time-coded excitations only where input codes remain linearly independent after sensor response and sampling;
- retain samples through the complete tail.

### Stage C — regime transitions

- controlled flow step;
- controlled flow split/momentum-ratio change;
- separate repeated runs for each transition;
- reversal only if the rig measures every port direction and all outlet/inlet flows with synchronized sign conventions.

The experiment must record actual inlet mass-flux waveforms, not the nominal injector command. It must characterize sensor response, sample-line delay, clock offset, calibration, detection limit and sampling interval. These are nuisance parameters, not unmodelled physical memory.

## 6. Identifiability and model discrimination

A dynamic kernel can be non-identifiable even when a static transfer matrix is identifiable. Repeating the same pulse many times improves precision but does not recover unexcited temporal modes.

Use the design-search work [#5259](https://github.com/Luminous-Dynamics/symtropy/issues/5259) to select:
- independent inlet excitations;
- pulse width and temporal code;
- sampling cadence and sensor bandwidth;
- operating states;
- outlet observation channels.

Extend the finite-block sufficient statistics to include time-response features, temporal covariance, phase/lag information and kernel parameter sensitivity. Cross-block covariance must remain explicit.

Competing hypotheses should include:
1. instantaneous complete mixing;
2. steady partial mixing P;
3. fixed-regime causal kernel;
4. regime-conditioned/time-varying kernel;
5. transient CFD.

A design that excites only the steady gain may not distinguish a zero-delay model from a delayed kernel with the same integrated gain. Pulse timing and sampling bandwidth are required to identify transient response.

## 7. Benchmarks and metrics

### Outlet-wise response

- peak concentration and peak-time error;
- first-arrival and 10/50/90% arrival quantile error;
- integrated outlet mass-flux error;
- transient curve distance, reported per outlet;
- impulse-response area/recovery by outlet;
- kernel positivity and causal support;
- storage and unresolved residual;
- cross-outlet response separation.

### Model selection

- blind predictive interval coverage and width;
- out-of-domain/abstention rate;
- static-versus-dynamic incremental error improvement;
- temporal feature identifiability and condition;
- calibration effort and number of required physical runs;
- runtime and parameter count.

### Mandatory controls

- two kernels with identical integrated steady P but different impulse responses;
- a static zero-delay limiting case;
- a delay smaller than sensor response time (must be unresolved/abstain, not invented);
- sensor lag deliberately confounded with physical kernel delay;
- truncated tail and missing outlet (mass ledger remains open/partial);
- flow-regime transition outside the declared operator domain;
- flow reversal with missing direction observation;
- repeated but non-independent inlet excitation;
- time-axis and mass-unit scaling metamorphic controls.

## 8. Proposed targets (not achieved)

- No false-safe use outside the validated geometry/flow/history domain.
- Nominal 90% predictive interval coverage on blind transient physical tests, with interval width and per-stratum coverage.
- Dynamic operator improves held-out transient prediction over P by a preregistered material margin at justified runtime/calibration cost.
- If no material improvement is observed, select the steady P model.
- All mass recovery is accounted for or explicitly remains unresolved.
- Report memory timescale estimates only when identifiable under the actual sensor bandwidth and excitation.

## 9. Compatibility and implementation boundary

The local dynamic operator must be inserted at the junction mixing closure while preserving the existing network-scale pipe-parcel transport, hydraulic solver, tank RTD operator and chemical-fate stage. It is not a replacement for EPANET's Lagrangian parcel engine.

It is also not a reactive chemistry model. Conservative tracer characterization precedes any reaction kinetics.

## 10. Evidence ceiling

This document is a research and experiment-design contract. It contains no measured transient operator, physical kernel, qualified sensor, validated CFD result or field measurement. T1 requires a controlled non-potable rig, competent oversight, calibrated/synchronized all-port observations, raw-data custody, tracer compatibility and approved handling/disposal. It does not establish potability, contamination, toxicology, SANS 241 compliance or water-release authority.


## 11. Implemented synthetic T0 increment

The branch contains a discrete causal-kernel evaluator at `tools/water-service-t0/src/bin/dynamic-kernel-t0.rs` and a 14-case frozen corpus/oracle/manifest.

The evaluator computes a discrete causal convolution of inlet tracer mass-flux histories with an outlet-by-inlet kernel. The steady transfer matrix is represented by a zero-delay kernel. A second convolution applies the calibrated sensor impulse response; model-discrimination separation is calculated on the sensor-filtered signals, not ideal pre-instrument signals.

The T0 controls cover:
- zero-delay steady transfer limit;
- distinct outlet delay responses;
- sub-stochastic kernel and open mass residual;
- finite observation window with physical/sensor tail outside the window;
- independently measured inventory;
- uncalibrated sensor response, for which observed mass and residual are null/unknown;
- calibrated sensor lag;
- flow reversal with unobserved direction;
- out-of-domain use;
- negative kernel coefficients;
- equal integrated steady gain but distinct transient response;
- sampling/sensor resolution that cannot resolve the declared minimum delay;
- incomplete outlet observation;
- positive mass/time scale invariance.

The kernel area is constrained to be no greater than one for each inlet. It is never renormalized. A missing portion remains inventory or unresolved mass. The temporal-resolution preflight compares a declared minimum resolvable delay (derived from sample interval, sensor bandwidth/response and clock synchronization) with the minimum physical delay of interest. It also refuses a claimed resolution finer than the sampling interval.

The single model-separation case uses an explicit independent/equal-variance observation-noise contract and a per-sample standard deviation. Its Mahalanobis-like Euclidean signal/noise separation is a synthetic discrimination proxy only; it is not expected information gain or posterior model probability.

T0 is discrete arithmetic, not a water solver. Native Rust tests, rustfmt and Clippy require exact-head execution and are not claimed passed by authoring this artifact.


## 12. Industry and adjacent SOTA comparison

### Direct junction baseline

Wéber et al. (published online 19 September 2026) is the direct current junction comparator: parametric CFD for cross-junction connection angles and flow distributions, followed by a low-cost regression closure validated against literature experiments. Its supporting data/models are available on reasonable request. The dynamic kernel should not claim superiority without acquiring/reproducing that comparator and using paired physical transients. The paper's declared scope is cross junctions and its studied parameter domain, not a universal dynamic operator.

Source: https://doi.org/10.1061/JHEND8.HYENG-14715

### South African field-model comparator

Luyaba, Biyela and Okedia (2026) compare simplified and more detailed EPANET water-distribution models against field measurements from a Gauteng supply area. The paper reports that additional hydraulic detail improved the accuracy for some measured quality determinants, with model differences in the 5–10% range for the reported field case. This is a valuable local benchmark for network-detail versus prediction-cost trade-offs and a reminder that a physically qualified local operator cannot compensate for a badly simplified hydraulic state. It is not an outlet-resolved junction tracer experiment; its supporting data are available from the corresponding author on request.

Source: https://doi.org/10.1080/1573062X.2025.2589866

### Adjacent transient RTD methodology

A 2026 cavity-transfer-mixer study used planar laser-induced fluorescence to measure RTDs and compared one- and two-parameter back-mixing models. It is an adjacent process-mixer methodology reference, not a water-distribution junction comparator. Its lesson for our protocol is to measure transient response directly and compare parsimonious back-mixing/state-space alternatives rather than assume a kernel's shape.

Source: https://doi.org/10.1016/j.ces.2025.122900

A 2026 tracer-aided hydrology study compared complete, partial and no-mixing assumptions. Multiple assumptions reproduced the calibration observables satisfactorily while differing in their ability to reproduce other observed variables. It reinforces the need for discriminating observations and blind observables; it is watershed-scale isotope modelling, not WDN junction evidence.

Source: https://doi.org/10.1002/hyp.70527

## 13. Implemented synthetic T0 increment

The branch contains:
- Rust evaluator: `tools/water-service-t0/src/bin/dynamic-kernel-t0.rs`;
- 14 frozen candidate cases, separate oracle and identity manifest;
- 19 authored Rust unit tests;
- exact-revision workflow for format, tests, Clippy, oracle replay and receipt archival.

The discrete causal kernel uses nonnegative coefficients indexed by outlet, inlet and nonnegative lag. It computes physical outlet mass-flux predictions by discrete convolution. A per-outlet sensor-response kernel is then convolved with the physical prediction. Competing hypotheses are compared only after both have passed through the same calibrated sensor response.

The operator retains the static transfer matrix as its zero-delay limit. Kernel area for each inlet cannot exceed one. An area below one is not normalized and remains measured inventory or unresolved residual. Predicted physical and sensor tails outside the observation horizon make the time boundary partial. If sensor response is uncalibrated, observed mass and signed residual are unknown (null), not zero.

Temporal resolution is preflighted with two declared quantities: minimum resolvable delay (derived from sample interval, sensor bandwidth/response and clock alignment) and minimum physical delay of interest. A declared resolvable delay finer than the sample interval is invalid. If the sensor cannot resolve the physical delay of interest, the evaluator abstains from dynamic-kernel use.

The one model-discrimination fixture requires an explicit independent/equal-variance noise contract and a per-sample uncertainty. Its sensor-filtered signal separation is a deterministic synthetic proxy, not expected information gain or posterior model probability.

**Evidence status:** an independent JavaScript numerical mirror generated and checked the 14-case oracle. Native `cargo test`, `cargo fmt` and `cargo clippy` have not been executed in this environment; the exact-head workflow is configured but is not yet evidenced as passing. No physical transient experiment or solver qualification has been performed.

