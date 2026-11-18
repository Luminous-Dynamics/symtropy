# Food quality and acceptance assessment V1

Status: software contract and synthetic fixtures. No physical experiment, consumer study, food-safety qualification or appliance validation has been performed.

## What this module does

The production assessor evaluates submitted instrumental observations and blinded consumer responses against one exact, versioned food-quality protocol. It reports descriptive distributions and whether the supplied evidence meets that protocol.

It deliberately does not calculate one composite kitchen score, infer taste from a sensor, make a universal acceptability claim, authenticate external safety authorities, or issue a serving permit.

## Independent evidence lanes

1. **Instrumental quality:** physical measurement, method, sample position, instrument, calibration, uncertainty, unit and basis.
2. **Consumer sensory response:** blinded randomized presentation, scoped population, scale, consent receipt, one-time respondent token and raw-response digest.
3. **Household personal acceptance:** separate private/local evidence; never merged into a representative consumer panel.
4. **External safety and conformity:** echoed separately. The assessor does not authenticate or override the authority, and never grants a serving permit.

A high liking score cannot cancel a failed physical quality limit. Neither can cancel unknown or failed food-safety, allergen, sanitation or regulatory evidence.

## Protocol-specific measurement

- Every sample reference and protocol revision is explicit.
- Liking summaries include count, mean, median, min/max and histogram; no missing response is imputed.
- Instrumental limits are specific to a metric, method, unit, basis and fixed-point scale.
- The whole measurement uncertainty interval must fit inside a limit to count as meeting it. An interval overlapping a limit remains indeterminate.
- Every response and measurement links to a raw evidence digest.
- Protocol digest binds the report to the exact typed protocol revision.
- Respondent tokens are intended to be one-time and study-scoped, not global identities.

## Standards references

Use ISO 11136:2014 (controlled-area consumer hedonic tests; ISO reports it was reviewed and confirmed current in 2025) as a starting point for consumer liking methodology. Use ISO 13299:2016 for sensory profiling (reviewed and confirmed current in 2026). These are methodological references, not a universal recipe score or a replacement for a competent sensory-science review.

For household dishwashers, IEC 60436:2025 including its corrected 2026-04 version is a performance-measurement method, not a safety standard or minimum-performance requirement. Cleaning performance, hygiene, allergen cross-contact and product safety remain separate qualification workstreams.

## Research targets (not achieved results)

- Zero false-complete results on the adversarial suite and at least 10,000 generated malformed/contradictory cases.
- 100% decision-affecting protocol, sample, method, calibration and raw-evidence reference coverage.
- 100% exact digest/revision binding.
- 100% deterministic replay from identical typed inputs and assessor version.
- Zero missingness imputation and zero serving-permit grants.
- Report sample-level distributions and subgroup uncertainty; do not rank on mean liking alone.
- For later physical work, preregister paired comparisons, sample size, consumer population, randomization, blinding, exclusions, analysis model and task-specific non-inferiority margins before collecting results.

## Limitations

The current V1 accepts one instrumental summary per sample/metric. Repeated-measures designs need a later explicit replicate schema. It computes descriptive consumer summaries, not statistical significance, non-inferiority, preference segmentation, nutrition, pathogen reduction, allergen absence or regulatory compliance.
