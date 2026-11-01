# SIM-REL-040 — causal action-realization evidence

Status: first typed logical auditor implemented on a research branch. It is not a device-authentication protocol, calibrated instrumentation system or physical-safety certification.

## 1. Objective

A commanded action is not an executed action. A transport acknowledgement is not a measured actuator state. A sensor reading is not, by itself, proof that the intended command caused the measured physical transition.

SIM-REL-040 defines a replayable evidence chain:

`Command intent -> Dispatch -> Device acknowledgement -> Actuator readback -> Physical transition -> Sensor acquisition -> Inference -> Decision`

Every stage has a distinct typed record and event identity. The audit must preserve ambiguity rather than infer missing evidence.

## 2. Implemented first increment

The Rust module `decision_value::causal_execution` exports `audit_causal_execution_bundle` over eight linked records:

1. **Command intent:** command ID, target device, requested value/unit, tolerance, operating envelope and uncertain timestamp.
2. **Dispatch:** transport ID, payload digest, device, explicit Accepted/Rejected/Unknown state and uncertain timestamp.
3. **Device acknowledgement:** firmware identity, Accepted/Rejected/Absent state, optional echoed setpoint and timestamp.
4. **Actuator readback:** measured value, measurement uncertainty, calibration receipt ID and timestamp.
5. **Physical transition:** before/after state digests, model version and an explicit parent reference to the actuator readback.
6. **Sensor acquisition:** instrument ID, raw sample digest, calibration receipt, quality flags and parent reference to the physical transition.
7. **Inference:** sensor event references, inference version, conclusion and uncertainty digests.
8. **Decision:** policy identity, decision digest, inference reference, abstention and risk-budget outcome.

The first version supports a single command, one dispatch, one acknowledgement and one actuator readback. Retries, asynchronous multi-command control, compensation and multi-sensor fusion are not yet represented.

## 3. Core invariants

- Event sequence is contiguous from command intent through final decision.
- Event IDs are globally unique inside the bundle.
- Every event is bound to exactly one command ID.
- Target device identity is consistent across command, dispatch, acknowledgement and readback.
- The physical-transition event references the exact actuator-readback event.
- Sensor acquisition references the exact physical-transition event.
- Inference references the sensor event, and decision references that inference.
- Each pair of adjacent event-clock uncertainty intervals must be strictly non-overlapping and ordered.
- Acknowledgement echo must match the commanded setpoint; contradictory transport/device status is rejected.
- The measured actuator interval is compared to the requested setpoint tolerance interval. It is classified as WithinTolerance, OutsideTolerance or MeasurementAmbiguous.
- If no device acceptance is present, actuator readback is Unattributed even when its numerical value matches the request.
- A decision that exceeds its risk budget remains recorded as a violation; abstention is distinct.

The strict time check means the latest plausible time of event A must be earlier than the earliest plausible time of event B. Nominal timestamps that are ordered but have overlapping uncertainty are insufficient to establish temporal precedence.

## 4. Realization assessment

For requested value (u), allowed tolerance (	au), measured value (m), and measurement uncertainty (e), the target interval is:

`[u-\tau, u+\tau]`

and the measured interval is:

`[m-e, m+e].`

The readback is:

- **WithinTolerance** if the entire measured interval is inside the target interval;
- **OutsideTolerance** if the intervals are disjoint;
- **MeasurementAmbiguous** if they overlap but the full measured interval is not contained;
- **Unattributed** if device acceptance is absent, regardless of numerical agreement.

This is deliberately conservative. A nominal value inside tolerance with an uncertainty interval crossing the boundary is not called a successful realization.

## 5. Independence boundary

The auditor independently validates typed-record consistency, event order, clock uncertainty, device identity, explicit causal references and setpoint/readback arithmetic.

It does **not** authenticate signatures, prove clocks are honest, verify that a calibration receipt is genuine, independently inspect raw instrument payloads, establish that physical state digests reflect the real apparatus, or prove that a state change was caused by the command. Those require trusted device identity, signed transport receipts, calibrated metrology, independent sensor provenance and where possible hardware-in-the-loop replay.

A passing receipt means the supplied evidence chain is internally consistent. It does not mean the physical intervention is true.

## 6. Test plan and qualification

The focused fixtures cover:
- a fully linked command-to-decision chain;
- acknowledgement versus actual actuator readback;
- missing device acceptance with matching readback (must remain Unattributed);
- within-tolerance, outside-tolerance and uncertainty-overlap cases;
- conflicting device/transport status and mismatched acknowledgement echo;
- device identity substitution;
- overlapping timestamp-uncertainty intervals;
- stale sensor causality, duplicate event IDs and operating-envelope violations;
- risk-budget violation versus explicit abstention.

The branch workflow must run pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.

## 7. Next extensions

- **SIM-REL-040a:** multiple dispatch attempts with idempotency keys, duplicate-delivery resolution, retry/cancel/compensate semantics and explicit unknown delivery.
- **SIM-REL-040b:** clock-source identity, synchronization receipts, drift bounds and calibrated timestamp uncertainty.
- **SIM-REL-040c:** signed device/transport evidence and independent raw-payload parsing.
- **SIM-REL-040d:** hardware-in-the-loop replay adapters for thermal, mechanical and laboratory instruments.
- **SIM-REL-041:** matched benchmark adapters that evaluate command realization and evidence attribution alongside scientific decision quality.

Do not infer physical causality from correlation alone. Where the evidence does not distinguish intended action from confounding events, report attribution uncertainty or abstain.
