# WPT Geometry Compatibility Protocol v0.1 (research draft)

**Protocol ID:** WPT-GE-01  
**Schema:** `wpt-geometry-compatibility-v1`  
**Status:** research draft; no new physical geometry sweep is represented here  
**Tracking:** [WPT-GEOMETRY-INTEROPERABILITY-001 #3503](https://github.com/Luminous-Dynamics/symtropy/issues/3503)

## 1. Purpose: qualify the operating region, not one alignment point

A VA/GA pair is not interoperable merely because one aligned test transfers power. The useful engineering object is its *qualified operating envelope*: the set of physical geometries, loads, temperatures and material conditions under which the full declared requirements are met.

Keep these envelopes distinct:
1. **Magnetic-coupling envelope:** mutual inductance, coupling coefficient, inductance matrix, Q or a declared coupling threshold.
2. **Power-transfer envelope:** regulated power and output stability across the geometry.
3. **Efficiency envelope:** efficiency at explicit source/output meter planes, load and thermal state.
4. **Thermal envelope:** component temperature and hot-state saturation margin.
5. **Safety/EMF envelope:** exposure and EMC requirements across the same geometry.
6. **Standards/conformance envelope:** only those clauses within the published edition's actual scope.

A magnetic-coupling envelope is not automatically an efficiency- or safety-qualified envelope.

## 2. Geometry vocabulary and coordinate frame

Every record must use an explicit vehicle/ground coordinate frame and separately define:
- X: lateral vehicle offset;
- Y: longitudinal vehicle offset;
- Z: vertical relative coil offset;
- yaw: relative vehicle/coil rotation;
- **coil-to-coil air gap:** vertical separation between the active magnetic coupling surfaces/packages;
- **vehicle ground clearance:** distance from roadway/floor reference to the vehicle-side coil package;
- installation style: surface-mounted, flush-mounted, embedded or bench.

Do not substitute vehicle ground clearance for the magnetic air gap. A flush-mounted ground assembly changes the physical geometry and field boundary; its coupling penalty must be measured rather than assumed.

Record coil geometry, winding, ferrite, shield, encapsulation, material lot and mounting generation for both GA and VA. If a value is unknown, keep it null and qualify the record as incomplete/source-claim-only.

## 3. Sampling design

A compatibility matrix must identify every GA/VA pairing and its sampled positions. The 2026 WPT4 interoperability study reports 189 discrete positions, 3,402 evaluation points and 13,608 inductance samples over four VAs and three GAs. This is a useful published sampling baseline, not proof that every future geometry is fully covered.

For each pair:
- pre-register the coordinate bounds, grid strategy and sampling density;
- capture every sampled coordinate and the instrument configuration;
- measure primary and secondary open-circuit and short-circuit inductances where supported;
- derive mutual inductance and coupling coefficient only through a versioned, auditable calculation;
- record coil Q, resonant detuning, output regulation and delivered power where available;
- refine sampling near the pass/fail boundary and around disconnected islands;
- preserve the raw point data and a SHA-256 digest.

A regular grid alone can miss narrow pass regions. Adaptive boundary sampling is allowed only when the initial grid, refinement rule, stopping criterion and all added points are preserved. Do not silently replace disconnected qualified regions with one convex hull.

## 4. Constructing the qualified operating envelope

Let the declared geometry domain be (D). For a specific VA/GA pair, define a point as qualified only when all required conditions pass simultaneously:

- magnetic coupling above the declared minimum or sufficient to support demanded power;
- output regulation within the declared tolerance;
- efficiency above the declared threshold at a named meter boundary;
- coil, ferrite, capacitor and power-stage currents/voltages within ratings;
- core flux below the temperature-specific Bsat(T) limit;
- thermal, exposure and EMC constraints pass;
- receiver identity, alignment and control-state assumptions are valid.

Then preserve:
- sampled domain and unsampled domain;
- count and fraction of qualified points;
- number of connected components;
- boundary uncertainty and lower confidence bound;
- the metric used to define a pass region;
- whether the envelope is measured, calculated from measured data or simulated.

Do not report an "operating range" without its threshold definition. A scalar X/Y tolerance is insufficient when Z, yaw, material, load and temperature alter the field.

## 5. Cross-generation compatibility matrix

At minimum, evaluate:
- legacy WPT3 vehicle assemblies with existing surface-mounted ground assemblies;
- legacy WPT3 vehicle assemblies with flush-mounted ground assemblies;
- proposed WPT4 vehicle assemblies with flush-mounted ground assemblies;
- mixed-generation pairings and power-class transitions.

Use the same instrument/coordinate convention and declare whether the target is:
- magnetic-coupling range;
- output-power range;
- efficiency-qualified range;
- thermal/safety-qualified range.

The comparison should report both absolute qualified operating regions and relative change against a stable, named baseline. Do not compare only each configuration's best point.

## 6. Evidence and qualification levels

- `NOT_RUN`: template only.
- `SOURCE_CLAIM_ONLY`: published claim without point-level geometry/raw data or required metadata.
- `INCOMPLETE`: a measurement exists but the geometry domain, calibration, uncertainty or raw artifact is missing.
- `QUALIFIED`: complete geometry domain, coordinate frame, dense measured sample set, calibration/evidence artifacts, raw-data digest, declared pass metric and independently reviewed analysis are present.
- `REJECTED`: integrity, calibration or protocol failure invalidates the record.

The current Koch et al. record is deliberately `SOURCE_CLAIM_ONLY`. It captures the published >=34% magnetic-coupling operating-range degradation claim but does not reconstruct its raw envelope.

## 7. Research targets (not achieved or standards requirements)

- At least 189 distinct positions per GA/VA pair as an initial comparison floor; increase density adaptively around boundaries.
- At least three VAs and two GAs for any broad interoperability claim.
- Target no more than 5% relative degradation of a named legacy VA's *qualified operating region* across a ground-assembly generation change, unless the product explicitly accepts that trade.
- Report a 95% lower confidence bound on qualified operating-region coverage.
- Repeated same-build/same-lab coverage fraction coefficient of variation <=2%; inter-lab envelope coverage difference <=5 percentage points after coordinate and fixture harmonization.
- Preserve air-gap and vehicle-ground-clearance dimensions independently in every record.
- No false standards claim: the 22 kW WPT4 concept is research interoperability evidence, not proof of compliance with SAE J2954_202408. The current J2954 edition's published light-duty scope remains limited to its issued classes; future WPT4 scope must be re-verified against an issued revision.

These values are proposed research objectives, not normative acceptance limits.

## 8. Data package and review receipt

A qualified compatibility envelope must reference:
- geometry/coordinate metadata and fixture calibration;
- raw open/short inductance or equivalent coil-parameter samples;
- computed mutual inductance/coupling and pass/fail labels;
- instrument calibration and uncertainty budget;
- analysis code, exact version and configuration digest;
- SHA-256 digests for all artifacts;
- excluded points, invalid points and protocol deviations;
- reviewer disposition and the precise claim ceiling.

## 9. Safety / implementation boundary

The current artifacts define source ingestion, schema, evaluator and protocol only. They do not authorize new coil construction, energized 22 kW testing, physical exposure testing, roadway installation or certification. Future physical work requires appropriately rated laboratory facilities, approved risk assessments and independent safety review.