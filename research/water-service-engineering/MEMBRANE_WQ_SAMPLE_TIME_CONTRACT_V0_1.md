# M2 Sample-Time Water-Quality Label Contract v0.1

**Dataset:** BRISTOL_POU_WQ_12W, EIDC DOI 10.5285/98148025-e8ef-40f9-907e-dae8b28106d2  
**Parent:** MEMBRANE-FOULING-QUAL-001 #4977  
**Status:** synthetic contract fixtures only; no laboratory sample values or timestamps are imported.

## Purpose

Keep laboratory evidence attached to the exact sample event it represents. The Bristol trial metadata describes five-minute operational telemetry and laboratory sample summaries from three treatment-chain locations at nine timepoints. That is not a dense water-quality time series.

A lab result is a label for an analyte at one sampling location and sample time, measured by one method and reporting limit. It must never be forward-filled, backward-filled, interpolated or copied to adjacent telemetry rows.

## Required sample record

- Stable sample ID and source dataset/revision.
- Actual sample-collection timestamp, timezone and provenance.
- Treatment-chain location/stage ID, not just a generic “water quality” label.
- Analyte identifier, measurement result, unit and qualifier.
- Lab/method ID, accreditation scope where applicable, method version.
- LOD and LOQ with units; result qualifier such as quantified, below-LOD, below-LOQ, invalid or not reported.
- Sample custody / QC status, replicate identity and any lab comments.
- Exact telemetry feature cutoff: features may use data measured at or before sample collection only; post-sample telemetry is forbidden.
- Explicit sample-only label scope. No rows are created for unobserved timepoints.

## Qualifier semantics

- Quantified results preserve the measured numeric value and unit.
- Below-LOD / below-LOQ results preserve the qualifier and relevant limit. Do not silently replace them with zero or the limit value.
- Invalid, rejected or missing results are not training labels.
- A result from one location is not reused as a label for another treatment stage.
- Multiple samples at the same nominal time but different locations are independent records.
- An analyte's detection limit is not an operational compliance limit; the legal/contractual quality threshold belongs in a separately versioned acceptance profile.

## Temporal alignment

The admissible feature window is strictly causal:
`feature_observation_time <= sample_collection_time`.

No interpolation of the lab result is allowed across five-minute telemetry. If no valid telemetry history exists before the sample, the model context is incomplete and must abstain or explicitly use only the sample as a standalone observation.

## Nonclaims

This schema and its synthetic fixtures do not contain real Bristol lab results, actual sampling timestamps, drinking-water limits or compliance determinations. They validate label semantics only. They do not qualify potability, membrane integrity, rejection, treatment efficacy or a production monitoring system.
