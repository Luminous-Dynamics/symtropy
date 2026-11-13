# Textile Attachment Removal — Instrumented Human Baseline Protocol v0.1

Status: research protocol proposal; no physical trials are claimed by this document.

Parent: Symtropy [TEXTILE-ATTACHMENT-REMOVAL-001 #4941](https://github.com/Luminous-Dynamics/symtropy/issues/4941).

## 1. Purpose and claim boundary

Establish a reproducible manual reference for removing selected garment attachments while measuring time, applied force, textile damage, recovered mass and unresolved material. This baseline is the comparator for later scripted and guarded-robot trials.

A human baseline is not a worker-safety certification, ergonomic approval, industrial throughput estimate, recycling qualification or environmental assessment. A small pilot is apparatus validation, not a population-level success-rate estimate.

## 2. Initial bounded task families

Start with a single non-PPE woven overshirt construction and traceable specimens. Freeze the garment revision, fabric lot, attachment identity and join method before each trial.

| Family | Initial treatment | Admission rule |
|---|---|---|
| Sewn-in care/content label | Manual seam opening and label separation | In scope only when stitch topology and tool access are visible |
| Sewn button | Thread/attachment removal | In scope when the button and thread path are identifiable |
| Hook-and-loop trim | Separate sewn attachment from textile | Adhesive-backed or multilayer unknown construction abstains |
| Coil zipper tape | Open seam and separate zipper tape | Keep slider/stopper removal as a separately identified subtask |
| Decorative sewn trim | Remove the named trim | Mixed or bonded joins require manual review |
| Seam opening | Separate a declared seam segment | Record seam type, length and panel identity |
| Bonded, fused or laminated join | No removal in v0.1 | Abstain/manual-review control only |
| Unknown, hidden or inaccessible attachment | No removal | Abstain/manual-review negative control |

Do not use powered cutting, heat, solvents or autonomous tools in this initial baseline. A task outside the declared in-scope family is recorded as unknown/abstained, not silently reclassified as failure or success.

## 3. Specimen and trial design

- Assign immutable IDs to garment revision, material lot, specimen, attachment, task, attempt, operator pseudonym and tool profile.
- Record source/custody, permission to test, prior wear/repair, visible damage, moisture/contamination state and conditioning history.
- Use one primary removal attempt per independent attachment specimen. A retry receives a new attempt ID and an explicit predecessor link; do not count retries as independent specimens.
- Randomize task order within each operator's available block. Keep the acceptance criteria and force limits fixed before observing outcomes.
- Pilot proposal: at least 10 independent specimens per admitted attachment family, sourced from at least three material/garment lots and handled by at least two trained operators. This is a feasibility and variance-estimation pilot only. Use pilot variance and a preregistered power analysis before making comparative or population claims.
- Record operator experience category and training version. Do not publish personal identifiers or face imagery.

## 4. Measurement and calibration

Before each measurement block, record instrument identity, range, resolution, calibration certificate/reference, calibration date, uncertainty and status.

| Quantity | Measurement rule |
|---|---|
| Input region mass | Define the specimen boundary and weighing method before removal |
| Recovered textile mass | Weigh the intended textile output separately |
| Recovered attachment mass | Weigh detached components separately; preserve component identity |
| Residual mass | Measure material remaining in the specimen/reject stream |
| Unexplained mass | Compute only from the declared balance; never redistribute it to force closure |
| Applied force | Use a calibrated instrument appropriate to the tool/task; publish sample rate, range, resolution and mounting geometry |
| Time | Define start/stop events and use a monotonic clock |
| Damage | Photograph the same registered region before/after; independently count holes, tears and severed yarns; record measured damage area and method |
| Strength retention | Use sacrificial matched coupons and a declared test method only where the sample design supports it |

Integer milligrams in the contract are storage units, not an assertion that every scale resolves one milligram. Store instrument resolution and uncertainty with each observation. Missing, saturated or uncalibrated measurements remain unknown; they are never converted to zero.

The mass boundary is:

`input = textile_output + attachment_output + residual + unexplained`

Report closure error and the unresolved term separately. A closed arithmetic balance does not prove chemical purity or downstream recyclability.

## 5. Operator and inspection procedure

1. Verify task/specimen/tool identity and the in-scope join method.
2. Capture a pre-action image set with a scale/reference and visible specimen ID; avoid faces and unnecessary personal data.
3. Verify the declared tool profile, measurement instruments and task-specific maximum-force/stop criteria.
4. Start the trial clock at the preregistered first-contact event. Record every tool change, reposition, regrasp, intervention and pause.
5. Stop on unknown construction, unexpected hidden layers, loss of tool access, force-limit exceedance, unexpected specimen movement or damage beyond the predeclared stop criterion.
6. Segregate and weigh the textile, attachment, residual and unexplained outputs.
7. Capture post-action images and the complete observation trace.
8. A second inspector, who did not perform the removal, independently labels completion, partial removal, remaining attachment, textile damage and output identity. Disagreements remain explicit and are adjudicated without overwriting the original observations.

Do not choose force or damage limits after seeing a successful result. Limits must be justified by a task-specific risk review and specimen acceptance definition; this protocol does not invent universal safe-force values.

## 6. Required outputs

For every attempt, retain:
- exact garment/revision/specimen/lot/attachment/task/operator/tool IDs;
- requested action, known join method and pre-action state;
- planned procedure, actual action sequence, tool changes and stop/intervention events;
- force/time observations, instrument and calibration identities;
- independent post-inspection and damage observations;
- input, textile, attachment, residual and unexplained mass;
- outcome: removed, partial, not removed, aborted or abstained;
- evidence references, digests, rights/custody and uncertainty;
- explicit missingness and adjudication history.

Keep completion, damage, output purity, mass accounting, cycle time and intervention burden as separate metrics. Do not create a single weighted success score that can trade away damage or a hard stop.

## 7. Matched benchmark sequence

Use the same frozen specimens, task definitions, tool profiles, attempt budgets and acceptance criteria for:
1. trained human manual baseline;
2. fixed scripted/geometric procedure on a guarded fixture;
3. RGB-only perception plus proposal;
4. RGB with force/torque feedback;
5. visual, force and tactile sensing with cloth-state feedback;
6. simulation-pretrained policy;
7. real-demonstration-only policy;
8. calibrated simulation plus real demonstrations/corrections;
9. abstention-enabled policy.

Report per-family and per-lot results, lower-tail performance, uncertainty intervals, failed/aborted trials, safe stops and human interventions. Do not use unfolding benchmarks as a proxy for attachment removal: the ICRA Cloth Competition evaluates grasp selection for in-air unfolding, not disassembly or seam execution.

## 8. Acceptance gates before robot trials

A guarded robot trial is out of scope until:
- the human baseline's instrument repeatability and task definitions are stable;
- the fixture and tool risk assessment is independently reviewed;
- force and safe-stop instrumentation are validated;
- the robot command path is separately authorized and interlocked;
- specimen-specific force/damage limits are frozen;
- an independent recovery procedure exists;
- negative controls prove unknown/bonded/inaccessible joins abstain;
- every trial is auditable from exact configuration and evidence references.

No autonomous cutting around people is part of the initial programme.

## 9. Research targets, not achieved results

After pilot data and a preregistered comparator:
- zero critical false-safe actions in the adversarial unknown/unsafe task set;
- at least 95% removal completion within one explicitly defined held-out attachment family;
- zero damage exceeding its preregistered specimen-specific threshold;
- at least 99% measured mass-accounting closure, or an explicit unresolved residual;
- lower median damage and intervention burden than a matched scripted baseline at comparable throughput;
- every physical removal claim bound to the exact specimen, attempt, tool, calibrated observations and independent inspection.

These are stretch objectives. They are not industry standards, validated results or guarantees.

## 10. Relationship to the executable contract

The Rust crate validates attempt semantics and evidence ceilings. This protocol specifies how a future human-baseline trial should be designed and observed. Neither the document nor a passing contract test proves that any specimen has been physically tested, that a source/reviewer is authentic or independent, or that recovered textile is accepted by a recycling process.
