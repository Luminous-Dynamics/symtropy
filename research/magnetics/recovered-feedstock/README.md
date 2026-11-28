# Recovered permanent-magnet feedstock route qualification

**Programme:** MAG-RECOVERED-FEEDSTOCK-ROUTES-001  
**Issue:** #7911  
**Revision:** 0.1-draft

## Strategic premise

Do not treat "recycled NdFeB powder" as a universal feedstock. Recovery changes chemistry, oxygen/carbon content, morphology, particle-size distribution, surface condition, phase and process behaviour. Route admission and material acceptance must be specific to the source lot, feedstock form, process recipe and product lot.

The important competitive shift is to compare recovery-enabled manufacturing routes, not only additive printer families.

## PEM is not MEX

The literature now provides an important route distinction:

- **Powder extrusion moulding (PEM):** metal / magnetic powder is compounded with a temporary binder system, shaped by extrusion moulding, debound and sintered. The relevant 2025 recovered-NdFeB study used in-situ magnetic particle alignment and subsequently sintered anisotropic magnets. It is a near-net-shape powder-processing route, not the same process as MEX bonded-magnet printing.
- **Material extrusion (MEX):** a polymer-bonded magnetic composite is deposited as a printed geometry. Binder fraction, powder loading, alignment, inter-layer bonding and post-processing create a different process–structure–property problem.
- **Recovered HDD NdFeB → MEX/FDM:** a June 2026 study used hard-drive magnet powder, milled/sieved below 50 μm, compounded with PBAT at up to 50 wt%, converted to filament and printed by FDM. A field applied during printing aligned particles and increased the reported remanence from 11 to 28 emu/g; the reduced-remanence ratio increased from 0.21 to 0.49. Coercivity fell from 1852 to 1310 Oe in the aligned 50 wt% specimen, so this is a multi-property trade-off, not a simple win.
- **LPBF/PBF-LB:** uses a laser powder bed with its own powder, atmosphere, energy-density, cracking, phase-retention and texture constraints. HPMS powder is not automatically eligible.
- **HDDR / dynamic-HDDR → DLP 4D:** a November 2026 study demonstrates magnetically programmed recycled powders in photopolymer composites and external-field actuation. This is an adjacent soft-actuator/robotics lane, not motor-grade magnet qualification.
- **Conventional sintering/remanufacturing:** remains an industrial comparator when the feedstock and route are admissible.
- **Direct reuse:** remains a separate service/second-life pathway; it is not conflated with powder conversion.

The evaluator's v0.1 admission matrix is intentionally narrow. A specific route can only be added through a documented family/form/route protocol and a source-lot-bound eligibility receipt.

## Literature baseline

Rathfelder et al., *Scientific Reports* 15, 35284 (2025), DOI 10.1038/s41598-025-19111-6, investigated end-of-life NdFeB powder processed by Hydrogen Processing of Magnetic Scrap (HPMS), then powder extrusion moulding with in-situ particle alignment and sintering. The study reports:
- Br/Js alignment ratio of approximately 0.96 for its sintered anisotropic magnet;
- carbon content around 0.045 wt%;
- an oxygen-content increase of approximately 0.4 wt% from HPMS powder to sintered part, varying with the initial material.

**Interpretation boundary:** Br/Js is an alignment indicator under the study's measurement definitions; it is not absolute remanence Br. None of these values is a universal route threshold, production capability claim or Luminous measurement. The synthetic fixture uses illustrative ranges only to exercise the evaluator and labels all records SyntheticControl.

The September 2026 NdFeB AM review separately reports MEX as the strongest surveyed printed route by remanence, while LPBF density does not remove the texture problem. These are cross-study literature observations, not matched comparisons with the 2025 PEM route. A fair tournament needs same-family, same-lot or appropriately paired virgin/recovered controls and matched measurement methods.

## Evaluator boundary

The Rust evaluator checks one route trial:
- exact source lot, feedstock family and form;
- explicitly admissible family/form/route combination;
- route-eligibility receipt bound to the source lot and recipe;
- process receipt, equipment, process revision and product-lot identity;
- feedstock and added-input mass versus all declared output streams;
- unique and receipted output streams, including magnet product and required residues/rejects;
- physical assays for declared product-lot metrics, with method, calibration, uncertainty and lab method-scope lineage;
- route-specific hard limits with conservative interval decisions.

It does not authenticate issuers, verify accreditation registries, perform chemistry, measure particle distributions, calculate a magnetic hysteresis loop, operate equipment or qualify a motor component.

## Decision boundaries

- **QualifiedFeedstockRoute:** route admission, process genealogy, gross mass balance and all declared physical material assays satisfy this route's predeclared limits.
- **Fail:** a valid measured interval definitely violates a hard limit or the gross interval mass balance is definitely inconsistent.
- **Indeterminate:** missing/invalid evidence, a measurement interval crossing a limit, incomplete output streams or unqualified process evidence.
- **NotComparable:** source family/form/route, recipe, source lot, output lot or required laboratory method scope does not match.

Qualification here does not equal M4 component acceptance, M5 manufacturing capability, audited LCA, techno-economic superiority or product certification. The downstream output lot composes into the recovery passport (#7791), route cohort (#7776), maturity evaluator (#7792) and component/architecture benchmarks (#7773, #7802, #7887, #7870).

## Matched experimental programme

The first physical study should use:
1. HPMS-recovered NdFeB powder via the PEM/alignment/sintering route.
2. A virgin NdFeB feedstock control processed through the same recipe, equipment and specimen geometry.
3. Where route-eligible, a separate MEX bonded-composite stratum with matched virgin and recovered material formulations.
4. LPBF only after its own recovered-feedstock qualification; do not reuse the PEM result as proof of LPBF suitability.

Pre-register feedstock lot, recipe, process revision, specimen geometry, orientation, magnetization, measurement method and uncertainty budget. Randomize run order where practical, retain failed/reworked builds and hold out entire feedstock lots / process revisions. The first exploratory 3-build / 2-lot screen is not manufacturing qualification; it is only a feasibility gate.

## Claim ceiling

The initial software fixture is synthetic. Passing its tests validates decision semantics only. No physical recovery yield, absolute Br, Hcj, energy product, process capability, functional component acceptance, environmental gain or cost advantage is claimed.
