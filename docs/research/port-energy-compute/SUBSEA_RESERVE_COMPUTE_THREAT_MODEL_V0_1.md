# Subsea Reserve Compute: Threat Model and Site-Screen v0.1

**Status:** research design basis; not a certified security design, geological survey, environmental impact assessment, site selection, or investment case.  
**Date:** 2026-11-20  
**Companion:** [OCEAN-COMPUTE-WORKLOAD-001](https://github.com/Luminous-Dynamics/symtropy/issues/6319), [subsea thermal/cable qualification](https://github.com/Luminous-Dynamics/symtropy/issues/6228), [port OT cyber-resilience](https://github.com/Luminous-Dynamics/symtropy/issues/6275), [commercial subsea benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/6280).

## 1. Decision

Do not use depth as the principal security control and do not place compute infrastructure in, on, or immediately adjacent to an active hydrothermal vent.

A subsea facility can add physical separation, stable thermal conditions, reduced surface-access exposure, and potentially favourable cooling. It does not create cyber isolation, guarantee cable integrity, or prevent a well-equipped actor from reaching deep-water infrastructure. The reserve function should be qualified by recoverability and independent failure domains first.

Recommended architecture for evaluation:

1. **Primary service:** serviceable coastal/port-edge compute, with shore access and a hybrid seawater heat exchanger.
2. **Warm recovery:** a geographically separated, hardened, land-based or shore-based site with independent grid, backhaul, identity/control plane, and trained operators.
3. **Immutable recovery:** an offline or logically isolated copy, using independently governed credentials and deletion-resistant storage; validate restores in a clean recovery environment.
4. **Optional subsea compute cell:** only for workloads with demonstrated marine/port value, a genuine cooling or resilience advantage, and a fully funded retrieval/repair plan. It must not be the only copy or only recovery site.
5. **Ocean science layer:** non-invasive, rights-aware observation of vents and deep-sea ecosystems. Active vents are research/conservation locations, not industrial heat-extraction sites for this programme.

This is a decision to investigate a coastal/port demonstrator and a distributed reserve architecture—not a decision to build a subsea facility.

## 2. What depth can and cannot defend against

### 2.1 Threat classes

| Threat class | Effect of additional depth | Primary controls |
|---|---|---|
| Opportunistic surface access, casual vandalism, routine anchoring | Can reduce exposure; benefit depends on local bathymetry, shipping, fishing, anchorage and seabed use | Site exclusion zones, surveillance, cable burial/protection where justified, tamper telemetry, response agreements |
| Commercial fishing / accidental seabed contact | May reduce some nearshore interactions, but does not eliminate deep-water fishing or gear interaction | Route/site planning, seabed-use data, monitoring, cable protection and repair readiness |
| Professional subsea intervention or state-level physical sabotage | No depth is a universal barrier; deep-rated ROVs and submersible systems exist | Detection, route diversity, physical segmentation, redundancy, deterrence, repair capability, continuity planning |
| Cyber intrusion, ransomware, malicious firmware, supplier compromise | Depth provides no inherent protection | Zero-trust access, signed/measured boot, secure update chain, least privilege, isolated management, immutable backups, clean-room recovery |
| Insider or credential compromise | Depth provides no inherent protection | Dual control for destructive actions, separated duties, independent approval, audit, offline recovery credentials |
| Landing-station, terrestrial backhaul, grid or cloud-provider compromise | Depth does not remove these external dependencies | Independent providers, separate routes and substations, provider-risk controls, local continuity, out-of-band communications |
| Earthquake, slope failure, turbidity current, storm-driven sediment, corrosion, biofouling, loss of power/cooling | More depth can increase some hazards and makes service/retrieval harder | Geohazard survey, corrosion/fouling tests, conservative design, environmental monitoring, redundancy, retrieval plan |

The Canadian Centre for Cyber Security's current submarine-cable guidance explicitly includes intentional damage by threat actors using sophisticated deep-water submersibles or commercial-grade fishing infrastructure. It also recommends layered measures rather than relying only on burial. A 2026 *Marine Policy* analysis further finds that sabotage risk is location- and system-specific, with bathymetry, surveillance and proximity among relevant factors. Neither source supports a universal “safe depth.”

### 2.2 Pressure is a cost and reliability multiplier

For seawater, a useful first-order gauge-pressure estimate is:

\[
p_g \approx \rho g h
\]

Using \(\rho \approx 1025\;kg/m^3\) and \(g \approx 9.81\;m/s^2\), pressure rises by roughly 10 MPa per kilometre of depth. Approximate figures:

| Water depth | External gauge pressure (approx.) | Interpretation |
|---:|---:|---|
| 36 m | 0.36 MPa | Similar order to Microsoft's 117-ft Project Natick deployment |
| 100 m | 1.0 MPa | About 10 bar differential for a one-atmosphere hull |
| 300 m | 3.0 MPa | Substantial pressure-hull, connector and penetrator burden |
| 1,000 m | 10 MPa | Deep-ocean engineering and recovery logistics |
| 2,800 m | 28 MPa | Extreme pressure-vessel, connector, deployment and maintenance burden |

These are hydrostatic estimates, not certified design pressures. A sealed one-atmosphere pod must withstand the external pressure differential; pressure-balanced/oil-filled designs change the electronics and servicing problem rather than removing it. Local waves, currents, impact, fatigue, thermal cycling, corrosion, seabed bearing and installation loads must also be included.

**Important:** a 6,000 m-rated scientific ROV demonstrates that deep-ocean access is technically possible; it does not imply every actor can deploy one or that such a vehicle can defeat a particular enclosure. The correct conclusion is narrower: depth raises the cost and complexity of access but does not establish a security boundary.

### 2.3 Depth bands are engineering test cases, not security ratings

Evaluate these as alternative design envelopes only after a site survey:

- **20–50 m:** shallow subsea / nearshore demonstrator band. Easier to service than deep ocean; more exposed to navigation, anchoring, fishing, storms and coastal hazards. Natick is a relevant bounded precedent at about 36 m.
- **50–150 m:** candidate band for a later subsea module if local bathymetry, cable route, marine use and retrieval support it. It is not a proven protection threshold.
- **150–300 m:** test only if a specific hydrodynamic, thermal, land-use or threat analysis justifies added pressure and maintenance cost.
- **Greater than 300 m:** do not pursue for a reserve data centre unless an independently reviewed total-system case demonstrates a material service or risk advantage. Deep-ocean placement should not be selected for “security by depth” alone.
- **Approximately 2,800 m near Longqi:** reject as a data-centre location. The depth is not a useful substitute for cyber controls, and the distance, pressure, remoteness, ecological significance and vent chemistry make the system unjustifiable.

There is no target depth in this note that can be described as defending against “most attacks.” Choose a location from a threat-specific risk assessment; do not choose a number first.

## 3. Back-up architecture: defend through recoverability

A reserve facility should be designed around independent recovery domains, not around a single hardened building or pod.

### 3.1 Required separation

- Production administrators must not be able to erase or rewrite every recovery copy.
- Backup identity, keys, control plane and audit must not be wholly dependent on production identity or the same cloud/management tenant.
- Keep at least one recovery copy offline or under a separately controlled immutability mechanism.
- Keep an independently validated full baseline; do not assume incrementals are useful if their baseline or catalogue has been destroyed.
- Test restoration into a clean room from the exact retained data, keys, configuration and software bill of materials.
- Maintain network and grid diversity based on actual routes and common-cause analysis. Two circuits sharing a duct, landing station, substation, corridor, provider or control plane are not independent.
- Retain a local operating mode for marine/port services when subsea export fibre, remote control, cloud identity or external analytics are unavailable.

This follows the core dependency-isolation principle in NIST SP 800-209: recovery copies must not silently depend on production baseline data in a way that defeats recovery during a production compromise.

### 3.2 Recovery tiers to test

These are proposed project targets, not achieved measurements or universal industry requirements:

| Tier | Purpose | Proposed RPO | Proposed RTO | Isolation |
|---|---|---:|---:|---|
| Local continuity | Keep essential local marine/port observations and non-safety analytics operating through external network loss | No loss of locally captured data; bounded by local write/replication design | No interruption for local functions; degraded output clearly labelled | Runs locally; no remote safety-control dependency |
| Warm recovery | Restore agreed compute and data services after primary-site loss | ≤15 minutes for nominated critical data; per-service contract required | ≤4 hours for nominated services | Independent site, power, backhaul and recovery administration |
| Immutable recovery | Recover from ransomware, destructive insider action or control-plane compromise | ≤24 hours for the nominated archive set, or explicitly declared per data class | ≤72 hours for the declared recovery set | Offline / separately administered immutable copy; clean-room restore |
| Subsea compute cell (optional) | Perform useful marine workloads when connected; continue bounded local operation during shore/network disturbance | Defined per workload; no claim of being a complete backup | Defined per workload and retrieval design | No single cable, shore power or remote management dependency |

Do not collapse these tiers into one availability percentage. Publish service-specific RPO, RTO, failure-domain and restore evidence.

## 4. Power and cooling: vents are not a dependable plant

### 4.1 Active hydrothermal vents

Longqi is reported at approximately 37°47′S, 49°39′E and roughly 2,755–2,800 m water depth. Studies describe multiple venting zones, fluids exceeding 300°C, distinctive vent fauna and species recorded only at this field. The Longqi area is within an ISA polymetallic-sulphide exploration contract area. A peer-reviewed review of Indian Ocean vent ecosystems concluded that active vent fields meet scientific criteria for protection and documented their vulnerability and ecological significance.

A hot vent-fluid temperature alone does not establish:
- continuous, harvestable thermal power at the scale and availability required by a data centre;
- a stable temperature/flow/chemistry envelope over the equipment life;
- a corrosion-resistant and maintainable heat-exchanger design;
- acceptable effects on fluid flow, plume chemistry, microbial processes or benthic communities;
- a viable export cable and service-vessel route.

No validated Longqi electrical-generation capacity, commercial heat-extraction design, or data-centre operating record is admitted in this benchmark. Therefore **vent power is not credited in the energy balance**. Do not drill, cap, divert, heat-exchange against, or otherwise alter active vent structures for this project.

### 4.2 More plausible South African alternatives

- **Coastal/port seawater heat exchange:** test a closed-loop heat exchanger that keeps data-centre coolant physically separated from seawater. Model seasonal source temperatures, fouling, biosecurity, corrosion, intake/outfall impacts, pump energy and fallback cooling.
- **Northern KwaZulu-Natal OTEC research:** South African marine spatial-planning material identifies theoretical thermal-gradient potential offshore of northern KZN at depths around 1,000 m. Treat this as a resource-screening lead, not a bankable or proven generation site. OTEC should have an independent feasibility case and should not be a prerequisite for data-centre backup power.
- **Wave / offshore wind / current:** consider only with measured resource, survivability, maintenance, grid/export and environmental evidence. South African national assessments identify potential in waves, offshore wind and the Agulhas Current, but local marine renewable deployment and commercial maturity remain limited.
- **Battery / grid-forming / dispatchable reserve:** qualify black-start, islanding, load shedding, cooling ride-through and restart at the port/coastal facility. This is the near-term route to dependable continuity; ocean generation can be integrated later when its own performance is established.

Cooling does not require a black smoker. A 2026 review of seawater/lake-water cooling for data centres notes that modern cooling arrangements can use medium-temperature water around 20–30°C. The preferred question is whether a measured local seawater loop reduces whole-facility energy and water impact after pumps, heat exchangers, fouling and backup chillers are counted.

## 5. Indian Ocean and South African site screen

### 5.1 Longqi — reject for data-centre siting

- Approximate position: 37°47′S, 49°39′E.
- Approximate depth: 2.8 km.
- Straight-line distance (great-circle estimate using approximate port coordinates): about 2,200 km from Port of Ngqura and 1,900 km from Port of Durban.
- Vent environment: active, chemically and thermally variable discharge; unique, deep-sea vent fauna.
- Infrastructure: remote from port maintenance and practical terrestrial fibre/power interconnection.
- Conclusion: valuable scientific and conservation location; unsuitable as a data-centre reserve or vent-powered industrial site.

Distances above are screening estimates, not cable route lengths or navigation distances.

### 5.2 Port of Ngqura / Coega — shore-first integration candidate

Continue the existing paired-site pre-feasibility work; do not infer a firm connection offer from public capacity estimates. The 2025 PtX Hub/CSIR port connection assessment describes Port of Ngqura supply through a 132/11 kV substation at Coega with 50 MVA installed capacity and 25 MVA firm supply capacity at that substation. The report remains a pre-feasibility assessment: actual project connection capacity, protection, fault level, export/import limits, redundancy, cost and timeline require utility studies and a formal application.

Best first use: a 100–250 kW IT shore-side demonstrator with an instrumented seawater heat-exchanger loop, renewable/storage integration, workload placement and local marine observation services. Preserve conventional service access.

### 5.3 Port of Durban — comparator, not an assumed better site

Retain Durban as the grid-stress and port-demand comparator in the existing paired-site issue. Evaluate it on actual utility evidence, water/cooling options, flood/storm exposure, cable/backhaul diversity, port operations, land-use constraints and repair access. Do not choose based only on proximity to more subsea cables or the largest local demand.

### 5.4 Northern KwaZulu-Natal — OTEC research lead, separate from vent fields

Investigate the published theoretical OTEC resource indication as a separate ocean-energy feasibility track. It is not evidence of a suitable subsea data-centre site, a viable thermal plant, or permission to develop near sensitive marine habitats. Require a measured temperature-depth profile, seasonal coverage, current and bathymetry data, intake/outfall modelling, ecological baseline and independent techno-economic assessment before naming a location.

## 6. Qualification benchmark: how to beat the current field

Do not claim superiority from depth, PUE, a single server-failure result or renewable percentage. Compete on independently verified useful service and recoverability across the full system boundary.

| Dimension | Required baseline | Proposed gate / stretch target |
|---|---|---|
| Recovery independence | Map production, backup, identity, keys, catalogue, grid, fibre and vendor common causes | No single production credential/control plane can destroy every recovery copy |
| Restore integrity | Blind restore tests from retained data, catalogue, keys and pinned software | 100% of sampled restore tests pass; any silent corruption is a hard fail |
| Recovery time | Measure per service from declared incident start to validated service | Warm tier ≤4 h; immutable tier ≤72 h for a frozen recovery set |
| Data loss | Measure per service from last accepted durable write | Nominated warm tier ≤15 min; offline archive ≤24 h or explicit class-specific exception |
| Network survivability | Physical GIS route graph, landing stations, terrestrial backhaul and provider common causes | No single cable/landing/backhaul failure removes both local continuity and the recovery control plane |
| Power autonomy | Measured kW/kWh and critical load; black-start/islanding test | 72 h critical-load ride-through as first qualification target; 7-day target only after generation, storage, fuel/resupply and weather evidence |
| Cooling survivability | Seasonal temperature, heat rejection, pump energy, fouling and fallback data | No loss of required service under one cooling-train failure; zero unqualified environmental discharge |
| Cyber isolation | External attack-surface inventory, identity dependencies, privileged access and signed update tests | No public management-plane exposure; zero unauthorized destructive action in negative-control exercises |
| Physical resilience | Surveyed threats, detection coverage, retrieval and repair exercise | Demonstrated detection/containment and bounded restoration under site-specific scenarios; no depth-only pass |
| Energy performance | Matched workload, facility meter boundary, network/pump/auxiliary loads | Measured PUE ≤1.15 as parity goal; lower only if annual, independently metered and seasonally representative |
| Environmental performance | Baseline habitat, intake/outfall, noise, thermal plume, sediment and lifecycle inventory | No placement in active vent habitat; no unapproved habitat disturbance; full monitoring and public incident reporting |
| Whole-life economics | CAPEX, OPEX, deployment vessel, retrieval, repairs, cable, spares, insurance, decommissioning | Must beat a shore-based control on lifecycle cost per accepted service without weakening hard gates |

**Non-compensatory rule:** a low PUE cannot offset a failed restore, unauthorized data egress, unsafe operating dependency, unacceptable ecological impact, or inability to recover the pod. Report Pareto trade-offs, not one weighted score.

### 6.1 Matched competitor comparison

Compare the same workload, quality threshold, service volume, latency class, climate period, carbon boundary and redundancy level across:
- a conventional land-based data centre;
- a port-edge/coastal centre using seawater heat exchange;
- a floating/nearshore module;
- a subsea module;
- external cloud.

Include the whole facility: compute, storage, network, pumps, cooling, facility auxiliaries, energy storage losses, cable construction/repair, vessel fuel, replacement hardware, and end-of-life. State excluded quantities explicitly. Compare reliability with matched hardware generations, duty cycles, operating temperatures and maintenance policies; never compare Microsoft's non-identical 2020 pilot cohorts as a universal industry baseline.

## 7. Minimum evidence package before site selection

1. **Bathymetry / seabed:** high-resolution multibeam, slope, sediment mobility, geotechnical cores, scour, landslide/turbidity-current exposure, seismicity, volcanic/vent exclusion and seabed-use mapping.
2. **Ocean conditions:** at least a seasonal temperature-depth profile; salinity/conductivity; currents and wave climate; marine growth/fouling; storm extremes; turbidity and sediment deposition.
3. **Thermal measurements:** calibrated inlet/outlet temperatures and flow, heat-exchanger differential pressure, pump power, thermal approach, fouling trend, fallback-chiller power and full facility metering.
4. **Cable and power:** independently mapped fibre and power paths; landing station, duct, substation and provider common-cause analysis; utility connection study; black-start and islanding proof.
5. **Environmental baseline:** benthic habitat and species survey, hydrothermal indicators where relevant, sound levels, sediment and plume measurements, intake/outfall modelling, protected-area and permitting screen.
6. **Security assessment:** threat actors and objectives, attack paths, credential/control-plane dependency graph, physical access and surveillance model, monitoring blind spots, incident response, recovery and repair plan.
7. **Service economics:** operator workload traces with rights and quality metadata, matched workload placement, lifecycle energy/cost/carbon and independently accepted service-unit denominator.
8. **Retrieval / end of life:** deployment and recovery vessels, weather window, lifting points, emergency recovery, hazardous materials, spares, repair turnaround, decommissioning and financial provision.

No public synthetic value in the current benchmark may be re-labelled as an observation. The first field-data admission must carry source, licence/rights, sensor identity, calibration, units, timestamps, quality flags, processing version and an auditable custody record.

## 8. Execution sequence

**P0 — now, desk and software work**
- Extend the synthetic scenario corpus with ransomware/control-plane compromise, cable partition, landing-station loss, grid loss, cooling loss, deep-water intervention, seabed geohazard and vent-site exclusion.
- Add a dependency graph for recovery copies, credentials, network, grid and identity; mark common-cause edges explicitly.
- Keep the placement evaluator hard-gated and multi-objective. Do not add a security score that lets good energy performance compensate for a failed recovery gate.

**P1 — evidence acquisition**
- Obtain rights-reviewed OOI/ERDDAP or authorized local ocean series through Mycelix #6149.
- Request site-specific utility and port data through the Ngqura/Durban paired-site process.
- Obtain hydrographic and environmental data for the candidate coastal area before selecting a depth.

**P2 — physical demonstrator**
- Build and test the shore-side seawater loop and redundant power/network arrangement first.
- Validate pressure, connectors, corrosion, biofouling, thermal rejection and retrieval on a non-critical instrument pod before considering compute.
- Only then compare a shallow subsea module against the shore-based control using the same workload and full metering boundary.

## 9. Evidence and references

- Microsoft Research, Project Natick facts and figures (240 kW, 12 racks, 864 servers, deployment at EMEC): https://www.microsoft.com/en-us/research/project/natick/facts-and-figures/
- Microsoft, Project Natick two-year trial summary (117 ft deployment): https://news.microsoft.com/source/features/sustainability/project-natick-underwater-datacenter/
- Canadian Centre for Cyber Security, *Cyber security considerations for submarine cables* (deep-water physical threats and layered mitigations): https://www.cyber.gc.ca/en/guidance/cyber-security-considerations-submarine-cables-itsp20001
- 2026 *Marine Policy*, *Mapping undersea cable risk from bathymetry to geopolitics*: https://doi.org/10.1016/j.marpol.2025.107012
- Copley et al., Longqi hydrothermal vent field, location/depth/ecology: https://www.nature.com/articles/srep39158
- *Active hydrothermal vent ecosystems in the Indian Ocean are in need of protection* (2022): https://doi.org/10.3389/fmars.2022.1067912
- UN World Ocean Assessment, hydrothermal vents and cold seeps: https://woa.un.org/third-world-ocean-assessment/changes-second-world-ocean-assessment/habitats/chapter-5-subchapter-5p-hydrothermal-vents-and-cold-seeps
- South African DFFE marine spatial-planning data, marine renewable-energy potential: https://www.dffe.gov.za/sites/default/files/reports/research/oceans/marinespatialplanning_nationaldata.pdf
- 2026 review, *Utilization of Seawater and Lake Water for Data Center Cooling*: https://doi.org/10.11174/dowas.27.1
- NIST SP 800-209, *Security Guidelines for Storage Infrastructure*: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-209.pdf
- PtX Hub / CSIR, *Load Connection Capacity Assessment of South African National Ports* (2025): https://ptx-hub.org/wp-content/uploads/2025/09/PtX-Hub-Load-Connection-Capacity-Assessment-of-South-African-National-Ports_final.pdf

---
**Claim ceiling:** this is a desk-screen and proposed qualification plan. It does not determine a safe depth, validate a security system, establish vent power capacity, select a South African site, grant environmental permission, establish grid connection, or prove economic/operational superiority.
