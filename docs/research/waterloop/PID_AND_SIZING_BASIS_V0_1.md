# WaterLoop First-Bench P&ID and Parametric Sizing Basis v0.1

**Status:** Preliminary engineering basis for review; not a construction release  
**Date:** 2026-11-15  
**Parent:** Symtropy #5335  
**Design basis:** WaterLoop physical demonstrator PR #5336

## 1. Design intent

Build the smallest useful isolated test rig that measures real delivered water service, pressure/flow, pump energy, leak response and water balance. The first unit is a **test-cell demonstrator**, not a household installation and not a potable-water treatment product.

The first pass intentionally excludes:
- connection to a live municipal service;
- connection to any occupied plumbing;
- potable-water treatment or drinking/cooking use;
- chemical dosing, disinfection, pathogen removal or health claims;
- wastewater discharge;
- automatic remote actuation;
- a treatment train before source/use analysis proves it is necessary.

Use a separately filled, declared test-water reservoir. Keep the test outlet, collected discharge and any later non-potable recovery circuit physically distinct. The controls are local and manually overrideable; telemetry is read-only.

## 2. P&ID-style functional diagram

This is a functional instrument/valve diagram, not a pipe-installation drawing. Exact pipe sizes, fittings, materials, relief settings, electrical protection, vents, backflow provisions and installation details require qualified review.

```text
                    FILL / SOURCE SIDE
       Declared test-water fill tote / supply
                         |
                  HV-101 (manual)
                         |
                  FIT-101 (inlet flow)
                         |
                  NRV-101 (check)
                         |
             +-----------v------------+
             | TK-101 BACKUP STORAGE   |
             |                          |
             | LT-101  level            |
             | TT-101  temperature      |
             | sample point SP-101      |
             | screened vent / cover    |
             | overflow -> FT-103       |
             | bottom drain -> FT-104   |
             +-----------+--------------+
                         |
                  HV-102 (isolation)
                         |
                  STR-101 (strainer)
                         |
                  PIT-101 (suction)
                         |
                  P-101 (pump + motor)
                         |
                  PIT-102 (discharge)
                         |
                  FIT-102 (delivery flow)
                         |
                  HV-103 (isolation)
                         |
                  PRV-101 (if required by
                  reviewed design)
                         |
                  DV-101 (demand emulator)
                         |
                  SP-102 outlet sample
                         |
                  +------v-------+
                  | TEST LOAD    |
                  | / collection |
                  +------+-------+
                         |
                     FT-105
                  delivered volume
                         |
                  measured collection
                  vessel / load cell

PUMP DISCHARGE BYPASS (if needed for safe test):
PIT-102 --> HV-104 --> measured return-to-TK-101
                           |
                     FIT-106 (bypass)

TK-101 OVERFLOW --> FT-103 --> contained overflow vessel
TK-101 DRAIN -----> FT-104 --> contained drain vessel
TEST LOAD DRAIN --> FT-105 --> separate collection vessel

POWER:
Protected local supply --> lockable isolator --> E-101 power/energy meter
                         --> P-101 motor
                         --> local stop / emergency isolation

DATA:
FIT / PIT / LT / TT / E / valve-state / leak-tray sensor
                         |
                  local logger (read-only)
                         |
         later Symtropy / Symthaea evidence adapter

NO CONNECTION between this backup/service circuit and any future
non-potable recovery or potable circuit.
```

### Instrument and valve tags

| Tag | Function | Minimum meaning |
|---|---|---|
| TK-101 | Backup/storage test vessel | Exact geometry, material, nominal/working volume, inspection and drain access |
| FIT-101 | Source inlet flow meter | Total source volume entering TK-101 |
| LT-101 | Storage level | Level only; volume is derived only through a calibrated geometry relation |
| TT-101 | Water temperature | Temperature at the declared tank location |
| PIT-101 | Pump suction pressure | Gauge/absolute reference must be declared |
| STR-101 | Suction strainer | Serviceable; differential pressure measurement may be added for test |
| P-101 | Pump and motor | Exact model/curve/revision only after duty-point selection |
| PIT-102 | Pump discharge pressure | Pressure at a defined location and datum |
| FIT-102 | Outlet flow meter | Flow entering the test-load boundary |
| DV-101 | Controlled demand valve/emulator | Repeatable demand steps; state logged |
| PRV-101 | Pressure protection/regulation | Setpoint and discharge path determined by the reviewed system design |
| FIT-103/104/105 | Overflow/drain/delivery measurement | Each distinct water path accounted once |
| E-101 | Electrical power/energy meter | Pump and auxiliary energy boundary explicitly stated |
| SP-101/102 | Sample locations | Sample identity, point, time, method and intended test-use association |
| LS-101 | Leak tray/moisture switch | Containment alert; not a substitute for metered leak quantification |
| E-STOP-101 | Local emergency stop/isolation | Local human-controlled safe state |

