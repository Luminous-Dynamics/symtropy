# T1 Water Engineering Test-Cell Design Basis v0.1

**Status:** concept design basis for engineering review, not an approved construction package  
**Composes with:** potable train #4958, pump system #4959, pressurized leak diagnostics #4963, Mycelix H1 #3239 and metrology #6034/#6057/#6025/#6029.

## 1. Design objective

Move the research programme from abstract comparison contracts to **repeatable, instrumented, safe component experiments**.

Use one shared evidence/acquisition approach, but three isolated hydraulic configurations:

1. **PUMP-T1:** clean-water pump and complete-system performance.
2. **FILTER-T1:** low-hazard treatment-media and membrane hydraulics / water-recovery performance.
3. **LEAK-T1:** controlled pressurized pipe loop with seeded, captured leaks.

They may share measurement standards, data acquisition and non-contact instrumentation. They must not share contaminated water, media, challenge materials or fluid circuits without a reviewed decontamination and compatibility procedure.

## 2. Relationship to Mycelix H1

Mycelix #3239 defines a useful reference topology:

`opaque reservoir -> circulation pump -> flow measurement -> instrument/test manifold -> restriction/bypass -> return`

It also requires explicit pump-command versus measured-flow separation, reservoir level/leak sensing and source/instrument provenance.

**H1 is not an ANSI/HI 14.6 pump acceptance rig, a potable-water treatment qualification plant, a pathogen challenge laboratory or a municipal pipe certification setup.** Reuse it as an instrumentation and recirculation reference where appropriate; do not transfer its small-pump ratings or H1 channel assumptions to other duty families.

## 3. Common instrumentation and data contract

Every test observation should resolve through existing Mycelix metrology owners, not a new Luminous generic sensor schema:

- instrument / model / channel / measurand / unit / range;
- installation and location identity;
- calibration or verification record, method, date and validity;
- sample-specific operating-condition applicability;
- synchronized timestamp, timebase, boot/session identity and data-gap status;
- uncertainty, resolution, drift and correction method where known;
- exact fixture, hardware, firmware, pump/media and configuration revision;
- operator, run, procedure, sample and chain-of-custody identity as applicable.

Use independent reference instrumentation for acceptance measurements. A controller command is not an observation. Two sensors sharing a reference transducer or calibration process are not automatically independent corroboration.

Sample rates are selected from the fastest physical phenomenon and instrument response. A 1 Hz stream may be adequate for slow energy/throughput trends but is not a water-hammer or acoustic-transient measurement. The required analog bandwidth, anti-alias filtering, sample rate, clock synchronization and uncertainty must be stated for each test.

## 4. PUMP-T1 — pump and system hydraulic bench

### Functional topology

`R-101 source tank -> XV-101 suction isolation -> STR-101 removable strainer -> P-101 pump + motor/VFD -> CV-101 discharge check -> PT-102/FT-101 measurement spool -> PCV-101 adjustable system resistance -> R-101 return`

Additional branches:
- upstream suction pressure / temperature tap;
- downstream pressure tap and independent mechanical pressure relief returning safely to tank;
- controlled bypass around the pump or test section with measured bypass flow;
- drain, vent and isolation points;
- guarded coupling/rotating equipment and a local manual stop;
- reservoir level and leak/spill sensing.

### Measurement boundary

Measure at minimum:
- reference volumetric flow;
- suction and discharge pressure, with total dynamic head calculation recorded;
- water temperature and density basis;
- electrical real power and cumulative energy at the declared boundary;
- pump speed; shaft torque if available;
- vibration and casing/bearing temperature where appropriate;
- storage level, bypass flow and leakage.

### Test sequence

1. Dry inspection, electrical protection and sensor check.
2. Static leak/pressure test under an approved procedure.
3. Prime and verify low-flow operation with relief/bypass available.
4. Record repeatable Q-H-P curves at named speeds and impeller configurations.
5. Compare measured curve to exact vendor/acceptance curve revision.
6. Sweep system resistance and map the pump/system intersection.
7. Run variable-demand traces: throttling, VFD and staged-parallel configurations.
8. Inject permitted faults: sensor disconnect/stuck value, low reservoir, strainer restriction, power interruption and standby-pump failure.
9. Complete shutdown, drain, inspection and post-run reference check.

### Acceptance evidence

Report measured pump curve and uncertainty, system curve, duty-point error, wire-to-water efficiency, power boundary, NPSH method/margin, preferred operating-region exposure, vibration/temperature, starts/stops, alarms, downtime and repeatability.

The detailed pump test procedure and tolerance grade must be frozen against the exact ANSI/HI 14.6-2022 application and procurement contract. This concept does not prescribe test pressure, casing rating, impeller trim, motor size or relief setting.

## 5. FILTER-T1 — household / community treatment component bench

### Functional topology

`T-201 homogenized non-potable feed tank -> feed pump / controlled supply -> common inlet manifold -> [A: commercial/reference train] and [B: candidate train] -> separate product collection tanks`

Each line should have:
- inlet / outlet pressure taps;
- reference flow / totalized-volume measurement;
- differential-pressure measurement across each stage;
- temperature and appropriate raw-water process indicators;
- sample ports before and after each stage and at final outlet;
- separately captured reject, concentrate, backwash, regeneration and residual streams;
- isolation/bypass valves with position and state evidence;
- secondary containment and spill/leak detection.

### T1 restrictions

