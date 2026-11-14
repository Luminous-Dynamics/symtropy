# Conservative-Tracer Mass Accounting — Partial Boundaries and Unresolved Residual v0.1

**Programme:** WATER-MIXING-MASS-ACCOUNTING-QUAL-001 / [#5242](https://github.com/Luminous-Dynamics/symtropy/issues/5242)  
**Parent:** WATER-MIXING-FIDELITY-QUAL-001 / [#5216](https://github.com/Luminous-Dynamics/symtropy/issues/5216)  
**Related design work:** [#5226](https://github.com/Luminous-Dynamics/symtropy/issues/5226) and draft [#5239](https://github.com/Luminous-Dynamics/symtropy/pull/5239)  
**Evidence ceiling:** synthetic accounting arithmetic only

## 1. Why this is separate from a mixing coefficient

A physically meaningful tracer result needs a defined control volume and time horizon. The injected mass, observed outlet mass, delayed recovery, independently measured inventory and unresolved residual cannot be collapsed into one normalized transfer matrix.

For a declared observation boundary:

\[
M_{\mathrm{residual}} =
M_{\mathrm{injected}}
-\sum_j M_{\mathrm{outlet},j}
-M_{\mathrm{delayed}}
-M_{\mathrm{known\ inventory}}
\]

The sign is meaningful and must be preserved. A positive residual is unaccounted injected mass; a negative residual indicates over-recovery, measurement inconsistency or a boundary/model error. Neither may be clamped to zero.

A recovered fraction is computed against injected mass, not against the sum of observed streams:

\[
r_{\mathrm{outlet}} =
\frac{\sum_j M_{\mathrm{outlet},j}+M_{\mathrm{delayed}}}
{M_{\mathrm{injected}}}
\]

\[
r_{\mathrm{accounted}} =
\frac{\sum_j M_{\mathrm{outlet},j}+M_{\mathrm{delayed}}+M_{\mathrm{known\ inventory}}}
{M_{\mathrm{injected}}}
\]

Neither ratio is silently renormalized to one.

## 2. Residual uncertainty

Use a measurement vector:

\[
x=[M_{\mathrm{injected}},M_1,\ldots,M_n,M_{\mathrm{inventory}}]^T
\]

where the inventory term is present only when independently measured and physically classified. For:

\[
c=[1,-1,\ldots,-1]^T
\]

and the declared covariance matrix \(\Sigma\):

\[
u^2(M_{\mathrm{residual}})=c^T\Sigma c
\]

The covariance order is exactly the measurement-vector order. A covariance matrix must be finite, symmetric, dimensionally correct and positive definite for v0.1. This supports correlated measurement errors without silently assuming independence. Future work can support positive-semidefinite covariance with an explicit rank-aware decomposition.

The evaluator reports the signed residual, standard uncertainty, standardized absolute residual and a declared k-sigma interval. The sigma threshold is a synthetic accounting decision rule, not a regulatory or drinking-water limit.

## 3. Observation-boundary states

A closure claim requires all of the following:

- all declared outlets have measured outlet streams;
- the declared temporal observation window is complete, including required delayed-tail capture;
- physical inventory is counted only when independently measured;
- measurement calibration is current and bound to the observations;
- provenance is bound to the exact measurement streams;
- units are compatible and the covariance contract is valid;
- the residual is within the declared combined measurement uncertainty.

Dispositions:

| Disposition | Meaning |
|---|---|
| CLOSED_WITHIN_DECLARED_UNCERTAINTY | Boundary and evidence requirements are met; signed residual lies within the declared uncertainty envelope |
| OPEN_RESIDUAL | Boundary is declared complete, but residual exceeds the envelope |
| PARTIAL_BOUNDARY | One or more outlets or the required temporal window are incomplete |
| INDETERMINATE | Calibration, provenance, classification or evidence is insufficient |
| INVALID_INPUT | Non-physical numeric input or invalid covariance |
| REJECTED_CONTRACT | Identity, dimensions, duplicate stream or declared-boundary contradiction |

The evaluator does not infer why tracer is missing. OPEN_RESIDUAL is not an adsorption, leakage, storage or contamination diagnosis.

## 4. Inventory and measurement-system loss

A physical tank inventory can be counted only when it is independently measured within the declared control volume. A sample-line loss, instrument flushing loss, unmeasured dead volume or unexplained disappearance is not physical tank inventory.

Such terms remain explicit unresolved causes. The evaluator must return INDETERMINATE rather than count a measurement-system artefact as stored mass.

Delayed recovery is represented as a separately identified stream. It is not merged into the nominal test-window outlet mass. A time window is complete only when its declared tail criterion has been satisfied.

## 5. Frozen T0 negative controls

The v0.1 corpus contains 17 cases:

- complete measured boundary with exact mass closure;
- truncated time window with 40% unresolved;
- independently measured physical inventory closing the balance;
- delayed tail recovered as a separate stream;
- missing outlet;
- sample-line loss incorrectly offered as physical inventory;
- correlated measurement covariance;
- open residual above uncertainty;
- residual within combined uncertainty;
- incompatible units;
- uncalibrated measurement;
- unbound provenance;
- positive-scale metamorphic case;
- negative recovery mass;
- duplicate stream identity;
- non-positive-definite covariance;
- declared all-outlets-observed inconsistent with the measured outlet set.

All inputs, covariance, outputs and dispositions are frozen in a known-answer corpus with a separate oracle and identity manifest. The Rust evaluator must recompute the residual and uncertainty; it cannot read expected outputs as inputs.

## 6. T1 laboratory design

T1 should first use a controlled non-potable closed loop with a documented control-volume diagram. A useful rig has a known injection waveform, independently measured inlet mass, calibrated flow meters, synchronized concentration measurements on every outlet, sufficient sampling cadence for the shortest claimed feature, and an extended recovery period that captures the tail.

For a tank, also measure level/volume and relevant remaining inventory. For a junction, preserve each outlet as a separate stream and vary the flow split/momentum ratio only after a repeatable baseline has been established.

Do not put tracer into a potable distribution asset. Tracer selection, compatibility, handling and disposal require competent oversight and an approved risk assessment.

## 7. External testbed opportunity: E-NET

Politecnico di Milano's FluidLab describes E-NET as an experimental water-distribution network based on the Jowitt–Xu benchmark. The facility page lists three inverter-driven pumps, 25–100 mm pipe diameters, 0–7 L/s demand, an approximate 1.1 km total length, and an EPANET input file available for download/contact. It explicitly lists experimental verification and technology testing among the facility goals.

The 2026 E-NET article describes the numerical scaling of the Jowitt–Xu network, with Reynolds-number and head-loss similarity, and reports a 0.38 m head-loss RMSE across 888 simulated conditions. Crucially, the article states that its paper focuses on the numerical implementation/digital counterpart and that the numerical model can subsequently be validated in controlled experiments at the physical E-NET facility. Do not cite that 0.38 m numerical comparison as a measured tracer or physical mixing result.

Candidate T1 path: evaluate E-NET as a host network and request a modular, instrumented junction/tank tracer test section. The facility page does not establish that the needed concentration sensors, injection hardware, tracer compatibility protocol or mass-recovery apparatus are already installed. These must be confirmed with the facility team before a test plan is claimed feasible.

References:
- FluidLab experimental facilities / E-NET: https://www.fluidlab.polimi.it/exp/
- Ferrarese, Fathi & Malavasi (2026), *From Real to Laboratory Scale: Simulation and Scaling Methodology for a New Benchmark Water Distribution Network Model*, DOI: https://doi.org/10.1007/s11269-026-04785-y

## 8. Public numerical benchmark assets

Use public network assets to test integration and domain transfer, but do not confuse simulated hydraulics with physical tracer truth:

- University of Exeter Centre for Water Systems hosts benchmark WDN inputs and associated information across network expansion, layout, operation, design and resilience: https://www.exeter.ac.uk/research/centres/cws/resources/benchmarks/
- DiTEC-WDN is a public dataset of 36,000 simulated hydraulic scenarios across multiple networks and 228 million graph states. It supports network-scale learning and split design, not local physical mixing qualification: https://doi.org/10.1038/s41597-025-06026-0
- EPANET 2.2 provides the reference network solver and documented complete junction mixing plus four tank mixing options: https://usepa.github.io/EPANET2.2/12_analysis_algorithms.html
- The 2026 incomplete-junction-mixing regression paper is an important comparator, but its data availability statement says supporting data/models are available from the corresponding author on reasonable request: https://doi.org/10.1061/JHEND8.HYENG-14715

## 9. Claim boundary

T0 validates only synthetic mass-accounting arithmetic and refusal semantics. It does not validate a sensor, tracer, physical mass boundary, mixing operator, tank, junction, CFD model, chemical fate model, potable system, contamination finding, toxicology, SANS 241 compliance or release authority.

A future T1 result must be tied to actual facility and instrument identities, calibration, sampling plan, raw data, custody, test boundary, operating condition and independent review. A passing T0 oracle is not an entry ticket to inject tracer into a potable asset.
