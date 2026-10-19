# Animal Observation Reference Packs V0

Status: normative research/data contract. Documentation only. This defines how evidence may qualify a model claim; it does not pre-approve datasets, grant usage rights, or assert that any animal simulation is scientifically validated.

## Purpose

Build an evidence bridge between observed animals and Symtropy's causal living-world simulation without allowing a dataset's size, visual quality, accessibility, or citation to overstate what it measures.

Related work:
- [Animal Kingdom Coverage and Cross-Taxon Qualification V0 — PR #2120](https://github.com/Luminous-Dynamics/symtropy/pull/2120)
- [Evidence-aware animal capability core — PR #2127](https://github.com/Luminous-Dynamics/symtropy/pull/2127)
- [ANIMAL-OBSERVATION-001 — issue #2128](https://github.com/Luminous-Dynamics/symtropy/issues/2128)

## Paradigm shift: every claim carries a measurement contract

Replace "this dataset contains animals, so it validates the animal model" with:

`source and rights -> observation manifest -> measurement semantics -> leakage-resistant split -> replayable comparison -> narrow, explicit claim`

Each reference pack must declare both its **measurement envelope** (what the source directly observes) and **claim ceiling** (what a result cannot establish). A dataset can qualify one modality or metric without validating a species model as a whole.

Examples:

| Reference modality | Claims it may support after review | Claims it cannot establish by itself |
|---|---|---|
| 2D annotated keypoints | Projection-space pose/keypoint error and some shape/pose transfer tests | 3D forces, torque, muscle activation, physiology, causal behavior or ecological fitness |
| 2D/3D motion capture | Declared coordinates, tracked trajectories, kinematic derivatives within sampling/calibration limits | Causal controller correctness, unmeasured forces, metabolism or universal species behavior |
| Sensorimotor experiment | Task outcome and controlled response under the described setup | Cross-species transfer or unrestricted cognition |
| Telemetry/migration tracks | Space-time movement statistics in a declared study/context | Fine-grained locomotor mechanics, motivations, or unmeasured physiological state |
| Population/field observations | Declared abundance, encounter, phenology, interaction or habitat-change metrics | Individual-level causal mechanisms absent supporting design/evidence |
| Audio/chemical/acoustic signals | Signal detection or propagation outcomes under declared recording conditions | A receiver's percept or action unless the response is also observed or experimentally controlled |

Visual similarity is not a measurement of physiology. A behavior video is not automatically a force plate. A movement track does not reveal why an animal moved.

## Source candidates and their limits

These are candidate benchmark sources, not automatically usable dependencies. Verify the exact data release, terms, attribution obligations, and proposed operation (local analysis, transformation, training, redistribution, publication) before accepting a source.

### AP-10K

Official project: https://github.com/AlexTheBad/AP-10K

The published AP-10K benchmark reports 10,015 images with annotated keypoints spanning 54 species and 23 animal families. It is a useful pose-estimation and visible-morphology reference. An AP-10K adapter must keep image/keypoint coordinates, visibility/occlusion flags, image dimensions, annotation convention and source split explicit. It must not promote a still-image result into a claim about dynamic gait, 3D contact forces, energetic cost, internal physiology or cognition.

The repository's code license and the dataset/annotation/image rights must be checked independently; a public GitHub repository or paper citation does not by itself establish all downstream rights.

### NeuroMechFly v2 / FlyGym

Paper: https://www.nature.com/articles/s41592-024-02497-y
Current project: https://github.com/NeLy-EPFL/flygym

This is a valuable narrow reference for adult Drosophila sensorimotor control, biomechanics, sensory inputs and motion/behavior under documented experimental conditions. It cannot establish a generic arthropod brain, universal insect performance, or cross-taxon validity. Version the exact release/API, task, morphology/model parameters, experimental reference, and dependency stack; do not silently combine records from incompatible versions. The project's 2.x rewrite is a breaking interface boundary and requires an explicit version pin.

Rust production code may consume licensed, normalized reference fixtures/manifests; it must not make the research project's Python API a production dependency merely because the reference system uses Python.

### Movebank Data Repository

Official repository: https://www.movebank.org/cms/movebank-content/data-repository
Official policy: https://www.movebank.org/cms/movebank-content/data-policy

Movebank can support movement ecology and animal-location replay within the chosen study's permitted scope. Study metadata, taxon assignment, tag identity, observer/deployment semantics and track quality are part of the evidence. Animal/tag identifiers should not be assumed globally unique across studies. Public visibility is not blanket permission for every operation. Avoid disclosing precise locations for sensitive or at-risk animals; location granularity may need to be degraded or data kept private according to study and conservation requirements.

Counts and catalogue contents change over time; any aggregate statement must be bound to a dated metadata snapshot rather than treated as a timeless property of the service.

## V0 manifest contract

A normalized reference-pack manifest shall be a versioned, deterministic record with at least the fields below. The canonical schema should use typed Rust structures and explicit serialized enums; schema upgrades are versioned migrations, not silent reinterpretations.

### Identity and integrity

- `schema_version`, stable `pack_id`, `source_id`, dataset/release/DOI or a stable source reference, retrieval timestamp and declared snapshot/version.
- Byte digest for every ingested artifact when retaining/hashing those bytes is permitted. If byte access or retention is forbidden, bind the permitted immutable source/version reference and record that a byte-level digest was not available; never fabricate one.
- Source schema/version, adapter version, parser version, and canonicalization version.
- Explicit source citations and required attribution text.
- Subject identifiers scoped to study/deployment, not assumed globally unique.
- Taxon identifier, taxonomy source and release, mapping provenance and unresolved alias/taxon ambiguity.

### Measurement semantics

- Modality and declared observable fields.
- Units, coordinate reference frame, axis order and handedness where spatial; timestamps, time zones/epoch, sampling rate and interpolation policy where temporal; image size/crop/projection and keypoint convention where visual.
- Instrument, method, calibration, experimental/task setup, environment/context and relevant conditions when reported.
- Sampling/selection design, missingness, exclusion rules, censoring, quality flags and uncertainty.
- Evidence class per field or observation: `measured`, `reconstructed`, `modeled`, or `synthetic`. These labels are not interchangeable.
- Transformations and assumptions as explicit, ordered operations with version, units in/out, parameters and digest. No implicit imputation, resampling, coordinate conversion, smoothing, filtering, extrapolation or unit conversion.

### Rights, privacy and permitted use

Represent permissions per intended operation rather than a generic boolean:
- inspect metadata;
- retrieve/access payload;
- perform local analysis;
- transform/derive a feature;
- redistribute original payload;
- redistribute derivatives;
- train or evaluate a model;
- publish metrics/figures;
- publish granular spatial/subject metadata.

Each permission must have a scoped decision, source/authority, date/version, conditions and any expiry/revocation state. At minimum distinguish `granted`, `attribution_required`, `restricted`, `unknown`, and `revoked`. Unknown is not grant. If required scope/authority is missing, the affected operation is blocked or marked indeterminate; do not infer it from a visible URL or general code license.

Store sensitive location/subject fields only as necessary and as permitted. Each export declares its spatial/temporal resolution ceiling and redaction/aggregation operations. If the required safe resolution cannot preserve the intended scientific metric, abstain rather than weaken privacy.

Source lineage and permission decisions should consume the system-of-record/provenance receipts from the proper Luminous/Mycelix owner; this project should not create a second authoritative rights registry.

### Claim envelope

Every pack must list:
- supported validation tasks and the exact output metrics;
- required source fields and their measurement semantics;
- permitted transfer set: species/taxon IDs, life stages, study/site/conditions and the evidence supporting that transfer;
- prohibited or unsupported claims;
- known bias, missingness and uncertainty;
- the expected observation-to-model comparison procedure;
- claim ceiling and reviewer/evidence status.

A validation for one taxon, stage or condition does not silently qualify another. If the target differs or its match is unknown, the result must report the mismatch/indeterminate status.

## Leakage-resistant split contract

Split assignment is a first-class, digest-bound artifact—not a convenience random split generated after seeing scores.

Use the split unit appropriate to the claim, often grouped by some combination of:
- individual/subject and tag identity;
- recording/session/track;
- study, site, colony, family or instrument;
- contiguous time window and environmental episode;
- near-duplicate image, video frame, augmentation lineage or repeated publication of the same data.

A split is invalid if a known related/duplicate subject or leakage group crosses train and test for a claim meant to generalize to new subjects, studies, sites, or time. Define the unit of generalization before selecting the split. Where records cannot be deduplicated/linked confidently, record uncertainty and avoid claiming strict independence.

Keep a final test set sealed from parameter selection. Report validation/selection, calibration, and final evaluation distinctly. A successful random frame split may establish interpolation to familiar videos, not generalization to new animals.

## Replay and comparison requirements

A replay receipt binds:
- reference-pack manifest and allowed-input digest;
- taxonomy source/release and any mapping digest;
- leakage-proof split manifest digest;
- simulator code HEAD and configuration;
- runtime/toolchain/platform and determinism settings;
- adapter/transform digests;
- task protocol, reference metric, uncertainty method and claim ceiling;
- per-group results and failures, not only a pooled score.

The evaluator should be able to reproduce expected result ordering from the same immutable inputs. CPU/GPU numeric deviations may use declared tolerances for the named continuous outputs; identity, permission, sample membership, unit/frame metadata, case disposition and evidence lineage invariants remain exact.

## Synthetic hostile benchmark before real datasets

Freeze at least 40 independently specified manifest/split cases. Initial cases must be entirely synthetic and must not look like real measurements. Include:
1. complete valid manifest;
2. absent schema/version;
3. malformed/duplicate pack ID;
4. invalid/stale/revoked source release;
5. missing permission;
6. public source but no permission for training;
7. attribution required but empty;
8. required operation outside granted scope;
9. missing byte digest where digest is required and permitted;
10. byte digest mismatches snapshot;
11. taxonomy alias resolves differently under another release;
12. taxonomy source/release mismatch;
13. unknown taxon mapping;
14. synthetic observation mislabeled measured;
15. modeled estimate mislabeled measured;
16. unit mismatch;
17. coordinate-frame/axis-order mismatch;
18. timestamp/timezone mismatch;
19. incompatible sampling rate;
20. missing calibration;
21. undocumented interpolation;
22. undocumented imputation;
23. duplicate observations with different filenames;
24. same subject in train/test;
25. same track split across train/test;
26. study/site leakage;
27. temporally adjacent data leak for new-time prediction;
28. tag collision between studies;
29. alias subjects that cannot safely be merged;
30. high-resolution at-risk animal location in export;
31. export finer than declared spatial resolution ceiling;
32. AP-10K-like still keypoints promoted to 3D dynamics;
33. static morphology promoted to physiological inference;
34. one fruit-fly task score promoted to other species;
35. incompatible FlyGym/API/release pair;
36. Movebank location series lacking required identity/deployment semantics;
37. incomplete/biased sampling silently treated as population truth;
38. observation order changes canonical digest;
39. forbidden artifact content hidden in an otherwise permitted pack;
40. transformed derivative lacks source lineage;
41. claim uses life stage not present in held-out split;
42. target animal/site differs from validated transfer envelope;
43. metric result omitted when a subset fails;
44. all sources unusable and evaluator must return `incomplete`/abstain.

Each case has an independently written expected disposition and reason code. Do not generate expected values by calling the implementation under test.

## Benchmarks and go/no-go gates

| Dimension | Frozen target | Empirical evaluation target |
|---|---|---|
| Rights/scope hard gates | 0 false accepts on hostile corpus | Audit each permitted operation and source snapshot |
| Schema/case oracle | 100% known-answer agreement | Not a biological accuracy metric |
| Leakage | 0 known prohibited split leaks accepted | Report grouping assumptions and uncertainty |
| Provenance | 0 silent class promotion | Field-level lineage for every compared output |
| Units/transforms | 0 undocumented conversions/imputation | Quantify transform error where a reference exists |
| Determinism | Same immutable inputs -> same canonical receipt | Exact receipt comparison; tolerances only for declared numerics |
| Reference-model error | No universal fixed threshold | Preregister threshold by task/measurement resolution before held-out evaluation |
| Calibration | No fabricated confidence when data are insufficient | Report interval coverage, error, missingness sensitivity and abstention |
| Generalization | Unsupported target mismatch is never marked validated | Separate metrics by species, stage, subject, study/site and condition |
| Privacy | 0 prohibited precise-location disclosures in frozen corpus | Verify location aggregation and access/export controls |
| Reproducibility | Exact input/code/config digest recorded | Independent replay on a second environment when feasible |

Do not optimize pooled performance while silently hiding failing taxa or conditions. Publish per-stratum sample counts and uncertainty. Missing evidence should increase abstention or lower the claim ceiling; it should not be "fixed" by weakening a hard gate.

## Staged implementation

1. Add a source-neutral V0 manifest schema and an independent 40+ case synthetic oracle.
2. Implement typed Rust parsing, normalization receipts, use-scope gates and split-integrity checks in a focused crate.
3. Add metadata-only adapters for AP-10K, FlyGym/NeuroMechFly and Movebank; do not fetch or redistribute payloads in this step.
4. Record exact release, licence/permission review and allowed operations before ingesting each source's payload.
5. Build a narrow replay task for one measurement modality at a time, then evaluate held-out groups.
6. Connect results to Symtropy physiology/behavior/physics only when the observation supports those claims.

## Evidence states

Documentation/research reviewed, metadata parsed, permission-reviewed, fixture-oracle pass, source-specific replay pass, and empirical validation pass are different states. A queued/skipped/stale/wrong-head workflow is not a pass. A successful schema validator does not prove a data licence; a licence review does not prove scientific accuracy; a pose benchmark does not prove causal animal behavior.
