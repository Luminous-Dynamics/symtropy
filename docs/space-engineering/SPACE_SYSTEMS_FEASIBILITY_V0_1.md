# SPACE SYSTEMS FEASIBILITY — ANALYTICAL SCREENING v0.1

**Date:** 2026-11-26  
**Implementation:** tools/space-systems-reference/  
**Status:** synthetic analytical screening; Rust tests authored; physical and specialist-model validation not performed.

## 1. Programme architecture

This work composes existing Symtropy and Sol-Atlas programmes rather than creating another simulator:

- Symtropy #4201 — propulsion qualification and matched mission service.
- Symtropy #3954 — interstellar precursor mission architectures.
- Symtropy #3989 — interstellar-medium exposure and uncertainty.
- Symtropy #1972 — demonstrated space-capability reference ledger.
- Symtropy #2161 — orbital foundry and assembly lifecycle.
- Sol-Atlas #106 — read-only terrestrial-to-cislunar logistics projection.
- Sol-Atlas #771 — Earth observability evidence protocol.
- Sol-Atlas #745 — Highveld passive cooling / water constraints.
- New work: Symtropy #7623 (q-drive), Symtropy #7624 (planetary shade and ISRU), Sol-Atlas #979 (regional climate and heat consequences).

## 2. Planetary shade: baseline and gap

