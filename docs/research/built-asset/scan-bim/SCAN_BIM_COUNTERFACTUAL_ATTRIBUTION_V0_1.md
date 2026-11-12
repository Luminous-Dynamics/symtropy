# Paired Counterfactual Scan-to-BIM Change Attribution v0.1

**Status:** P0 synthetic challenge manifest and validator implemented; exact-head Rust/CI qualification pending.  
**Owner:** #4677 — SCAN-BIM-COUNTERFACTUAL-ATTRIBUTION-001  
**Composes:** #1607 survey/change evidence; #4086/#4088 admission oracle; #4116 leakage-resistant replay; #1226 blind-run receipts; #1892 decision impact.

## 1. The paradigm shift: benchmark pairs, not isolated cases

A single scan-to-BIM case asks whether a candidate output matches one expected answer. It does not necessarily prove that the evaluator is testing the causal distinction it claims to test.

This benchmark adds a second unit of evaluation: a **paired intervention**.

```text
same physical/reference world
  ├─ control evidence
  └─ treatment evidence with one declared intervention
       ↓
paired candidate outcomes
       ↓
did the disposition change for the correct reason?
```

For nuisance-only interventions, the candidate must abstain or reject when that nuisance invalidates the evidence; it must not reinterpret the same apparent geometric residual as a physical building change. For a qualified physical-change intervention, the system must remain sensitive enough to admit the change when the declared task tolerance is exceeded and all required upstream evidence remains qualified.

This does not infer real causality from a point cloud. It makes the benchmark's intended intervention explicit and machine-checkable.

## 2. Ownership and non-duplication

| Concern | Owner | This challenge consumes |
|---|---|---|
| Survey, control, registration, independent checkpoints, change-estimate computation | #1607 | Qualified survey status and exact-scope receipt references |
| Pointwise element/model-delta adjudication | #4086 / #4088 | Per-case dispositions and reason codes |
| External replay, evaluator isolation, building/capture grouping | #4116 | Held-out data and candidate/evaluator execution boundaries |
| Blind commitments and tamper-evident run receipts | #1226 | Blind-run custody and protocol commitment |
| Decision-impact and value of information | #1892 | No duplicate planner or VOI calculation |
| Component identity and lifecycle continuity | #3331 | Persistent component identity where the external challenge requires it |

No new registration, geometry, sensor, metrology, event-log, dataset-catalogue, IFC, component-registry or VOI engine is introduced.

## 3. P0 manifest and exact one-factor contract

The executable manifest is:

`docs/research/built-asset/scan-bim/v0.1/counterfactual-pairs.json`

It references the existing 37-case admission corpus; it does not duplicate case truth. Every pair binds:
- a unique pair ID and pair kind;
- control and treatment case IDs;
- a declared list of intervention JSON pointers;
- expected control/treatment dispositions and reason-code sets.

The validator requires:
1. Both cases exist and belong to the same held-out building group.
2. Their independent reference planes are byte-structure equivalent.
3. Candidate inputs become exactly equal after removing only the declared intervention field(s).
4. Every declared intervention field exists and actually differs.
5. Actual control/treatment dispositions and reason codes agree with both the pair manifest and the pointwise admission oracle.
6. Nuisance-only treatments never produce `ADMIT_CANDIDATE_DELTA` or `REAL_CHANGE_CANDIDATE`.
7. A physical-change sensitivity pair has a `NO_CHANGE_SUPPORTED` control and a qualified `ADMIT_CANDIDATE_DELTA` treatment.

Pointwise case reports are sorted by case ID and pair reports by stable pair ID. Reversing input case order therefore preserves semantic case and pair results; the exact input SHA-256 still changes because it binds the serialized bytes. Pair reports separately bind the exact manifest bytes by SHA-256; reordering manifest entries preserves pair semantics/order but intentionally changes the manifest digest.

### Initial pair set

The P0 manifest contains **14 pairs**:

