# HAPS Robust Qualification 001 — persistence, thermal closure and fleet service

**Programme:** AeroWeft / AeroCommons  
**Status:** research benchmark specification v0.1; synthetic-first  
**Parent architecture:** [AERO-HAPS-ENERGY-NETWORK-001 #5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885)  
**Benchmark work:** [AERO-HAPS-ROBUST-QUALIFICATION-001 #5910](https://github.com/Luminous-Dynamics/symtropy/issues/5910)  
**Claim ceiling:** offline research benchmark. No airworthiness, flight, power-beaming or operational claim.

---

## 1. Qualification objective

The question is not simply whether a battery state of charge rises over a typical day. It is whether a configuration-identified aircraft can deliver a declared sensing, communications or relay service over a declared geographic and seasonal envelope while maintaining energy, thermal, structural, reserve, recovery and maintenance constraints—and whether a fleet can keep delivering that service while individual aircraft are unavailable.

Keep three claims separate:

1. **Sortie endurance:** elapsed time for one airframe to remain airborne.
2. **Platform persistence:** repeatable operation over an environmental and seasonal envelope, including degraded modes and recovery.
3. **Service availability:** continuity delivered by a fleet with handover, inspection, maintenance, spares and common-cause risks.

A multi-month design proposition is not a multi-month measured flight. A continuous regional service is not proof that one airframe flew for that same duration. Keep lift physics explicit: fixed-wing solar HAPS are heavier-than-air aircraft that require aerodynamic speed and propulsion for station-keeping; buoyant HAPS use lighter-than-air lift and may be free balloons or propelled airships with different wind-control and energy assumptions. Neither class should be modelled as an ordinary unpowered glider.

## 2. Industry and research baseline

These are source-attributed anchors, not interchangeable performance claims.

| Programme / study | Reported evidence | Comparison boundary |
|---|---|---|
| Airbus / AALTO Zephyr | 67 days, 6 hours and 52 minutes continuous stratospheric flight, completed 28 April 2025; supersedes the earlier 64-day 2022 result | Strong public fixed-wing stratospheric endurance benchmark; one flight, not a fleet availability distribution |
| Mira Aerospace / UAVOS ApusNeo18 | 100 hours 50 minutes, 2–6 November 2026, maximum altitude 14.5 km; UAVOS reports turbulence, dense high-altitude cloud and strong winds; flexible wing adapts via controlled bending and torsional deformation | Four-day HAPS flight and structural/weather-adaptation demonstration; not a multi-week endurance record or comparable mission to Zephyr/Sceye |
| BAE / Prismatic PHASA-35 | Stratospheric trials in 2023 and 2024 above 66,000 ft; solar generation by day and battery operation at night | Altitude trials; months-long operation is a design proposition, not a reported months-long flight |
| BAE / ARIA PHASA-35 programme | September 2026 programme to demonstrate ground-to-HAPS power beaming | Ground transmitter to airborne HAPS receiver; not HAPS-to-UAV transfer |
| U.S. Navy / Skydweller | Publicly reported 73-hour solar-powered unmanned flight in 2025 | Multi-day solar-flight comparator, not a stratospheric HAPS endurance record |
| DRDO lighter-than-air HAPS prototype | October 2026 official report: reached 21 km, remained at 20 km for more than 30 minutes, then descended and was recovered | Altitude-control / recovery milestone for a buoyant HAPS; not a long-endurance or persistent-service result |
| Sceye ST1 buoyant LTA HAPS | Approximately 30-day, nearly 30,000 km stratospheric round trip in Aug–Sep 2026; more than seven days over Japan; SoftBank reported station-keeping radius as small as 5 km during the Japan test | Buoyant free-balloon/LTA platform; not a 30-day station-keeping claim and not comparable to a fixed-wing continuous sortie record |
| Sceye SE2 buoyant HAPS endurance campaign | More than 12 days and 88+ hours at selected areas, station-seeking radius reported as low as 1 km, April 2026 | Separate SE2 campaign; do not merge with ST1 flight data/configuration |
| Sceye / SoftBank 5-year HAPS fleet agreement | Announced 3 November 2026: manufacture, launch and operate a fleet over Japan; most intended for mobile broadband, a subset for stratospheric FSO networking | Commercial deployment plan, not evidence that nationwide fleet service is already operating; FSO is communications, not power transfer |
| Dianovský et al., Drones 2026 | Curved-wing PV model: 6.0% RMS power error and 3.5% daily-energy agreement against ground charging measurements over three clear-sky days spanning a 90-day calendar range | PV-model validation; not a 90-day flight |
| ACS Energy Letters 2026 battery study | Cell energy sweep from 100–1,000 Wh/kg; storage-to-solar-collection transition near 500 Wh/kg in the studied cases; predicted battery fractions of 30–40% of takeoff mass | Model-specific result, not a universal threshold or installed-pack result |
| Stratospheric pouch battery-pack study (Journal of Energy Storage, 2025) | Coupled pack thermal-distribution and mechanical preload model at low temperature / low pressure conditions; paper highlights −65 °C and 5 kPa and cell-to-cell performance variation | Simulation/design study, not an installed HAPS pack qualification; motivates pressure, preload, temperature uniformity and ageing tests |
| Liang et al., Progress in Aerospace Sciences (1 October 2026) | System-level review couples wind, thermal state, propulsion demand, onboard energy, vehicle control, payload and network performance; notes fully coupled multi-day assessments remain less common | Literature synthesis and gap assessment, not a demonstrated platform-performance result |
| Reach Power / PowerLight | Vendor-reported 10-hour remotely powered UAV flight (October 2026) and in-flight laser power delivery to a fielded UAS (April 2026) | Ground-to-air power delivery; not an airborne HAPS transmitter demonstration |
| Optical WPT research | Outdoor optical transfer reported at 15% over 1 km under atmospheric turbulence | Link experiment, not a moving-aircraft HAPS-to-UAV mission |

Primary references:

- [AALTO's 67-day Zephyr record, 1 May 2025](https://www.aaltohaps.com/zephyr-sets-world-record-for-longest-continuous-flight-flying-67-days-in-stratosphere/)
- [UAVOS ApusNeo 100-hour 50-minute flight, 12 November 2026](https://www.uavos.com/apusneo-haps-completes-record-breaking-100-hour-flight-in-european-airspace/)
- [BAE Systems PHASA-35](https://www.baesystems.com/en-us/product/phasa-35)
- [BAE Systems / ARIA power-beaming programme, 15 September 2026](https://www.baesystems.com/en-uk/article/PHASA-35-selected-for-ARIA-backed-power-beaming-project-)
- [U.S. Navy / Skydweller demonstration, 5 December 2025](https://www.usff.navy.mil/Press-Room/News-Stories/Article/4354688/navy-demonstrates-multi-day-solar-uas-flight/)
- [Dianovský et al., 4 July 2026](https://doi.org/10.3390/drones10070510)
- [ACS Energy Letters, 21 August 2026](https://doi.org/10.1021/acsenergylett.6c01616)
- [Reach Power, 28 October 2026](https://reachpower.com/news/wireless-powered-drone-10-hour-flight/)
- [PowerLight, 20 April 2026](https://powerlighttech.com/press-release-powerlight-achieves-industry-first-wireless-power-beaming-to-a-fielded-military-uas-in-flight/)
- [Ochiai et al., optical WPT research](https://doi.org/10.1049/ell2.70379)
- [Solar-UAV energy systems review, April 2026](https://doi.org/10.1016/j.nexres.2026.101404)
- [Near-space motor heat-pipe cooling study, November 2026](https://doi.org/10.1016/j.ast.2026.113202)
- [Sceye / SoftBank five-year HAPS fleet agreement, 3 November 2026](https://sceye.com/press-releases/sceye-softbank-sign-agreement-for-fleet-of-stratospheric-platforms)
- [DRDO HAPS flight trial, Ministry of Defence / PIB, 6 October 2026](https://www.pib.gov.in/PressReleasePage.aspx?PRID=2319551&lang=1&reg=48)
- [Stratospheric pouch battery-pack modelling, Journal of Energy Storage, DOI 10.1016/j.est.2025.117290](https://doi.org/10.1016/j.est.2025.117290)
- [Liang et al., system-level review of persistent high-altitude unmanned airships, 1 October 2026](https://doi.org/10.1016/j.paerosci.2026.101243)
- [Shi and Wu, function-based HAPS authorisation framework under international law, September 2026](https://doi.org/10.1016/j.telpol.2026.103293)
- [Mbarek et al., demand-aware HAPS congestion relief, 28 September 2026](https://doi.org/10.1016/j.comnet.2026.112795)

The 2026 review identifies PV, hybrid storage, thermal regulation, power electronics and autonomous energy control as coupled design variables. The November 2026 motor-cooling study treats heat rejection as a specific near-space design problem. These motivate explicit coupling; neither establishes one universal cooling solution.

## 3. Paradigm shifts

### 3.0 Compare lift classes explicitly

The HAPS tournament must contain at least three distinct platform families:

- **Heavier-than-air fixed wing:** aerodynamic lift; propulsion is required for station-keeping, and the 67-day Zephyr result is the current public fixed-wing endurance comparator.
- **Buoyant free balloon:** buoyancy supports the vehicle; the wind/altitude-control strategy and ability to remain over a service region must be measured rather than inferred from an airship.
- **Buoyant airship:** buoyancy provides lift but propulsion may be available for wind-relative positioning; do not transfer its control/energy assumptions to a free balloon.

Sceye's 2026 ST1 mission is described by Sceye's press release as an unmanned free balloon and by SoftBank as a lighter-than-air HAPS. It completed an approximately 30-day trans-Pacific round trip and was tested over Japan for more than seven days; SoftBank reports a station-keeping radius as small as 5 km. A separate Sceye SE2 Endurance Program report describes a 12-day flight and 88+ hours at selected areas with a station-seeking radius as low as 1 km. Keep the ST1 and SE2 flights separate. [Sceye ST1 return announcement](https://www.prnewswire.com/news-releases/sceyes-stratospheric-platform-re-enters-united-states-after-record-breaking-roundtrip-stratospheric-flight-between-us-and-japan-302873007.html); [SoftBank trial report](https://www.softbank.jp/en/corp/news/press/sbkk/2026/20260902_01/); [Sceye SE2 report](https://sceye.com/press-releases/sceye-completes-historic-12-day-6400-mile-stratospheric-flight-advancing-a-new-layer-of-infrastructure-for-humanity/).

Sceye's product page states 425 Wh/kg for its advanced lithium-sulphur battery, and a months-to-a-year design proposition. Record these as manufacturer claims. They are not independently validated installed-pack figures or demonstrated year-long endurance. [Sceye platform page](https://sceye.com/platform/).

On 3 November 2026 Sceye and SoftBank announced a five-year agreement to manufacture, launch and operate a HAPS fleet over Japan, targeting nationwide coverage; most initial platforms are intended for broadband and a subset for a stratospheric free-space-optical (FSO) network. This is an announced programme, not a deployed nationwide fleet. The FSO network is an optical communications/backhaul concept and must not be treated as proof of laser power transfer. [Sceye / SoftBank fleet agreement](https://sceye.com/press-releases/sceye-softbank-sign-agreement-for-fleet-of-stratospheric-platforms/).

The buoyant platform trade is not "balloons are better" or "fixed wing is better." IEEE Spectrum reports buoyant platforms can carry more solar area, batteries and payload, while fixed-wing platforms have so far flown longer and are easier to steer. Convert these qualitative claims into matched metrics: payload kg and kW, service-area-hours, station-keeping error/energy, solar area, wind envelope, repositioning, launch/recovery burden and lifecycle cost. [IEEE Spectrum, 2026](https://spectrum.ieee.org/high-altitude-platform-station).

### 3.1 Optimize service continuity, not the record flight

Treat an aircraft as one node in a service system. Include launch/recovery, inspection, maintenance, battery replacement, weather holds, spares, handover and common-cause outages. Measure delivered service-hours and coverage continuity separately from sortie duration.

### 3.2 Treat a HAPS as an energy-constrained microgrid

Model generation, storage, prioritized loads, reserve, optional export and waste heat. External demand is a revocable service request: admit it only when the platform can preserve its own flight, thermal and reserve constraints under the declared forecast uncertainty.

### 3.3 Add exergy and thermal closure to the Wh ledger

Electrical energy is high-grade work potential. Conversion losses become heat and may require mass, surface area, conductive paths, heat pipes, radiators or active cooling. A modest energy loss can still create a local hot spot or cooling-mass penalty.

For heat transferred at absolute temperature T against a declared environmental reference T0, the associated heat exergy may be represented as:

\[
B_Q = Q\left(1 - \frac{T_0}{T}\right)
\]

This is not a substitute for a thermal model. It emphasizes that energy quantity alone does not describe energy quality or work potential.

For each motor, inverter, battery, transmitter and receiver, account for heat generation, heat capacity, conduction, radiation, convection assumptions, temperature gradients, derating, thermal-control power and limits. Do not assume rarefied air provides sea-level convective cooling or that cold ambient conditions eliminate local thermal problems.

### 3.4 Co-design energy, structure and aerodynamics

PV, batteries, transmitters, receivers, shielding and heat rejection change installed mass, stiffness, deformation, drag, wiring and maintenance access. Optimize the installed system, not component-level efficiency in isolation. Solar collection is also heading- and wing-geometry-dependent; a flat-plate approximation is a weak default.

### 3.5 Schedule in four dimensions

Solar geometry, latitude, season, altitude, wind layers, coverage footprints and service demand all vary with time. The energy-optimal trajectory may not be coverage-optimal or safest. Preserve these as separate objectives and constraints.

### 3.6 Treat export as a separate aircraft mission

HAPS-to-UAV transfer adds transmitter/receiver mass, alignment, acquisition, interruption, cooling, drag and safety constraints. It must be compared against ground-direct power beaming, battery-only and docking on a matched mission.

### 3.7 Treat the atmosphere as a controllable resource — with platform-specific physics

Wind-layer selection is not a new discovery; it is an established stratospheric-balloon research field. The opportunity is to move beyond minimizing distance from a fixed target and instead co-optimize **service footprint, energy, forecast uncertainty, altitude/position authority and reserve** for each lift class.

Relevant published baselines:

- Liu et al. (2022) modelled time-varying achievable altitude ranges and used real wind fields, thermal/altitude effects and 366-day datasets across locations between 48°N and 48°S to improve super-pressure-balloon station-keeping. [Aerospace Science and Technology, DOI 10.1016/j.ast.2022.107399](https://doi.org/10.1016/j.ast.2022.107399).
- A 2024 seasonal/geographic viability study used ERA5 wind fields and a trajectory search to maximize time within 50 km of a target; it found large regional and seasonal variation, with mid-latitude station-keeping more challenging than lower latitudes. [Scientific Reports, DOI 10.1038/s41598-024-71445-9](https://doi.org/10.1038/s41598-024-71445-9).
- Fan et al. (2024) compared wind forecasts and receding-horizon station-keeping for stratospheric balloons. In that paper's data, 1-hour median wind-speed error was 4.8% for PredRNN, 6.1% for ResNet and 10.0% for persistence; at 4 hours, average errors rose to 25.3%, 26.2% and 33.0%. Their SOO planning comparison beat greedy and A* in their own simulated cases. Those values are paper-specific, not a universal forecast benchmark. [Electronics, DOI 10.3390/electronics13204032](https://doi.org/10.3390/electronics13204032).
- Delgado et al. (2024) formulated a 24-hour sprint-and-drift trajectory for stratospheric airship station-keeping to reduce energy while tolerating a bounded displacement from a ground reference. [Aerospace Science and Technology, DOI 10.1016/j.ast.2024.109365](https://doi.org/10.1016/j.ast.2024.109365).
- A 2025 analysis of 338 altitude bands using more than 250 million wind profiles from 2020–2024 shows that navigational potential depends on the vertical diversity of the wind column and on the accessible altitude envelope; the result argues against treating one fixed stratospheric layer as universally favorable. [Communications Earth & Environment, DOI 10.1038/s43247-025-02526-4](https://doi.org/10.1038/s43247-025-02526-4).

- Bu et al. (2026) STF-RL is a current station-keeping baseline: it consumes an external 12-hour, multi-altitude wind forecast, uses a Transformer with time–height encoding/task-conditioned attention and PPO, and evaluates a mission-level kinematic airship model. Its published setup uses 21 altitude levels across 18–22 km at 200 m spacing, ERA5 at 1-hour / 0.25-degree resolution and 300-second decision steps. It does not generate its own wind forecast. The authors explicitly identify full 6-DOF dynamics, actuator transients, envelope deformation, thermal management, payload modes, battery ageing and emergency operation as omitted; they also call for calibrated ensemble forecast uncertainty and higher-fidelity validation. [Aerospace 2026, DOI 10.3390/aerospace13060551](https://doi.org/10.3390/aerospace13060551).
- Zheng et al. (2026) WOC-SRL combines interpretable weather-optimal outer guidance, decoupled SAC inner-loop tracking and a high-order control-barrier-function QP safety filter. This is a direct safety-control baseline, not a model to be replaced by a plain reward-penalty RL policy. [Engineering Applications of AI, DOI 10.1016/j.engappai.2026.115178](https://doi.org/10.1016/j.engappai.2026.115178).
- Bai et al. (2026) compares TD-MPC with convex optimization for stratospheric-airship station-keeping under nonlinear dynamics, uncertain winds and energy constraints. [IEEE TAES, DOI 10.1109/TAES.2026.3722785](https://doi.org/10.1109/taes.2026.3722785).

The differentiation target must therefore be the *coupling and evidence boundary*, not claiming novelty for multi-altitude wind look-ahead, Transformer RL, MPC or CBF safety in isolation. Compare STF-RL, WOC-SRL, TD-MPC, SOO/receding-horizon and robust/ensemble MPC on identical weather inputs, platform dynamics, training/solver budgets and service metrics.

The candidate-specific control problems are different:

- **Buoyant free balloon:** horizontal motion is primarily wind-driven. Its controller may choose when/where to change altitude to exploit wind-layer differences, subject to actual altitude-control actuator, thermal, gas/ballast, envelope, solar and altitude limits. Do not assign an aircraft propulsion-power model to a free balloon without evidence.
- **Buoyant airship:** may have lateral propulsion plus altitude control. Compare powered station-keeping, wind-aware drift, and sprint/drift policies using the same coverage tolerance and payload.
- **Solar fixed wing:** has aerodynamic lateral control and propulsion; it can trade heading, airspeed, altitude and loiter geometry against solar incidence, drag, wind-relative ground track and propulsion energy. It must not inherit the free-balloon altitude-only steering abstraction.

The Luminous research gap is a **service-aware, robust wind-field planner** that optimizes time-varying coverage and backhaul quality together with propulsion/altitude-control energy, reserve, solar collection and fleet handover. Forecasts should be distributions or ensembles, not a single deterministic wind vector. When forecast uncertainty, data age or wind regime leaves the candidate outside its validated domain, return `NOT_COVERED` / `INDETERMINATE` rather than extrapolating silently.

### 3.8 Use the network as the mission objective, not a passive payload

A HAPS network is not simply a fleet of aircraft carrying radio payloads. Demand selection, beam allocation, platform positioning, backhaul and edge compute can be jointly optimized. Mbarek et al. (Computer Networks, 28 September 2026) combine context-conditioned cell selection with Lyapunov-based HAPS positioning under a maneuver-energy budget; the paper reports up to 64% critical excess traffic removed and 24% throughput gain over stationary deployment on real operator traffic traces. It also identifies a regime change: cell selection matters more under small beam budgets, while platform geometry dominates as the assisted set expands. These are research results on specific traffic traces, not operational commercial fleet figures. [DOI 10.1016/j.comnet.2026.112795](https://doi.org/10.1016/j.comnet.2026.112795).

Multi-platform literature already addresses parts of the problem:
- Multi-airship coverage and connectivity control with distributed algebraic-connectivity estimates, position adjustment and CBF safety filters: [DOI 10.1016/j.conengprac.2025.106544](https://doi.org/10.1016/j.conengprac.2025.106544).
- Dynamic multi-airship communication coverage in time-varying winds reports up to 19.1% reduced wind resistance and 30–76% faster execution than Voronoi methods in its simulations: [DOI 10.1016/j.ast.2025.110514](https://doi.org/10.1016/j.ast.2025.110514).
- Collaborative multi-agent RL coverage planning with revisit scheduling under wind and energy constraints: [DOI 10.1016/j.ast.2025.110736](https://doi.org/10.1016/j.ast.2025.110736).
- Energy-aware hierarchical mission scheduling/path planning/station-keeping reports 90% task completion, 74.5% higher task revenue and +6.85% minimum SoC in one 48-hour simulation: [DOI 10.1016/j.asr.2026.03.105](https://doi.org/10.1016/j.asr.2026.03.105).
- Repairable k-out-of-n reliability using a Semi-Markov model provides a fleet availability/reliability baseline with non-exponential sojourn times: [DOI 10.1016/j.ress.2025.111450](https://doi.org/10.1016/j.ress.2025.111450).

The next integration layer is therefore **demand → cell/beam allocation → platform positioning → topology/backhaul → edge work → service assurance**, with all energy, thermal, reserve, outage and maintenance consequences charged to the same service boundary. The planner should choose not to move an aircraft when beam selection, gateway routing, edge processing or a neighbor node can satisfy demand at lower total system cost. The dedicated workstream is [AERO-HAPS-NET-ORCH-001 #5948](https://github.com/Luminous-Dynamics/symtropy/issues/5948).

### 3.9 Treat onboard compute as a network resource and thermal load

Sceye / SoftBank reported that ST1 hosted a mobile core network and web server onboard the HAPS. Their source reports a **68 ms average round-trip processing response time**, with over 40% lower communications latency than internet-cloud processing, for the demonstrated service path. The same service campaign verified direct-to-device smartphone communications and HAPS-to-drone control, location and video exchange. This is a source-attributed trial result, not a general p95 edge-compute benchmark or a guarantee for other platforms. [Sceye ST1, 9 September 2026](https://sceye.com/press-releases/sceye-completes-record-us-japan-stratospheric-flight/); [SoftBank trial service, 2 September 2026](https://www.softbank.jp/en/corp/news/press/sbkk/2026/20260902_01/).

The benchmark should treat compute placement as a decision variable:
- platform edge vs drone edge vs terrestrial gateway edge vs satellite edge vs remote cloud;
- latency by explicit measurement boundary (processing-only, application round-trip, one-way network, or control-loop round-trip);
- input data rate, request rate, compute demand, placement and fallback path;
- local bytes processed, backhaul bytes avoided, task accuracy/completion and response-time distribution (p50/p95/p99);
- compute electrical power, waste heat, cooling burden and thermal derating added to the HAPS multi-day energy ledger;
- service continuity under gateway/backhaul outage and platform node loss.

The important paradigm shift is to optimize **where a task is processed**, not assume all data must be backhauled or that all compute should sit onboard. Some missions should process video locally and send only alerts/metadata; some should relay raw data to a ground gateway; safety-critical drone commands need a bounded control-loop response and defined fail-safe when the HAPS edge path is lost.

A separate workload-placement benchmark now makes this executable rather than leaving it as a service-profile aspiration: [AERO-HAPS-EDGE-PLACE-001 #5989](https://github.com/Luminous-Dynamics/symtropy/issues/5989), with [schema](schemas/haps_edge_placement_case_v1.schema.json), [synthetic exact-oracle fixture](examples/synthetic_haps_edge_placement_case_v1.json) and [research specification](HAPS_EDGE_PLACEMENT_001.md).

The current synthetic case enumerates 36 assignments for three tasks across HAPS, drone, gateway and cloud nodes; only 11 satisfy all declared deadlines, compute capacities, node power/heat limits and HAPS reserve-facing power/heat/backhaul limits. The exact minimum-system-power assignment uses HAPS for emergency fusion and drone control, with wildfire analytics at the gateway: 93 W total system power, 58 W HAPS incremental power, 49 W HAPS heat and 8 Mbps HAPS backhaul. A lowest-latency feasible assignment uses 113 W total / 73 W HAPS power / 65 W heat. The 17.7% power reduction is a fixture-specific Pareto illustration; its priority-weighted response is higher, and none of the values are hardware measurements. Do not conflate a 68 ms average round-trip trial metric with a p95 control-loop SLA.



## 4. Benchmark scenario matrix

| ID | Scenario | Essential stressors | Primary question |
|---|---|---|---|
| H0 | Reference diurnal closure | Declared 24-hour sun, loads, battery, power limits and reserve | Does the ledger close? |
| H1 | Consecutive-day sequence | Multi-day changing solar and loads | Does SoC recover after each night? |
| H2 | Seasonal/geographic holdout | Solstice/equinox, low solar elevation, altitude and latitude | What is the valid operating envelope? |
| H3 | Wind-aware station-keeping / sprint-drift | Time-varying vertical wind shear, thermal/altitude control limits, forecast ensembles, station-keeping radius, service footprint, propulsion/actuator energy | What coverage can each lift class sustain at what total energy and forecast-regret? |
| H4 | Battery end-of-life | Usable capacity loss, impedance, power fade, imbalance and cold | Does reserve still hold? |
| H5 | Thermal extremes | Solar heating, battery and motor heat, rarefied-air cooling, payload peaks | Can thermal limits be maintained? |
| H6 | Payload duty cycles | Continuous low power and burst communications/imaging/compute | Can payload service be delivered without eroding flight reserve? |
| H7 | Component degradation | PV-string loss, shading, soiling, cell mismatch and drive derating | Can degradation be detected and absorbed? |
| H8 | Export admission | Receiver demand, conversion, pointing, interruption and heat rejection | Is there useful export at the receiver meter? |
| H9 | Fleet/node failure | Node loss, maintenance, recovery, spares, grounding and handover | Does the service survive an unavailable aircraft? |
| H10 | Lift-class comparison | Fixed-wing versus buoyant free balloon versus propelled airship; payload, solar area, wind field, altitude control and launch/recovery | Which lift class delivers the same coverage service with the lowest robust system burden? |
| H11 | Demand-aware network orchestration | Cell demand surge, finite beams, traffic forecast uncertainty, positioning/maneuver energy, backhaul and node loss | Does joint demand/beam/position/fleet scheduling increase useful service per total system Wh without service or safety regressions? |
| H12 | Edge-compute placement and co-scheduling | Application deadlines, p95/p99 tails, platform/ground/drone/space placement, compute power/heat, backhaul outage and platform reserve | Where should each workload execute to meet its deadline with the lowest robust total-system energy and thermal burden? |
| H13 | Fleet common-cause and repair availability | k-out-of-n redundancy, correlated weather grounding, shared gateway/launch/maintenance dependencies, repair duration, spare/recovery logistics, handover | Does the fleet meet its service availability under independent and correlated node/dependency failures? |

Preserve weather autocorrelation, synoptic regime, altitude correlations and geography. Do not shuffle hourly irradiance and wind independently. Hold out entire weather campaigns, seasons or locations. Monte Carlo seeds are not independent weather campaigns. Wind-layer control must also be evaluated against strong passive-drift, fixed-altitude, rule-based/greedy, A*, SOO/receding-horizon and RL baselines with the same forecast data, horizon, actuation limits and compute budget. Forecast error alone is insufficient: report achieved footprint time, p95 target distance, request/coverage outage, altitude transitions and associated actuator/propulsion Wh. Hold out full seasons, geographies and years; report the energy/coverage regret caused by forecast error.

Use model-specific time scales and demonstrate numerical time-step convergence for solar/battery dispatch, battery/motor thermal transients, wind-relative station-keeping, beam acquisition/reacquisition and fleet handover. Do not force one sample rate across all physics.

## Machine-readable cases and arithmetic controls

The versioned [HAPS energy case schema](schemas/haps_energy_case_v1.schema.json) defines a provenance-aware time-series contract. Two fixtures pin down both the positive and negative control:

- [Synthetic day/night surplus-export case](examples/synthetic_haps_daynight_surplus_case_v1.json): 14,076.72 Wh solar generation; 5,040 Wh own-load demand; 4,444.444... Wh battery charge input; 2,520 Wh battery discharge output; 1,500 Wh source-side export; 1,200 Wh receiver-useful energy; 3,200 Wh ending SoC; no own-load deficit; the daily cycle closes.
- [Synthetic rejection of battery-funded export](examples/synthetic_haps_reject_battery_funded_export_v1.json): 1,200 Wh own-load demand served from 1,333.333... Wh stored-energy withdrawal; 2,400 Wh requested receiver energy is unserved; zero source export and zero receiver energy; 1,666.667 Wh ending SoC remains above the 1,000 Wh reserve but below initial SoC, so the daily cycle and export-service admission both fail.
- [Synthetic two-day recovery case](examples/synthetic_haps_two_day_recovery_v1.json): Day 1 ends at 600 Wh from a 1,000 Wh start and does not close its own daily cycle; Day 2 recovers to 1,200 Wh. Across the 48-hour horizon, 5,400 Wh PV generation serves 4,200 Wh own loads, with 2,000 Wh battery charge input, 1,800 Wh battery discharge, 1,000 Wh source-side export and 800 Wh receiver-useful energy. The complete horizon closes with reserve preserved and no ledger residual.

All three fixtures are synthetic arithmetic oracles, not aircraft profiles or measured performance. The interval contiguity, horizon sum, initial SoC/reserve bounds and ledger equations are semantic invariants in addition to JSON Schema validation. The oracle distinguishes `daily_energy_cycle_closed` (every constituent day closes independently) from `horizon_energy_cycle_closed` (the complete evaluation window ends with at least its initial SoC). This distinction permits a low-solar day to be recovered by a later surplus day without pretending that the first day was independently sustainable.

Symthaea PR #12674 now adds `evaluate_horizon` alongside the exact 24-hour `evaluate_day`: it carries battery state between days, splits input intervals at day boundaries, aggregates interval/day receipts, rejects partial-day horizons and admits export only if every day preserves own-service/reserve and the whole horizon closes. Three Rust unit tests cover SOC carry-forward/recovery, interval splitting and partial-day rejection. These tests are authored but **not yet executed**: the PR's draft workflows are skipped and no local Rust toolchain was available. This is an arithmetic horizon evaluator, not weather/season simulation, battery ageing, thermal coupling, buoyancy or flight validation.

## 5. Service-level comparison contract

HAPS communications and observation are area/coverage services, not point-to-point transport missions. Do not force them into the rural-cargo route profile. The new [HAPS service-profile schema](schemas/haps_service_profile_v1.schema.json) defines the common coverage region, device demand, aggregate throughput, latency, service window, altitude/weather envelope, candidate lift families, fleet redundancy, reserve policy, access modes, candidate backhaul modes (including FSO) and matched comparators. The contract keeps direct-to-device access, HAPS-to-drone data relay, edge computing and platform backhaul separate from wireless energy transfer.

The accompanying [synthetic disaster-resilience connectivity profile](examples/synthetic_disaster_resilience_haps_service_v1.json) is deliberately illustrative: 10,000 km², 20,000 target devices, 100 Mbps aggregate throughput, 120 ms one-way latency, 99.5% service availability, 99.9% fleet availability, a 30-day window, 95% coverage and a two-node minimum portfolio with one-node-loss tolerance. The 5% planned-maintenance downtime allowance is per node, not a 5% fleet outage allowance. These values are benchmark fixtures, not a claim about Sceye, Zephyr, SoftBank or an actual Luminous platform.

The fixture compares `solar_haps` and `buoyant_haps` on the same service contract and preserves separate candidate references. Energy cases remain separately versioned; service availability cannot be inferred from a daily Wh ledger. Export is disabled in this service profile. A future energy-export service requires a distinct profile and surplus-only admission after each platform's own reserve and service loads.

The service profile now includes explicit traffic-demand timing, critical-excess priority, beam budget, per-beam capacity, reallocation interval, terrestrial/satellite/FSO backhaul capacity and edge-compute budget. It now also defines workload-level edge placement candidates, input rates, compute TOPS, electrical power, waste-heat fraction, response deadline/measurement basis, criticality and failover policy. The synthetic profile includes emergency-alert fusion, drone control/telemetry and wildfire video analytics (2.55 TOPS, 110 W electrical, 99 W synthetic waste heat in total). These are test values, not a platform power or thermal capability claim. A separate [30-day synthetic traffic-demand trace schema](schemas/haps_traffic_demand_trace_v1.schema.json) and [four-cell, 30-day sample trace](examples/synthetic_haps_disaster_traffic_trace_v1.json) now provide a complete synthetic demand input for the illustrative service window. This trace is synthetic only and is explicitly unavailable as an operational decision-time forecast. Its [HAPS network allocation case schema](schemas/haps_network_allocation_case_v1.schema.json) and [synthetic multi-cell beam-allocation fixture](examples/synthetic_haps_cell_demand_beam_allocation_v1.json) supply a small deterministic control case. The fixture is intentionally distinct from the profile's larger beam budget: it is an allocation-oracle test, not a claim about the throughput of an operating HAPS. The broader orchestration work is [AERO-HAPS-NET-ORCH-001 #5948](https://github.com/Luminous-Dynamics/symtropy/issues/5948).

The allocation work now also has a full 30-day replay and a separate temporal holdout comparison:
- [30-day greedy allocation case](examples/synthetic_haps_disaster_30day_allocation_v1.json): 30 daily intervals × 4 cells, 2 beams × 10 Mbps, 1,018 Mbps-intervals critical excess offered and 548 served. This is a synthetic whole-period screen.
- [15-day dynamic holdout](examples/synthetic_haps_15day_holdout_dynamic_allocation_v1.json) versus [15-day fixed-assignment control](examples/synthetic_haps_15day_holdout_static_allocation_v1.json): first 15 days select the fixed cells (`cell-east`, `cell-north)); last 15 days form the holdout. On identical demand and resources, dynamic daily reassignment serves 274 of 503 Mbps-intervals critical excess; static assignment serves 234. That is a 17.1% relative improvement in this deliberately synthetic fixture. It is a deterministic allocation result, not an operational gain or a comparison against Mbarek et al.'s real operator traces. The holdout cases preserve both relative offsets and original source-trace offsets and use the same 2-beam / 10-Mbps budget.



The service profile references a separate [HAPS wind-navigation case schema](schemas/haps_wind_navigation_case_v1.schema.json) and [synthetic wind-column fixture](examples/synthetic_haps_wind_column_case_v1.json). That fixture carries 36 synthetic wind samples across three ensemble labels, three altitudes and four valid-time offsets; it deliberately has no forecast availability, altitude-control authority, trajectory or energy solution. The contract records data source, coordinate frame, vertical reference, time cadence, forecast initialization/lead-time semantics, ensemble identity and platform-specific control-model references. In particular, ERA5 reanalysis must not be replayed as an operational forecast that was available to a historical controller.

## 5. Qualification gates

| Gate | Evidence | Permitted claim |
|---|---|---|
| Q0 — identity | Aircraft/configuration/mission/environment IDs, unit contracts and input digests | Inputs are identified |
| Q1 — arithmetic oracle | Independent energy fixtures and negative controls | Accounting matches declared arithmetic |
| Q2 — owner-model replay | Solar, wind, battery, thermal and aero outputs with applicability receipts | Model result within declared owner domains |
| Q3 — independent comparison | Matched baselines and held-out data/conditions | Comparative model evidence |
| Q4 — component bench | Calibrated PV, battery, motor/inverter, thermal and receiver measurements | Component result within tested conditions |
| Q5 — integrated ground emulator | Integrated energy/thermal path, receiver emulator, interruption/reacquisition and fault injection | Ground-integrated research result |
| Q6 — controlled flight campaign | Separately approved test plan, instrumentation, safety, recovery and authority | Flight-test result for exact configuration/envelope |
| Q7 — service campaign | Observed operation, maintenance, downtime, handovers and delivered service | Service-availability result for observed period |
| Q8 — jurisdictional operation authority | Applicable aircraft/airspace, operator, spectrum, payload/service, traffic-management, security and contingency permissions for the exact jurisdiction and configuration | Only the specifically authorized scope may be claimed; a spectrum allocation or technical demonstration alone is insufficient |
| Q9 — fleet service reliability | Independent node failure, shared common-cause events, required gateway/backhaul dependencies, repair/maintenance distributions, spares and handover receipts | Availability applies only to the declared service region/window/failure model; independence-only estimates do not support a common-cause resilience claim |

Evidence does not automatically promote between gates. Keep NOT_COVERED, OUT_OF_DOMAIN, INDETERMINATE, FAILED and NOT_RUN distinct. HAPS authorisation is a function-of-operation question, not only a spectrum question: aircraft/airspace category, operator, service, payload, jurisdiction, security and contingency provisions all require applicable authority review. A 2026 international-law study proposes a functional authorisation framework and distinguishes public-interest and commercial pathways; it is research analysis, not a binding authorisation or legal opinion. No flight or service permission is inferred by this benchmark.

### Network and beam-allocation metrics

For HAPS providing communications or data relay, separately report critical excess traffic removed, traffic-weighted and geographic coverage, useful throughput, beam occupancy and useful bits per beam-second, demand-selection precision/recall, cell-level SLA satisfaction, gateway/FSO/satellite backhaul reachability, edge-compute throughput, service handover gaps, algebraic connectivity (lambda2), and service delivered per total system Wh. Record whether the bottleneck is demand selection, radio/beam capacity, platform geometry, backhaul, edge compute, energy, or fleet availability. Do not report only aggregate peak throughput. For each edge workload, report p50/p95/p99 response against the declared measurement basis, task completion/accuracy, data processed locally, backhaul bytes avoided, failover time, compute utilization, platform electrical Wh, waste heat, cooling Wh and thermal-derating effects. Keep processing-only, application round-trip and drone control-loop round-trip latency separate.

## 6. Metrics and definitions

Report a vector. Preserve raw measurements, distributions, uncertainty and missing strata. Never hide a hard failure inside a weighted score.

**Energy and persistence:** PV generation; own-load demand; battery charge input and stored energy; discharge withdrawal and delivered energy; curtailment and export; minimum SoC; reserve margin; own-load unserved Wh; end-of-day/horizon SoC; ledger residual; lower-tail energy margin by held-out weather/season regime; consecutive days within reserve.

**Thermal and exergy:** peak/percentile temperatures; component gradients; time outside limits; heat generated and rejected by component; thermal-control power and mass; heat per useful receiver Wh; exergy destroyed by conversion stages where supported; residual against calibrated thermal measurements.

**Installed-system/aerostructural:** mass of PV, battery, transfer equipment, shielding and cooling; wing loading, drag, stiffness/deformation, aeroelastic margin, control/trim effects, inspection burden and payload lost to the export package.

**Network/fleet:** service-hours, coverage continuity, payload throughput, node/fleet availability over 30/90/365-day windows, maintenance, weather-grounding, spares, handover gap, request-service fraction and p50/p95/p99 delay. For fleet reliability, report single-node vs k-out-of-n availability, common-cause event probabilities and affected-node sets, shared-dependency outages, service retained after one/two node failures, p50/p95/p99 repair and relaunch time, weather correlation by region/altitude, and the optimism gap between independent-only and common-cause-aware estimates.

**Evidence quality:** out-of-domain and NOT_COVERED rates, prediction-interval coverage/width, false-feasible rate, hard-constraint violations, independent sites/seasons/weather campaigns, source rights, calibration and reproducibility.

## 7. Matched baselines

Physical/platform baselines:
1. Solar-electric HAPS without export.
2. Solar-electric HAPS with ground-to-HAPS power support.
3. Ground-to-UAV power beaming.
4. HAPS-to-UAV transfer hypothesis.
5. Battery-only UAV plus ground charging.
6. Docking, conductive/inductive charging and pack exchange.
7. One HAPS versus a rotating fleet.
8. Fixed altitude/loiter versus wind/solar/coverage co-optimized trajectory.

Design/decision baselines:
1. Analytical lower bounds and first-principles sizing.
2. Sequential discipline-by-discipline workflow.
3. Existing owner models composed without adaptive analysis selection.
4. OpenMDAO gradient-based MDO where differentiability and applicability permit.
5. NSGA-II / NSGA-III or another documented evolutionary optimizer.
6. Bayesian/multi-fidelity optimization with calibrated surrogate uncertainty.
7. Robust-feasibility/minimax-regret selection when probability assumptions are unavailable.
8. Symthaea decision-aware analysis/fidelity selection.
9. Repairable k-out-of-n / semi-Markov HAPS availability with explicit non-exponential repair and service states.
10. Common-cause-aware availability model with weather grounding and shared gateway/site dependencies.
11. Discrete-event fleet maintenance/launch/recovery/handover simulation using measured or source-qualified repair distributions.

Edge workload-placement baselines (see #5989):
9. Fixed all-HAPS, all-drone, gateway and cloud placement.
10. Earliest-deadline-first / priority-aware greedy placement.
11. Minimum total system energy with hard deadline, node-capacity, HAPS-power and heat constraints.
12. Exact enumeration / ILP on small cases.
13. MAPPO, MADDPG and HAPS-assisted MATD3 (Nguyen et al. 2026); hybrid-action MADRL with task migration; priority-aware MAPPO + bipartite matching; HAPS–LEO MAPPO/potential-game coordination.

For edge comparisons, freeze task arrivals, candidate nodes, compute capacities, response-basis semantics, node and network power, heat-rejection limits, backhaul, training budget and failure traces. No policy may see future task arrivals or outages. Reproduce the published MATD3/MAPPO/MADDPG comparison before claiming algorithmic superiority.

Freeze mission, geography, weather sequence, payload, reserve, model versions, input digests, compute/test budget and meter boundaries. A method must not receive a better surrogate, more solver calls, broader operating domain or more independent evidence than its comparator. Publish seeds, solver failures, aborted runs, budgets and Pareto fronts.

## 8. Research targets and staged milestones

These are goals, not certification criteria or performance guarantees.

**Immediate software/model goals**
- Independent oracle agreement within the declared numerical tolerance on every synthetic energy case.
- Zero reserve/own-load/thermal false-feasible outcomes in the frozen benchmark corpus.
- Zero double-counting across PV, battery, transmitter, receiver and useful-load meter planes.
- Unsupported conditions return NOT_COVERED / INDETERMINATE, never assumed zero loss.
- 100% source-to-useful-load traceability for any energy-export claim.

**Model-validation goals**
- First reproduce the 2026 curved-wing PV study's validation boundary and published 6.0% RMS / 3.5% daily-energy figures.
- Then seek less than 3% RMS power error and less than 2% daily-energy error on a new independently instrumented, multi-heading, multi-temperature and multi-season holdout. These are stretch goals and only count with an independently frozen test set.
- Calibrate battery and thermal models against measured cells/packs at relevant pressure/temperature; do not infer pack behavior from cell data.
- Report station-keeping energy prediction error by altitude/wind regime before claiming trajectory optimization superiority.

**Edge-compute placement and co-scheduling goals**
- Reproduce Nguyen et al.'s MATD3/MAPPO/MADDPG HAPS-assisted multi-UAV cost comparison under a matched simulation boundary before claiming SOTA.
- At equal task completion and deadline compliance, target at least 10% reduction in total system energy or latency-energy-deadline cost on held-out task traces.
- At equal HAPS incremental Wh and heat headroom, target at least 10% more priority-weighted tasks completed.
- Zero safety-critical control-loop tasks assigned to paths that exceed the validated response limit; link loss or late/stale commands must activate the declared local-safe fallback.
- Zero node compute/power/heat, HAPS incremental power, heat rejection, backhaul or battery-reserve violations in runs labelled feasible.
- Report task-level p50/p95/p99 latency, deadline miss rate by criticality, placement, compute utilization, bytes backhauled/avoided, heat generated/rejected, HAPS Wh and gateway/space-link failover. Do not compare Sceye's source-reported 68 ms average trial response directly with p95/p99 or control-loop deadlines.

**Wind-navigation qualification goals**
- Reproduce the published Fan et al. 2024 forecast baseline on its reported region/split before comparing against it: 1-hour median wind-speed error (4.8% PredRNN / 6.1% ResNet / 10.0% persistence) and 4-hour mean errors (25.3% / 26.2% / 33.0%). These are source-paper values, not universal SOTA.
- Reproduce the ERA5 seasonal/geographic baseline using time within the declared station-keeping radius (including 50 km for direct comparison to the 2024 viability study); then add service-specific radii and coverage footprints.
- On fully held-out geography/year/season and the same weather observations, target at least 10% relative improvement in both 1-hour and 4-hour vector-wind forecast error versus the strongest matched baseline, while maintaining calibrated 90%/95% prediction intervals and reporting worst-regime failures.
- At fixed service coverage, p95 distance-to-footprint and availability, target at least 10% lower total station-keeping energy (including altitude-control/ballast/gas/propulsion actuator energy as appropriate) than the best matched non-adaptive controller. Alternatively, at equal total energy target at least 5 percentage points better coverage-time. Either target must be independently held out and must not trade away reserve or thermal margin.
- Compare passive drift, fixed altitude, rule-based/greedy, A*, SOO/receding-horizon and RL/MPC with identical wind inputs, action limits, forecast horizon, compute budget and failure accounting. Report energy/coverage regret under forecast perturbations; forecast RMSE alone cannot qualify a station-keeping controller.

**Fleet common-cause and repairability goals**
- Reproduce the published repairable k-out-of-n / semi-Markov HAPS reliability baselines where model parameters are available. These methods are established prior work; the Luminous differentiation target is explicit service-level common-cause weather, shared gateway, launch/recovery, maintenance and handover coupling.
- The initial [fleet reliability case schema](schemas/haps_fleet_reliability_case_v1.schema.json) and [synthetic common-cause oracle](examples/synthetic_haps_fleet_common_cause_reliability_v1.json) enumerate 32 scenarios: a three-node k=2 fleet with 0.92 independent node availability, a 6% shared weather-grounding event and a 97% shared gateway dependency.
- That synthetic case yields 83.89% service availability for a single-node baseline, 89.52% for the common-cause-aware k=2 fleet and 95.24% for the naïve independent-only k=2 estimate. The independence-only result overstates the common-cause-aware fleet result by 5.71 percentage points. This is a deliberately simplified static Bernoulli example, not a 30-day observed SLA or operational forecast.
- A matched [weather-diversified variant](examples/synthetic_haps_fleet_diverse_weather_reliability_v1.json) changes only the affected-node set: the same 6% event grounds haps-a but leaves haps-b/c available under the explicit synthetic assumption that their weather exposure is sufficiently diverse while their service footprints overlap. Exact enumeration increases k=2 fleet service availability from 89.52% to 94.45% (+4.93 percentage points). The naive independent-only model remains 95.24%, now overstating the diversified result by 0.79 points. Both cases enumerate 32 scenarios with probability mass 1.0; these are architecture-screening fixtures, not measured geographic wind correlations.
- Do not claim 99.9% service availability unless the exact region, service definition, weather correlation, failure model, repair distributions, maintenance logistics, shared dependencies, confidence bounds and independent campaign data support it.

**Persistence milestones**
- Seven consecutive measured days with independent energy/reserve accounting before a 30-day campaign.
- Thirty measured days before planning a 90-day campaign.
- A verified 90-day stratospheric fixed-wing mission would exceed the current public 67-day Zephyr flight duration. Valid comparison requires matched claim boundaries, complete data, independently reviewed instrumentation and applicable record verification.
- Report 30/90/365-day fleet availability independently of any individual flight record.

**Energy-export milestones**
- Start with non-energized receiver emulation and measured mass/drag/thermal integration.
- Close the source-to-useful-load ledger on a stationary bench before moving-target work.
- Establish acquisition/reacquisition and interruption handling before any separately authorized energized flight test.
- Beat ground-direct transfer only on matched receiver, range, power, availability, safety, infrastructure and lifecycle boundaries.

## 9. Thermodynamic, lifecycle and falsification contract

Account for solar input; electrical work to propulsion, avionics, payload, storage and transfer; battery, motor/inverter, transmitter and receiver losses; thermal-management mass and power; radiative/conductive/convective assumptions; degradation and replacement; and launch/recovery operations.

Lifecycle boundaries must include aircraft production/repair, battery replacements, ground stations, energy source, fleet spares and delivered service. Solar input alone is not a lifecycle-emissions result.

Reject or mark INDETERMINATE when the energy ledger fails to close; a sequence breaches reserve; thermal limits or cooling mass break aircraft closure; wind/solar conditions break station-keeping or coverage; receiver mass/drag/heat erases claimed benefit; the transfer link cannot maintain useful receiver power; or the service relies on an unmodelled single node.

## 10. Ownership and non-duplication

- [Symthaea #7343](https://github.com/Luminous-Dynamics/symthaea/issues/7343): solar HAPS daily energy and persistent-platform owner; [draft PR #12674](https://github.com/Luminous-Dynamics/symthaea/pull/12674) adds a bounded 24-hour surplus-only ledger screen.
- [Symthaea #7646](https://github.com/Luminous-Dynamics/symthaea/issues/7646): aircraft mission energy and reserve.
- [Symtropy #3903](https://github.com/Luminous-Dynamics/symtropy/issues/3903): aircraft battery/capacitor evidence from cell to installed system; HAPS-specific low-temperature/low-pressure profile includes the literature-anchored −65 °C / 5 kPa test point, not a universal operating requirement.
- [Symthaea #11411](https://github.com/Luminous-Dynamics/symthaea/issues/11411): mission electro-thermal feedback, composing existing [ENG-THERM #5681](https://github.com/Luminous-Dynamics/symthaea/issues/5681); HAPS thermal behavior must reuse this owner rather than create a parallel solver.
- [Symtropy #5853 / PR #5856](https://github.com/Luminous-Dynamics/symtropy/pull/5856): whole-aircraft portfolio/MDO.
- [Symtropy #5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885): HAPS energy network and architecture comparison.
- Symtropy #3885 / #4286: aircraft/energy co-design and replenishment.
- Symtropy #3458 / #3448 / #4080 / #4115 / #4157: optical physics, empirical WPT boundaries, session lifecycle, observability and ground-air scheduling.
- Mycelix #3638: component, configuration, inspection and maintenance provenance.
- [Symtropy #5996](https://github.com/Luminous-Dynamics/symtropy/issues/5996): fleet-level k-out-of-n reliability, common-cause weather/gateway events and repair/maintenance service availability; the first exact oracle is explicitly static Bernoulli, not semi-Markov. It now includes matched fully correlated vs weather-diversified synthetic fleet cases.
- [Symtropy #5989](https://github.com/Luminous-Dynamics/symtropy/issues/5989): HAPS workload placement and latency/energy/thermal co-scheduling; it composes this service contract and the existing mission electro-thermal and battery owners.

This document defines benchmark contracts only. It must not create duplicate discipline solvers or self-authorize engineering results.

## 11. Claim ceiling

This supports research specification, source synthesis and offline benchmarking only. It authorizes no laser/RF energization, battery experiment, public-airspace activity, flight-critical control change, airworthiness claim, certification claim or operating permission. Every physical test requires its own reviewed safety, instrumentation, recovery and authority basis.
