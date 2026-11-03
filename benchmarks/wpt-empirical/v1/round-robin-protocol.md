# WPT Round-Robin Benchmark Protocol v0.1 (research draft)

**Protocol ID:** WPT-RR-01  
**Schema:** wpt-test-run-v1  
**Status:** research design; no physical test has been performed under this protocol  
**Tracking:** [WPT-ROUND-ROBIN-001 #3477](https://github.com/Luminous-Dynamics/symtropy/issues/3477)

## 1. Purpose

Create reproducible measurements across independent laboratories. The output is not a leaderboard of unrelated efficiency percentages. It is a traceable evidence package describing a specific system, configuration, operating point, measurement boundary, uncertainty and safety assessment.

Every run must be independently interpretable. Missing meter planes, unreported conditions, unknown stage coverage or absent uncertainty remain explicit and make the result ineligible for strict ranking.

## 2. Non-negotiable measurement principles

1. Measure **real power** for energy accounting. Report apparent power, reactive power and power factor separately wherever AC is present.
2. Define the exact input and output planes before computing efficiency. Include every intervening stage or state exactly which stages are excluded.
3. Synchronize electrical meters and integrate over the same time window. Include startup, shutdown, transitions and auxiliary/cooling energy in service-level outcomes.
4. Record stored electromagnetic energy at the beginning and end of transient windows. Never silently force the energy residual to zero.
5. Separate measurement, author calculation, model output and inference. A simulation is not a measurement.
6. Store raw waveforms, calibration references, analysis code, firmware/controller identifiers and SHA-256 digests.
7. Report forward and reverse power flow separately. Do not average directional efficiencies.
8. Safety, EMC, exposure, thermal and fault-containment conditions are hard constraints, not terms that can be traded for higher efficiency.

## 3. Run identity and configuration freeze

A run is uniquely identified by a run ID and binds to:
- protocol ID and semantic version;
- prototype/system ID, hardware revision and topology;
- controller firmware/software revision and a SHA-256 configuration digest;
- power-flow direction, power class, commanded load, frequency, coil gap, 3D alignment and yaw;
- receiver count, number of active receivers and each receiver's load request;
- battery/emulator voltage, impedance/load profile and ambient/thermal conditions.

Changing topology, firmware, compensation values, coupler, receiver, calibration, meter placement or safety configuration creates a new configuration identity. Do not overwrite an earlier run.

## 4. Required operating matrix

The matrix is stratified by architecture; do not force a 6–30 kW laboratory prototype and a 50–200 kW bus/roadway system onto one nominal-power curve.

### 4.1 Static, unidirectional and bidirectional EV charging

For systems designed to support the relevant power classes:
- test 6, 11 and 30 kW vehicle-side assemblies on the same ground-side equipment;
- measure grid-to-vehicle and vehicle-to-grid separately;
- sweep battery/emulator voltage and load from 10%, 25%, 50%, 75% and 100% of declared rated power;
- measure aligned baseline, then declared X/Y/Z and angular offsets. A ±100 mm X/Y sweep is a proposed common stress case only where the coupler's mechanical design permits it;
- perform load steps, receiver disconnect/reconnect, direction reversal, startup and shutdown;
- record idle/standby and all auxiliary loads.

For 20–70 kW high-power architectures, use their own rated-power and geometric lanes with the same measurement discipline. Published 50 kW and 65 kW results are not assumed to be matched until meter planes, stage coverage and test conditions are verified.

### 4.2 Multi-receiver systems

Test 0, 1, 2 and N active receivers, including:
- equal requests;
- highly unequal requests;
- independent load steps;
- arrival/departure and inactive-channel leakage;
- one receiver losing authorization or communication;
- aggregate source-power limitation and graceful curtailment.

Report per-channel and aggregate real power, integrated energy, efficiency, cross-regulation, channel isolation, unmet demand and fairness.

### 4.3 Power-quality and thermal matrix

At each selected operating point, record:
- primary/inverter/receiver current and voltage waveforms;
- current and voltage THD, harmonic spectrum and measurement bandwidth/window;
- real/apparent/reactive power and power factor;
- switching frequency, ZVS/ZCS state and switching losses where measurable;
- device, coil, ferrite, shield, enclosure and ambient temperatures;
- thermal steady-state criterion and thermal-cycle history.

A THD percentage without measurement location, frequency span, windowing method and load point is not a comparable result.

### 4.4 Dynamic roadway systems

Only where the hardware is designed for dynamic transfer:
- freeze vehicle speed, acceleration, trajectory, coil pitch, energized segments, handover logic and receiver geometry;
- integrate useful vehicle energy over the complete route window;
- record entry/exit, dwell, dead zones, speed-dependent handover, segment availability and non-equipped vehicle behavior;
- report both instantaneous transfer power and integrated energy per distance/time. Do not substitute one for the other.

## 5. Metrology and calibration

Each metering channel must provide:
- unique meter/sensor ID and model;
- calibration certificate or traceable reference, validity start and due date;
- measurement role/plane;
- bandwidth, sample rate, voltage/current range and phase error;
- timestamp synchronization uncertainty;
- voltage/current/power uncertainty and any correction factors;
- calibration state before and after the test where drift is material.

Use a common time base for source, receiver, auxiliary and thermal channels. Document anti-alias filtering and the exact integration window. Record both measured values and uncertainty; do not hide phase error inside a generic meter accuracy figure.

## 6. Energy balance

For an enclosing control volume:

E_source = E_useful + sum(E_loss_i) + delta_W_stored + E_residual

The loss ledger may include grid/front-end conversion, DC link, inverter, resonant compensation, primary/secondary copper, core, shield, coupling, rectifier/regulation, control, communication, cooling and standby. Include only stages that exist in the tested system.

- Preserve signed delta_W_stored and E_residual.
- Identify whether each loss is measured, author-calculated or model-derived.
- Avoid double counting between electrical metering and calorimetry.
- Report a residual fraction and propagated uncertainty.
- Proposed residual target: <=0.5% steady state and <=1.0% for declared dynamic windows. These are research targets, not established achievements or standards requirements.

## 7. Safety and fault containment

Before any physical test, the laboratory must independently establish the applicable legal, standards, facility, high-voltage, RF/EMF, thermal and foreign-object controls. This protocol does not provide authorization to energize equipment.

Test and document:
- unauthorized/non-equipped receiver;
- foreign-object and living-object detection as applicable;
- receiver position uncertainty;
- loss/delay/corruption of communication;
- sensor disagreement and controller reset;
- thermal over-limit and power-stage fault;
- emergency stop, de-energization latency, restart and recovery;
- exposure assessment method/standard edition and spatial/time averaging.

A passing benchmark test is not certification or a substitute for national requirements.

## 8. Cross-laboratory round robin

A future round robin should involve at least three independent laboratories using:
- the same frozen protocol revision and reference configuration;
- shared reference loads or independently characterized equivalent loads;
- traceably calibrated instruments;
- blind sample IDs and a pre-registered analysis plan;
- identical raw-data and metadata requirements.

Report within-lab repeatability, inter-lab bias, between-lab variance and expanded uncertainty separately. Do not pool measurements until systematic differences are investigated.

Proposed research objectives:
- inter-lab expanded uncertainty <=1.0 percentage point for efficiency after method harmonization;
- within-lab standard deviation <=0.5 percentage point at frozen steady-state operating points;
- energy-balance residual <=0.5% steady and <=1.0% dynamic;
- zero unresolved protocol deviations hidden from the run record.

These are proposed objectives and must be revised if instrument capability or test risk analysis shows they are not defensible.

## 9. Machine-readable data package

Each run must include a test-run.schema.json manifest and references to:
- synchronized raw electrical waveforms;
- per-plane processed measurements;
- calibration certificates and meter configuration;
- thermal and field measurement files;
- controller/firmware commit, hardware revision and configuration digest;
- analysis source and exact tool versions;
- test deviations, reviewer notes and qualification disposition.

Every artifact reference carries a SHA-256 digest. Use rights-aware storage; do not redistribute restricted source material or proprietary waveforms without authorization.

## 10. Qualification and comparison rules

- QUALIFIED: all required planes, conditions, calibration, statistics and uncertainty are present and independently reviewed.
- INCOMPLETE: data are missing; report gaps without inferring values.
- NOT_COMPARABLE: the physical quantity, direction, boundary, stage coverage or operating condition differs materially.
- MODEL_ONLY: numerical or analytical output without the required physical measurement.
- REJECTED: integrity, calibration, protocol or safety failure invalidates the run.

Only matched, qualified measurements enter a strict performance ranking. Publish all exclusions and the reason codes. A reported paper result may remain useful as a source claim while being ineligible for the benchmark leaderboard.

## 11. Current implementation boundary

This file and the accompanying schema/template are research artifacts. There is no physical WPT test, three-laboratory campaign, hardware safety validation or standards certification completed under this protocol.

### Frequency, voltage and mechanical interoperability matrix

The full-text paper's Table 1 maps the three vehicle-side assemblies to distinct compatibility envelopes: 6 kW / 380–440 V / 190–210 mm / 85 kHz; 11 kW / 600–700 V / 160–180 mm / 85 kHz; 30 kW / 420–610 V / 270–290 mm / 20 kHz. The ground assembly uses two independently enabled resonant networks, while the grid interface has four parallel low-power modules with adaptive activation.

Use these as the initial class configuration fixtures. Benchmark frequency-network isolation and switching, all battery-voltage endpoints, clearances, module activation counts, reverse-flow operation, misalignment, load transients and safety response. Do not claim that the article's aggregate efficiency ranges are per-class measured points.

## 12. Multi-frequency resonant-network isolation

For a system with multiple selectable resonant networks, record each branch separately. An INACTIVE command is not evidence of zero current: mutual inductance and closed compensation paths may induce circulating current in a nominally inactive network.

Each branch record must include its frequency, commanded state, isolation-switch state, measured RMS/peak current, continuous current rating, RMS voltage and voltage rating, thermal maximum, measurement method and raw-evidence digest.

- For a qualified physical test, every active and inactive branch requires measured current/voltage/temperature evidence.
- For a non-concurrent design, exactly one branch is ACTIVE during steady transfer; all other branches must be INACTIVE or ISOLATED, not UNKNOWN or TRANSITIONING.
- Proposed steady-state inactive-branch current target: <=1% of that branch's continuous RMS current rating. Capture transient peak current separately and enforce component ratings under switching/fault conditions.
- A simulated branch current is a model result, not a physical measurement and cannot qualify the run.
- Record switching energy, delay, contact resistance, switch state, thermal rise and relay failure modes. Test stuck-open, stuck-closed, welded contact, sensor disagreement and control/communication loss.
- If simultaneous multi-frequency operation is intentionally supported, declare it in the system configuration and qualify each active network and all cross-coupled branches independently.

This protocol addition is motivated by the 2026 multi-vehicle BIPT paper's modelled 17 A induced current in an inactive 85 kHz loop and 4 A in an inactive 20 kHz loop. Those values are simulation findings, not measured currents; they motivate the test, not a claim about a finished product.