| Pair | Control → treatment | Declared intervention | Expected distinction |
|---|---|---|---|
| CF-001 | SB-002 → SB-026 | Registration stability: stable → drift suspected | Apparent change admitted → request more observation |
| CF-002 | SB-002 → SB-027 | Change-estimate receipt becomes unresolved | Apparent change admitted → request more observation |
| CF-003 | SB-002 → SB-028 | Independent-checkpoint receipt loses its digest | Apparent change admitted → stale/unqualified |
| CF-004 | SB-002 → SB-035 | Change-estimate receipt frame scope changes | Apparent change admitted → stale/unqualified |
| CF-005 | SB-002 → SB-036 | Registration receipt asset scope changes | Apparent change admitted → stale/unqualified |
| CF-006 | SB-002 → SB-037 | Checkpoint receipt survey-product scope changes | Apparent change admitted → stale/unqualified |
| CF-007 | SB-001 → SB-002 | Door deviation and proposed delta change | No change → qualified physical-change admission |
| CF-008 | SB-002 → SB-031 | Qualified change-estimate receipt digest removed | Apparent change admitted → stale/unqualified |
| CF-009 | SB-002 → SB-032 | Qualified registration-stability receipt digest removed | Apparent change admitted → stale/unqualified |
| CF-010 | SB-002 → SB-033 | Independent-checkpoint evidence becomes unresolved | Apparent change admitted → request more observation |
| CF-011 | SB-002 → SB-034 | Survey-product qualification becomes unresolved | Apparent change admitted → request more observation |
| CF-012 | SB-001 → SB-038 | Registration stability becomes drift-suspected | No change → request more observation |
| CF-013 | SB-001 → SB-039 | Change-estimate status becomes unresolved | No change → request more observation |
| CF-014 | SB-001 → SB-040 | Independent-checkpoint status becomes unresolved | No change → request more observation |

The ten evidence-gating nuisance pairs share the same tolerance-clearing apparent geometry, task tolerance and reference. Their single intervention degrades or invalidates one upstream evidence property; the expected result is abstention/stale evidence, never a physical-change admission.

The three no-change nuisance pairs start from the same `NO_CHANGE_SUPPORTED` world. Their single intervention changes only registration stability, change-estimate status or independent-checkpoint status. The system must request additional observation rather than fabricate a change. These three pairs directly probe false-positive change attribution on a known no-change synthetic world.

CF-007 is the positive sensitivity control: only the declared door-deviation and delta fields differ; survey evidence and reference remain fixed.

The Rust validator requires all pairs to remain within the same held-out group and compares the complete candidate objects after removing only the declared JSON-pointer intervention(s). Every nuisance-pair pointer must terminate at a scalar or nullable leaf; object/array subtrees are rejected, and final array-element leaf pointers are supported. The single positive physical-change sensitivity pair may additionally declare the array-valued `/delta/target_ids` field, because its unchanged control contains an empty target set and its treatment adds one target. No other array-valued intervention is allowed. The observed changed-leaf/field set must therefore equal the declared intervention set exactly. A pair that changes any undeclared candidate field is invalid before its outcome can contribute to the report.

All 40 synthetic admission cases refer to one synthetic asset/reference world and therefore belong to one held-out building group. Repeating cases or perturbations does not create additional independent buildings.

## 4. Metrics — no scalar leaderboard

Report separately:
- **Nuisance false-admission rate:** nuisance-only treatments producing an admitted or real-change candidate.
- **Qualified change sensitivity:** sensitivity by physical change type and magnitude/tolerance ratio.
- **Abstention quality:** unresolved evidence correctly producing request-more-observation or stale/unqualified outcomes.
- **Pair contract validity:** reference/group equality, intervention presence, one-factor equality and exact reason/disposition agreement.
- **Selective risk:** false-admission risk versus coverage/admission rate; disclose rejected and unresolved cases.
- **Generalization:** results per independent building, epoch, sensor/profile and capture lineage—not per point or per view.
- **Operational cost:** acquisition time, processing latency, expert review person-minutes, rework and time-to-accepted-change.

Never trade a nuisance-induced false model mutation against a small improvement in surface-distance error.

## 5. Promotion gates and benchmark targets

### P0 — synthetic contract

- 100% expected pair-manifest validation and exact one-factor checks.
- Zero nuisance-only false physical-change admissions.
- 100% pair disposition/reason-code agreement.
- 100% deterministic pair report ordering under case and pair permutations.
- A seeded physical change is admitted only when its lower uncertainty bound clears the task tolerance and every required survey/registration/checkpoint receipt is qualified and correctly scoped.

### P1 — external, blind replay

