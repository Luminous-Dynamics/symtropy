# MFG-INTERVENTION-001 — Governed intervention transaction and post-action verification

**Version:** 0.1  
**Date:** 2026-10-31  
**Evidence tier:** T0 synthetic event-state known-truth  
**Implementation:** `tools/manufacturing-assurance/intervention_ledger.rs`

## 1. Research question

Can a digital-twin-derived manufacturing intervention be represented as a complete, immutable, independently checkable transaction—from proposal through authority, bounded execution, independent outcome measurement, recovery and post-recovery verification?

The key claim is not that a twin can choose a useful action. It is that the action's complete governance and evidence chain is observable, replayable, scoped and auditable.

## 2. Why this work matters now

A 2026 systematic review (published 2026-10-28) examined 235 manufacturing quality-assurance digital-twin studies and reported that none of the studies selected met its L5 governed adaptive QA autonomy criteria. The review highlights fragmentation between evidence that motivates an action, authority, scope, execution, recovery/fallback and post-action quality verification. It also notes that absence of reported information does not prove that a capability is absent in practice, and its maturity classification was performed by one reviewer.

This prototype does not validate that literature-level conclusion. It turns the cited gaps into seeded, falsifiable record requirements.

## 3. Paradigm shift: intervention as a transaction, not an approval flag

A confidence score is not authority. A model recommendation is not an executable command. An acknowledged command is not evidence of a successful outcome. A recovery command is not evidence that recovery worked.

Treat an intervention as a state machine with a signed, immutable event trail:

1. **Proposal:** action, asset, process scope, target, unit, bounded envelope, validity interval and linked assurance-case digest.
2. **Admission:** current precondition, assurance-case and interlock evidence.
3. **Authorization:** named human/system principal; exact asset, scope and action; grant/reject; approval time and expiry.
4. **Execution:** idempotent command ID, executor, command digest, issue time and explicit acknowledgment/rejection.
5. **Outcome verification:** independent measured values checked against versioned acceptance intervals.
6. **Recovery:** rollback, safe-state or containment record after any failed measurement.
7. **Post-recovery verification:** independently measured return to acceptance, covering every failed post-action metric.

The ledger is strictly offline and only evaluates event records. It does not drive equipment or select recovery strategies.

## 4. State machine

`BLOCKED` covers admission/authorization/chronology failure. A rejected command becomes `ABORTED`. An acknowledged command without outcome evidence is `ISSUED_UNVERIFIED`. Passing post-action measurements produce `VERIFIED`. A failed measurement without confirmed recovery yields `RECOVERY_REQUIRED`; a failed recovery or out-of-range post-recovery measurement yields `RECOVERY_FAILED`; and an acknowledged recovery plus passing verification of all failed metrics yields `RECOVERED`.

The state names are local to this prototype; they are not a normative ISA-95, IEC 61508 or safety standard state model.

## 5. Required record fields

### PLAN
- unique plan ID; asset; process/line/site scope;
- action identifier; target value and engineering unit;
- inclusive low/high parameter bounds;
- action-envelope, precondition and assurance-case SHA-256 bindings;
- valid-from/valid-until UTC interval;
- whether a human approval is mandatory.

### CHECK
- unique event ID and plan ID;
- kind: PRECONDITION, ASSURANCE or INTERLOCK;
- PASS/FAIL; UTC timestamp; evidence digest;
- precondition and assurance digests must match the plan's declarations.

All three check classes are mandatory in v0.1. A future profile must define which checks are mandatory for each action class, without silently weakening gates.

### AUTH
- principal type and stable principal identifier;
- asset, scope and action exact match;
- GRANTED/REJECTED decision; approval time and expiry;
- receipt digest.

The checker distinguishes a system principal from a human principal. A plan requiring human approval cannot be satisfied by a system identity alone.

### EXEC
- unique command ID and executor;
- exact approved action digest;
- issue and acknowledgment times;
- ACKNOWLEDGED or REJECTED result.

This version permits one execution record per plan. Multi-step plans and repeated attempts require explicit transaction/retry semantics in a future version; a second EXEC must not be silently interpreted as a valid retry.

### REQUIRE / VERIFY / RECOVERY
- REQUIRE declares each required post-action metric, unit and lower/upper acceptance interval before execution;
- VERIFY records metric, numeric value, unit, measured time and evidence digest;
- POST_ACTION and POST_RECOVERY stages are separate;
- RECOVERY records mode, issue/acknowledgment times, status and evidence digest.

Every REQUIRE metric must have a POST_ACTION verification before the transaction can become VERIFIED. Recovery verification must re-measure every failed post-action metric with the same metric, unit and acceptance bounds. Missing one required or failed metric cannot close the transaction.

## 6. Frozen known-truth panel

The valid fixture is a synthetic machine/line action:
- target feed rate 120 mm/min within 80–150 mm/min;
- precondition, assurance and interlock records PASS;
- action digest matches the command digest;
- current HUMAN authorization is granted for exact machine, line and action;
- execution is acknowledged;
- surface roughness 1.4 μm is within [0, 2] μm.

