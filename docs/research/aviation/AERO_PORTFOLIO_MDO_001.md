# Aero Portfolio MDO 001 — matched whole-aircraft design trade study

**Programme:** AeroWeft / AeroCommons  
**Status:** research specification v0.1; synthetic-first; no physical aircraft claim  
**Owner:** Symtropy (physical execution receipts and portfolio comparison orchestration)  
**Composed owners:** Symthaea aircraft models and engineering decision policy; Mycelix/AeroCommons evidence identity and manufacturing lineage; Sol-Atlas source-qualified spatial context  
**Parent work:** [AERO-PORTFOLIO-MDO-001 #5853](https://github.com/Luminous-Dynamics/symtropy/issues/5853)  
**Upstream suite:** [AeroWeft Benchmark Suite V1](AEROWEFT_BENCHMARK_SUITE_V1.md)  
**Claim ceiling:** whole-aircraft architecture screening and matched synthetic/model-based trade study. No airworthiness, certification, operating permission, flight safety, or physical performance claim.

---

## 1. Purpose and boundary

This workstream compares electric and hybrid-electric aircraft architectures against the same declared transportation or utility service. Its question is:

> Which aircraft architecture can deliver the required service within a defensible mass, energy, thermal, aerodynamic, structural, operational, infrastructure, environmental and economic envelope—and which next analysis or experiment is most likely to change that decision?

This is an **upstream whole-aircraft design/MDO portfolio layer**. It extends and feeds the existing AeroWeft Benchmark Suite V1; it does not create a second airspace-planning, reachability, traffic-conflict, service-lifecycle, model-credibility, independent-statistics, evidence-composition or mission-chain suite.

The existing aircraft and system owners remain authoritative for their own domain calculations. This specification defines how their exact outputs are assembled into a fair candidate comparison. It must not add another aircraft dynamics model, battery solver, thermal network, CFD solver, route scheduler, optimizer, trust store or universal evidence database.

### Responsibility partition

- **Symthaea:** engineering cognition; requirement decomposition; candidate and analysis proposals; design-space navigation; discrepancy interpretation; decision-relevant experiment selection; explanation and abstention.
- **Symtropy:** deterministic execution orchestration; frozen configurations; geometry/mesh/solver adapters; discipline interfaces; mass/CG/inertia/mission closure receipts; independent comparison fixtures; replay and discrepancy records.
- **Mycelix / AeroCommons:** artifact identity, access/rights, configuration lineage, manufacturing and inspection provenance, multi-party evidence custody and scoped attestations. A signature or consensus outcome does not establish physical truth.
- **Sol-Atlas:** read-only, source-qualified terrain, land, infrastructure, environmental and spatial constraints, with source time, rights and validity metadata.
- **Discipline tools and qualified engineering organizations:** own their calculations, measurement procedures and engineering acceptance.
- **Competent aviation authorities and accountable design organizations:** retain certification, airworthiness, conformity, flight-test and operational-authority decisions.

No output from this benchmark authorizes an aircraft to fly or allows Symthaea to change flight-critical control law.

---

## 2. Paradigm shift: optimize the service system and evidence plan together

A traditional design funnel can over-invest in a candidate before determining whether its limiting uncertainty is worth resolving. A conventional optimizer can also exploit a low-fidelity model's bias and return a nominally excellent but physically infeasible design.

The research target is an adaptive, decision-aware loop:

`requirements
 -> mission and operating domain
 -> candidate architecture + configuration
 -> critical constraints and model applicability
 -> analysis plan (candidate AND fidelity)
 -> deterministic discipline executions
 -> independent receipts and discrepancy update
 -> robust feasible/Pareto set
 -> next decision-relevant analysis or test
 -> reviewed engineering disposition`

The design decision and the evidence-acquisition decision are coupled. A cheap model may screen a candidate only if its applicability domain supports the question being asked. A failed, non-converged, out-of-domain or incomplete execution remains visible in the cost and evidence ledger; it is not silently replaced by a cheap estimate.

### Five design principles

1. **Mission-first, not airframe-first.** Compare the same delivered payload/service, schedule, reserve policy, destination accessibility and ground-transfer assumptions.
2. **Whole-aircraft closure, not isolated component optima.** Mass, CG, inertia, aerodynamic loads, propulsion, energy storage, thermal management, structure, payload and mission must close on one exact configuration.
3. **Robust feasibility before nominal optimality.** A candidate that is attractive at nominal temperature, density altitude, mass or battery condition but violates a hard boundary in the declared envelope is not a feasible Pareto point.
4. **Choose the next experiment, not just the next design.** Allocate engineering and test budgets to uncertainties capable of changing the surviving decision, while preserving a budget for falsifying the working model.
5. **Claims are scoped to evidence.** Synthetic, analytic, model, SIL, HIL, bench, flight-test and authority-accepted evidence remain distinct. A result outside its model/configuration/domain is `NOT_COVERED`, not extrapolated into a pass.

---

## 3. Architecture portfolio

The comparison layer should support multiple architecture families, while allowing a family to be `NOT_APPLICABLE` or `INFEASIBLE` for a particular service. Solar-electric HAPS is a distinct persistent-infrastructure family, not merely an unpowered glider: its solar arrays and batteries supply an electrically propelled station-keeping aircraft. A glide-capable emergency descent mode does not change that classification.

| Family | Typical strength to investigate | Dominant trade-offs to keep visible |
|---|---|---|
| Battery-electric fixed-wing | Efficient wing-borne cruise and regional/rural routes | Runway/strip access, ground transfer, climb and reserve energy, weather, payload volume |
| Electric STOL | Short-field service with wing-borne cruise | Takeoff/landing field conditions, high-lift devices, propeller/wing integration, hot/high performance |
| Multirotor | Hover, precise low-speed positioning, compact landing footprint | Hover power, payload-range trade, rotor redundancy, noise, thermal load and short mission radius |
| Lift-plus-cruise VTOL | Separate vertical lift and efficient cruise propulsors | Lift-system dead mass/drag, transition architecture, failure cases and duplicated propulsion hardware |
| Tilt-wing / tiltrotor | Potential integration of vertical and cruise propulsion | Conversion mechanism, transition envelope, aeroelastic/loads coupling and control complexity |
| Electric helicopter / rotorcraft | Hover and vertical access with a single primary rotor system | Rotor/drive complexity, vibration, rotor efficiency, maintenance and installed power |
| Hybrid-electric fixed-wing or VTOL | Range and reserve flexibility beyond battery-only energy limits | Fuel/electric split, thermal and acoustic impacts, generator mass, dispatch and lifecycle emissions |
| Glider / solar-assisted / high-endurance | Endurance, observation and low-power missions | Weather dependence, payload/power envelope, launch/recovery and energy intermittency |
| Solar-electric fixed-wing HAPS / HALE | Long-duration stratospheric sensing and communications; demonstrated day/night flight, strong repositioning capability | 24-hour energy closure, night storage, propulsion/station-keeping energy, seasonal latitude limits, high-aspect-ratio structure and recovery/maintenance |
| Buoyant HAPS (free balloon / airship) | Buoyancy can support greater envelope, solar-collection and payload capacity; useful for area-focused communications and observation | Wind-relative control, station-keeping authority and energy, gas/pressure management, envelope and solar-skin durability, launch/recovery, and payload-vs-endurance trade-offs. Keep free balloons and propelled airships distinct. |

### 3.1 HAPS platform-class comparison

The `solar_haps` candidate family in this v0.1 specification denotes the solar-electric heavier-than-air / fixed-wing HAPS path. The new `buoyant_haps` family covers lighter-than-air platforms; `lift_class` distinguishes aerodynamic from buoyant lift, and `buoyant_subtype` distinguishes free balloon, airship or unknown. A buoyant free balloon must not silently inherit an airship's propulsion or station-keeping assumptions.

Current source-attributed benchmarks:

- **AALTO Zephyr:** 67 days, 6 hours and 52 minutes continuous fixed-wing stratospheric flight, ending 28 April 2025. AALTO currently describes it as its fixed-wing day/night endurance record.
- **Mira Aerospace / UAVOS ApusNeo18:** 100 hours 50 minutes, 2–6 November 2026, reached a maximum altitude of 14.5 km. UAVOS reports turbulence, high-altitude cloud and strong winds, and a flexible wing adapting through controlled bending/torsional deformation. The facility reports more than 3,000 km, 14,534 m, 18 m wingspan, 12 m length and approximately 45 kg. This is a four-day HAPS/structural-weather-adaptation campaign, not a Zephyr-class multi-week endurance record. [UAVOS flight report](https://www.uavos.com/apusneo-haps-completes-record-breaking-100-hour-flight-in-european-airspace/); [Canarias Stratoport report](https://ptfue.com/en/The-Canarias-Stratoport-at-the-Fuerteventura-Technology-Park-consolidates-its-aerospace-leadership-with-a-new-European-record-for-a-HAPS-flight/).
- **Sceye ST1:** launched 9 August 2026, returned to the United States on 9 September after a roughly 30-day, nearly 30,000 km stratospheric round trip; the Japan trial included more than seven days over the operating region. SoftBank reports a station-keeping radius as small as 5 km during its Japanese operating period. Sceye's release describes ST1 as an unmanned free balloon; SoftBank classifies the platform as lighter-than-air. This is not a 30-day station-keeping claim.
- **Sceye SE2:** a separate April 2026 Endurance Program reported more than 12 days and 88+ hours at selected operating areas, including a station-seeking radius as low as 1 km. Do not merge SE2 and ST1 records into a single aircraft/configuration.
- **Sceye manufacturer claims:** its platform page states a 425 Wh/kg lithium-sulphur battery energy density and a months-to-a-year design proposition. These are not independently validated installed-pack data or a demonstrated year-long flight.

References: [AALTO Zephyr record](https://www.aaltohaps.com/zephyr-sets-world-record-for-longest-continuous-flight-flying-67-days-in-stratosphere/); [Sceye ST1 return announcement](https://www.prnewswire.com/news-releases/sceyes-stratospheric-platform-re-enters-united-states-after-record-breaking-roundtrip-stratospheric-flight-between-us-and-japan-302873007.html); [SoftBank HAPS trial](https://www.softbank.jp/en/corp/news/press/sbkk/2026/20260902_01/); [Sceye SE2 endurance report](https://sceye.com/press-releases/sceye-completes-historic-12-day-6400-mile-stratospheric-flight-advancing-a-new-layer-of-infrastructure-for-humanity/); [Sceye platform claims](https://sceye.com/platform/); [IEEE Spectrum comparison](https://spectrum.ieee.org/high-altitude-platform-station).

On 3 November 2026 Sceye and SoftBank also announced a five-year agreement to manufacture, launch and operate a fleet of HAPS over Japan, targeting nationwide coverage. Most initial platforms are intended for broadband, with a subset intended to establish a stratospheric free-space-optical (FSO) network. This is a commercialization plan, not a report that nationwide fleet operations have begun. The FSO component is a communications/backhaul layer, not wireless energy transfer. [Sceye / SoftBank fleet agreement](https://sceye.com/press-releases/sceye-softbank-sign-agreement-for-fleet-of-stratospheric-platforms/).

Do not rank these flight records as if they share one mission. Compare them on matched service-area-hours, coverage radius and continuity, payload mass/power, repositioning distance, wind/season envelope, energy margin, recovery, maintenance and life-cycle cost. An ST1 trans-Pacific transit and seven-day Japan service window are different metrics from Zephyr's uninterrupted fixed-wing sortie duration.

Do not force a universal winner. A family can dominate for one service and lose for another. Keep vehicle performance, service accessibility and infrastructure/economic burden as separate axes.

---

## 4. Frozen mission contract

Each benchmark case must be a versioned data profile, not a hidden code constant. The first executable mission contract is [aero_mission_profile_v1.schema.json](schemas/aero_mission_profile_v1.schema.json), with [synthetic_rural_medical_logistics_v1.json](examples/synthetic_rural_medical_logistics_v1.json) as a schema fixture. Candidate identity and owner-model references are defined in [aero_aircraft_candidate_v1.schema.json](schemas/aero_aircraft_candidate_v1.schema.json), with three initial conceptual records in [synthetic_three_family_candidate_portfolio_v1.json](examples/synthetic_three_family_candidate_portfolio_v1.json), a separate [conceptual fixed-wing solar-HAPS energy-node record](examples/conceptual_solar_haps_energy_node_v1.json), and a source-bound [Sceye ST1 buoyant-HAPS reference candidate](examples/reference_sceye_st1_buoyant_haps_v1.json). A second source-bound [Mira Aerospace ApusNeo18 heavier-than-air HAPS reference candidate](examples/reference_apusneo18_heavier_than_air_haps_v1.json) records the 100-hour-50-minute flight and flexible-wing source claims while retaining propulsion architecture, installed solar/battery performance, aeroelastic closure and detailed mass properties as unknown. The HAPS records are reference/research candidates, not part of the initial three-family rural-medical-cargo evaluation. New optional candidate fields capture lift class, buoyant subtype and claim-scoped source references. All example values and architecture records are explicitly synthetic; the candidate records intentionally contain no aircraft performance values or closed physical model receipts.

The profile binds:

### 4.1 Service requirements

- service class and intended beneficiary;
- payload or passenger definition, mass, volume, handling and delivery constraints;
- origin/destination or coverage region;
- payload throughput and mission frequency;
- required departure/arrival window and service deadline;
- ground-mode alternative and first/last-mile transfer time/cost;
- mission success definition and permissible cancellation/deferment.

### 4.2 Operating environment

- route geometry and distance, with coordinate reference and vertical datum;
- field/runway/landing-site dimensions, surface and obstacle assumptions;
- altitude and density-altitude profile;
- temperature, wind, turbulence, visibility and precipitation envelope;
- terrain, obstacles, noise-sensitive receptors and ecological constraints;
- operating day/night and lighting profile where relevant;
- source identity, observation time, validity interval, rights and uncertainty for spatial/environmental inputs.

### 4.3 Energy, reserve and recovery

- energy source and exact battery/fuel/capacitor configuration;
- state-of-charge / state-of-health bounds and temperature profile;
- propulsion and thermal-system operating envelope;
- avionics, payload, cabin and auxiliary loads;
- ground charging/refuelling, charge-rate, efficiency, dwell and turnaround assumptions;
- reserve policy and required diversion/recovery set;
- declared component degradation/failure cases;
- energy, power and thermal uncertainty, kept separate unless a justified joint model exists.

### 4.4 Infrastructure and operations

- runway, landing pad, vertiport, taxi, stand, parking and charger capacity;
- maintenance and inspection resources;
- staffing and dispatch assumptions;
- charging power availability and grid/renewable mix;
- finite buffers and blocking-after-service behavior where applicable;
- emergency access and recovery-site availability.

A candidate comparison is not matched if one candidate receives a runway/charger/stand/network for free while another is charged for equivalent infrastructure. Any boundary difference must be explicit and included in the result vector.

---

## 5. Whole-aircraft closure contract

For each candidate, bind a complete configuration identity and the exact artifacts consumed from the domain owners.

Minimum closure dimensions:

1. **Geometry:** wing, fuselage, rotor, propeller, nacelle, control surface, landing gear and payload-bay geometry as applicable.
2. **Mass:** component mass, installation mass, cabling, cooling, structure, occupant/payload and growth allowance; mass properties must not be double-counted.
3. **CG and inertia:** configuration- and loading-specific centre of gravity and inertia tensor, including approved loading cases.
4. **Aerodynamics and loads:** applicable coefficients, trim/flight condition, propulsor interaction, operating envelope, structural/aeroelastic load cases and model applicability.
5. **Propulsion:** motor, inverter, gearbox, rotor/propeller, generator and distribution topology; power, torque, RPM, efficiency and thermal limits by mission phase.
6. **Energy storage:** battery/capacitor/fuel system mass, usable energy, power limits, voltage behavior, aging and temperature limits.
7. **Thermal management:** heat generation, cooling/heating parasitic power, fluid/air flow, pressure drop, mass, temperature gradients and derating.
8. **Mission:** climb, transition, cruise, hover, descent, approach, reserve, diversion and ground energy where applicable.
9. **Maintainability:** access, inspectability, removal/replacement time, tool requirements, repair skill and return-to-service evidence.
10. **Infrastructure/service:** charging, landing, queueing, turnaround, route accessibility and ground transfer.

Every output is bound to the same candidate revision. A changed geometry, component, mass, control profile or energy topology triggers an explicit impact assessment. Superseded evidence is retained and marked superseded; it is not silently reused.

### Closure states

- `CLOSED_FOR_DECLARED_MODEL_SCOPE`
- `OPEN_MASS_OR_CONFIGURATION`
- `OPEN_CG_OR_INERTIA`
- `OPEN_AERO_OR_LOADS`
- `OPEN_PROPULSION_OR_POWER`
- `OPEN_THERMAL_OR_ENERGY`
- `OPEN_MISSION_OR_RESERVE`
- `OUT_OF_DOMAIN`
- `INDETERMINATE`

A closure state is not an airworthiness decision.

---

## 6. Evidence and evaluation receipt

Each discipline evaluation should produce or reference a content-addressed receipt with at least:

- candidate ID, revision and configuration digest;
- mission-profile ID and revision;
- operating-case ID and required-envelope coverage;
- exact geometry, mesh, component, mass-property and model input digests;
- solver/tool name, version, build, settings, precision and adapter revision;
- method/fidelity class and evidence-origin class as separate fields;
- source dataset, parameter provenance, rights and split identity;
- units, coordinate frame, datum and time basis;
- residual, convergence, conservation and solver warnings;
- applicability disposition and reasons;
- uncertainty representation and known correlation/common-cause assumptions;
- raw output digest, derived metric identities and evaluator/verifier identity;
- elapsed time and consumed budget, including failed, aborted, invalid and out-of-domain runs;
- explicit claim ceiling and limitations.

Do not invent a universal fidelity scalar or trust rank. A validated CFD case, an instrumented motor bench, an unvalidated high-fidelity simulation and an analytic bound answer different questions.

Unknown is not zero. Missing models are not cheap evaluations. A failed solver call still consumes budget.

---

## 7. Metrics: a vector, not a magic score

Report candidate-level metrics and uncertainty distributions separately.

### 7.1 Physical and mission

- delivered payload and completed service count;
- payload/range/speed/climb envelope;
- mass, payload fraction, CG and inertia closure residuals;
- wing/rotor/propulsor loading and efficiency over the declared profile;
- takeoff/landing distance or vertical-lift power as applicable;
- mission time and phase-by-phase energy;
- reserve and diversion margin under declared contingencies;
- structural/load margin within model applicability;
- dispatch availability and turnaround time.

### 7.2 Electrical and thermal

- bus, pack, branch, inverter and motor voltage/current/power;
- usable battery energy and power capability over temperature/SOC/age;
- energy consumed by cooling/heating, pumps, fans, compressors and avionics;
- peak temperature, thermal gradients and time above declared limits;
- derating, recharge time and cycle/aging penalty where supported;
- hybrid fuel and electrical energy reported separately;
- energy-balance and measurement-uncertainty closure.

### 7.3 Service, infrastructure and lifecycle

- Wh per payload-km and energy per completed mission;
- cost per successful delivery / payload-km / passenger-km with cost boundary stated;
- charger occupancy, peak power, turnaround and finite-buffer blocking;
- maintenance labour, inspection burden, spares and return-to-service;
- aircraft plus infrastructure lifecycle cost;
- lifecycle emissions and grid/fuel assumptions;
- noise by flight phase and ground receptor;
- service coverage, affordability and rural/remote access.

### 7.4 Evidence quality and decision cost

- hard-constraint violations and false-feasible outcomes;
- OOD / unsupported / abstention rate and uncovered strata;
- prediction-interval coverage, width and tail misses by operating regime;
- model-to-model and model-to-measurement discrepancy;
- independent units and cluster counts (identity-derived, not manually typed);
- evidence maturity and source independence;
- engineer-hours, compute, solver/license cost, specimen cost and elapsed time to decision;
- regret / robust-feasibility uncertainty reduction under matched budget.

No weighted total may hide a hard constraint failure, unmeasured operating stratum, non-converged solver or insufficient independent evidence. Preferences may select among a preserved Pareto set, but must not delete candidates or raw evidence.

---

## 8. Robust feasibility and adaptive evidence acquisition

Use a two-level decision process:

### 8.1 Feasibility first

Classify each candidate/operating-case pair as:

- `FEASIBLE_WITHIN_DECLARED_BOUNDS`
- `INFEASIBLE` (known hard constraint violation)
- `INDETERMINATE` (material evidence absent or contradictory)
- `OUT_OF_DOMAIN`
- `NOT_APPLICABLE`

Hard constraints cannot be compensated by lower cost, less noise or higher throughput. Interval endpoints are not probability distributions. Do not convert a worst-case interval into a probability without an independently justified distribution.

For each robust-feasibility result, retain which constraints bind and their conservative margin. Include correlated uncertainty and common-cause model errors where known; independent marginal intervals may be misleading.

### 8.2 Decide which analysis to run next

Compose Symthaea [AERO-ENGINEERING-LOOP-001 #10902](https://github.com/Luminous-Dynamics/symthaea/issues/10902) and [decision-impact experiment selection #7803](https://github.com/Luminous-Dynamics/symthaea/issues/7803). Select candidate AND analysis fidelity using:

- whether the result could change a hard-feasibility disposition;
- whether it could change the ordering of surviving robust alternatives;
- the decision regret or EVSI only when a calibrated probability model supports it;
- distribution-free worst-case regret reduction where probabilities are not justified;
- total cost to obtain and validate the result, including failed runs and follow-up validation;
- model applicability, common-mode uncertainty and operating-case coverage;
- the remaining budget for higher-fidelity tests and model falsification.

Do not maximize information gain in isolation. High uncertainty in an irrelevant variable can waste a finite test budget. Conversely, reserve some budget to challenge the working model rather than merely sharpen parameters inside its assumed structure. Any reserved-falsification policy is a comparator to test, not an assumed optimum.

---

## 9. Matched baselines and SOTA comparison

Every comparison must freeze initial evidence, candidate set, mission cases, operating-envelope coverage, code/tool versions, total engineering/compute/test budget and evaluator.

### 9.1 Design and optimization baselines

1. Analytical sizing / handbook lower bounds where valid.
2. Sequential discipline-by-discipline workflow.
3. Existing owner-specific Symthaea/Symtropy models composed without the adaptive selection policy.
4. OpenMDAO/Dymos gradient-based MDO where smooth derivatives and model assumptions apply.
5. NSGA-II/NSGA-III or another documented evolutionary multi-objective optimizer.
6. Bayesian and multi-fidelity optimization where probability, surrogate calibration and acquisition assumptions are supported.
7. Symthaea decision-aware adaptive candidate/fidelity selection.
8. Distribution-free robust-feasibility / minimax-regret selection when probability assumptions are unavailable.

Do not claim superiority because one method receives a better surrogate, more candidate evaluations, a more generous compute budget or different physical assumptions. Report all algorithm seeds, budgets, solver failures and frontiers.

### 9.2 Discipline tool references

Use existing tools as independently identified references, not as code to imitate or blindly trust:

- [NASA OpenVSP / VSPAERO](https://www.nasa.gov/reference/openvsp-vspaero-basics/) for parametric geometry and suitable conceptual aerodynamic analysis. NASA notes a major VSPAERO solver transition in OpenVSP 3.45, so pin the exact version and method before comparison.
- [SU2](https://su2code.github.io/docs/home/) for CFD and PDE-constrained aerodynamic shape optimization when mesh, turbulence, convergence and boundary-condition requirements are met.
- [OpenMDAO](https://www.openmdao.org/newdocs/versions/latest/index.html) for coupled multidisciplinary analysis and gradient-based optimization, with gradient-free and mixed-integer options where appropriate.
- [PyBaMM](https://docs.pybamm.org/en/latest/) for electrochemical battery model comparisons (SPM/SPMe/DFN and supported thermal submodels) when chemistry, cell, loading and thermal boundaries are matched.
- PX4 SITL/SIH/HITL and ArduPilot SITL/AutoTest for **flight-stack/controller** comparisons, not whole-aircraft MDO. Pin firmware, vehicle model and simulator; note that current PX4 HITL documentation says HITL is community-maintained and may not work with current versions, while its on-autopilot SIH labels fixed-wing and VTOL profiles experimental. ArduPilot AutoTest provides repeatable SITL regression infrastructure.
- NASA [NEAT](https://www.nasa.gov/eap-labs-and-testbeds/neat/) is a high-value physical-test reference for integrated MW-class electrified powertrains and altitude-representative ground testing; it is a facility/maturity comparator, not a software optimizer or a benchmark score to claim against.

### 9.3 Current research observations to incorporate

- EASA published MOC-5 SC-VTOL Issue 2 on 5 November 2026. The programme should pin the applicable SC-VTOL/MOC edition and aircraft category per study; do not treat a means-of-compliance publication as an aircraft certification.
- An August 2026 mission-level eVTOL electro-thermal study reports thermal-management demand exceeding 20% of total mission energy in its *own study cases*. This is a warning against omitting thermal-system parasitic load, not a universal eVTOL constant or a Luminous result.
- A 15 November 2026 eVTOL embedded battery-thermal-management study reports numerical and experimental investigation with multi-objective optimization. It is a current comparator for battery thermal design; reproduce only where its configuration, cell, test and boundary data are available and comparable.
- Industry flight stacks and MDO platforms already have mature capabilities. The potential contribution here is not “we have an optimizer” or “we have an autopilot”; it is decision-aware evidence acquisition, whole-aircraft closure, exact lineage, robust-feasibility treatment and cross-domain comparison under matched budgets.

References:
- [EASA MOC-5 SC-VTOL Issue 2 (5 Nov 2026)](https://www.easa.europa.eu/en/downloads/144423/en)
- [Mission-level integrated electrical–thermal simulation for eVTOL (Aug 2026)](https://doi.org/10.1016/j.applthermaleng.2026.132253)
- [Embedded eVTOL battery thermal management optimization (15 Nov 2026)](https://doi.org/10.1016/j.ijheatmasstransfer.2026.129090)
- [PX4 HITL guidance](https://docs.px4.io/main/en/simulation/hitl)
- [PX4 SIH vehicle support](https://github.com/PX4/PX4-Autopilot/blob/main/docs/en/sim_sih/index.md)
- [ArduPilot AutoTest](https://docs.ardupilot.org/dev/docs/the-ardupilot-autotest-framework.html)

---

## 10. First bounded synthetic study

Begin with three architecture families, then extend only when the interface is stable:

1. Battery-electric fixed-wing.
2. Electric STOL.
3. Lift-plus-cruise VTOL.

Use one common service case with fixed payload, mission distance, arrival window, reserve policy and environmental scenarios. Keep mode-specific launch/landing and infrastructure differences explicit; include first/last-mile ground transfer. Do not insert assumed aircraft performance numbers where no exact, sourced candidate model exists.

### 10.1 Required synthetic adversarial cases

- A nominally attractive candidate violates a hard mass or energy/reserve limit after closure.
- A low-fidelity aerodynamic estimate is biased near a hard boundary; high-fidelity/analytic truth rejects the candidate.
- Thermal-management power changes the candidate ordering.
- Hot/high conditions make one nominally feasible candidate infeasible or indeterminate.
- A geometry or component revision invalidates downstream mass/aero/thermal evidence.
- Correlated battery/thermal uncertainty invalidates an independence assumption.
- A solver fails, does not converge or is requested outside its applicability domain.
- A candidate lacks required landing/charging infrastructure and cannot meet the service deadline.
- An uncertainty is high but decision-irrelevant; information-gain-only selection wastes the budget.
- A narrow uncertainty changes the robust-feasibility or Pareto ordering; a decision-aware experiment can resolve it.
- No remaining analysis is admissible or decision-relevant; correct output is abstention.
- Input ordering, candidate labels or stakeholder preference changes do not alter raw evaluation receipts or erase the full frontier.

### 10.2 Independent oracle

Create a tiny transparent fixed-wing/eSTOL/VTOL toy problem with rational or exactly representable quantities. Its truth function must not import production selector, mass-closure, frontier or evaluation code.

Required checks:
- exact mass/energy bookkeeping and conservation identities;
- one analytically infeasible candidate whose nominal score is attractive;
- robust intervals and a correlated uncertainty counterexample;
- exact Pareto dominance on the small fixture;
- wrong-configuration and wrong-mission receipt rejection;
- failed / OOD / non-convergent run retention and cost accounting;
- deterministic replay and candidate/permutation invariance;
- preference changes select a different member without mutating the frontier.

The first study is synthetic architecture-screening only. It does not establish a design that is ready for detailed design, physical manufacture or flight.

---

## 11. Proposed internal scorecard and research targets

Targets below are proposals for this benchmark, not regulatory standards or achieved results.

| Dimension | Initial target / gate | Evidence needed |
|---|---|---|
| Toy-oracle correctness | 100% exact agreement | Independent implementation and frozen known-answer corpus |
| Hard feasibility | Zero false-feasible outputs in the bounded confirmatory corpus | Full case denominator, negative controls, hard-constraint oracle |
| Model applicability | 100% explicit OOD / NOT_COVERED / INDETERMINATE where unsupported | Frozen operating-domain manifest |
| Configuration lineage | 100% metric-to-candidate/mission/model/evidence binding | Exact digests and impact graph |
| Data separation | Zero split leakage at required flight/configuration/test-campaign/generator cluster | Identity-derived unit and cluster manifests |
| Thermal/energy fidelity | Pre-registered, regime-specific coverage/error targets | Matched cell/pack/system test data and uncertainty |
| Decision efficiency | Aspirational ≥20% reduction in total engineering + validation cost at non-inferior robust decision quality | Matched budget study against sequential/fixed-fidelity baselines |
| Robust Pareto quality | Preserve all nondominated feasible candidates; report uncertainty and uncovered domains | Independent frontier oracle and confirmatory profiles |
| Runtime | Report p50/p95/p99, memory, solver calls and wall-clock | Exact hardware/toolchain and complete failed-run accounting |

Never report “zero risk” because no failure was observed in a finite corpus. Never pool independent simulation seeds, physical flights, airframes, test campaigns and model families into one denominator.

---

## 12. Integration with AeroWeft and AeroCommons

- Vehicle physics and configuration receipts feed AeroWeft's existing model-credibility and reachable-set tracks only within their declared applicability domains.
- Mission and site/resource outcomes feed existing network scheduling, finite-buffer blocking, recovery portfolio and community/lifecycle tracks. This document does not recalculate those owners' algorithms.
- Mycelix receives content-addressed candidate, manufacturing, inspection and evaluation artifacts with rights and custody metadata. Reputation or consensus cannot promote physical claims.
- Sol-Atlas context remains source-qualified and read-only for this benchmark.
- Model changes, toolchain changes, component substitutions and geometry edits trigger explicit impact analysis and evidence requalification.
- The existing AeroWeft benchmark suite remains the cross-project integration destination.

## 13. Promotion ladder and claim ceiling

1. **Specification:** profile semantics, owner map and test manifests.
2. **Synthetic oracle:** exact bounded arithmetic and adversarial cases.
3. **Offline model comparison:** external discipline solvers, matched input and applicability receipts.
4. **SIL:** flight-stack software with a named simulator/model; remains simulation evidence.
5. **HIL / component bench:** real controller/hardware or instrumented component with calibration and uncertainty; does not establish flight performance.
6. **Integrated powertrain/thermal ground test:** qualified instrumentation, synchronized signals and exact configuration.
7. **Controlled flight-test:** separate test plan, qualified personnel, approvals and independent safety arrangements.
8. **External airworthiness / certification / operational acceptance:** only the competent organization or authority can make the applicable decision.

Evidence stages do not automatically promote one another. Any physical test or flight campaign requires its own engineering, safety, regulatory and operational approvals. This research specification grants none.

---

## 14. Definition of done for v0.1

- [x] Versioned mission-profile JSON Schema and synthetic schema fixture.
- [x] Candidate identity/profile schema and three-family conceptual portfolio with explicit unknowns and owner-work references.
- [ ] Candidate-specific physical model receipts and configuration/closure outputs.
- [x] Solar-electric HAPS family admitted in mission and candidate profile schemas; performance and closure remain unfilled pending owner evidence.
- [ ] Three-family evaluated synthetic comparison: fixed-wing, eSTOL and lift-plus-cruise.
- [ ] Exact whole-aircraft closure inputs/outputs and unknown-field semantics.
- [ ] Independent tiny oracle and adversarial known-answer fixtures.
- [ ] At least three matched optimization baselines under identical candidate/evaluation budgets.
- [ ] Full result vector and Pareto frontier with hard-feasibility gates.
- [ ] Failed, censored, OOD, incomplete and non-converged evaluations retained.
- [ ] Independent-unit and holdout manifests prevent pseudo-replication/leakage.
- [ ] Cross-links to existing AeroWeft owners; no duplicate physics, optimizer, trust or evidence engine.
- [ ] Exact-head tests and independent review before any claim of implementation success.

**Bottom line:** the competitive opportunity is not a new airframe optimizer in isolation. It is a falsifiable, evidence-aware engineering process that jointly selects aircraft architecture, model fidelity and the next most decision-relevant test—while accounting for the complete service and infrastructure the aircraft needs.
