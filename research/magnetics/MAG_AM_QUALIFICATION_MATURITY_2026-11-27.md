# Magnetic manufacturing qualification maturity

**Programme:** MAG-AM-QUALIFICATION-MATURITY-001  
**Issue:** [Symtropy #7792](https://github.com/Luminous-Dynamics/symtropy/issues/7792)  
**Revision:** 0.1-draft  
**Date:** 27 November 2026

## Decision rule

Derive the highest evidence maturity supported by the candidate's complete evidence graph. Do not trust a requested maturity label. Missing evidence blocks promotion but does not erase a valid lower level. Every receipt is bound to a frozen candidate scope and to the relevant specimen, lot, build, component, method and laboratory scope.

## MOEM evidence gates

| Level | Required evidence | Does not prove |
|---|---|---|
| M1 | Concept/design/model evidence bound to the frozen application and route hypothesis | Physical material properties |
| M2 | Identified physical coupon and source lot; measured Br/Hcj or other method-appropriate permanent-magnet properties; conditions, method scope, calibration/reference and uncertainty | Process–structure–property causality or component function |
| M3 | M2 plus process recipe/build record, microstructure/phase/texture/porosity/defect evidence and an explicit PSPP link joining process, structure and property observations | Application-level component function |
| M4 | M3 plus inspected as-built component and measured functional response under the frozen application target and conditions, followed by component acceptance | Repeatable manufacturing, durability or process capability |
| M5 | M4 plus independent build/lot cohort evidence against a preregistered replication policy, variability analysis, durability, production-site quality evidence and required equipment/lab-scope records | Formal certification or automatic conformity with any standard |

The M1–M5 scheme follows the MOEM perspective of Fodor et al. (2026). It is a decision/reporting heuristic, not a formal standard.

## Standards are independent capability layers

- ISO/ASTM 52920:2023: production-site and industrial AM process quality assurance.
- ISO/ASTM TS 52930:2021: equipment IQ/OQ/PQ for PBF-LB; its scope excludes feedstock qualification and post-processing beyond powder removal.
- IEC 60404-5:2015: magnetic flux density, polarization, field strength, demagnetization curve and recoil-line measurement methods.
- IEC 60404-18:2025: DC hard-magnet property measurement in an open magnetic circuit using a superconducting magnet, with self-demagnetizing-field correction.
- ISO/IEC 17025:2017: laboratory competence, impartiality and consistent operation; method scope must match the reported measurement.

The evaluator records references and evidence receipts. It does not issue a conformity assessment or certificate.

## Capability vector

Report separately: equipment qualification, site/process quality, feedstock/lot identity, coupon properties, PSPP linkage, as-built geometry, component function, component acceptance, replicated build/lot cohort, variability, durability, laboratory method scope and recovery/downstream-use traceability.

A machine qualification does not qualify feedstock. A lab accreditation does not prove process capability. An M2 coupon cannot become M4 because its geometry is complex. M4 does not imply M5 reproducibility.

## Benchmark controls

The sealed negative set includes maturity-label inflation, literature/synthetic promotion, wrong lot/specimen/component, wrong route/application/measurement setup, missing calibration/uncertainty, cross-layer substitution, insufficient builds/lots, missing durability, absent site-quality evidence and missing PBF-LB IQ/OQ/PQ.

Target: zero false maturity promotions, deterministic receipts and 100% auditable gate-to-evidence traceability on the sealed controls.

## Claim ceiling

The software's result is a structured evidence-maturity disposition only. It is not an engineering approval, accredited laboratory decision, certification, production release or route ranking.