- Use repeated acquisitions/epochs of the same physical building to construct nuisance pairs; keep those epochs, views, derivatives and sensor lineages in one building/capture group.
- Use a separate reference custodian and blind candidate/evaluator execution from #4116/#1226. Reference geometry, complete-object bounds, truth labels and expected outcomes remain evaluator-only.
- Include stable/no-change, physical change, viewpoint change, temporary object pose, occlusion, registration drift, stale calibration, sensor-profile OOD and material/environment effects.
- Report per-building and per-change-family confusion matrices, abstention, calibrated uncertainty, one-sided confidence bounds for false admissions, and all independent-unit denominators.
- Reproduce historical and commercial baselines on matched acquisitions and explicit human/compute budgets before claiming superiority.

### P2 — bounded physical article

Use one independently surveyed, non-load-bearing panel/service assembly across multiple controlled epochs. Include a true displacement and nuisance-only re-captures with deliberately changed view/registration conditions. Blindly adjudicate the physical truth and keep physical repair/actuation out of the experiment.

## 6. P1 preregistration and statistical acceptance

The unit of generalization is an independently referenced **building / decision unit**, not a point, triangle, pixel, scan station, or repeated view. Multiple epochs and nuisance interventions on one building are paired/repeated observations clustered within that building.

Before candidate execution, pre-register:
- named sensors, firmware/software, calibration/profile envelopes and acquisition settings;
- building, epoch, capture and sensor-lineage grouping;
- physical-change families and magnitude/tolerance ratios;
- nuisance interventions: registration perturbation, view/pose, occlusion/coverage, sensor-profile OOD, environment/material response and stale/mismatched evidence;
- independent reference/checkpoint method and its own uncertainty;
- candidate versions, commercial/published baselines, human-review budget and compute budget;
- exclusions, missing-data handling, stopping rule, primary/secondary endpoints and statistical analysis.

### Primary endpoint: nuisance-induced false physical-change admission

For each nuisance pair, define a false admission as treatment output `ADMIT_CANDIDATE_DELTA` or `REAL_CHANGE_CANDIDATE` when the independent referee confirms no relevant physical asset change. Report the rate by nuisance family and overall, with building-clustered uncertainty intervals. Separately report `REQUEST_ADDITIONAL_OBSERVATION`, stale/unqualified, partial/unknown and incorrect admitted/rejected dispositions; do not hide abstention as either a correct detection or a failure.

As a transparent sample-size calibration, if there are zero false admissions in (n) genuinely independent Bernoulli decision units, the one-sided exact 95% upper bound is (1-0.05^{1/n}). Illustratively:
- 59 independent units with zero events give an upper bound of about 4.95%;
- 149 independent units give about 1.99%;
- 299 independent units give about 1.00%.

These values are **not** a guarantee of adequate power for clustered or heterogeneous trials. If multiple pairs come from the same building, they do not count as independent units. Use a building-clustered bootstrap or a justified hierarchical model, publish the number of buildings and within-building pairs, and report per-building results. Five or six buildings can support exploratory replay, not a strong generalization or rare-failure claim.

### Positive sensitivity and calibration

For independently referee-confirmed physical changes, report detection/admission sensitivity by change family, severity/magnitude-to-tolerance ratio, sensor/profile and visibility stratum. Report the false-rejection and abstention rates separately. For measurement uncertainty, report empirical interval coverage and interval width by task/stratum; do not call a deterministic bound a 95% probability interval. Report confidence intervals for all major rates and retain every unresolved/excluded trial with a reason.

### Matched SOTA and industry comparison

Run identical blinded acquisition/reference sets through:
1. manual expert baseline;
2. pinned commercial workflow(s): Leica Cyclone 3DR, Trimble RealWorks and Autodesk ReCap/Revit where the feature/path is supported;
3. source-faithful published method reproduction(s);
4. Luminous pipeline.

Pin versions, hardware, parameters, tolerances, reference/checkpoint split, visible-surface masks and human effort. Record acquisition, registration, manual editing, review, corrections and rework. Keep the native metric for each research method, then compare common terminal outcomes: independently adjudicated false physical-change admissions, true-change sensitivity, uncertainty coverage, accepted-change time and total expert minutes. No superiority claim is permitted from unmatched vendor feature pages or differently defined published metrics.

