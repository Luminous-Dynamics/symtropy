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
- one actual slot with simultaneous positive ground and air delivery;
- strict domain/modality/receiver identity agreement;
- synthetic-only envelope, authorization, safety and coexistence states.

The bounded exact objective is lexicographic: maximize sessions meeting their minimum service floor; maximize the sorted minimum-to-maximum request-fulfilment ratios (leximin); maximize total useful energy; minimize source energy; then use a stable schedule tie-break. If the node bound is exhausted, the evaluator returns an error and emits no partial optimum.

v0 bounds the scenario to 8 time slots, 8 channels, 8 sessions, 8 power quanta per slot and at most 1,000,000 search nodes. It models one aggregate grid/PCC source; storage, generation, multiple feeders, ramp dynamics and stochastic arrivals remain later increments.

### Fixture arithmetic (not physical performance)

The synthetic fixture has a 3-quanta-per-slot PCC cap, four 60-second slots, an 80 Wh source budget, 10 Wh per input quantum-slot, a 16 Wh ground useful-energy request and a 12 Wh air useful-energy request. Its declared synthetic coefficients permit a 28 Wh total useful-energy allocation while preserving simultaneous service and both minimum floors.

Those values are invented known-truth arithmetic controls. The reported useful-to-source ratio is **not** an experimentally measured conversion efficiency, and the input coefficients are not electromagnetic or aircraft models.

The channel records intentionally declare `DECLARED_SYNTHETIC_SUPPORT`, `NOT_EVALUATED_SYNTHETIC_ONLY` for both safety and coexistence, and `SYNTHETIC_FIXTURE_ONLY` session authorization. The oracle refuses measured/field evidence or any state that could be mistaken for physical permission. It is not an energization controller.

### Test status

Nine Rust unit-test declarations cover the known-truth allocation, row-order determinism, insufficient budget, unresolved safety/coexistence states, domain/modality mismatch, non-overlapping sessions, search-bound exhaustion, energy-coefficient conservation and unknown/non-synthetic input rejection.

No local `cargo`, `rustc` or `rustfmt` is installed in this authoring environment. The source, lockfile and CI wiring are committed, but compilation, formatting and test execution are **not claimed** pending exact-head CI.
