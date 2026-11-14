# Maritime repair outcome and capability restoration v0.1

**Owner:** [MARITIME-REPAIR-OUTCOME-001 #5094](https://github.com/Luminous-Dynamics/symtropy/issues/5094)  
**Parent:** [MARITIME-ENGINEERING-HEALTH-001 #4444](https://github.com/Luminous-Dynamics/symtropy/pull/4444)  
**Status:** synthetic P0 contract and evaluator authored; hosted qualification pending  
**Data status:** no external marine-engine model or test-bench outcome has been evaluated by this module.

## Why this is a distinct engineering capability

A maintenance record can show that work was completed. An alarm can clear. A sensor can return to normal. None of those facts alone establishes that the affected physical capability was restored.

The new evaluator asks a narrower but stronger question:

**Under comparable load, speed, configuration and other declared conditions, does an independent calibrated measurement show that the repaired capability is now inside its acceptance envelope, and did the intervention avoid degrading other required capabilities?**

It complements:
- #4444's asset/configuration-bound evidence admission and read-only capability impact;
- #4545's RUL and maintenance-policy utility;
- #4548's source-grounded causal risk evidence;
- #4535's sensor/data trust and missingness contracts;
- #4609's fault observability and independent-measurement semantics;
- #4628's control-intent / actuator-feedback / physical-response distinctions;
- #5085's immutable historical diagnoses and evidence-driven revisions.

It creates no second diagnosis engine, RUL engine, work-order lifecycle, or authority surface.

## Paradigm: from "repair done" to measured restoration

The contract keeps seven different facts separate:

1. a diagnosis revision motivated an engineering response;
2. an external system records the repair/intervention and its authorization reference;
3. a pre-repair test episode records the degraded capability;
4. the intervention has a bounded start/completion time and explicit before/after configuration digests;
5. a post-repair test uses operating conditions within frozen matching tolerances;
6. independent calibrated measurements are compared to an owner-defined capability envelope;
7. follow-up exposure is later needed to establish recurrence/durability.

This P0 evaluator covers steps 2–6. It intentionally does not claim that the intervention was authorized, that the evidence issuer is authentic, or that the repaired condition will remain healthy.

## Executable P0 semantics

The Rust module is `src/repair_restoration.rs`.

### Matched duty

Every configured condition (for example engine load and speed) must be present in both test episodes, use the declared unit, and remain within a preregistered absolute tolerance. A lower post-repair load cannot create a false restoration result. A condition mismatch produces `not_evaluable`, not a pass.

### Independent measurement path

Every post-repair metric must:
- be current and valid;
- have verified calibration;
- have uncertainty no greater than the metric envelope's maximum;
- use a measurement principle not present in the diagnostic evidence path;
- use a dependency domain not present in the diagnostic evidence path.

If the diagnostic sensor/gateway is also the only post-repair measurement, physical restoration is not established. Replacing or calibrating a sensor may restore measurement quality, but does not by itself prove that the plant has recovered.

### Capability envelopes and uncertainty

For each capability, the evaluator checks the entire measurement uncertainty interval against the declared acceptance envelope:

- interval wholly inside: `within`;
- interval wholly outside: `outside`;
- interval crossing an acceptance boundary: `boundary_uncertain` and the case is `not_evaluable`.

A targeted function must exhibit a pre-repair deficit before it can be called restored by the intervention. The post-repair result is reported capability by capability.

### Non-target capability regression

A healthy non-target capability that moves outside its declared envelope after the repair forces `non_target_capability_regression`. No aggregate score can compensate for it. If one of several target capabilities is restored while another remains degraded, the result is `partial_restoration`.

### Causal attribution

Even a matched before/after improvement with no recorded concurrent change is labelled `before_after_association_only`. If a controller update, sensor replacement or other concurrent change is recorded, the result is `confounded_concurrent_changes`.

Causal repair-effect claims require an appropriate controlled or counterfactual design beyond this P0 evaluator.

## Synthetic known-answer contract

The evaluator currently contains 12 Rust tests and an 8-case machine-readable challenge manifest. The cases cover:
- verified restoration under matched conditions;
- repair record with no physical recovery;
- unmatched operating load;
- reuse of the diagnostic dependency domain;
- non-target capability regression;
- partial restoration across multiple targets;
- no pre-repair deficit;
- uncertainty overlapping an acceptance boundary;
- concurrent changes and causal attribution;
- repair chronology;
- missing/stale evidence;
- protocol/corpus integrity.

The JSON corpus describes scenario-level expected outcomes. The Rust tests construct complete episode/case objects with separate measured values.

## How to compare against SOTA and industrial platforms

This work does not try to outperform an anomaly classifier on its own metric. It establishes a second evaluation stage after diagnosis.

Industrial offerings such as ABB Ability Digital Powertrain and Digital Powertrain Insights publicly describe monitoring of drives/motors/driven equipment, anomaly detection, condition/trend analysis and prioritized maintenance intelligence. ABB's current Digital Powertrain help also distinguishes strong, weak and unknown issues and invites users to review asset evidence to confirm or reject a diagnosis. These are meaningful deployed capabilities; the public product material does not provide a matched marine-engine repair-restoration benchmark score. See:
- https://www.abb.com/global/en/areas/motion/services/asset-health-and-monitoring/abb-ability-digital-powertrain
- https://www.abb.com/global/en/areas/motion/services/asset-health-and-monitoring/abb-ability-digital-powertrain-insights
- https://powertrain.abb.com/powertrainonlinehelp/en

The differentiator to test is whether a system can tie the exact diagnosis revision and intervention to independent, matched-duty, per-capability restoration evidence—without confusing a sensor fix, alarm reset or low-load trial with recovered physical service.

A 2025 systematic review of industrial condition-monitoring evaluations found substantial heterogeneity in performance measures and economic analysis, motivating transparent maintenance-policy, sensitivity and uncertainty reporting. A 2025 intervention-aware risk-curve study explicitly models measured maintenance effectiveness and collateral effects, but its validation is limited to a feedwater-pump type. A 2026 causal repair-impact workflow using hydraulic sensor logs is a replication lead, not a marine-engine result:
- https://doi.org/10.1016/j.jmsy.2025.06.015
- https://doi.org/10.3390/app152010998
- https://doi.org/10.1016/j.eswa.2026.133024

## Benchmark metrics and aspiration

Report separately:
- false restoration acceptance on sham/no-op/sensor-only controls;
- per-capability restoration precision/recall;
- matched-condition eligibility and rejection;
- independent verification coverage;
- non-target capability regression rate;
- time to independently verified restoration;
- recurrence per engine-hour/cycle after a declared follow-up horizon;
- concurrent-change confounding rate;
- repeat interventions, downtime, labor/parts and lifecycle cost only when source data support those quantities.

Initial aspirations (not measured results):
- zero false restoration acceptance on the frozen synthetic suite;
- zero physical restoration claims from sensor-path correction alone;
- 100% of accepted restoration results use an independent calibrated measurement path;
- zero unreported non-target capability regressions;
- 100% of restoration claims bind a diagnosis revision, repair record, post-test evidence and acceptance-envelope digest.

## Claim ceiling and next steps

This P0 contract checks the consistency of declared repair records, test windows, matched conditions, measurement quality/independence and per-capability envelopes. It does not authenticate the repair provider, validate the acceptance envelope's engineering authority, infer causal repair effects, establish safe life, or authorize maintenance/operation.

Next evidence steps:
1. run the exact-head Rust/schema workflow;
2. compose an evidence reference to the exact diagnosis revision and repair record;
3. recover permissioned test-bench pre/post runs with instrument maps and calibration records;
4. add independent controls for sensor-only correction, sham intervention, controller changes and low-load confounding;
5. add recurrence/follow-up exposure before any durability claim.
