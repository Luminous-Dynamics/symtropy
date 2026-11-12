# Mine-Shaft Pumped-Storage Qualification Benchmark v0.1

**Status:** research benchmark / evidence contract, 9 November 2026. Not a mine feasibility study, pressure-boundary design, geotechnical safety case, water-use authorization or construction package.

Related Symtropy issue: [MINE-SHAFT-PUMPED-STORAGE-001 #4424](https://github.com/Luminous-Dynamics/symtropy/issues/4424).

## 1. System boundary and the first-principles constraint

The electrical storage service is the energy returned to the grid/load after a complete charge–hold–discharge cycle, not the gross gravitational potential calculated from a shaft depth.

For water-based pumped hydro:

`E_gross = rho * g * V_usable * H_net`

where:
- `rho` is the density of the water actually used (kg/m³);
- `g` is standard gravity (m/s²);
- `V_usable` is the cyclic volume between approved upper/lower operating limits (m³), not total geometric mine void volume;
- `H_net` is the net elevation/head at the declared hydraulic boundary, after conveyance and machine losses as appropriate (m).

In kWh:

`E_gross_kWh = rho * g * V_usable * H_net / 3,600,000`

The net stored electrical service requires separate pump and turbine/generator/converter maps, auxiliaries, standby/holding losses, water losses, and operating chronology. Round-trip efficiency is measured at a single declared electrical boundary:

`RTE = E_net_exported / E_net_imported`

A shaft gives head, not necessarily volume. Connected workings may supply volume, but only if their geometry, hydraulic connectivity, sealing, rock support, water quality and cyclic stability are demonstrated.

### Synthetic dimensional examples (not site estimates)

At 1,000 kg/m³ water density and 100% ideal conversion:
- 1 m³ at 100 m net head contains about 0.272 kWh of gravitational potential;
- 1 m³ at 500 m contains about 1.36 kWh;
- 1 m³ at 1,000 m contains about 2.72 kWh;
- 1,000,000 m³ at 400 m contains about 1.09 GWh gross.

These values are energy upper bounds before pump/turbine/generator/converter losses, head losses, water constraints, standby loss and availability. They illustrate why a very deep but narrow shaft may still provide little usable storage energy.

## 2. Architectures to compare — keep distinct

### A. Mine shaft as lower reservoir, surface reservoir as upper reservoir

Water is pumped from a verified underground lower reservoir to a surface upper reservoir, then passed through a reversible pump-turbine during discharge. The mine workings serve as a storage reservoir only if containment and geomechanical suitability are verified.

### B. Mine workings as lower reservoir, shaft as hydraulic conduit

A connected mine network may provide greater usable volume than the shaft alone. The shaft may carry the penstock/riser. This configuration has a larger geotechnical and hydrogeological evidence burden because the hydraulically connected volume, flow paths and sealing are not necessarily visible in the shaft survey.

### C. Mine workings as upper reservoir

A mine void at a verified higher elevation may serve as an upper reservoir, with a surface or lower-level reservoir. The same containment, access, pressure and water-quality requirements apply. Mine water and clean storage water must not be assumed interchangeable.

### D. Existing reservoir pair with an underground conveyance segment

An existing surface reservoir pair may already provide most storage capacity, while a mine shaft or tunnel could be a conveyance/plant route. In that case the underground asset is an infrastructure component, not the energy store itself.

### E. Solid-mass gravity storage in a shaft

A mass is hoisted/translated vertically. This is not water-based pumped hydro. It has its own mechanical drive, cable/rail, braking, mass, structural fatigue, energy conversion and safety boundaries. It must not reuse water storage capacity or pump/turbine efficiency assumptions.

### F. Mine-water thermal storage

Heat/cold stored in rock and mine water is distinct from electrical pumped hydro. Symtropy [MINE-THERMAL-STORE-001 #4070](https://github.com/Luminous-Dynamics/symtropy/issues/4070) owns the thermal, hydrogeochemical and rock/water coupling. Sol-Atlas [SA-MINE-WATER-THERMAL-SERVICE-001 #690](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/690) owns South African source-to-consumer heat/cooling service assessment.

## 3. Candidate screening sequence

Do not begin with the most attractive shaft depth. Use a gated sequence.

### Gate 0 — asset identity, rights and closure

Require:
- exact mine, shaft, level, working and surface-asset identities;
- current operator, owner, custodian, land/access rights and jurisdiction;
- mine closure, rehabilitation, financial provision and long-term liability status;
- approved survey/plans, revisions, coordinate reference and datum;
- permitted access, inspection, water-use, abstraction, reinjection and discharge boundaries;
- community, worker, environmental, cultural and land-use constraints.

No owner/access/closure evidence means no site work or eligibility claim.

### Gate 1 — geometry and usable water volume

Required:
- shaft diameter, depth, lining geometry and usable level range;
- connected workings, cross-cuts, stopes, bulkheads, seals, fill and backfill;
- upper/lower reservoir geometry and operating levels;
- survey uncertainty, water-level chronology, void volume and dead/storage-inaccessible volume;
- water balance, groundwater ingress, existing dewatering, leakage and replenishment;
- sediment, debris and suspended solids.

Geometric volume is not usable storage volume. Exclude dead zones, unverified workings, unstable areas and volumes that cannot be controlled or isolated.

### Gate 2 — geomechanical and hydrogeological integrity

Require specialist investigation of:
- rock type, mineralogy, discontinuities, faults and stress state;
- shaft lining/steel/concrete condition, support, corrosion, spalling and degradation;
- subsidence, convergence, deformation and mine-induced damage;
- permeability, hydraulic connectivity, leakage pathways, pore pressure and groundwater levels;
- cyclic wetting/saturation and pressure loading;
- hydraulic isolation/containment and potential connections to other workings or aquifers;
- long-term stability under the declared operating cycles and service life.

A GIS mine boundary, shaft depth or historical mine plan does not establish structural or hydraulic tightness.

### Gate 3 — hydraulic design envelope

Require:
- surveyed elevation datum and gross/usable head at all operating states;
- flow rate, conveyance length, diameter/roughness and local-loss inventory;
- branch/bypass and isolation configuration;
- water density/temperature/chemistry;
- upper/lower reservoir operating windows and ramp rates;
- pump-mode and turbine-mode equipment maps at the actual operating envelope;
- surge/water-hammer/load-rejection/valve-closure specialist analysis;
- cavitation, air entrainment, sediment and transient operating exclusions;
- emergency shutdown and safe dewatering/fill procedures.

Do not use the Symtropy conduit-PAT rectangular map cell as a substitute for a large reversible pumped-storage unit model. The machine and operating domain must be separately qualified.

### Gate 4 — electrical and system interface

Require:
- pump input, turbine output, generator, converter and transformer maps;
- auxiliary and standby power;
- grid/PCC import/export and protection requirements;
- power quality, ramp, response and ancillary-service capability;
- storage duration and dispatch window;
- maintenance, outage, start/stop and cycling limits;
- metering calibration, time synchronization and import/export register direction.

### Gate 5 — environment, water and whole-life

Require:
- mine-water contamination and treatment needs;
- clean/contaminated circuit separation;
- water abstraction, reinjection and discharge approval;
- aquifer/receiving-water effects and monitoring;
- acid mine drainage and mine-water remediation obligations;
- shaft/working access, ventilation, rescue and emergency response;
- refurbishment, replacement, maintenance, closure and remediation liability;
- local supplier capability, spares, lifting, access and outage costs.

Energy storage does not override water security, remediation or mine-closure obligations.

## 4. Matched competitor scenarios

All candidates use the same grid-service trace and duration requirement.

| ID | Candidate | Comparison purpose |
|---|---|---|
| B0 | No-build / closure / remediation | Preserve no-build as a legitimate result |
| B1 | Existing conventional surface-reservoir PHS | South African operating baseline |
| B2 | New conventional reservoir-pair PHS | New-build comparison |
| B3 | Mine-shaft UPHS, upper surface reservoir | Main water-based reuse concept |
| B4 | Connected mine-working lower reservoir UPHS | Larger-volume underground reuse concept |
| B5 | Mine shaft as upper reservoir | Alternative elevation arrangement |
| B6 | Solid-mass gravity storage in shaft | Distinct non-water gravity architecture |
| B7 | BESS | Shorter-duration storage comparator |
| B8 | CAES / other mechanical storage | Alternative long-duration storage comparator |
| B9 | Hybrid PV/wind + BESS + PHS/UPHS | Portfolio-level dispatch comparison |

The 2026 Tubatse announcement (1.5 GW / ~21 GWh / up to 14 h) is a South African *conventional reservoir* development benchmark. It is not a mine-shaft feasibility result.

## 5. Reference calculations and independent cross-checks

### Hydraulic energy and power

`P_hydraulic = rho * g * Q * H_net`

`E_hydraulic = integral(rho * g * Q(t) * H_net(t) dt)`

The reported output must separate:
- hydraulic energy into the pump/turbine;
- pump motor and pump conversion losses;
- stored potential energy and storage-period leakage/standing losses;
- turbine shaft output and turbine loss;
- generator/converter gross output and losses;
- auxiliary loads;
- net grid import and net grid export.

### Required conservation checks

- Mass/water balance over each reservoir and the full system.
- Hydraulic energy consistency across flow/head/conveyance.
- Electrical balance across input, output, auxiliary and losses.
- State-of-charge/usable-water-volume continuity over time.
- No simultaneous charge/discharge unless physically modelled as two independent branches and permitted.
- No negative or invented storage state.
- No free water replenishment or free auxiliary energy.
- Import/export meter direction is explicit and cannot be inferred from sign without a declared convention.

### Round-trip efficiency

The headline RTE must use the same net electrical import/export meter boundary over a complete cycle with declared start and end inventory. Report separate:
- pump-to-reservoir hydraulic efficiency;
- turbine-generator efficiency;
- converter/transformer efficiency;
- water/conveyance/standing losses;
- total grid-to-grid RTE.

Do not multiply nameplate efficiencies and call the result measured RTE.

## 6. Synthetic benchmark corpus

Start with at least 24 frozen deterministic cases:
1. Ideal single-reservoir-pair known answer.
2. Shaft-only volume with no viable upper reservoir — no capacity credit.
3. Deep shaft, low volume — high head does not imply high MWh.
4. Large connected workings with uncertain geometry — needs investigation.
5. Shaft lining defect/leakage.
6. High-permeability connected workings — containment fails.
7. Groundwater ingress / water balance not closed.
8. Insufficient source water / replenishment.
9. Water chemistry outside material/equipment limits.
10. Head loss exceeds available gross head.
11. Pump/turbine map outside operating envelope.
12. Pump input overload / motor limit.
13. Turbine minimum stable flow failure.
14. Load rejection / pressure surge requires specialist analysis.
15. Valve closure outside declared envelope.
16. Storage level outside approved operating bounds.
17. Simultaneous charge/discharge request.
18. Auxiliary consumption exceeds plausible net output.
19. Grid outage during pumping.
20. Grid outage during generation.
21. One unit unavailable / maintenance case.
22. Water-service / mine remediation constraint violation.
23. Meter direction/timebase ambiguity.
24. Closure/remediation liability omitted from cost model.

For each case preserve input manifest, exact asset/model revisions, units, evidence class, assumption provenance, expected disposition and independent balance checks.

## 7. Performance and acceptance metrics

Report separate:
- usable storage MWh and the range/uncertainty of that value;
- rated and sustainable MW;
- discharge duration at matched output;
- net grid-to-grid RTE;
- response, ramp and dispatch availability;
- hydraulic loss, leakage and water consumption;
- pump/turbine efficiency maps and operating-point coverage;
- geotechnical/hydraulic integrity evidence coverage;
- cyclic deformation and leakage observations;
- maintenance, outage, MTTR and component lifetime;
- CAPEX/OPEX, refurbishment, replacement, closure liability, environmental/monitoring cost and finance;
- delivered grid service (MW, MWh, duration, response, reserve);
- evidence uncertainty and independent-review status.

### Hard acceptance gates

- Zero false-eligible sites in the hard-negative corpus.
- Missing geotechnical, hydrogeological, water-rights or closure evidence blocks the relevant claim.
- No usable MWh claim based on shaft depth or geometric volume alone.
- Full water/energy/meter balances close within declared measurement/model uncertainty.
- No storage energy or grid-service credit without the complete import/export boundary.
- No claim of pressure-surge/water-hammer safety without a specialist independent analysis.
- No mine-water/thermal energy double count.
- No single weighted score can compensate for an unsafe shaft, environmental breach or unapproved water use.

## 8. SOTA / industry benchmark protocol

Use:
- Eskom Drakensberg, Palmiet and Ingula for existing South African operating PHS context;
- Eskom Tubatse RFQ as a current South African conventional reservoir development comparison;
- Colas et al. (2025) for abandoned-coal-mine geology/containment workflow;
- Amorim et al. (2026) for solid and liquid gravity-storage architecture taxonomy and evidence gaps;
- Zhang et al. (2026) as an integrated mine energy hub *model* comparator, not a field plant;
- BESS/CAES alternatives at the same MW/MWh/duration/response requirements.

A “better than SOTA” result must demonstrate a matched improvement in lifecycle cost or grid service without weakening geotechnical, water, environmental or safety requirements. The first success may be proving that a mine-shaft concept is *not* feasible and that an alternative wins; that is useful engineering evidence.

## 9. Claim ceiling

This benchmark defines an evidence and comparison protocol. It does not determine that any South African mine is suitable, establish water availability, validate a reservoir or pressure boundary, authorize dewatering/refilling, or certify a pumped-storage plant. Physical inspection, boreholes, pressure testing, mine access and water interventions require owner/operator permission, licensed engineering, statutory authorization, independent review and a site-specific safety/environmental case.