The final tag list may change after test-cell integration, but the measurement boundaries and semantic identities must not.

## 3. Candidate first-bench configuration

The first demonstrator should compare gravity delivery and pump-assisted delivery using the same measured source vessel and outlet load.

### Candidate test envelope (not a legal threshold or product specification)

| Variable | First sweep proposal | Why |
|---|---:|---|
| Flow points | 5, 10, 20 L/min | Low / nominal / higher demand sweep |
| Outlet pressure points | 100, 200, 300 kPa gauge where safely supported | Characterize service envelope rather than one best point |
| Demand profiles | steady, pulsed, step change, intermittent | Expose cycling and transient response |
| Storage fill states | 100%, 75%, 50%, minimum deliverable reserve | Separate inventory from usable service |
| Pump cases | gravity, fixed-speed, variable-speed if candidate supports it | Compare control/energy behavior |
| Supply state | normal fill, fill interruption, low inlet head | Test buffer behavior |
| Electrical state | normal, loss, restoration | Test service loss and restart/fault response |

These are proposed bench test points. They do not imply that every household fixture must operate at those conditions, nor do they set the required service floor for a product deployment.

### Vessel sizing relation

Select nominal volume only after fixing the test duration and usable-volume fraction:

`V_nominal >= Q_test × t_test / f_usable`

where:
- `Q_test` is the outlet flow in L/min;
- `t_test` is the desired duration in minutes;
- `f_usable` is the measured fraction deliverable above the minimum pump/fixture operating level.

Example sizing only: for a 10 L/min, 15-minute run and a planning assumption of 0.8 usable fraction, nominal volume is at least 187.5 L. This is a planning example, **not** an assumption that 80% of a tank is always usable. Determine actual usable volume from tank geometry, outlet/suction elevation, pump minimum inlet condition, air-ingress margin, measurement uncertainty and the service boundary.

A compact ~250–300 L demonstrator vessel is a reasonable candidate for short shakedown trials, but endurance duration and product storage capacity remain open decisions.

## 4. Hydraulic sizing equations

### 4.1 Pump head

Required total dynamic head is the sum of:
- required outlet gauge pressure converted to head;
- static elevation difference;
- pipe/fitting/valve/filter losses;
- any pressure margin required by the declared service profile.

`H_total = Δp_required/(ρg) + Δz + h_f + h_minor`

Do not select a pump from maximum-head or maximum-flow headline values. The candidate must meet the **intersection** of the actual pump curve and the system curve across the declared storage states and flow sweep.

### 4.2 Hydraulic and electrical power

`P_hydraulic = Δp × Q = ρgQH`

`P_electrical ≈ P_hydraulic / η_wire_to_water + P_auxiliary`

At 200 kPa pressure rise:
- 10 L/min requires approximately 33.3 W hydraulic power before losses;
- 20 L/min requires approximately 66.7 W hydraulic power before losses.

For a purely illustrative 40% wire-to-water efficiency assumption, those points correspond to approximately 83 W and 167 W before auxiliary loads and any additional static/friction head. Efficiency must be measured from synchronized electrical input and hydraulic output; 40% is not a claimed pump efficiency.

Measure:
- voltage, current, real power and energy;
- flow and pressure at matching timestamps;
- pump speed/state if available;
- bypass and auxiliary loads;
- starts/hour, run duration and pressure/flow transient.

### 4.3 Pipe velocity

`v = Q / (πD²/4)`

Calculate with measured internal diameter, not nominal trade size. Check velocity, suction losses, noise, water hammer, air ingress and cavitation/NPSH conditions against the selected components' actual documentation. No pipe size is released by this document.

## 5. Water balance and metrology

For the test boundary:

`V_start + V_source_in = V_delivered + V_overflow + V_drain + V_bypass + V_end + V_residual`

`R_V = V_start + V_source_in - V_delivered - V_overflow - V_drain - V_bypass - V_end - V_residual`

The evaluator must report `R_V`, the combined/expanded measurement uncertainty and all unmeasured paths. Do not coerce a residual to zero.

For a closed-loop recirculation test, recirculated flow is an **internal transfer**, not new supply or new delivered service. Do not sum the same physical flow at multiple meters.

A proposed closure gate is:

`|R_V| <= U_95(R_V)`

where `U_95` is the expanded uncertainty for the actual measurement model and its covariance assumptions. The coverage factor and uncertainty propagation method must be declared. If uncertainty cannot be defensibly estimated, report closure as not qualified rather than inventing a percentage tolerance.

### Calibration / verification

