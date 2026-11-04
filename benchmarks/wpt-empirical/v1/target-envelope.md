# WPT target envelope and SOTA qualification plan

**Status:** proposed research targets, not achieved results or a published performance claim.

This document translates the empirical envelope into operating-condition-specific objectives. It is deliberately a multi-lane benchmark: static IPT, dynamic IPT, CPT, room-scale relay, optical LPT and Earth–ionosphere propagation are different engineering problems and must not share one scalar leaderboard.

## 1. Current external reference points

| Lane | Source-reported reference | Measurement boundary / qualification limit |
|---|---|---|
| Static EV IPT, bus opportunity charging | Riekerk et al.: 50 kW prototype; 96.824–97.165% DC–DC efficiency range under 100 mm X/Y misalignment; stray-field scalar 13.86 μT | Primary abstract does not specify exact input/output meter planes, frequency, gap, uncertainty, field probe location or whether the 13.86 μT is measured or computed. |
| Static EV IPT, survey row for Riekerk | 50.96 kW, 97.16%, 85 kHz, 160 mm gap, 4,813 cm² TX and RX area | Survey-secondary Appendix A Table A1. Do not silently merge with primary abstract's 50 kW and misalignment range. |
| Static EV IPT, long gap | Zavrel et al.: >70 kW at 200 mm and 96.3% in back-to-back tests; 65 kW control comparison reports 92.7% whole-WCS and 95.5% DC–DC subchain | Back-to-back DC-link efficiency, whole WCS and DC–DC subchain are distinct boundaries. The paper's abstract's "overall" wording requires reconciliation with Table VII. |
| Static IPT industry envelope | 2026 survey of 133 experimental publications (78 IPT, 55 CPT) from 2020–2025 | Empirical range across heterogeneous test boundaries, not a controlled head-to-head benchmark. |
| Dynamic EV IPT roadway | Purdue/INDOT: 50–190 kW at speeds up to 65 mph; INDOT reports ~85% at rated power from roadway-inverter DC bus to vehicle DC bus | DC-bus-to-DC-bus test boundary; do not call this wall-to-battery efficiency. |
| CPT | Survey reports demonstrations exceeding 44 kW | Reported high-power envelope; operating gap, area, efficiency and measurement boundary must be extracted per prototype before ranking. |
| Room-scale MQS relays | Sasatani et al.: >20% passive AC–AC relay-to-receiver link in one configuration; separate ~500 mW DC output from 9 W input (~5% DC–DC) | Passive link and DC source-to-rectified-output are different boundaries. |
| Optical LPT | Yigit et al.: ~99 W, ~12.4% end-to-end over 20 m indoors; 95.96% laser-driver component efficiency | Driver-only result is not end-to-end efficiency. Indoor proof is not outdoor weather/pointing/safety qualification. |
| ELF/VLF propagation | ITU-R P.684-8 (in force) predicts field strength below about 150 kHz using waveguide and wave-hop methods | Earth–ionosphere propagation reference, not bulk-soil conduction or useful-power-transfer prediction. |

Primary / reference sources:
- Riekerk et al., DOI 10.1109/JESTPE.2025.3631233.
- Zavrel et al., DOI 10.1109/OJVT.2025.3604561.
- van Ieperen, Derammelaere and Minnaert, 2026, DOI 10.3390/electronics15081575.
- Moncada et al., Purdue JTRP 2026/18, DOI 10.5703/1288284318633; INDOT dynamic WPT programme reporting.
- IEC/IEEE 63184:2025 for WPT human-exposure assessment methods within its stated 3 kHz–30 MHz, stationary IPT scope; applicable exposure limits and local law are separate.
- ITU-R P.684-8 for sub-150-kHz field-strength prediction in the Earth–ionosphere propagation regime.

## 2. Proposed qualification targets

The following are intentionally difficult targets. Every target requires a matched test plan, pre-registered boundary definition, independent instrumentation, uncertainty statement, and a frozen evidence package. A target is not a result.

