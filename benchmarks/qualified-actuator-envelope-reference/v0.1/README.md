# Qualified Actuator Capability Envelope v0.1

**Tracking issue:** [ACTUATOR-CAPABILITY-ENVELOPE-001 #6501](https://github.com/Luminous-Dynamics/symtropy/issues/6501)  
**Related mission supervisor:** [#6499](https://github.com/Luminous-Dynamics/symtropy/pull/6499)  
**Related actuator evidence:** #4516, #6443

**Status:** M1 deterministic admission reference. The fixture values are synthetic and do not qualify any actual motor, actuator, joint or robot.

## Why capability is a region

A datasheet peak torque is not a safe command limit at every rotor speed, winding temperature, bus voltage, duty cycle or command duration. The envelope stores a table of empirically qualified rectangular operating cells in speed × temperature space. Each cell has independent positive/negative torque limits and a maximum uninterrupted command duration.

The v0.1 evaluator intentionally does not interpolate or extrapolate. A request inside a cell receives that cell's qualified bound; any request outside the grid is rejected. This avoids silently assuming smooth behavior around nonlinear thermal limits, transmission friction, saturation, backlash or current limiting.

## Typed units

- speed breakpoints and request: milli-RPM;
- temperature breakpoints and request: milli-degrees Celsius;
- supply voltage: mV;
- torque: mN·m (signed request, positive/negative magnitudes in each cell);
- duty: basis points (0–10,000 = 0–100%);
- duration: ms;
- calibration validity: Unix epoch ms.

## Contract and decision

The envelope binds article ID, calibration ID, 32-byte calibration evidence digest, validity interval, supply-voltage band, maximum duty, additional derating margin and a row-major speed/temperature cell matrix.

The evaluator checks, in order: telemetry completeness, calibration validity, voltage, duty, operating-grid bounds, cell duration and the direction-specific derated torque limit. It returns a deterministic decision with the source digest, selected cell index, allowed torque and exact rejection reason.

Cell limits must already be justified by the measurement/uncertainty process. The additional derating margin can only reduce those values. A digest does not authenticate calibration provenance by itself; trust and signatures must be checked upstream.

## Tests

The reference includes positive/negative torque asymmetry, exact internal/outer breakpoint behavior, out-of-domain fail-closed behavior, derating, expiry, supply voltage, duty, duration, missing telemetry, malformed grids and deterministic result binding. Its isolated workflow runs rustfmt, tests and Clippy with warnings denied.

## Promotion path

M0 pins an actual characterization protocol and article/test configuration. M1 is this deterministic reference. M2 derives conservative cell bounds from multiple measured samples with an explicit uncertainty method and independent review. M3 wires admission decisions into Symtropy mission planning and motor-step limits. M4 correlates the capability cells with task-level success, energy, thermal state and recovery. M5 repeats on multiple articles and after service/wear.

## Claim boundary

No physical data is included. The sample fixture is synthetic. This software does not measure torque, estimate an uncertainty distribution, authenticate a calibration certificate, replace a current/thermal limit controller, or serve as a safety-rated interlock.