**P1 promotion rule:** do not promote on a weighted score. First require zero false admissions on the pre-registered critical nuisance negative controls; then disclose the upper confidence bound using independent building/decision units, and show no material regression in positive physical-change sensitivity, geometric accuracy, uncertainty coverage or completeness. The acceptable upper bound and minimum sensitivity must be chosen by the responsible project/authority before data reveal; this document does not invent a universal safety tolerance.

## 7. Research and industry context

The paired protocol addresses the interface between reconstruction and physical-state attribution, not just geometry reconstruction:
- Wang et al.'s 2026 representation-centred Scan-to-BIM review identifies continuity gaps across measured observations, geometry, relations, semantics, uncertainty, object identity and model state: https://doi.org/10.3390/buildings16204095
- Naeimaei & Schön (2026) show why TLS uncertainty needs both stochastic terms and bounded systematic effects when evaluating inter-epoch deformation: https://doi.org/10.5194/isprs-annals-XI-1-2026-391-2026
- Gregg, Muralikrishnan & Shilling (2026) describe a statistical procedure for detecting longitudinal changes in TLS point-coordinate precision, a useful sensor-health comparator distinct from physical building change: https://doi.org/10.1111/1556-4029.70256
- Jayasinghe & Brilakis (2026) identify complete-object bounding-box leakage in occlusion completion and motivate evaluator-only reference bounds: https://doi.org/10.1016/j.autcon.2026.107052
- Trinh, Hou & Wu (2026) report LOA30 structural element reconstruction with some elements approaching LOA40, showing the need to compare against strong task-specific geometry baselines without conflating them with change attribution: https://doi.org/10.1016/j.autcon.2026.107161

### Commercial baselines must be strong and version-pinned

Do not benchmark against a straw-man “commercial BIM” baseline. The current product surface is materially stronger than geometry-only export:

- **Leica Cyclone 3DR 2026.1.x:** 2026.1.0 adds Scan to Steel, profile selection using point-cloud coverage/deviation, manual adjustment, beam alignment/extension, repeated-element duplication, Navisworks NWD import and BIM analysis workflows. More importantly for the IFC comparison, **2026.1.6 (released 30 October 2026) adds building and storey spatial structure to the Scan to Pipe IFC export**. Pin the exact sub-workflow: the 2026.1.6 hierarchy statement is scoped to Scan to Pipe IFC export, not a blanket guarantee for every export path. Record which steps are automatic versus manually corrected: https://rcdocs.leica-geosystems.com/docs/cyclone-3dr-2026-1-0-cyclone-3dr-release-notes ; https://rcdocs.leica-geosystems.com/docs/cyclone-3dr-2026-1-6-cyclone-3dr-release-notes ; https://rcdocs.leica-geosystems.com/v1/docs/cyclone-3dr-cyclone-3dr-which-bim-analysis-feature-to-use
- **Trimble RealWorks 2026.10:** its release notes document registration refinement using targets, points and point clouds in one operation, plus noise reduction for X7/X9 scans. Pin the station-link and refinement settings, scanner unit, and before/after noise-processing settings: https://help.fieldsystems.trimble.com/realworks/2026.10.htm
- **Autodesk ReCap / Revit:** Autodesk documents point-cloud conversion, registration from scanner/photo sources, cleaning, cropping, regions/classification and mesh workflows for downstream Revit use. Record the exact workflow path and what the downstream representation preserves: https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/Role-of-Autodesk-ReCap-in-scan-to-BIM-workflows.html

These vendor pages establish available capabilities, not measured comparative accuracy, labor or productivity. The benchmark must run matched source observations/reference checkpoints, pin product versions and capture settings, and record human intervention minutes. Use the same task tolerances, visible-surface masks, building split, and output acceptance rubric.

### Research SOTA is not a single leaderboard

