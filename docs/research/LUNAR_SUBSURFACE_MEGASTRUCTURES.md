# Lunar Subsurface Megastructures — Research Baseline v0

**Status:** research baseline / unqualified architecture study  
**Owner:** Symtropy research programme  
**Parent work:** [LUNAR-SUBSURFACE-MEGASTRUCTURES-001](https://github.com/Luminous-Dynamics/symtropy/issues/5963)  
**Collider child:** [LUNAR-UNDERGROUND-ACCELERATOR-001](https://github.com/Luminous-Dynamics/symtropy/issues/5964)  
**Existing collider programme:** [#3953](https://github.com/Luminous-Dynamics/symtropy/issues/3953)

This note is an architecture research baseline. It is not a site selection, geotechnical design, safety case, mine plan, structural certification, accelerator feasibility result, or lunar construction capability claim.

## Executive result

The Moon is a credible first off-Earth case for studying **engineered subsurface scientific and industrial infrastructure**, but not because its interior is automatically stable, cold, or easy to excavate.

The most promising architecture is a **distributed underground utility campus**:
- purpose-excavated tunnels through mapped competent rock for precision equipment and transport;
- independent pressure-rated modules under engineered overburden for habitats and control rooms;
- natural lava tubes as surveyed candidate spaces for selected non-critical uses or separately supported structures, not as assumed safe shells;
- large caverns only where detector, research, or industrial functions genuinely need them;
- surface solar collection, communications, radiator panels, landing access, and other systems that need exposure to space;
- a monitored utility spine connecting the nodes, with compartmentalization and multiple access/escape paths.

A lunar underground collider is worth researching as a placement alternative. It does **not** gain energy merely by going underground. At fixed circumference, field, fill, and lattice, its first-order magnetic-rigidity energy is unchanged. Its case must be made through system-level gains in shielding, thermal stability, protection from surface operations, metrology, maintainability, and scientific uptime—after subtracting excavation, structural support, access, heat export, and power-distribution costs.

The Moon should remain a comparator, not a selected construction site. Earth remains the operational baseline; Mars is the most useful second rocky-body comparator.

## 1. What “inside the Moon” should mean

Do not interpret “inside” as placing an accelerator near the lunar centre. The first credible concept is a **crustal tunnel network at a surveyed, site-specific depth**, with the burial depth treated as a design variable.

Three different environments must not be conflated:

| Environment | Potential value | Dominant uncertainty |
|---|---|---|
| Shallow regolith cover | Shielding and partial suppression of surface thermal cycling | Bearing, compaction, settlement, dust, excavation and support |
| Competent-rock tunnel | Precision corridor, shielding, protected utilities, controlled access | Lithology, discontinuities, stress, excavation damage, convergence and seismic response |
| Natural lava tube | Large pre-existing void, radiation and micrometeorite shielding, possible campus space | Actual dimensions, roof/floor/wall integrity, debris, access, long-term stability |

A natural void is not a tunnel that has already passed acceptance. An engineered tunnel is not automatically stable because it is underground. Both require site-specific ground truth.

### 1.1 Natural lava tubes: useful, but not a shortcut

The 2025 study *Crack Development Inside and Around Lunar Lava Tubes* predicts tensile failure in tube floors and roofs and compressive wall failure in large tubes. It notes that current observations do not confidently constrain actual tube dimensions and internal conditions. The study identifies relatively intact floor zones near tube walls in some modeled cases, but the useable region depends on shape and size.  
Source: [AGU / Journal of Geophysical Research: Planets (2025)](https://doi.org/10.1029/2024JE008553).

A 2024 *Icarus* study found that variable cross-section geometry can substantially change modeled stability and estimated 65–285 m widths beneath some well-known skylights.  
Source: [Icarus (2024)](https://doi.org/10.1016/j.icarus.2023.115928).

**Design conclusion:** prioritize robotic survey, mapping and ground-truthing before occupancy or heavy construction. Prefer placing independent structures inside or adjacent to a tube over relying on its roof as the primary engineered structure. Do not reshape or excavate a candidate tube until its site-specific failure and preservation implications are modeled.

### 1.2 The thermal advantage is real but bounded

An Apollo 17 deep-drill-core analysis published 3 September 2026 estimates an average temperature of 248 ± 14 K and a 1.0 ± 0.5 K/m gradient over the sampled interval from 1.33 to 2.91 m. At that site, the annual thermal wave did not extend beyond about 1.35 m. This is useful evidence that shallow subsurface conditions attenuate surface thermal cycles; it is not a deep lunar geothermal profile and must not be extrapolated to kilometre depths.  
Source: [JGR: Planets (2026)](https://doi.org/10.1029/2026JE009720).

Subsurface thermal stability is not a heat sink. A buried collider, data centre, workshop or habitat still needs an explicit heat-rejection chain. The Moon has no atmosphere to provide convective cooling; a system that rejects heat to space ultimately needs radiator area, orientation, view factor, transport pipes/heat pipes, and dust/maintenance assumptions.

### 1.3 Moonquakes and impacts remain in the load case

NASA records deep, shallow, impact-related and thermal moonquakes. Shallow moonquakes associated with tectonic scarps can produce strong shaking many tens of kilometres away; modeled slopes near some south-polar faults may be susceptible to landslides under even modest seismic shaking.  
Sources: [NASA Moonquakes](https://science.nasa.gov/moon/moonquakes/) and [NASA south-polar faults](https://www.nasa.gov/solar-system/moon/shrinking-moon-causing-moonquakes-and-faults-near-lunar-south-pole/).

A precision ring needs a frequency-resolved transfer chain from ground motion through tunnel support and magnet supports to magnetic centres and beam optics. A static installation survey is not a lifetime alignment strategy. Acceptance tolerances must be derived from the actual lattice and beam-dynamics model, not copied from an Earth machine.

## 2. First collider concept: a local ring inside competent rock

### 2.1 Geometry, not great-circle circumference

The existing local-ring work compares 50, 100 and 150 km circumferences and field scenarios. The subsurface question is whether those rings can be placed inside a suitable crustal corridor.

The ring is local. It does not need to follow the Moon's great-circle circumference. A 100 km ring on the Moon and an otherwise identical 100 km ring on Earth have the same first-order rigidity energy. Underground placement adds or subtracts infrastructure terms; it does not increase the bending field or bending radius.

### 2.2 Excavation volume is an early decision filter

A useful first-order civil estimate is:

\[
V_{\mathrm{exc}} = C \, A_{\mathrm{section}} \, N_{\mathrm{bores}} + V_{\mathrm{shafts}} + V_{\mathrm{caverns}} + V_{\mathrm{cross-passages}}
\]

For illustration only, a single 100 km tunnel with a 30 m² excavated cross-section would remove approximately 3 million m³ of material before shafts, detector caverns, service cross-passages, overbreak and support allowances. A second full-length bore doubles the first term. These are arithmetic scenarios, not a selected cross-section or a lunar tunnelling estimate.

This makes excavation productivity, support mass, spoil handling, tool life, energy per cubic metre, and repair strategy first-order feasibility variables—not details to add after collider energy is calculated.

### 2.3 Tunnel placement should be tested as a tournament

The initial depth bins below are experimental cases, not engineering recommendations or known safe lunar depths.

| Architecture | Initial scenario | Main possible benefit | Main cost / risk |
|---|---:|---|---|
| Surface + regolith berm | Surface | Minimal tunnel excavation; accessible | Thermal cycling, dust, impacts, beam-loss shielding, operations |
| Shallow cover | 5–20 m overburden | Shielding and partial thermal-cycle attenuation | Regolith support, settlement, compaction, access |
| Competent-rock tunnel | 50–300 m depth | Protected precision corridor; shielding | Rock-mass uncertainty, convergence, excavation and support |
| Deep rock tunnel | 0.5–2 km depth | Test stronger environmental isolation and shielding | Long shafts, heat export, stress, access and repair burden |
| Lava-tube-adjacent hybrid | Site-dependent | Reuse candidate void for selected functions | Unknown tube condition, debris, failure zones, awkward geometry |
| Mixed surface/subsurface | By subsystem | Bury only where benefit exists | Interfaces, transitions, differential motion and distributed utilities |

Use the same ring, lattice, beam conditions and operating schedule across all placements. Compare radiation spectra, beam-loss transport, detector backgrounds, ground-motion transfer, tunnel convergence, heat transport, electrical distribution, access, maintainability and lifecycle resource use.

**Reject any “underground is better” conclusion that does not show the benefit after the tunnel, support, shaft, radiator, power, service and repair systems are included.**

### 2.4 Architecture principles

- Keep precision beam tunnels in surveyed, purpose-excavated corridors unless a natural void is independently shown to meet the same requirements.
- Use detector and beam-dump caverns only after structural, radiation, access and thermal qualification.
- Separate beam, utility, service and emergency routes where common-cause failures justify the additional excavation.
- Instrument tunnel convergence, strain, crack growth, temperature, vibration, anchor load and support condition over the full operating life.
- Provide redundant geodetic references and independent metrology checks.
- Model heat export to external radiators; do not use rock as an unlimited thermal reservoir.
- Treat beam abort, loss of power, quench, fire, local collapse, blocked access, radiation event and stale metrology as explicit failure states.

## 3. Lunar subsurface megastructure portfolio

The goal is not a single hollowed-out Moon. It is a distributed system in which each underground volume has a clear service role and an explicit interface to surface or orbital infrastructure.

| Structure | Value proposition | Design rule / critical test |
|---|---|---|
| Accelerator and detector complex | Precision beam path, shielding, detector caverns, protected utilities | Lattice-derived alignment, dose transport, heat export and tunnel qualification |
| Low-background science caverns | Rare-event and neutrino experiments, cosmic-ray and lunar geophysics instruments | Measure rock radioactivity, radon/outgassing analogues, muon flux, detector background and seismic noise |
| Shielded habitat clusters | Radiation and micrometeorite protection; controlled thermal environment | Independent pressure modules, compartmentation, egress and life-support fault isolation |
| Industrial / repair galleries | Machine tools, metrology, additive manufacturing, spare parts, service corridors | Vibration zoning, contamination control, remote servicing, accepted product tests |
| Volatile cold-chain | Preserve and move water/other volatiles from cold traps | Survey and assay before extraction; separate cold stores from hot industrial operations |
| Thermal-storage vaults | Buffer intermittent power or industrial heat | Prove full charge/discharge efficiency, insulation and radiator balance |
| Controlled-environment farms | Protected crop production and biological recycling | Photon, water, nutrient, food acceptance and heat/power budgets; underground is not inherently energy-saving |
| Subsurface freight and mass-driver interfaces | Protected logistics and bulk transport interfaces | Keep a linear electromagnetic launcher distinct from a circular collider or orbital ring; qualify loads and terminal geometry |
| Archive / compute rooms | Protected long-life instruments or data systems | Compare with surface/orbital alternatives; quantify heat rejection, access, power and radiation |

Polar cold traps have unique value for volatile storage, but are not necessarily ideal places for high-power industrial plants or a collider. NASA identifies strong variation in illumination and thermal conditions around the south pole, while water ice stability depends on local cold-trap conditions. Keep volatile-preservation zones and high-temperature industrial zones spatially separated.  
Sources: [NASA Lunar South Pole environment](https://www.nasa.gov/reference/moonbase-environment/) and [Nature Astronomy: micro cold traps](https://www.nature.com/articles/s41550-020-1198-9).

## 4. What should be underground on other bodies?

| Body | Strongest initial subsurface hypothesis | Why it differs from the Moon | Avoid assuming |
|---|---|---|---|
| Mars | Buried habitation, protected science and industrial tunnels | Rock/regolith, atmosphere, gravity and thermal environment differ; local geology and water history matter | Lunar excavation parameters transfer directly |
| Ceres | Shallow anchored/buried infrastructure and volatile processing | Low gravity and potentially ice-rich/heterogeneous material | Conventional deep rock tunnelling is the default |
| Callisto | Ice-shell research, protected modules and cryogenic science | Icy crust and Jovian system environment | Ice behaves like competent basalt or that subsurface ocean access is practical |
| Titan | Ice/organic chemistry facilities and protected modules | Cryogenic ice substrate and dense atmosphere | Lunar dust, thermal and pressure assumptions transfer |
| Mercury | Polar cold-trap research and thermally isolated facilities | Extreme insolation gradients and unusual polar thermal opportunities | Global subsurface temperature is uniform |
| Phobos / Deimos / small asteroids | Anchored vaults, enclosed modules and regolith-contained infrastructure | Very low gravity, cohesion, rotation and uncertain internal structure | A deep tunnel behaves like a tunnel in a large rocky planet |

Earth remains the benchmark for tunnel boring, support, metrology, underground utilities, safety and accelerator operation. Mars is the strongest second-body comparator for rock excavation and buried habitats. Small bodies are more likely to need anchored or enclosed structures than conventional deep tunnel networks.

## 5. Industry baseline — November 2026

The current industry frontier is **technology development for surface infrastructure**, not operational lunar megastructure construction.

- NASA's 2026 Lunar Surface Technology portfolio covers power, thermal technology, autonomy/robotics, dust mitigation, excavation/construction, communications/navigation, and ISRU. Its IPEx figures are presented differently across current NASA pages: the portfolio summary says 10 metric tonnes over 100 m and 11 days; the dedicated IPEx page says up to 10,000 kg in a single lunar day; the detailed portfolio description says on the order of 20 metric tonnes over a mission. Preserve these as separate source-specific design profiles and resolve the mission configuration before benchmarking; none is a deep tunnel-boring result. [NASA Lunar Surface Technology](https://www.nasa.gov/lunar-surface-technology/) and [NASA IPEx](https://www.nasa.gov/infrastructure-pilot-excavator/).
- NASA's September 2026 Lunar Enabling Infrastructure solicitation emphasizes vertical solar arrays, oxygen from regolith, radioisotope Stirling generators, in-space advanced manufacturing and innovative nanomaterials. The agency's stated goal is maturation and ground demonstration; it is not evidence that these systems are already deployed on the Moon. [NASA release, 8 September 2026](https://www.nasa.gov/news-release/nasa-calls-for-proposals-to-accelerate-lunar-surface-technologies/).
- Astroport's Brickbot programme is developing molten-tile deposition for roads, landing pads and surface pavements from processed regolith. It is useful as a surface construction benchmark, not a deep subsurface construction analogue. [NASA SBIR award](https://www.sbir.gov/awards/220309).
- NASA describes ICON's Olympus technology as a robotic laser-based approach for transforming local regolith into ceramic-like structures. This is a relevant additive construction comparator, but it does not demonstrate support of kilometre-scale tunnels or large lava tubes. [NASA construction technology overview](https://www.nasa.gov/directorates/stmd/nasa-enables-construction-technology-for-moon-and-mars-exploration/).
- NASA has funded work on regolith overburden structures and protection, including berms, trenches and emplaced protection. These are relevant to shielding and habitat comparisons, but must not be confused with a qualified underground facility. [NASA TechPort project 154421](https://techport.nasa.gov/projects/154421).

**Competitive opening for Luminous:** do not claim to out-build NASA or the mining/construction industry today. Build the evidence and qualification method that connects their separate capabilities into a measured, fault-tolerant service architecture: survey → excavation → support → utilities → commissioning → maintenance → evidence. This system-integration and independent-verification layer is a credible research contribution before lunar-scale hardware exists.

## 6. Benchmark programme

Do not begin by claiming a new SOTA. First establish matched terrestrial baselines and repeatable test conditions. The following are proposed measures; numerical targets should be set only after a baseline dataset is frozen.

| Area | Primary metric | Required comparison |
|---|---|---|
| Geology mapping | 3D mapping error, discontinuity recall, uncertainty calibration | Blind borehole/scan ground truth |
| Excavation | m³/kWh, advance rate, overbreak, wear/tonne, kg spoil moved | Same rock/simulant, tool and duty cycle |
| Support | support mass/metre, convergence, crack growth, collapse reserve | Independent geomechanics model plus instrumented test |
| Alignment | support/magnet displacement spectrum, cumulative drift, recovery time | Ground → tunnel → magnet → optics transfer chain |
| Seismic resilience | response to impact, shaker and representative event spectra | Measured input/output transfer functions |
| Radiation | dose and particle spectra, activation, detector background | Independent transport codes plus dosimetry |
| Thermal | cycle attenuation, gradient, heat export losses, radiator W/m² | Calibrated thermal model and heat balance |
| Robotics | unattended hours, intervention rate, fault recovery, safe retreat | Fault-injected long-duration operation |
| Utility resilience | N−1 load support, black-start, storage endurance, controlled abort | Hardware-in-loop and fault-injection |
| Lifecycle service | accepted science/service per Earth-launched kg, MWh and maintenance hour | Common boundary and uncertainty propagation |

The core differentiator should be **evidence-qualified system performance**:
- source and toolchain pinned;
- assumptions and units typed;
- measurements distinguished from models;
- uncertainty propagated rather than hidden;
- negative controls for invalid parameter combinations;
- independent cross-checks for safety- or performance-critical outputs;
- reproducible run receipts tied to exact source and inputs;
- held-out conditions and physical analogue evidence before any SOTA claim.

## 7. Staged execution plan

**Stage 0 — desk evidence and geometry.** No site selected. Build the geologic evidence atlas, scenario schema, uncertainty register, and matched trade boundary.

**Stage 1 — Earth analogue.** Use an instrumented basalt/rock tunnel and a separate berm/cavern module. Measure mapping quality, excavation energy, tool wear, support response, vibration transfer, heat paths, dust and robotic repair.

**Stage 2 — robotic lunar prospecting.** Demonstrate remote approach, entry, mapping, sampling and retreat with no human occupancy. Preserve site data and uncertainty.

**Stage 3 — pilot excavation.** Demonstrate short, instrumented tunnel and cavern sections, before/after geophysics, support installation and long-term convergence monitoring.

**Stage 4 — pilot subsurface utility spine.** Connect power, communications, thermal transport, logistics and repair in an unoccupied facility.

**Stage 5 — facility-specific qualification.** Habitats, detector caverns, industrial plants, farms and an accelerator each require separate acceptance cases. No facility inherits another's safety qualification.

Unknown or stale critical state blocks progression. A simulator pass is not a physical demonstration; an Earth analogue is not lunar qualification.

## 8. Immediate next work

1. Extend the existing local-ring geometry screen with burial depth and tunnel-section parameters, explicitly marking them as civil scenario assumptions.
2. Build an excavation-volume and spoil-handling calculator using transparent geometry only; do not imply tunnelling productivity.
3. Define the ground-to-magnet vibration and alignment interface for #5932, then request the relevant sensitivity matrix from beam-dynamics owners.
4. Create matched surface/berm/shallow/deep radiation cases with a named loss map and independent transport tools.
5. Add external heat-export and access/repair costs to the existing host-utility comparison (#5907).
6. Rank survey and Earth-analogue experiments by expected decision impact, not data volume.
7. Keep the output as Pareto comparisons; do not collapse physics, cost, safety, maintenance and uncertainty into a single weighted score.

## Existing work to compose

- [#5963 — subsurface megastructure architecture](https://github.com/Luminous-Dynamics/symtropy/issues/5963)
- [#5964 — buried local-ring collider architecture](https://github.com/Luminous-Dynamics/symtropy/issues/5964)
- [#5924 — local Earth/Moon rings](https://github.com/Luminous-Dynamics/symtropy/issues/5924)
- [#5932 — ground motion and lifetime alignment](https://github.com/Luminous-Dynamics/symtropy/issues/5932)
- [#5907 — power, storage and heat rejection](https://github.com/Luminous-Dynamics/symtropy/issues/5907)
- [#3953 — comparative space-collider programme](https://github.com/Luminous-Dynamics/symtropy/issues/3953)
- [#3952 / #4002 — cavern bio-loop and Earth analogue](https://github.com/Luminous-Dynamics/symtropy/issues/3952)
- [#5341 — cislunar industrialization gate](https://github.com/Luminous-Dynamics/symtropy/issues/5341)

## References

1. Crack Development Inside and Around Lunar Lava Tubes, JGR: Planets (2025), https://doi.org/10.1029/2024JE008553
2. Structural Stability of Lunar Lava Tubes with Consideration of Variable Cross-Section Geometry, Icarus (2024), https://doi.org/10.1016/j.icarus.2023.115928
3. Subsurface Temperatures, Thermal Gradient, and Thermal Wave Propagation Depth Constrained Using Thermoluminescence of the Apollo 17 Deep Drill Core, JGR: Planets (published 3 September 2026), https://doi.org/10.1029/2026JE009720
4. NASA Moonquakes, https://science.nasa.gov/moon/moonquakes/
5. NASA, Shrinking Moon Causing Moonquakes and Faults Near Lunar South Pole, https://www.nasa.gov/solar-system/moon/shrinking-moon-causing-moonquakes-and-faults-near-lunar-south-pole/
6. Assessment of Significant Geotechnical Parameters for Lunar Regolith Excavations, Acta Astronautica (2022), https://doi.org/10.1016/j.actaastro.2022.04.008
7. NASA Lunar Surface Technology, https://www.nasa.gov/lunar-surface-technology/
8. NASA Calls for Proposals to Accelerate Lunar Surface Technologies (8 September 2026), https://www.nasa.gov/news-release/nasa-calls-for-proposals-to-accelerate-lunar-surface-technologies/
9. NASA SBIR: Brickbot for Lunar Surface Subscale Technology Demonstration, https://www.sbir.gov/awards/220309
10. NASA, Construction Technology for Moon and Mars Exploration, https://www.nasa.gov/directorates/stmd/nasa-enables-construction-technology-for-moon-and-mars-exploration/
11. NASA TechPort: Regolith Overburden Structures on the Moon, https://techport.nasa.gov/projects/154421


## 9. Additional architecture candidates and explicit non-candidates

### 9.1 Subsurface power and industrial heat vaults

A protected fission-power conversion plant or high-voltage switchyard could be emplaced in a shielded, compartmentalized underground vault, while the radiator field remains at the surface with an unobstructed view to space. This is a siting hypothesis, not a reactor design. The comparison must include reactor servicing, shielding, cable losses, thermal transport, fire/pressure isolation, and independent shutdown/decay-heat removal. A buried power plant that cannot reject heat safely is not a viable plant.

### 9.2 Rotating artificial-gravity settlement inside a cavern

A very large engineered cavern could, in principle, host one or more mechanically independent rotating habitat rings or cylinders. This is a long-horizon megastructure option, not a reason to excavate a giant lunar cave first. The primary design challenges become rotor dynamics, bearing/support loads, containment, fatigue, pressure boundaries, vibration isolation from nearby scientific equipment, emergency braking, and maintenance of rotating interfaces. The cavern should be compared against a free-standing or surface-assembled rotating habitat; underground placement is beneficial only if its shielding and containment benefits exceed excavation and structural costs.

For scale, a rotating ring's radial acceleration is a = omega² r. At a 500 m radius and 1 rpm this is approximately 5.5 m/s² (0.56 g). That number is kinematics only; it says nothing about structural feasibility, comfort, or required mass.

### 9.3 Radio astronomy belongs above ground

Do not place the active collecting aperture of an optical or radio telescope underground. Rock blocks the incoming signal. On the lunar far side, a surface or orbiting radio array may exploit shielding from terrestrial radio interference; underground galleries can house power conditioning, timing, computing, repair equipment, and cable routes, but the antennas must retain a clear sky view. This is an important counterexample to the idea that every major scientific instrument should be subterranean.

### 9.4 Linear freight launcher versus collider

A mass driver or electromagnetic freight launcher is a long linear accelerator for payload logistics, not a particle collider. An underground or mountain-side launch track might protect precision guideways and allow a controlled terminal aperture, but the exit corridor, payload acceleration, launch plume/ejecta, orbital insertion, and failure containment dominate. Keep this as a separate architecture with its own trajectory and safety authority.

### 9.5 Excavation-volume sensitivity table

The table below exposes the scale of the civil works before detector halls, shafts, cross-passages, overbreak, lining/support, or spoil-handling allowance. It assumes a constant tunnel cross-section and one bore; a second equivalent full-length bore doubles every entry.

| Ring length | 10 m² section | 20 m² section | 30 m² section | 40 m² section |
|---:|---:|---:|---:|---:|
| 50 km | 0.5 million m³ | 1.0 million m³ | 1.5 million m³ | 2.0 million m³ |
| 100 km | 1.0 million m³ | 2.0 million m³ | 3.0 million m³ | 4.0 million m³ |
| 150 km | 1.5 million m³ | 3.0 million m³ | 4.5 million m³ | 6.0 million m³ |

This is a deliberately simple geometry lower bound: V = length × excavated cross-section. It is not a prediction of actual ring civil layout, because tunnel diameters, beam separation, emergency/service galleries, detector caverns, shafts, support thickness, local geology, and construction method are not yet selected. The value of this table is that it makes excavation volume visible early enough to influence architecture.

### 9.6 Body-specific geology references

- **Ceres:** NASA's Dawn synthesis identifies a water-rich, differentiated body with high-latitude ice and a crust containing mixtures of hydrated salts, rocks and clathrates. The substrate should be treated as heterogeneous rather than as ordinary dry rock: https://science.nasa.gov/mission/dawn/science/ceres/
- **Titan:** NASA describes a dense nitrogen/methane atmosphere and a water-ice surface that behaves mechanically as rock at Titan's cryogenic temperatures. This makes excavation and pressure/thermal boundaries fundamentally different from lunar construction: https://science.nasa.gov/saturn/moons/titan/facts/
- **Mercury:** MESSENGER-era observations and thermal modeling support permanently shadowed polar deposits with local temperatures below 100 K. This is a specialized cold-trap architecture, not a general underground settlement advantage: https://science.nasa.gov/photojournal/hot-and-cold/

The Callisto and Mars entries remain screening hypotheses in this baseline. Before architecture ranking, add body-specific primary sources for crust composition, temperature, radiation, seismicity, gravity, and subsurface uncertainty. Do not fill missing evidence with analogy.


## 10. New observability evidence and the survey bottleneck

A 28 March 2026 JGR: Planets paper evaluates seismic interferometry, H/V spectral ratios, distributed acoustic sensing (DAS), and rotational measurements for lunar lava-tube imaging. In its synthetic simulations, the techniques respond to cavities and can estimate roof thickness, but total depth and the lower boundary are substantially harder to recover. Structures deeper than approximately 10 m require ambient noise below 1 Hz; the common thermal-moonquake signal is mainly above 1 Hz and primarily sensitive to the upper ~10 m. The authors argue for combinations of passive seismic techniques and complementary geophysical methods.

Source: Keil et al. (2026), https://doi.org/10.1029/2025JE009576

**Research consequence:** candidate-site surveying should be treated as an observability problem, not simply a mapping exercise. A multi-modal campaign could combine orbital / rover morphology, radar, gravimetry, active seismic sources, passive seismic, DAS, and eventually close-range robotic mapping. Every method needs a declared depth/scale detection envelope and false-positive model. A single best-fit cave mesh is not a sufficient basis for excavation or entry.

### Model-form uncertainty is now a first-order design input

The literature does not support treating one deterministic factor of safety as a universal answer:
- Williams & Montési (2025) predict extensive internal roof/floor failure zones and note that current observation strategies do not confidently constrain exact tube dimensions: https://doi.org/10.1029/2024JE008553
- A 2024 Icarus variable-cross-section analysis finds irregular geometry can materially reduce modeled stability and estimates 65–285 m widths beneath some studied skylights: https://doi.org/10.1016/j.icarus.2023.115928
- A 2024 Earth and Planetary Science Letters study reports that layered roofs with weak interlayers can be less stable in its modeled cases and predicts local slice-like collapses: https://doi.org/10.1016/j.epsl.2024.118918
- A 2026 Icarus study reports a different role for weak interlayers in its elasto-brittle cellular-automata models, where layer splitting can disperse loads and limit vertical crack propagation; it also emphasizes span and constitutive-model effects: https://doi.org/10.1016/j.icarus.2026.117054
- A 2025 probabilistic study reports more than 30,000 numerical analyses and characterizes distributions of collapse behavior under uncertain geometry: https://doi.org/10.1016/j.gsf.2025.102076
- A January 2026 paper proposes reshaping a lava-tube roof by removing partial rock mass to reduce modeled plastic-zone height and convergence in PLAXIS 2D. This is a numerical mitigation hypothesis, not demonstrated field practice; roof excavation could itself introduce dangerous transient states and must be compared against support, lining, preservation, and no-intervention alternatives: https://doi.org/10.1016/j.asr.2025.11.011

The research programme should therefore compare model ensembles, constitutive assumptions, geometry priors and held-out cases. Preserve disagreement between models as model-form uncertainty instead of averaging it away or picking the most favorable result.

## 11. Industry benchmark update: excavation and assembly are distinct capabilities

The NASA 2026 *In-space Servicing, Assembly, and Manufacturing (ISAM) State of Play* distinguishes four lunar surface infrastructure capability groups: excavation/site preparation, regolith processing, logistics, and construction. It describes IPEX and LANCE as surface regolith collection / site-preparation prototypes, STOMP as a concept, and ARMADAS as a prototype; other construction activities range from concepts and scale models to demonstrated work. The report describes 50 m truss assembly technology, small autonomous voxel-assembled shelters, and a 5 m modular communications tower demonstration. These are important building blocks, but they are surface assembly and regolith-handling capabilities—not deep tunnel construction, large cavern stabilization, or accelerator-grade underground alignment.

Source: NASA Langley Research Center et al., *2026 ISAM State of Play*, especially pages 40–41, https://assets.science.nasa.gov/content/dam/science/missions/isam/ISAM_State_of_Play_2026.pdf

For terrestrial hard-rock tunnel-boring comparisons, avoid quoting one universal "industry advance rate." The literature separates:
- penetration rate / rate of penetration (m/h) while actively cutting;
- utilization (fraction of the scheduled period actually boring);
- advance rate (m/day or m/week), which depends on penetration rate, utilization, shift length and shift count;
- cutter life / replacement rate;
- downtime from support, mucking, maintenance, geology and logistics.

The operational distinction is essential: high instantaneous penetration can coexist with poor calendar advance if utilization collapses. A lunar robotic benchmark should therefore report cutting, clearing/mucking, support, inspection, charging, thermal dwell, maintenance and fault recovery separately, then report end-to-end accepted metres and excavated volume per mission time. Compare with terrestrial TBMs only on matched geology, diameter, support standard, shift/crew regime, and system boundary. Useful reference methods include Rostami (2016), https://doi.org/10.1016/j.tust.2016.01.009, and Zhang et al. (2024), https://doi.org/10.3390/app14114512.

## 12. Executable quantity screen added

A separate dependency-free Rust crate now owns the narrow civil quantity calculation:

- `tools/lunar-subsurface-reference/` is independent of the collider beam-physics crate.
- It emits a deterministic 72-case geometry matrix: 50/100/150 km; 10/20/30/40 m²; one/two bores; 0/10/20% overbreak.
- It reports design bore volume, overbreak volume, and shaft/cavern/cross-passage volumes separately.
- The first known answer is 100 km × 30 m² × one bore = 3,000,000 m³. Two bores with 20% overbreak produce 7,200,000 m³ before ancillary volumes.
- A workflow pins Rust 1.96.0, checks formatting/tests/Clippy, executes twice for deterministic JSON, asserts known answers, and retains a source/report digest manifest.

The tool intentionally does not calculate spoil mass, swell, excavation power, tunnel advance rate, support quantities, cost, geotechnical stability, or lunar feasibility. Those remain outside the geometry-only evidence class.

Tracked implementation: [#5980 LUNAR-EXCAVATION-ACCOUNTING-001](https://github.com/Luminous-Dynamics/symtropy/issues/5980).  
Observability work: [#5983 LUNAR-SUBSURFACE-OBSERVABILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5983).
Terrestrial physical analogue: [#5988 LUNAR-SUBSURFACE-EARTH-ANALOG-001](https://github.com/Luminous-Dynamics/symtropy/issues/5988).


## 13. Source-scoped bulk-mass accounting — new executable layer

The standalone Rust reference now includes a separate material-accounting object. It uses literature-scoped **bulk density** only where the material class matches:

- Feldspathic highland crust: 2,200–2,600 kg/m³.
- Impact-basin ejecta: 2,350–2,600 kg/m³.

Kiefer et al. (2012) reports these as bulk-density intervals for those specific lunar geological classes, not as a universal density for every potential tunnel corridor: https://doi.org/10.1029/2012GL051319.

For the 100 km × 30 m² × one-bore geometry reference (3,000,000 m³), the highland-crust interval gives 6.6–7.8 billion kg of in-situ bulk mass; the impact-basin-ejecta interval gives 7.05–7.8 billion kg. For the two-bore, 20% overbreak reference (7,200,000 m³), the corresponding intervals are 15.84–18.72 billion kg and 16.92–18.72 billion kg. These are conditional material-class scenarios, not site-specific mass estimates or transport loads.

The report separately catalogs:
- Apollo regolith source rows from NASA/TM-20240011783 Table 15, preserving mission and underlying publication;
- terrestrial lunar-regolith simulant ranges from the same guide's Table 16, preserving simulant and regionality;
- a separate loose_spoil_and_swell status of not_calculated because no loose-spoil density or handling-specific bank-to-loose ratio is supplied.

NASA's guide shows why the provenance must remain granular: Apollo mission and source rows can differ substantially, including multiple source rows for the same mission. Do not collapse these into one universal lunar regolith value. The 2026 open database likewise preserves mission, test location, environment, method and multiple reported values: https://doi.org/10.1002/nag.70432.

This material-accounting layer is deliberately not applied to the 72 geometry-only ring scenarios as a site-selected material. It exposes reference calculations and preserves the material-domain mismatch. Follow-on issue: [#6004 LUNAR-MATERIAL-ACCOUNTING-001](https://github.com/Luminous-Dynamics/symtropy/issues/6004).

## 14. New closed-loop benchmark: planned excavation to accepted as-built asset

A further paradigm shift is to measure **accepted construction output**, not only planned tunnel volume or machine-reported spoil.

The new [#6006 LUNAR-ASBUILT-CLOSURE-001](https://github.com/Luminous-Dynamics/symtropy/issues/6006) defines the evidence chain:
survey and calibration → geology/material assignment → planned segment → machine action → measured post-cut geometry → material mass/volume observations → spoil disposition → installed support → independent inspection and acceptance.

Required metrics include:
- planned versus measured volume and spatially resolved overbreak;
- excavated mass versus transported, disposed and reused material closure;
- kWh per independently accepted m³;
- cutting, mucking, support, inspection and repair time shares;
- accepted metres per mission-hour;
- support mass/length and as-built coverage;
- survey uncertainty, calibration drift and defect closure;
- fraction of accepted asset with complete source-to-as-built evidence.

Missing sensors, stale calibration, unit mismatches, duplicate IDs, conflicting measurements or incomplete mass balance produce UNKNOWN / RECONCILIATION_REQUIRED. They do not silently create accepted construction.

This creates an evidence-carrying construction method that can be benchmarked on Earth now, and later adapted to lunar conditions without pretending a terrestrial result is flight qualification.
