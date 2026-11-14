# FILTER-ELI-OBSERVABILITY-001 — Filter Condition and End-of-Life Qualification v0.1

**Tracking issue:** [#5084](https://github.com/Luminous-Dynamics/symtropy/issues/5084)  
**Parent:** [#4958 whole-train benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/4958)  
**Status:** synthetic T0 service-disposition contract; no filter, claim or water-quality result qualified.

## 1. Purpose

Qualify filter condition, service life, bypass state, configuration identity and evidence quality without confusing any of them with contaminant reduction or drinking-water safety.

The comparison target is **verified profile-conforming service over operating life**, supported by measured claim-specific samples and a complete service record.

## 2. Five distinct evidence classes

1. Declared product limit: cartridge/media revision, exact contaminant claim, claimed capacity, interval, flow/pressure envelope and raw-water envelope.
2. Operating state: throughput, elapsed time, flow, differential pressure, bypass/seal state and maintenance history.
3. Claim outcome: paired influent/effluent laboratory samples for the exact analyte, method, unit, sample time, detection/quantification limits and custody.
4. Inspection: cartridge identity, housing/seal condition, fouling/scaling, valve position and device-specific inspection findings.
5. Measurement validity: calibration, instrument range, sampling, laboratory method and uncertainty.

An end-of-life indicator, counter, pressure-drop trend or conductivity proxy is not itself evidence of contaminant reduction.

## 3. Standards crosswalk

- NSF/ANSI 244-2026 is supplemental filtration for otherwise microbiologically safe water. NSF describes an end-of-life indicator requirement and 3-log bacteria and 3-log virus performance criteria. It is not a universal purifier for unsafe raw water.
- NSF/ANSI 42, 53, 55, 58 and 401 have distinct technology and claim scopes. Freeze exact editions and claim listings before comparison.
- WHO household treatment evaluations address bacteria, viruses and protozoa. That is distinct from chemical reduction, service life and local regulatory approval.
- South African projects must use the applicable in-force SANS 241 edition. The 6 June 2025 compulsory national standards require water safety plans and defined monitoring programmes with credible laboratory analysis.

References:
https://www.nsf.org/th/en/knowledge-library/nsf-ansi-244-technical-requirements
https://www.nsf.org/consumer-resources/articles/standards-water-treatment-systems
https://www.who.int/tools/international-scheme-to-evaluate-household-water-treatment-technologies
https://www.gov.za/sites/default/files/gcis_document/202506/52814gon6292.pdf

## 4. Paradigm shift: claim-bounded capacity passport

Bind every ELI to the exact filter/housing/media revision, named claim, raw-water profile, maximum flow/pressure and declared capacity. Measure throughput and feed load when those channels exist. Inspect bypass and seal integrity. Validate end-of-life with challenge tests and independent influent/effluent sampling.

A fixed-volume ELI remains an essential baseline. The research question is whether condition-aware indicators reduce premature replacement without increasing late replacement or false-safe exposure.

## 5. T0 disposition semantics

F0 returns only filter-service and sample-event dispositions. It never returns "water safe", "potable", "SANS compliant", WHO-star or NSF-certified.

- Missing sample means no current claim sample, not no breakthrough.
- Below-LOD/LOQ is censored, not zero.
- Pressure loss indicates hydraulic condition, not contaminant performance.
- Breakthrough is observed only at a valid accepted sample bound to the exact product claim/analyte.
- Configuration mismatch, counter discontinuity, bypass, stale calibration and source-profile shift prevent a qualified service disposition.

The 13 synthetic cases cover normal service, capacity/time limit, high flow/pressure, bypass, sample breakthrough, missing sample, stale calibration, source profile shift, below-LOQ, wrong cartridge and unreconciled service counter.

## 6. T1/T2 physical qualification

T1: freeze product revision and source profile; measure baseline pressure-flow; run matched feed lots; record flow, pressure loss, throughput and interventions; test intermittent use, stagnation/restart, end-of-life, bypass and wrong-cartridge states using controlled non-hazardous methods.

T2: independent laboratory testing for each named microbial/chemical claim at run-in, registered throughput intervals, near declared EOL and post-EOL. Preserve methods, challenge material, custody, uncertainty, LOD/LOQ and actual paired results. Microbial or hazardous chemical challenge tests belong only in appropriately equipped, approved laboratories.

## 7. Metrics and proposed targets (not achieved)

- False-safe disposition count: zero in the frozen corpus.
- False ELI alarms and late replacements per filter-month and per 1,000 L.
- Verified profile-conforming service volume per consumable set.
- Remaining-life error and interval coverage at 50%, 80% and 95% of declared capacity.
- Pressure loss, flow, downtime, water waste, replacement cost and availability.
- Research target: at least 20% more verified service volume or 20% fewer premature replacements in one matched profile, without more missed breakthrough or false-safe exposure.

## 8. Nonclaims

No NSF certification, WHO classification, SANS 241 compliance, potability, contaminant reduction, product life or field reliability is established by this protocol or its T0 synthetic evaluator.
