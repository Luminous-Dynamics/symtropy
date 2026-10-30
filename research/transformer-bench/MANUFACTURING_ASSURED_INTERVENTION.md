# Assured Manufacturing Intervention Chain

**Status:** research contract only. No actuator, PLC, robot, transformer test-bench or factory control interface is added.

## Why this is a separate capability

A digital twin that predicts quality or recommends a parameter change is not yet an assured intervention system. Once software can affect a physical manufacturing process, the assurance problem expands beyond prediction accuracy: the intervention must be context-bound, permitted, authorized, bounded, executed with a recoverable record, and independently checked afterward.

A systematic review published 28 October 2026 surveyed 235 manufacturing-QA digital-twin studies and reported no reviewed study that brought a fully assured intervention chain together. That review defines an Embodiment-Authority-Assurance framing: what physical action occurs, who controls it, and how its safe/desired effects are proven. DOI: https://doi.org/10.1186/s44147-026-01295-x

This is a promising differentiator for the Luminous stack: Symthaea reasons and proposes; Symtropy owns process constraints, authority and evidence; the physical controller executes only after a separately governed gateway approves.

## Proposed state machine

```text
OBSERVED
   |
   v
PROPOSED -> MODEL_VALIDATED -> CONSTRAINT_CHECKED -> AUTHORIZED
   |              |                 |                  |
   v              v                 v                  v
REJECTED       REJECTED          REJECTED           EXPIRED
                                                       
AUTHORIZED -> EXECUTED -> POSTCONDITION_VERIFIED -> CLOSED
                  |                 |
                  v                 v
               ABORTED         RECOVERY_REQUIRED
```

States are immutable events, not mutable labels. Each transition appends an evidence record. Missing evidence never implies success. Rejection, expiry, abort and recovery are terminal/exception branches requiring explicit resolution.

### Transition rules

| Transition | Required evidence |
|---|---|
| OBSERVED -> PROPOSED | Input snapshot, product/serial/lot, equipment/station, route and test-plan revisions |
| PROPOSED -> MODEL_VALIDATED | Model/version, domain-validity status, predicted effect, uncertainty, discrepancy/OOD assessment, simulation receipt |
| MODEL_VALIDATED -> CONSTRAINT_CHECKED | Parameter/unit envelope, preconditions, safety interlocks, operating window, approved recipe/route, stop conditions |
| CONSTRAINT_CHECKED -> AUTHORIZED | Named authority, signed authorization, exact action/context digest, expiry, one-use nonce, permitted equipment and maximum attempts |
| AUTHORIZED -> EXECUTED | Machine execution receipt, nonce consumption, actual setpoints, start/end timestamps, controller identity and actual process telemetry |
| EXECUTED -> POSTCONDITION_VERIFIED | Independent measurement, calibrated instrument, postcondition limit, measurement uncertainty and evidence receipt |
| POSTCONDITION_VERIFIED -> CLOSED | Verified success, complete evidence graph, no unresolved alarms/NCR/recovery actions |

### Immediate rejection or abort conditions

- Stale or mismatched input/context/model/equipment digest.
- Missing, expired, revoked or invalid authorization.
- Duplicate or replayed nonce, repeated action ID or action outside its validity interval.
- Any proposed parameter outside the approved operating envelope or unit mismatch.
- Unknown/failed calibration, missing test plan, unapproved recipe or failed safety interlock.
- Model reports OOD/unsupported regime, incomplete discrepancy evidence or invalid uncertainty.
- Controller reports a different command/setpoint than the authorized envelope.
- Missing postcondition evidence, expired postcondition window or failed postcondition.

## Authority boundary

- **Symthaea:** proposes actions with evidence, uncertainty, alternatives, constraints and expiry. It cannot sign, authorize, execute or close an intervention.
- **Symtropy:** evaluates product/route/metrology constraints, verifies receipts and policy, obtains accountable authorization, reconciles execution and post-action evidence.
- **Independent policy gateway:** future separately deployed component that performs interlock and authorization checks at the machine boundary. It is not part of the current PR.
- **Machine/PLC/controller:** executes only an authorized, single-use, bounded command and emits a signed or otherwise authenticated execution record.
- **Human authority:** approves consequential actions under the applicable quality plan, risk assessment and operational procedure. Autonomy can be introduced only after the corresponding independent validation and safety case.

The acceptance tool's `eligible_for_authorized_review` result is not authorization. The new acquisition evidence envelope's `eligible` value is not authorization. Neither can cause physical execution.

## Action envelope (proposed schema)

Each proposal/authorization should bind at least:

- action_id, proposal_id, one_use_nonce, action_revision;
- product_family, product_id, serial_number, component/lot genealogy;
- equipment_id, station_id, controller_id, operation_code;
- route_revision, recipe_revision, test_plan_revision, policy_revision;
- input_snapshot_sha256, context_sha256, model_version and model_evidence_sha256;
- parameter names, values, SI units, hard lower/upper bounds and slew/rate limits;
- preconditions, interlocks, stop conditions, max duration, max attempts and abort behavior;
- predicted outcome, uncertainty interval, model-discrepancy status and simulation evidence;
- authorization identity, signature/certificate reference, authorized-at timestamp, valid-until timestamp;
- machine execution receipt, consumed nonce, actual setpoints and telemetry artifact digest;
- required postcondition test ID, instrument/calibration receipt, actual result and uncertainty;
- recovery/compensating action and final state.

All content hashes must cover canonical serialized bytes and the exact policy/model/context; merely storing a 64-character digest is not proof that the content is authentic. The authorization signature must be verified against a trusted identity/role policy, with revocation and expiry checked at execution time.

## Benchmark and assurance gates

Freeze an action-envelope corpus with positive and adversarial cases: correct proposal, stale context, wrong product, changed route, expired/revoked authority, nonce replay, out-of-range parameter, unit mismatch, failed interlock, OOD regime, telemetry mismatch, abort, failed postcondition, missing postcondition and false close.

Proposed pilot targets:

- 100% of executed actions trace to a unique authorization bound to the exact context and parameter digest.
- Zero executions from expired, replayed, unsigned, out-of-envelope or context-mismatched proposals in frozen negative controls.
- Zero CLOSED states without independent postcondition evidence.
- 100% of post-action tests trace to a valid calibration record and current test-plan revision.
- Every abort/failure routes to a declared recovery state, not silent closure.
- Measure false-authorize, false-reject, evidence completeness, action-to-authorization latency, postcondition latency, recovery completion and manual review workload.

These are proposed engineering gates, not certification claims or achieved data. Safety and quality authority remains with the responsible organization and applicable standards/purchaser requirements.

## Industry comparison

The 2026 review highlights the gap between digital-shadow/advisory twins and governed adaptive QA twins. The opportunity is not to maximize autonomy first; it is to make every step of the intervention auditable and rejectable. NISTIR 8620 (21 July 2026) separately identifies interoperability, VVUQ, cybersecurity and workforce readiness as barriers to trustworthy manufacturing digital twins: https://doi.org/10.6028/NIST.IR.8620

## Scope boundary

This specification does not implement physical control, PLC messaging, a robot driver, process write access, cryptographic identity issuance or product release. Those capabilities require separate threat modelling, hazard analysis, trusted-key management, integration testing against equipment simulators and physical interlocks, and formal operational approval.
