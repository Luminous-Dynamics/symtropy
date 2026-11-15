# AG-ROTOR-3: contained inert-payload demonstrator protocol

**Status:** test planning and assurance contract only. This document is not rotor construction instructions, a component specification, a containment design, a human-use design, or authorization to operate machinery.

## Objective

Establish a measured reference for rotation-generated acceleration and model-to-test correlation using a professionally reviewed, fully enclosed, remotely operated, low-energy demonstrator carrying inert payloads only.

The demonstrator tests the complete metrology/evidence chain:
- known geometry and mass properties;
- measured angular speed and acceleration;
- local acceleration at independently surveyed radii;
- vibration/imbalance and bearing/thermal response;
- spin-up/coast-down/controlled-stop telemetry;
- comparison against the analytic reference and declared uncertainty;
- independent review of procedure, calibration and raw data.

It is not a scaled human centrifuge, does not test human tolerance, and cannot qualify an orbital habitat.

## Design principles

1. **Safety case before test article:** the system owner, qualified mechanical engineer, competent test facility and independent safety reviewer must approve hazard analysis, containment, stored-energy envelope, remote operation, exclusion zone and emergency response before fabrication or operation.
2. **Energy-bounded:** select stored energy and speed only after professional containment analysis. No generic RPM, rotor size, wall thickness or material recipe is prescribed here.
3. **Containment-first:** the test article operates inside professionally rated, inspected containment with verified interlocks and remote shutdown. A guard or improvised enclosure is not containment qualification.
4. **Inert payload only:** no person, animal, biological specimen or occupied platform.
5. **No single sensor authority:** use independent speed and acceleration measurement paths. A disagreement or stale channel invalidates the run and commands the approved safe state.
6. **Fail-closed operation:** unknown state, loss of telemetry, interlock disagreement, overspeed, unacceptable vibration, bearing-temperature limit, enclosure fault, or power/control fault prevents start or initiates the approved stop.
7. **No software-only safety:** Symtropy, Symthaea and Mycelix are observational/analysis tools. They do not replace hardwired protective equipment, qualified operators or site emergency controls.

## Required pre-test engineering package

- intended use and explicit non-goals;
- system block diagram and configuration identity;
- hazard analysis covering rotor burst, fragment release, imbalance, bearing seizure, fire/thermal events, electrical/control loss, sensor failure, communication loss, unexpected restart and operator access;
- stored-energy calculation and credible worst-case envelope, including overspeed and failure states;
- containment rating, inspection and independent certification evidence;
- rotor/shaft/bearing/drive manufacturer documentation and operating envelope;
- mounting/foundation and resonance review;
- guarding, interlocks, emergency-stop and restart-inhibit verification;
- exclusion-zone and remote-operation procedure;
- metrology plan, calibration certificates, sampling rates, clock synchronization and uncertainty budgets;
- run procedure, abort criteria, operator checklist, emergency-response plan and post-test inspection;
- change-control and requalification policy.

No test proceeds if any safety-critical design input, certificate, inspection, interlock test or operator authorization is missing, stale or contradictory.

## Instrumentation and measurement channels

Use qualified equipment and calibration traceable to an appropriate national metrology institute or accredited laboratory, as available.

| Quantity | Measurement | Purpose |
|---|---|---|
| Angular position/speed | Independent encoder/tachometer channel(s) | Actual RPM, phase, speed ripple, overspeed detection |
| Acceleration | Calibrated accelerometer/IMU at surveyed radius | Local radial and tangential acceleration |
| Position/radius | Independent dimensional survey | Bind measured acceleration to actual radius |
| Vibration | Triaxial vibration sensor(s) | Imbalance, harmonics, resonance, transient response |
| Bearing state | Approved temperature and condition sensors | Thermal trend and abnormal bearing response |
| Drive input | Voltage/current/power measurement | Electrical input and spin-up energy |
| Time | Synchronized monotonic time base | Cross-channel phase and event alignment |
| Protective chain | Interlock/stop state and independent event log | Demonstrate safety functions and abort chronology |

Do not infer acceleration solely from commanded RPM. Do not use an uncalibrated IMU as the sole speed reference. Preserve raw samples, clock/synchronization quality, calibration uncertainty, sensor placement, filter/processing configuration and dropped-sample counters.

## Test phases

### P0 — documentation and independent review
No powered spin. Close the pre-test package, containment review, operating envelope, failure analysis and instrumentation uncertainty plan.

