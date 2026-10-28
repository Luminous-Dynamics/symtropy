# Earth Desert and Solar-System Environment Domain Catalog (2026)

**Status:** research taxonomy and benchmark-scoping artifact. It is not a simulator, engineering design, resource assessment, habitat-feasibility result, or certification. The catalogue is intentionally broad but not exhaustive: each entry names a benchmark family that must be parameterized from a source-backed site/body model before quantitative use.

**Core rule:** environmental names are not interchangeable operating envelopes. “Desert,” “Mars,” “lunar polar,” and “Europa” should not be represented by generic temperature/pressure labels alone. The initial state, physical process, deployment mode, timescale, and evidence class are part of the scenario.

## 1. The paradigm shift: benchmark environments as interacting constraints

A single climate or reliability score cannot represent all relevant failure mechanisms. We should describe each scenario as a **versioned environment contract**:

- **Location and mode:** body/region, altitude/depth, surface/orbit/atmospheric-entry/plume sample/underground model, and terrain class.
- **Boundary conditions:** illumination spectrum and geometry, thermal/radiative environment, atmosphere/pressure/composition, gravity, dust/regolith/particles, chemistry/corrosion, available volatiles, communications, and time scale.
- **Dynamic events:** chronology and duration, correlated disturbances, recovery windows, state carried across events, and forecast/telemetry uncertainty.
- **Assets and mission:** device/material constraints, required services, allowable maintenance, resupply, autonomy, latency and power/thermal budget.
- **Evidence basis:** measurement/mission data/model/reanalysis/synthetic stress, source and dataset version, parameter uncertainty, calibration/validation split, and domain-of-applicability.
- **Independent gates:** preserve service floors and domain-specific physical limits. Do not add unlike physical quantities into a single scalar; do not let a good energy-service average establish electrical stability, crew safety, or planetary-protection compliance.

This structure lets one evidence system host diverse domains while avoiding a false claim that one model, score, or controller applies everywhere.

## 2. Earth extreme-environment coverage, including deserts

Deserts must be a first-class benchmark family, not a single “hot/dry” setting. Hot arid, hyper-arid, coastal fog desert, high-altitude desert, cold desert, and evaporitic/salt-flat sites stress different equipment and resource assumptions. The mission of a desert test is not merely to raise ambient temperature: compound heat, water limitation, dust, salt, maintenance access, and prolonged cloudy/dusty periods should affect system state over time.

