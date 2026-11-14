# Low-Voltage DC Fan Reference Article v0.1

**Status:** requirements/architecture proposal; no CAD freeze and no physical article built  
**Related:** [APPLIANCE-QUAL-001 #1805](https://github.com/Luminous-Dynamics/symtropy/issues/1805), [household appliance substrate PR #1806](https://github.com/Luminous-Dynamics/symtropy/pull/1806)  
**First article:** an indoor tabletop personal air-circulation fan with a replaceable fan cartridge and a separate low-voltage external power supply

## 1. Why this article first

A small fan lets us validate repair-task graphs, replaceable-module boundaries, measurement repeatability, independent assembly, and low-voltage advisory diagnostics before introducing water, heating elements, refrigeration circuits, pressure or mains switching into a custom design.

It is a bounded engineering demonstrator, **not** a safety-exempt product. IEC 60335-2-80:2024 covers household and similar electric fans and expressly includes DC-supplied and battery-operated types; applicable IEC 60335-1 provisions and national adoption must be reviewed by qualified product-safety specialists before a powered physical prototype is released beyond a controlled lab or offered for sale. Source: https://webstore.iec.ch/en/publication/70375.

## 2. Intended use and exclusions

### Intended
- Indoor tabletop personal airflow.
- Physical speed control and local on/off.
- Replaceable fan cartridge, controller, wiring harness, grille, base/tilt parts and feet.
- Operation from an appropriate external, certified low-voltage DC supply.
- Documentation and diagnosis usable offline.
- Manual operation with no account, internet, app or Symthaea dependency.

### Explicitly out of scope for v0.1
- Heater, humidifier, ionizer, UV, air-quality or medical claims.
- Batteries/charging, mains-connected electronics inside the custom housing, wireless control or remote actuation.
- AI-generated control decisions or remote firmware control.
- Any energised test involving a removed guard, accessible impeller, exposed conductors, or intentionally jammed moving parts.
- A claim of IEC/UL/CE/UKCA/other compliance; such conformity is not established by this requirements document.

## 3. First architecture hypothesis

Start with a **documented, replaceable 140 mm low-voltage fan cartridge** (e.g. a commercially available DC/PWM fan module) mounted in a custom guard/base assembly. The first iteration may replace the cartridge as a unit rather than opening the motor itself. That is intentional: internal motor teardown is only justified if failure analysis shows that the added interface, tools and test burden produce a measured net benefit.

Candidate modules:

1. **External DC supply** — separate, replaceable, appropriately rated and certified for the target market; never custom-built for the first article.
2. **Power-entry harness** — keyed connector, published polarity/ratings, strain relief, replaceable wire assembly and documented isolation step.
3. **Controller/user interface** — separate board/module where feasible; physical on/off and speed control; fail-safe default on loss of control signal.
4. **Fan cartridge** — motor + impeller assembly, with exact make/model/revision and pinout captured; replace as a whole until evidence justifies deeper repair.
5. **Guard/grille** — reversible fasteners, cleaning access and documented inspection criteria; guard geometry must be checked against the applicable safety requirements and test probes.
6. **Base/tilt assembly** — replaceable feet, pivot/fasteners, non-tip stability and cable routing.
7. **Fasteners/seals/labels** — commonly available where practical, with exact sizes, materials and replacements documented.

No connector pinout may be inferred from wire colour or a product-family name. Bind pin assignment, voltage/current limits, control-signal behavior and allowed substitutions to the exact cartridge part/revision and verify them before power is applied.

## 4. Interface requirements

- **Power boundary:** external DC supply to keyed power entry. Record nominal/allowed voltage, maximum current, polarity, connector, cable, protection and shutdown behavior before selecting components.
- **Module boundary:** fan cartridge and controller have separately documented electrical and mechanical interfaces. Unknown compatibility is not treated as compatible.
- **Service access:** common service tasks must not require destructive removal of bonded cosmetic parts. Avoid adhesives in routine service paths unless they are essential to the function and the removal method is documented.
- **Safe isolation:** service instructions specify disconnect, verify de-energized state, allow relevant stored energy to dissipate, and prevent accidental reconnection.
- **No hot-plug assumption:** a replaceable module is changed only when power is disconnected unless a specific connector/system has been assessed and qualified for hot-plugging.
- **Control authority:** physical controls and deterministic protections are independent of optional diagnostics. No model or network process can set a safety limit or override a protection.
- **Open source package:** publish editable mechanical source, drawings/tolerances, wiring/schematic sources, firmware where used, BOM, pinout, build steps, repair procedures, instrument/setup details, test fixtures and a component/licence manifest. The licence for each original and third-party artifact must be explicit.

## 5. Architecture alternatives to compare

Do not choose by aesthetic preference; capture the trade-offs against the fixed requirements and baseline.

| Option | Advantages | Costs/risks to test |
|---|---|---|
| Off-the-shelf 140 mm DC/PWM fan cartridge + separate controller/guard | Fastest prototype path; established replacement format; isolates the module boundary | Connector/pinout variation, cartridge cost, fan's intended-use rating, low-speed behavior, noise, vibration and guard integration |
| Bare BLDC motor + custom impeller + custom driver | More control over efficiency, acoustics, motor and impeller service | Much greater electrical/mechanical design, balancing, containment, thermal and qualification burden |
| Fully integrated retail fan used as a comparator/control | Cheap baseline; known off-the-shelf product | Not a candidate open design; disassembly, component access and documentation may be poor |

**Initial recommendation:** evaluate the cartridge architecture first. Do not commit to a custom motor/driver until common-condition baseline data and teardown/repair observations show that the cartridge option cannot meet useful-function targets or causes a meaningful lifecycle penalty.

## 6. Requirements we can freeze now

- Essential function operates offline with no account/app.
- The custom controller and harness can be serviced independently of the fan cartridge where architecture permits.
- Every replaceable module has a stable identifier, revision and published interface.
- Every BOM part is specified or explicitly `unknown`; candidate substitutes are not called qualified before testing.
- Routine service tasks have reversible procedures and defined tools/isolation/post-checks.
- Original failures, rejected parts, unsuccessful repair attempts and residual defects remain in the evidence history.
- The safety case and required conformity route are determined before field use or sale.

## 7. Performance criteria to freeze after control measurements

Do not choose a target wattage, airflow, decibel figure or cost from intuition. First purchase three representative retail units in the chosen size/use class, capture exact model/revision/settings, and measure them using one published protocol and calibrated instruments.

Candidate comparative acceptance hypothesis, to be frozen before testing the prototype:
- At a predefined useful airflow operating point, candidate airflow is no worse than the median control.
- At that matched airflow, candidate input power and sound pressure do not exceed the corresponding preregistered baseline values, or any unavoidable Pareto trade-off is explicitly reported rather than hidden.
- Unit-to-unit spread and measurement uncertainty are reported; the performance test is repeated with a fixed setup and independent reviewer.
- No superiority claim is accepted if airflow, speed setting, distance, acoustic environment, instrument calibration, or exclusions differ materially between candidate and control.

Recommended record set: input voltage/current/power, RPM if measurable, airflow using a declared method, A-weighted sound pressure with distance/environment declared, vibration where instrumentation permits, enclosure/motor temperature where relevant, measurement uncertainty and raw observations. Test points and repetition count must be preregistered.

## 8. Repair tasks and seeded fault corpus

Initial high-value service tasks:
1. Replace the external supply/cable and check polarity/connector integrity.
2. Replace the power-entry harness.
3. Replace the controller module.
4. Replace the fan cartridge.
5. Remove, inspect, clean and reinstall the grille.
6. Replace feet or tilt mechanism.
7. Perform post-repair functional verification.

Synthetic contract cases may represent open circuit, incorrect speed signal, stalled/low RPM, reduced airflow, increased vibration, sensor failure, stale observations, unit mismatch, conflicting readings and recurrence after apparent restoration. The case oracle must distinguish fault *symptom* from observed root cause; multiple faults may produce similar traces.

Physical fault introduction is a separate, hazard-reviewed lab activity. Do not run a fan with guards removed, expose conductors, or intentionally obstruct a powered rotor as a casual test.

Post-repair verification must bind the exact unit, part/revision, procedure, test profile, instrument identity/calibration, measured result and unresolved limitations. A nominal command is not proof of airflow. One successful reading is not proof of recurrence-free operation.

## 9. Gates before first powered lab run

- Product-safety specialist reviews intended-use profile, guard, access, tip stability, cable strain, connector polarisation, current/thermal protection, cleaning/service instructions, and the applicable standard/test programme.
- The lab has an approved written procedure and equipment suited to the hazard; powered tests use guards and controlled access.
- Supply and all custom electronics have traceable ratings; unknown limits block energization.
- Measurement instruments and methods have defined calibration/status and recording formats.
- Stop conditions are explicit for abnormal heat, smell, vibration, current, sound, mechanical interference or damaged guard.
- Service tasks include power isolation and post-repair functional verification.
- Test reports distinguish simulation, bench measurement, independent replication, and safety/conformity assessment.

## 10. Open decisions (must remain visible)

- Exact control units/models and purchase dates.
- DC supply voltage/current and connector.
- Fan cartridge model/pinout/current and speed-control interface.
- Housing material, manufacturing process, guard geometry and cleaning behavior.
- Quantified user needs, airflow target, acoustic environment and use profile.
- Applicable safety standard editions, national adoptions, test-lab scope and eventual conformity route.
- Actual BOM prices, part lead times, second-source compatibility, repairability outcomes and material/lifecycle data.
- Component and documentation licence matrix.

These are not settled by the existence of this brief. Record evidence and decisions here as the project gains hardware data.

## 11. Claim ceiling

This document defines a plausible first article and evidence gates. It does not establish that components have been selected, CAD exists, the appliance is built, the proposed interfaces work, performance is competitive, service life is long, repairs are successful, or any safety/legal conformity has been achieved.
