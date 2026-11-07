# Ground–air wireless energy coexistence v0

**Status:** research specification and synthetic-only implementation target. No integrated hardware, field test, aviation approval or safety claim.

Tracking issue: [#3980](https://github.com/Luminous-Dynamics/symtropy/issues/3980)

## 1. Core thesis

One site can share electrical supply, storage, source accounting and high-level scheduling while operating separate ground and air wireless-energy delivery channels concurrently.

Do **not** infer that a common electrical source requires a common transmitter coil, resonant circuit, carrier frequency, antenna, optical path or receiver. The first architecture is multi-bearer:

- Ground dynamic road IPT / stationary EV pads.
- Stationary or docked aircraft IPT/CPT.
- Near-field hover UAV IPT/CPT.
- Far-field UAV RF power beaming.
- Far-field UAV optical laser power transfer.
- Future eVTOL airborne transfer remains a separate high-power research lane.

The shared system is a site-energy network, not a universal WPT coupler.

## 2. Existing work to compose

| Owner | Existing work | What this work reuses |
|---|---|---|
| Aircraft energy | [#3885](https://github.com/Luminous-Dynamics/symtropy/issues/3885) | Mission demand, aircraft energy architecture |
| Aircraft static dock | [#3902](https://github.com/Luminous-Dynamics/symtropy/issues/3902) | Docked IPT/CPT service, not airborne beam power |
| Optical LPT | [#3458](https://github.com/Luminous-Dynamics/symtropy/issues/3458) | Optical path, PV receiver and beam safety evidence |
| Ground multi-receiver | [#3473](https://github.com/Luminous-Dynamics/symtropy/issues/3473) | Receiver/channel isolation and power-class tests |
| Robust dispatch | [#3559](https://github.com/Luminous-Dynamics/symtropy/issues/3559) | Constrained allocation, fairness, oracle comparison |
| Energy ledger | [#3472](https://github.com/Luminous-Dynamics/symtropy/issues/3472) | Typed source/load meter boundaries and losses |
| WPT/grid stability | [#3715](https://github.com/Luminous-Dynamics/symtropy/issues/3715) | Converter-network stability |
| Multiport passivity | [#3530](https://github.com/Luminous-Dynamics/symtropy/issues/3530), [#3546](https://github.com/Luminous-Dynamics/symtropy/issues/3546) | Physical admissibility |
| Verified pass evidence | [#3950](https://github.com/Luminous-Dynamics/symtropy/issues/3950) | Exact pass/session/artifact lineage |

The gap is cross-domain coexistence: same site, overlapping ground and air service windows, shared source limits, separate physical channels and one accountable site energy ledger.

## 3. Feasibility evidence (separate regimes, not a common leaderboard)

| Regime | Published/announced evidence | Limitation |
|---|---|---|
| Ground dynamic WPT | Purdue–INDOT: reported 50–190 kW, up to 65 mph, near 85% at roadway inverter DC→vehicle DC | Not grid→battery efficiency |
| Two airborne receivers | IEEE 2025: 500 W prototype, 5 A to each of two drones, air gap extended 150→175 mm | Not ground+air integrated operation |
| UAV adaptive inductive array | IEEE 2026: 350 W, 85 kHz, 50 mm gap; 88.6% (2-Tx), 87.9% (4-Tx) peak DC–DC | Must also score field uniformity and off-target leakage |
| Capacitive UAV multi-output | IEEE 2026: up to 709.9 W single-load, 89.06% peak DC–DC | Separate capacitive topology |
| Drone in-flight IPT | IEEE 2024: 73.8 W, 88.3% over its tested range | Small-UAV power class |
| RF beam-powered flight | Reach announced 10-hour autonomous drone flight in Oct 2026 | Public announcement omits range, delivered power and efficiency |
| Optical LPT | #3458 records a 20 m indoor demo at ~99 W and 12.4% end-to-end | Indoor optical result; not comparable to IPT/RF |

The sources support the building blocks. They do not show a shared site simultaneously charging ground road vehicles and aircraft with one coordinated source, ledger and coexistence controller.

## 4. Shared-source feasibility equation

At the site boundary, account for real energy and instantaneous power separately.

For each time window:

`P_grid + P_generation + P_storage,discharge
 >= P_ground,input + P_air,input + P_aux + P_conversion/losses + P_storage,charge`

Use a signed storage convention in implementation so storage charge and discharge cannot both be counted as supply. A real source/feeder/PCC envelope constrains the aggregate, while each channel has its own qualified envelope.

This is source scheduling, not proof that channels are electromagnetically isolated. Cross-coupling, EMC, harmonics, common-bus stability, RF interference and optical path availability remain separate gates.

## 5. Proposed stack

1. **Site energy:** feeder/PCC, local generation, stationary storage, auxiliaries and site configuration.
2. **Ground channel:** dynamic roadway or stationary road-vehicle IPT.
3. **Air near-field channel:** dock/hover IPT or CPT.
4. **Air far-field channel:** RF or optical LPT, each with independent acquisition, tracking, exposure and path-termination states.
5. **Supervisory allocator:** session admission, service windows, source limits, fairness, forecast uncertainty and explicit curtailment.
6. **Independent safety authorities:** local converter protection, RF/laser interlocks, aircraft operations and electrical safety; the site allocator cannot override these.
7. **Evidence loop:** exact artifacts, meter planes, calibration and trajectory/mission lineage.

## 6. Paradigm shifts

- Multi-bearer energy fabric instead of a universal wireless charger.
- Space-time-carrier co-design: schedule road lane, airspace/hover/dock opportunity, transmission channel and feeder capacity together.
- Service contracts: request useful Wh by a deadline with a minimum floor, not simply a kW setpoint.
- Pareto assurance: optimize service, availability, fairness and losses; keep safety and evidence validity non-compensatory.
- Cross-modal observability: maintain separate typed physical models but test interactions at shared buses, sensors, spatial zones and safety barriers.
- Robust dispatch: uncertainty-aware allocation with explicit abstention when a channel's support or safety disposition is unknown.
- Geographic energy mobility: coordinate road corridors, vertiports, rooftops and logistics hubs with Sol-Atlas site context.

## 7. Benchmark design

### Scenario dimensions

- 1–8 ground receivers and 1–8 air receivers for bounded software cases.
- Dynamic road pass, stationary aircraft dock, UAV hover, UAV RF/optical session.
- Concurrent overlap, staggered arrival/departure, failed handover and channel outage.
- Feeder/PCC cap, BESS power/energy limits, local generation and forecast error.
- Equal/unequal priorities, deadline pressure, reserve service and emergency curtailment.
- Load/coupling changes, air/ground position uncertainty and missing sensor data.
- Shared-bus disturbance, cross-channel leakage, EMC and field/beam safety dispositions.

Keep modalities separate: inductive, capacitive, RF and optical efficiencies must not be averaged or compared without matching endpoints and operating conditions.

### Required outputs

- Source/PCC real Wh, per-channel input and useful endpoint Wh at named planes.
- Storage energy, standby and auxiliary consumption.
- Aggregate energy residual with uncertainty.
- Per-session useful service, request shortfall, wait/time-to-ready and availability.
- Cross-regulation, off-target leakage, common-bus stability and hard-limit margins.
- Session fairness and Pareto trade-offs.
- Dispatcher runtime, deadline misses and exact-oracle gap for small episodes.

### Baselines

1. Independent ground/air schedulers with fixed source reservations.
2. Equal-share allocation.
3. Round-robin service.
4. Greedy highest instantaneous service/efficiency.
5. Domain-priority allocation.
6. Reproduced robust MPC where the model and solver assumptions can be frozen.
7. Integrated robust policy.
8. Exact offline oracle for bounded synthetic instances.

Every method receives identical frozen requests, source limits, channel envelopes, uncertainty sets and measurement boundaries.

## 8. Proposed targets (not achieved / not standards)

- 100% expected outcomes for the frozen synthetic/adversarial corpus.
- Zero admitted source/PCC, storage or channel hard-limit violations.
- Zero false-safe admissions when a required modality safety/interference/airspace state is unknown.
- Exact energy accounting with no hidden standby/auxiliary or storage flows.
- >=99.99% correct software admission/deactivation/fault-state handling over 10,000 scripted events, with confidence bounds.
- Jain fairness >=0.98 for genuinely equal requests; report Pareto results for unequal priorities.
- P99 supervisory allocation latency <=10 ms only on the declared small 2–8 receiver software benchmark; not an inner control/safety timing target.
- Any efficiency improvement must use the same useful service and named meter planes; never compare component efficiencies with end-to-end values.

## 9. Safety / evidence boundary

SAE J2954 is a light-duty road WPT reference, not aviation certification. IEC PAS 61980-5:2024 and ISO/PAS 5474-6:2025 belong to road-WPT lanes. IEC/IEEE 63184:2025 provides inductive WPT human-exposure assessment methods in its stated frequency range, but it does not qualify aircraft operations, RF beaming or laser safety.

RF spectrum/exposure/EMC and aviation operations require jurisdiction-specific review. Optical transfer needs appropriate laser accessible-emission, beam termination/interlock and aviation review. No numeric exposure or laser safety limit is invented by this benchmark.

This work authorizes research, schemas, synthetic software and offline/emulator design only. It does not authorize high-power construction, RF/laser operation, roadway/flight testing or human-exposure testing.

## 10. Execution sequence

- P0: recover source-level evidence and normalize experimental operating points from #3473/#3477/#3458/#3885/#3902.
- P1: freeze site/channel/session schema and NOT_RUN template.
- P2: implement bounded Rust source allocation and coexistence evaluator with exact small-instance oracle.
- P3: connect #3559, #3715, #3950, aircraft mission energy and Sol-Atlas site context.
- P4: compare independent-domain schedulers with integrated robust/MPC policy on frozen synthetic cases.
- P5: independent metrology, coexistence and modality-specific safety review before an energized prototype.

## References

- [Purdue–INDOT 2026 report](https://doi.org/10.5703/1288284318633)
- [Dual-drone IPT 2025](https://doi.org/10.1109/ACCESS.2025.3601215)
- [UAV reconfigurable multi-transmitter IPT 2026](https://doi.org/10.1109/JESTPE.2026.3724184)
- [Capacitive UAV multi-output 2026](https://doi.org/10.1109/TPEL.2025.3603638)
- [Drone in-flight IPT 2024](https://doi.org/10.1109/TPEL.2024.3440963)
- [Reach Power 10-hour flight announcement](https://reachpower.com/news/wireless-powered-drone-10-hour-flight/)
- [IEC/IEEE 63184:2025](https://standards.ieee.org/ieee/63184/10156/)


## Machine-readable starting artifacts

- [coexistence.schema.json](./coexistence.schema.json) defines source, site, modality, channel qualification, ground/air sessions, allocations, limits and separate safety/coexistence states.
- [coexistence.NOT_RUN.template.json](./coexistence.NOT_RUN.template.json) is deliberately empty and explicitly NOT_RUN. It is not a benchmark result.
- The non-NOT_RUN contract requires at least one ground and one air session. Temporal overlap, channel-to-domain identity, source/PCC sums, storage accounting, allocation coverage and concurrent-window constraints are evaluator invariants; JSON shape validation alone cannot prove them.

The first positive known-truth fixture must use synthetic, fully declared source/channel numbers and independently hand-computed allocations. It must not be assembled by relabelling separate ground and UAV prototype measurements as one co-located experiment.

## 11. Executable bounded allocator — synthetic v0

The first source-level implementation is now present:

- `tools/ground-air-coexistence-oracle` is a Rust workspace crate using only the existing `serde_json` dependency.
- [`scenario.schema.json`](./scenario.schema.json) is the closed execution-scenario contract.
- [`scenario.synthetic-example.json`](./scenario.synthetic-example.json) is the common four-slot ground-truck + docked-UAV arithmetic fixture.
- The package is registered in the workspace and lockfile; CI runs `cargo test --locked -p ground-air-coexistence-oracle`.

### What the exact oracle does

It allocates integer input-power quanta over discrete time slots subject to:
- shared PCC power cap for every slot;
- total source-energy budget across the episode;
- per-channel and per-session input-power caps;
- session availability windows and requested useful-energy caps;
- simultaneous positive ground and air delivery, with the number of concurrent slots explicitly maximized after service fairness and source use are optimized;
- strict domain/modality/receiver identity agreement;
- synthetic-only envelope, authorization, safety and coexistence states.

The bounded exact objective is lexicographic: maximize sessions meeting their minimum service floor; maximize the sorted minimum-to-maximum request-fulfilment ratios (leximin); maximize total useful energy; minimize source energy; maximize the count of time slots with positive ground and air delivery; then prefer earlier delivery with a stable tie-break. This treats sustained simultaneity as an explicit service objective, not a one-time episode flag. If the node bound is exhausted, the evaluator returns an error and emits no partial optimum.

v0 bounds the scenario to 8 time slots, 8 channels, 8 sessions, 8 power quanta per slot and at most 1,000,000 search nodes. It models one aggregate grid/PCC source; storage, generation, multiple feeders, ramp dynamics and stochastic arrivals remain later increments.

### Fixture arithmetic (not physical performance)

The synthetic fixture has a 3-quanta-per-slot PCC cap, four 60-second slots, an 80 Wh source budget, 10 Wh per input quantum-slot, a 16 Wh ground useful-energy request and a 12 Wh air useful-energy request. Its exact optimum allocates one air and one ground quantum in **all four slots**, delivering 12 Wh to the aircraft battery boundary and 16 Wh to the ground-vehicle battery boundary. Total synthetic useful energy is 28 Wh from 80 Wh of site input. The 35% arithmetic ratio is not a measured charging efficiency, and the fixture coefficients are not physical conversion models.

Those values are invented known-truth arithmetic controls. The reported useful-to-source ratio is **not** an experimentally measured conversion efficiency, and the input coefficients are not electromagnetic or aircraft models.

The channel records intentionally declare `DECLARED_SYNTHETIC_SUPPORT`, `NOT_EVALUATED_SYNTHETIC_ONLY` for both safety and coexistence, and `SYNTHETIC_FIXTURE_ONLY` session authorization. The oracle refuses measured/field evidence or any state that could be mistaken for physical permission. It is not an energization controller.

### Test status

Nine Rust unit-test declarations cover the known-truth allocation, row-order determinism, insufficient budget, unresolved safety/coexistence states, domain/modality mismatch, non-overlapping sessions, search-bound exhaustion, energy-coefficient conservation and unknown/non-synthetic input rejection.

No local `cargo`, `rustc` or `rustfmt` is installed in this authoring environment. The source, lockfile and CI wiring are committed, but compilation, formatting and test execution are **not claimed** pending exact-head CI.


## 12. UAV charging evidence update — 7 November 2026

A new open-access review, Wu et al., *Autonomous Charging and Battery Swapping for Unmanned Aerial Vehicles: A Review*, first published 25 October 2026 (DOI [10.1002/aisy.70584](https://doi.org/10.1002/aisy.70584)), compares autonomous contact docking, IPT, CPT, laser power transfer, microwave power transfer and battery swapping. It changes the first-demonstrator priority:

### Contact docking is a mandatory baseline, not an afterthought

The review reports one autonomous contact-charging platform that charged reliably across 832 landing poses at greater than 98% efficiency and completed 30/30 outdoor flights. It also notes that environmental conditions affect precision landing and that connector durability remains insufficiently characterized for UAV service; the cited approximately 3,000 mating-cycle degradation figure is from general connector testing, not a UAV-specific endurance result.

Therefore the first shared-site demonstration should include an independently metered **conductive/robotic aircraft dock** alongside the ground dynamic-inductive roadway. This already tests true concurrent site-energy allocation and metrology without pretending that every aircraft must be wirelessly charged.

### Near-field UAV WPT is credible, but must be compared by the full operating envelope

The review's heterogeneous source table includes:
- IPT at 125 W and 97.66% peak efficiency, falling to 85.48% at 100 mm lateral misalignment in the same study;
- IPT at 260 W and 91.57%, with reported tolerance to 25 mm X, 40 mm Y and 25° rotational misalignment;
- IPT at 175 W and 92.3% at a 5 mm gap, with 200 mm lateral tolerance;
- a 350 W / 85 kHz multi-transmitter UAV IPT study with 88.6% (two transmitters) and 87.9% (four transmitters), which also treats field uniformity and off-target leakage as objectives;
- a 500 kHz single-input/multiple-output CPT experiment reporting 709.9 W maximum single-load output, 89.06% peak DC–DC efficiency and tolerance to 125 mm lateral / 60° angular misalignment.

These are different topologies, gaps, receiver masses, measurement conditions and peak/steady-state profiles. Preserve each as its own source-bound benchmark lane; do not construct a universal UAV charging envelope from their maxima.

### Far-field UAV power remains a distinct readiness class

The review's microwave examples include approximately 2.28 W received at 1 m with about 0.7% efficiency, 17.6 mW at 0.8 m with about 0.6% efficiency, and approximately 1.7 mW at 1.5 m with 0.044% efficiency. It concludes that the surveyed microwave systems do not yet meet typical rotary-wing UAV propulsion demand. Older laser-powered UAV research has demonstrated long endurance in constrained operating areas, but the review identifies alignment, low end-to-end efficiency and laser safety as major practical obstacles.

Reach Power's 28 October 2026 announcement is a separate and important operational milestone: it reports a 10-hour autonomous RF-beam-powered quadcopter flight, autonomous energy requests, beam release, departure and reacquisition. The public announcement does not disclose delivered RF power, transmitter-to-aircraft distance or end-to-end energy efficiency. It therefore belongs in an **autonomous endurance / reacquisition** benchmark, not a numeric efficiency leaderboard.

### Staged demonstrator sequence

1. **M0 — shared-site concurrency:** dynamic ground IPT plus a separately metered conductive/robotic docked-UAV charger on one constrained site source. Demonstrate simultaneous demand, allocation, source accounting and service fairness.
2. **M1 — two near-field channels:** replace or supplement the aircraft conductive dock with a separately qualified UAV IPT/CPT pad. Retain independent converters and channel-local protection.
3. **M2 — multi-air receiver service:** multiple docked/hovering UAVs, channel caps, misalignment envelopes, load decoupling, inactive-port leakage and fair service while the road lane remains active.
4. **M3 — far-field UAV research:** RF and optical power beaming, each with its own path/acquisition, receiver-mass, spectrum/exposure or laser/interlock and weather/availability evidence. Keep delivered power and range null until measured.
5. **M4 — parked eVTOL:** mission-matched high-power aircraft ground charging and vertiport/grid integration. In-flight eVTOL power beaming remains outside the near-term product assumption.

This order distinguishes the **shared-energy-site problem** from the much harder **in-flight energy-transfer problem**. We can validate the first with M0 while the far-field research remains immature.

### Updated benchmark principle

Every modality must compete on useful mission service, turnaround, payload/receiver mass, availability, energy per completed mission, source-to-endpoint energy, alignment/position tolerance, field/path safety, maintainability and lifecycle cost. For docked aircraft, include conductive charging, robotic contact, battery swapping and near-field IPT/CPT. For airborne charging, compare onboard battery, tethered power, near-field relay/landing opportunities and RF/optical beaming only within a matched mission and safety envelope.

The 7 September 2026 eVTOL charging review in IEEE Transactions on Smart Grid further supports modelling aircraft charging as a mission-driven distribution-network load, including vertiport aggregation, uncertain arrivals and coordinated grid operation (DOI [10.1109/TSG.2026.3731478](https://doi.org/10.1109/TSG.2026.3731478)).


### M0 baseline is conductive air docking, not dual wireless charging

The shared-site allocator's known-truth scenario now uses:
- ground lane: `GROUND_DYNAMIC_INDUCTIVE`;
- air lane: `AIR_DOCK_CONDUCTIVE`.

This is intentional. M0 validates concurrent ground and aircraft energy service behind one PCC/source constraint using independent meter planes. It does **not** demonstrate two wireless links operating concurrently. M1 changes the air channel to IPT or CPT and adds channel-specific electromagnetic/thermal/interference measurements. The schema accepts the conductive aircraft-dock modality for service-allocation studies while retaining separate air-near-field and air-far-field modality values.


### v0 meter-boundary identity contract

The allocator now requires three distinct measurement identities:

- Site source power boundary: `SITE_PCC_GRID_AC_INPUT`.
- Channel input boundary, selected by transfer modality (for this M0 fixture: `GROUND_ROAD_INVERTER_DC_INPUT` versus `AIR_DOCK_CHARGER_AC_INPUT`).
- Useful receiver energy boundary, selected by receiver class (ground-vehicle battery terminal versus aircraft battery terminal).

The allocation quanta are normalized synthetic site-PCC input energy. The useful-energy coefficient maps that site input to the declared receiver endpoint for arithmetic purposes only. The channel-input boundary is a separate typed reference; v0 does not calculate intermediate conversion losses or pretend the shared feeder meter is a channel meter. A future measured adapter must provide actual synchronized readings and uncertainty at each boundary before emitting physical efficiency values.

The evaluator rejects a channel meter boundary inconsistent with the transfer modality and a useful-energy boundary inconsistent with the receiver class. This prevents a vehicle-battery figure, aircraft-battery figure and inverter-input figure from being treated as interchangeable Wh.


### Concurrency continuity metric

The exact report exposes `simultaneous_delivery_slot_count` and a per-slot `simultaneous_ground_air_delivery` Boolean. The frozen fixture achieves 4/4 concurrent slots. This is a synthetic service-continuity target; physical validation must separately measure actual ground and aircraft delivery windows, meter windows, channel availability and transient dropouts.

## 13. Operational autonomy benchmark update — 7 November 2026

The open-access review by Wu et al., *Autonomous Charging and Battery Swapping for Unmanned Aerial Vehicles: A Review*, first published 25 October 2026 (DOI [10.1002/aisy.70584](https://doi.org/10.1002/aisy.70584)), materially changes the technology ordering. The review screened roughly 100 studies but explicitly targets **rotary-wing UAVs**; it excludes fixed-wing UAV charging studies. Its evidence must not be generalized to eVTOL or fixed-wing aircraft without separate data.

### Contact/robotic docking is the near-term reference

The review reports a contact-charging platform that operated across 832 landing poses at greater than 98% efficiency and completed 30/30 outdoor flights. The 832 figure denotes tested landing poses, not 832 independent flights. The review also warns that environmental conditions can reduce landing accuracy and that long-term UAV-specific connector durability evidence is thin. Its ~3,000-cycle connector-degradation reference comes from general electrical connector accelerated testing, not a UAV dock lifetime qualification.

Therefore conductive/robotic docking is a mandatory comparator for M0 and not an inferior fallback by assumption. Compare it on complete service: autonomous approach/landing, docking success, charging, release, contamination tolerance, connector wear, maintenance, and time-to-ready.

### Near-field wireless ranges and gaps

The review's inductive table includes:
- 85.2 W with 89% peak efficiency, with >75% efficiency over a 300×700 mm charging area in one transmitter-array study;
- 175 W / 92.3% at a 5 mm transfer gap, with 200 mm lateral tolerance in one particular geometry;
- 260 W / 91.57% at 2 mm gap, with stable operation over 25 mm X, 40 mm Y and 25° rotation;
- 125 W / 97.66% peak, falling to 85.48% at 100 mm X misalignment;
- 350 W / 85 kHz / 50 mm gap, with 88.6% (two-transmitter) and 87.9% (four-transmitter) peak DC–DC efficiency in the position-adaptive array.

Its CPT table includes one SIMO UAV-swarm prototype reporting up to 709.9 W maximum single-load output, 89.06% peak DC–DC efficiency, less than 10% power drop within 125 mm lateral offset and up to 60° angular misalignment. That is a study-specific single-load maximum within a multi-output architecture; do not report it as proven simultaneous power per aircraft or a general UAV rating.

The differences in gap, alignment, receiver mass, transmitter/receiver electronics, load profile and measurement boundary make these incomparable as a single efficiency ranking. Store each result as a source-bound operating point with its own conditions.

### Far-field power is promising but not yet a generic UAV power supply

The review's listed microwave examples report received power from milliwatts to ~2.28 W under their stated short-range conditions, with low efficiencies. Its laser examples range from tens of milliwatts to around 5 W for several experimental systems; the cited 180 W laser row omits efficiency and range. The review emphasizes beam divergence, pointing, limited receiver area and safety. These values should be treated as representative research examples, not upper bounds for the whole field.

Reach Power's 28 October 2026 announcement reports a 10-hour autonomous RF-beam-powered quadcopter demonstration with machine-to-machine power requests, beam release and reacquisition. Aviation Week reported the announcement the following day. The public materials still do not disclose range, delivered electrical power, frequency, transmitter electrical input or end-to-end efficiency. Classify this as an autonomy/endurance milestone with unreported energy performance—not a measured efficiency comparator.

### Add non-wireless energy replenishment baselines

For an actual UAV mission, wireless charging is not automatically superior. Add:
- contact/robotic dock;
- autonomous battery swapping;
- tethered power for stationary/hovering missions (direct aircraft-bus supply is not battery charging; count the onboard battery as any buffer/reserve);
- power-line perching/replenishment only for specifically approved infrastructure-inspection missions;
- onboard battery-only operation;
- near-field IPT/CPT dock/hover;
- RF/optical beaming.

The review describes power-line perching systems, including a 5 W inductive perching example and a 2024 field system with five outdoor inspection/charging cycles over more than two hours, but notes that autonomous charge initiation/interruption was not fully tested. This is a mission-specific option, not a generic airport/vertiport interface. High-voltage line work remains outside this software workstream and requires the relevant utility and aviation safety approvals.

### Updated demonstrator ladder

- **M0 — shared energy-site service:** dynamic ground IPT plus a separately metered conductive/robotic docked-UAV charger on a shared, constrained PCC. Also compare a tethered-UAV service case for missions that permit stationary operation. This qualifies simultaneous site service and accounting, but is **not** a dual-wireless demonstration.
- **M1 — dual near-field wireless:** replace or supplement the conductive aircraft dock with a separately qualified docked-UAV IPT or CPT channel while ground dynamic IPT continues. This is the first actual simultaneous ground-wireless + air-wireless target.
- **M2 — autonomous multi-UAV service:** landing/hover alignment, two-or-more receivers, departure/reacquisition, per-session service, inactive-port isolation and maintenance while the roadway lane remains active.
- **M3 — far-field research:** RF and optical sessions, each with its own acquisition/path safety, spectrum or laser controls, receiver mass, environment and independent energy meter evidence.
- **M4 — parked eVTOL service:** mission-matched conductive and optional inductive charging integrated with vertiport/grid demand. Do not infer eVTOL charging performance from rotary-wing UAV studies.
- **M5 — airborne eVTOL beaming:** future exploratory work only, requiring an independent power/range/receiver-mass/airworthiness and safety case.

### New evaluation principle: mission replenishment autonomy

Peak link efficiency is only one axis. Each autonomous episode should record:
- approach and alignment success;
- successful authorization and safe start;
- delivered useful endpoint Wh and exact meter planes;
- interruption, abort, re-acquisition and safe-release counts;
- time from arrival to mission-ready departure;
- availability over repeated episodes;
- maintenance burden, connector/coupler wear and cleaning;
- receiver mass and payload lost to the energy interface;
- mission completion and useful work per site energy;
- uncertainty and independent evidence for all reported values.

Report reliability with denominators and confidence intervals. For example, 30/30 successful flights is promising but still a small sample; it is not evidence of a zero-failure rate. Do not turn the proposed 99.99% software-event target into a field-reliability claim.

### Scope limitation of the v0 oracle

The executable allocator remains a shared-source, synthetic scheduling oracle. Its input coefficients are synthetic mappings from site energy to useful endpoint energy; the named intermediate channel meter boundary is identity metadata in v0, not a separately simulated or measured conversion stage. It does not model landing dynamics, contact resistance, coil coupling, CPT fields, RF propagation, laser pointing, queueing, storage, feeder voltage dynamics or aircraft propulsion. Those require the domain owners and later adapters. The v0 exact result is an allocation proof for its discrete assumptions only.
