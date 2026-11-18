# Pump Acceptance-to-Service Transferability v0.1

**Research snapshot:** 18 November 2026  
**Owner:** Symtropy #6008  
**Implementation:** draft Stage 0 Rust evaluator, `tools/pump-acceptance-transfer`.

## Standards and external baselines

### ANSI/HI 14.6-2022 — factory acceptance

The Hydraulic Institute/ANSI description says this standard supplies uniform acceptance criteria and procedures for rotodynamic pump performance, NPSH and hydrostatic testing, including data recording and result reporting and application-specific acceptance grades:

https://webstore.ansi.org/standards/hi/ansihi142022

A test result remains bound to the exact tested article/configuration, test fluid, speed, boundary, facility, procedure and grade.

### ANSI/HI 9.6.1-2024 — NPSH margin

The 2024 guideline separates:
- NPSHa: a system-specific characteristic determined by the system designer;
- NPSHr: a pump-specific requirement supplied by the manufacturer.

It also updates margin guidance and the relationship between tested NPSH3 and manufacturer-supplied NPSHr:

https://webstore.ansi.org/standards/hi/ansihi2024-2610556

This implementation makes the provenance distinction structural. The test NPSH3 field is never used in the site-margin equation. Manufacturer NPSHr and site NPSHa must be separately sourced.

### ISO 9906:2012 and product applicability

ISO 9906:2012 is the published international rotodynamic-pump hydraulic acceptance reference listed under periodic review at the checked catalogue snapshot:

https://www.iso.org/standard/41202.html

The applicability receipt is consumed from [#5987](https://github.com/Luminous-Dynamics/symtropy/issues/5987); this evaluator does not maintain a parallel standards registry.

### Conventional system assessment

The US Department of Energy pump-system programme provides PSAT and MEASUR alongside whole-system guidance:

https://www.energy.gov/cmei/ito/pump-systems

This remains a distinct comparator: a pump factory curve is not a whole-system assessment.

## Decision contract

The Stage-0 input binds:
- pump serial, exact configuration and test speed;
- factory test fluid and hydraulic boundary;
- product/test applicability receipt, standard edition, catalogue snapshot and as-of dates;
- facility, acceptance grade, report and calibration references;
- measured flow/head curve points with uncertainty;
- a separate manufacturer NPSHr curve/reference for the same serial/configuration/speed;
- site/system NPSHa interval, source and method;
- an application-specific NPSH margin profile and required margin;
- site duty point and evidence state;
- post-test repair/change state.

The evaluator does not create a Q-H curve from a nameplate or model. It uses measured points in the fixture only, does not extrapolate, and reports INDETERMINATE when uncertainty overlaps the required head.

## Stage-0 challenge set

The 31 cases challenge:
- serial/configuration mismatch across factory, manufacturer and site;
- speed mismatch (no affinity-law corrections in v0.1);
- fluid/boundary mismatch;
- inapplicable acceptance profile and mismatched catalogue snapshot date;
- missing calibration, manufacturer NPSHr, site NPSHa or application margin;
- duplicate/invalid curve points and unsupported unit profile;
- out-of-domain flows, Q-H interval overlap and head shortfall;
- NPSHa/NPSHr interval clearance, overlap and insufficiency;
- NPSH3 source copied into the manufacturer-curve reference;
- post-test repairs without a current retest;
- stale site evidence and malformed uncertainty intervals.

The expected file is authored independently of the evaluator.

## Metrics and promotion

Report per case:
- exact serial/configuration/standard applicability joins;
- duty-domain coverage;
- Q-H interval width and conservative head residual;
- false transfer and false abstention;
- NPSH3/NPSHr/NPSHa source separation;
- NPSH margin interval coverage;
- evidence completeness and out-of-domain detection;
- execution time and artifact/replay cost.

Hard gates: zero false transfer, zero curve extrapolation, zero NPSH3-to-NPSHr substitution, zero silent identity or unit mismatch and zero promotion from missing/stale evidence.

Promotion:
- T0 synthetic semantics and known answers;
- T1 clean-water factory acceptance data at an appropriately qualified facility;
- T2 held-out test points and uncertainty/measurement-system qualification;
- T3 supervised installed-system measurement at matched duty and boundary;
- T4 replicated duty profiles, facilities, vendors and lifecycle/maintenance outcomes.

## Limits

The current evaluator checks declared identifiers and conservative interval logic on synthetic records only. It does not authenticate referenced documents, derive site NPSHa, select application NPSH margin, qualify instrumentation/facilities, validate an acceptance standard, operate a pump, or establish safety/product/service performance.
