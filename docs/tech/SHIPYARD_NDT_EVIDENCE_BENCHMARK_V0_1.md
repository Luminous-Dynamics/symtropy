# Shipyard NDT Evidence Qualification Benchmark v0.1

**Status:** synthetic evidence contract and adversarial corpus. Not an NDT procedure, inspection instruction, acceptance table, defect classifier, probability-of-detection qualification, weld-release authority or class approval.

## Ownership and boundaries

Parent: [SHIPYARD-WELD-QUALITY-LOOP-001 #4443](https://github.com/Luminous-Dynamics/symtropy/issues/4443).

- [SHIPYARD-WELD-TWIN-001 #4433](https://github.com/Luminous-Dynamics/symtropy/issues/4433) owns workpiece/equipment/process pass execution and process-trace lineage.
- [COMPONENT-DIGITAL-THREAD-001 #3331](https://github.com/Luminous-Dynamics/symtropy/issues/3331) owns component/joint identity, canonical inspection/NDT records, repair and as-built qualification.
- [SHIPBUILD-CO-DESIGN-001 #2048](https://github.com/Luminous-Dynamics/symtropy/issues/2048) owns quality-adjusted production planning and accepted-output economics.
- [MARITIME-ASSET-TWIN-COMPOSITION-001 #4406](https://github.com/Luminous-Dynamics/symtropy/issues/4406) owns federated PLM/MES/QMS/EAM composition.
- Mycelix remains owner of inspector/provider authority, custody, approval and revocation.
  
This benchmark owns a **method-specific NDT evidence envelope** and a frozen synthetic adversarial corpus. It must not become an NDT system of record or decide whether a real weld is acceptable.

## Standards and method-specific maturity

The product must represent a standard's exact identity, edition, maturity and effective date. "Newest listed" does not imply "current and applicable."

### Automated phased-array ultrasonic testing

**ISO 13588:2019, Edition 2** is the current published standard for semi- or fully automated phased-array ultrasonic testing (PAUT) of fusion-welded joints. ISO says it was last reviewed and confirmed in 2024. Its declared scope includes:
- metallic materials, minimum thickness 6 mm;
- full-penetration welds of simple geometry in plates, pipes and vessels;
- low-alloy and/or fine-grained steel weld and parent material;
- four testing levels corresponding to different probability-of-detection levels;
- guidance on detection, location, sizing and characterization.

It does **not** specify discontinuity acceptance levels. Those must be separate, valid project/class/product-code criteria from the responsible authority. For coarse-grained or austenitic steels, ISO 22825 also applies. It is not a general standard for every hull geometry, every alloy, every weld access condition or steel-product automated testing covered by ISO 10893-8/-11 or ISO 3183.

Sources:
- ISO 13588:2019 status and scope: https://www.iso.org/standard/72747.html
- ISO Online Browsing Platform: https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3A13588%3Aed-2%3Av1%3Aen

### PAUT successor maturity

**ISO/DIS 13588, Edition 3** is under development. The ISO lifecycle records CD registered 13 February 2026, CD approved for DIS registration 19 October 2026, DIS registered 27 October 2026, and the DIS enquiry ballot initiated. It is not yet a published/effective International Standard and must not be selected as current normative basis before publication/adoption by the applicable authority.

Source: https://www.iso.org/standard/90920.html

### Manual ultrasonic testing is a different scope

**ISO 17640:2018, Edition 4** is the current published manual UT standard. Its public scope is manual ultrasonic testing of fusion-welded metallic joints >=8 mm with low attenuation, object temperature 0–60 °C, primarily full-penetration ferritic joints. Testing level D is for special applications where specified; that does not make ISO 17640 a blanket substitute for the automated PAUT method standard.

Source: https://www.iso.org/standard/75737.html

### General method selection and acceptance separation

**ISO 17635:2025** gives general rules for NDT method selection and evaluation of welds based on quality requirements, material, thickness, welding process and extent of testing. It explicitly cautions that NDT acceptance levels are not directly interpretable as ISO 5817 / ISO 10042 quality levels; any relation is general and technique-dependent.

Source: https://www.iso.org/standard/85705.html

Other modality-specific comparators:
- ISO 23277:2015 for penetrant-testing acceptance levels: https://www.iso.org/standard/62315.html
- ISO 17636-1:2022 for X-/gamma-ray techniques using industrial radiographic film; do not assume film scope automatically covers digital radiography/CR: https://www.iso.org/standard/78319.html
- ISO 22825 for ultrasonic testing of austenitic/coarse-grained welds (scope must be pinned from the actual standard before implementation).

### What the benchmark is and is not

A method standard defines how a method is applied within a declared scope; a testing level/POD category is not a discontinuity acceptance level; a project/class acceptance basis is a separate authority-owned input; a qualified examiner makes the method-based interpretation; final release is another owner-authorized decision. The evaluator only checks the structure and declared applicability of synthetic records. It does not perform NDT or acceptance evaluation.

## Paradigm shift: from "inspection = accepted" to a traceable measurement and authority chain

Represent separate evidence stages:

1. **Method selection:** declared material, thickness, weld/process geometry, inspection purpose and method standard.
2. **Technique / coverage planning:** testing level, scan/exposure plan, volume/surface coverage and known access limitations.
3. **Acquisition:** exact equipment/probe/detector, software, calibration/reference checks, raw data, timestamps and subject identity.
4. **Processing:** processing/beamforming/image enhancement revision, derived-data digest and parent raw-data digest.
5. **Indication characterization:** examiner-reported location/extent/orientation/uncertainty, without automatic acceptance.
6. **Evaluation:** separate acceptance basis, edition, criterion reference, issuer and qualified disposition.
7. **Repair/reinspection:** exact repair route, region, process revision and a new inspection record.
8. **Release:** responsible owner/class/flag/contract authority, separate from model or instrument status.

A classifier output is an indication candidate, not a code-based evaluation or acceptance disposition. More detections can mean better inspection rather than worse manufacturing; fewer detections can mean missed defects or reduced coverage rather than better weld quality.

## Synthetic input contract

Every record binds:
- asset/component/serial/parent assembly/joint/weld region and exact workpiece revision;
- material, thickness, penetration, geometry, parent/weld material, temperature and inspection context;
- method family, technique, manual/semi/fully automated mode, method-standard identity/revision/maturity/effective time;
- method applicability statement and in-scope/out-of-scope result;
- testing level, scan/inspection plan, actual inspected volume/area, coverage and access limitations;
- equipment, probe/transducer/wedge/focal laws or radiation source/detector/film, software and calibration;
- inspector/examiner identity, qualification reference/scope/validity and responsible issuer;
- raw acquisition artifact path/digest, source observation time, acquisition/retrieval time, processing and derived artifact digest/lineage;
- synthetic indication identity and reported location/extent/orientation/characterization; no actual acceptance inferred from a model score;
- separate project/class acceptance-basis reference and authorized decision issuer;
- data grouping: physical specimen/weld group, production lot, campaign, equipment, method revision and held-out split.

No real NDT threshold, WPS/PQR, acceptance table, or physical signal is included. The only raw acquisition fixture is a tiny synthetic metadata manifest and explicitly contains no NDT signal.

## Frozen corpus v0.1

The corpus contains **57 unique cases**: one contract-valid synthetic PAUT record and 56 adversarial controls. They cover:
- PAUT method/standard pairing and scope;
- minimum thickness, full-penetration, geometry, steel family and exclusions;
- future draft versus current published standard;
- testing level versus acceptance-level conflation;
- ISO 5817 quality-level versus NDT acceptance-basis conflation;
- manual UT, PT and film radiography method mismatches;
- coverage, scan plan, calibration, equipment and qualification;
- raw acquisition digest, path, subject binding, processing lineage and temporal cutoff;
- authority scope, revoked/expired approval, missing acceptance basis;
- leakage across adjacent scans of a single physical specimen/weld;
- model score substituted for qualified examiner disposition;
- synthetic fixture promoted to physical observation or release.

The corpus is a specification fixture. It is not yet a Rust verifier or a physical POD/accuracy benchmark.

## Benchmarks and acceptance targets

### Evidence-contract correctness
- 100% independent-oracle agreement on frozen synthetic cases.
- Zero method/standard scope false promotion.
- Zero draft-to-current or future-effective standard promotion.
- Zero use of a testing-level/POD category as an acceptance criterion.
- Zero model-score or digest promotion to qualified examiner/issuer authority.
- Exact raw and derived artifact hash/lineage verification.

### Physical NDT performance (future, only with lawful qualified data)
- probability-of-detection curves with confidence bounds, stratified by method, defect family, size, orientation, depth, material and geometry;
- false calls per inspected weld length/area/volume;
- sensitivity at fixed false-call operating points and missed-critical-indication rates;
- indication location, sizing and orientation error against qualified reference specimens;
- calibration and uncertainty interval coverage;
- inter-/intra-reader agreement and blind examiner comparison;
- out-of-domain/abstention rate and failure to meet coverage/calibration prerequisites.

Do not declare a universal POD threshold here. The acceptance/qualification target comes from the governing application risk, contract/code and responsible authority.

### Split protocol
- Split by physical weld/specimen, workpiece, block, batch, production campaign, material lot, procedure revision, scanner/equipment and facility where identifiers permit.
- Never randomly split neighbouring A-scans, FMC frames, S-scans or image patches from the same physical weld into train/test and report that as unseen-weld generalization.
- Keep PAUT, manual UT, PT, film RT, digital RT/CR, radiographs and surface photos as distinct modality/task strata.
- Report raw acquisition and derived-processing lineage with every prediction and disposition.

### Industry comparison
Compare on the same modality, reference specimens, defect family, acquisition protocol, calibration, coverage, examiner labels and held-out physical subjects. Keep indication detection, characterization, code/project evaluation, inspector disposition, root-cause feedback and final release as separate claims. A higher mAP on a different image split is not an NDT qualification.

## Claim ceiling

A passing synthetic corpus establishes only the structure and expected handling of synthetic NDT evidence, method/standard applicability metadata and authority separation. It does not establish detection probability, method qualification, inspector competence, flaw truth, weld acceptance, repair adequacy, class/flag approval or authority to release a vessel/structure.
