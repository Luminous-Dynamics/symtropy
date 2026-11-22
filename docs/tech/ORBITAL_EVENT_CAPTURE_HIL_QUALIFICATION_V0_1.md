# Orbital Event-Capture Hardware-in-the-Loop Qualification v0.1

**Status:** proposed test protocol; no equipment, campaign, calibration or physical result is asserted to exist.  
**Owners:** [#6659 — observation-channel completeness](https://github.com/Luminous-Dynamics/symtropy/issues/6659), [#6256 — uncrewed orbital-service ground test cell](https://github.com/Luminous-Dynamics/symtropy/issues/6256).  
**Consumes:** #3987 metrology architecture, #5894 instrument/calibration freeze, #6266 synchronized dynamic measurement, #5062 covariance-aware uncertainty, #6140 benchmark contract and #6598 typed assurance bundle.  
**Software reference:** [draft #6676 — event-capture observability evaluator](https://github.com/Luminous-Dynamics/symtropy/pull/6676).  
**Claim ceiling:** bounded event-capture performance for the exact tested sensor / firmware / acquisition / environment / procedure configuration. No flight, mission reliability, universal losslessness or operational authority claim.

## 1. Purpose

Reliability analysis cannot infer whether the measurement system recorded every physical event merely because the retained event history is replayable or fits a statistical model. An upstream sensor threshold, analogue front-end, ADC, firmware queue, DMA ring, packet transport, storage writer, clock or parser can lose or distort events before analysis.

This protocol turns the synthetic observation-channel contract in #6659 into a staged hardware-in-the-loop (HIL) campaign in the existing uncrewed ground test cell. It qualifies the observation path independently from the mechanical, fluid, structural and reliability-model acceptance decisions that consume its evidence.

A strict campaign-profile contract is provided at [orbital-event-capture-hil-profile-v0.1.schema.json](../../schemas/tech/orbital-event-capture-hil-profile-v0.1.schema.json). It binds DUT/reference paths, calibration and common-cause declarations, event definition, qualification envelope, per-stratum sampling plan, multiplicity allocation and acceptance criteria. The schema hard-codes that a profile is proposed planning evidence only; it cannot assert that a physical test, approval or flight qualification occurred.

The target chain is:

\[
\text{physical stimulus}
\rightarrow \text{measurand}
\rightarrow \text{sensor}
\rightarrow \text{analogue front end}
\rightarrow \text{ADC}
\rightarrow \text{firmware/DAQ}
\rightarrow \text{transport}
\rightarrow \text{storage}
\rightarrow \text{event admission}
\rightarrow \text{independent replay}.
\]

Every segment is in scope for the integrated physical campaign. A result on one segment cannot be promoted to the whole chain.

## 2. Two non-substitutable qualification campaigns

### A. Electronic injection — DAQ and persistence path

Inject known calibrated electrical waveforms at the declared sensor-output / DAQ input boundary. Verify analogue conditioning, anti-aliasing, ADC sampling, firmware queues, DMA, packetization, transport, storage, event parsing and replay.

This campaign can establish a bounded result for the downstream acquisition path only from the injection boundary onward. It does not qualify the physical transducer, mechanical coupling, mounting, measurand sensitivity, sensor saturation, physical threshold behaviour or installation.

Requirements:
- characterize the injection waveform at the actual injection boundary using a calibrated independent reference instrument;
- freeze the exact connector, termination, impedance, gain, range, firmware and acquisition configuration;
- record both the commanded injection and the independently measured waveform actually present at the boundary;
- include signal levels below, at and above the predeclared detection boundary, plus overload/saturation cases;
- preserve raw injected and acquired waveforms and all hardware counters;
- do not let the software test harness fabricate expected DAQ records or backfill missing hardware samples.

### B. Physical stimulus — end-to-end sensor and observation path

Apply a controlled physical stimulus to the actual coupon / fixture / sensor mounting configuration. Independently measure the physical measurand using reference metrology that is not the device under test (DUT) sensor and does not share an unresolved acquisition failure domain.

Candidate stimulus families, selected only when applicable to the #6256 test-cell article:
- calibrated force / impulse / vibration input to the capture coupon;
- controlled displacement / pose step or ramp;
- latch / retention engagement and release state transitions;
- bounded structural coupon load / strain response;
- benign-fluid loop state changes such as mass-flow, pressure or temperature transitions, only after the relevant physical subsystem and safety reviews.

The physical reference must demonstrate that the intended event actually occurred and characterize its magnitude, time and uncertainty. A software command, actuator position request, motor-step counter or robot state flag is not independent proof that the physical event occurred.

This campaign qualifies only the tested sensor and mounting, measurand, stimulus method, hardware/firmware/acquisition revision, operating mode and environment. It cannot inherit a pass from the electronic-injection campaign.

## 3. Independent reference architecture

For each measurand, define a reference path with its own instrument, calibration record, acquisition configuration, raw-data custody and declared failure domains.

A reference plan must explicitly identify:
- reference instrument serial and model;
- calibration certificate and validity/scope;
- range, resolution, bandwidth, uncertainty and response time;
- sensor mounting and physical coupling;
- reference acquisition device, firmware/software and storage;
- clock source, monotonicity and measured time-alignment uncertainty;
- power supply, cabling, signal conditioning and shared infrastructure;
- failure domains shared with the DUT path.

Draw a common-cause graph before the campaign. The evaluator must not infer independence merely because two instruments have different serial numbers. Shared power, clock, sensor mounting, amplifier, trigger, cable route, DAQ, firmware, host, network or storage can create common-cause loss.

Where a common trigger or timing marker is required for alignment, identify it as a shared dependency and bound its role. Do not use the same DAQ and clock as both the only physical reference and the DUT capture path.

The reference path must verify actual physical excitation, not only the stimulus generator's requested command. For electronic injection, the independent reference must characterize the waveform at the injection boundary. For physical stimulation, independent reference metrology must measure the physical response at the declared measurand.

## 4. Freeze the intended-use envelope before testing

The test owner must preregister a versioned qualification envelope. At minimum it binds:

| Dimension | Frozen declaration |
|---|---|
| Article / fixture | serial, coupon, mounting, datum and revision |
| Sensor | manufacturer, model, serial, range, bandwidth, calibration and mounting |
| Acquisition | analogue front-end, gain, filter, ADC, sample rate, bit depth and channel map |
| Software | firmware, drivers, parser, event detector and configuration digests |
| Event definition | measurand, threshold, polarity, duration and event identity rule |
| Time | exposure clock, timestamp source, synchronization method and allowed uncertainty |
| Rate | minimum / nominal / maximum event rate and minimum separation |
| Environment | temperature, humidity, vibration, EMI and other applicable declared conditions |
| Operating mode | power, actuator, fluid / mechanical state and concurrent loads |
| Data path | buffer sizes, transport, storage, rotation and retention configuration |
| Acceptance | detection, false-positive, timing, payload and uncertainty limits |
| Analysis | preregistered strata, replicate counts, multiplicity method and confidence level |

The test rate, event amplitude, minimum pulse width and allowable timing error must come from the sensor/DAQ requirements, physical use case, manufacturer limits and uncertainty budget. This document intentionally specifies no universal force, pressure, temperature, sample rate or pulse threshold.

A firmware, sensor, gain, filter, ADC range, cable, mounting, calibration, parser, clock, host, storage or procedure change invalidates the evidence only to the extent defined by the approved change-impact analysis. Affected strata must be requalified; unrelated evidence may remain current only if the scope and dependency analysis justify it.

## 5. Stratified test design

Do not report one aggregate detection percentage that hides a weak event class. Freeze the test strata and randomization before observing results.

### 5.1 Detection-envelope factors

For each applicable event class, cover the declared range of:
- stimulus amplitude / signal-to-noise ratio;
- event duration / pulse width / rise and fall time;
- event rate and inter-event separation;
- polarity and waveform family;
- overlapping / coincident events;
- baseline and drift;
- noise and interference;
- temperature and other approved environmental corners;
- sensor orientation / mounting / coupling;
- concurrent channels and maximum declared DAQ load;
- exact firmware, acquisition configuration and operating mode.

Use boundary points near the claimed detection limit and operating-envelope extremes. A nominal-only demonstration does not qualify the envelope.

### 5.2 Positive controls

Positive controls independently verify that known, in-envelope stimuli occurred and should be detected. They include nominal, threshold-edge, high-rate, short-duration, overlap and declared environmental cases.

A positive control is valid only if the reference path measured an event meeting the preregistered physical / electrical event definition. Invalid reference measurements are INDETERMINATE, not DUT misses or passes.

### 5.3 Negative controls

Negative controls have no qualifying physical event, or are below the preregistered event definition. They measure false positives and spurious event generation.

Include:
- blank windows with no stimulus;
- sub-threshold stimuli;
- known electrical / mechanical disturbances outside the declared event class;
- static / steady-state intervals;
- invalid or disconnected sensor;
- saturation, clipping or range-change states;
- duplicate or replayed packet;
- corrupt checksum / payload;
- stale timestamp and clock discontinuity;
- reset, queue overflow and storage interruption;
- parser / event-detector misclassification.

The independent reference must verify that a negative control did not contain an event meeting the event definition. A negative-control result is not a pass if the reference path itself is invalid.

### 5.4 Fault-injection controls

Where safe and supported by the equipment, inject faults at individual acquisition boundaries: queue saturation, packet loss, delayed writer, storage full, sensor disconnect, reset, clock step, timestamp wrap, firmware restart and checksum corruption.

The aim is to show that the test cell reports incomplete, disputed or indeterminate evidence rather than silently converting faults into zero events or a successful capture result.

Do not induce unsafe faults into pressure, electrical, motion or stored-energy hardware. Fault injection is restricted to a reviewed low-energy/isolated configuration and approved test method.

## 6. Event matching and timing semantics

The independent reference stream defines which qualifying physical events occurred. The DUT stream records which events it captured.

For each reference event, record:
- immutable event ID;
- physical/electrical reference onset and end;
- reference measurand peak, impulse/area or other relevant descriptor;
- reference uncertainty and time-alignment uncertainty;
- event class and stratum;
- expected detection disposition.

For each DUT record, preserve:
- unique record ID;
- referenced event ID where a one-to-one match exists;
- raw payload digest and raw byte location;
- DUT timestamp and measured timestamp uncertainty;
- parsed event class and measured amplitude/duration;
- channel and configuration identity.

The adjudication policy must be frozen before trials:
- one reference event captured once = one matched event;
- no DUT record for a qualifying reference event = missed event;
- multiple DUT records for one reference event = duplicate capture;
- DUT record with no matching reference = unexpected event;
- mismatched payload/class/time beyond the declared bound = disputed event;
- missing reference, invalid calibration, broken clock or incomplete counters = indeterminate;
- event outside declared envelope = out-of-envelope, not an envelope pass.

For overlapping events, define the reference identity and detection rule before testing. Do not force one-to-one matching if the physical event definition is a burst or compound event. A cluster-level adjudication rule must be preregistered and represented in the receipt.

Clock tolerance must derive from measured reference and DUT timing uncertainty plus the approved event-matching rule. It must not be chosen after inspecting observed timestamp errors.

## 7. Loss counters and independent cross-reconciliation

Record all available hardware and software counters before and after each run, and at every reset or acquisition epoch:
- analogue over-range / saturation;
- ADC overrun / sample-clock discontinuity;
- DMA / ring-buffer overflow;
- queue and packet drop;
- transport retry / sequence gap;
- checksum failure;
- reset / watchdog / brownout;
- writer error / storage full / file rotation;
- parser rejection / malformed record;
- time synchronization loss / clock step.

A missing required counter is not zero. A counter that resets or wraps must have its reset epoch and hardware semantics recorded.

Cross-reconcile:
1. independently generated/reference events;
2. sampled raw waveform or raw sensor data;
3. hardware sample / buffer / packet counters;
4. packet and persisted byte records;
5. parsed event stream;
6. final evidence receipt.

All source bytes remain immutable. Each transformation is a separately versioned derivation. A successful software replay proves only that the retained bytes replay consistently; it does not prove that the physical stimulus or sensor event was captured.

## 8. Statistical acceptance and uncertainty

Report results per preregistered stratum, not only pooled totals.

For k missed qualifying events among N independently and representatively injected trials, compute the one-sided exact Clopper–Pearson upper confidence bound on miss probability q by solving:

\[
P_{q_U}(X\le k)=\alpha,\qquad X\sim\mathrm{Binomial}(N,q_U).
\]

For zero misses at one-sided 95% confidence:

\[
q_U=1-0.05^{1/N}.
\]

Illustrative zero-miss sample sizes in one fixed, representative regime:
- 299 trials: upper miss-probability bound below 1%;
- 2,995 trials: below 0.1%;
- 29,956 trials: below 0.01%.

These numbers are not universal qualification requirements. They assume iid Bernoulli trials within one unchanged detection regime. They do not handle burst-correlated loss, dependence across repeated pulses, shared hardware common causes, time-varying detection probability or selection of successful runs.

Required statistical safeguards:
- test amplitude/rate/environment strata separately;
- define what constitutes an independent trial / run / article;
- count bursts or acquisition epochs as clusters where appropriate;
- use simultaneous confidence control (for example a preregistered Bonferroni allocation) when claiming across multiple strata;
- record both misses and false-positive events;
- retain all aborts, invalids, resets and excluded data with reason codes;
- do not pool strata to hide a weak regime;
- keep test thresholds and detector parameters frozen on a calibration set, then evaluate on an untouched acceptance set;
- compare linearized measurement uncertainty with Monte Carlo propagation where the measurement model is nonlinear or non-Gaussian, using the canonical #5062 owner output.

A bounded miss-probability result is conditional on the event definition, reference capability, sampled strata, article/configuration, environmental range and independence assumptions. It is not universal losslessness.

## 9. Benchmarks and report metrics

Each physical-capture report must include:

| Metric | Required reporting |
|---|---|
| Detection probability | per-stratum point estimate and exact interval |
| Miss probability | per-stratum upper confidence bound |
| False-positive rate | per blank / negative-control exposure and interval |
| Timing | onset error, end error, jitter and reference uncertainty |
| Amplitude | bias, repeatability, resolution, saturation and uncertainty |
| Duration | measured width error and missed short-pulse boundary |
| Rate capability | detection vs event rate / inter-event separation |
| Burst / overlap | per-event and per-cluster miss disposition |
| DAQ integrity | overrun, overflow, drop, reset, gap and checksum counts |
| Persistence | raw byte retention, digest and end-to-end reconciliation |
| Replay | independent exact-input reproducibility |
| Scope | sensor, firmware, config, calibration, environment and procedure exactness |
| Cost | test hours, setup/rework, facility time and cost per valid stratum |

Proposed acceptance targets (not achieved):
- zero false acceptance on frozen negative controls;
- 100% independent-oracle agreement on the declared software corpus;
- every required counter present and every positive loss counter blocks a clean capture bound;
- 100% exact subject/configuration/procedure binding;
- zero silent record deletion or timestamp backfill;
- zero stale/mismatched calibration acceptance;
- detection and false-positive bounds met in every required stratum;
- no aggregate pass while any critical stratum is incomplete, indeterminate, disputed or below its requirement;
- independent raw-artifact replay reproduces the declared receipt.

A new firmware or sensor configuration needs its own change-impact and regression evidence. Do not inherit a hardware pass solely because the same product family or software version is used.

## 10. HIL campaign stages and exit gates

**H0 — design freeze and safety review.** Freeze the event definition, physical measurand, sensor configuration, reference path, failure-domain graph, calibration scope, acquisition chain, envelope, negative controls, statistical plan and approved safety boundary.

**H1 — electronic boundary qualification.** Use calibrated electrical waveforms at the declared DAQ input boundary. Qualifies only the downstream chain.

**H2 — sensor/transducer characterization.** Map amplitude, duration, frequency/rise-time and range response using an appropriate calibrated physical stimulus and independent reference. Establish the detection envelope and measurement uncertainty.

**H3 — end-to-end physical capture.** Run the frozen acceptance campaign on the exact sensor/mount/DAQ/firmware/procedure setup. Include all positive, negative and fault-injection controls.

**H4 — load, environment and concurrency corners.** Exercise declared simultaneous channel load, event bursts, environmental corners and relevant electrical/mechanical interference. Requalify affected strata only within an approved configuration-change plan.

**H5 — independent replay and typed receipt.** An independent verifier consumes raw reference and DUT artifacts, counters, calibration and scope declarations. The #6676 evaluator may be reused as a synthetic software oracle, but the physical receipt requires additional authenticated metrology evidence and an independently reviewed HIL disposition.

**H6 — composed ground-service evidence.** Feed capture, mechanical, fluid, receiver and structural receipts into the canonical #6221/#6598 composition boundary. One subsystem pass does not promote another subsystem or the whole service cycle.

No stage automatically authorizes the next. P2 inert-fluid operations and any stored-energy hardware remain subject to the #6256 equipment-specific safety review and approval.

## 11. Evidence artifacts and receipt inputs

A HIL campaign shall retain:
- approved test protocol and revision;
- campaign, run, article, coupon and fixture IDs;
- sensor / firmware / acquisition / parser / host / storage digests;
- reference instrument calibration and traceability records;
- DUT instrument calibration and validity records;
- independent reference raw data;
- DUT raw waveforms, packets and persisted files;
- counters and reset epochs;
- clock synchronization and uncertainty evidence;
- event injection schedule and blinded acceptance seed where appropriate;
- frozen stratum matrix and test exclusions;
- event-matching / adjudication transcript;
- statistical analysis, confidence bounds and uncertainty;
- independent verifier output;
- signed review / approval records from responsible authorities.

#6659's current v0.1 corpus/receipt is not yet the physical HIL receipt schema. Treat it as a synthetic software evaluator until a separate physical evidence adapter binds and authenticates these raw measurement artifacts and metrology records.

## 12. Claim taxonomy

Keep evidence states distinct:
- SYNTHETIC_FIXTURE_RECONCILED — software reference/observation semantics pass on frozen synthetic inputs;
- ELECTRONIC_DAQ_BOUNDARY_BOUNDED — electronic injection demonstrates the downstream DAQ/persistence chain for one exact declared envelope;
- SENSOR_CHARACTERIZATION_COMPLETE — sensor response is characterized for the declared measurand and calibration scope;
- PHYSICAL_HIL_CAPTURE_BOUNDED — end-to-end physical event detection has an independent reference and bounded per-stratum performance;
- INDETERMINATE — reference, calibration, clocks, counters, raw artifacts or declared scope insufficient;
- INCOMPLETE — known events missed or acquisition loss observed;
- DISPUTED — event match, payload, clock, reference or custody conflict;
- OUT_OF_ENVELOPE — test conditions outside the declared tested envelope.

No state in this taxonomy means universally lossless, flight-qualified, orbital-proven, reliable for all modes or authorized for operation.

## 13. Ownership and interfaces

- #6659 owns the observation-channel contract, software evaluator and capture-completeness evidence.
- #6256/#6303 own test-cell integration, physical campaign, safety and instrumentation selection.
- #3987/#5894 own metrology and calibration architecture.
- #6266 owns synchronization / dynamic channel timing and uncertainty.
- #5062 owns covariance-aware measurement uncertainty propagation.
- #6540 owns qualification campaign coverage and sequence constraints.
- #6598 owns typed claim composition; it cannot upgrade a synthetic receipt to physical evidence.
- #6617/#6618 consume capture evidence as an independent prerequisite for event-history model admissibility.
- #6636/#6683 own reliability-model GOF and predictive diagnostics; they may not self-certify the completeness of their inputs.

## 14. Claim ceiling

This document is a proposed HIL protocol. It does not assert equipment procurement, lab availability, calibration, safety approval or test execution. A passing campaign supports only the declared measurand, sensor/DAQ/firmware/configuration, calibration, procedure, tested environment, stimulus range, event-rate strata and reference assumptions. It is not a pressure-vessel qualification, spacecraft qualification, orbital demonstration, operational service, reliability guarantee or mission authorization.
