# Habitat service integration bench — M0 design basis v0.1

**Parent:** Symtropy [HABITAT-SERVICE-INTEGRATION-BENCH-001 #4531](https://github.com/Luminous-Dynamics/symtropy/issues/4531)  
**Research cut:** 10 November 2026  
**Branch:** `research/habitat-service-bench-m0-v0.1`  
**Classification:** requirements and interface design only

## 1. Purpose and limits

Define an unoccupied, room-scale test cell that can independently qualify, then integrate, WaterLoop, HeatBank and AirCare service modules. This package creates a design-review starting point; it is not an engineering drawing, a selected site, a bill of materials, a construction specification, an electrical/plumbing/pressure design, or authorization to purchase, construct, energize or operate.

All dimensions, capacities, materials, temperatures, pressures, airflow, electrical ratings, duty cycles and acceptance tolerances remain unselected. They require actual service profiles, measurement error budgets, hazard analysis and qualified professional review.

The test cell is **unoccupied only**. No person is exposed to outage, heat, cold, contaminants, water-treatment risk or other stress to generate benchmark data.

## 2. Owner map

| Concern | Authoritative owner | Bench responsibility |
|---|---|---|
| Household service definitions | Symthaea HOME-SVC #10322 and WaterLoop #10323 / HeatBank #10329 / AirCare #10330 | Consume each product's exact service and test endpoints |
| Cross-appliance repeatability and trial design | Symthaea HOME-SVC-METROLOGY #10341 | Separate repeatability, inter-lab reproducibility, production variation and household validity |
| Building instrument profiles | Mycelix #6025 / draft PR #6028 | Consume exact admitted measurand/channel profiles; no sensor is qualified by this document |
| Service availability and stable recovery | Symtropy #1848 / implementation PR #1849 | Consume owner-native Restored/Lost service events only; physical measurements remain separate |
| Thermal-outage arithmetic | Symtropy #4507 / generic arithmetic PR #4508 | Consume metric definitions without mislabelling generic degree-hours as published DHR |
| Post-occupancy outcome comparability | Sol-Atlas #633; service mapping #700 | Later read-only outcome binding; no post-occupancy claim from an unoccupied bench |
| Resident authority / physical effects | Mycelix Hearth #2328 | No action authority is created here |

## 3. Functional decomposition

The cell comprises six conceptual modules. These are functional boundaries, not selected pieces of equipment.

**CELL_ENVELOPE** defines the controlled unoccupied test boundary, geometry revision, openings, service access and sensor placement references. The envelope must be characterized independently before a thermal response is attributed to a candidate design.

**WATERLOOP_SKID** is a source-identified, physically segregated non-potable test loop. It needs clear boundaries for feed, storage, usable delivery, rejects/purges and residual routing. Potable connection is not assumed. Any connection to a building potable system, greywater stream, sewer or aerosol-producing use is excluded from this package.

**HEATBANK_SKID** is an isolated thermal/hot-water service loop with controlled draw emulation. It must separate source heat, delivered useful thermal service, storage/distribution losses, and each electrical input. Any pressure vessel, hot surface, refrigerant circuit, electrical heater or hygiene-control function requires a separate qualified design and hazard review.

**AIRCARE_SKID** defines outdoor-air, supply, exhaust/return and room measurement boundaries. Filtration, ventilation, humidity/condensate, filter loading, occupied-zone acoustics and electrical use are separate endpoints. No hazardous pollutant/aerosol challenge is authorized by this design basis.

**ELECTRICAL_AND_THERMAL_METERING** defines a source-to-load allocation so that electricity, useful heat, thermal loss and backup inputs are attributed once. A device command or nameplate rating is not a meter reading; compressor COP is not whole-system delivered service.

**METROLOGY_AND_LOCAL_ACQUISITION** is read-only by default. It consumes Mycelix's building profile and retains raw/derived distinction, instrument identity, firmware, installation, calibration/reference check, uncertainty, response, timebase, method and fault state.

## 4. Physical boundary and interface inventory

The manifest contains 14 candidate interface identities, each with sizing and construction details intentionally unselected:

- Water source input, named non-potable service output, residual stream and physical isolation boundary.
- Thermal source input, useful thermal service output and measured loss boundary.
- Outdoor-air inlet, room supply delivery and exhaust/return boundary.
- Main electrical source and separately metered subsystem branches.
- Read-only observation/data path and explicit absence of model/device actuation authority.

A future interface specification must bind the exact upstream/downstream owner, connection geometry, fluid/air/electrical class, operating envelope, isolation method, metering point, unit, uncertainty, maintenance access, failure state and approval authority. It must not reuse an interface ID for a changed physical configuration.

## 5. Metrology gap found before the bench is designed

The draft Mycelix building profile currently has 14 channel definitions. They cover air temperature, derived mean radiant temperature, relative humidity, air speed, CO2, PM2.5, outdoor-air flow, water flow and volume, pH, conductivity, electrical power/energy and useful thermal energy.

The bench likely needs additional channels that are **not yet defined** in profile v0.1, including water pressure, filter/duct differential pressure, water temperature, tank level, sound pressure at declared positions, voltage/current/power quality, filter loading and selected gas-contaminant methods.

Do not invent channel IDs, silently extend the schema, or map these to a nearby-looking existing field. The exact channel must remain `UNMAPPED_TO_PROFILE` until Mycelix versions and admits it. In particular:

- Tank level is not volume without a verified tank geometry and calibration curve.
- Pump current is not water flow.
- Fan command or free-air rating is not installed outdoor-air delivery.
- Air temperature is not water temperature or operative temperature.
- CO2 is not a universal IAQ measure.
- CADR is not outdoor-air ventilation.
- Electrical power is not accumulated energy.
- Useful thermal energy requires a declared calculation/meter boundary.

## 6. Authority and control boundary

M0/M1 operation is read-only observation and isolated manual test controls. Symthaea and Symtropy cannot dispatch device commands from a model recommendation. A Mycelix/Hearth physical-effect permission, local exclusive executor and crash/reconciliation process would be necessary before any future automated actuation; none is configured by this work.

Cloud services are not assumed for observation or essential-service operation. Manual safe isolation is a requirement still needing an engineering design and verification, not an established feature.

## 7. Stage gates

**M0 — design and safety preflight:** site permission/suitability; cell geometry/envelope; structural and anchorage; electrical/earthing/protection; water separation/backflow/residual; pressure/thermal/burn; air/condensation/materials; fire/egress/emergency isolation; hazard analysis; metrology/error budgets; data/privacy/rights.

**M1 — single-service and metrology:** exact instrument/model/firmware identity; calibration within range; installation/placement; local read-only acquisition and synchronized clock; independent cross-check or assay where required; single-service balance closure.

**M2 — ordinary and concurrent integration:** the same task and exogenous traces for all architectures; resource attribution with no double counting; delivered/unmet service by product; shared-resource and common-mode review.

**M3 — controlled unoccupied disruption:** only after a reviewed safe operating envelope and approved fault-injection plan. Include recovery, repair, re-verification and repeat-fault behavior. Life-safety system disablement and hazardous indoor challenge are out of scope.

**M4 — transfer evidence:** report within-unit repeatability, inter-lab reproducibility, production-unit variation and household ecological validity as separate claims. A room-scale unoccupied bench cannot establish household validity.

## 8. Architecture comparison

The comparison arms remain unranked:

1. Independent water/thermal/air systems with separately metered service boundaries.
2. Shared utilities only where exact physical separation/interfaces are reviewed and common-mode/cross-contamination risks are addressed.
3. Passive-first hybrid with bounded local storage, local control and manual fallback; no assumed autonomy duration.

All arms must receive the same exact exogenous trace bytes, event order, initial state, information budget, task request and observation window. Each uses the same measurement boundary and service acceptance definition. A lower annual or subsystem energy number cannot compensate for a service-floor failure, quality-assay failure, inaccessible repair, unsafe state or non-consensual telemetry.

## 9. Standards and comparator selection

Standards are test-method references only after exact edition, scope and local applicability are verified.

- IEC TR 63250:2021 provides household-appliance repeatability/reproducibility and uncertainty guidance; it does not establish between-unit production variation or household ecological validity.
- HeatBank's heat-pump-water-heater method candidates include EN 16147:2017+A1:2022 and ISO 19967-1:2019 with defined draw profiles and conditions. IEC 60379:2023 is for electric storage water heaters and does not itself specify safety requirements.
- ASHRAE 62.2-2025 is a residential minimum-ventilation reference; it is not a pollutant-specific IAQ guarantee or thermal-comfort standard.
- ISO/CD 25446 remains a draft under development, not a final greywater-reuse approval basis.
- ASHRAE Standard 140 capability tests do not calibrate the actual M0 cell; measured reference comparison remains necessary.

Use the product-owner protocols and exact comparator products; do not invent a generic vendor baseline or claim a standard-compliance result from this design package.

## 10. Current status

The machine-readable manifest deliberately records:

- No site, geometry, dimensions, materials, ratings or equipment selected.
- No authority to construct, purchase, energize, pressure-test or operate.
- No instruments physically admitted.
- All stage gates M0–M4 uncompleted/blocked in order.
- No occupied use, potable-water route, hazardous aerosol challenge or device-control authority.

This package is ready for design review and owner-schema gap work only.