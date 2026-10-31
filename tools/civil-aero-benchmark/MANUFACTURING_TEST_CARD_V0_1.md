# AERO-CIVIL-001: non-flight-critical airfoil coupon test card v0.1

Status: proposed protocol; not an approved test procedure. This card describes a small, non-flight-critical research article and its measurement plan. It is not an aircraft drawing, structural design, flight article qualification, or authorization to operate a wind tunnel.

## 1. Objective

Measure whether the geometry and surface of a manufactured airfoil test coupon match its declared digital definition, whether repeated builds reproduce that geometry, and whether a controlled low-speed aerodynamic test can be compared fairly against a validated model in the same Reynolds-number domain.

The experiment is intentionally split into three questions:

1. **Manufacturing conformance:** Did the physical coupon match its released geometry and build specification?
2. **Measurement repeatability:** Can independent measurement and force-balance runs reproduce the recorded result within declared uncertainty?
3. **Model validity:** Does the predicted force/pressure behavior agree with measurements for the tested conditions, within a declared uncertainty and applicability domain?

Passing one question does not imply passing the others.

## 2. Article and boundary

- Article class: non-flight-critical, removable airfoil coupon for ground testing only.
- Geometry: a versioned public airfoil section selected for the chosen tunnel's Reynolds-number and measurement envelope. NACA 0012 may be used as a comparison geometry only when the selected test conditions match the reference data's applicability; do not assume a NASA high-Reynolds-number case validates small-aircraft low-Reynolds-number behavior.
- Chord/span: selected after the facility owner documents test-section dimensions, wall corrections, blockage constraints, mounting interference and target Reynolds-number range. Record dimensions and units in the test manifest. Do not scale geometry by guesswork.
- Build process: record material, supplier/lot, machine/printer, settings, orientation, post-processing, operator, ambient conditions and all rework. Do not change settings between nominal replicate builds unless the change is a declared experimental factor.
- Replicates: at least three independently fabricated copies of the same released digital definition for a first repeatability study. This is a minimum exploratory sample, not enough to characterize all process variation or certify production.
- Non-use: no flight, no installation in a flight-critical structure, and no claim of safe operation based on these tests.

## 3. Pre-build release packet

Freeze the following before fabrication:

- unique experiment, geometry, build and specimen IDs;
- source geometry URI, revision/version, units, and downloaded-file SHA-256 computed from the actual bytes;
- geometry conversion and repair steps, if any, with scripts and their versions;
- material specification and lot/receipt record;
- machine, toolpath/process settings and post-processing recipe;
- declared dimensional tolerances and the reason each tolerance is measurable;
- measurement procedure, calibrated instrument IDs, stated resolution, uncertainty and calibration due dates;
- tunnel facility, test-section dimensions, blockage limit, flow uniformity/turbulence specifications, temperature/pressure sensors, force-balance range and calibration method;
- intended angle/speed points, Reynolds-number range, repeat counts, tare/baseline corrections, wall/support corrections and exclusion criteria;
- analysis script version and the results that would count as a model discrepancy.

No missing numeric input should silently become zero or a nominal value. Mark it unresolved and block the dependent comparison.

## 4. Fabrication traveler

For each specimen:

1. Confirm the released source geometry and checksum before sending it to the fabrication process.
2. Record material lot, machine identity, operator/process version, orientation and all settings.
3. Record actual build start/end, consumed material, failed starts, rework, finishing and hands-on labor.
4. Retain a photograph and non-sensitive process record linking the specimen ID to the as-built part.
5. Do not sand, fill, coat, trim or otherwise change the external surface without recording the intervention and remeasuring the article.
6. Quarantine visibly damaged specimens or parts with unidentified provenance; do not repair them silently into the accepted sample.
7. Assign each article a permanent non-flight test ID that cannot be confused with a flight component.

## 5. Dimensional and surface metrology

Use a documented, repeatable datum scheme. At minimum record:

- chord, span, maximum thickness, maximum-thickness location and trailing-edge thickness;
- upper/lower contour deviations from the released profile at fixed chordwise stations, with the station schedule stored as data;
- twist/straightness along the span;
- total mass and, when relevant to force interpretation, center-of-mass location;
- surface finish using an identified instrument and stated method; report the instrument's measured quantity (for example, Ra) rather than an unqualified adjective such as "smooth";
- photos/scan data, raw readings, operator, instrument identity, calibration state and expanded measurement uncertainty.

Do not set the acceptance band after looking at the results. Declare tolerances before fabrication. A result within the drawing tolerance is not automatically aerodynamically equivalent; preserve measured as-built geometry for the model input and report the difference from nominal.

## 6. Ground test sequence

Only use a facility approved for the test and follow the facility owner's safety procedures.