| Scenario family | Conditions to model | Coupled failure hypotheses | Resource/service gates |
|---|---|---|---|
| earth.hot_arid_sahara_arabian | High ambient/ground temperature, large diurnal thermal cycling, intense solar/UV, low precipitation, wind-blown sand/dust | PV/inverter derating plus battery thermal constraints; dust deposition during heat peak; water and cooling demand rises during low generation | Water service, cooling/heat safety where applicable, power quality, storage temperature, cleaning water, spare/repair burden |
| earth.hyperarid_atacama | Extremely low precipitation, high solar/UV, dust, salts; selected high-altitude locations add low air density and cold nights | Water-source uncertainty plus corrosion/abrasion; altitude-related cooling/combustion performance where relevant; solar abundance does not remove storage/water constraints | Water quantity/quality, power availability, thermal limits, equipment derating and maintenance interval |
| earth.coastal_desert_namib | Hyper-aridity alongside coastal fog/marine aerosols, sand and salt deposition | Salt corrosion plus abrasive dust; fog capture varies and must not be treated as guaranteed water supply | Water yield uncertainty, corrosion/degradation, service continuity, maintenance burden |
| earth.cold_desert_gobi | Cold winters, hot summers, low precipitation, wind-blown dust/sand, freeze–thaw cycles | Cold batteries during low-generation events; dust ingress combined with cold start; fuel line/valve/thermal failures | Critical load service, storage health, fuel reliability, cold-start/recovery |
| earth.high_altitude_dry_plateau | Low ambient pressure/density, strong UV, high solar, large day-night temperature swing, low water availability | Cooling/combustion assumptions fail; battery thermal stress; water treatment/freezing | Derating-corrected capacity, cooling, water and maintenance logistics |
| earth.salt_flat_evaporitic | High salinity/salt dust, reflective ground in some sites, episodic wetting, abrasive deposits | Corrosion, electrical tracking/insulation contamination, sensor and connector degradation | Electrical isolation, corrosion life, inspection interval, service continuity |
| earth.desert_dust_storm | Time-varying aerosol loading, visibility, wind, deposition, solar attenuation and dust ingress | Multi-day PV yield reduction combined with filter clogging, sensor bias, cooling degradation and transport delay | Reserve depletion, sensor validity, filtration/maintenance, critical-service shortfall |
| earth.heatwave_drought | Multi-day/night heat, high cooling demand, drought-related water restrictions, grid/utility stress | Peak demand occurs as assets derate; water and power availability fail together | Absolute service floors, reserve margin, water budget, temperature limits, recovery time |
| earth.mountain_cold_dry | Low pressure, high UV, snow/ice, strong wind and terrain/road access constraints | Supply-chain delay plus cold-start; local generation may be abundant but storage inaccessible | Critical services, transport/access, maintenance response time, cold reserve |
| earth.tropical_hot_humid | High wet-bulb temperature, humidity, corrosion, biological growth, heavy rain | Cooling demand plus wet electrical faults and corrosion; heat stress persists overnight | Thermal safety, insulation, water, power quality, maintainability |
| earth.monsoon_flood_cyclone | Extreme rain, flooding, wind, water ingress, debris, grid/fuel disruption | Flooding and communications loss plus resupply failure | Life-safety services, flood elevation, islanding, restoration, access |
| earth.coastal_storm_salt | Wind, salt spray, surge, humidity, corrosion and access interruption | Salt contamination plus surge and grid outage | Ingress protection, corrosion, critical services, spare parts |
| earth.polar_station_mobile_ice | Cold, low solar season/polar night, wind/icing, isolation, snow/ice access limits | Energy scarcity plus derating, fuel delay and repair queue; moving sea ice requires different boundary assumptions from fixed land | Heat, water, sanitation, communications, autonomy/endurance and recovery |
| earth.wildfire_smoke_drought | Heat, smoke/particulate, fire exposure, dry soils, evacuation/access constraints | PV attenuation, filter clogging, line/fuel damage, loss of communication | Evacuation/life safety, service continuity, air filtration, repair and shutdown decisions |

**Desert-specific benchmarks to add first:** (1) dust deposition plus hot thermal derating; (2) multi-day dust storm plus water scarcity and delayed cleaning/resupply; (3) cold desert night plus depleted battery and cold start; (4) coastal salt/fog plus dust ingress; (5) high-altitude derating plus heat/UV; (6) heatwave with high cooling load and externally constrained water. Use measured weather/solar/wind/dust/water time series when available; where they are synthetic, keep that label and do not infer event probabilities.

The Atacama, Namib, Sahara/Arabian, and Gobi examples are **site archetypes**, not fixed climate distributions. Regional station data, elevation, season, equipment siting and historical extremes must parameterize each experiment.

## 3. Solar-System environment catalogue

