# Pump System Energy Service Benchmark M0 v0.1

**Tracking:** [#5103](https://github.com/Luminous-Dynamics/symtropy/issues/5103)  
**Parent hardware design basis:** [PUMP-T1-CW-01 #4994](https://github.com/Luminous-Dynamics/symtropy/issues/4994)  
**Status:** synthetic equal-service head-loss comparison only.

## Why this benchmark exists

ISO/ASME 14414:2019 evaluates the whole pump system from energy inputs to useful work, not the pump nameplate alone. ISO confirms the 2019 edition remained current after its 2024 review. The standard explicitly does not prescribe design, measurement/calibration, savings validation or safe operation, so our benchmark does not claim to replace those activities.

The UNIDO pump-system assessment manual warns that affinity laws transform pump curves but do not determine the operating point after connecting the pump to a system; the system curve is also needed. Static head changes the feasible speed reduction and savings potential.

## M0 synthetic model

For one fixed flow demand Q:
- fixed-speed pump curve: H_p = H0 - b Q²;
- system requirement: H_s = H_static + k Q²;
- fixed-speed throttling head: H_throttle = H_p - H_s;
- synthetic variable speed: n = sqrt((H_s + b Q²) / H0), where the speed-scaled pump curve is H_p(n,Q) = n² H0 - b Q².

Both strategies deliver exactly the same Q and H_s. The fixed-speed pump dissipates H_throttle across the valve; the variable-speed pump lowers its speed to the system requirement.

Hydraulic power is rho*g*Q*H. Electrical power is hydraulic power divided by an assumed wire-to-water efficiency. The fixture intentionally uses the same efficiency for both strategies to isolate the hydraulic head-loss effect; this is not a realistic part-load efficiency map.

## Five synthetic scenarios

The cases cover friction-dominated low static head, high static head, mixed static/friction, near-full-speed duty and small surplus head. Their expected outputs are stored separately and recomputed by the Rust evaluator and an independent workflow oracle.

## What this does not prove

It is not a product comparison, a measured energy-saving test, a full pump curve acceptance test, NPSH/cavitation analysis, transient/hydraulic safety analysis, or an ISO 14414 assessment. It excludes real motor/VFD efficiency maps, power quality, system losses outside the simplified curve, demand time-series, tank dynamics, leakage, degradation and lifecycle costs. The next physical step requires measured curves and matched-service metrology.
