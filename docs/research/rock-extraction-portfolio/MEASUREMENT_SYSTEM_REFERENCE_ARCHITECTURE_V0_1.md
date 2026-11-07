# Excavation measurement-system reference architecture v0.1

**Status:** reference architecture and qualification-record contract. It is not a vendor BOM, instrument certification, laboratory accreditation or safe operating design. Final ratings, materials, installation, guarding, electrical protection, dust controls and pressure-system limits require the responsible rig designer and competent safety/engineering authority.

## 1. Design principle: reference truth plus online observability

Build the pilot around two distinct measurement layers:

1. **Independent physical references** establish mass, sample properties and instrument checks.
2. **Online estimators and diagnostics** provide continuous observability between physical reference samples.

A high-rate online sensor is not automatically a reference. A model cannot validate itself. All claims must bind to raw artifacts, calibration, material cohort, physical station, installed path, common timebase and an explicit measurement equation.

The primary system outputs are:
- dry-solid mass collected / recovered;
- dry-solid mass-flow interval at a declared physical boundary;
- mass-flow by exact parent PSD bin;
- sample-based size and morphology distributions;
- synchronized pressure, airflow, electrical and condition signals;
- explicit residual/deposit/reject/fines branches;
- uncertainty and source lineage.

## 2. Physical measurement topology

Instrument and sample every physically distinct branch where practicable:

`Cutter discharge → Capture inlet → Conveyance inlet → Conveyance outlet → Separator → Accepted discharge`

Add branch boundaries:
- floor residual;
- line deposits / cleanout;
- separator coarse product;
- filter/cyclone fines;
- rejects;
- recovered material;
- other declared losses.

A transition must not be evaluated as closed if a physically relevant branch is unmeasured. Missing flow is UNKNOWN, not zero.

### Recommended reference boundaries

| Boundary | Primary reference | Online / diagnostic channels |
|---|---|---|
| Generated / cutter discharge | timed gravimetric collection or independently weighed feed/depletion with inventory model | cutter force/torque/vibration/current; PSD sampling |
| Capture inlet / floor | separately weighed captured and residual streams; registered spatial survey | pressure/airflow; support pose/slip diagnostics |
| Conveyance inlet/outlet | timed catch-and-weigh / calibrated weighing hopper as applicable | differential pressure, airflow, current/power, temperature, vibration; candidate solids-flow estimator |
| Bend / line deposits | recovered and weighed cleanout; independently measured coupon/pipe-wall condition | pressure taps, vibration/acoustic diagnostics |
| Separator coarse / filter fines / reject | separate weighed collection by branch | filter differential pressure; pressure/airflow and electrical diagnostics |
| Accepted discharge | independent calibrated mass scale / weigh hopper and timed collection | online solids-flow estimator compared to gravimetric reference |
| Particle characterization | representative increments and independently weighed split/sample | sieve analysis, dynamic imaging, optional qualified 3D point-cloud method |

For transient tests, feed and discharge measurements must be aligned using a measured residence-time/holdup model. Do not subtract simultaneous inlet/outlet rates as if the line has no inventory or time delay.

Dry-solid flow must declare moisture correction. The moisture method, sampling point and conversion model require their own provenance and uncertainty.

## 3. Candidate online flow estimator: electrostatic-coupled capacitance

Li et al. (2025), *Sensors* 25(22), 6850 (DOI 10.3390/s25226850), provide a promising secondary sensor candidate.

Their method:
- uses upstream/downstream electrostatic signals and cross-correlation to estimate average particle velocity;
- uses a calibrated capacitance response to estimate particle concentration;
- computes flow as ( dot m = A v eta ), where A is pipe cross-sectional area, v is particle velocity and β is particle mass concentration.

Their test evidence:
- 50 mm laboratory pipe, polypropylene pellets with 3.85 mm mean diameter and density 0.90 g/cm³;
- calibration data from one blower operating regime and validation on other regimes;
- maximum relative error about 5.5% for mass-flow estimation at 2.04–6.43 kg/m³ particle concentration;
- 100 mm pilot conveying rig with polypropylene pellets at 3–10 t/h;
- maximum relative concentration error 3.6% over 30.98–68.87 kg/m³;
- 20 kHz signal acquisition; reported correlation distances 120 mm for electrostatic and 60 mm for capacitance channels.

Source: https://pmc.ncbi.nlm.nih.gov/articles/PMC12656069/