- Source/outlet meters: traceable volume or gravimetric comparison at multiple flow points.
- Pressure instruments: zero check and reference comparison across the test range.
- Level: tank-specific geometry calibration; confirm at multiple fill heights.
- Temperature: reference thermometer check across the expected range.
- Electrical meter: documented accuracy class and verification procedure.
- Logger: synchronized clock, sampling cadence and missing-data semantics.

Sampling frequency should be selected from the fastest transient being assessed. It must not be chosen after looking at results. Preserve raw channel samples, calibration references and timestamp semantics.

## 6. Operating and fault states

| State | Trigger | Expected system behavior |
|---|---|---|
| Ready | Fill and checks complete | Local manual start permitted |
| Normal service | Valid source / storage and operating range | Deliver measured flow and pressure |
| Low reserve | Tank level below tested deliverable reserve | Warn and stop service-crediting below boundary |
| Low suction / pump protection | Pressure or level outside pump envelope | Locally isolate/stop per reviewed pump protection |
| Outlet overpressure | Reviewed protection threshold reached | Local protective action; no cloud dependency |
| Leak containment | Tray sensor or measured unexplained outflow | Alert and local safe isolation under approved test procedure |
| Sensor disagreement | Independent channels inconsistent | Mark state unknown; do not credit unverified delivery |
| Power loss | Supply interruption | Stop pump safely; manual/gravity fallback only if separately designed |
| Recovery | Power/source restored | Require reconciliation and local restart check |
| Maintenance | Isolation and lockout applied | Service access; return to service only after leak/functional check |

No remote AI action is permitted in this demonstrator. Symthaea may analyse read-only observations and produce operator-facing recommendations. Local safety devices and human authority remain independent.

## 7. Stage-gated test programme

### T0 — dry integration
- inspect physical assembly, supports, materials, wiring isolation and guarding;
- verify valves, labels, access, drains, overflow and emergency stop;
- no water-quality or hydraulic-performance claim.

### T1 — static fill / leak test
- fill with declared test water;
- inspect all joints and containment;
- verify inlet, level and overflow measurements;
- test drain and isolation behavior.

### T2 — gravity delivery baseline
- fixed head / storage states;
- sweep flow and outlet pressure;
- quantify useful delivery and residual heel;
- establish a no-pump energy baseline.

### T3 — pump-assisted service
- same storage, demand and initial conditions as T2;
- map flow/pressure/duty points;
- measure wire-to-water energy and cycling;
- check suction and protection behavior.

### T4 — fault injection
- controlled leak path with independent reference volume;
- demand shift / valve transient (no leak);
- blocked strainer surrogate;
- biased/stuck level or flow channel;
- pump power loss and restoration;
- outlet restriction and overflow obstruction;
- common-mode logger/power fault.

### T5 — independent reconciliation
- reproduce delivered/residual volume independently;
- compare logger-derived calculations with reference measurement;
- verify exact configuration and calibration identities;
- freeze a reproducible run receipt.

Non-potable recovery and any treatment module remain a separate later stage with its own feed characterization, source/use boundary, analyte panel, laboratory method, residual route and independent qualification.

## 8. Manufacturing and service architecture

The first build should use modular, serviceable assemblies:
- tank module with removable access, drain and level reference;
- suction strainer module with isolation on both sides where safe;
- pump module with removable unions, electrical isolation and clear curve/serial identity;
- pressure/flow measurement spool with replaceable instruments;
- local logger/power enclosure physically separated from wet service;
- fixture-equivalent demand/load module;
- removable measured overflow/drain collection.

For every wetted item, record material, pressure/temperature envelope, chemical compatibility, manufacturer, model, revision, serial/lot and replacement interval where supplied. Avoid mixed-material joints or unqualified seals.

Produce:
- dimensioned assembly drawing;
- hydraulic schematic and valve list;
- electrical isolation/control boundary diagram;
- preliminary BOM categories and interfaces;
- assembly and inspection instructions;
- post-maintenance leak/functional test;
- configuration and replacement-part record.

No procurement SKU is approved in this document. Select components only after the duty envelope, material compatibility, safety review, local availability and serviceability comparison are completed.

## 9. Decision gates still open

1. Exact first outlet/use and required service duration.
2. Whether a 250–300 L test vessel is adequate for the selected run duration.
3. Actual tank material/geometry and calibrated usable fraction.
4. Required outlet pressure/flow and static height.
5. Pump curve / motor architecture and acceptable electrical supply.
6. Instrument ranges, accuracy classes, sampling cadence and calibration access.
7. Local electrical and plumbing review requirements.
8. Whether the first recovery stream should be rainwater or separately prepared greywater.
9. Whether treatment materially improves a declared use enough to justify its process, residual and maintenance burden.

## 10. Nonclaims

This is a preliminary design/sizing basis. It is not a construction drawing, component selection release, pump specification, potable-water system, treatment validation, plumbing/electrical code approval, field installation, measured product performance or manufacturing readiness finding.
