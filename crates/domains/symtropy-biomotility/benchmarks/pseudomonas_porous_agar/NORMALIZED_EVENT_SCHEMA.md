# Normalized event record contract

Status: proposed adapter contract. The Zenodo files have not yet been mapped or admitted.

The contract separates:
1. physical mechanism (active motor, passive trap, unresolved);
2. observed event duration and source-pinned duration bin;
3. evidence status and method used to infer the mechanism.

The three are independent. Duration must never be used as a causal label.

## Record grain

One normalized record represents one observed interruption between two tracked run segments for one bacterium. It binds the preceding run, event, following run, track, recording and biological replicate. This permits event i to be paired with the measured outcome of run i+1 without losing the ancestry needed for grouped validation.

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

## Required trajectory and measurement semantics

| Field | Units / type | Notes |
|---|---|---|
| agar_percent | percent w/v | Preserve source and normalized values |
| frame_rate_hz | Hz | Source cadence, not guessed from nominal camera settings |
| gradient_heading_rad | rad | Direction of increasing attractant in the declared image/world frame |
| preceding_run_duration_s | s | Detected run start/end duration |
| preceding_run_start_heading_rad | rad | Initial orientation at run start |
| preceding_run_displacement_heading_rad | rad | First-to-last displacement vector; published run-orientation observable |
| preceding_run_displacement_um | µm | Magnitude of that vector |
| preceding_run_end_tangent_heading_rad | rad | Instantaneous estimated tangent at event onset, if temporal cadence supports it |
| event_duration_s | s | Observed interruption duration |
| event_turn_angle_rad | rad | Signed angle between preceding- and following-run displacement headings |
| following_run_displacement_heading_rad | rad | First-to-last displacement vector of the following run |
| mean_event_speed_um_s | µm/s | Mean tracked speed during the event, if available |
| event_direction_change_rad | rad | Direction change feature used by source segmentation, if available |

Do not substitute the endpoint tangent for the net run-displacement vector. Keep run-start heading (causal model input), realized displacement direction (published analysis observable), incoming tangent (physical state estimate), and following-run displacement direction separate.

## Physical mechanism (independent of duration)

| Field | Values / meaning |
|---|---|
| event_mechanism | ACTIVE_MOTOR_EVENT, PASSIVE_MECHANICAL_TRAP, UNRESOLVED |
| mechanism_evidence_status | DIRECTLY_OBSERVED, ALGORITHM_INFERRED, MODEL_LATENT, UNRESOLVED |
| mechanism_assignment_method | Source method or explicit reason for unresolved |
| mechanism_evidence_reference | Source observation, fluorescence record, analysis receipt or model state reference |

The 2025 *P. putida* study segments runs and immobile episodes using k-means over speed and change-of-direction features. The 2026 study interprets short events as more likely active motor turns and long events as dominated by passive trapping, but duration alone does not directly observe physical cause.

Rules:
- Do not assign ACTIVE_MOTOR_EVENT solely because event duration is <= 1 second.
- Do not assign PASSIVE_MECHANICAL_TRAP solely because event duration is > 1 second.
- Preserve source event labels exactly; derived mechanism labels must record their inference method and evidence class.
- Where the data cannot distinguish mechanisms, use UNRESOLVED rather than forcing a binary label.
- High-speed fluorescence evidence for flagellar mode changes is distinct from phase-contrast trajectory segmentation and must retain its own source reference.

## Observed duration bin (independent of mechanism)

| Field | Values / meaning |
|---|---|
| event_duration_bin | SHORT, LONG, EXACT_BOUNDARY_EXCLUDED |
| event_class_threshold_s | Source-pinned threshold, normally 1.0 s for this study comparison |
| threshold_boundary_policy | SHORT_INCLUSIVE, LONG_INCLUSIVE, EXCLUDE_EXACT_BOUNDARY |
| source_event_label | Original label if supplied; do not overwrite |

Policies:
- SHORT_INCLUSIVE: short <= 1 s; long > 1 s.
- LONG_INCLUSIVE: short < 1 s; long >= 1 s.
- EXCLUDE_EXACT_BOUNDARY: exactly 1 s is retained but not assigned to either two-class comparison.

The 2026 paper uses <=1 s in Figure 3/4 descriptions and <1 s elsewhere; the 2025 paper uses a strict short/long description. Every analysis must pin the convention and run sensitivity cases. A duration bin is not ground-truth mechanism.

## Direction partition

Use a source-pinned direction rule. The current working partition is the sign of the displacement projection onto the gradient axis:
- UPGRADIENT when the projection is positive beyond the numerical deadband;
- DOWNGRADIENT when negative beyond the deadband;
- CROSSGRADIENT only within the declared numerical deadband.

The current model uses dimensionless tolerance 1e-12. This is not a biological angular cone. Confirm the original source-analysis partition from supplementary material/source code before claiming exact reproduction; preserve classifier revision and run partition sensitivity.

## Directional conditioning and causal interpretation

Keep these four direction variables independent:
1. Run-start heading: causal simulation input to the next-run time policy.
2. Realized preceding-run displacement heading: measured first-to-last vector; published run-orientation observable and condition for turn-angle analysis.
3. Run-end tangent: estimated instantaneous heading at event onset, if temporal resolution permits.
4. Following-run displacement heading: measured post-event outcome.

The first reference model samples run time from run-start direction as a declared causal approximation, then conditions the active motor turn-angle distribution on the realized run displacement direction. The benchmark must report the difference between run-start and realized directions rather than treating them as equivalent.

Post-event statistics pair event i with following run i+1. The final event with no observed following run is excluded from conditional denominators.

## Statistical hierarchy and splits

The 2026 study reports 10 one-minute recordings from different plates over two days for each agar concentration. Track counts are not independent biological replicate counts.

Retain day -> plate -> recording -> trajectory -> run/event ancestry. All events from a trajectory must remain in one split. Prefer held-out biological replicate/plate/day splits. Cluster bootstrap must resample independent recording/plate/day clusters; do not resample frames or individual events as though independent.

## Missingness and exclusions

Represent missing, censored, truncated, low-confidence, filtered and invalid values explicitly; never replace them with zero. Each exclusion must include a stable locator, exact filter predicate/version, reason and affected count, and whether exclusion occurred in source analysis or our adapter.

## Normalization receipt

Every ingestion run must emit:
- DOI and exact record/version;
- file name, byte size and SHA-256 for each ingested file;
- file-level licence and access status;
- parser and event-detection revision;
- source-to-normalized column mapping;
- row/event counts before and after each filter;
- missing/invalid/censored counts;
- units and coordinate-frame declarations;
- event-mechanism assignment method and evidence status;
- threshold policy and direction-classifier revision;
- replicate/recording/trajectory counts by condition;
- exact train/validation/test grouping;
- output artifact digests.

A successful parse establishes provenance and transformation traceability only; it is not a scientific reproduction.
