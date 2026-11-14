# WATER-FILTER-LIFECYCLE-F1-001 — Household Filter Service-Life Test Protocol v0.1

**Tracking issue:** [#5140](https://github.com/Luminous-Dynamics/symtropy/issues/5140)  
**Parent assurance kernel:** [#5136](https://github.com/Luminous-Dynamics/symtropy/issues/5136)  
**Evidence class:** synthetic schedule/record contract. No filter performance is measured.

## Layer A — WHO filtration Round IV comparator

For products within its scope, use the official WHO HWT Scheme Filtration Protocol v4.0 as the authoritative test method. The F1 structural validator checks schedule and record completeness; it does not reimplement the microbiological analytical procedure or confer WHO status.

Key protocol requirements relevant to this contract:
- three production units run as triplicates in two test waters;
- GTW and technology-specific CTW, with daily test-water preparation and measured water characteristics;
- 4-day sample schedule: Day 1 GTW start, Day 2 GTW end, Day 3 CTW start, Day 4 CTW end;
- flowing systems receive 10 void volumes or 1 L (whichever is greater) of challenged water for seeding before sample collection;
- minimum 3 L flowing-system effluent sample, or more where analytical method requires;
- cleaning/backwash trigger and actions recorded; do not clean during seeding or sample collection;
- a component that provides microbiological performance is not replaced during the test;
- end-of-test is completion of the prescribed sampling schedule/capacity, or unrecoverable clogging defined as >75% flow reduction from initial flow after permitted cleaning/component replacement;
- manufacturer must provide an explicit effective-use-life warning/indicator or service/replacement instruction.

Primary source: [WHO HWT Scheme Filtration Protocol v4.0](https://cdn.who.int/media/docs/default-source/wash-documents/water-safety-and-quality/household-water-treatment/hwt_scheme_filtration_protocol_4.0.pdf). See Sections 4.1–4.4 and 5; test water and organism specifications are technology-specific and must be followed directly.

The protocol's own organism scope and product-specific test plan govern which microbiological claims can be assessed. This F1 contract does not invent a protozoa method and does not assign a WHO star rating.

## Layer B — supplementary extended-life trajectory

A separate Luminous engineering extension may record condition and performance at registered cumulative-volume landmarks (e.g. start, 50%, 90%, declared capacity) and after every cleaning/backwash/replacement. These supplementary observations do not replace WHO's schedule or become WHO Scheme results.

Each unit must retain serial identity, lot, exact configuration, declared capacity, manual revision, cumulative volume, flow/pressure, water profiles, event logs and independent sample chain of custody.

## End-of-life and restoration

A >75% flow decline is only a clogging endpoint if it cannot be restored by the cleaning/component-replacement procedures permitted by the applicable test plan. If flow recovers after permitted cleaning, do not label it an unrecoverable clogging end-of-test; record the intervention and continue when protocol allows. Flow recovery does not prove contaminant removal has recovered.

A manufacturer end-of-life indicator may terminate discharge, alert, or give explicit service/replacement instructions based on volume/time or other appropriate criterion. The test report must record the declared mechanism and whether the device actually signalled it.

## F1 comparator metrics

- challenge-specific log reduction by organism/test water at required sample points;
- chemical reduction by exact analyte/method;
- throughput before end-of-test;
- flow decline and pressure differential trajectory;
- cleaning frequency and recovery;
- leakage, bypass and storage integrity;
- false-lifetime-remaining and end-of-life indicator behavior;
- cost, cleaning water/chemicals, waste and downtime per litre of independently verified conforming water.

## Nonclaims

The attached 16 synthetic fixtures validate test schedule and record integrity only. They are not test water, microbiological challenge or analytical results. No filter, cartridge, contaminant removal, potability, WHO product rating, lifetime capacity or field availability is qualified.