NASA's planetary fact and mission pages establish why body-level profiles must be distinct: Mercury has large day/night thermal extremes and a thin exosphere; the lunar south pole has complex illumination and permanently shadowed cold traps; Mars is a cold, dusty world with a thin atmosphere; Venus' surface is around 467 °C and roughly 92 bar; Titan has a dense nitrogen/methane atmosphere, hydrocarbon liquids and a surface near 94 K; Europa is exposed to intense Jovian radiation while evidence points to a subsurface ocean; and the gas/ice giants have no ordinary solid surface to which a terrestrial microgrid model can be applied. See the [NASA Planetary Fact Sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/), [Moon Base environment](https://www.nasa.gov/reference/moonbase-environment/), [Mars facts](https://science.nasa.gov/mars/facts/), [Mercury facts](https://science.nasa.gov/mercury/facts/), [Venus facts](https://science.nasa.gov/venus/venus-facts/), [Europa facts](https://science.nasa.gov/jupiter/jupiter-moons/europa/europa-facts/), [Titan facts](https://science.nasa.gov/saturn/moons/titan/facts/), and [Neptune facts](https://science.nasa.gov/neptune/neptune-facts/).

### 3.1 Cross-cutting space environments

| Family | Must test | Common modeling mistake to avoid |
|---|---|---|
| space.vacuum_thermal_cycling | Radiative heat rejection, eclipse/illumination, thermal inertia, cold soak, material outgassing | Applying terrestrial convection or ambient-air cooling in vacuum |
| space.solar_spectrum_particle_events | Distance/geometry-dependent illumination, UV, solar particle events, accumulated dose, transient radiation | Treating mean solar flux as the complete radiation risk |
| space.galactic_cosmic_rays | Mission-duration dose, shielding/material effects and uncertainties | Equating a short exposure test with lifetime dose |
| space.micrometeoroid_debris | Impact probability model, shielding and loss-of-function modes for the mission orbit | Reusing a surface dust model for orbital impact hazards |
| space.radiation_sensitive_electronics | Single-event effects, latch-up/upsets, degradation, redundancy and recovery | Reporting only average dose while ignoring rare critical events |
| space.delayed_disconnected_comms | Light-time, outages, asymmetric latency, contact windows, autonomous decision and safe state | Assuming cloud control or immediate human intervention is available |
| space.low_gravity_mobility_anchoring | Traction, excavation, reaction force, dust lofting, anchoring, landing/anchoring, tool recoil | Reusing Earth traction/load models without gravity and regolith changes |
| space.cryogenic_fluid_materials | Phase changes, seals, lubricants, valves, fuel/propellant management and thermal contraction | Assuming Earth fluid/material property tables remain valid |
| space.planetary_protection | Forward contamination, sample integrity, clean/dirty zones, sterilization and biological measurement | Treating contamination compliance as an energy score or an optional controller metric |
| space.long_duration_autonomy | Maintenance, part degradation, fault isolation, spare inventory, crew/robot work and recovery without resupply | Resetting wear, inventory, memory or accumulated damage between test episodes |

### 3.2 Body- and mission-specific profiles

Each profile is an independent scenario family. Some locations have strong measurements; others have only flyby data or model-dependent boundaries. Do not label inferred subsurface ocean properties or a proposed habitat as confirmed operational ground truth.

| Scenario ID | Operating envelope / stressors | Required model emphasis | Evidence caution |
|---|---|---|---|
| ss.mercury_sunlit | High incident solar power; extreme hot surface; near-vacuum/exosphere; strong thermal radiation and UV | Heat flux, radiative cooling, thermal cycling, solar particle/electronics | Site/terrain and surface thermal inertia matter |
| ss.mercury_polar_shadow | Permanently shadowed craters may preserve water ice beside intensely heated terrain | Sharp illumination boundaries, cryogenic traps, mobility/access, thermal interfaces | Volatile occurrence and resource grade need site data |
| ss.venus_surface | Very high temperature and pressure, CO₂-dominated atmosphere, reactive trace chemistry | Pressure vessels, high-temperature electronics/materials, chemical degradation, limited mission duration | Not a conventional human-habitat or microgrid deployment scenario |
| ss.venus_cloud_layer | Altitude-dependent temperature/pressure, sulfuric-acid aerosol chemistry, atmospheric circulation | Altitude-resolved chemistry/material compatibility, aerostat/flight dynamics, power and communications | A cloud-layer platform is a distinct hypothetical vehicle problem, not a surface case |
| ss.moon_equatorial | Vacuum, low gravity, abrasive regolith, strong surface thermal swings, radiation and micrometeoroids | Dust ingress, thermal design, excavation/anchoring, eclipse/night endurance | Use local latitude, terrain and illumination time series |
| ss.moon_south_pole_illuminated_ridge | Low-angle Sun, rugged terrain, long but location-dependent light intervals, long shadows | Terrain-resolved illumination, shadow cast by infrastructure, mobility, solar/storage scheduling | Do not assume permanent sunshine; local topography changes availability |
| ss.moon_permanently_shadowed_region | Persistent darkness, very low temperatures, volatile-bearing regolith possibilities, difficult access | Cryogenic mobility, volatile sampling, thermal isolation, power/communications relays | Ice concentration is heterogeneous and requires measurements |
| ss.mars_equatorial_surface | Thin CO₂ atmosphere, cold range and large diurnal change, lower solar irradiance than Earth, dust, radiation | Dust effect on solar, heat rejection, pressure, thermal mass, life-support/resource model | Dust/sol/season and site altitude must be parameterized |
| ss.mars_global_dust_storm | Regional/global dust suspension, sunlight attenuation, sensor contamination, prolonged power penalty | Dust optical depth dynamics, array cleaning, reserve endurance, autonomy and fallback | Build event chronology from observations/models; avoid an arbitrary fixed derating |
| ss.mars_polar_seasonal_ice | Seasonal CO₂/water-ice processes, very low temperatures, seasonal illumination and dust | Sublimation/condensation heat and mass transfer, ice stability, seasonal storage | Separate north/south and season/site |
| ss.mars_highland | Lower pressure, high UV/radiation, terrain and thermal constraints | Altitude pressure effects, lander/vehicle performance, local power and thermal | Exact site elevation and atmosphere are model inputs |
| ss.phobos_deimos_surface | Microgravity, low gravity anchoring, dust/regolith and thermal cycles | Contact mechanics, excavation, escape velocity, reaction forces, proximity operations | Sparse direct surface data; uncertainty must be explicit |
| ss.asteroid_rubble_pile | Irregular shape, microgravity, coarse/boulder-rich regolith, rapid thermal cycling and weak gravity | Contact dynamics, anchoring/sample acquisition, thermal model and dust/fragment response | Do not assume smooth granular soil; Bennu data revealed rugged, weak surface layers |
| ss.comet_active_nucleus | High-vacuum icy body with solar-driven sublimation, gas/dust jets and time-varying activity | Outgassing, particle impacts, attitude/trajectory coupling, thermal state and volatile loss | Activity varies with orbit and illumination |
| ss.ceres_surface | Low solar flux relative to Earth, cold regolith, salts/ice and possible brine-related geology | Thermal/volatile stability, excavation, long power budget | Resource abundance and accessibility are not established by a generic body label |
| ss.jupiter_orbital_radiation | Strong planetary magnetosphere and energetic particle environment varying by orbit | Shielding, accumulated dose, single-event effects, safe modes and autonomy | Dose is orbit/time/spacecraft dependent |
| ss.jupiter_atmospheric_entry | Gas giant with no ordinary solid surface; descent encounters changing pressure, temperature and winds | Entry/descent thermal protection, pressure-varying atmosphere and telemetry window | Define probe trajectory/altitude; never assign a surface habitat contract |
| ss.io_surface | Intense volcanism, tidal deformation, sulfur compounds/SO₂ and strong radiation/plasma | Volcanic/thermal events, deposition chemistry, radiation-hardened hardware | Rapidly changing surface and limited direct observations require uncertainty bounds |
| ss.europa_surface_orbit | Icy surface, intense Jovian radiation, vacuum/thermal cycling; possible subsurface ocean and fractures | Radiation dose, ice/volatile material behavior, sampling autonomy, planetary protection | Subsurface ocean is strongly supported but not directly sampled; habitability is not proof of life |
| ss.ganymede_surface_orbit | Cold icy terrain, tenuous exosphere, own magnetosphere inside Jovian system | Charged particle environment, ice/regolith mobility, thermal and comms | Distinguish Ganymede's local magnetic field from Jupiter's field |
| ss.callisto_surface_orbit | Cold, ancient heavily cratered icy/rocky terrain, tenuous exosphere, distance from Jupiter | Low-gravity mobility, deep-crater access, thermal power, orbital radiation model | Subsurface ocean remains model/inference dependent |
| ss.saturn_orbital_rings | Ring particles, low solar flux, long seasonal cycles, radiation and navigation constraints | Debris/impact environment, orbital operations, power/thermal and comms | Orbit-specific particle and radiation distributions required |
| ss.saturn_atmospheric_entry | Gas giant without ordinary solid surface; descending pressure/temperature and wind envelope | Entry thermal protection, changing pressure, structural/telemetry limits | Must define altitude/depth and mission duration |
| ss.enceladus_surface_plume | Icy/cold terrain; water-ice/vapor plumes with salts/organics; potential subsurface ocean | Cryogenic collection, plume sampling trajectory, contamination control, plume variability | Plume composition varies; protect samples and planetary-protection constraints |
| ss.titan_surface | About 94 K at surface, nitrogen-rich dense atmosphere, methane/ethane liquids, organic haze, very weak sunlight | Cryogenic materials/fluids, hydrocarbon-compatible seals, atmospheric flight/rotorcraft, long energy budget | Do not substitute terrestrial liquid water chemistry or room-temperature assumptions |
| ss.titan_lakes_coast | Hydrocarbon seas/rivers/lakes, shoreline terrain and weathering | Liquid methane/ethane fluid properties, shore mobility, sampling and chemical compatibility | Site-specific bathymetry/liquid state uncertain |
| ss.uranus_atmospheric_level | Ice giant without ordinary solid surface; very low temperatures, high winds and unusual magnetosphere | Pressure-depth profile, wind/entry dynamics, communications and power/thermal | State the reference pressure/altitude; “surface temperature” is misleading |
| ss.neptune_atmospheric_level | Ice giant without ordinary solid surface; very strong winds, cold, pressure increasing with depth | High-speed atmospheric dynamics, entry/descent, limited power and long communications latency | Must define atmospheric level, not a fictional ground site |
| ss.triton_surface | Extreme cold, nitrogen frost/tenuous atmosphere, seasonal energy, geyser-like activity seen by Voyager 2 | Cryogenic volatiles, surface frost and plume sampling, thermal and mobility | Very limited direct observations; mark inferred process uncertainty |
| ss.pluto_charon_system | Cryogenic ices, extremely low insolation, tenuous/seasonal Pluto atmosphere, long orbital seasons | Long-duration power, thermal stability, volatile phase transitions, deep-space comms | Pluto and Charon require separate body models |
| ss.kuiper_belt_small_body | Very low sunlight, cold/vacuum, weak gravity, long communications delays, uncertain geology | Radioisotope/alternative power assumptions, thermal design, autonomous operations | Generic profile for trade studies only; each target needs a specific ephemeris and body data |

### 3.3 Do not model an entire body as a single point

For gas/ice giants, specify an atmospheric pressure/depth level and mission type (orbiter, descent probe, balloon/float concept). For icy moons, distinguish orbit, surface, plume and inferred subsurface ocean. For Mercury and the Moon, distinguish sunlit terrain from permanent shadow. For Mars, distinguish equatorial, highland, polar and dust-storm cases. These are physically different experiments, not optional labels.

## 4. Solar-system benchmark dimensions

Each profile should declare which of these are measured, derived, modeled, synthetic, or not applicable:

1. **Illumination/energy:** irradiance spectrum and geometry, eclipses/shadows, solar distance, dust/haze attenuation, solar-array degradation, alternative power availability.
2. **Thermal:** environmental boundary temperature, absorbed/emitted radiation, conduction/contact, internal waste heat, thermal cycling rate and extreme duration.
3. **Pressure/atmosphere:** gas composition, density/pressure versus altitude, winds, convection regime, reactive aerosols, outgassing, atmospheric entry heating.
4. **Radiation/plasma:** UV, solar particle events, galactic cosmic rays, trapped particles, charge accumulation, single-event effects and lifetime dose.
5. **Dust/regolith/particles:** grain size/shape/abrasion, electrostatic charging, adhesion, deposition, dust storms, plume grains, micrometeoroids and orbital debris.
6. **Gravity/terrain/contact:** gravity field, slope, roughness, bearing strength, traction, drilling reaction forces, anchoring, launch/landing and mobility risk.
7. **Chemistry/materials:** oxidants/salts/perchlorates, sulfuric-acid aerosols, CO₂/SO₂, hydrocarbon fluids, cryogenic phase changes, seals, lubricants and corrosion.
8. **Resources/ISRU:** water/ice/volatile availability and uncertainty, local feedstock, extraction/processing energy, yield, waste, storage and transport. Do not assume accessible reserves from remote evidence.
9. **Time/communications/autonomy:** day/season/mission durations, light-time, contact windows, fault-detection latency, autonomous safe modes and maintenance/resupply delays.
10. **Biological and planetary protection:** crew thermal/radiation/pressure limits if crewed; contamination control and sample integrity where scientifically required. These are separately governed safety gates.
11. **Cascading hazards:** correlated events across tracks, including storm + power deficit + comms loss + thermal derating + repair delay, not just independent one-factor perturbations.
12. **Model validity:** verification/validation evidence, discrepancy, uncertainty range, extrapolation distance and excluded physics.

## 5. Benchmark architecture: one provenance system, multiple physics models

Use four layers:

- **L0 — domain catalog/contract:** scenario IDs, source URLs, units, coordinate/time frame, deployment mode, evidence labels, and model applicability.
- **L1 — domain simulators:** validated tools specialized for power, thermal, atmosphere/CFD, radiation, terrain/robotics, resource extraction, communications or biology as required. Coupling is explicit and versioned.
- **L2 — trajectory and service evaluation:** common event timeline, state carry-over, service floors, electrical/environmental hard constraints, per-gate margins, recovery and failure-frontier outputs.
- **L3 — independent report verification:** recompute metric summaries from raw traces; check pairing, units, hashes, coverage, missing data, and negative controls; keep synthetic/model/HIL/field evidence labels.

A simulation can only claim what its calibrated physics and validation support. A passing energy-system model does not validate radiation shielding, a planetary rover's contact mechanics, a crew medical limit, or planetary-protection compliance. Domain-specialist validation is mandatory for high-stakes claims.

## 6. Desert-specific and solar-system benchmark targets

These are proposed gates for future preregistered experiments, not current results.

| Target | Proposed criterion |
|---|---|
| Environmental coverage | Every catalog profile has an explicit status: implemented, planned, not applicable with rationale, or blocked by missing data/model |
| Compound-hazard tests | Every priority profile includes single hazards and preregistered compound-event sequences with state carry-over |
| Negative controls | Zero false passes across explicit floor breaches, invalid model inputs, missing data, controller-visible ground-truth leaks and illicit state resets |
| Desert resilience | Report critical-service deficit, longest breach, water budget/quality where modeled, dust/thermal degradation, cleaning/resupply burden and recovery for each arid archetype |
| Model fidelity | Compare the simulator to independent reference data/mission observations within preregistered tolerances; report failures outside the validation envelope |
| Frontier reliability | Reproduce every counted counterexample; retain monotonicity conflicts rather than assuming failure boundaries are monotone |
| Domain separation | No claim of cross-body transfer without an explicit transfer model and held-out validation |
| Reproducibility | Exact scenario, model, input, asset/controller version and outputs hashes for every report |
| Evidence honesty | Report synthetic, modeled, mission-observed, HIL and field evidence separately; unmodeled conditions remain unknown, not passes |

## 7. Implementation sequence

1. Add this domain catalog and its machine-readable manifest; validate IDs, category coverage, required fields, URLs and evidence labels.
2. **Implement Earth desert archetypes first** because weather, solar, water, dust, access and infrastructure data are more obtainable and directly improve remote/off-grid use cases.
3. Add one common environmental trace interface with unit/coordinate/time metadata and explicit uncertainty; keep body-specific models behind adapters.
4. Add a small solar-system representative suite: lunar polar lighting/thermal shadows; Mars dust/energy/endurance; Mercury sunlit-versus-shadowed thermal contrast; Titan cryogenic materials/power; Europa/Jovian radiation plus planetary protection; and asteroid microgravity/contact mechanics.
5. Establish one validated simulator and an independent reference dataset for each domain before comparing controllers or designing for deployment. For the cross-domain program, treat Venus surface and giant-planet atmospheric entry as extreme technology test cases, not ready habitat scenarios.
6. Freeze the scenario generator and holdout, then compare strong published/open baselines within each relevant domain. Publish full trade-offs; do not claim an all-solar-system SOTA from a finite synthetic catalog.

## 8. Primary sources for the initial catalog

- [NASA Planetary Fact Sheet](https://nssdc.gsfc.nasa.gov/planetary/factsheet/) — comparative planetary parameters.
- [NASA Lunar South Pole environment](https://www.nasa.gov/reference/moonbase-environment/) and [Moon Base Architecture User's Guide](https://www.nasa.gov/wp-content/uploads/2026/04/moon-base-architecture-users-guide.pdf) — changing light/shadow, cold traps, volatile uncertainty, and terrain.
- [NASA Mars Facts](https://science.nasa.gov/mars/facts/) — thin atmosphere, temperature range, dust storms and seasons.
- [NASA Mercury Facts](https://science.nasa.gov/mercury/facts/) — day/night temperature extremes, exosphere and polar ice in permanent shadows.
- [NASA Venus Facts](https://science.nasa.gov/venus/venus-facts/) — approximately 467 °C surface and extreme pressure.
- [NASA Io Facts](https://science.nasa.gov/jupiter/jupiter-moons/io/facts/) — volcanism, SO₂ and Jovian plasma environment.
- [NASA Europa Facts](https://science.nasa.gov/jupiter/jupiter-moons/europa/europa-facts/) — radiation and subsurface-ocean evidence.
- [NASA Ganymede Facts](https://science.nasa.gov/jupiter/jupiter-moons/ganymede/facts/), [Callisto Facts](https://science.nasa.gov/jupiter/jupiter-moons/callisto/facts/) — distinct icy-moon geology and magnetosphere/ocean evidence.
- [NASA Enceladus mission summary](https://science.nasa.gov/mission/cassini/science/enceladus/) and [Saturn moons](https://science.nasa.gov/saturn/moons/facts/) — plumes, ocean evidence and sample environment.
- [NASA Titan Facts](https://science.nasa.gov/saturn/moons/titan/facts/) — cryogenic hydrocarbon weather and dense atmosphere.
- [NASA Uranus Facts](https://science.nasa.gov/uranus/facts/), [Neptune Facts](https://science.nasa.gov/neptune/neptune-facts/), [Triton](https://science.nasa.gov/neptune/moons/triton/), [Pluto Facts](https://science.nasa.gov/dwarf-planets/pluto/facts/) — atmospheric-level, cryogenic and seasonal extremes.
- [NASA Planetary Protection Research](https://science.nasa.gov/astrobiology/researchers/funded-research/ppr/) — contamination transport and special-region research.
- [NASA Bennu surface findings (17 March 2026)](https://science.nasa.gov/missions/osiris-rex/asteroid-bennus-rugged-surface-baffled-nasa-we-finally-know-why/) — why small-body surface mechanics should not be assumed to match familiar regolith.

**Non-claim:** this catalog organizes relevant known and hypothesized environmental constraints. It does not show that any proposed habitat, power system, resource extraction method, or controller is feasible on any body. A complete operational design space is not finite or fully known; the defensible goal is a traceable, extensible catalog of modeled and measured conditions with explicit unknowns.
