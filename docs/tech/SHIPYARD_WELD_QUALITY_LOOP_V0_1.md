# Shipyard Weld Inspection, Repair and Feedback Benchmark v0.1

**Status:** synthetic evidence-contract seed. No NDT classifier, inspection procedure, acceptance engine, repair controller, production WPS/PQR or weld-release authority.

## Purpose and owner boundaries

Parent: [SHIPYARD-WELD-QUALITY-LOOP-001 #4443](https://github.com/Luminous-Dynamics/symtropy/issues/4443).

- [SHIPYARD-WELD-TWIN-001 #4433](https://github.com/Luminous-Dynamics/symtropy/issues/4433) owns multipass weld workpiece/equipment/process pass identity and process traces.
- [COMPONENT-DIGITAL-THREAD-001 #3331](https://github.com/Luminous-Dynamics/symtropy/issues/3331) owns canonical component/joint inspection, NDT and as-built records.
- [SHIPBUILD-CO-DESIGN-001 #2048](https://github.com/Luminous-Dynamics/symtropy/issues/2048) owns accepted shipyard production and production scheduling.
- [MARITIME-ASSET-TWIN-COMPOSITION-001 #4406](https://github.com/Luminous-Dynamics/symtropy/issues/4406) owns federated quality/maintenance records and twin commissioning.
- Mycelix remains owner of responsible issuer, inspector/repair authority, attestation, custody, revocation and approvals where connected.

This work owns the NDT report/indication/repair/reinspection evidence contract and feedback benchmark; it must not duplicate the source quality system or the weld process verifier.

## Standards and technical boundary

**ISO 17635:2025, Edition 4**, published 2 April 2025, gives general rules for selecting NDT methods for metallic welds and evaluating results based on quality requirements, material, weld thickness, welding process and testing extent. It covers method, technique and acceptance-level selection. Its public abstract explicitly says NDT acceptance levels are not a direct interpretation of ISO 5817/ISO 10042 quality levels; correlations are only general and technique-specific.

Authoritative public metadata/abstract: https://www.iso.org/standard/85705.html

The standard is copyrighted. This benchmark records its identity and public abstract only. Do not reproduce normative tables or use non-open ISO text for ML/AI; obtain lawful access and respect the publisher's terms.

## Paradigm: quality feedback rather than post-weld labels

The end-to-end manufacturing loop is:

`process trace -> inspection plan -> acquisition -> indication -> qualified evaluation -> disposition -> repair -> reinspection -> release evidence -> governed learning`

A higher defect-detection rate may mean the inspection became more sensitive or coverage increased; it does not necessarily mean manufacturing quality worsened. A lower reported defect rate may mean inspection coverage/method quality degraded. The benchmark therefore binds method, technique, sensitivity/task, coverage, denominator and examiner authority to every observation.

A causal model may propose hypotheses; correlation is not root cause. No learned recommendation automatically changes a WPS/PQR, procedure envelope, robot program, inspection coverage, acceptance criterion, repair route or source-of-record disposition.

## Typed record contract v0.1

Each inspection binds:
- exact asset/component/serial/assembly/joint and weld-region identity;
- workpiece/design revision and weld-process reference;
- inspection method and technique, procedure/revision, qualification scope, material/thickness/process applicability;
- planned/actual coverage, sampled extent, inspected length/area/volume and denominator;
- governing project specification/contract acceptance basis and edition, criteria reference, quality level (if used) and separate NDT acceptance level;
- equipment/probe/sensor, software/processing revision, calibration/reference check and validity;
- inspector identity, qualification/certification scope and validity;
- observation/acquisition/evaluation/report/transaction/retrieval/cutoff times as distinct roles;
- raw signal/image/report artifact digest, rights, source, derivative-processing lineage and signature/issuer context;
- indications with region, location, orientation, reported type, measured extent, uncertainty/confidence and report link;
- qualified evaluation/disposition, reason, responsible authority and supersession/revocation state.

Each repair binds its source indication, authorized route/procedure, responsible authority, removed/prepared region, repair execution evidence, new workpiece/process revision, dimensional/metrology evidence where required, reinspection plan/result and final release authority. A report marked accepted by a synthetic fixture is not a real NDT acceptance.

## Feedback graph and governance

Join exact evidence in a source-reversible graph:

`workpiece/joint -> pass/layer -> process trace -> inspection -> indication -> disposition -> repair -> reinspection -> accepted configuration`

Candidate features can include material heat/lot, fit-up, joint geometry, pass/layer, process trace, robot/equipment calibration, tooling, WPS/PQR revision, environment, NDT modality, coverage and inspector report. Keep source ownership, missingness, confounding and dataset split grouping visible.

Feedback promotion ladder:
1. Descriptive pattern: a reproducible association with exact records and denominator.
2. Hypothesis: candidate cause with alternatives/confounders and uncertainty.
3. Independently adjudicated root-cause label: qualified expert review with blinding where feasible.
4. Controlled improvement proposal: exact revision, scope, expected benefit/risk and requalification needs.
5. Authorized engineering/process change: owner-approved, requalified and recorded separately.
No benchmark output itself moves a case up this ladder.

## Frozen v0.1 corpus

The frozen executable corpus contains 42 unique synthetic cases: one valid inspection/repair chain and 41 adversarial controls. The Rust evaluator must independently reproduce every expected disposition. It covers method/technique applicability, project acceptance basis, ISO quality-level vs NDT-acceptance confusion, coverage and sampling denominator, calibration, examiner scope, artifact integrity, indication/joint/pass binding, ambiguous/rejected disposition, duplicate/out-of-order report delivery, repair authorization/scope, reinspection lineage, configuration release, coverage-shift confounding and leakage across welds/batches/campaigns/facilities.

The corpus is a fixture seed; it is not a NDT evaluator or defect classifier.

## External research-data candidates

Sources are modality-specific and require exact-byte/licence/label audit:
- Strathclyde PAUT probe-localization data, available 6 June 2025, 98.9 MB, CC BY 4.0. It supports probe/weld localization, not general defect acceptance: https://pureportal.strath.ac.uk/en/datasets/data-for-using-phased-array-ultrasound-to-localize-probes-during-/
- Strathclyde FMC ultrasonic data: welded 316L stainless-steel plate with a stated 6 mm lack-of-fusion flaw and metadata; university page states CC BY 4.0: https://pureportal.strath.ac.uk/en/datasets/fmc-dataset/
- 2026 SMAW surface-defect imagery: 448 images and three surface categories, reported CC BY 4.0; separate from volumetric GMAW NDT: Mendeley Data record titled "Annotated Image Dataset for Shielded Metal Arc Welding (SMAW) Surface Defect Detection".
- RIAWELC radiography: 24,407 images and four classes; repository says freely released but a specific licence was not established here. RightsReviewRequired: https://github.com/stefyste/RIAWELC
- WELD-DETR 3,247-image / 5,892-region dataset: reported 70/15/15 random image split; paper-specific baseline only, no proof of weld-/campaign-/facility-level generalization: https://pmc.ncbi.nlm.nih.gov/articles/PMC12656187/
- SWRD paper (June 2026) reports 3,675 radiographs and 7 classes with high image-level accuracy. Results are not a universal SOTA and source data rights/splits need independent review: https://doi.org/10.1016/j.compeleceng.2026.111135

Do not conflate surface photos, radiographs, PAUT and FMC. Detection, localization, characterization, code-based evaluation, disposition and root-cause attribution are different tasks.

## Benchmarks and targets

- Evidence contract: 100% oracle agreement on the frozen structural corpus; zero false acceptance on missing basis, incomplete coverage, expired calibration, wrong examiner scope, wrong subject, revoked report and unapproved repair negatives.
- NDT detection: sensitivity at a fixed false-call rate per weld length/volume; indication localization error; per-class sensitivity; probability calibration and uncertainty; inspection coverage and blind-reference adjudication.
- Disposition: qualified expert agreement and abstention, separately from model detection.
- Repair: first-pass yield, final yield, repeat-defect rate by region, repair yield, repair labour, reinspection load, time to disposition and final release completeness.
- Causal feedback: root-cause ranking precision/recall against blinded expert adjudication, confidence calibration, abstention, false-attribution rate and robustness to inspection-method/coverage shifts.
- Production economics: cost/time per accepted joint and accepted weld length, not raw arc-on time or defect count alone.

Split by complete physical weld/joint, component, batch/material lot, campaign, equipment, WPS/PQR revision, acquisition session and facility where identifiers permit. Image patches or adjacent scan segments from one physical weld cannot cross train/test. Hold out full campaigns and facilities for generalization claims.

Never claim performance superiority from random image-level accuracy alone. Report the benchmark source/version, licence, labels, split unit, denominator, inspection method and coverage alongside every metric.

## Claim ceiling

A synthetic PASS establishes only the declared evidence-contract behaviour. It does not establish NDT validity, inspector qualification, defect truth, acceptance-code compliance, root cause, weld soundness, process change approval, class approval, or permission to release a vessel/structure.


## Executable feedback contract seed — 2026-11-14

The reviewed branch adds a synthetic Rust evaluator and a mutation corpus. It reuses this issue's schema and parent ownership rather than forking process identity or canonical NDT records.

The valid chain begins with a rejected synthetic indication, records an authorized repair, reinspection of the same weld region after repair completion, and final synthetic release after the reinspection. Exact reinspection and release timestamps are required and point-in-time checked. The evaluator emits only `REVIEW_CANDIDATE` for engineering review. It does not infer root cause or authorize a WPS/PQR, process or release change.

The 42-case corpus contains one positive chain and 41 negative controls for exact subject/pass identity; method/material/thickness/process scope; coverage/denominator; separate NDT acceptance basis; equipment calibration and examiner qualification; issuer authentication, artifact digest and rights; report supersession/revocation and point-in-time cutoff; qualified disposition and required repair; repair route/authority/source indication/region/execution evidence; reinspection of the same region and final release; train/test leakage and method/coverage-shift confounding; and root-cause/WPS-change promotion without independent adjudication and owner authorization.

Owner interfaces remain:
- #4433 exact weld/pass/process trace;
- #4500 NDT provider, method, examiner, equipment, coverage and report scope;
- #4443 indication disposition, repair, reinspection and governed feedback;
- #3331 canonical component inspection/as-built record.

This is synthetic software verification only. It does not establish physical probability of detection, sizing accuracy, defect truth, accepted weld quality or actual root cause.
