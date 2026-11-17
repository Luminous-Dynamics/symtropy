# Power Fabric: Native Energy Transport Primitives
**Research and architecture note — v0.1**  
**Date:** 2026-11-17  
**Tracking issue:** [Symtropy #5832](https://github.com/Luminous-Dynamics/symtropy/issues/5832)  
**Status:** Proposal and literature-screening baseline; no new physical performance is claimed.

## Executive decision

Power Fabric should become **service-first and multi-vector**, not electricity-first. It should represent the energy carrier, physical transport mechanism, medium, conversion stages, storage, and useful service as separate concepts. A planner should select among qualified paths for a particular duty cycle and location, rather than assume every service should be supplied by electricity or wireless electricity.

The unifying object is an evidence-bound **transport path**. It composes typed modules and transfer edges, but it does not flatten different physics into one generic efficiency field.

The initial programme should:
1. close the wireless coupling gap in the Power Fabric module/service semantics;
2. define a carrier/mechanism taxonomy and cross-vector path contract;
3. benchmark complete source-to-useful-service paths under matched duty cycles;
4. pursue a small number of high-value frontier candidates: direct-to-load wireless conversion, subsea acoustic power, advanced passive thermal transport, and multi-vector industrial/mobile power delivery.

## 1. The conceptual model

Do not encode the following dimensions in one enum or conflate them:

| Dimension | Question | Examples |
|---|---|---|
| Energy vector / carrier | What physical form carries the energy? | Electrical, electromagnetic field, thermal, mechanical, acoustic, chemical potential, electrochemical, gravitational/potential |
| Transport mechanism | How does energy cross the boundary or move along the path? | Conduction, induction, capacitive coupling, radiation, guided wave, advection, phase-change circulation, shaft, hydraulic/pneumatic flow, acoustic wave, physical logistics |
| Medium / path | Through what physical medium? | Conductor, superconducting tape/cable, coil, electrodes, free space, waveguide, solid, liquid, gas, pipe, shaft, road, rail, ship |
| Conversion / transduction | What changes the energy form? | Inverter, rectifier, motor/generator, optical/PV, heat pump, electrolysis, cracking, fuel cell, combustion, compressor/expander |
| Storage | Is energy being accumulated and later released? | Field, battery, thermal, mechanical, chemical, gravitational |
| Useful endpoint | What service must be delivered? | Electrical load, shaft work, traction, heat at a specified temperature, cooling, optical output, process heat, chemical feedstock |

**Critical distinctions**
- HVDC is a network mode/topology for electrical power, not a new energy vector.
- A superconducting cable is an electrical-conduction implementation with cryogenic parasitic loads and specific lifecycle requirements.
- Inductive IPT and capacitive CPT are different near-field transfer mechanisms.
- RF/microwave and optical/laser transfer are distinct radiative lanes.
- A heat pipe is a thermal transport device; a thermochemical store is a storage/conversion system.
- Hydraulic and pneumatic systems transfer mechanical work through pressurized fluid; a thermal fluid network transports heat.
- Hydrogen, ammonia, methanol and LOHC are chemical energy-carrier logistics chains. Their heating value is not equivalent to useful electrical or mechanical output at the receiving boundary.
- Moving a charged battery is storage logistics, not direct energy transfer through a transport edge.

A complete service path is a directed graph of source, conversion, storage, transport, conditioning and useful-load stages. Every edge and conversion has explicit input/output boundaries and any stored-energy change. Physical paths, data/control paths, protection associations and authority relationships remain distinct.

## 2. Candidate transport portfolio

### A. Electrical conduction
AC, DC, HVDC and superconducting cable variants. This remains the reference for many high-power stationary and networked applications. Compare at delivered service after converter, transformer, line, protection, cooling and maintenance costs.

### B. Near-field electromagnetic transfer
Inductive/resonant magnetic coupling and capacitive electric-field coupling. Preserve the existing WPT evidence, metrology, passivity, multi-receiver, dynamic roadway, and robust-dispatch owners. Make coupling a first-class transfer edge, not a fake wire.

### C. Radiative and guided electromagnetic transfer
RF/microwave and optical/laser beaming are distinct safety and physical regimes. Coax, waveguide and other guided electromagnetic paths are not interchangeable with free-space beaming. Require an exact propagation and receiver boundary, and account for pointing, aperture, atmospheric/path loss, conversion and fail-safe shutdown.

### D. Thermal transport
District heating/cooling, industrial heat networks, heat-transfer fluids, solid conduction, heat pipes, loop heat pipes and two-phase circulation. Compare delivered heat by temperature band and time, not thermal kWh alone. Track pumping, pressure drop, insulation, temperature decay, parasitics, fouling, corrosion and maintenance. Heat grade and exergy matter.

### E. Mechanical and fluid-power transport
Shafts, gears, belts, hydraulic circuits and pneumatic networks. Compare net useful shaft/work at the endpoint, including pump/compressor, throttling, leakage, conversion, pipe/line loss, noise, cooling, maintenance and safety. This lane is especially relevant to mines, construction, agricultural machinery and mobile industrial equipment.

### F. Acoustic / ultrasonic transfer
Acoustic waves through a declared fluid or solid medium. Treat underwater ultrasonic power as a specialized subsea/AUV/sensor candidate, not a universal grid technology. Characterize transducer conversion, acoustic path, receiver conversion, pressure/cavitation, multipath, alignment, marine fouling, bioenvironment and safe exposure.

### G. Chemical-carrier transport
Hydrogen, ammonia, methanol, LOHC and synthetic fuels. Model production, compression/liquefaction, conversion, transport, leakage/boil-off, storage, cracking/reforming, purification and end-use conversion. Keep feedstock service distinct from re-electrification. Compare only at matched receiving service and lifecycle boundaries.

### H. Storage and energy-bearing logistics
Batteries, flow-battery electrolytes, thermal stores, chemical stores and mobile storage transport. These can be composed with transport paths but must not be mislabelled as transfer mechanisms.

## 3. Frontier research hypotheses

These are testable hypotheses, not established advantages.

### H1 — direct-to-load wireless energy conversion (existing owner: #3680)
Compose [WPT-DIRECT-CONVERSION-001](https://github.com/Luminous-Dynamics/symtropy/issues/3680), which already owns this research frontier, its EnergyService contract and matched wired / wireless / storage-mediated comparisons. #5832 must integrate its service semantics into the broader transport taxonomy rather than create a second direct-conversion programme. The 2026 *Nature Reviews Electrical Engineering* review identifies wireless energy conversion to mechanical, thermal and optical loads as an emerging direction. Any advantage must include receiver complexity, controllability, efficiency, service quality and safety.

### H2 — subsea acoustic energy service
Explore acoustic/ultrasonic charging for autonomous underwater vehicles and long-lived ocean sensor nodes where a connector-free wet interface has operational value. A 2025 prototype reported 2.5 W transmitter input, 433 mW received power at 35 cm and 17% under its stated conditions. A 2026 study examined a 40 kHz cavitation-constrained acoustic transmit ceiling of 6.76 W under a modeled seawater condition. These are distinct evidence types and must not be combined into one demonstrated performance claim.

### H3 — passive and low-parasitic thermal transport
Screen loop heat pipes, capillary/phase-change approaches, osmotic heat pipes, low-temperature district networks, mine-water and cavern thermal paths. The opportunity is not simply high heat flux: it is useful temperature-grade service with low pumping/parasitic energy, long life, reliable startup/reversal and maintainability.

### H4 — mission-matched multi-vector industrial delivery
For one mine, construction site or mobile fleet, compare electric cable/charging, battery exchange, inductive WPT, direct mechanical drive, hydraulic/pneumatic distribution and process-heat networks against the same work output, route, duty cycle, uptime and site constraints. A hybrid may dominate a single-vector design.

### H5 — superconducting high-capacity links
Compare superconducting AC/DC corridors against conventional AC/HVDC after accounting for refrigeration, cryogenic reliability, fault recovery, access, material availability, lifecycle cost and utilization. A 2026 roadmap assesses AC superconducting cable maturity more highly than DC; do not assume the maturity or economics transfer between use cases.

### H6 — chemical vectors for the right time/distance/service scale
Use chemical carriers where industrial feedstock, shipping, seasonal storage or remote logistics justify them—not as a default substitute for direct electricity. The IEA 2026 review shows a large gap between announced hydrogen pipeline lengths and projects operational or committed, and reports meaningful energy penalties for some shipping/reconversion routes.

## 4. First-class transport path contract

Each admitted path needs:
- stable source/destination asset and port identities;
- carrier, mechanism, medium, topology, flow direction and reversibility;
- exact source, intermediate and useful-load measurement boundaries;
- applicable operating regime: frequency, voltage/current, geometry/gap, pressure/flow, temperature, distance, environment, duty cycle and load;
- delivered power/energy and useful-service quality/grade;
- explicit conversions, storage changes, auxiliary consumption, standby, cooling and loss partitions;
- calibration, uncertainty, clock synchronization, provenance, evidence class and raw/derived trace digests;
- exact asset/configuration, firmware, physical-model profile, solver version and validity envelope;
- protection/isolation, local safe-state behavior, permissions and hazard envelope;
- lifecycle, maintenance, repair/requalification and material evidence where available.

Unknown data remains unknown. Missing energy cannot be set to zero. No path can infer a physical connection from a logical/control association. A signed receipt proves provenance/custody of a statement, not that its physical claim is true.

## 5. Benchmark protocol

There should be no universal leaderboard across unlike carriers. The primary comparison unit is **useful service delivered under a matched task**.

### Primary metrics
1. Source-to-useful-service energy efficiency and exergy efficiency, with all boundaries declared.
2. Useful delivered energy/work/heat by temperature grade, endpoint and time window.
3. Transport loss per distance and per delivered service; throughput, power density and energy density.
4. Range/distance, capture tolerance, latency, ramp/load-following, handover and transition interruption.
5. Availability, capacity factor, outage continuity, degraded operation, recovery time and maintenance interval.
6. Auxiliary energy: cooling/refrigeration, compression, pumping, standby, communication and control.
7. Installed and lifecycle cost per delivered service; embodied/operational carbon and material criticality only where source-backed.
8. Modality-specific safety and environmental limits: electric/magnetic field, optical irradiance, EMC, acoustic exposure/cavitation, pressure, temperature, leakage, toxicity, fire and ecological effects.
9. Uncertainty, calibration, repeatability, model residuals and energy-balance closure.
10. Interoperability, configuration-change impact, repairability and evidence freshness.

### Fairness and evidence rules
- Match useful endpoint, power/energy scale, duty cycle, distance/route, environment, service quality, lifetime and availability requirement.
- Compare end-to-end boundaries; do not rank a converter substage against a full system.
- Keep measured, independently computed, model-derived, simulated, vendor-declared and inferred values separate.
- Freeze hardware/configuration, data split, model/solver version, forecast information, optimization budget and evaluation protocol.
- Include the best conventional baseline, modality-specific state-of-practice baselines, transparent heuristics and exact small-instance oracles.
- Report hard constraints and feasibility before Pareto objectives. Do not hide safety, starvation, service failure or uncertainty in a weighted average.
- Include hostile controls for missing meters, hidden auxiliaries, overlapping/double-counted boundaries, stale configuration, out-of-regime models, unauthorized receiver, loss of communication and false safe-state.
- Compare only within physically compatible lanes, then compare complete architecture portfolios at a common useful-service boundary.

### Proposed qualification gates
- 100% of admitted paths have typed carrier, mechanism, medium, conversions and service boundaries.
- Zero silent carrier conversions, missing-value imputation or incompatible-boundary comparisons.
- 100% of unknown/out-of-domain cases return an explicit unresolved/not-evaluable disposition or a conservative safe fallback.
- 100% of frozen authorization/safety negative controls produce the expected disposition.
- Every calibrated reference rig closes its balance within a modality-specific uncertainty budget. Reuse the existing WPT proposal of ≤0.5% steady and ≤1.0% dynamic residual only where the WPT rig and measurement conditions justify it; do not impose that number on every vector.
- At least three independently sourced reference architectures in each lane before publishing comparative rankings; maintain held-out, source-disjoint cases.
- Publish matched-baseline regret, Pareto fronts, tail/worst-case service and recovery performance, not just best-case efficiency.
- No SOTA/superiority claim from synthetic fixtures, feature counts or simulation-only results.

## 6. Initial execution sequence

**P0 — Semantic contract.** Extend the existing Power Fabric module and service semantics with carrier/mechanism/medium/conversion/service distinctions. Decide explicitly whether the electrical graph remains an electrical subgraph under a broader service composition; do not silently broaden its meaning.

**P1 — Hand-computable fixtures.** Build synthetic path fixtures for conductive AC/DC, inductive WPT, capacitive WPT, RF/optical, thermal loop/heat pipe, shaft/hydraulic/pneumatic and chemical-carrier routes. Each should have explicit source/load boundaries, losses, stored-energy changes, unknowns and hostile controls.

**P2 — First cross-vector demonstrators.** Compose one stationary IPT service and one industrial/mobile multi-vector duty cycle. Add a bounded subsea acoustic fixture and a direct-to-load wireless-conversion case for research screening only.

**P3 — Evidence and deployment feasibility.** Select physical experiments only after hazard analysis, calibrated metrology, qualified components, site permissions and an independent stop/safety plan. The present work authorizes no high-power laser, RF, acoustic, high-voltage, hazardous-gas or high-pressure experiment.

## 7. Ownership and non-duplication

Compose with:
- Symtropy #5159 / #5180 / #5183 / #5607 for Power Fabric contracts, compatibility, source paths and physical-model qualification.
- Symtropy #5241 for industry-matched benchmark methods.
- Symtropy #3396 for EM model selection.
- Symtropy #3448 / #3466 / #3469 / #3472 / #3473 / #3452 / #3458 / #3493 / #3530 / #3546 / #3559 for existing WPT, metrology, optical transfer and dispatch owners.
- Symtropy #1863 / #1877 for thermal reference reproduction, exergy and matched dispatch.
- Symtropy #2083 / #2085 for duty-cycle-led industrial delivery decisions.
- Mycelix #6297 for electrical-equipment identity and lifecycle provenance.
- Sol-Atlas #295 / #301 / #306 / #308 / #332 for energy pathway, port graph, reliability and dispatch.

Do not create a second grid solver, EM/thermal/acoustic/fluid solver, energy ledger, asset registry or decision optimizer.

## 8. Research references

- Li et al., *Wireless energy conversion in wireless energy internet*, Nature Reviews Electrical Engineering (2026-04-02): https://www.nature.com/articles/s44287-026-00279-6
- IEA, *Global Hydrogen Review 2026*, trade and infrastructure: https://www.iea.org/reports/global-hydrogen-review-2026/trade-and-infrastructure
- Liao et al. (2025), underwater ultrasonic WPT prototype, DOI 10.24425/aoa.2025.154821: https://doi.org/10.24425/aoa.2025.154821
- Li et al. (2026), cavitation-constrained underwater ultrasonic WPT, DOI 10.1016/j.apor.2026.105249: https://doi.org/10.1016/j.apor.2026.105249
- 2026 superconducting cable roadmap, Superconductor Science and Technology 39 (2026) 023501: https://snf.ieeecsc.org/files/ieeecsc/2026-02/noe_2026_supercond-sci-technol-39_023501.pdf
- Sorknæs et al. (2026), excess-heat potential in district heating, DOI 10.1016/j.rser.2026.117000: https://doi.org/10.1016/j.rser.2026.117000
- 2026 loop heat pipe review: https://www.sciencedirect.com/science/article/pii/S1359431126035283

---
**Claim ceiling:** This is a research and architecture proposal. It does not establish that any Luminous transport primitive is implemented, physically validated, certified, deployed, or better than state of the art.