| Lane | Proposed target | Conditions that must be held/reportable |
|---|---|---|
| **Static IPT, 50 kW opportunity charging** | DC–DC efficiency ≥97.5% nominal and ≥97.0% at the specified ±100 mm X/Y misalignment envelope | 50 kW, 160 mm, 85 kHz reference geometry; same DC meter planes across designs; load sweep; thermal steady state; test all misalignment corners; uncertainty U95 ≤0.3 percentage points target. The published Riekerk primary range is 96.824–97.165%, with exact meter planes still to be verified. |
| **Static IPT, 65 kW / long gap** | ≥97.0% at the back-to-back DC-link boundary at ≥65 kW / 200 mm; separately ≥94.0% whole-system AC-input-to-battery-DC efficiency | Separate measurement campaigns and records. Do not treat DC-link efficiency as full station efficiency. Characterize gap 150–250 mm and two-axis misalignment, thermal soak and load sweep. |
| **Dynamic IPT roadway** | ≥90% roadway-inverter DC-bus to vehicle DC-bus at ≥100 kW and 65 mph; route energy balance ≥0 (no net battery depletion at declared speed/duty cycle) | Matched 85% Purdue/INDOT reference boundary; include speed, power, gap, vehicle load, weather, segment transitions and traction demand. Proposed reliability objective: ≥99.9% energized-lane availability over a 1,000 h endurance campaign, and zero unauthorized energizations in ≥10,000 non-equipped vehicle passages. |
| **CPT high power** | Demonstrate ≥50 kW while maintaining ≥90% declared DC-to-DC efficiency | Publish electrode area, gap, dielectric/air environment, fringing-field distribution, alignment, real/reactive power, power factor, thermal and exposure data. The ≥44 kW survey demonstration is the current high-power reference, not a claim that this target is already competitive in every condition. |
| **Room-scale multi-receiver MQS** | ≥10% source-DC-to-useful-load-DC efficiency at ≥500 mW per active receiver, with at least three simultaneous receivers | Same geometry, 1.34 MHz baseline configuration, declared receiver positions and loads; publish both per-receiver and aggregate efficiency, scheduling fairness, field maps, availability and thermal state. Do not optimize only the passive AC–AC sublink. |
| **Optical LPT** | ≥20% electrical-input-to-useful-DC-load efficiency at 20 m and ≥100 W delivered | Indoor, optically enclosed laboratory path with engineered interlocks and access controls; declare wavelength, beam aperture, PV conversion, pointing tolerance, air conditions, thermal loads and every conversion stage. Do not use component driver efficiency as the system metric. |
| **Earth–ionosphere propagation models** | On a frozen held-out field-strength corpus, target median absolute prediction error ≤3 dB and calibrated 95% prediction-interval coverage of 90–98% | Compare against ITU-R P.684-8 under identical frequency, path, time, ionospheric and ground assumptions. Report waveguide/modal residuals, day/night/seasonal conditions and path-length dependence. This lane predicts field strength, not useful delivered watts. |
| **Tesla Earth-wave model discrimination** | ≤1% energy-balance residual on canonical analytic/numerical cases; <1% change in selected observables under refinement; ≥95% correct attribution on frozen synthetic known-truth cases | Competing bulk-earth, Earth–ionosphere, local circuit/near-field and radiative/mixed models; separate phase, group and causal-front velocity. Must emit UNRESOLVED when observables are non-identifying. No physical transmission or safety conclusion follows from synthetic validation. |

### Why these targets are meaningful

- Static IPT's ≥97.5% nominal and ≥97.0% misalignment targets would move beyond the reported Riekerk DC–DC range only on a carefully matched subsystem boundary. The long-gap lane separately tests power delivery and whole-chain losses.
- The 65 kW / 200 mm objective improves on the 96.3% Zavrel back-to-back reference at that gap, while the ≥94% whole-system objective is deliberately a separate goal from the 92.7% Table VII whole-WCS reference.
- Dynamic roadway's target is compared only to the Purdue/INDOT DC-bus-to-DC-bus ~85% figure, not to EV stationary wall-to-battery charging.
- A room-scale relay has to improve the complete source-to-load chain, not headline its passive relay efficiency.
- Optical LPT must improve the full laser-to-electric path, not the driver stage alone.
- Earth-wave models must first beat an applicable propagation reference on held-out observations. No power-transfer target is meaningful until a useful load, both electrical ports and a complete energy balance are measured.

## 3. Hard constraints and Pareto objectives

Do not combine these into a weighted scalar score. Maintain a Pareto set with hard feasibility constraints.

**Hard gates (no trade-off permitted):**
- Applicable human-exposure limits and national regulations; evaluate WPT exposure methodology within IEC/IEEE 63184:2025 scope and record where it does not apply.
- Electrical safety, thermal limits, dielectric/insulation margin, EMC and implant/medical-device considerations where relevant.
- No unintended energization; safe state on loss of communication, alignment or vehicle authorization.
- No missing meter planes, unit conversions or uncertainty silently replaced with defaults.
- No claim of compliance based solely on simulated field plots or a single unlocated stray-field scalar.

**Pareto objectives:**
- Useful energy delivered per unit time and per unit coupler/roadway area.
- Source-to-load efficiency and loss partition by stage.
- Power density, pad mass/volume, material cost and lifecycle embodied carbon.
- Misalignment, gap, speed, load and environmental robustness.
- Thermal rise, field exposure margin and EMC emissions.
- Reliability, availability, false activation, maintenance and degradation.
- Deployment economics (installed cost per kW, cost per delivered kWh, roadway civil works).

Report nominal, worst-case and uncertainty-qualified performance. A design that is more efficient only at perfect alignment but fragile to realistic tolerances should not dominate the robust Pareto set.

## 4. Paradigm shifts to pursue

### A. From a coil benchmark to a metrology graph

Every measured quantity binds to a stage or field-probe node in the complete system. Eligibility rules compare only like quantities, units, statistics and boundaries. PR #3469 begins this contract; quantity-to-unit and boundary-equivalence semantics remain to be implemented.

### B. From static alignment optimization to adaptive field-and-load orchestration

Co-design pad geometry, compensation, power electronics, load impedance tracking, segmented energization, vehicle localization and thermal control together. For dynamic roads, energize only authorized occupied segments, predict receiver trajectory, allocate power across adjacent pads and transition without large power ripple.

### C. From one transmitter/one receiver to modular cooperative infrastructure

Study multiple transmitter modules, receiver coils, relay nodes and loads as a controlled network. Optimize routing/scheduling, coupling and fairness under bounded fields and thermal constraints. Do not assume extra relay stages are free: every stage adds conversion loss and must be included in source-to-load efficiency.

### D. From nominal optimization to uncertainty-aware robust Pareto design

Optimize over distributions and bounded uncertainty in coil position, gap, material parameters, soil properties, load state, temperature and manufacturing tolerances. Use interval/chance constraints and worst-case margins; retain multiple Pareto-optimal designs rather than hiding trade-offs in one weighted score.

### E. From solver output to cross-validated digital twins

