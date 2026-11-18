# Normalized event record contract

Status: proposed adapter contract, not a claim that the Zenodo files have been mapped.

The published trajectory deposit must be inspected before writing a file parser. Do not infer column names, event labels, units or experimental-unit identifiers from directory names or paper text.

## Record grain

One normalized record represents one observed interruption between two detected run segments for one tracked bacterium.

A record must link back to the original source file and trajectory. It must not discard the parent run or following run context needed to reproduce directional conditioning.

## Required identity and provenance

| Field | Meaning |
|---|---|
| source_dataset_doi | Persistent identifier of the dataset |
| source_record_version | Exact Zenodo record/version identifier |
| source_file_name | Original path/key within the deposit |
| source_file_sha256 | Digest of the exact source file |
| source_license_id | File-level licence identifier, or explicit unknown |
| source_row_or_event_id | Stable source row/event locator |
| condition_id | Declared organism/strain/agar/gradient condition |
| biological_replicate_id | Plate/day/independent preparation ID where available |
| recording_id | Recording / video / plate observation ID |
| trajectory_id | Track identity within the recording |
| data_transform_id | Exact parser/event-detection/mapping revision |

## Required measurement semantics

| Field | Units / type | Notes |
|---|---|---|
| agar_percent | percent w/v | Keep source representation and normalized numeric value |
| frame_rate_hz | Hz | Source cadence, not guessed from nominal camera settings |
| gradient_heading_rad | rad | Direction of increasing attractant in the declared image/world frame |
| previous_run_duration_s | s | Time between detected run start/end |
| previous_heading_rad | rad | Heading of preceding run in the declared frame |
| event_duration_s | s | Duration of the interruption |
| turn_angle_rad | rad | Signed change in heading using a declared wrap convention |
| following_heading_rad | rad | Heading of the following run |
| event_label_source | string/enum | Original source label if present; preserve exactly |
| event_class_derived | enum | Derived short/long class under a versioned threshold |
| event_class_threshold_s | s | Threshold versioned with the transformation |
| time_unit_source | string | Original time-unit declaration |
| coordinate_frame | string | Source image/world frame and handedness |

The derived direction classes use the paper's declared 36-degree cones only if the exact analysis definition is confirmed from the article/supplement. Record the cone angle and classifier revision in each analysis manifest.

## Derived classes

The initial convention uses:
- SHORT_REORIENTATION when event duration <= 1.0 s;
- LONG_MECHANICAL_TRAP when event duration > 1.0 s.

These are operational analysis bins, not ground-truth causal labels. Preserve source labels and allow threshold sensitivity analyses (for example 0.75, 1.0 and 1.25 seconds) without overwriting the original record.

Direction is derived separately for:
- the preceding run;
- the following run;
- the gradient frame.

Do not infer active-motor causation from duration alone. That requires corroborating imaging or source-specific experimental evidence.

## Missingness and exclusions

Represent missing, censored, truncated, low-confidence, filtered and invalid values explicitly. Never replace them with zero. Each exclusion must include:
- a stable record locator;
- the exact filter predicate/version;
- the reason and number of affected events;
- whether exclusion occurred in the source analysis or our adapter.

## Experimental split rules

Keep all events from one trajectory in one split. Prefer splits grouped by biological replicate, plate/day or independent sample preparation. The recording/trajectory hierarchy must be retained so confidence intervals can be computed at the independent experimental unit rather than treating each frame/event as independent.

## Normalization receipt

A data-ingestion run must emit:
- DOI and exact record/version;
- file name, byte size and SHA-256 for each ingested file;
- file-level licence and access status;
- parser revision and source-to-normalized column mapping;
- row/event counts before and after each filter;
- count of missing/invalid/censored observations;
- units and coordinate-frame declarations;
- replicate and track counts by condition;
- exact train/validation/test grouping;
- output artifact digests.

A successful parse is not a scientific reproduction. The receipt establishes provenance and transformation traceability only.
