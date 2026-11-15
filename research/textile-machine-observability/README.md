# Machine observation schema and synthetic fixtures

This directory contains the passive read-only observation contract for [TEXTILE-MACHINE-OBSERVABILITY-001 #5311](https://github.com/Luminous-Dynamics/symtropy/issues/5311).

## Files

- `machine-observation.schema.json` — JSON Schema Draft 2020-12.
- `fixtures/lockstitch-passive-observation.json` — synthetic valid read-only lockstitch observations.
- `fixtures/explicit-missing-channel.json` — synthetic valid record preserving a missing channel as `quality=missing`.
- `fixtures/invalid-command-field.json` — negative fixture that attempts to include a machine command.
- `fixtures/invalid-calibration-reference.json` — negative fixture with a calibrated signal but no calibration digest.

## Boundary

The record is an observation of an exact machine, configuration, job, process recipe, product configuration and material lot. Each channel carries a timestamp, unit, quality state and calibration state. The top-level and nested objects reject undeclared fields. There is deliberately no command, setpoint, start/stop, motor-control or safety-interlock field.

A read-only record is not a machine safety function. A valid schema record is not proof that the sensor is calibrated, the value is physically true or the output article conforms; those require independent calibration and inspection evidence.

## Semantic checks required beyond the schema

A future Rust validator must check:
1. source sequence uniqueness and monotonicity within the exact machine/source/clock identity;
2. channel timestamps within the record acquisition window and no future observations;
3. stale/missing/degraded status not silently promoted to valid;
4. calibration validity interval and sensor-to-machine configuration match;
5. machine, job, process, recipe, product configuration and material-lot identities remain consistent;
6. machine-class/channel compatibility (for example, a shaft phase signal cannot silently appear on an unverified machine profile);
7. no data-origin confusion between OEM, OPC UA, MTConnect, external sensor and manual inspection;
8. no write/actuation surface is exposed by the adapter;
9. derived seam or fabric metrics retain the exact camera/metrology profile and uncertainty;
10. deterministic receipts bind validator, schema, input-record and calibration digests.

## Validation status

All records are synthetic. This is a schema and fixture seed only. No JSON Schema engine, physical sensor, machine connection, electrical measurement, safety approval or production trial is claimed in this artifact.
