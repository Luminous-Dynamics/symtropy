# Recovered permanent-magnet feedstock route qualification protocol

**Programme:** MAG-RECOVERED-FEEDSTOCK-ROUTES-001  
**Issue:** [Symtropy #7911](https://github.com/Luminous-Dynamics/symtropy/issues/7911)  
**Revision:** 0.1-draft  
**Date:** 28 November 2026

## Decision question

Does one exact recovered-material lot, in one exact feedstock form and one exact manufacturing route, produce a route-eligible magnet material output whose measured chemistry, structure and magnetic properties satisfy a predeclared route-specific acceptance envelope?

A route label alone is never evidence of process eligibility. A feedstock family that has been recovered by HPMS or hydrolysis cannot automatically be substituted into another material family or manufacturing route.

## Why the competitive set changed

Rathfelder et al. (2025), DOI 10.1038/s41598-025-19111-6, demonstrated end-of-life NdFeB powder processed by HPMS, followed by powder extrusion moulding with in-situ magnetic particle alignment and sintering. The paper reports a study-specific Br/Js ratio around 0.96, carbon around 0.045 wt%, and an oxygen increase of approximately 0.4 wt% from HPMS powder to the sintered magnet.

That is a significant near-net-shape manufacturing comparator for recovered NdFeB. It is **PEM**, not MEX:
- PEM is powder extrusion moulding with binder removal and sintering.
- MEX is material extrusion of a polymer-bonded magnetic composite.
- The two routes have different material forms, binder functions, thermal histories, phase/texture evolution, defect modes and acceptance methods.

The 2026 NdFeB AM review, DOI 10.1016/j.jsamd.2026.101226, independently treats MEX and LPBF as distinct routes and identifies magnetic texture and Nd2Fe14B phase retention as performance issues. Its cross-study literature comparisons are not a matched PEM-vs-MEX experiment.

## Route and feedstock strata

The first admission matrix is conservative:
- HPMS-recovered NdFeB powder → powder extrusion moulding / alignment / sintering.
- Virgin NdFeB powder → the same PEM route as a matched control.
- Recovered bonded-NdFeB composite feedstock → MEX only where an exact source-lot/recipe route-eligibility receipt supports it.
- Virgin NdFeB composite feedstock → matched MEX control.
- Virgin atomized NdFeB powder → LPBF only as its own route-specific stratum with its own eligibility evidence.
- HPMS powder is not automatically eligible for LPBF.
- SmCo, SmFeN and ferrite are separate material families; NdFeB results do not transfer to them.

A future route may be added only through a reviewed protocol revision and exact feedstock-family / form / route evidence.

## Evidence graph

Every process trial binds:
1. Original magnet identity and source lot.
2. Recovery process and recovered feedstock lot.
3. Feedstock family, material form, particle characterization and chemistry.
4. Route eligibility receipt scoped to source lot, family, form, recipe and route.
5. Process recipe, process revision, equipment, initiated build and process record.
6. Separate input masses for recovered feedstock and added binder/lubricant/other materials.
7. Every measured output stream: product, residue, reject, scrap, process loss and other output.
8. Output product lot and route-specific physical assays.
9. Method, calibration, uncertainty record, laboratory identity and lab method-scope receipt.

This evaluator validates digest shapes and identity joins. It does not authenticate issuers, confirm accreditation registries or independently verify measurement equipment.

## Material acceptance

Assays are specified by the exact campaign and route. Candidate dimensions may include:
- oxygen, carbon, chemistry and contamination;
- particle-size distribution, morphology and flow properties;
- phase constitution, grain size, texture and orientation distribution;
- density, porosity, cracking and geometric defects;
- Br, Hcj, (BH)max, recoil behaviour and temperature dependence;
- irreversible flux loss after representative exposure;
- mechanical integrity and route-specific green / debound / sintered strength.

Br/Js may be used as an alignment indicator when the saturation method and specimen state support that interpretation. It is not absolute remanence Br and cannot replace Br/Hcj/BHmax or component-function evidence.

The 2025 paper's values are source-specific contextual evidence only. The software fixture is synthetic; its assay limits are illustrative test-policy values and are not universal material standards.

## Mass balance

The v0.1 evaluator compares interval-bounded feedstock plus declared added inputs with the sum of all receipted output streams. A definite non-overlap is FAIL. Missing streams, invalid mass evidence and arithmetic uncertainty are INDETERMINATE.

This is a gross process mass balance, not an elemental balance. Elemental accounting, off-gas chemistry, life-cycle assessment and techno-economic analysis require separate measured data and matched boundaries.

## Decision outcomes

- **QualifiedFeedstockRoute:** route admission, process genealogy, all required output streams, gross mass balance and all declared physical assays pass their route-specific limits.
- **Fail:** valid physical evidence definitely violates a hard limit or the declared gross mass balance.
- **Indeterminate:** missing evidence, invalid receipts, non-physical evidence, unresolved measurement interval, incomplete output stream or process evidence not in a production-record class.
- **NotComparable:** feedstock family/form/route, source lot, product lot, recipe, process revision or laboratory method scope does not match.

The result is not an M4 component acceptance. Recovered material is not considered functionally reused until a consumption receipt links it to a new material lot/build and an independently accepted M4 component, using #7791 and #7870.

## Matched experimental design

The preferred first physical experiment is paired within one feedstock family:
- virgin NdFeB via PEM;
- HPMS-recovered NdFeB via the same PEM recipe;
- same equipment, nominal geometry, alignment-field definition, debinding/sintering schedule, magnetization and measurement method.

Then test MEX in its own paired virgin/recovered bonded-composite stratum. Keep route families separate. Randomize processing order where feasible; preserve all initiated builds, failed builds, scrap and rework; hold out complete source lots and process revisions. An exploratory screen of three builds and two material lots is not a production-capability demonstration.

## Integration and claim ceiling

Compose with:
- #7791 recovery passport;
- #7776 manufacturing cohort denominator;
- #7792 evidence-derived M1-M5 maturity;
- #7798 matched experiment-selection benchmark;
- #7773 as-built specimen route evaluator;
- #7802 system architecture tournament;
- #7887 full-duty-cycle evidence aggregator;
- #7870 second-life rotor architecture.

The synthetic fixture and Rust tests validate software semantics only. No physical recovery yield, powder quality, absolute Br/Hcj, route superiority, accepted component, production capability, environmental advantage, cost advantage or certification is claimed.
