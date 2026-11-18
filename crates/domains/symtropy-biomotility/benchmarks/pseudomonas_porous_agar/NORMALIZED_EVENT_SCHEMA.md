# Normalized event record contract

Status: proposed adapter contract. The Zenodo files have not yet been mapped or admitted.

The published analysis uses run-segment displacement vectors to characterize run orientation. The simulation kernel preserves additional state observables so causal policy inputs, realized run directions and instantaneous tangents are never conflated.

## Record grain

One normalized record represents one observed interruption between two tracked run segments for one bacterium. It binds the preceding run, the event, and the following run. This allows event i to be paired with the outcome of run i+1 without losing track/recording/replicate ancestry.

## Required identity and provenance

| Field | Meaning |
|---|---|
| source_dataset_doi | Persistent identifier of the dataset |
| source_record_version | Exact Zenodo record/version identifier |
| source_file_name | Original path/key within the deposit |
| source_file_sha256 | Digest of the exact source file |
| source_license_id | File-level licence identifier, or explicit unknown |
| source_row_or_event_id | Stable source row/event locator |
| condition_id | Organism/strain/agar/gradient condition |
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
| preceding_run_duration_s | s | Time between detected run start/end |
| preceding_run_start_heading_rad | rad | Initial orientation at the start of the preceding run |
| preceding_run_displacement_heading_rad | rad | Net displacement orientation from first to last detected position; this is the published run-orientation observable |
| preceding_run_displacement_um | µm | Magnitude of the preceding run displacement |
| preceding_run_end_tangent_heading_rad | rad | Instantaneous estimated tangent immediately before the event, if supported by frame cadence |
| event_duration_s | s | Duration of the interruption |
| event_turn_angle_rad | rad | Signed angle from preceding-run displacement heading to following-run displacement heading, using a declared wrap convention |
| following_run_displacement_heading_rad | rad | Net displacement orientation of the following run |
| source_event_label | string/enum | Original source label if present; preserve exactly |
| derived_event_class | enum | Derived short/long class under a versioned threshold policy |
| event_class_threshold_s | s | Threshold versioned with the transformation |
| threshold_boundary_policy | enum | SHORT_INCLUSIVE, LONG_INCLUSIVE, or EXCLUDE_EXACT_BOUNDARY |
| time_unit_source | string | Original time-unit declaration |
| coordinate_frame | string | Source image/world frame and handedness |

Do not substitute the endpoint tangent for the net run-displacement vector. The simulator stores both: run-start heading is the causal input for run-time sampling; realized net displacement direction conditions the short-event turn-angle distribution; instantaneous end tangent is retained as a separate observation.

## Direction partition

Use a source-pinned direction rule. The implementation's current working partition is the sign of the projection of the heading/displacement vector onto the gradient axis:
- UPGRADIENT when the projection is positive beyond the numerical deadband;
- DOWNGRADIENT when it is negative beyond the numerical deadband;
- CROSSGRADIENT only within the declared numerical deadband around zero.

The current implementation uses a dimensionless numerical tolerance of 1e-12 for the dot-product projection. This is not a biological angular cone. The precise rule used by the paper's data-analysis code/supplement remains to be confirmed; encode the source rule in the final data-transform revision and run partition sensitivity if it differs.

## Event threshold and boundary policy

The papers use overlapping descriptions for the exact 1.0-second boundary. The 2026 paper's Figure 3/4 grouping describes short events as <=1 s and long events as >1 s, while other prose uses <1 s; the 2025 paper also describes a strict <1 s / >1 s split.

Therefore every analysis must declare one of:
- SHORT_INCLUSIVE: short <= 1 s; long > 1 s;
- LONG_INCLUSIVE: short < 1 s; long >= 1 s;
- EXCLUDE_EXACT_BOUNDARY: events exactly 1 s are retained but excluded from the two-class contrast.

The default code fixture uses SHORT_INCLUSIVE to match the 2026 Figure 3/4 captions. Report sensitivity to the other policies; do not relabel the operational class as ground-truth biological causation.

## Directional variables and causal interpretation

Preserve these separately:
1. Run-start heading: current simulation input to the run-time policy. This is a causal policy state.
2. Realized preceding-run displacement heading: measured first-to-last trajectory vector. This is the published analysis observable and conditions the next short-event turn-angle distribution.
3. Run-end tangent: instantaneous final heading estimate, where temporal resolution supports it.
4. Following-run displacement heading: measured post-event outcome.

The published analysis groups run times by realized run orientations. The initial reference simulator uses run-start heading for run-time sampling, which is a declared modelling approximation. The benchmark must retain both and quantify their discrepancy; never silently claim they are identical.

## Missingness and exclusions

Represent missing, censored, truncated, low-confidence, filtered and invalid values explicitly. Never replace them with zero. Each exclusion must include:
- a stable record locator;
- the exact filter predicate/version;
- the reason and number of affected events;
- whether exclusion occurred in the source analysis or our adapter.

## Experimental split rules

Keep all events from one trajectory in one split. Prefer splits grouped by biological replicate, plate/day or independent sample preparation. The 2026 study reports 10 one-minute recordings from different plates across two days per agar condition; the track count must not be treated as the independent biological replicate count. Retain recording/plate/day ancestry so uncertainty and held-out tests operate at the right level.

## Normalization receipt

A data-ingestion run must emit:
- DOI and exact record/version;
- file name, byte size and SHA-256 for each ingested file;
- file-level licence and access status;
- parser revision and source-to-normalized column mapping;
- row/event counts before and after each filter;
- count of missing/invalid/censored observations;
- units and coordinate-frame declarations;
- replicate, recording and track counts by condition;
- exact train/validation/test grouping;
- output artifact digests.

A successful parse is not a scientific reproduction. The receipt establishes provenance and transformation traceability only.
