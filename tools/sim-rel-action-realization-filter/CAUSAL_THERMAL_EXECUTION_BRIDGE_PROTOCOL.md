# SIM-REL-044 — causal-to-thermal physical execution bridge

Status: first composition layer implemented on a draft research branch. It does not validate a real thermal apparatus or grant a physical safety certificate.

## 1. Motivation

SIM-REL-039 generates exact finite thermal episodes with explicit commanded power, realized actuator power, thermal state and sensor bias. SIM-REL-040 audits command intent, transport, acknowledgement, actuator readback, physical transition, sensor acquisition, inference and decision.

Those modules must not be treated as integrated merely because each independently verifies its own artifact. A valid causal record might refer to a different command than the thermal episode; a thermal simulator might model an actuator fraction that the measured readback does not support.

SIM-REL-044 joins these records into one cross-checked intervention.

## 2. Implemented composition

The public entry point is `verify_thermal_causal_execution_bridge`. It accepts:

- a generated finite thermal challenge;
- the exact finite thermal recipe and registered split registry;
- a SIM-REL-040 causal execution bundle;
- a typed physical/sensor readback binding;
- a pluggable exact-state and sensor-sample commitment verifier.

The verifier first regenerates and verifies the thermal challenge using recipe, registered evidence split and hidden generation seed. It then runs the independent causal execution auditor.

## 3. Explicit unit conversion

The first bridge profile uses:
- causal command setpoint: integer milliwatts;
- actuator readback: integer milliwatts;
- finite thermal model: rational watts.

The conversion is exact:

`P_mW = 1000 * P_W`

The causal command value must equal the thermal model command after this conversion. The actuator readback uncertainty interval must contain the exact thermal realized power after the same conversion.

For a measured value `m` and uncertainty `e`, the non-negative actuator interval is:

`[max(0,m-e), m+e]`

The thermal realized power must lie inside this interval. The verifier does not infer realized power from device acknowledgement.

The binding also records the actuator readback unit explicitly; a unitless integer is not silently assumed to be milliwatts.

## 4. Cross-module consistency

The bridge requires agreement across:

- thermal task identity, recipe/version, family-group ID and evidence split;
- causal command ID and explicit device acceptance;
- command setpoint and thermal commanded power;
- measured actuator interval and thermal realized power;
- physical transition event ID and actuator parent reference;
- exact pre/post temperatures from the thermal episode;
- physical-state digest commitments to the exact temperatures;
- sensor event, instrument, calibration receipt and raw-sample digest;
- exact biased sensor temperature from the thermal episode;
- sensor-sample commitment to the instrument/calibration/readback tuple;
- downstream risk decision versus the thermal hazard outcome.

A thermal hazard may not be promoted to `risk_budget_satisfied=true`. Abstention remains distinct from a passing decision.

A partial actuator delivery may still produce a structurally valid receipt if the causal trace explicitly acknowledges the device, reports the measured realized power, and the thermal model's realized-power value is contained in the measurement interval. The causal audit can correctly classify that realization as outside the requested setpoint tolerance while the bridge confirms that the physical model and instrument record agree about the partial delivery.

## 5. Commitment provider boundary

The physical transition record contains pre/post-state digests; the sensor record contains a raw-sample digest. The bridge does not implement a cryptographic hash. It calls `ThermalEvidenceCommitmentVerifier` to check that those digests commit to the exact model version, state values, instrument, calibration receipt and measured temperature.

The test provider uses deterministic canonical strings only and is explicitly non-cryptographic. Production use requires a versioned canonical encoding and a real cryptographic digest/provider. The test provider is not a production adapter.

## 6. Independence and limitations

A successful bridge receipt establishes that:
- the finite thermal episode regenerates from the recipe and registered split;
- the causal evidence ordering passes SIM-REL-040;
- command and actuator values agree under the specified exact unit conversion;
- model-state and sensor sample commitments agree with the supplied exact thermal values;
- the final decision does not contradict the simulated hazard.

It does **not** establish:
- that the one-node thermal model describes a real furnace;
- that actuator readback/calibration is genuine;
- that the sensor clock and raw data are authentic;
- that physical causation occurred in hardware;
- that the declared operating envelope is safe for an actual apparatus.

Those remain calibration, metrology, cryptographic provenance, hardware-in-the-loop and validated-physics obligations.

## 7. Qualification tests

Focused tests cover:
- exact partial-delivery command/readback/model reconciliation;
- command setpoint mismatch;
- actuator interval excluding the simulated realized power;
- explicit actuator readback unit mismatch;
- pre/post physical state and sensor readback substitution;
- missing device acceptance despite coincidentally matching values;
- thermal hazard incorrectly promoted to a passing risk decision;
- command identity, model version and sensor event substitution.

The branch workflow must run pinned Rust 1.96 formatting, focused bridge tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.