Couple electromagnetic field models, circuit/power-electronics models, thermal/material models, controls, vehicle dynamics and operational schedules. Use analytic limiting cases, independent solvers, mesh/time-step convergence, calibrated parameters and held-out tests. A simulator is qualified only for its declared domain and observables.

### F. From a Tesla narrative to a falsifiable model contest

Use the patent as a source of hypotheses, not a validation target. Compare bulk-Earth conduction, ordinary ground-return coupling, Earth–ionosphere ELF/VLF modes (including Schumann resonances), and radiative/mixed paths. ITU-R P.684-8 is appropriate to the Earth–ionosphere field-strength lane below about 150 kHz; it does not model a bulk-soil power path. Keep field detectability, receiver voltage and useful load power separate.

## 5. Release gates for the benchmark

1. **Record integrity:** 100% numerical records have quantity kind, raw unit, source locator and evidence kind; missing uncertainty is explicitly marked.
2. **Efficiency boundaries:** 100% efficiency records have named input/output planes, stage coverage, operating mode and statistic—or are blocked from strict ranking.
3. **Unit semantics:** zero cross-dimension comparisons; every normalization is reproducible from raw value + conversion ID + version.
4. **Source lineage:** duplicate papers/prototypes are linked rather than counted twice; primary and survey-secondary facts remain distinct.
5. **Adversarial tests:** zero false comparisons for all frozen negative controls; unknown boundaries return UNRESOLVED.
6. **Independent annotation:** target Cohen's κ / equivalent agreement ≥0.90 for quantity kind, evidence type, statistic and boundary on a dual-annotated sample; retain disagreement records.
7. **Uncertainty:** publish confidence/coverage intervals; no leaderboard rank where the uncertainty and measurement boundary are insufficient to support it.
8. **Reproducibility:** exact input hashes, solver/build identity, measurement configuration, calibration information and complete energy balance accompany every qualified experimental or modeled result.

## 6. Immediate work order

1. Complete source-text extraction for Riekerk and Zavrel, including exact meter planes and test points; resolve the abstract/table wording discrepancies without rewriting source claims.
2. Implement Rust semantic validation for quantity-to-unit mappings, qualifier/range consistency, measurement-purpose compatibility and exact boundary signatures.
3. Implement deterministic v0-to-v1 migration that leaves free-text or ambiguous records unresolved.
4. Build the matched-condition comparator and Pareto report; generate rejection explanations for cross-boundary cases.
5. Extend the frozen experimental set with the 2026 survey's most relevant IPT and CPT examples, each linked to its primary publication and prototype lineage.
6. Build independent analytic and solver verification cases for IPT/CPT couplers, MQS relays, optical paths and Earth–ionosphere propagation.

**Claim ceiling:** these are proposed research targets and qualification gates. They are not achieved results, regulatory certification, SOTA claims or proof of Tesla's hypotheses.

## 7. Newly clarified industry service boundary and design frontier

### Dynamic roadway: compare service energy, not only instantaneous power

The Purdue–INDOT testbed is reported to deliver 50–190 kW at up to 65 mph, with efficiencies approaching 85% in the Purdue report. INDOT's current project page clarifies the measured efficiency boundary as roadway-inverter DC bus to vehicle DC bus at rated power, not utility-mains-to-battery. It also states that instantaneous power from an energized transmitter is not reduced by vehicle speed; speed changes dwell time over each coil and therefore the integrated energy received over a roadway length. Non-equipped vehicles do not energize the transmitters, and only authorized equipped receivers can trigger the test system. (Purdue report DOI 10.5703/1288284318633; INDOT DWPT project page.)

This yields a stricter dynamic benchmark:
- instantaneous delivered power per occupied energized segment (kW);
- integrated energy delivered to the vehicle per metre / kilometre (kWh/km);
- traction energy demand over the same spatial interval;
- battery state-of-charge change and net battery power;
- segment-to-segment energy continuity and transition dead-zone;
- activation latency, false activation, missed activation, inter-vehicle isolation and safe de-energized state;
- speed, receiver dwell time, pad pitch, vertical/lateral offset, tire/suspension motion and road/weather conditions.

The target that the vehicle should maintain state of charge on the energized corridor is a service-level objective, not proof that the battery is being charged. At steady cruising, much of the received power may go directly to the drivetrain with near-zero net battery power. Report these separately.

### Static pad co-design: Pareto frontier, not efficiency alone

Riekerk et al. explicitly optimize trade-offs among pad area-related power density, efficiency, insulation constraints, winding current density and misalignment tolerance; their published abstract describes critical pad regions susceptible to insulation failure. The benchmark should therefore report separate objectives and hard constraints:
- source-to-load or DC-to-DC efficiency at a declared boundary;
- transmitter and receiver area and the precise area denominator used for areal power density;
- useful load power, mass and thermal rise;
- winding current-density distribution and insulation electric-field stress;
- misalignment and gap robustness;
- field-exposure assessment under applicable guidance.

Do not infer a safe design from a low external stray-field scalar, and do not promote the reported 13.86 μT into a compliance conclusion without probe geometry, averaging, measurement/model provenance and the applicable assessment method.

### Standards revision control