**Hard-rock transfer risks** include particle morphology, angularity, size distribution, abrasion, moisture, dust, electrostatic charging, wall deposition, phase regime, pipe geometry and material dielectric/electrical properties. The method's concentration calibration cannot be assumed to transfer from polypropylene to rock. Its 5.5% mass-flow error and 3.6% concentration error are distinct quantities and must never be combined as one accuracy claim.

### Candidate sensor test protocol

1. Install as a **secondary estimator**, never as the mass-balance authority.
2. Compare its velocity estimate to an independently characterized velocity reference where feasible.
3. Compare its concentration and mass-flow outputs against timed, independently weighed collections, aligned for residence time and line inventory.
4. Randomize test order and hold out complete material cohorts, path configurations and campaigns.
5. Recalibrate/test across the declared rock PSD/morphology/moisture/density domain.
6. Measure drift, fouling, wear, charging effects, response time, data loss, hysteresis and sensor-health behaviour.
7. Publish bias, repeatability, maximum absolute relative error, interval coverage and worst-domain subgroup separately.

Do not promote it to reference status because it correlates with pressure/current or because its model predicts mass flow consistently.

## 4. Particle sampling and characterization

### Sampling

ISO 14488:2007 plus Amendment 1:2019 remains the current published sampling and sample-splitting standard, last confirmed in 2023. ISO/DIS 14488 entered the DIS enquiry phase on 7 August 2026 and is under development as a replacement. Use the current published edition and monitor the draft; version the actual sampling method/protocol used.

- ISO 14488 current published edition: https://www.iso.org/standard/39988.html
- ISO/DIS 14488 draft: https://www.iso.org/standard/89269.html

Sampling design must describe the defined lot/stream, increment strategy, sample mass, splitting method, bias controls, sample custody, exclusions and representativeness evidence. A precise measurement on a biased sample remains a biased estimate of the population.

### Size and shape

- ISO 13322-2:2021 for dynamic image analysis; qualify pixel scale, field-of-view uniformity, reference material, motion blur, depth of field and orientation effects: https://www.iso.org/standard/72566.html
- ISO 9276-6:2008 for representation of particle shape/morphology distributions; its current status was confirmed in 2022: https://www.iso.org/standard/39389.html

Keep number-, area-, volume- and mass-weighted distributions distinct. Never convert number-count morphology into mass fractions without an explicit, validated weighting/sampling model. Use 2D descriptors only as 2D descriptors; 3D descriptors need 3D acquisition and reconstruction qualification.

## 5. Time synchronization

A common DAQ timebase must connect:
- mass-flow and weighing collection windows;
- PSD and morphology samples;
- pressure/airflow/current/power/temperature/vibration streams;
- cutter and machine telemetry;
- line residence-time estimates;
- spatial floor surveys and support-footprint observations.

Record raw source clock, common clock reference, clock-sync protocol, measured channel latency, timing uncertainty and drift. Measure latency; do not infer it from nominal sample rate. If clocks cannot be aligned within the predeclared decision-relevant bound, the comparison is UNKNOWN / NOT ALIGNED.

## 6. Measurement model and uncertainty budget

The calibration/evaluation method must record:
- measurement equation and exact model/software digest;
- calibration certificate, method, lab, scope, traceability chain, range and validity;
- as-found and as-left status;
- material/path/geometry/environment calibration domain;
- repeatability (Type A), calibration (Type B), resolution, drift, environment, sampling, timing, model discrepancy and other components;
- standard uncertainties and units;
- sensitivity coefficients for a linearized model;
- component dependence/correlation assumptions and covariance evidence;
- propagation method and versioned result artifact.

### Standards update

The BIPM/JCGM publication list currently includes:
- JCGM 100:2008 Amendment 1:2026 — nonlinearity in measurement models;
- JCGM GUM-5:2026 examples;
- JCGM 101:2008 — Monte Carlo propagation of distributions;
- JCGM 102:2011 — extension to multiple output quantities;
- JCGM 106:2012 — role of measurement uncertainty in conformity assessment.

Source: https://www.bipm.org/en/web/guest/publications/guides

For a nonlinear derived flow estimator such as (A v eta) with nonlinear (eta=f(Delta V)), do not use a linearized-only budget without a justified linearity study. Monte Carlo propagation may be needed, with correlated inputs preserved. A declared independence assumption must be explicit and reviewed; no silent zero-correlation default.