### P1 — static metrology
Measure geometry, radius, rotor mass properties, mounting alignment, sensor locations, sensor orientation, encoder scale, clock alignment, and zero/bias/noise. Verify run identifiers and raw-data capture.

### P2 — protective-function verification
With the rotor stationary and energy isolated, verify interlocks, start inhibit, independent overspeed trip inputs, communication-loss response, emergency stop, restart inhibit, event logging and safe-state indication. Use approved simulated inputs; do not create unsafe live faults.

### P3 — bounded spin campaign
Only after P0–P2 have independent sign-off. Use an approved, staged, energy-bounded operating envelope. Each stage has explicit speed, acceleration, vibration, temperature, duration, stop and inspection criteria derived by the responsible facility/engineer. This protocol intentionally does not choose numerical operating limits.

### P4 — coast-down and repeatability
Measure natural coast-down only where approved; characterize repeat-run variability, speed ripple, vibration spectra, thermal drift and measurement uncertainty.

### P5 — post-test inspection and evidence closure
Inspect rotor, shaft, mounts, bearings and containment under the approved procedure. Record wear, cracks, looseness, thermal marks, sensor displacement, faults and deviations. A post-test inspection failure invalidates any performance-only pass.

## Model-to-measurement comparison

At surveyed radius r and measured angular velocity omega:

a_radial_model = omega^2 * r

For a declared ideal rotation about a fixed axis, compare measured radial acceleration with the analytic value after accounting for sensor orientation, gravity vector projection, bias, cross-axis sensitivity, filtering, timing, radius uncertainty, speed uncertainty, vibration and transient response.

If uncertainty sources are independent and appropriately modeled, combine them through a declared propagation method; otherwise preserve covariance and correlations. Report:
- signed residual and relative residual;
- combined standard uncertainty and stated coverage interval;
- residual versus RPM, radius, phase and time;
- p50/p95/p99 and maximum vibration over the declared window;
- speed ripple and spin-up/coast-down response;
- repeatability between runs;
- calibration validity and dropped-sample count;
- all aborts, invalidated runs, deviations and exclusions.

A target of <1% acceleration prediction-to-measurement error may be considered only inside the calibrated unmanned envelope, and only after uncertainty and baseline repeatability are established. It is a programme target, not a pass criterion already demonstrated.

## Negative controls and falsification

- stationary sensor-bias and cross-axis checks;
- independently checked geometry/radius;
- encoder scale/sign mismatch;
- deliberately stale or dropped telemetry in a non-powered software replay;
- raw-data truncation, duplicate samples and clock drift in offline tests;
- wrong coordinate frame and wrong sensor orientation;
- commanded-versus-measured RPM divergence;
- fixture configuration mismatch;
- invalid calibration interval;
- missing abort/interlock event;
- deliberately incorrect radius or omega in an offline oracle comparison.

The evaluator must reject false passes. Do not induce an unsafe physical failure as a negative control.

## Evidence receipt

Bind every run to:
- exact code/model commit and blob SHA;
- article/rotor/shaft/bearing/mount/containment configuration and serial identities;
- drawings and as-built geometry digest;
- material and supplier records where relevant;
- procedure and hazard-analysis revision;
- calibration certificates, instrument serials and sensor locations;
- commanded and measured operating envelope;
- time base, sample rate, raw-file digests and processing version;
- uncertainty budget and covariance assumptions;
- protective-chain tests, aborts, warnings and post-test inspection;
- independent reviewer and disposition.

Expected fixtures, analytic calculations, simulations, hardware-in-loop runs and measured runs must be typed separately. A run hash proves artifact identity, not truth or safety.

## Exit criteria

- Approved hazard analysis, containment and independent review before operation.
- Protective functions verified and restart inhibited after stop/fault.
- Calibrated, synchronized, independently checked measurement channels.
- Complete raw data and configuration identity for every run.
- Agreement with analytic kinematics within a predeclared, uncertainty-aware envelope.
- Repeatability characterization and full post-test inspection.
- All invalid/aborted runs retained in the evidence history.
- Independent disposition for the exact test configuration.

## Explicitly excluded

- Human or animal occupancy.
- Open rotor, improvised containment or unreviewed high-energy operation.
- Habitat-scale rotor, crew transfer, pressure vessel, life support, radiation shielding or spacecraft certification.
- Medical/physiological or vestibular tolerance claims.
- Orbital or long-duration qualification.