Matonti et al. (2026), *Roadmap toward a planetary sunshade for space-based solar geoengineering*, DOI [10.1016/j.asr.2026.04.007](https://doi.org/10.1016/j.asr.2026.04.007), proposes phased precursor missions, manufacturing/assembly, solar-sail transfer and large-scale swarm management in the Sun–Earth/Moon L1 region. The study gives a very broad total-mass order of magnitude (10^8–10^12 kg) because membrane optical properties, areal density and architecture dominate the trade. It identifies autonomous assembly of ultra-thin membranes and very large swarm coordination as immature capabilities.

Relevant demonstrated and technology baselines:
- NASA ACS3 deployed an 80 m² solar sail from a 12U CubeSat on 28 August 2024. NASA's 2026 mission-results record documents deployment challenges and lessons learned: https://ntrs.nasa.gov/citations/20260004973
- ESA Proba-3 demonstrated millimetre-class alignment between two spacecraft approximately 150 m apart for several hours: https://www.esa.int/Enabling_Support/Space_Engineering_Technology/Proba-3_achieves_precise_formation_flying
- NASA's Blue Alchemist / ISRU power work describes regolith-simulant processing into silicon solar cells, aluminium wire, oxygen, iron and slag. NASA TechPort reports TRL 6 for the integrated process technology: https://techport.nasa.gov/projects/146991

These are separate capability baselines. They do not demonstrate lunar field manufacturing of high-performance membranes, fully autonomous L1 assembly, or a million-node sunshade swarm.

NASA's 2026 small-spacecraft propulsion survey also provides a procurement baseline for the Q-drive's electric-thruster stage: JPL MaSMi is listed at 55 mN / 1,920 s / 1,000 W / 3.4 kg and Orbion Aurora at 15.6 mN / 1,280 s / 300 W / 1.5 kg. The report cautions that its small-spacecraft tables do not capture the full technology envelope and that public subsystem performance/maturity data can be incomplete or inconsistent. These are not direct Q-drive rivals; the external-energy harvesting stage, medium drag, high exhaust velocity and mission regime are different. Use them to anchor the onboard electric-thruster part of the architecture only.

### V0 calculation boundary

The first-order idealized relation is:

A_eq = f * pi * R_E^2 / epsilon_a

M = A_eq * sigma_installed

where f is target flux-reduction fraction and epsilon_a is an abstract target-plane attenuation-effectiveness input. Multiplying this equivalent target-plane area by a scenario areal mass density creates a mass proxy only.

The equation is not a physical aperture, L1 sail footprint, installed mass estimate or guaranteed lower bound. The equivalent target-plane attenuation area is a bookkeeping quantity. A physical L1 mass estimate needs finite-Sun optical transfer (umbra/penumbra), diffraction, scattering, module arrangement, sail attitude and orbit dynamics, halo-orbit families, solar-radiation pressure, station-keeping, collision avoidance, degradation, repair and end-of-life. A target-plane area or equivalent-area mass proxy must never be converted directly into a global temperature or wet-bulb change.

### First sensitivity result

For the illustrative 1%/0.90 case, the equivalent target-plane attenuation area is 1.4168 million km². Applying scenario areal mass densities of 1, 10, 100 and 1,000 g/m² produces equivalent-area mass proxies of approximately 1.42 million, 14.17 million, 141.68 million and 1,416.85 million metric tonnes. These are sensitivity values only—not physical L1 membrane areas, installed mass estimates, or guaranteed lower bounds. The finite-distance optical transfer must be established before any density can be mapped to physical membrane mass; all structural, power, control, communications, spares, transfer and repair mass then needs to be accounted separately.

## 3. Q-drive: published concept and open engineering terms

Greason (2019), *A Reaction Drive Powered by External Dynamic Pressure*, JBIS 72(5), pp. 146–152, describes harvesting energy from relative motion through a surrounding medium and using that energy to expel onboard reaction mass: https://tauzero.aero/a-reaction-drive/

The paper states that practical assessment needs explicit conversion efficiency and parasitic drag, detailed mass estimates and treatment of thrust not parallel to drag. Its pulsed plasma-magnet arrangement is a proposed embodiment/sizing study, not a demonstrated spacecraft propulsion system.

Larrouturou, Higgins & Greason (2022), *Dynamic soaring as a means to exceed the solar wind speed*, DOI 10.3389/frspt.2022.1017442, proposes a staged architecture to accelerate toward q-drive operating speeds. It also states that q-drive and wind–pellet shear sailing require pre-accelerating significant reaction mass: https://www.frontiersin.org/journals/space-technologies/articles/10.3389/frspt.2022.1017442/full

### V0 calculation boundary

This crate closes the first-order Newtonian arithmetic chain for both relative speed and exhaust speed beta <= 0.05:
- dynamic-pressure proxy;
- effective harvesting drag;
- harvested mechanical power;
- explicit mechanical-to-bus and bus-to-jet conversion losses with a power-chain residual;
- exhaust kinetic power and ideal thrust;
- parasitic drag;
- net axial force and acceleration.

It does not solve plasma-magnet electrodynamics, collisionless plasma response, relativistic flow, magnetic field mass, superconducting cryogenics, thermal rejection, off-axis control, variable medium density, trajectory or braking. A future 0.05c–0.2c model requires a separate relativistic energy-momentum derivation and independent oracle.

### First sensitivity result

The illustrative input gives 1.347 GW bus power and 0.808 GW jet power, producing 125.41 N ideal thrust, but harvesting plus assumed parasitic drag total about 123.58 N. The nominal net force is only 1.83 N (about 1.48% of the opposing force), with the assumed 0.043c exhaust velocity close to the model's 0.04364c break-even value. This is a deliberately fragile synthetic case. The next Q-drive gate should require a positive lower confidence bound on net acceleration after uncertainty in medium density, interaction coupling, field direction, conversion, thruster efficiency, parasitic drag and full subsystem mass—not simply a nominal positive result.

## 4. Climate and human heat-risk boundary

A planetary shade is a solar-radiation-modification proposal, not greenhouse-gas removal. IPCC AR6 WGII SPM B.5.5 reports that SRM would introduce widespread risks to people and ecosystems that are not well understood, that regional/seasonal residual or overcompensating changes are possible, and that SRM would not reduce atmospheric CO2 or ocean acidification under continued emissions: https://www.ipcc.ch/report/ar6/wg2/

The climate endpoint must be a regional, seasonal, multi-model outcome vector:
- global and regional temperature distributions;
- precipitation, monsoon circulation, soil moisture and drought;
- heatwave duration, humidity, wet-bulb temperature and WBGT;
- population-hours above locally appropriate occupational/public-health thresholds;
- agriculture/ecosystem impacts, ocean heat/sea-level and atmospheric chemistry;
- uncertainty, model disagreement, observational evaluation and termination scenarios.

Wet-bulb temperature and WBGT are not interchangeable. Neither can be inferred from an aperture-area calculation or a global-mean temperature scalar. The Sol-Atlas issue #979 establishes the climate-impact comparison boundary; actual climate response must come from source-pinned, specialist climate models and validated observations.

## 5. Benchmark and SOTA strategy

Do not rank unlike propulsion classes by isolated thrust or specific impulse. Compare accepted mission service at matched:
- vehicle and payload mass;
- initial and terminal state;
- environment and operating envelope;
- arrival velocity / braking requirement;
- time, power, thermal rejection and lifetime;
- installed mass, propellant, maintenance and reliability;
- source quality and evidence maturity.

For the shade, compare effective radiative service per installed kilogram, material throughput per local manufacturing kWh, accepted membrane m² per year, repairability, degradation and swarm fault tolerance.

For the q-drive, compare net acceleration after harvesting and parasitic drag, complete field/power/thruster/reaction-mass mass, and energy/momentum closure. Do not claim superiority over Hall, ion, solar-sail, magnetic-sail or nuclear-electric systems until matched mission cases exist.

### Proposed software gates

- 100% agreement on frozen analytical fixtures with an independently calculated oracle.
- Zero false passes on zero-input, invalid-domain, sign, efficiency and conservation negative controls.
- Deterministic outputs for identical input and code revision.
- All outputs labelled synthetic / analytical with explicit claim ceiling.
- No model promotion without held-out data and independent specialist review.

## 6. Next fidelity ladder

1. Analytic formula checks and dimensional/unit tests (this PR).
2. Source-pinned inputs and uncertainty distributions.
3. Independent orbit/optics and relativistic q-drive derivations.
4. Validated specialist climate, radiative-transfer, plasma and structural solvers.
5. Material and component tests with calibrated instrumentation.
6. Integrated subsystem demonstrations.
7. Flight evidence and independent operational assessment.

A green software workflow would establish only that the analytic code builds and its reference tests pass. It would not establish any physical or climate claim.
