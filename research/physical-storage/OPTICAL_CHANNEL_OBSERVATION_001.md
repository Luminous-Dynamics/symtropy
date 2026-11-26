# Optical Channel Observation Contract — v0.1

**Programme:** OPTICAL-CHANNEL-OBSERVATION-001  
**Tracking:** [Symtropy #7534](https://github.com/Luminous-Dynamics/symtropy/issues/7534)  
**Parent physical lane:** [Symtropy #7477](https://github.com/Luminous-Dynamics/symtropy/issues/7477)  
**Consumer decoder:** [Symthaea #13712](https://github.com/Luminous-Dynamics/symthaea/issues/13712)  
**Consumer optimizer:** [Symthaea #13662](https://github.com/Luminous-Dynamics/symthaea/issues/13662)

## 1. Purpose

This contract defines the handoff from an optical write/read experiment to exact-recovery coding and spatial-layout evaluation. A physical reader does not merely emit a bit-error rate. It emits raw detector observations whose interpretation depends on material, voxel state, depth, optical train, calibration, registration and reader implementation.

The record keeps raw observation, hard symbol, optional soft information, spatial coordinates, calibration and evidence provenance distinct. Ground truth is separately referenced and unavailable to inference-time decoders/optimizers.

## 2. Evidence classes are not interchangeable

- **synthetic_fixture:** generated values for software tests only; calibration status is synthetic_only.
- **published_reproduction:** values transcribed or reproduced from an identifiable publication, with unavailable raw measurements left null.
- **physical_measurement:** instrument observations tied to a specimen, process run, reader and calibration chain.
- **model_derived:** outputs from a versioned specialist model, with model applicability and lineage documented separately.

A record's evidence_class is immutable provenance. A transform, schema validator, optimizer or benchmark cannot promote one class to another. Missing measurements remain null, not guessed.

## 3. Coordinate and identity model

Every observation is tied to a specimen and a coordinate frame. The 4×4 transform maps the specimen frame into the declared reader/measurement frame; units are micrometres. Registration uncertainty is separate from raw detector uncertainty.

Specimen identity, material batch, writer identity, reader identity and calibration ID are required for joining repeated runs. If a field is unavailable, preserve null and mark the evidence tier accordingly. Never use an arbitrary generated ID as if it identified a real physical specimen or instrument.

## 4. Raw and inferred channel data

Each observation contains:
- raw_signal: the source measurements/features and units, or null if no readable observation exists;
- read_status: readable, missing, saturated, invalid or unregistered;
- hard_symbol: the categorical decision, if any;
- soft_information: unavailable, illustrative_only, uncalibrated or calibrated.

A calibrated soft-information record must have a calibration reference, correct alphabet cardinality, finite probabilities in [0,1] and total probability mass equal to one within a declared numeric tolerance. JSON Schema can bound each number but cannot enforce vector normalization or prove probability calibration. A calibrated claim therefore requires a separate semantic validator and calibration evidence.

The synthetic fixture intentionally uses illustrative_only probabilities with a synthetic_only reader calibration status. It must never be consumed as a physically calibrated likelihood distribution.

## 5. Ground-truth isolation and leakage resistance

Expected symbols and original payload labels are not embedded in the observation rows. They live in a separately referenced access-controlled label corpus, or are absent. Partitioning is by specimen/material batch/writer/reader grouping, not a random split of neighbouring voxels from the same physical object.

A held-out evaluation partition must be frozen before optimizer tuning. Training and calibration material cannot be reclassified as held-out after performance is inspected. The decoder receives raw observations and authorized soft information, never the evaluation labels.

## 6. Required semantic checks

In addition to Draft 2020-12 schema validation:
1. Observation IDs are unique within a dataset.
2. Every hard symbol belongs to the dataset symbol alphabet.
3. Every non-empty soft vector contains exactly one entry per alphabet symbol, with no duplicates.
4. Probabilities are finite, in [0,1], and sum to 1 within a predeclared tolerance.
5. A calibrated vector references a current, traceable calibration record and compatible reader/configuration.
6. Synthetic evidence has synthetic_only calibration status and cannot be labelled physically calibrated.
7. Physical measurements have raw-data digests, specimen/process/reader lineage and a calibration state; missing calibration makes the observation unqualified for calibrated-likelihood claims, not invalid as raw evidence.
8. All coordinates lie within known specimen bounds, and the transform/registration identity is consistent across the batch.
9. Raw data, instrument logs, calibration and any evaluation-label corpus have independent digests and access control.
10. Training, calibration and held-out split groups do not overlap under the declared grouping key.

## 7. Symthaea adapter contract

The adapter maps each observation to a deterministic channel symbol record containing:
- physical coordinate and registration uncertainty;
- hard symbol or explicit erasure;
- optional calibrated likelihood vector with calibration reference;
- raw-observation digest and specimen/reader lineage;
- evidence class and held-out split identity.

It must not convert raw signal magnitude, a classifier score or an uncalibrated confidence into a physical symbol-error probability. When calibration is absent, hard-decision sensitivity tests may be run, but probability-weighted optimization remains unavailable.

## 8. Benchmark progression

Before physical measurements exist, validate only parsing, semantic invariants, provenance separation and negative controls. After partner-lab data become available, evaluate:
- per-symbol confusion matrix by material, depth, batch and reader;
- raw symbol/bit error and erasure rates with sample counts and confidence bounds;
- spatial autocorrelation, burst geometry and layer-to-layer registration drift;
- probability calibration (reliability diagram, Brier score/ECE where sample size permits);
- exact payload recovery using the same payload, code and held-out defect map;
- reader-to-reader calibration transfer and independent implementation agreement.

Do not infer extremely low archive failure probabilities from a small voxel dataset. Keep the spatial fault masks as deterministic coverage tests until repeated physical observations support a justified distribution.

## 9. Evidence boundary

The accompanying JSON fixture is synthetic and illustrates structure only. It does not report a physical BOROFLOAT 33 or K10 experiment, measured confusion matrix, laser configuration, material yield, retention result or device performance. The contract creates an auditable path for future observations; it is not an optical simulator or laboratory instrument.


## 10. Schema-enforced read-state and calibration conditions

The schema now also rejects two classes of contradictory record:
- a `readable` observation must carry a non-null raw-signal object;
- a `missing` observation must have a null raw signal and hard symbol, and an unavailable empty soft-information vector.

If any observation is marked `calibrated`, the batch reader must be marked `current_traceable`. Each such observation also requires a calibration reference. These checks prevent common metadata contradictions; semantic validation is still required to prove calibration compatibility, unique observation IDs, alphabet coverage and normalized probability mass.
