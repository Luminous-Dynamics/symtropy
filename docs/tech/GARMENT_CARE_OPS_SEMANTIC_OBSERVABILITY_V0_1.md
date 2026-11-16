# Garment-Care Production Episodes, Semantic Observability and Continuous Qualification v0.1

**Status:** evidence/benchmark contract proposal; no physical fleet episodes or operational observations.  
**Tracker:** [GARMENT-OPS-001 #5581](https://github.com/Luminous-Dynamics/symtropy/issues/5581).  
**Composes with:** ready-state/finishing #5561, material passport #5549, enzyme care #5569, laundry service #5471 and microfiber accounting #5495.

## 1. Paradigm shift: the benchmark continues after deployment

A robot may pass a staged fold trial and still fail in production because a gripper wears, lighting changes, garment supply changes, a station is moved, an operator clears a jam, the stack is misplaced, or a sensor drifts. A one-time task score does not measure the production system.

The unit of evidence is therefore a **production episode**: one started attempt at a declared service outcome, linked to its SOP steps, material identity, machine state, failure/recovery, interventions, destination and final customer acceptance.

Two different questions must remain separate:
1. Capability: did this attempt produce the declared accepted outcome?
2. Production reliability: does the system continue to do so across shifts, sites, garment/material lots, hardware revisions, maintenance and environmental drift?

A high success rate from a short demonstration is not production readiness.

## 2. Current external evidence and why it matters

### DYNA's production evaluation loop

In its August 2026 Dyna-2 operational report, DYNA describes a move from a 24-hour napkin demonstration to production evaluation. The company says it records production task episodes, segments them into SOP steps, labels step duration/outcome/failure mode and compares behavior across sites, robots and checkpoints. It provides an example where aggregate throughput had slipped; step-level labels narrowed the cause to missed grabs during initial napkin singulation. It also describes retaining local recording windows and uploading promoted episodes only after the upstream copy is verified.

Reference: https://www.dyna.co/research/scaling-customer-deployments

This is a company-authored operational report, not independent evidence that its implementation is optimal. The important benchmark insight is that its evaluation object is a continuously observed production process, not a short test clip.

### Cloth-manipulation evaluation gap

A February 2026 systematic review examined 41 cloth-manipulation papers from 2019–2024. It identifies diverse garment initial states, Sim2Real transfer, real-world dataset limitations, multimodal sensing and inconsistent evaluation metrics as continuing research gaps.

Reference: https://doi.org/10.3389/frobt.2026.1752914

Our operational episode model should support the research gaps: non-static cloth/material conditions, contact and manipulation uncertainty, step/failure labels, long-horizon outcomes and machine/site context.

## 3. Episode event model

A production episode is a graph of linked, ordered events. Event types are intentionally distinct:

1. `production_run`: task identity, site/station/device, overall start/end and terminal acceptance.
2. `sop_step`: step identity/version, start/end, attempt count, outcome, quality result and relevant sensor/action references.
3. `failure_event`: controlled failure class, severity, step link, evidence references and recovery state.
4. `intervention_event`: operator action, reason, duration, override/recovery and resulting state.
5. `machine_health_event`: calibration, sensor/tool/actuator drift, maintenance, component replacement and fault.
6. `resource_event`: electricity, water, detergent/solvent/enzyme, filter/fibre waste and other material/resource counters.
7. `custody_event`: local artifact digest, promotion trigger, durable transfer, receipt verification, retention and redaction.

Every event should bind:
- immutable episode ID and monotonic sequence number;
- previous-event digest and canonical event digest;
- UTC wall time plus monotonic clock value and clock-quality estimate;
- site/station/robot/appliance ID and hardware revision;
- software/model/checkpoint/configuration digests;
- task/SOP/acceptance-rubric version;
- pseudonymous garment/material passport token, region and endpoint class;
- step/action outcome, quality result, retry/rework and recovery linkage;
- sensor summaries with calibration IDs and uncertainty;
- customer acceptance, adjudication provenance and destination assignment;
- intervention, damage/loss and maintenance links;
- artifact references and custody state.

Do not use owner names or raw household imagery as episode keys. Raw video is not required for ordinary operation if the required telemetry and evidence references exist.

## 4. Reconciliation rules

Counts must reconcile at the declared episode boundary:

- `attempts_started = first_pass_accepted + first_pass_rejected_or_timeout`
- `eventual_accepted = first_pass_accepted + accepted_after_counted_rework`
- retries remain linked to the original failed attempt; a retry is not an independent first-pass success;
- every started attempt has exactly one terminal state: `accepted_destination`, `quarantined`, `human_handoff`, or `lost_or_damaged`;
- a fold in the wrong bin/stack is not an accepted destination;
- unknown terminal state, missing event sequence or unresolved count mismatch blocks the production summary.

A system may continue safe operation after a telemetry problem only if the deterministic local policy permits it; evidence loss must still be visible and must not be recoded as task success.

## 5. Local-first evidence custody

Use a bounded local circular buffer for ordinary high-rate telemetry. On failure, drift, maintenance, quality dispute or audit trigger, promote the relevant time window (including the lead-up and post-event period) to a durable episode.

Rules:
- essential episode recording and reconciliation work offline;
- network loss does not destroy the local episode or block local safe recovery;
- promoted artifacts bind content digests and a signed/attested source identity where available;
- upload can be asynchronous, but the local copy is retained until the upstream receipt is verified;
- partial uploads, checksum failure, duplicate sequences or clock discontinuity remain explicit failures;
- a full ring buffer triggers a recorded capacity fault and deterministic retention action; it cannot silently erase promoted windows;
- privacy minimization/redaction is recorded as an event and cannot erase required outcome/reconciliation evidence.

## 6. Failure taxonomy (versioned)

Initial top-level families:
- intake/singulation: missed grasp, double pick, entangled item, wrong garment;
- perception/material: identity error, occluded region, pose error, wrong destination class;
- manipulation: slip, drop, stretch, snag, collision, wrong fold, flattening failure;
- process/endpoint: incomplete clean/rinse/finish, wrong moisture endpoint, residue or process drift;
- placement: wrong bin/stack/hanger, poor stack organization, misorientation, pickup not ready;
- system: calibration drift, actuator degradation, gripper wear, sensor dropout, clock skew;
- human/system interaction: manual intervention, override, jam clearing, emergency stop;
- evidence: missing event, chain break, digest mismatch, partial upload, receipt absent.

Unknown failures map to `unclassified`, not success. Taxonomy versions are immutable; updates require explicit migration mapping.

## 7. Benchmark protocol

- Freeze identical work cohorts across sites, shifts, hardware revisions, software/model versions and maintenance states.
- Include sustained production: 18-hour commercial shifts, 24-hour endurance, multi-week field intervals and maintenance/cleaning cycles.
- Inject negative controls: calibration drift, gripper wear, changed station layout, lighting shift, textile lot substitution, clock skew, network partition, ring-buffer exhaustion, partial upload, artifact corruption and sensor dropout.
- Test whether semantic labels locate the injected step/failure, not merely whether the system stops.
- Audit random successful and failed episodes to detect missing-not-at-random evidence.
- Blind/adjudicate quality labels where practicable; preserve disagreement and measure inter-rater agreement.
- Report by site/device/hardware/software/garment family; do not hide poor strata in a fleet-wide average.
- Distinguish simulation, desk/analytical models, lab runs, deployment observations and independent replication.

## 8. Metrics and proposed target hypotheses

| Metric | First target hypothesis | Stretch direction |
|---|---:|---:|
| Started-attempt terminal coverage | >=99.99% | 100% reconciled attempts |
| SOP step boundary coverage | >=99% | >=99.9% with independent audit |
| Failure taxonomy coverage | >=99% classified; unknown remains explicit | >=99.9% |
| Adjudicated label agreement | Cohen's kappa >=0.85 | >=0.95 |
| Promoted artifact custody | 0 unverified artifacts accepted as durable evidence | 0 chain breaks in endurance trial |
| Injected fault localization | >=95% correct step/failure family | >=99% |
| Network outage retention | 100% of started attempts preserved locally | Lossless recovery after power/network restoration |
| Operational availability | Report MTBF, MTTR and availability by site/hardware revision | Improve without quality degradation |
| Evidence reconciliation | 100% run/step/retry/terminal counts reconcile | Continuous reconciliation with alerts |
| Cost per accepted output | Include rework, staff, maintenance, idle time and energy | Reduce without reducing service acceptance |

These are proposed targets, not measured results. Freeze thresholds only after pilot estimates establish base rates, label reliability, sensor coverage, clock uncertainty and sample-size requirements. Report confidence intervals and missing-data rates.

## 9. Negative controls and integrity gates

- Missing video/sensor telemetry cannot erase an attempt or convert it to a success.
- Unknown terminal state blocks the production summary.
- A retry remains linked to the original attempt and is not a fresh first-pass success.
- Correct fold/wrong destination is a failed service outcome.
- Run-level summaries reconcile with steps, failures, interventions, rework, damage and destination records.
- Promoted evidence cannot be deleted before verified durable receipt.
- Clock skew, duplicate sequence number, chain break or partial artifact transfer fail integrity qualification.
- Raw video is not mandatory for ordinary operation; pseudonymous identifiers and least-data capture are required.
- Symthaea may propose failure hypotheses and next tests but cannot authorize physical actuation or override deterministic interlocks.

## 10. Luminous integration

- Symtropy: episode schema, telemetry, event reconciliation, benchmark harness, drift and production qualification.
- Symthaea: advisory failure hypothesis ranking, counterfactual diagnostics and next-test selection; no safety authority.
- Mycelix: durable custody/provenance for promoted episode digests, garment lot, service history and maintenance/calibration events.
- Sol-Atlas: operational lifecycle, energy/water/waste, maintenance and uptime trends from qualified telemetry.
- Material passport #5549: binds episodes to garment region/material and treatment history.
- Ready-state #5561: supplies declared endpoint, final quality and destination acceptance.

**Claim ceiling:** benchmark contract and evidence-model proposal only. No continuous recorder, production fleet, failure classifier or deployment reliability result has been built or qualified by this workstream.
