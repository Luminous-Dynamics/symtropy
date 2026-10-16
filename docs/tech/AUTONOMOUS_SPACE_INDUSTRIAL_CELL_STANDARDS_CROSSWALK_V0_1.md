# Autonomous Space Industrial Cell: Standards Crosswalk v0.1

**Research snapshot:** 2026-10-14  
**Status:** Engineering reference for requirements and evidence design. This document does not assert compliance, certification, authorization, or applicability to a particular mission contract.

## Decision

Use standards as sources for traceable requirements, verification plans, software assurance, measurement integrity, machinery risk reduction and technology-maturity vocabulary. Record the exact edition/revision, applicability decision, tailoring rationale and verification evidence for each adopted requirement. A standard name in metadata does not establish compliance; only requirements actually adopted into a controlled project baseline can be verified.

No single cross-cutting space-ISRU qualification standard was identified in the official sources below. Industrial-cell qualification must therefore have a controlled, project-specific requirements and test baseline, with relevant standards explicitly selected and tailored by the responsible engineering/safety authority.

## Official standards and reference sources

| Source | Status checked 2026-10-14 | Apply to this architecture | Boundary / caution |
|---|---|---|---|
| [NASA Systems Engineering Handbook](https://www.nasa.gov/reference/systems-engineering-handbook-appendix/) | NASA engineering handbook/reference; guidance, not a universal flight approval | Requirements validation, verification/validation matrix, test-article pedigree, acceptance/qualification evidence, integration and end-to-end verification. Use the appendices to require a named method, test level, acceptance criterion, and evidence artifact for each adopted “shall.” | Handbook guidance is not itself a mission requirements baseline. Define which program requirements govern. |
| [NASA-STD-8739.8, Software Assurance and Software Safety, Rev B](https://standards.nasa.gov/standard/nasa/nasa-std-87398) | Listed ACTIVE; dated 2022-09-08 | Software assurance/safety lifecycle, hazard contribution analysis and independent verification/validation. Relevant to Symthaea-assisted proposals, deterministic controller baselines, planning adapters, command gates and fault-recovery logic. | A model confidence, consciousness metric, replay or green CI badge is not a software safety case. Tailor the actual standard requirements to the responsible program. |
| [NASA-STD-8739.12, Metrology & Calibration, Rev A](https://standards.nasa.gov/standard/NASA/NASA-STD-873912) | Listed ACTIVE; dated 2024-11-20 | Calibration and traceability for scales/load cells, energy/power instruments, temperature sensors, assay instruments and wear measurements. Attach calibration identity, validity interval, uncertainty and instrument context to measured evidence. | An uncalibrated reading, stale calibration or missing uncertainty cannot close a measured-process gate. |
| [ECSS-E-ST-10-02C Rev.1 — Verification](https://ecss.nl/standard/ecss-e-st-10-02c-rev-1-verification-1-february-2018/) | Listed on ECSS active standards page | Requirements verification strategy, verification control document, test/inspection/review reports, method and verification level. Align each claimed capability with an explicit verification record. | ECSS scope and customer/supplier document model must be checked and tailored for the mission/project. |
| [ECSS-E-ST-10-03C Rev.1 — Testing](https://ecss.nl/standard/ecss-e-st-10-03c-rev-1-testing-31-may-2022/) | Listed on ECSS active standards page; published 2022-05-31 | Ground verification testing of relevant space-segment elements/equipment, test conditions, retest, test uncertainty/tolerances and test reports. Useful when designing pre-deployment qualification packages. | ECSS states exclusions for in-orbit testing, system-level validation and stand-alone software. This is not, by itself, a field-robot or ISRU process qualification recipe. |
| [ECSS active standards list](https://ecss.nl/standards/active-standards/) — including ECSS-E-ST-40C Rev.1 (software) | ECSS currently lists E-ST-40C Rev.1 as active (published 2025-04-30), plus E-ST-10-02C Rev.1 and E-ST-10-03C Rev.1 | Use the software engineering and software product-assurance standards together with the system-level verification/testing standards; retain the particular revision and tailored requirements in the manifest. | A crosswalk entry is not an assertion that this codebase is ECSS compliant. |
| [ISO 10218-1:2025 — Industrial robot safety](https://www.iso.org/standard/73933.html) and [ISO 10218-2:2025 — Robot applications and cells](https://www.iso.org/standard/73934.html) | Editions published 2025-02 | Risk-reduction reference for an Earth-based industrial robot and integrated-cell test rig: robot-level design vs application integration, commissioning, operation, maintenance and decommissioning. | ISO 10218-2:2025 explicitly excludes airborne and space applications, including outer space. Use it only within scope for terrestrial test rigs; do not claim it directly covers lunar/Martian operations. Off-Earth robotics needs a separately approved mission-specific hazard, requirements and qualification baseline. |
| [ISO 12100:2010 — Machinery risk assessment and risk reduction](https://www.iso.org/standard/51528.html) | ISO page says current after 2022 review; a successor proposal, [ISO/DIS 12100.3](https://www.iso.org/standard/88578.html), was in enquiry in 2026 | Structure hazard identification, risk estimation/evaluation, risk reduction and verification across the machinery life cycle. Keep hazards, controls, evidence and residual risk linked. | Track the status of the proposed revision; do not treat a draft International Standard as a published replacement. Applicability to an off-Earth autonomous system requires explicit tailoring. |
| ISO 16290 technology-readiness levels, adopted by ECSS-E-AS-11C (see [ECSS active standards](https://ecss.nl/standards/active-standards/)) | ECSS lists the adoption notice as active | Classify technology maturity with defined criteria and evidence; record the assessed item and context. | TRL is not industrial closure, economic viability, mission authorization, operational readiness or evidence that dependencies are locally reproducible. |

## Gate-to-evidence crosswalk

The existing campaign uses G0–G8. The table below adds verification evidence obligations without redefining or lowering those gates.

| Campaign gate | Minimum evidence hooks | Standard-informed practice | Fail-closed rule |
|---|---|---|---|
| **G0 — Contract** | Requirements baseline, units/frames, source commit, scenario-manifest digest, provenance, dependency graph and claim ceiling | NASA systems-engineering verification/validation matrices; ECSS verification control document concepts | No requirement or dependency with missing identity/units may be treated as closed. |
| **G1 — Safe local operation** | Fault corpus, independent oracle, hazards/controls, stop/recovery traces, exact test configuration | NASA software assurance/safety and IV&V; ISO machinery risk assessment; ISO robot safety for a terrestrial cell where applicable | A plan score or successful nominal run cannot substitute for fault injection, safe-stop behavior or recovery evidence. |
| **G2 — Bulk handling** | Repeated trials, weighed input/output, energy, tool wear, uncertainty, instrument/calibration IDs | NASA metrology/calibration; verification matrix and controlled test report | One successful movement or uncalibrated mass report cannot establish reliable throughput or a reserve. |
| **G3 — Qualified local product** | Frozen design/recipe, batch identities, inspection/assay, acceptance criteria, worst-case results, independent review | ECSS testing/verification structure; ISO 10218 applies only to robot and cell hazards, not product material qualification | A rendered object or one coupon is not multi-batch product qualification. |
| **G4 — Measured ISRU process** | Feedstock provenance, calibration, feed/product/co-product/waste mass balance, power/thermal data, product assay and uncertainty | NASA metrology; requirements-based verification and controlled test conditions | Remote estimates/model outputs stay estimates. Do not mint accepted inventory until measured balance and acceptance criteria pass. |
| **G5 — Repair capability** | Named failure, replaced/repaired part and provenance, elapsed time, reinspection, post-repair tests | Configuration control, verification and acceptance evidence | A successful restart alone is not proof of repair capability. |
| **G6 — Local manufacturing** | Tooling, metrology/control dependencies, qualified inputs, process control and accepted repeat batches | Software assurance for control code plus measurement/product assurance | Any undeclared critical imported dependency blocks a closure claim. |
| **G7 — Resilience** | Frozen failures for comms, sensor/map, power/thermal, relay/process-cell loss, degradation and recovery; independent expected outcomes | System-level V&V and safety-case reasoning across components | No denominator reduction for failed, skipped or missing cases; NotRun is not Pass. |
| **G8 — Reproduction claim** | Second instance built from accounted inputs, complete BOM/dependency provenance, independent QA, retained failures and hidden-dependency audit | Configuration and qualification evidence at the system and product levels | One replicated assembly does not imply general self-replication; the claim is bounded to the tested build and envelope. |

## Required record fields for any adopted requirement

Every requirement promoted into a controlled qualification baseline should have a stable requirement ID, source standard plus exact revision/clause, tailoring or applicability decision, hazard/criticality link where relevant, verification method, verification level, frozen article/configuration, acceptance threshold, instrument/calibration and uncertainty context, scenario-manifest digest, source commit, run/test report digest, independent review identity, result (including fail/blocked/not-run), and disposition of anomalies.

Keep evidence classes distinct: **Synthetic software contract → model/simulation → ground test → relevant-environment test → operational measurement → physical qualification**. Progression is per capability, product and tested envelope, not one global maturity score.

## Status note on retired standards

NASA's official standards system marks NASA-STD-8719.13C (Software Safety Standard) inactive and identifies NASA-STD-8739.8 as its successor; the currently listed active record is NASA-STD-8739.8 Rev B. New records should not cite the retired 8719.13C as if it were active.

## Research sources

- NASA Systems Engineering Handbook and appendices: https://www.nasa.gov/reference/systems-engineering-handbook-appendix/
- NASA software assurance/safety standard active record: https://standards.nasa.gov/standard/nasa/nasa-std-87398
- NASA metrology/calibration standard active record: https://standards.nasa.gov/standard/NASA/NASA-STD-873912
- ECSS official active standards list: https://ecss.nl/standards/active-standards/
- ECSS verification: https://ecss.nl/standard/ecss-e-st-10-02c-rev-1-verification-1-february-2018/
- ECSS testing: https://ecss.nl/standard/ecss-e-st-10-03c-rev-1-testing-31-may-2022/
- ISO industrial robot standards: https://www.iso.org/standard/73933.html and https://www.iso.org/standard/73934.html
- ISO machinery risk assessment current edition and draft successor: https://www.iso.org/standard/51528.html and https://www.iso.org/standard/88578.html


## Applicability correction — 2026-10-14

The official ISO page for [ISO 10218-2:2025](https://www.iso.org/standard/73934.html) explicitly lists “airborne and space, including outer space” among exclusions. Therefore this standard is a useful safety reference only for terrestrial robot/cell integration within its scope; it must not be cited as direct conformity or sufficient tailoring evidence for a lunar/Martian robot cell. Keep the project-specific hazard analysis, system safety requirements, and qualification basis distinct. The Mycelix `StandardApplicabilityRecord` should capture this as `NotApplicable` for off-Earth application (with a rationale) unless a governing authority makes a separate, justified applicability decision for a specific terrestrial subsystem.
