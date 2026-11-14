# PCC Metrology Assurance Protocol

**Protocol:** GRID-PCC-METROLOGY-001  
**Version:** 0.1  
**Parent issue:** #3752  
**Status:** research contract and synthetic fixtures  
**Claim ceiling:** qualified measurement uncertainty for a pinned installed meter chain, boundary, operating range and evidence period; not grid-service or capacity accreditation.

## 1. Purpose

A flexibility trajectory is only as trustworthy as the measurement system that produced it. A reported reduction of a few megawatts must not receive capacity credit when instrument-transformer ratio/phase error, meter calibration, clock offset, integration-window mismatch, clipping or uncertain background load could explain the observed result.

This protocol adds a metrology qualification layer upstream of response delivery, stateful resource comparison and system adequacy. It reuses existing meter-hierarchy and time-series validation; it does not create a second nested-meter topology.

The output is a corrected measurement and an uncertainty envelope, not merely a number.

## 2. Distinct assurance boundaries

- **Meter hierarchy / accounting (#3435/#3437):** identify parent/child measurement boundaries and prevent nested readings from being added.
- **Metrology assurance (this protocol):** establish calibration traceability, correction functions, uncertainty, timing and dynamic measurement range for the installed chain.
- **Response assurance (#3649/#3652):** establish dispatch latency, ramp, sustained response and repeated delivery from qualified measurements.
- **Workload/physical assurance (#3673/#3675):** establish useful-work and state feasibility independently.
- **Adequacy (#3667/#3687):** consume qualified full-horizon net PCC trajectories.

A correct topology does not imply an accurate meter. A meter accuracy class does not by itself qualify the installed CT/PT, wiring, burden, firmware, timebase, sampling, integration and data pipeline as a whole.

## 3. Metrology references and scope

Use the current applicable normative instruments and jurisdictional requirements selected by the site/operator. Relevant references include:

- JCGM 100:2008 (GUM), with BIPM-listed Amendment 1:2026 addressing nonlinearity in measurement models;
- JCGM 101:2008 for propagation of distributions using Monte Carlo methods;
- IEC/IEEE 61869-21:2025 for uncertainty evaluation in accuracy testing of instrument transformers, including on-site testing;
- IEC 61869-1:2023 and the applicable instrument-transformer product parts for the actual voltage class and technology;
- IEC 62053-22:2020 where the installed meter falls within its specified transformer-operated static active-energy meter/type-test scope.

These references guide the measurement model; this protocol does not claim compliance with any of them. Applicability must be checked against instrument type, voltage level, installation, rated frequency, jurisdiction and normative edition. Do not use a high-voltage instrument-transformer requirement as though it automatically covered an LV product.

## 4. Pinned measurement-chain contract

A receipt must bind, at minimum:

### 4.1 Boundary and topology
- site, PCC, feeder and electrical one-line/topology revision;
- measured quantity (active power, reactive power, energy, voltage/current, frequency);
- import/export sign convention and phase configuration;
- meter location, boundary owner and whether other non-data-centre loads share the PCC;
- parent/child meter relationships reused from the existing hierarchy authority.

### 4.2 Instrument chain
- meter make/model/serial number, firmware, configuration and measurement mode;
- CT/PT or low-power instrument-transformer make/model/serial, ratios, polarity, phase, rated burden, class and wiring;
- sensor technology, secondary signal format, phase mapping, scaling and correction functions;
- calibration certificate/reference device identity, certificate digest, date, expiry, conditions and traceability chain;
- installation verification, polarity/phase checks, burden and wiring checks, environmental operating range.

### 4.3 Sampling and time
- native sample interval, aggregation/integration convention and filter/window behavior;
- clock source, synchronization method, offset estimate, uncertainty, holdover/drift and timestamp semantics;
- acquisition/transport delay, sample sequence and quality flags;
- UTC instants plus explicit time zone/calendar mapping for interval aggregation.

### 4.4 Operating domain
- current, voltage, frequency, power factor and harmonic/distortion envelope;
- temperature, burden and load range during calibration and evaluation;
- clipping, saturation, overrange, meter reset, missing sample and clock-quality indicators.

Any material meter, transformer, wiring, ratio, firmware, clock or sampling change invalidates or narrows the existing receipt until requalified.

## 5. Corrected active-power measurement

Let raw samples be (z), installed-chain calibration parameters be (	heta), operating context be (x), and measurement model be (h). The corrected estimate is:

\[
\hat{P}(t)=h(z(t),\theta,x).
\]

The model must document units, sign, phase combination, scale factors, offset/gain corrections, phase-angle corrections, frequency dependence, nonlinearity, temperature/burden correction, filtering and any clipping/overrange behavior.

Do not assume a universal linear gain/offset model. It is a deterministic known-truth baseline. If calibration curves, phase interaction, saturation or other behavior is nonlinear, retain the model and propagate parameter/input distributions.

The actual PCC active-power channel is authoritative for grid-facing import response. GPU, rack, UPS, cooling and other internal measurements are diagnostic/attribution inputs; they must not be added as if independent when a higher-level meter already includes them.

If the PCC also includes unrelated site loads, report the measured whole-site net response. Do not attribute the whole change to the data centre without a validated background-load attribution model and its uncertainty.

## 6. Paired response and covariance

For matched baseline and candidate measurements:

\[
\hat{R}(t)=\hat{P}_{base}(t)-\hat{P}_{candidate}(t).
\]

For a locally linear model with standard uncertainties (u_b,u_c):

\[
u_R^2=u_b^2+u_c^2-2\operatorname{Cov}(P_b,P_c).
\]

The covariance term is not an optional optimization. Reuse of the same CT/PT, meter, clock, calibration reference and environmental conditions can create common-mode errors that partially cancel in a paired difference. Changes in range, power factor, phase, burden, time window or nonlinear correction may prevent that cancellation.

Never set covariance to a favorable value merely because the same device was used. The receipt must identify shared components, independent components, covariance assumptions and evidence. When covariance cannot be established, use a conservative bound and disclose the resulting qualification uncertainty.

For nonlinear/non-Gaussian inputs, propagate the joint distribution (including covariance) through the actual measurement model, e.g. with a preregistered Monte Carlo method. Report the method, distributions, seed and convergence diagnostics.

## 7. Time alignment and interval energy

Power response is time-dependent. A clock offset can make a baseline and candidate trace refer to different physical intervals; a window-boundary mismatch can manufacture or suppress a short response.

Requirements:

1. align samples to common physical time intervals using the declared clock model;
2. retain raw timestamps and corrected timestamps, offset and offset uncertainty;
3. use the meter's documented averaging/integration semantics;
4. compute energy using actual interval duration and the chosen reconstruction/integration method;
5. propagate time uncertainty through the power trajectory, particularly where power ramps or steps occur;
6. use distribution-based time-offset propagation when the signal is discontinuous or a first-order derivative approximation is invalid;
7. prohibit hidden interpolation, dropped samples, sample duplication or unrecorded resampling;
8. treat missing or ambiguous samples as ineligible/indeterminate unless a separately validated reconstruction model explicitly supports them.

For smooth locally differentiable signals, a first-order timing sensitivity may be expressed as (u_{P,t}\approx |dP/dt|u_t). This is not a universal rule for step changes, threshold events, clipped signals or non-smooth waveforms; those need explicit time-offset propagation.

Energy is an integral of the corrected power trajectory, not an independently invented value. The receipt must state whether samples are instantaneous, interval averages or meter-register increments and must never mix these interpretations.

## 8. Conservative qualification gate

The evaluator emits a signed response estimate and an uncertainty interval/bound for every interval, plus gross reduction, rebound and net energy bounds.

For a product requiring at least (R_{min}) response, the metrology gate must use a declared lower bound:

\[
R_{lower}(t)=\hat{R}(t)-U_R(t)
\]

where (U_R) is derived from the declared coverage method and joint uncertainty model. The exact coverage probability must be stated. A fixed (k=2) expanded-uncertainty shortcut is not appropriate for every distribution, degrees-of-freedom case or nonlinear model.

A trajectory must not pass its measurement gate when the lower bound falls below the contracted response requirement. If the interval includes zero response, the measured value alone cannot establish positive delivery. If uncertainty is unbounded, stale, out-of-range or unsupported, abstain.

This gate only addresses metrology. Response timing, workload, physical state, owner authorization, full recovery and system adequacy remain separate.

## 9. Calibration expiry, drift and requalification

Evidence expires or is narrowed when any of the following occurs:

- calibration expiry or adverse check-standard result;
- meter/CT/PT replacement, firmware or configuration update;
- CT/PT ratio, burden, polarity, wiring, phase or location change;
- clock source, synchronization or sample aggregation change;
- observed operation outside calibration range;
- clipping/saturation, drift, data-quality failure or unexplained energy-balance residual;
- environmental conditions outside the validated model;
- a material change to the meter hierarchy or shared PCC boundary.

A receipt must record the effective qualification window and the exact invalidation/requalification reason. Unknown is not nominal.

## 10. Frozen synthetic known-truth corpus

`benchmarks/ai-data-centre-energy/v1/pcc-metrology-fixtures.json` defines the first cases:

- exact gain/offset correction;
- paired measurements with correlated shared calibration error versus independent error;
- uncertainty bound crossing the minimum response threshold;
- clock offset near a step response;
- missing sample/integration-window mismatch;
- calibration expiry;
- nonlinear correction requiring Monte Carlo propagation;
- shared PCC with unrelated background load;
- nested meter hierarchy non-addition.

Fixture values are synthetic. Their purpose is to test model arithmetic and failure behavior before owner-authorized real meter traces are admitted.

## 11. Proposed acceptance targets

Engineering goals, not standards limits or achieved results:

- 100% exact agreement on deterministic calibration and integration fixtures;
- zero false passes when the conservative lower response bound misses the contracted threshold;
- zero hidden energy change through resampling or integration;
- all accepted data bound to a traceable calibration receipt and declared operating range;
- expanded measurement uncertainty no more than 10% of the minimum contracted response within the qualified domain, or stricter where the product requires;
- timestamp uncertainty no more than 10% of the shortest response-latency tolerance, or stricter where the product requires;
- no independence assumption for shared calibration, clock or transformer errors without evidence;
- explicit abstention for expired, out-of-range, clipped, missing or unsupported measurement states;
- replayable corrected samples, uncertainty and eligibility decision from frozen digests.

## 12. Integration and non-claims

- **#3435/#3437:** reuse meter hierarchy, sample ordering and time-axis rules.
- **#3649/#3652:** response delivery/repeatability must consume qualified PCC measurement receipts.
- **#3673/#3675:** keep workload and physical-state evidence separate; both ultimately meet at the declared PCC trajectory.
- **#3698:** reuse calibration/time evidence for overlapping PCC waveform channels.
- **#3713:** GPU/CPU/DRAM channels remain diagnostics unless independently qualified; the grid-facing boundary must be metrology-qualified.
- **#3667/#3687/#3670:** only metrology-qualified signed PCC trajectories may be offered to adequacy studies.

Passing establishes only a metrology-qualified measurement envelope for the pinned installed chain, boundary, operating range and evidence period. It does not establish workload equivalence, physical feasibility, dispatch reliability, ELCC, transmission deliverability, grid-code conformity or market accreditation.