T1 is a **hydraulic and non-hazardous media-characterization bench**. It may test clean-water flow, pressure loss, cartridge fit, media loading with safe nonhazardous surrogate particulates/tracers, water recovery, clogging and backwash repeatability.

It must not use live pathogens, toxic chemicals, unknown raw-water contaminants or make a potability claim. Microbial and hazardous chemical challenge belongs in a qualified laboratory at T2 under validated methods and approved handling/disposal.

### Test sequence

1. Verify cartridge/media identity, revision, orientation and seals.
2. Establish clean-water baseline flow and pressure-drop curve.
3. Run a declared non-hazardous loading profile with measured feed and outlet.
4. Track pressure loss, flow, captured solids/tracer mass, backwash/reject and water-volume closure.
5. Apply repeated stop/stagnation/restart and approved clean/backwash cycles.
6. Verify bypass detection and fail-closed behaviour on missing/invalid critical measurements.
7. Record end-of-life / replacement decision only for the non-hazardous surrogate capacity tested.

### Acceptance evidence

Report flow and pressure drop versus cumulative throughput, captured mass where valid, water recovery, residual streams, cleaning recovery, replacement time, energy and consumables. Do not translate surrogate capacity into pathogen or chemical removal performance.

## 6. LEAK-T1 — controlled pressurized pipe diagnostics loop

### Functional topology

`R-301 tank -> P-301 pressure source -> isolation/check -> segmented pipe loop with branch / joints / interchangeable sections -> return manifold -> R-301`

Provide a controlled leak fixture for:
- orifice opening;
- gasket/joint leakage;
- longitudinal/circumferential seeded defect surrogate where mechanically appropriate;
- no-leak positive control.

All discharge is captured in a sealed collection/secondary-containment arrangement and returned or disposed of under the test procedure. No connection to public or potable infrastructure is permitted.

### Sensors

- calibrated flow at source and return / branch where practical;
- pressure transducers at multiple declared nodes;
- acoustic accelerometers/hydrophones on qualified mounting points;
- temperature and pump/valve event state;
- synchronized acquisition and a surveyed pipe/feature coordinate reference;
- optional acoustic correlator and sensor-position variants.

### Test sequence

1. Characterize the no-leak loop across flow, pressure and background noise.
2. Randomize seeded leak type/position/size within an approved mechanical envelope.
3. Run matched no-leak confounders: demand change, pump switching, valve transition, sensor drift, common-mode calibration and communication gaps.
4. Blind model/operator evaluation to seeded leak location.
5. Independently reveal ground truth after scoring and calculate event detection, false alarms, localization error and uncertainty.
6. Repeat across pipe material/diameter/joint, sensor positions, water/air state where allowed, and network topology.
7. Inspect the loop for wear/damage and confirm all seeded outlets remained captured.

This is a lab-scale dataset; it cannot claim municipal-network generalization.

## 7. Safety and approval hold points

Before procurement, construction or energization, a competent engineer and facility safety owner must review:
- maximum allowable working pressure, component ratings, fatigue and relief design;
- hydraulic reaction forces, pipe restraints, hose whip, water hammer and trapped pressure;
- motor/coupling guards, rotating machinery, electrical isolation, earthing, wet-location enclosures and applicable RCD/GFCI-equivalent protection;
- leak containment, slip hazards, water/electrical segregation, lifting and access;
- chemical compatibility, media dust, reject/backwash disposal and any laboratory biosafety requirements;
- emergency stop, lockout/tagout, manual isolation, depressurization and safe restart.

Use low voltage on wet-side instrumentation where feasible. Do not assume a software interlock or digital twin is a physical safety system. This design basis is not a construction drawing, pressure-system design, electrical design or risk assessment.

## 8. Shared T1 exit gates

- Every measurement binds to the exact instrument, method, calibration/condition and configuration revision.
- Independent reference checks and uncertainty budgets exist for the key accepted quantities.
- Commanded state and measured physical state remain separate.
- Water, marker/solids, energy and residual balances close within predeclared, uncertainty-supported tolerances where those quantities are measured.
- Fault injection is repeatable and the expected state is independently adjudicated.
- Missing, stale, conflicting, out-of-range or unobservable critical input yields Indeterminate/Unsupported/no-release.
- No surrogate or synthetic outcome is promoted to contaminant safety, field reliability, regulatory approval or SOTA superiority.
- No automated physical actuation is enabled by the benchmark or by Symthaea.

## 9. Proposed equipment classes — no canonical vendor

- pressure-rated reservoirs/tanks and compatible wetted piping;
- centrifugal/submersible pump test candidate selected against the declared duty;
- VFD and protected power measurement;
- independently calibrated reference flow and pressure instrumentation;
- differential-pressure, temperature, vibration and level sensing as appropriate;
- reference and candidate filter housings/media;
- controlled bypass/restriction/manifold valves;
- isolated data acquisition and synchronized logger;
- containment, mechanical guards, relief components, drains, vents and manual isolation;
- calibration/check equipment and safe tracer/particulate test media.

Specific sizes, pressure classes, material compatibility, sensor ranges, sampling rates, power architecture and procurement choices remain unresolved until the target duty, source-water profile and competent design review are frozen.

## 10. Next design decision

The next engineering artifact should be a parameterized pump-loop P&ID / instrument list with one declared low-risk clean-water duty profile. In parallel, choose a single household filter comparator and a non-hazardous surrogate loading test. Do not procure a broad universal rig or start with biological challenge work.