NIST AMS 100-85 (5 October 2026) recommends risk-scaled credibility assessment connecting computational model influence and decision consequence to metrology and VVUQ effort: https://doi.org/10.6028/NIST.AMS.100-85

ISO/IEC 17025:2017 laboratory competence is relevant, but only the actual accredited laboratory scope/method can be relied upon. A certificate from a laboratory does not establish that every test method or material matrix is within its scope: https://www.iso.org/standard/66912.html

The v0.1 Rust validator admits the uncertainty evidence and rejects missing provenance/nonlinearity/covariance records. It intentionally does **not** compute combined standard uncertainty. A separate calculation implementation needs its own verified measurement equation, covariance handling and GUM conformance tests.

## 7. Calibration and validation partitions

Keep these distinct:

1. **Calibration set:** fit channel calibration, sensor coefficients and model parameters.
2. **Validation set:** independent campaigns/material/path conditions for model selection and bias checks.
3. **Prospective holdout:** untouched material cohorts and installed configurations; freeze code, thresholds and model digest before evaluation.

Repeated samples from the same batch, specimen, run or campaign do not automatically count as independent. A secondary publication of an existing trial is not another independent experiment.

Reference-comparison records must declare whether they were used in training/calibration. Validation records must be independent of model training. Prospective holdout evidence must be independently adjudicated and retained.

## 8. Functional BOM classes (not procurement specifications)

| Class | Function | Procurement/engineering fields to qualify |
|---|---|---|
| Reference gravimetry | weigh hoppers, scales, collection vessels, load cells | capacity/range, resolution, linearity, creep, hysteresis, calibration chain, environment |
| Pressure | absolute/differential transducers, static pressure taps | range, overload, accuracy, drift, impulse-line fouling, material compatibility |
| Gas flow | calibrated airflow measurement | rangeability, pressure/temperature compensation, installation effects, calibration |
| Electrical | current/voltage/real-power transducers or power analyzer | bandwidth, isolation, phase accuracy, calibration, harmonic behaviour |
| Solids online estimator | electrostatic/capacitance sensor candidate | material domain, pipe size, wear, charge, moisture, concentration range, transfer error |
| Particle sampling | sample cutter / splitter / sieve equipment | representative sampling, size range, recovery, bias, cleaning |
| Imaging | dynamic imager / camera / optics / lighting | pixel scale, depth of field, motion blur, orientation, dust/window fouling |
| 3D characterization | LiDAR / structured-light / photogrammetry | point density, occlusion, registration, reference geometry, reconstruction uncertainty |
| Timing / DAQ | synchronized acquisition and clock | channel sample rate, simultaneity, latency, jitter, drift, dropped data, raw artifact integrity |
| Wall wear | inspectable coupons or metrology | repeatable location/area, thickness/mass method, material reference and baseline |

Do not purchase based on advertised range alone. The instrument must resolve the smallest decision-relevant effect after calibration and sampling uncertainty are considered.

## 9. Measurement-system acceptance gates

There is no defensible universal sensor tolerance to declare before the measurand and decision consequence are known. Before physical trials:
- estimate repeatability/reproducibility from pilot repeats;
- quantify bias and drift;
- qualify calibration range and material/path transfer;
- quantify representative-sampling uncertainty;
- demonstrate clock synchronization and latency;
- bound uncertainty at the operating-domain edges;
- set maximum allowable uncertainty from the decision-relevant effect size and safety/engineering review;
- pre-register exclusions, recalibration triggers and stopping criteria.

A qualification record must identify the exact domain: material cohort, PSD/morphology, moisture/density, pipe/path geometry, environmental conditions and signal regime. A sensor can be qualified inside one domain and OUT_OF_DOMAIN in another.

## 10. Integration

- #3934: physical pilot programme.
- #3876: parent fragmentation/transport measurements.
- #3910: morphology distributions.
- #3913: event-level conveying-state evidence.
- #3927: stage-resolved material stream ledger.
- #3890: residual/support geometry.
- #3917: experiment-value and stopping.
- #3987: this measurement-system reference architecture and qualification-record validator.

## 11. Non-claims

This document does not prescribe vendor models, installation dimensions, pipe pressure, airflow, conveying velocity, cutting depth, operating procedure or machine safety limits. It does not qualify a sensor, laboratory, pipe, separator or mine installation. Polymer-pellet sensor accuracy is not transferred to hard rock. Physical qualification requires independent traceable references, representative sampling, a declared uncertainty model and prospective testing on the intended material/path domain.
