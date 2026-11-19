# Ocean Compute: Hybrid Cooling and Subsea Connectivity Benchmark v0.1

**Protocol:** OCEAN-COMPUTE-THERMAL-CABLE-001  
**Snapshot:** 19 November 2026  
**Status:** research and scenario contract; no site selected, no CFD validation or field qualification.

## Decision

Do not treat the sea as an unlimited cold sink, and do not treat two logical cable connections as independent. Qualify a complete coastal/offshore compute service across four coupled domains:

1. IT and cooling energy;
2. seawater intake, heat rejection and local marine environment;
3. submarine fibre/power route and landing failure domains;
4. repair, maintenance, local fallback and service recovery.

The commercial proposition is a **coastal compute service with adaptive thermal operation and repairable, geographically diverse connectivity**, not simply a sealed module on the seabed.

## New industry evidence

### Hybrid seawater cooling in warm, shallow waters

Mahmood, Bayhan and Al-Ansari, *Thermo-economic analysis of depth-optimised hybrid seawater cooling for offshore data centres in the shallow Arabian Gulf*, Applied Thermal Engineering 309 (November 2026), DOI [10.1016/j.applthermaleng.2026.133740](https://doi.org/10.1016/j.applthermaleng.2026.133740).

The paper models a 15 MW data centre and a three-mode cooling strategy:
- direct seawater / water-side cooling when conditions permit;
- partial water-side economiser plus mechanical chiller;
- chiller-only operation when the marine heat sink is too warm.

Its abstract reports 17.0 GWh/year of modeled system electricity versus 33.1 GWh/year for its air-cooled baseline and 19.5 GWh/year for a binary water-side economiser, plus modeled 20-year lifecycle cost reduction of 22.1%. It also reports material sensitivity to temperature: a ±1°C seawater-temperature bias changed annual electricity cost by −6.4% to +7.4%. These are model-specific results, not measured field outcomes, and must be compared only after the exact energy-metering boundary is checked.

**Paradigm shift:** optimise depth, intake/discharge geometry, exchanger approach, pump energy, chiller operation and heat-rejection limits together. Use adaptive mode switching and a mechanical fallback. Do not assume continuous free cooling or transfer Arabian Gulf results directly to South African waters.

Source: [Applied Thermal Engineering / Elsevier](https://doi.org/10.1016/j.applthermaleng.2026.133740).

### Submarine cable repair and concentrated redundancy

Palmer-Felgate, *Global Submarine Cable Faults and Repair Dynamics: A Statistical Analysis*, first published online 16 September 2026, DOI [10.3233/NICSP260018](https://doi.org/10.3233/NICSP260018).

The chapter analyses 3,224 valid repair records across 137 countries, from a database spanning 18 years. It reports 150–200 submarine telecommunications cable repairs annually, approximately 80% associated with fishing/anchoring, recurring multi-cable incidents, and recent repair times approaching 50 days in some contexts. It also describes concentrated redundancy risk when multiple cables share one corridor or chokepoint.

**Strict scope limit:** these are global submarine *telecommunications* cable data. They are not failure-rate estimates for submarine power cables, a local South African route probability, or reliability statistics for a proposed data centre. Do not transfer these percentages or annual counts to subsea power cables.

**Paradigm shift:** route diversity is a graph and failure-domain property, not the count of logical circuits. Distinct routes that share a beach manhole, landing station, trench, inland duct, substation, vessel repair dependency or permit authority may still share a common failure.

Source: [SAGE / NATO Science for Peace and Security](https://doi.org/10.3233/NICSP260018).

## Existing Luminous model boundaries

- Symtropy `symtropy-fluid` currently contains simplified SPH scaffolding with placeholder pressure/viscosity gradients. It is not admitted for ocean circulation, heat-plume, seabed or cable-load qualification.
- Symthaea `symthaea-thermofluids` provides elementary engineering calculations (Reynolds, Bernoulli, Darcy-Weisbach, conduction and cooling). It is not a calibrated marine CFD or environmental-plume model.
- The archived `symtropy-oceanic` concept was removed because its crate did not exist. Do not reactivate a generic ocean physics crate simply to produce a demonstration.
- Use explicit interfaces to independently validated external thermal/hydrodynamic/CFD/geotechnical solvers, plus local measurement and calibration. Solver adapters and each physical validation case must be qualified separately.

## Benchmark families

### A. Cooling system

Use identical IT heat/load traces, marine conditions and facility boundaries for:
1. air-cooled land facility;
2. closed-loop liquid-cooled land facility;
3. binary seawater economiser;
4. three-mode direct / partial / chiller hybrid;
5. floating or subsea arrangement with its real intake, discharge, access and maintenance geometry.

Report separately:
- cooling subsystem kWh per MWh of IT heat removed;
- facility PUE at a declared ISO/IEC 30134-2 boundary;
- pump, chiller, exchanger and auxiliary electricity;
- component COP and total system COP with equations;
- freshwater WUE and seawater intake/withdrawal/discharge volumes;
- intake depth and hourly temperature profile;
- discharge temperature, thermal plume and recirculation exposure;
- fouling, corrosion, biofouling, cleaning, chemistry and maintenance;
- lifecycle CAPEX/OPEX, material replacement and decommissioning.

Cooling-only energy must never be compared with whole-facility energy. PUE must not replace a subsystem heat/energy balance.

### B. Thermal robustness and ecology

The synthetic scenario matrix includes seasonal conditions, +1°C and +2°C intake temperature perturbations, marine heatwaves, stratification changes, current reversal, fouling, pump failure, chiller failure, intake obstruction and sensor faults.

These are stress cases, not environmental limits. Site-specific baseline data and environmental authorization define legal discharge conditions. No universal discharge ΔT or plume radius is implied.

### C. Subsea fibre and power connectivity

Compare single routes, shared-corridor dual routes, physically diverse routes/landings, terrestrial fallback and local autonomous operation. Model common-mode failures explicitly:
- shared route corridor / seabed hazard / fishing and anchoring exposure;
- common beach manhole, landing station, duct, inland backhaul;
- common substation, converter, cooling room or power source;
- simultaneous cable cuts, earthquake/landslide/turbidity current, dredging and abrasion;
- repair-vessel availability, spare cable, weather window, permits and access.

Use 1, 7, 14, 30 and 50-day outages as deterministic stress scenarios. The 50-day case is motivated by one telecommunications-cable repair study and is not a universal failure distribution.

### D. Service continuity and compute placement

Measure:
- critical port service continuity under loss of each single and multiple links;
- useful compute completed, on-time jobs, latency tails and state consistency;
- workload migration/replication feasibility under data-locality and destination-capacity limits;
- RTO/RPO and time-to-qualified restoration;
- energy, cost, emissions and storage consequences of local fallback;
- whether the backup path is genuinely independent of the failed physical assets.

A model-generated route diversity label does not establish independence. Unknown common failure domains remain unknown.

## First executable slice — 0D thermal balance verifier

A standalone, std-only Rust package is included in this research PR at `tools/ocean-thermal-balance-verifier/`.

It checks one declared steady-state interval:
- IT heat load = direct heat transferred to seawater + chiller evaporator heat;
- seawater heat pickup = mass flow × specific heat × intake/discharge temperature difference;
- expected seawater heat rejection = direct heat + chiller evaporator heat + chiller electric input + declared pump-heat fraction + other declared heat;
- cooling electric energy includes chiller, water-loop pump and explicitly supplied other cooling auxiliaries;
- optional site-supplied discharge-temperature and temperature-rise limits are tested without hardcoding a universal marine limit.

The package contains three synthetic reference modes (direct seawater, partial economiser + chiller, and chiller-only) and 17 authored unit tests. The synthetic examples were independently recalculated in Python; the three reference heat-balance residuals are zero. Rust tests have not been executed here because Cargo/rustc are unavailable.

The verifier assumes no thermal-storage change over the interval, no unlisted heat path, and synchronized input boundaries. It is strictly a 0D arithmetic sanity check; it does not model heat-exchanger performance, intake/discharge hydraulics, temperature stratification, current-driven recirculation, plume dispersion, entrainment, ecological impacts or legal limits.

## Acceptance gates

- Every site-specific temperature, depth, flow, bathymetry, cable, seabed, habitat, repair and environmental claim has source, date, units, evidence class, uncertainty, rights and applicability.
- Unknown fields remain null/unknown, not plausible defaults.
- Cooling subsystem energy and heat rejection close under the declared meter boundary.
- External CFD / thermal / hydrodynamic models have explicit validation cases, mesh/time-step identity and uncertainty.
- No PUE/WUE/environmental-compliance claim is derived from a synthetic heat balance.
- No local cable failure probability is inferred from global telecom repair counts.
- No positive route independence claim if common route, landing, duct, substation, repair or permitting dependencies are shared or unknown.
- All outage, censored and not-evaluable scenarios remain in published denominators.

## Cross-project owners

- Sol-Atlas: evidence-grade bathymetry, marine constraints, cable routes, landings, hazard overlays, rights, CRS/datum and source reversibility.
- Symtropy: deterministic energy/thermal accounting, fault/recovery replay, cable dependency cut sets and service scenario execution; not unvalidated CFD.
- Symthaea: candidate designs, sensitivity and information-gathering proposals; not site approval or safety authority.
- Mycelix: source permission, ownership, authorization, observations, repairs, incident and approval receipts; a DHT record is not physical proof.

Compose with Symtropy #6211/#4582/#6209/#2721, Sol-Atlas #942/#654, Symthaea #11572/#11473 and Mycelix #5974/#4882. Do not create a second energy ledger, GIS or port W/Wh verifier.

## Claim ceiling

This contract establishes research comparisons and scenario coverage only. It does not select a subsea site, validate thermal plume or cable mechanics, establish environmental compliance, prove commercial superiority, certify route independence, or authorize construction/operation.


### Cross-owner dependency correction

The cable/fibre/power-route inventory and its transitive shared dependencies should compose with Mycelix [ASSURE-PORTFOLIO-CUT-002 #5762](https://github.com/Luminous-Dynamics/mycelix/issues/5762), which owns multi-valued dependency identities and common-cause cut-set semantics. Sol-Atlas supplies the spatially grounded route/landing/duct/substation/geohazard evidence. Symtropy consumes the resulting dependency/failure-domain references to simulate the service consequences and restoration trajectories. This avoids a competing second dependency graph or a false independence score.