Negative controls in the Rust tests mutate one or more records:
- missing/failed gate;
- digest mismatch;
- authority absent, expired/rejected, wrong action, wrong asset/scope or system-only when human required;
- out-of-bounds target;
- command mismatch;
- missing execution acknowledgment or outcome measurement;
- out-of-range post-action metric with no recovery;
- failed/early/late recovery;
- missing post-recovery evidence or incomplete failed-metric coverage;
- post-recovery metric outside its declared range;
- duplicate event/command, invalid timestamps, malformed digest and NaN.

These are record semantics, not actual factory process trials.

## 7. Metrics and target thresholds

These are proposed engineering gates, not measured claims.

| Axis | T0 target | T2/T3 target |
|---|---|---|
| Admission defects | reject 100% frozen missing/failed/stale/mismatched gates | independent generated fault campaigns, confidence-bounded false-completion rate |
| Authority | reject all scope/action/time/principal mismatches | external identity, signatures, key rotation/revocation and separation-of-duty trials |
| Command identity | reject duplicate command IDs and digest mismatch | idempotency/replay analysis across real gateways and event transport |
| Outcome verification | no successful completion without post-action measurement | independently traceable sensors, calibrated uncertainty and coverage |
| Recovery | no recovery success without acknowledged recovery and complete passing re-measurement | independent fault-injection and safe-state acceptance tests |
| Chronology | exact UTC order and half-open interval correctness | bounded clock uncertainty, event ordering under distributed clock faults |
| Auditability | deterministic report bytes and reason codes | independent replay across hosts/toolchains and signed provenance |
| Performance | report on fixed small fixtures | runtime/memory/tail-latency at 10k/100k events; no unsupported real-time claims |

The highest-cost error is false completion, not false rejection. Report false completion, false rejection, abstention, defect localization and missing-evidence cut sets separately.

## 8. Industry and standards comparison

### Manufacturing governance frontier

Use the 2026 systematic review as a gap taxonomy, not as a vendor benchmark. It identifies authority, scope, traceability, reversibility/recovery and post-action verification as important missing evidence categories.

### Assurance-case tooling

Adelard ASCE is an established GSN/CAE authoring and evidence-management product with defeater support and structural validation. Ansys medini Analyze covers safety analysis, traceability and consistency checking. The ledger is not intended to replace their visual authoring or engineering analysis workflows.

Instead compare on whether the same seeded intervention transactions can be admitted/rejected consistently and whether each system captures:
- exact action envelope/version;
- current named authority and revocation;
- command identity and digest;
- independent post-action measurement;
- complete recovery and post-recovery evidence;
- deterministic audit export and change invalidation.

No capability should be marked missing from a product based only on public marketing pages. Test the specific release, configuration and integration.

### Industrial digital thread

Compare with event traces from MES/SCADA/PLC gateways and change/quality systems under a read-only data export. Evaluate whether the event trail can be bound to the assurance-case graph, VVUQ artifacts and lifecycle-change evidence without granting this verifier actuation access.

### Relevant safety/automation standards

Future requirements mapping must review applicable domain and site-specific standards, including risk management, machinery functional safety, process safety, industrial control system security and quality management. This prototype is not a compliance implementation of IEC 61508, IEC 62061, ISO 13849, ISA-95, IEC 62443 or any other standard. Applicability must be determined by qualified engineering authorities for the actual machine/process.

## 9. Integration with existing work

- #3121/#3150: composition VVUQ, evidence admission and causal/intervention outcome evaluation.
- #3165: invalidate action envelopes/assurance digests when their upstream process, material, calibration or model dependencies change.
- #3167: diagnostic faults can block a plan or force recovery/manual review.
- #3173: coupling conservation/time contracts become plan admission evidence.
- #3174: covariance/uncertainty determines whether acceptance margins are adequately justified.
- #3178: assurance-case graph records proposal, authority, execution and post-action evidence as typed claims/evidence.

A future integration must bind exact evidence artifact bytes and signed receipts. Mutable URLs, branch names and free-text descriptions are not sufficient identities.

## 10. Evidence roadmap

1. **T0:** synthetic transaction fixture and fault-injection panel.
2. **T1:** property-based sequence generation, independent checker, signed event receipts, replay/duplicate delivery and clock-skew attacks.
3. **T2:** public or partner-provided historical manufacturing process/quality traces with licence, calibrated sensors, units, lineage and immutable digests; read-only only.
4. **T3:** independent lab validation of measurement and recovery evidence under controlled perturbations.
5. **T4:** industrial shadow mode and independent safety/quality review. Any later actuation design requires separate hazard analysis, safety architecture, human authority and site acceptance.

## 11. Claim ceiling

The v0.1 code is an offline event-record verifier. It does not send commands, select actions, authenticate principals or signatures, verify raw measurement bytes, prove physical recovery safety, or establish causal benefit. A `VERIFIED` record result is not a safety certification, production permission or authorization to actuate.