IEEE now lists P63184 as an Active PAR approved 10 September 2025 that will supersede the published IEC/IEEE 63184:2025 edition. The project is a revision-in-progress, not a published replacement standard. The benchmark must bind a compliance assessment to the actually used edition and the authority's adoption; it must not treat the PAR as normative. Also preserve the wording discrepancy between IEEE's overview (stationary and dynamic, with a 1 kHz–30 MHz summary) and the IEC formal scope/title (stationary, 3 kHz–30 MHz). See the source-catalogue revision watch and [IEEE project record](https://standards.ieee.org/ieee/63184/12249/).

## 8. Modular interoperability / multi-receiver research targets

New 2026 references establish a distinct design frontier: power-class switching and simultaneous multi-receiver operation. This is not a direct substitute for increasing a single pad's kW rating.

Proposed targets:
- **Power-class switching:** support SAE WPT1/WPT2/WPT3-compatible operating envelopes with no manual coupler redesign; report efficiency and thermal limits at every actual power class, not just maximum efficiency.
- **Multi-channel independence:** two or more active receiver channels with independently variable loads; target cross-regulation ≤1% of full-scale output during load/coupling steps, and no uncommanded activation.
- **Channel fairness:** publish per-receiver delivered energy, latency and curtailment; target Jain's fairness index ≥0.98 when identical service is requested, without sacrificing hard safety constraints.
- **Partial-load performance:** report aggregate and per-channel efficiency with 0, 1, 2 and N active channels, including standby/idle loss in the whole-system boundary.
- **Reconfiguration reliability:** target 99.99% successful power-class transitions over 10,000 scripted transitions, with fail-safe de-energization on invalid state or loss of vehicle authorization.
- **Shared hardware advantage:** show total system size/mass/cost and lifecycle maintenance separately from electrical efficiency; verify that a shared inverter/LCL network has a net benefit after switching/coupling/thermal overhead.

The current literature references include a 4.60 kW five-mode prototype with all modes >80% DC-to-DC (Liu et al., DOI 10.1109/TIE.2026.3697464), a 6.6 kW six-mode dual-channel prototype with reported maximum DC-to-DC efficiencies 90.27–94.48% (DOI 10.1109/TPEL.2026.3689138), and a two-channel series transmitter array demonstrating 4 kW aggregate output at up to 95.7% maximum efficiency (Singhal et al., DOI 10.1109/TIE.2025.3605456). These are small-scale/architecture demonstrations; not a matched comparison to the 50 kW bus prototypes.

## 9. Multi-channel and power-class interoperability qualification

The source set now includes Liu's scalable five-mode multichannel IPT, Liu's six-mode dual-channel power-class prototype, Wang's combined WPT/LVDC receiver, and Singhal's simultaneous two-EV series transmitter array. These demonstrate architecture options, not a single comparable operating point.

See [WPT-MULTI-RECEIVER-001 #3473](https://github.com/Luminous-Dynamics/symtropy/issues/3473) for the scenario corpus and acceptance plan. The benchmark should track independently: per-channel real power, aggregate power, cross-regulation, inactive-channel leakage, mode switching, activation authorization, fairness, partial-load efficiency and source-to-load energy.

The key comparison trap is exposed by the new frozen case: Singhal's 95.7% maximum at 4 kW aggregate output is not directly rankable against Zavrel's 92.7% whole-WCS at 65 kW. It is a different power point and still has insufficiently documented meter planes and statistics. No one-number ranking is warranted.


## New system-level benchmark lanes (proposed, not achieved)

The current industry evidence argues for a joint objective rather than a single efficiency score. A candidate architecture should publish a full Pareto surface across useful energy, power quality, compatibility, exposure, thermal stress, reliability, mass/cost and controllability.

### 1. Harmonic-aware high-power conversion

Reference evidence: the 20 kW magnetic-integration prototype reports 90.58% DC-DC efficiency, inverter-output current THD reduced from 19.75% to 5.71%, and ZVS over its tested range. The exact boundaries and THD measurement setup require primary-text extraction.

Proposed matched-boundary target:
- DC-DC efficiency >=92% at a declared 20 kW operating point, with the complete converter boundary and uncertainty stated.
- Inverter-output current THD <=5.0% over 20-100% load and the declared frequency range.
- ZVS demonstrated over the entire declared load/alignment envelope, with switching loss and device temperature recorded.
- No trade of lower THD for unreported thermal, insulation, common-mode or exposure penalties.
- Publish efficiency/THD/thermal Pareto curves, not a single best point.

### 2. Bidirectional, multi-vehicle interoperability

Reference evidence: one 2026 experimental system reports aligned grid-to-battery efficiency of 91.7-94.3% and battery-to-grid efficiency of 89.5-93.5%, interoperating with vehicle-side assemblies rated at 6, 11 and 30 kW. Abstract-level conditions do not identify meter planes or map efficiency endpoints to each class.

Proposed matched-boundary target:
- >=95.0% grid-to-battery and >=94.0% battery-to-grid at declared rated points, each with independently metered, calibrated real-power boundaries.
- Demonstrate all 6/11/30 kW vehicle classes on the same grid-side hardware; publish a class-by-class efficiency/load-voltage map.
- Maintain >=90% end-to-end efficiency over the declared misalignment envelope (including at least +/-100 mm X/Y where mechanically applicable); this is a proposed target, not a standards requirement.
- Include four-quadrant power flow, battery voltage variation, direction reversal, transition latency, reactive power, harmonics and standby/auxiliary consumption.
- Prove both directions independently; never average forward and reverse efficiencies into a single opaque number.

### 3. Multi-receiver service rather than paired-coil maxima

- Test 1, 2 and N active receivers with equal and highly unequal load requests.
- Target cross-regulation <=1% at non-faulted channels under a declared worst-case receiver load step.
- Target Jain fairness >=0.99 for equal-priority equal-demand receivers, while reporting delivered-energy shortfall and per-channel efficiency.
- Target >=99.99% successful power-class transitions over at least 10,000 transitions; report every failed transition and safe-state latency.
- An inactive or unauthorized receiver must remain below a published leakage/energization threshold.

### 4. Energy-ledger closure and cross-laboratory reproducibility

- Use synchronized meters and a frozen test protocol with measured meter uncertainty, bandwidth, phase error, sample clock, calibration traceability and environmental conditions.
- Proposed energy-balance residual: <=0.5% in steady state and <=1.0% during declared dynamic trajectories, including change in stored field energy.
- Run a blind round-robin across at least three independent laboratories using shared reference loads and calibration artifacts. Proposed inter-lab expanded uncertainty <=1.0 percentage point for efficiency and <=10% relative for field/thermal results after method harmonization.
- Preserve raw waveforms, calibration certificates, analysis code, exact firmware/controller versions and signed run manifests.

### 5. PV, storage and WPT module reuse

Treat the 2026 PV/storage/WPT cascaded-H-bridge paper as an architecture frontier, not a measured performance benchmark. Require three independently declared operating modes (PV-only, WPT-only, simultaneous PV+WPT), transition tests, storage equalization, MPPT tracking error, circulating current, thermal distribution and source-to-load energy balance. Compare against a conventional separate-converter baseline at equal input/output conditions. Proposed integration acceptance: no more than 1 percentage point loss in end-to-end efficiency versus the optimized separate-converter baseline at equal useful output, while demonstrating a measured reduction in mass, volume or component count.

### 6. Safety and reliability are hard constraints

Efficiency gains do not compensate for exposure, EMC, thermal, fault-containment or interoperability failures. For every claimed envelope, include worst-case alignment, foreign-object/living-object detection, communication loss, unauthorized receiver, fault injection, thermal soak, restart and shutdown. Report exposure assessment under the applicable standards and jurisdiction; the benchmark is not a substitute for certification.

All targets in this section are development targets. None is evidence of a tested Symtropy design or a standards-mandated threshold.

### 7. Frequency-polymorphic vehicle interoperability

The full text of the 2026 bidirectional interoperability study (DOI 10.1016/j.geits.2025.100307), Table 1, specifies three different vehicle assemblies:

| VA | Rated power | Battery-voltage range | Nominal voltage | Ground-clearance range | Frequency |
|---|---:|---:|---:|---:|---:|
| 6 kW | 6 kW | 380–440 V | 400 V | 190–210 mm | 85 kHz |
| 11 kW | 11 kW | 600–700 V | 660 V | 160–180 mm | 85 kHz |
| 30 kW | 30 kW | 420–610 V | 550 V | 270–290 mm | 20 kHz |

The paper describes two independently enabled ground-side resonant networks for the 85 kHz and 20 kHz frequency bands, plus a four-module adaptive front-end converter. This is stronger than simple power-class switching: interoperability spans frequency, battery-voltage, clearance and compensation regimes.

Proposed benchmark targets (not achieved):
- Demonstrate 6/11 kW at 85 kHz and 30 kW at 20 kHz with the same ground-side platform, and publish separate class/frequency response surfaces.
- Verify isolation between the two resonant networks during activation, deactivation, fault and mode transition; no inactive-network overcurrent or unintended receiver energization.
- Sweep each class across its full declared battery-voltage range and mechanical ground-clearance range; publish efficiency, output regulation, reactive power, THD, coil current, thermal stress and exposure in the same operating grid.
- Test module activation counts 1/2/3/4 across load, power-class changes and reverse power flow; report fixed overhead, switching transients and energy cost of transitions rather than assuming module shedding is efficient.
- Target ≥99.99% correct frequency-network and converter-module transitions across 10,000 scripted synthetic/bench events, with all faults and safe-state transitions enumerated.
- Do not assign 91.7–94.3% or 89.5–93.5% range endpoints to individual vehicle assemblies until the primary result plots/tables establish the mapping.

## 8. Frequency-polymorphic resonant-network isolation

The 2026 Zhang et al. full text (DOI 10.1016/j.geits.2025.100307) reports two separately selectable GA resonant networks at 85 kHz and 20 kHz. Its integrated model predicts 17 A in the closed 85 kHz loop when the 30 kW / 20 kHz network is enabled, and 4 A in the closed 20 kHz loop when the 6/11 kW / 85 kHz network is enabled. The paper notes waveform distortion and efficiency loss, with relay isolation as a possible mitigation that adds control, cost, weight and conduction loss. These are model-derived values, not measurements.

Proposed qualification targets:
- Inactive branch steady RMS current <=1% of its continuous current rating; all transient peak current and induced voltage remain under component ratings.
- Measure branch-level current, voltage, temperature and switching energy; a commanded-off branch cannot be assumed electrically inactive.
- No simultaneous ACTIVE networks when the architecture declares concurrent operation disallowed.
- No branch left UNKNOWN/TRANSITIONING during a qualified steady-state run.
- Test both frequency-selection directions, all vehicle classes, voltage/gap/alignment corners, stuck-open/stuck-closed switches, contact welding, sensor disagreement and communication loss.
- Benchmark the efficiency/weight/cost trade-off between physical relay isolation, solid-state isolation and passive decoupling as separate design alternatives.

The 1% inactive-branch target is a proposed research threshold, not a standard limit or an achieved result. It must be applied to each branch's own continuous current rating, not to the active branch or the system's aggregate current.

## 9. Cyber-physical resilience as a hard benchmark dimension

A 2026 BWPT review frames modelling, control, communication and cybersecurity as a coupled system. It identifies spoofing/replay, firmware manipulation, inductive sensor-data injection, timing/latency exploitation, credential compromise, DoS and grid-side command injection as paths into control instability, loss of ZVS, increased circulating current, unintended V2G and thermal stress. (Elumalai et al., DOI 10.1002/ese3.70608.)

Proposed synthetic/bench qualification objectives (not achieved):
- 100% rejection of a frozen corpus of forged receiver identities, stale/replayed commands and unauthorized power-class/direction requests.
- No frequency-network selection unless receiver identity, authenticated session, requested power class and physical alignment agree.
- All replay/freshness checks bind nonce/timestamp/sequence to session identity, direction, power class, frequency and expiry; stale state cannot be reused after reconnection.
- Firmware integrity and protection-threshold configuration are measured and tied to the run manifest; modified firmware causes a fail-safe rejection.
- Sensor cross-checks detect injected current/voltage/phase/alignment/temperature disagreement before control actuation; the safe fallback must not depend solely on the compromised sensor.
- Networked-mode p99 latency and maximum jitter are measured at each load/mode transition; timeouts lead to safe de-energization or a qualified local-safe mode.
- A denial-of-service or EMI-corrupted communication test may reduce service availability but must not cause unintended energization, reverse power flow, switch chatter or resonant-network co-activation.
- Report energy/efficiency impact of security controls separately from cryptographic CPU load, communication standby, sensing and fault-response energy.

Security, safety, availability and efficiency form a constrained Pareto surface. A higher transfer efficiency never compensates for unauthorized power flow, replay acceptance, sensor-trust failure or unsafe fallback.

## 10. Temperature-aware core flux and saturation margin

For each resonant network, publish the ferrite/core material, saturation-versus-temperature curve and worst-case peak flux. Target peak flux <=80% of the saturation threshold at maximum measured core temperature (20% hot-state margin). Include uncertainty, temperature gradients, material lot variation and waveform distortion; do not use the room-temperature saturation number to qualify a thermally soaked system.

The 2026 bidirectional prototype paper reports a calculated 376 mT core flux and cited ferrite saturation values of 530 mT at 25°C and 410 mT at 100°C. From these values, utilization rises from 71.0% to 91.7% at the hot reference, leaving 8.3% nominal margin to the cited hot threshold. This is a source-based calculation, not a measured failure. The proposed 80% limit is a research design target, not a standard requirement.


## 11. Standards-aware interoperability scoring

A standards score must be scope-qualified before it contributes to any performance comparison. Proposed gate:
- 0 false-compliance claims: the record never labels an out-of-scope test as normative or certified.
- For SAE J2954_202408 normative claims, require stationary, surface-mounted, grid-to-vehicle operation and measured source-side apparent power <=11 kVA; this test checks declared applicability, not all provisions of the standard.
- For >11 kVA or heavy-duty systems, record J2954/2 context and the exact independently verified requirements used. J2954/2_202212 is an Information Report with incomplete detailed requirements, not a complete compliance test.
- For bidirectional WPT, record J2954_202408 as partial/out-of-scope as appropriate and separately identify J3072/J2836/3/IEEE/UL provisions actually applied. J3072 is conductive-EVSE interconnection scope and J2836/3 is DER use-case guidance; neither alone certifies wireless V2G.
- For dynamic roadway tests, do not cite a future/planned J2954/3 as a published requirement. Track it as PLANNED_NOT_ISSUED until a current issued edition is confirmed.
- Distinguish standard applicability, test method borrowed, self-assessment, and independent certification in reports.

The goal is not standards name-dropping. It is a machine-checkable boundary between what was required, what was tested, what was merely referenced and what remains outside the published scope.


## 12. Dynamic WPT mission-level targets

The primary dynamic outcome is not peak power; it is whether a real vehicle mission can maintain its battery energy while accounting for roadway energy and segment reliability.

Proposed goals (not achieved or standards thresholds):
- Energy sufficiency ratio >=1.00 on the declared reference mission (battery energy accepted from roadway divided by propulsion energy over the same route). Stretch goal >=1.10 for a declared reserve margin.
- Publish vehicle-side energy delivered in Wh/km and net battery Wh/km with measured meter planes; do not report peak kW as a proxy for energy sufficiency.
- 0 unauthorized/false segment energizations across the frozen negative-control and route event corpus.
- Segment activation success >=99.999% across >=1,000,000 controlled state transitions, with false activations reported separately and always treated as a fail.
- No single segment dropout may be hidden by averaging: report count, duration, route location, lost energy and recovery.
- Net vehicle DC energy per km repeatability <=2% coefficient of variation across labs for the same speed/profile, receiver geometry and instrument boundary.
- Route/source-to-battery energy balance residual <=0.5% steady and <=1% dynamic, alongside expanded measurement uncertainty and per-stage loss ledger.
- Endurance availability >=99.9% only after a declared mission profile, accumulated energized hours, weather exposure and maintenance plan are defined.
- Publish infrastructure idle energy per lane-hour and per vehicle-km, not just energy transfer during an active vehicle crossing.

Industry baseline: Purdue/INDOT describes 85 coils embedded in a quarter-mile roadway and field delivery from 50 to 190 kW at up to 65 mph, with approximately 85% roadway-inverter-DC-bus to vehicle-DC-bus efficiency at rated power. The 2026 SS/LCC comparative paper reports 0.14 and 0.15 kWh/km in simulation against an assumed vehicle demand of 0.129 kWh/km at 90 km/h. Our benchmark improvement is to require the route profile, energy meter boundaries, battery energy, propulsion use, segment activations and uncertainty to be present before accepting a mission-level claim.


## 12. Foreign-object detection and active thermal suppression

### Current evidence and SOTA

- MIT sensor-array FOD: 120 channels, 13 engineered features, 17,800 pre-transfer frames; best reported live cross-session accuracy 98% (49/50) after feature engineering, versus 62% raw RF (31/50). The 75-80 ms reported pipeline is classifier latency, not confirmed power-stage de-energization latency.
- Active-transfer FOD from the same study is promising but small and controlled. Some displayed percentage values do not reconcile exactly with confusion counts; preserve the discrepancy and do not treat it as safety proof.
- Active DeMag: reported 76% lower 25 mm coin temperature rise after 600 s (56.6 °C to 13.7 °C), with reported overall efficiency 94.06% at 1.5 kW and 95.07% at 3.3 kW. Auxiliary-inverter energy boundaries need full-text extraction.

### Proposed targets (not achieved, not standards limits)

1. At least 10,000 object-and-session-held-out hazardous-object positive cases and 10,000 held-out negative cases, with zero false negatives. For 0 misses in 10,000 independent positive trials, the exact one-sided 95% upper bound on miss probability is approximately 0.03%. Repeated frames from one object/session are correlated and must not be represented as 10,000 independent objects.
2. Split by object identity, acquisition session and ultimately laboratory/site. Report per-material, size and position errors. Random-frame-only splits do not support safety claims.
3. p99 detection decision latency <=50 ms on target edge hardware, improving on the published 75-80 ms host pipeline; measure safe-de-energization latency separately.
4. Localization median error <=5 mm and p95 <=10 mm on a declared spatial grid, with calibrated uncertainty.
5. Expected calibration error <=2%; 100% abstention on unknown materials, failed sensors, out-of-domain signals or localization uncertainty crossing the safe/unsafe boundary.
6. Validate the same held-out object suite before transfer and during full declared power, frequency, misalignment, temperature and EMI conditions. Do not retune thresholds on the held-out set.
7. Target >=85% reduction in object temperature rise against a matched baseline (the cited DeMag paper reports 76% for its coin case), across multiple materials and geometries.
8. Compare DeMag ON/OFF at matched useful output and common meter planes. Include auxiliary inverter, sensing, cooling and standby energy. Proposed system-efficiency penalty <=0.5 percentage point while meeting the thermal constraint.
9. On actuator, thermal-sensor, localization or branch-health fault, de-energize safely. Continued transfer is allowed only inside an independently validated object/material/location/power envelope.
10. Reproduce across at least three laboratories with unseen object/material sets, inter-lab uncertainty, frozen code/configuration digests and independent thermal ground truth.

### Paradigm shift

Co-design the MIT sensor geometry, resonant magnetic field, ferrite/shield thermal path and active DeMag coils as one constrained system. Optimize an uncertainty-aware safety envelope rather than raw classification accuracy or nominal transfer efficiency. A safe action may be INHIBIT, DERATE, ACTIVE_DEMAG or ABSTAIN; continued transfer is acceptable only when object material, localization, thermal horizon and mitigation health are inside a validated envelope.

## 13. Cross-generation geometry compatibility

### Research baseline

Koch et al. (2026), DOI 10.1109/JWPT.2026.3721477, report four vehicle assemblies, three ground assemblies, 189 discrete positions, 3,402 evaluation points, 13,608 inductance samples and at least 34% magnetic-coupling operating-range degradation when legacy WPT3 VAs are paired with a flush-mounted GA. The proposed WPT4 coil concept mitigates the loss. The indexed abstract lacks raw point coordinates, the exact baseline/threshold and uncertainty. It is not an efficiency result and is preserved SOURCE_CLAIM_ONLY.

### Proposed qualification targets (not achieved)

- At least 189 unique geometry positions per GA/VA pairing as an initial comparison floor; adaptively refine boundary regions and preserve all added points.
- Publish separate regions for coupling-only, output-power, regulation, efficiency, thermal, safety and full system-interoperability scopes.
- Report evaluated points, qualified points, fraction of the sampled domain that passes, connected-component count, 90–99.99% confidence level, lower confidence bound and raw pass/fail classification digest.
- Never fill a disconnected region by replacing it with a convex hull. If hulls are used for display, preserve the underlying component map and label the hull as an approximation.
- For SYSTEM_INTEROPERABILITY, require hard-gate evidence for coupling, output power, output regulation, efficiency, thermal, safety, component stress and EMC.
- Require numeric expanded uncertainty, coverage factor and coverage probability; no uncertainty checkbox without a numerical budget.
- Cross-generation goal: no more than 5% relative reduction of the measured qualified operating-region fraction for a declared legacy VA under a new GA, unless product requirements explicitly accept the trade.
- Repeat same-fixture qualified-region fraction at <=2% coefficient of variation; inter-laboratory coverage difference <=5 percentage points after frame/fixture harmonization.

These are research targets, not SAE or IEC requirements.

## 14. Industry scorecard: matched comparison lanes, not a single WPT leaderboard

| Research lane | Current industry/reference evidence | What it actually measures | Proposed stretch target (not achieved) | Comparison rule |
|---|---|---|---|---|
| Static high-power IPT | Riekerk 50 kW opportunity charger reports 96.824–97.165% DC–DC under ±100 mm X/Y misalignment; Zavrel 65 kW WCS reports separate back-to-back points (97.5% at 15 cm, 95.3% at 25 cm) and whole-WCS 92.7%. | Different DC–DC and whole-system boundaries, power classes, geometries and conditions. These are not one leaderboard. | >=98.0% DC–DC nominal; >=97.5% DC–DC at a declared ±100 mm X/Y stress envelope; separately target >=95% grid-to-useful-load efficiency with full auxiliary inclusion. | Match exact meter planes, stage set, rated/load power, air gap, alignment, thermal steady state, statistic and uncertainty before ranking. Do not treat the Riekerk survey-secondary 50.96 kW point as the primary abstract range. |
| Dynamic roadway WPT | Purdue/INDOT: 50–190 kW, up to 65 mph, approximately 85% roadway inverter DC-bus to vehicle DC-bus, with 85 embedded coils. | Field transfer efficiency on a defined DC-bus boundary; route energy sufficiency, upstream grid/storage, battery acceptance and no-vehicle idle energy are not all included. | >=90% on the same inverter-DC-bus to vehicle-DC-bus boundary at declared 100–190 kW and up to 65 mph; >=1.05 net battery WPT Wh / propulsion Wh on a declared motorway mission after standby and battery acceptance. | Report instantaneous kW and integrated Wh/km separately. Include source/idle energy, battery-terminal gain, propulsion demand, segment handover and availability. |
| Bidirectional multi-class EV WPT | Zhang et al. 2026 reports aligned system-efficiency ranges 91.7–94.3% G2V and 89.5–93.5% V2G; class-specific link DC–DC ranges are 94.0–95.7% (6 kW), 94.7–97.1% (11 kW) and 93.6–94.5% (30 kW). | Whole-system direction-separated ranges and link-stage values are different measurements; table slash order is not used to assign per-class direction. | >=95% full-system efficiency in each direction at each declared class, with <=0.5 percentage-point degradation across the qualified voltage/alignment envelope. | Keep G2V and V2G separate. Do not compare link DC–DC to full-system efficiency, and do not assign Table 4 slash pairs to directions without source evidence. |
| Cross-generation geometry interoperability | Koch et al. 2026 reports >=34% degradation of magnetic-coupling operating range for legacy WPT3 VAs paired with a flush-mounted GA; 189 positions, 3,402 evaluation points and 13,608 inductance samples. | Author-defined magnetic-coupling operating range, not efficiency. The indexed abstract lacks absolute coordinates, baseline threshold, raw point data and uncertainty. | <=5% reduction of the *same named qualified operating-region metric* for legacy VA under a ground-assembly generation change, across >=3 VAs and >=2 GAs. | First recover the source's precise operating-range definition and baseline; a new qualified-system region is a distinct metric until matched. |
| Foreign-object safety | MIT study reported 98% live cross-session classifier accuracy (49/50) after feature engineering; a separate active DeMag study reports 76% lower coin temperature rise after 600 s. | Classifier accuracy, classifier latency, safe shutdown and measured object temperature are separate outcomes. | 0 false negatives in >=10,000 pre-registered object/session-held-out hazardous-positive cases; thermal predictor coverage 90–98%; p99 pre-energization decision <=100 ms. | Report exact binomial confidence bound, independent objects/sessions, material coverage, localization, abstention, active-transfer EMI, safe-de-energization time and thermal ground truth. |
| Magnetothermal / magnetic materials | Wei et al. 2026 magnetic flux bridge reports 31.3% lower maximum ferrite temperature, 8.1% lower leakage flux and 2.06% system-efficiency improvement (relative-vs-percentage-point convention unresolved in abstract). | Matched-condition improvements are not yet independently comparable; exact baselines, sensor locations and uncertainty need full-text extraction. | Peak ferrite flux <=80% of lower-confidence Bsat(T) at maximum measured hot-spot temperature; >=20% hot-state saturation margin, with no unreported efficiency/leakage trade. | Compare simultaneously at matched power, gap, alignment, waveform, cooling, ambient and duration. Record material lot and ageing state. |
| Metrology / independent replication | Published WPT papers use heterogeneous meter planes, stage sets, gaps, statistics and reporting. The 2026 interoperability review identifies cross-testing, environmental variability and standard test methods as continuing barriers. | Published numerical rankings are often not matched; high efficiency claims can hide upstream/downstream or standby losses. | >=3 independent labs; <=1.0 percentage-point inter-lab expanded uncertainty for efficiency after harmonization; <=0.5% steady / <=1.0% dynamic energy-balance residual. | No SOTA superiority claim until frozen conditions, calibration, uncertainty, source-to-load ledger and independent lab replication are available. |

### How to beat SOTA rather than just beat a headline

1. **Choose one lane and one boundary.** Static DC–DC, dynamic roadway DC-bus transfer, full-system G2V/V2G, geometry coupling, thermal or FOD results are not interchangeable.
2. **Match the strongest published operating point.** Same power class, actual load, frequency, coil gap, misalignment, temperature, receiver/battery state, duration and statistic.
3. **Measure the entire service.** Include upstream source energy, converters, auxiliaries, standby, battery acceptance, useful energy and stored-field changes. Publish loss partition and signed residual.
4. **Optimize lower-confidence performance.** Report P5/P50/P95 and worst-case qualified-envelope fraction. Optimize the robust envelope, not nominal peak.
5. **Demonstrate transfer.** Use held-out geometries/material lots, cross-generation VAs, variable speed and multi-vehicle contention; then reproduce at three independent laboratories.
6. **Make safety a hard constraint.** A higher efficiency score cannot compensate for a false-negative FOD event, unauthorized energization, hot-state saturation, excessive exposure, EMC failure or unsafe control fallback.

Outperformance is demonstrated only when the confidence interval for the matched new result exceeds the reference interval under a pre-registered comparison protocol. Where source uncertainty or boundaries are missing, the correct result is UNRESOLVED—not a fabricated victory.
