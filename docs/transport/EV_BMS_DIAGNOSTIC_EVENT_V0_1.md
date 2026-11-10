# EV BMS Diagnostic Event Evidence Contract v0.1

**Status:** Synthetic evidence-contract validator. Not a BMS implementation, vehicle controller, diagnostic accuracy model or automotive safety case.
**Owner:** [EV-BMS-DIAG-001 #4459](https://github.com/Luminous-Dynamics/symtropy/issues/4459).
**Research anchor:** SAE J3356_202609, issued 28 September 2026. J3356 is an SAE Information Report covering fault identification, fault classification, system reaction and diagnostic data/reporting; this project does not claim certification or conformity by referencing it.

## Paradigm shift

A diagnostic event is a lifecycle, not a fault-code boolean:

Observation → candidate identification → classification → reaction proposal → authority decision → execution/acknowledgement → post-action measurement → independent verification.

The contract keeps these states separate. A low SOH estimate is not itself a BMS fault. A fault code is not proof of a physical failure mechanism. A recommended service is not an authorized service; an issued/acknowledged command is not proof the physical response occurred; a closed work order is not proof of benefit.

## Evidence bindings

Every event binds vehicle/pack/module identity and configuration digests; BMS hardware/software/firmware/calibration/diagnostic-rule identity; an operating-context window (temperature, SOC and time); channel observations with units, timestamps, validity, sensor calibration and source digests; fault candidate class/severity/confidence and supporting evidence; and reaction-policy, authority, operating-envelope and execution/verification receipts.

The v0.1 validator requires every fault-candidate evidence digest to reference a valid observation in the same event. Suspect/invalid observations remain visible, but cannot support a classified fault in this contract.

## Reaction state machine and hard invariants

- A model may recommend `service`, `replace`, `derate` or `isolate` while authority is `not_requested`, provided execution remains `not_executed`.
- Acknowledged, executed, failed or unknown execution requires explicit approved authority plus an execution-receipt digest.
- Unknown/failed execution requires reconciliation and mutation blocking. A known failed event binds its execution receipt; an unknown event may lack an execution receipt because the outcome itself is unresolved, but it still requires approved attempted-action authority. The unresolved result must be reconciled before a further mutation is considered.
- Executed service/replacement requires a maintenance-event digest.
- A pending verification state is distinct from a verified outcome.
- Verified/rejected outcome requires an execution receipt, post-action test evidence, and an independent verifier reference/digest distinct from the reaction authority.
- No command is issued, no BMS parameter is modified and no vehicle is actuated by this v0.1 tool.

## Frozen synthetic corpus

Ten fixtures, five accepted and five rejected. Accepted cases distinguish classification-only, recommendation-not-authorized, executed-but-unverified, independently verified service and unknown-execution reconciliation. Rejected controls cover unauthorized execution, verified-without-post-test, unsupported classification, stale/out-of-window observation and acknowledgement without authorization.

## Benchmark programme

Compare threshold/rule baselines, physical/model residual diagnostics, classical classifiers and multimodal learned models on identical group-disjoint physical assets and operating contexts. Report event-level sensitivity, false alarms per vehicle-hour, detection delay, fault-isolation accuracy, macro-F1, worst-domain recall, calibration (Brier/log/reliability), abstention coverage/selective risk, reaction constraint violations, telemetry completeness, and latency/memory/energy.

Synthetic injection, simulation and hardware-in-the-loop are separate evidence classes from physical fleet observation. Classification accuracy without event prevalence, asset-level splits, missed-event denominators and false-alarm exposure is not a meaningful deployment claim.

## Claim ceiling

The audit checks schema, provenance and state-transition consistency in producer-supplied events. It does not prove source sensors are accurate, verify the physical meaning of a fault class, independently attest authority, validate a vehicle response, authorize service, certify a BMS or establish safety. Hashes prove byte identity, not truth.
