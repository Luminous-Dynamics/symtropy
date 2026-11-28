# Paired virgin/recovered feedstock manufacturing cohort protocol

**Programme:** MAG-PAIRED-FEEDSTOCK-COHORTS-001  
**Issue:** [Symtropy #7917](https://github.com/Luminous-Dynamics/symtropy/issues/7917)  
**Revision:** 0.1-draft  
**Date:** 28 November 2026

## Research question

Under a controlled, matched process route, does recovered NdFeB feedstock deliver material and functional outcomes that are non-inferior to virgin feedstock while improving one or more preregistered dimensions such as virgin element consumption, accepted-part cost or first-pass yield?

## Why pairing matters

Published recovered-material studies vary in feedstock history, powder morphology, chemistry, recipe, geometry, orientation, magnetization, post-processing, measurement method and evidence maturity. Cross-paper comparisons or comparisons of the best specimen in each paper cannot isolate a recovered-feedstock effect.

The comparison therefore holds process scope and application boundary constant, pairs independent virgin/recovered source lots, randomizes build order, blinds measurements and preserves the full initiated-build denominator.

## Route separation

Run independent strata:
- HPMS-recovered NdFeB powder versus virgin NdFeB powder through the same powder extrusion moulding / alignment / sintering process (PEM).
- HDD-derived recovered NdFeB powder versus a matched virgin composite control through the same PBAT formulation, compounding, filament preparation and FDM/MEX route.
- LPBF only as its own separately admitted route with route-specific feedstock eligibility.

PEM is not MEX. A recovered powder route cannot be assumed suitable for LPBF based on PEM or MEX evidence.

## Preregistration

Before build outcomes are unsealed, freeze:
- material family and feedstock form;
- source-lot pair IDs, source history, incoming chemistry and particle characterization;
- recipe, binder/lubricant, equipment, machine state and process revision;
- geometry, orientation, magnetization and post-processing;
- measurement methods, calibration, uncertainty, blind sample map and acceptance policy;
- randomized pair order and seed commitment;
- metric definitions, directions, hard limits, non-inferiority margins and improvement margins;
- every initiated build ID and the complete paired-block manifest.

## Replication and statistical validity

The proposed first feasibility screen is at least three independent source-lot pairs and at least three paired build blocks per lot pair. This is not a universal sample-size rule, production qualification or process-capability demonstration.

A confirmatory sample size must be determined from pilot variance and a preregistered practical margin / power analysis. Independent source lots and build blocks are the inferential units. Multiple specimens, repeated scans, multiple hysteresis loops or multiple microscope fields from the same build are nested measurements, not independent replicates.

Hold out entire material lots, builds and process revisions for predictive modelling. Do not randomly split measurement rows from the same build across training and test.

## Yield and rework

First-pass accepted fraction is first-pass qualified builds divided by every initiated build. Rework does not retroactively change first-pass status. Final accepted fraction is reported separately. Missing inspection, scrap, failed builds and non-evaluable outcomes remain in the denominator. Cost and process energy must later use the same cohort denominator.

## Decision logic

- NotComparable: route, material family, feedstock form, recipe, process revision, geometry, orientation, magnetization, measurement setup or acceptance scope mismatch.
- Fail: recovered cohort has a definite physical hard-limit violation.
- Indeterminate: insufficient lot/build replication, missing/invalid paired evidence, unqualified evidence or unresolved uncertainty.
- InferiorToVirgin: recovered cohort definitively violates a preregistered non-inferiority margin or observed first-pass yield margin.
- RecoveredRouteNonInferior: every required pair is non-inferior on all mandatory metrics and first-pass yield is non-inferior within the declared margin.
- RecoveredRouteAdvantage: all non-inferiority gates pass and at least one preregistered benefit is robustly better across every required paired block.

The v0.1 evaluator compares finite cohort intervals. It does not make a population-level statistical inference, infer M5 capability, or replace a powered analysis plan.

## Integration and claim ceiling

Compose #7911 route eligibility, #7776 manufacturing cohort semantics, #7798 experimental design, #7792 maturity, #7773 specimen acceptance, #7791 recovery genealogy, #7887 mission profile, #7802 architecture comparison and #7912 element intensity.

The synthetic fixture uses SyntheticControl evidence only and must remain Indeterminate. No physical recovered-vs-virgin superiority, process capability, M4/M5 qualification, LCA or commercial advantage is claimed.
