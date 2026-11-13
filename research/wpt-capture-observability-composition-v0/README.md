# WPT capture-conditioned robust observability v0

**Status:** exact finite-set composition of an upstream mechanical capture relation and an upstream robust observation-support relation. This is not a contact-mechanics solver, sensor estimator, active probe planner, EM model, power controller or safety/flight permit.

## Why this benchmark exists

A passive guide can reduce the set of possible post-contact states without making the remaining pose uniquely observable. A nominal sensor model can also appear unique while an admissible drift/fault model aliases another pose. The complete chain therefore has separate obligations:

1. Mechanical capture: enumerate every possible post-contact state and outcome.
2. Robust observation: retain every pose/model/fault tuple consistent with the observed signature.
3. Pose projection: report unique only when all surviving tuples refer to exactly one pose-equivalence ID.

The output explicitly says that uniqueness is only within the declared finite hypotheses. It does not infer physical model completeness or continuous 6-DOF observability.

## Ownership

- #4266 / PR #4270 owns finite mechanical capture set-map semantics.
- #4115 owns modality-specific pose equivalence and observation semantics.
- #4261 / PR #4265 owns robust finite model/fault support semantics.
- #4241 / PR #4262 owns motion-bounded hypothesis propagation during probe latency.
- #4214 / PR #4239 owns adaptive probe selection over upstream-qualified actions.
- #3902 owns the aircraft dock and complete turnaround service.
- #4157 owns benchmark composition and evidence joins.
- This crate owns only the composition of capture and observation set relations.

It does not generate the guide map, observation signatures, sensor fault set, calibration bounds, or probe actions.

## Inputs

The CLI accepts two separate TSV files.

### Capture relation

Exact header:

```text
case_id evidence_class decision_time_us initial_approach_states capture_map_sha256 receiver_config_sha256 guide_geometry_sha256 frame_sha256 calibration_sha256 approach_state_id post_state_id pose_id outcome transition_in_domain valid_until_us
```

Every initial approach state must have at least one transition. Duplicate approach/post-state edges and conflicting post-state-to-pose mappings are rejected. Possible outcomes include CAPTURED, NOT_CAPTURED, JAMMED, REBOUND, SLIP, OUT_OF_DOMAIN and UNKNOWN. Any possible non-capture, unknown or out-of-domain result prevents robust pose promotion for that approach.

### Observation support

Exact header:

```text
case_id evidence_class decision_time_us observation_map_sha256 calibration_set_sha256 model_set_sha256 frame_sha256 observed_signature post_state_id pose_id model_id model_sha256 fault_id predicted_signatures valid_until_us
```

Each possible captured post-state needs at least one observation hypothesis. Every tuple (post_state, model_id, fault_id) is unique; a model ID must map to exactly one model digest. Observation supports are set-valued and may overlap. No priors or probabilities are invented.

### Composition rules

- Case ID, evidence class, decision time, frame and calibration revisions must agree.
- Every captured post-state must have observation support. Missing support is a hard error, not an implicit exclusion.
- Any expired row invalidates the whole case rather than shrinking uncertainty.
- Referenced artifact digests are syntax-checked as lowercase SHA-256; this tool does not fetch or verify their bytes.
- For each approach state, first require that every possible capture edge is CAPTURED and in-domain.
- Retain every model/fault tuple whose support contains the observed signature for every reachable post-state.
- Project the survivors onto pose IDs. Exactly one pose ID gives POSE_UNIQUE_WITHIN_FINITE_HYPOTHESES; multiple pose IDs gives AMBIGUOUS_POSE; no support gives ABSTAIN_NO_SUPPORT.
- Mechanical capture never establishes pose uniqueness by itself.

## Frozen baselines

- Flat pad: three possible post-contact pose classes; the observed signature aliases two poses -> ambiguous.
- Passive V-guide: two post-contact pose classes, but a drift/fault model preserves an alias -> ambiguous.
- V-guide plus upstream-qualified joint probe signature: the remaining pose classes are distinguished across the declared model/fault set -> unique within finite hypotheses.
- Rebound control: at least one possible rebound branch prevents the whole approach from being treated as captured.
- Test-only uncertainty expansion: adding an admissible model/fault tuple cannot remove an existing survivor and can turn a previously unique result into ambiguity.

All values and signatures are synthetic known-truth software fixtures. They are not measured mechanical capture, physical sensor response, localization accuracy, probe energy, electrical efficiency, safety limits, or SOTA superiority.

## Run

```sh
cargo test --locked --manifest-path tools/wpt-capture-observability-composition/Cargo.toml
cargo run --locked --manifest-path tools/wpt-capture-observability-composition/Cargo.toml -- research/wpt-capture-observability-composition-v0/flat-pad-capture.tsv research/wpt-capture-observability-composition-v0/flat-pad-observations.tsv
cargo run --locked --manifest-path tools/wpt-capture-observability-composition/Cargo.toml -- research/wpt-capture-observability-composition-v0/v-guide-capture.tsv research/wpt-capture-observability-composition-v0/v-guide-probe-observations.tsv
cargo run --locked --manifest-path tools/wpt-capture-observability-composition/Cargo.toml -- research/wpt-capture-observability-composition-v0/rebound-capture.tsv research/wpt-capture-observability-composition-v0/rebound-observations.tsv
```

## Claim ceiling

A pass proves finite relation composition only. It does not validate mechanical guide physics, aircraft landing performance, calibration/model/fault completeness, continuous pose identifiability, electrical transfer, exposure/EMC, hardware safety, regulatory compliance or operational authority.