1. Confirm facility limits, mounting integrity, exclusion zone, emergency stop, sensor calibration and test article identity.
2. Record a tunnel-off zero/tare and a no-article/support baseline where the facility procedure calls for it.
3. Verify freestream velocity, temperature and pressure with the facility's documented calibration. Compute Reynolds number using measured conditions and a declared viscosity source.
4. Test a predeclared, bounded set of flow conditions and angles that remains inside the facility and instrumentation envelope. Randomize or interleave runs where useful to detect drift; record sequence and any excluded run with its reason.
5. Repeat selected points after removing and reinstalling the coupon to expose mounting/setup variability.
6. Store raw time series and calibration records, not only smoothed plots or averaged coefficients. Preserve sensor overloads, invalid readings and aborted runs as explicit events.
7. Do not infer stall or unsteady behavior from a sparse steady force sweep; document the limits of the instrumentation and sampling rate.

The required facility-specific speed/angle envelope and force-balance loading must be reviewed and frozen before the experiment. This card intentionally does not authorize a universal speed or incidence range.

## 7. Required outputs

For every specimen and operating point, publish or retain:

- the immutable experiment/specimen IDs and SHA-256 digests of released input and raw output files;
- as-built dimensional and surface measurements with uncertainty;
- actual Reynolds/Mach conditions, flow speed, angle, density/viscosity source, and facility corrections;
- raw and processed lift/drag/pressure readings where available;
- tare and support correction values, calibration state, repeat count, exclusions and exclusion reasons;
- per-specimen and per-condition mean, dispersion and uncertainty; do not pool distinct Reynolds regimes or build IDs without also showing separate results;
- difference from the analytical/CFD prediction, the model version, and the model's declared validation domain;
- manufacturing material use, first-pass yield, failed builds, scrap, rework, labor, measured part mass and cost inputs with currency/as-of date;
- evidence disposition: synthetic, declared, measured_unqualified, independently_reviewed, or another explicitly versioned state. A producer-asserted "measured" label is not an independent qualification.

## 8. Acceptance logic

Acceptance is based on predeclared, observable conditions, not on whether the result favors the design.

- **Source gate:** source geometry and input artifacts are readable, licensed for the intended use, versioned and byte-hashed. Otherwise block comparison.
- **Metrology gate:** required dimensions, instrument identifiers, calibration and uncertainty are complete. Otherwise block quantitative conformance claims.
- **Build repeatability gate:** compare each replicate with the predeclared tolerances and report the whole distribution. A single passing specimen cannot stand in for production yield.
- **Facility gate:** calibration, zero/tare, operating conditions and applicable corrections are documented; otherwise the affected observations are invalid or explicitly indeterminate.
- **Model comparison gate:** compare only conditions inside the model's declared domain. Show lift and drag separately, uncertainty and worst-condition error. Do not collapse differences into one score that hides an unacceptable quantity.
- **Negative controls:** invalid calibration, missing provenance, sensor saturation, mismatched units, wrong Reynolds regime and corrupted raw-file bytes must block a qualified claim.
- **Evidence gate:** synthetic data, generated data and unqualified measurements can develop the workflow but cannot support real-world performance or manufacturing-release claims.

Numeric tolerances and error thresholds must be derived from the requirement, reference-data quality, facility uncertainty and intended use. Freeze them before looking at the test outcome.

## 9. Research lineage and limits

- NASA TMR NACA 0012 case: https://tmbwg.github.io/turbmodels/naca0012_val.html
- NASA CFD verification/validation overview: https://www.grc.nasa.gov/www/wind/valid/tutorial/overview.html
- NASA TM-4074, Effects of Independent Variation of Mach and Reynolds Numbers on the Low-Speed Aerodynamic Characteristics of the NACA 0012 Airfoil Section: https://ntrs.nasa.gov/citations/19880019495
- SU2 CFD and design-optimization software: https://github.com/su2code/SU2
- NIST industrial CFD verification, validation and uncertainty quantification overview: https://www.nist.gov/publications/summary-industrial-verification-validation-and-uncertainty-quantification-procedures

NASA TMR's commonly used validation case is at Mach 0.15 and Reynolds number 6 million, with a fully turbulent boundary-layer assumption across most of the airfoil. It is useful for a high-Re solver-validation lineage, but is not direct validation for a small electric aircraft operating at a different Reynolds number. Near-stall force comparisons have known three-dimensional experimental limitations. Record and respect these limits rather than averaging unlike tests into a headline score.

## 10. Decisions this experiment can and cannot support

It can support statements about the specific specimens' measured dimensions, surface finish, repeatability, and aerodynamic behavior within the tested facility/condition envelope, if the measurement chain and uncertainty are adequate.

It cannot, by itself, establish flight safety, aircraft stability, structural strength, battery endurance, manufacturing readiness at scale, certification, or superiority to an industry aircraft. Those require separate methods and evidence.
