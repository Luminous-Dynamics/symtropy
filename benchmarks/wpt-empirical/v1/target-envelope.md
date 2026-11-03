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
