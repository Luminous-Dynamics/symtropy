# Built Asset Inspection and Repair Corpus v0.1

**Status:** research draft; synthetic only; not a qualified sensor, repair, structural, safety, or building-code benchmark.  
**Parent:** Symtropy #4043 — BUILT-ASSET-ROBOT-LOOP-001  
**Corpus child:** Symtropy #4062 — BUILT-ASSET-CORPUS-001

## Purpose

This document defines the first finite reference subject for robotic and human-assisted inspection of an existing, non-load-bearing building-envelope/service-panel assembly. It separates three states which are often conflated:

1. **Design intent:** what the approved model says should exist.
2. **As-built reference:** what an independent metrology process says actually exists.
3. **Inspection observation:** what the candidate system observed, with its own coverage, uncertainty, calibration, coordinate frame, time and lineage.

The benchmark evaluates whether a pipeline preserves those distinctions and abstains when evidence is incomplete, stale, conflicting or outside its declared scope. It is not a facade-defect detector, a scan-to-BIM algorithm, a structural solver, a repair controller or an acceptance authority.

## Claim ceiling

All v0.1 cases are synthetic, hand-authored known-answer fixtures. Passing proves only that an implementation agrees with this finite contract. It does not establish:
- physical sensor accuracy or field robustness;
- defect-diagnosis validity for real materials;
- structural adequacy, fire safety, weather-tightness or building-code compliance;
- physical repair quality, durability, robot safety or operational readiness;
- lifecycle cost, productivity or carbon advantage.

Every report must display this ceiling.

## Reference subject

The synthetic assembly contains one replaceable, non-load-bearing panel, its frame, four fastener positions and a perimeter seal.

| Identity | Meaning | Reference condition |
|---|---|---|
| SITE-TEST-01 | Synthetic site | Fixed local frame SITE-ENU-01 |
| BLDG-TEST-01 | Synthetic building | One storey, no occupancy model |
| ASM-ENV-01 | Envelope panel assembly | Non-load-bearing |
| PNL-01 | Replaceable panel | Design revision D-02 |
| FRM-01 | Supporting non-structural frame | Design revision D-02 |
| FST-01..04 | Fastener positions | Four declared positions |
| SEAL-01 | Perimeter seal | One declared inspection region |

All geometry values are expressed in millimetres unless a field explicitly states otherwise. Numbers are invented test values, not recommended tolerances or material properties.

### Required observation regions

- PANEL-FRONT
- PANEL-EDGE-LEFT
- PANEL-EDGE-RIGHT
- FASTENER-01, FASTENER-02, FASTENER-03, FASTENER-04
- SEAL-PERIMETER

A complete image count is not equivalent to complete spatial coverage. The oracle checks region identities, not frame count.

### Evidence planes

Each observation binds:
- asset and component identity;
- source capture identity and original byte digest (when a real artifact exists);
- sensor/modality and upstream lineage identity;
- capture time and declared clock domain;
- sensor-to-asset transform and frame identity;
- unit, calibration identity and calibration validity interval;
- observed regions and explicit unobserved regions;
- uncertainty and method profile;
- evidence class and claim ceiling.

Two products derived from the same raw capture remain one lineage for independence accounting. A digest proves byte identity, not sensor correctness or truth.

## Canonical dispositions

Use explicit dispositions rather than a single boolean:
- `ACCEPTED_FOR_DECLARED_SYNTHETIC_CHECK`
- `DEFECT_OBSERVED`
- `CANDIDATE_NOT_CONFIRMED`
- `UNKNOWN_REQUIRED_COVERAGE`
- `INSUFFICIENT_INDEPENDENT_EVIDENCE`
- `STALE_EVIDENCE`
- `CONFLICTED_EVIDENCE`
- `INVALID_INPUT`
- `OUT_OF_DOMAIN`
- `INTERVENTION_NOT_AUTHORIZED`
- `REPAIR_REJECTED_FUNCTIONAL_TEST`
- `REPAIR_ACCEPTED_FOR_DECLARED_NONSTRUCTURAL_FUNCTION`
- `NOT_EVALUATED`

A defect observation is not a cause diagnosis. A repair acceptance is not building approval or permission to reoccupy. The return-to-service authority remains external to this corpus.

## Corpus files

- `fixtures/v0.1/corpus.json` contains the versioned synthetic subject, known truth, observed evidence and expected oracle dispositions.
- This document defines the human-readable semantics and claim ceiling.

The truth section is the evaluator's sealed known-answer reference. Candidate implementations must not read it as an inference input. In physical evaluation, the referee controls truth and the candidate receives only the observation package.

## Required independent-oracle behavior

The reference oracle must:
1. Validate required fields, units, frames, identities, revisions and calibration references before interpreting observations.
2. Compute region coverage from declared region identities; never infer coverage from image count.
3. Preserve sensor lineage and reject false independence from correlated derived products.
4. Invalidate evidence when a relevant design, asset, frame, sensor or calibration generation changes.
5. Keep observations, candidate anomalies, adjudicated defects and causal diagnoses distinct.
6. Keep task completion, claimed repair, independent functional acceptance and return-to-service distinct.
7. Emit deterministic normalized results and stable reason codes.
8. Preserve every case and terminal disposition; crashes/timeouts are not correct defect detection.

The candidate implementation and oracle must not share the code path that creates the expected disposition. Known-answer truth is authored independently. Permutation invariance applies only where order has no declared meaning.

## Paired baseline protocol

For later physical trials, compare at least:
- manual specialist inspection/repair;
- instrumented manual workflow;
- teleoperated robot workflow;
- supervised robotic workflow.

Give every route the same asset revision, reference evidence, defect prevalence, access constraints, acceptance tests and time/resource accounting. Include setup, supervision, access equipment, calibration, tool changes, consumables, travel, rework and post-repair inspection in the workflow boundary.

Do not set productivity or cost improvement thresholds until a matched baseline and measurement protocol exist. Report accepted functional units per robot-hour, direct worker-hour and total workflow crew-hour separately.

## Safety and promotion

v0.1 is offline and synthetic. A later physical stage requires a new, task-specific hazard analysis; guarded commissioning; independent calibrated reference instrumentation; metrology uncertainty suitable for each decision margin; responsible professional review; and a defined stop/recovery procedure. No v0.1 result authorizes motion, repair, structural acceptance or return-to-service.

Initial physical work is limited to a guarded, low-energy, non-load-bearing panel/service article. Exclude high-rise access, occupied/public-space operation, load-bearing repair, fire-protection impairment, live electrical work, gas work and lifting people.

## External benchmark alignment

- NIST Performance Assessment Framework for Robotic Systems: compose perception, mobility, dexterity and safety subtests, then add task-level accepted-work outcomes. https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems
- 2026 review of 135 UAV/deep-learning façade inspection studies: use as a review map, not a single performance baseline. https://pmc.ncbi.nlm.nih.gov/articles/PMC13306407/
- 2026 review of BIM for infrastructure lifecycle management: emphasizes continuity, interoperability, governance and long-term validation. https://doi.org/10.3390/app162110749
- 2026 review of digital twins for civil-infrastructure inspection: https://doi.org/10.1007/s41024-026-00894-8

## Next step

Implement a small Rust parser/validator and independently authored known-answer oracle for the frozen JSON. Add property/metamorphic tests for permutation, unit/frame mismatch, stale revisions, missing regions, correlated lineage and failed post-repair tests. Run locally and report the exact command, toolchain, input digest and output digest; later CI status is separate evidence.