- **Active3D (AAAI 2026)** combines implicit/explicit geometry, hierarchical uncertainty volumes, Expected Hybrid Information Gain view selection and risk-sensitive path planning. Benchmark on matched reconstruction quality versus acquisition/motion cost; do not substitute its reconstruction uncertainty for survey-grade absolute accuracy: https://doi.org/10.1609/aaai.v40i8.37585
- **GO-PRE (ICML 2026)** selects views by reducing predictive-rendering entropy over a declared target-view manifold. Add it as a prediction-space acquisition baseline and separately measure downstream building-task correctness: https://proceedings.mlr.press/v306/song26k.html
- **Trinh, Hou & Wu (Automation in Construction 191, November 2026)** report a structural Scan-to-BIM method on incomplete point clouds, with LOA30 (some elements approaching LOA40), over 97% area accuracy, 98% perimeter accuracy, 92% moment-of-inertia accuracy and approximately 11 minutes in their evaluated setting. Reproduce with matching structural classes, reference, sampling and denominators; these values do not establish physical-change attribution or independent metrology: https://doi.org/10.1016/j.autcon.2026.107161
- **Naeimaei & Schön (ISPRS Annals 2026)** show why millimetre-scale inter-epoch change analysis must include interval bounds for persistent systematic effects as well as stochastic uncertainty: https://doi.org/10.5194/isprs-annals-XI-1-2026-391-2026
- **Hartmann, Neumann & Alkhatib (Measurement, April 2026)** report scanner-specific TLS distance-uncertainty modelling against a high-precision laser-tracker reference and explicitly warn that calibration is not transferable across different scanner units. This argues for profile/unit-specific external qualification rather than a generic “LiDAR accuracy” scalar: https://doi.org/10.1016/j.measurement.2026.120908
- **Gregg, Muralikrishnan & Shilling (Journal of Forensic Sciences, January 2026)** describe a statistical interim performance assessment for detecting longitudinal changes in TLS coordinate precision, complementing rather than replacing manufacturer calibration: https://doi.org/10.1111/1556-4029.70256

The strategic differentiation should therefore not be “we also align scans and compare BIM.” Mature tools already do that. Compete on the evidence contract across time: independently scoped survey/change/registration receipts, uncertainty and systematic-effect bounds, persistent identity, explicit abstention, evaluator-blind paired attribution, evidence-preserving IFC exchange, and accepted decision outcomes. Each is an independently benchmarked obligation; no weighted score should allow geometry improvements to offset false physical-change admissions.

### openBIM / IFC comparison: validate the actual published contract

The interoperability track must pin standard maturity, not silently treat development drafts as adopted schemas:

- **IFC 4.3.2.0 / IFC 4.3 ADD2** is the latest official published IFC release and ISO 16739-1:2024. Use it as the normative baseline for current exchange qualification: https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/ and https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/
- **IFC 4.4** documentation is still a working draft. The IFC 4.4 integration plan for ISO submission was approved by buildingSMART's Standards Committee in September 2026, but approval of the plan is not publication of IFC 4.4 as an official standard. Pin a specific 4.4 development build only as a forward-compatibility experiment: https://standards.buildingsmart.org/IFC/DEV/IFC4_4/HTML/ and https://www.buildingsmart.org/standards/standards-committee-consultation/
- **IDS v1.0** is a final standard for specifying and checking information requirements. Its first version does not support geometry checks, computed/dynamic values, checks against external data, or the full as-built comparison problem. Treat IDS as the information-requirement lane, not as a physical or geometric conformance oracle: https://www.buildingsmart.org/information-delivery-specification-ids-v1-0-is-approved-as-a-final-standard/ and https://github.com/buildingSMART/IDS/blob/development/Documentation/UserManual/specifications.md

Therefore, a valid IFC schema parse or IDS pass is necessary for some delivery obligations but cannot establish geometric agreement, sensor measurement accuracy, change attribution, or preservation of uncertainty and survey provenance. The benchmark should separately report schema compliance, IDS information fulfilment, geometry/topology preservation, persistent identity, attribute transformation fidelity, uncertainty/provenance retention and round-trip loss. Keep these independent rather than collapsing them into a binary "IFC valid" field.

Commercial comparison remains with pinned Leica Cyclone 3DR, Trimble RealWorks and Autodesk ReCap/Revit workflows from #4088/#4116. This challenge asks an additional question: under matched observations and reference truth, does a pipeline avoid false physical-change attribution when only a nuisance/evidence-validity variable changes, while still detecting qualified real changes?

## 8. Claim ceiling

P0 proves deterministic paired-contract behavior on synthetic cases. It does not prove real causal attribution, registration stability, producer-byte authentication, instrument calibration, physical measurement accuracy, structural safety, code compliance or authority to mutate an authoritative asset state. P1/P2 claims require independent physical reference evidence, blind evaluation, adequate independent sample sizes and competent professional review.
