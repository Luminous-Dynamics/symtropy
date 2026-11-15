# Dynamic Junction Mixing T1 Campaign — Pre-registration Template v0.1

**Programme:** WATER-MIXING-DYNAMIC-T1-PREREGISTRATION-001 / [#5287](https://github.com/Luminous-Dynamics/symtropy/issues/5287)  
**Parent operator research:** [#5274](https://github.com/Luminous-Dynamics/symtropy/issues/5274)  
**Mass accounting:** [#5242](https://github.com/Luminous-Dynamics/symtropy/issues/5242)  
**Experiment search:** [#5259](https://github.com/Luminous-Dynamics/symtropy/issues/5259)  
**Evidence class:** protocol template; no experiment authorized or performed

## 1. Entry gate and safety

This template is for a controlled, closed-loop **non-potable** rig only. No tracer is to be injected into a live drinking-water asset. The facility owner/operator and competent safety personnel must approve the tracer identity, compatibility with all wetted materials, concentration range, handling, containment, spill response and disposal route before the campaign is activated.

A facility's ability to pump water and measure pressure is not proof that it can conduct a dissolved-tracer test. The campaign is blocked until outlet concentration measurement, injection metrology, sample-line characterization, sensor calibration, time synchronization and full mass-recovery boundaries are confirmed.

The template deliberately leaves equipment models, tracer choice and numeric operating conditions blank until a real facility, approved test section and calibrated instruments are identified.

## 2. Test article and operating envelope

Record for each test article:

| Field | Required record |
|---|---|
| Test article ID / revision | Immutable identifier and geometry drawing digest |
| Junction family | Tee, cross, double tee, or other declared geometry |
| Port geometry | Internal diameter/area, angle, spacing, connection length and orientation |
| Surface/material | Wetted materials, roughness evidence and assembly revision |
| Operating fluid | Approved non-potable fluid matrix, temperature and measured properties |
| Flow regime | Measured per-port flow, signed direction, Reynolds number and flow uncertainty |
| Scalar regime | Tracer diffusivity evidence and Péclet/Schmidt descriptors where supported |
| Similarity envelope | Declared scale/geometry/flow domain and unsupported dimensions |

Do not infer scalar-mixing similitude from Reynolds number and head-loss similarity alone. The experiment record must include the variables thought to influence scalar transport: flow momentum ratio, flow split, port angle/area ratio, inlet velocity/concentration profile, temperature and tracer diffusivity where available.

## 3. Instrumentation and metrology

The following are minimum functional requirements, not named-product recommendations.

### Inlet measurements

Every active inlet requires synchronized flow and tracer-concentration histories. The actual input to the local operator is the measured mass-flux history:

\[
m_i^{in}(t)=Q_i^{in}(t)C_i^{in}(t)
\]

Record instrument serial/model, range, accuracy, calibration revision, uncertainty, sampling interval, response time and clock mapping. Record the actual injection waveform, not only the pump or injector command.

### Outlet measurements

Every outlet requires its own signed flow and concentration time series. A shared downstream sensor is not a substitute for separate outlet measurements unless a documented collection manifold preserves outlet identity and its transfer function has been characterized.

If a port is not instrumented, mark the observation boundary partial. Never use the model's predicted concentration at that port as a measured value.

### Sensor response and sample line

Before physical mixing trials, independently characterize each concentration channel's response and sample-line delay using a traceable bypass/reference arrangement approved by the facility. Record the response kernel, uncertainty, drift, bandwidth, detection limit, calibration method and validity window. The physical junction kernel must not absorb an uncalibrated sensor lag.

### Timing

Use a common clock or a measured clock-offset/drift map. Target timing uncertainty below 0.1 of the shortest event feature claimed resolved. Set the sampling interval to no more than 0.1 of that feature. If sensor bandwidth or sample timing cannot resolve the minimum delay of interest, do not fit or report a physical memory timescale below that limit.

## 4. Experimental sequence

Run order should be randomized within practical safety and rig-stability constraints. Each operating condition has independent repeats and a unique run ID.

### A. Metrology and baseline

1. Verify geometry and every port's sign convention.
2. Calibrate inlet/outlet flow and concentration measurements.
3. Measure the sensor/sample-line response through the approved bypass.
4. Verify clock synchronization and sample cadence.
5. Establish a conservative-tracer background and blank response.
6. Run a stable hydraulic baseline with no tracer injection.
7. Verify the closed-loop containment and collection/disposal route.

### B. Steady mixing baseline

At fixed geometry and fixed hydraulic condition:
- measure each inlet independently;
- excite one inlet at a time;
- repeat with the other inlet;
- run a declared combined-inlet plateau only after independent excitations;
- record every outlet's plateau concentration and flow.

This stage estimates the steady outlet-resolved transfer matrix P and provides the nested zero-delay comparator for dynamic models.

### C. Fixed-regime transient response

At the same fixed hydraulic condition:
- apply a single-inlet pulse;
- apply a single-inlet step;
- apply independent pulses to the remaining inlet(s);
- use additional time-coded or orthogonal sequences only when their actual measured mass-flux records remain independently identifiable;
- continue observations until the declared tail criterion is met.

For each run, retain the raw inlet/output time series through full recovery. A nominally complete run with a detected tail beyond the observation window is partial.

### D. Hydraulic regime changes

Only after fixed-regime repeatability is established:
- run pre-registered flow-split/momentum-ratio conditions;
- apply a controlled flow step in a separate repeatable run;
- repeat the same transient sequence in each regime;
- test reversal only if every port has signed bidirectional flow observation and the local operator's orientation convention is preregistered.

Unobserved flow direction means abstention, not a fitted average.

## 5. Preregistered model families

All models receive the same measured hydraulic boundaries and the same input tracer mass-flux histories.

1. EPANET complete instantaneous junction mixing embedded in its existing Lagrangian pipe-parcel transport.
2. Steady outlet-resolved transfer matrix P.
3. Fixed-regime causal outlet-resolved kernel h(j,i,tau;x).
4. Regime-conditioned/time-varying local operator, only if the regime-transition data are sufficient to identify it.
5. Geometry-specific transient CFD or published reduced-order regression, where exact inputs and implementation are available.

Do not replace the network pipe-parcel model when comparing only the local junction closure. Do not add chemical reactions in this campaign; qualify conservative-tracer transport first.

A dynamic operator is selected only when it improves pre-registered blind transient prediction by a material margin after accounting for parameter count, calibration effort, instrument requirements and runtime. If steady P is adequate, prefer it.

## 6. Holdouts and anti-leakage

Assign holdouts before observing tracer outputs:

- **Run holdout:** entire independent repeat runs, never random time rows from one trace.
- **Excitation holdout:** complete pulse/step schedule families withheld.
- **Geometry holdout:** one complete junction geometry family withheld.
- **Regime holdout:** one complete flow-split/momentum-ratio regime withheld.
- **Facility/scale holdout:** use when genuinely comparable independent data exist.

Calibration and holdout raw-data digests must be separately recorded. Do not tune parameters, time alignment, model order or kernel support on blind traces.

## 7. Mass accounting and data custody

For every run, record:
- injected mass and uncertainty;
- integrated mass flux through each inlet;
- integrated mass at every measured outlet;
- delayed-tail recovery as a separate stream;
- independently measured physical inventory, if applicable;
- sample-line and instrument flushes as measurement-system streams, never relabelled as physical tank inventory;
- unresolved mass residual and combined uncertainty;
- physical and temporal boundary declaration;
- immutable raw-data, calibration, geometry and operating-state digests.

Use the signed residual:

\[
M_{res}=M_{injected}-\sum_jM_{outlet,j}-M_{delayed}-M_{known\ inventory}
\]

Keep the sign. Do not normalize outlet responses or assign the residual to an assumed mechanism. If an outlet is missing or the temporal tail is incomplete, record PARTIAL_BOUNDARY. If calibration/provenance is insufficient, record INDETERMINATE.

## 8. Metrics and acceptance criteria

Report per outlet, geometry, operating regime and run:

- transient concentration-curve error;
- peak concentration and peak-time error;
- first arrival and 10/50/90% arrival quantiles;
- integrated outlet mass-flux error;
- impulse-response area and tail-mass fraction;
- signed mass residual and uncertainty;
- predictive interval coverage and interval width;
- out-of-domain and abstention rates;
- calibration effort, sensor count, test duration, cost and compute time.

Proposed research targets (not achieved):
- nominal 90% predictive interval coverage on blind physical tests, by stratum, with widths published;
- zero false-safe operator use outside the validated domain;
- no false closure when outlet/time boundaries are partial;
- a preregistered material transient-prediction improvement before selecting a dynamic operator;
- reproducible evidence receipts with raw-data custody and independent review.

## 9. Run manifest schema

Each run record should minimally bind:

- run_id, preregistration revision and protocol digest;
- geometry/test-article revision and digest;
- inlet/outlet instrument identities, calibration digests and clock mapping;
- actual measured inlet mass-flux time series;
- outlet flow/concentration time series and sample-line response;
- tracer, temperature and fluid property records;
- hydraulic operating state and regime descriptors;
- temporal and physical boundary declarations;
- injected, recovered, delayed, inventory and unresolved mass values;
- raw-data digests and custody records;
- holdout assignment and model-evaluation revision.

No result should be accepted if its provenance graph has missing raw data, calibration or test article identity.

## 10. E-NET candidate host boundary

Politecnico di Milano's E-NET is a candidate for a hydraulic testbed collaboration. Published sources describe its network, SCADA, node pressure sensors and dual metering; they do not confirm a dissolved-tracer concentration channel on every junction outlet, tracer injection hardware, sample-line characterization or complete mass-recovery capability. Any proposal must scope a modular tracer measurement section and confirm access, operator permission and safety controls before feasibility is claimed.

## 11. Evidence ceiling

This document is a pre-registration template, not permission to conduct the campaign. No physical trial has been performed. No result establishes potability, contamination, toxicology, SANS 241 compliance or release authority.
