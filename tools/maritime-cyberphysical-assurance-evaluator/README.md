# Maritime cyber-physical assurance evaluator v0.1

This is an archive-independent P0 contract evaluator for a synthetic 12-class shipboard-microgrid event taxonomy.

It is not a reproduction of the 2026 field-validated microgrid study, and it is not a trained estimator, diagnostic model, energy-management policy or control system.

## Scope

The frozen corpus represents normal operation, propulsion load step, generator trip and derating, BESS capacity and thermal degradation, converter-efficiency degradation, voltage-sensor bias, current-sensor drift, packet loss/delay, one compound physical-plus-information fault, and emergency islanded operation.

The evaluator reconciles physical and information fault IDs separately, checks reported event class and diagnosis set, and fails closed when a compound truth is collapsed into one fault. It computes an uncertainty-bounded event-to-diagnostic delay only when temporal order is proven.

## Timing and consequence metrics

The receipt separates sensor sampling interval, algorithm compute time, event-to-diagnostic delay interval, and sensor-to-command latency. Sensor-to-command latency is deliberately reported as NOT_MEASURED for this synthetic P0. A few milliseconds of compute inside a four-second sampling pipeline is not millisecond end-to-end response.

The receipt also reports synthetic energy-not-supplied, critical-load availability, recovery time and reserve-margin summaries. These values are fabricated fixtures, not vessel measurements.

## Evidence and authority

The input is synthetic known truth. The evaluator verifies SHA-256 over the exact corpus bytes, binds the receipt to the exact source commit, and records source-artifact payload verification as false. No raw vessel telemetry is included.

The evaluator is read-only. It cannot initiate measurements, write controller state, command generators or BESS, shed load, switch topology, authorize maintenance, change vessel mode or clear restrictions.

A pass proves only the frozen synthetic conformance cases; it does not reproduce paper metrics or validate any physical vessel, microgrid, engine, controller, class requirement or operating permission.
