//! Resolution-aware particle metrology and nonparametric sample-adequacy evaluator.
//!
//! This crate qualifies only the declared reference checks and independent
//! random-sampling scope. It does not certify an instrument, cure selection
//! bias, handle clustered samples, or implement the complete GUM.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const TOLERANCE: f64 = 1.0e-10;
const MAX_SAMPLES: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceReport {
    pub reference_id: String,
    pub replicate_count: usize,
    pub session_count: usize,
    pub operator_count: usize,
    pub orientation_count: usize,
    pub mean_measured_value: f64,
    pub certified_value: f64,
    pub certified_expanded_uncertainty: f64,
    pub coverage_factor: f64,
    pub absolute_bias: f64,
    pub repeatability_envelope: f64,
    pub conservative_error_envelope: f64,
    pub maximum_abs_error: f64,
    pub disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuantileReport {
    pub probability: f64,
    pub confidence_target: f64,
    pub achieved_confidence: f64,
    pub estimate: Option<f64>,
    pub interval_lower: Option<f64>,
    pub interval_upper: Option<f64>,
    pub interval_width: Option<f64>,
    pub maximum_interval_width: f64,
    pub disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequentialStabilityReport {
    pub probability: f64,
    pub maximum_allowed_drift: f64,
    pub maximum_observed_drift: f64,
    pub last_instability_batch_index: Option<usize>,
    pub terminal_stable_transition_count: usize,
    pub required_terminal_stable_transitions: usize,
    pub batch_count: usize,
    pub disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SampleSetReport {
    pub sample_set_id: String,
    pub parent_bin_index: usize,
    pub descriptor_type: String,
    pub sample_count: usize,
    pub batch_count: usize,
    pub sequential_stability: Vec<SequentialStabilityReport>,
    pub exact_sample_count: usize,
    pub left_censored_count: usize,
    pub right_censored_count: usize,
    pub minimum_observed_feature_mm: Option<f64>,
    pub quantiles: Vec<QuantileReport>,
    pub disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub instrument_id: String,
    pub parent_morphology_case_id: String,
    pub parent_interval_id: String,
    pub reference_checks: Vec<ReferenceReport>,
    pub sample_sets: Vec<SampleSetReport>,
    pub evidence_disposition: String,
    pub overall_disposition: String,
    pub claim_boundary: String,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|number| number.is_finite())
}

fn uint(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOLERANCE * a.abs().max(b.abs()).max(1.0)
}

fn descriptor_basis(descriptor: &str) -> Option<&'static str> {
    match descriptor {
        "FERET_ASPECT_RATIO_2D"
        | "CIRCULARITY_2D"
        | "SOLIDITY_2D"
        | "FOURIER_ANGULARITY_2D"
        | "FOURIER_ROUGHNESS_2D" => Some("PROJECTION_2D"),
        "SPHERICITY_3D" | "ASPECT_RATIO_3D" | "CONVEXITY_3D" => {
            Some("RECONSTRUCTION_3D")
        }
        _ => None,
    }
}

fn descriptor_in_domain(descriptor: &str, value: f64) -> bool {
    match descriptor {
        "FERET_ASPECT_RATIO_2D" | "ASPECT_RATIO_3D" => value >= 1.0,
        "CIRCULARITY_2D" | "SOLIDITY_2D" | "SPHERICITY_3D" | "CONVEXITY_3D" => {
            value > 0.0 && value <= 1.0
        }
        "FOURIER_ANGULARITY_2D" | "FOURIER_ROUGHNESS_2D" => value >= 0.0,
        _ => false,
    }
}

fn method_supports_3d(method: &str) -> bool {
    matches!(
        method,
        "LIDAR_POINT_CLOUD" | "PHOTOGRAMMETRY_3D" | "STRUCTURED_LIGHT_3D" | "MULTIVIEW_RECONSTRUCTION"
    )
}

fn validate_descriptor_binding(
    descriptor: &str,
    basis: &str,
    method: &str,
    path: &str,
    errors: &mut Vec<Violation>,
) {
    match descriptor_basis(descriptor) {
        None => push(
            errors,
            "UNKNOWN_DESCRIPTOR_TYPE",
            path,
            "descriptor is outside the frozen v0.1 vocabulary",
        ),
        Some(expected) if expected != basis => push(
            errors,
            "DESCRIPTOR_BASIS_MISMATCH",
            path,
            format!("{descriptor} requires {expected}"),
        ),
        Some("RECONSTRUCTION_3D") if !method_supports_3d(method) => push(
            errors,
            "METHOD_CANNOT_SUPPORT_3D",
            path,
            "the declared acquisition method cannot support a 3D reconstruction descriptor",
        ),
        Some(_) => {}
    }
}

/// Validate the measurement case against the exact parent morphology case.
pub fn validate_against_parent(case: &Value, parent: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA_VERSION", "$.schema_version", "schema_version must be exactly 1");
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    if !matches!(
        text(case, "evidence_class"),
        Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")
    ) {
        push(&mut errors, "INVALID_EVIDENCE_CLASS", "$.evidence_class", "evidence class is outside v0.1 vocabulary");
    }

    let Some(instrument) = case.get("instrument") else {
        push(&mut errors, "MISSING_INSTRUMENT", "$.instrument", "instrument metadata is required");
        return errors;
    };
    let method = text(instrument, "method").unwrap_or("");
    if !matches!(
        method,
        "STATIC_2D_IMAGE"
            | "DYNAMIC_2D_IMAGE"
            | "LIDAR_POINT_CLOUD"
            | "PHOTOGRAMMETRY_3D"
            | "STRUCTURED_LIGHT_3D"
            | "MULTIVIEW_RECONSTRUCTION"
    ) {
        push(&mut errors, "UNSUPPORTED_INSTRUMENT_METHOD", "$.instrument.method", "instrument method is outside v0.1 vocabulary");
    }
    for key in ["instrument_id", "calibration_ref", "source_artifact_ref", "reference_profile_ref"] {
        if text(instrument, key).is_none() {
            push(&mut errors, "MISSING_INSTRUMENT_PROVENANCE", &format!("$.instrument.{key}"), "instrument identity, calibration, source and reference profile are required");
        }
    }
    for key in ["spatial_resolution_mm", "descriptor_resolution"] {
        match finite(instrument, key) {
            Some(value) if value > 0.0 => {}
            _ => push(&mut errors, "INVALID_INSTRUMENT_RESOLUTION", &format!("$.instrument.{key}"), "instrument resolution must be finite and positive"),
        }
    }

    let Some(protocol) = case.get("protocol") else {
        push(&mut errors, "MISSING_PROTOCOL", "$.protocol", "sampling and parent protocol is required");
        return errors;
    };
    for key in [
        "parent_morphology_case_id",
        "parent_interval_id",
        "sampling_frame_ref",
        "sampling_protocol_ref",
        "quantile_method_ref",
        "review_authority_ref",
    ] {
        if text(protocol, key).is_none() {
            push(&mut errors, "MISSING_PROTOCOL_PROVENANCE", &format!("$.protocol.{key}"), "parent, sampling, quantile method and review references are required");
        }
    }
    if text(protocol, "parent_morphology_case_id") != text(parent, "case_id") {
        push(&mut errors, "PARENT_MORPHOLOGY_CASE_MISMATCH", "$.protocol.parent_morphology_case_id", "exact morphology parent case identity is required");
    }
    let null_acquisition = Value::Null;
    let parent_acquisition = parent.get("acquisition").unwrap_or(&null_acquisition);
    let parent_interval_id = text(parent_acquisition, "parent_interval_id");
    if text(protocol, "parent_interval_id") != parent_interval_id {
        push(&mut errors, "PARENT_MORPHOLOGY_INTERVAL_MISMATCH", "$.protocol.parent_interval_id", "exact morphology parent interval identity is required");
    }

    let Some(reference_checks) = case.get("reference_checks").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_REFERENCE_CHECKS", "$.reference_checks", "reference_checks must be an array");
        return errors;
    };
    if reference_checks.is_empty() {
        push(&mut errors, "EMPTY_REFERENCE_CHECKS", "$.reference_checks", "at least one certified reference artifact is required");
    }
    let mut reference_ids = BTreeSet::new();
    for (index, reference) in reference_checks.iter().enumerate() {
        let path = format!("$.reference_checks[{index}]");
        let Some(id) = text(reference, "reference_id") else {
            push(&mut errors, "MISSING_REFERENCE_ID", &format!("{path}.reference_id"), "reference_id is required");
            continue;
        };
        if !reference_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_REFERENCE_ID", &format!("{path}.reference_id"), "reference identifiers must be unique");
        }
        let descriptor = text(reference, "descriptor_type").unwrap_or("");
        let basis = text(reference, "basis").unwrap_or("");
        validate_descriptor_binding(descriptor, basis, method, &format!("{path}.descriptor_type"), &mut errors);
        if text(reference, "calibration_ref") != text(instrument, "calibration_ref") {
            push(&mut errors, "REFERENCE_CALIBRATION_MISMATCH", &format!("{path}.calibration_ref"), "reference measurement must bind to the declared instrument calibration");
        }
        if text(reference, "descriptor_definition_ref").is_none()
            || text(reference, "certificate_ref").is_none()
        {
            push(&mut errors, "MISSING_REFERENCE_DEFINITION_OR_CERTIFICATE", &path, "descriptor definition and reference certificate are required");
        }
        match finite(reference, "certified_value") {
            Some(value) if descriptor_in_domain(descriptor, value) => {}
            _ => push(&mut errors, "INVALID_CERTIFIED_REFERENCE_VALUE", &format!("{path}.certified_value"), "certified reference value must be finite and in the descriptor domain"),
        }
        match finite(reference, "coverage_factor") {
            Some(value) if value >= 1.0 => {}
            _ => push(&mut errors, "INVALID_REFERENCE_COVERAGE_FACTOR", &format!("{path}.coverage_factor"), "expanded uncertainty coverage factor must be finite and at least 1"),
        }
        for key in ["certified_expanded_uncertainty", "feature_dimension_mm", "maximum_abs_error"] {
            match finite(reference, key) {
                Some(value) if value >= 0.0 && (key != "feature_dimension_mm" && key != "maximum_abs_error" || value > 0.0) => {}
                _ => push(&mut errors, "INVALID_REFERENCE_BOUND", &format!("{path}.{key}"), "reference uncertainty must be non-negative; feature dimension and maximum error must be positive"),
            }
        }
        if finite(reference, "feature_dimension_mm").zip(finite(instrument, "spatial_resolution_mm"))
            .is_some_and(|(feature, resolution)| feature < resolution)
        {
            push(&mut errors, "REFERENCE_BELOW_SPATIAL_RESOLUTION", &format!("{path}.feature_dimension_mm"), "reference artifact feature is below the instrument's declared spatial resolution");
        }

        let Some(replicates) = reference.get("replicates").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_REFERENCE_REPLICATES", &format!("{path}.replicates"), "replicates must be an array");
            continue;
        };
        if replicates.len() < 3 {
            push(&mut errors, "INSUFFICIENT_REFERENCE_REPLICATES", &format!("{path}.replicates"), "at least three reference replicates are required");
        }
        let mut replicate_ids = BTreeSet::new();
        let mut reference_sources = BTreeSet::new();
        let mut reference_analysis = BTreeSet::new();
        let mut sessions = BTreeSet::new();
        let mut operators = BTreeSet::new();
        let mut orientations = BTreeSet::new();
        for (replicate_index, replicate) in replicates.iter().enumerate() {
            let replicate_path = format!("{path}.replicates[{replicate_index}]");
            if let Some(rep_id) = text(replicate, "replicate_id") {
                if !replicate_ids.insert(rep_id.to_owned()) {
                    push(&mut errors, "DUPLICATE_REFERENCE_REPLICATE", &format!("{replicate_path}.replicate_id"), "reference replicate IDs must be unique");
                }
            } else {
                push(&mut errors, "MISSING_REFERENCE_REPLICATE_ID", &format!("{replicate_path}.replicate_id"), "replicate ID is required");
            }
            for (key, set) in [
                ("session_ref", &mut sessions),
                ("operator_ref", &mut operators),
                ("orientation_ref", &mut orientations),
            ] {
                if let Some(value) = text(replicate, key) {
                    set.insert(value.to_owned());
                } else {
                    push(&mut errors, "MISSING_REFERENCE_REPLICATE_CONTEXT", &format!("{replicate_path}.{key}"), "session, operator and orientation are required");
                }
            }
            if let Some(source) = text(replicate, "source_artifact_ref") {
                if !reference_sources.insert(source.to_owned()) {
                    push(&mut errors, "DUPLICATE_REFERENCE_SOURCE_ARTIFACT", &format!("{replicate_path}.source_artifact_ref"), "reference replicates must use distinct raw scan artifacts");
                }
            } else {
                push(&mut errors, "MISSING_REFERENCE_REPLICATE_ARTIFACT", &format!("{replicate_path}.source_artifact_ref"), "raw reference scan artifact is required");
            }
            if let Some(analysis) = text(replicate, "analysis_record_ref") {
                if !reference_analysis.insert(analysis.to_owned()) {
                    push(&mut errors, "DUPLICATE_REFERENCE_ANALYSIS_RECORD", &format!("{replicate_path}.analysis_record_ref"), "reference replicates must use distinct analysis records");
                }
            } else {
                push(&mut errors, "MISSING_REFERENCE_REPLICATE_ARTIFACT", &format!("{replicate_path}.analysis_record_ref"), "reference analysis record is required");
            }
            match finite(replicate, "measured_value") {
                Some(value) if descriptor_in_domain(descriptor, value) => {}
                _ => push(&mut errors, "INVALID_REFERENCE_REPLICATE_VALUE", &format!("{replicate_path}.measured_value"), "replicate value must be finite and in the descriptor domain"),
            }
        }
        if sessions.len() < 2 || operators.len() < 2 || orientations.len() < 2 {
            push(&mut errors, "INSUFFICIENT_REFERENCE_REPRODUCIBILITY_COVERAGE", &format!("{path}.replicates"), "reference qualification needs at least two sessions, operators and orientations");
        }
    }

    let Some(sample_sets) = case.get("sample_sets").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_SAMPLE_SETS", "$.sample_sets", "sample_sets must be an array");
        return errors;
    };
    if sample_sets.is_empty() {
        push(&mut errors, "EMPTY_SAMPLE_SETS", "$.sample_sets", "at least one parent-size-stratum sample set is required");
    }
    let mut sample_set_ids = BTreeSet::new();

    for (set_index, sample_set) in sample_sets.iter().enumerate() {
        let path = format!("$.sample_sets[{set_index}]");
        let Some(set_id) = text(sample_set, "sample_set_id") else {
            push(&mut errors, "MISSING_SAMPLE_SET_ID", &format!("{path}.sample_set_id"), "sample_set_id is required");
            continue;
        };
        if !sample_set_ids.insert(set_id.to_owned()) {
            push(&mut errors, "DUPLICATE_SAMPLE_SET_ID", &format!("{path}.sample_set_id"), "sample set identifiers must be unique");
        }
        if text(sample_set, "parent_morphology_case_id") != text(parent, "case_id")
            || text(sample_set, "parent_interval_id") != parent_interval_id
        {
            push(&mut errors, "SAMPLE_SET_PARENT_MISMATCH", &path, "sample set must bind to the exact parent morphology case and interval");
        }
        if text(sample_set, "selection_method") != Some("RANDOMIZED_WITHIN_PARENT_SIZE_STRATUM") {
            push(&mut errors, "UNSUPPORTED_SAMPLE_SELECTION_METHOD", &format!("{path}.selection_method"), "v0.1 requires randomized selection within the exact parent size stratum");
        }
        if text(sample_set, "independence_basis") != Some("ONE_PARTICLE_PER_INDEPENDENT_ACQUISITION_BLOCK") {
            push(&mut errors, "UNSUPPORTED_SAMPLE_INDEPENDENCE_BASIS", &format!("{path}.independence_basis"), "v0.1 only admits one particle per independently declared acquisition block");
        }

        let descriptor = text(sample_set, "descriptor_type").unwrap_or("");
        let basis = text(sample_set, "basis").unwrap_or("");
        validate_descriptor_binding(descriptor, basis, method, &format!("{path}.descriptor_type"), &mut errors);
        if text(sample_set, "descriptor_definition_ref").is_none()
            || text(sample_set, "sampling_protocol_ref") != text(protocol, "sampling_protocol_ref")
        {
            push(&mut errors, "SAMPLE_SET_PROTOCOL_MISMATCH", &path, "sample set descriptor definition and sampling protocol references must be explicit and consistent");
        }

        let parent_bin_index = uint(sample_set, "parent_bin_index");
        let parent_strata = parent.get("size_strata").and_then(Value::as_array);
        let parent_stratum = parent_strata.into_iter().flatten().find(|stratum| {
            uint(stratum, "parent_bin_index") == parent_bin_index
        });
        let parent_bin_lower = parent_stratum.and_then(|stratum| finite(stratum, "lower_mm"));
        let parent_bin_upper = parent_stratum.and_then(|stratum| stratum.get("upper_mm").and_then(Value::as_f64));
        let parent_descriptor = parent_stratum
            .and_then(|stratum| stratum.get("descriptors"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|item| {
                text(item, "descriptor_type") == Some(descriptor)
                    && text(item, "basis") == Some(basis)
                    && text(item, "descriptor_definition_ref") == text(sample_set, "descriptor_definition_ref")
            });
        if parent_descriptor.is_none() {
            push(&mut errors, "PARENT_MORPHOLOGY_DESCRIPTOR_NOT_FOUND", &path, "sample set must match a descriptor in the exact parent PSD stratum and definition version");
        } else if let (Some(sample_count), Some(parent_count)) = (
            sample_set.get("samples").and_then(Value::as_array).map(Vec::len),
            parent_descriptor.and_then(|value| uint(value, "sample_count")),
        ) {
            if sample_count > parent_count as usize {
                push(&mut errors, "SAMPLE_COUNT_EXCEEDS_PARENT_DECLARATION", &format!("{path}.samples"), "qualification samples cannot exceed the parent morphology descriptor's declared sample count");
            }
        }

        let lower_domain = finite(sample_set, "domain_lower_bound");
        let upper_domain = sample_set.get("domain_upper_bound").and_then(Value::as_f64).filter(|value| value.is_finite());
        if lower_domain.is_none()
            || upper_domain.is_some_and(|upper| lower_domain.is_some_and(|lower| upper <= lower))
        {
            push(&mut errors, "INVALID_DESCRIPTOR_DOMAIN", &path, "descriptor domain requires a finite lower bound and an optional finite upper bound greater than lower");
        }
        match (finite(sample_set, "descriptor_resolution"), finite(instrument, "descriptor_resolution")) {
            (Some(actual), Some(expected)) if actual > 0.0 && close(actual, expected) => {}
            _ => push(&mut errors, "DESCRIPTOR_RESOLUTION_MISMATCH", &format!("{path}.descriptor_resolution"), "sample set descriptor resolution must match the declared instrument resolution"),
        }
        match (finite(sample_set, "minimum_resolvable_feature_mm"), finite(instrument, "spatial_resolution_mm")) {
            (Some(minimum), Some(resolution)) if minimum >= resolution => {}
            _ => push(&mut errors, "INVALID_MINIMUM_RESOLVABLE_FEATURE", &format!("{path}.minimum_resolvable_feature_mm"), "minimum resolvable feature must be finite and no smaller than declared spatial resolution"),
        }
        if text(sample_set, "parent_interval_id") != parent_interval_id {
            push(&mut errors, "SAMPLE_SET_INTERVAL_MISMATCH", &format!("{path}.parent_interval_id"), "sample set interval must match the parent morphology interval");
        }

        let Some(targets) = sample_set.get("quantile_targets").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_QUANTILE_TARGETS", &format!("{path}.quantile_targets"), "quantile targets must be an array");
            continue;
        };
        if targets.is_empty() {
            push(&mut errors, "EMPTY_QUANTILE_TARGETS", &format!("{path}.quantile_targets"), "at least one quantile target is required");
        }
        let mut probabilities = BTreeSet::new();
        for (target_index, target) in targets.iter().enumerate() {
            let target_path = format!("{path}.quantile_targets[{target_index}]");
            let p = finite(target, "probability");
            if p.is_none_or(|value| !(0.0..1.0).contains(&value)) {
                push(&mut errors, "INVALID_QUANTILE_PROBABILITY", &format!("{target_path}.probability"), "quantile probability must be finite and strictly between zero and one");
            } else if !probabilities.insert(p.unwrap().to_bits()) {
                push(&mut errors, "DUPLICATE_QUANTILE_TARGET", &format!("{target_path}.probability"), "quantile target probabilities must be unique");
            }
            match finite(target, "confidence_level") {
                Some(value) if value > 0.5 && value < 1.0 => {}
                _ => push(&mut errors, "INVALID_QUANTILE_CONFIDENCE", &format!("{target_path}.confidence_level"), "confidence level must be in (0.5, 1)"),
            }
            match (finite(target, "maximum_interval_width"), finite(sample_set, "descriptor_resolution")) {
                (Some(width), Some(resolution)) if width > 0.0 && width + TOLERANCE >= resolution => {}
                _ => push(&mut errors, "TARGET_BELOW_INSTRUMENT_RESOLUTION", &format!("{target_path}.maximum_interval_width"), "maximum quantile interval width must be at least one descriptor-resolution quantum"),
            }
        }

        let Some(samples) = sample_set.get("samples").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_SAMPLES", &format!("{path}.samples"), "samples must be an array");
            continue;
        };
        if samples.len() < 2 || samples.len() > MAX_SAMPLES {
            push(&mut errors, "SAMPLE_COUNT_OUT_OF_RANGE", &format!("{path}.samples"), format!("sample count must be between 2 and {MAX_SAMPLES} for v0.1 exact order-statistic evaluation"));
        }
        let mut particle_ids = BTreeSet::new();
        let mut acquisition_blocks = BTreeSet::new();
        let mut particle_sources = BTreeSet::new();
        let mut segmentation_records = BTreeSet::new();
        let mut batch_counts: BTreeMap<u64, (String, usize)> = BTreeMap::new();
        let mut batch_ids = BTreeSet::new();
        let mut previous_batch_index: Option<u64> = None;
        for (sample_index, sample) in samples.iter().enumerate() {
            let sample_path = format!("{path}.samples[{sample_index}]");
            let particle_id = text(sample, "particle_id");
            let block_id = text(sample, "acquisition_block_ref");
            let batch_index = uint(sample, "batch_index");
            let batch_id = text(sample, "batch_id");
            match (batch_index, batch_id) {
                (Some(index), Some(id)) => {
                    if previous_batch_index.is_some_and(|previous| index < previous || index > previous + 1) {
                        push(&mut errors, "NONCONTIGUOUS_OR_UNSORTED_SAMPLE_BATCH", &sample_path, "sample batches must be ordered and contiguous from index zero");
                    }
                    previous_batch_index = Some(index);
                    if let Some((existing_id, count)) = batch_counts.get_mut(&index) {
                        if existing_id != id {
                            push(&mut errors, "SAMPLE_BATCH_ID_MISMATCH", &format!("{sample_path}.batch_id"), "all samples in one batch index must share the same batch ID");
                        }
                        *count += 1;
                    } else {
                        if !batch_ids.insert(id.to_owned()) {
                            push(&mut errors, "DUPLICATE_SAMPLE_BATCH_ID", &format!("{sample_path}.batch_id"), "different batch indices must use unique batch IDs");
                        }
                        batch_counts.insert(index, (id.to_owned(), 1));
                    }
                }
                _ => push(&mut errors, "MISSING_SAMPLE_BATCH_IDENTITY", &sample_path, "each sample must declare a sequential batch index and ID"),
            }
            if let Some(source) = text(sample, "source_artifact_ref") {
                if !particle_sources.insert(source.to_owned()) {
                    push(&mut errors, "DUPLICATE_PARTICLE_SOURCE_ARTIFACT", &format!("{sample_path}.source_artifact_ref"), "each independently sampled particle must have a distinct raw acquisition artifact in v0.1");
                }
            } else {
                push(&mut errors, "MISSING_PARTICLE_ARTIFACT_PROVENANCE", &format!("{sample_path}.source_artifact_ref"), "raw particle acquisition artifact is required");
            }
            if let Some(segmentation) = text(sample, "segmentation_record_ref") {
                if !segmentation_records.insert(segmentation.to_owned()) {
                    push(&mut errors, "DUPLICATE_SEGMENTATION_RECORD", &format!("{sample_path}.segmentation_record_ref"), "each independently sampled particle must have a distinct segmentation record");
                }
            } else {
                push(&mut errors, "MISSING_PARTICLE_ARTIFACT_PROVENANCE", &format!("{sample_path}.segmentation_record_ref"), "particle segmentation record is required");
            }
            if text(sample, "size_measurement_ref").is_none() {
                push(&mut errors, "MISSING_PARTICLE_SIZE_PROVENANCE", &format!("{sample_path}.size_measurement_ref"), "particle size bin assignment needs a measurement reference");
            }
            match (
                finite(sample, "particle_size_lower_mm"),
                finite(sample, "particle_size_upper_mm"),
                parent_bin_lower,
                parent_bin_upper,
            ) {
                (Some(measured_lower), Some(measured_upper), Some(parent_lower), Some(parent_upper))
                    if measured_lower >= parent_lower && measured_upper >= measured_lower && measured_upper < parent_upper => {}
                (Some(measured_lower), Some(measured_upper), Some(parent_lower), None)
                    if measured_lower >= parent_lower && measured_upper >= measured_lower => {}
                _ => push(&mut errors, "PARTICLE_SIZE_INTERVAL_CROSSES_PARENT_BIN", &sample_path, "particle size uncertainty interval must fit wholly inside the exact parent PSD bin; upper boundary is exclusive"),
            }
            if particle_id.is_none_or(|id| !particle_ids.insert(id.to_owned())) {
                push(&mut errors, "MISSING_OR_DUPLICATE_PARTICLE_ID", &format!("{sample_path}.particle_id"), "each particle must have a unique ID within the sample set");
            }
            if block_id.is_none_or(|id| !acquisition_blocks.insert(id.to_owned())) {
                push(&mut errors, "DUPLICATE_ACQUISITION_BLOCK", &format!("{sample_path}.acquisition_block_ref"), "one particle per independently identified acquisition block is required; clustered observations are not treated as IID");
            }
            match text(sample, "resolution_status") {
                Some("EXACT") => {
                    match finite(sample, "value") {
                        Some(value) if descriptor_in_domain(descriptor, value)
                            && lower_domain.is_some_and(|lower| value >= lower)
                            && upper_domain.is_none_or(|upper| value <= upper) => {}
                        _ => push(&mut errors, "INVALID_EXACT_SAMPLE_VALUE", &format!("{sample_path}.value"), "exact sample value must be finite and within the declared descriptor domain"),
                    }
                    if sample.get("censor_bound").is_some() {
                        push(&mut errors, "EXACT_SAMPLE_HAS_CENSOR_BOUND", &sample_path, "exact sample cannot also carry a censor bound");
                    }
                    match finite(sample, "feature_dimension_mm").zip(finite(sample_set, "minimum_resolvable_feature_mm")) {
                        Some((feature, minimum)) if feature >= minimum => {}
                        _ => push(&mut errors, "SAMPLE_BELOW_SPATIAL_RESOLUTION", &format!("{sample_path}.feature_dimension_mm"), "sample feature dimension is missing or below the declared resolution limit; it cannot be treated as an exact measurement"),
                    }
                }
                Some("LEFT_CENSORED" | "RIGHT_CENSORED") => {
                    if sample.get("value").is_some() {
                        push(&mut errors, "CENSORED_SAMPLE_HAS_EXACT_VALUE", &format!("{sample_path}.value"), "censored observations cannot be replaced with an exact value or resolution floor");
                    }
                    match finite(sample, "censor_bound") {
                        Some(bound) if descriptor_in_domain(descriptor, bound)
                            && lower_domain.is_some_and(|lower| bound >= lower)
                            && upper_domain.is_none_or(|upper| bound <= upper) => {}
                        _ => push(&mut errors, "INVALID_CENSOR_BOUND", &format!("{sample_path}.censor_bound"), "censor bound must be finite and within the declared descriptor domain"),
                    }
                }
                _ => push(&mut errors, "INVALID_RESOLUTION_STATUS", &format!("{sample_path}.resolution_status"), "resolution status must be EXACT, LEFT_CENSORED or RIGHT_CENSORED"),
            }
        }

        let Some(stability) = sample_set.get("sequential_stability") else {
            push(&mut errors, "MISSING_SEQUENTIAL_STABILITY_POLICY", &format!("{path}.sequential_stability"), "sequential terminal-stability policy is required");
            continue;
        };
        let minimum_batches = uint(stability, "minimum_batch_count").unwrap_or(0);
        let minimum_samples_per_batch = uint(stability, "minimum_samples_per_batch").unwrap_or(0);
        let stable_tail_batches = uint(stability, "stable_tail_batch_count").unwrap_or(0);
        if minimum_batches < 3 || minimum_samples_per_batch < 2 || stable_tail_batches < 1 {
            push(&mut errors, "INVALID_SEQUENTIAL_STABILITY_POLICY", &format!("{path}.sequential_stability"), "minimum batches >= 3, samples per batch >= 2, and stable tail >= 1 are required");
        }
        if batch_counts.len() < minimum_batches as usize {
            push(&mut errors, "INSUFFICIENT_SEQUENTIAL_BATCHES", &format!("{path}.sequential_stability"), "observed batch count is below the predeclared minimum");
        }
        if batch_counts.len().saturating_sub(1) < stable_tail_batches as usize {
            push(&mut errors, "STABLE_TAIL_EXCEEDS_AVAILABLE_BATCHES", &format!("{path}.sequential_stability"), "stable tail transition count exceeds observed batch transitions");
        }
        for (batch_index, (_, count)) in &batch_counts {
            if *count < minimum_samples_per_batch as usize {
                push(&mut errors, "INSUFFICIENT_SAMPLES_IN_BATCH", &format!("{path}.sequential_stability"), format!("batch {batch_index} has fewer samples than the predeclared minimum"));
            }
        }

        let Some(stability_targets) = stability.get("targets").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_SEQUENTIAL_STABILITY_TARGETS", &format!("{path}.sequential_stability.targets"), "stability targets must be an array");
            continue;
        };
        let mut stability_probabilities = BTreeSet::new();
        for (target_index, target) in stability_targets.iter().enumerate() {
            let target_path = format!("{path}.sequential_stability.targets[{target_index}]");
            match finite(target, "probability") {
                Some(probability) if probability > 0.0 && probability < 1.0 => {
                    if !stability_probabilities.insert(probability.to_bits()) {
                        push(&mut errors, "DUPLICATE_SEQUENTIAL_STABILITY_TARGET", &target_path, "stability target probabilities must be unique");
                    }
                    if !probabilities.contains(&probability.to_bits()) {
                        push(&mut errors, "STABILITY_TARGET_WITHOUT_QUANTILE_TARGET", &target_path, "each stability target must match one predeclared quantile target");
                    }
                }
                _ => push(&mut errors, "INVALID_SEQUENTIAL_STABILITY_PROBABILITY", &target_path, "stability probability must be finite and in (0,1)"),
            }
            match finite(target, "maximum_batch_to_batch_drift") {
                Some(drift) if drift > 0.0 => {}
                _ => push(&mut errors, "INVALID_SEQUENTIAL_STABILITY_DRIFT", &target_path, "maximum batch-to-batch drift must be finite and positive"),
            }
        }
        if stability_probabilities != probabilities {
            push(&mut errors, "SEQUENTIAL_STABILITY_TARGET_SET_MISMATCH", &format!("{path}.sequential_stability.targets"), "stability target probabilities must exactly match the quantile precision target set");
        }
    }

    errors
}

fn log_factorials(n: usize) -> Vec<f64> {
    let mut values = vec![0.0; n + 1];
    for index in 1..=n {
        values[index] = values[index - 1] + (index as f64).ln();
    }
    values
}

fn binomial_probabilities(n: usize, probability: f64) -> Vec<f64> {
    let log_factorials = log_factorials(n);
    let log_p = probability.ln();
    let log_q = (1.0 - probability).ln();
    let mut probabilities = Vec::with_capacity(n + 1);
    for k in 0..=n {
        let log_probability = log_factorials[n]
            - log_factorials[k]
            - log_factorials[n - k]
            + k as f64 * log_p
            + (n - k) as f64 * log_q;
        probabilities.push(log_probability.exp());
    }
    let sum: f64 = probabilities.iter().sum();
    if sum > 0.0 && sum.is_finite() {
        for value in &mut probabilities {
            *value /= sum;
        }
    }
    probabilities
}

fn nonparametric_quantile_interval(
    values: &[f64],
    probability: f64,
    confidence: f64,
    domain_lower: f64,
    domain_upper: Option<f64>,
) -> Option<(f64, f64, f64)> {
    let n = values.len();
    if n < 2 || !(0.0..1.0).contains(&probability) {
        return None;
    }

    let binomial = binomial_probabilities(n, probability);
    let mut prefix = vec![0.0; n + 2];
    for (index, value) in binomial.iter().enumerate() {
        prefix[index + 1] = prefix[index] + value;
    }

    let mut best: Option<(f64, f64, f64)> = None;
    for lower_rank in 0..=n {
        let lower = if lower_rank == 0 {
            domain_lower
        } else {
            values[lower_rank - 1]
        };
        for upper_rank in (lower_rank + 1)..=n + 1 {
            let upper = if upper_rank <= n {
                Some(values[upper_rank - 1])
            } else {
                domain_upper
            };
            let Some(upper) = upper else {
                continue;
            };
            if upper < lower {
                continue;
            }
            let achieved = (prefix[upper_rank] - prefix[lower_rank]).clamp(0.0, 1.0);
            if achieved + TOLERANCE < confidence {
                continue;
            }
            let width = upper - lower;
            let replace = best.is_none_or(|current| {
                width < current.2 - TOLERANCE
                    || ((width - current.2).abs() <= TOLERANCE && achieved > current.0)
            });
            if replace {
                best = Some((achieved, lower, upper));
            }
        }
    }
    best
}

fn empirical_quantile(values: &[f64], probability: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((probability * sorted.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    sorted[index]
}

fn sequential_stability_reports(
    sample_set: &Value,
    samples: &[Value],
) -> (usize, Vec<SequentialStabilityReport>) {
    let mut batches: BTreeMap<u64, Vec<f64>> = BTreeMap::new();
    for sample in samples {
        let index = uint(sample, "batch_index").expect("validated batch index");
        let value = finite(sample, "value").expect("validated exact sample value");
        batches.entry(index).or_default().push(value);
    }

    let policy = &sample_set["sequential_stability"];
    let required_tail = uint(policy, "stable_tail_batch_count").expect("validated tail count") as usize;
    let batch_count = batches.len();
    let mut cumulative = Vec::new();
    let mut estimates_by_probability: BTreeMap<u64, Vec<(u64, f64)>> = BTreeMap::new();

    for (batch_index, values) in &batches {
        cumulative.extend(values.iter().copied());
        for target in policy["targets"].as_array().expect("validated stability targets") {
            let probability = finite(target, "probability").expect("validated stability probability");
            let estimate = empirical_quantile(&cumulative, probability);
            estimates_by_probability
                .entry(probability.to_bits())
                .or_default()
                .push((*batch_index, estimate));
        }
    }

    let mut reports = Vec::new();
    for target in policy["targets"].as_array().expect("validated stability targets") {
        let probability = finite(target, "probability").expect("validated stability probability");
        let maximum_drift = finite(target, "maximum_batch_to_batch_drift")
            .expect("validated maximum drift");
        let estimates = &estimates_by_probability[&probability.to_bits()];
        let mut maximum_observed_drift = 0.0_f64;
        let mut last_instability = None;
        for pair in estimates.windows(2) {
            let drift = (pair[1].1 - pair[0].1).abs();
            maximum_observed_drift = maximum_observed_drift.max(drift);
            if drift > maximum_drift + TOLERANCE {
                last_instability = Some(pair[1].0 as usize);
            }
        }
        let last_batch_index = estimates.last().map(|(index, _)| *index as usize).unwrap_or(0);
        let terminal_stable_count = last_instability
            .map(|index| last_batch_index.saturating_sub(index))
            .unwrap_or_else(|| batch_count.saturating_sub(1));
        let pass = batch_count >= uint(policy, "minimum_batch_count").expect("validated minimum batch count") as usize
            && terminal_stable_count >= required_tail;
        reports.push(SequentialStabilityReport {
            probability,
            maximum_allowed_drift: maximum_drift,
            maximum_observed_drift,
            last_instability_batch_index: last_instability,
            terminal_stable_transition_count: terminal_stable_count,
            required_terminal_stable_transitions: required_tail,
            batch_count,
            disposition: if pass { "PASS_TERMINAL_STABILITY" } else { "FAIL_TERMINAL_STABILITY" }.to_owned(),
        });
    }
    (batch_count, reports)
}

fn reference_reports(
    case: &Value,
) -> Vec<ReferenceReport> {
    let instrument = &case["instrument"];
    let resolution = finite(instrument, "descriptor_resolution").expect("validated descriptor resolution");
    case["reference_checks"]
        .as_array()
        .expect("validated reference checks")
        .iter()
        .map(|reference| {
            let replicates = reference["replicates"].as_array().expect("validated replicates");
            let values: Vec<f64> = replicates
                .iter()
                .map(|replicate| finite(replicate, "measured_value").expect("validated replicate value"))
                .collect();
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let repeatability_envelope = values
                .iter()
                .map(|value| (value - mean).abs())
                .fold(0.0_f64, f64::max);
            let certified = finite(reference, "certified_value").expect("validated certified value");
            let absolute_bias = (mean - certified).abs();
            let certified_expanded_uncertainty =
                finite(reference, "certified_expanded_uncertainty").expect("validated expanded uncertainty");
            let coverage_factor =
                finite(reference, "coverage_factor").expect("validated coverage factor");
            let conservative_error_envelope = absolute_bias
                + certified_expanded_uncertainty
                + repeatability_envelope
                + resolution / 2.0;
            let maximum = finite(reference, "maximum_abs_error").expect("validated maximum error");
            ReferenceReport {
                reference_id: text(reference, "reference_id").expect("validated reference ID").to_owned(),
                replicate_count: replicates.len(),
                session_count: replicates.iter().filter_map(|value| text(value, "session_ref")).collect::<BTreeSet<_>>().len(),
                operator_count: replicates.iter().filter_map(|value| text(value, "operator_ref")).collect::<BTreeSet<_>>().len(),
                orientation_count: replicates.iter().filter_map(|value| text(value, "orientation_ref")).collect::<BTreeSet<_>>().len(),
                mean_measured_value: mean,
                certified_value: certified,
                certified_expanded_uncertainty,
                coverage_factor,
                absolute_bias,
                repeatability_envelope,
                conservative_error_envelope,
                maximum_abs_error: maximum,
                disposition: if conservative_error_envelope <= maximum + TOLERANCE {
                    "PASS"
                } else {
                    "FAIL_REFERENCE_ERROR_ENVELOPE"
                }
                .to_owned(),
            }
        })
        .collect()
}

fn sample_set_reports(case: &Value) -> Vec<SampleSetReport> {
    let instrument = &case["instrument"];
    case["sample_sets"]
        .as_array()
        .expect("validated sample sets")
        .iter()
        .map(|sample_set| {
            let samples = sample_set["samples"].as_array().expect("validated samples");
            let mut exact_values = Vec::new();
            let mut left_censored = 0usize;
            let mut right_censored = 0usize;
            let mut min_feature = f64::INFINITY;
            for sample in samples {
                match text(sample, "resolution_status").expect("validated resolution status") {
                    "EXACT" => {
                        exact_values.push(finite(sample, "value").expect("validated exact value"));
                        min_feature = min_feature.min(
                            finite(sample, "feature_dimension_mm").expect("validated feature size"),
                        );
                    }
                    "LEFT_CENSORED" => left_censored += 1,
                    "RIGHT_CENSORED" => right_censored += 1,
                    _ => unreachable!("validated resolution status"),
                }
            }

            let (batch_count, sequential_reports) = if left_censored + right_censored == 0 {
                sequential_stability_reports(sample_set, samples)
            } else {
                (samples.iter().filter_map(|sample| uint(sample, "batch_index")).collect::<BTreeSet<_>>().len(), Vec::new())
            };
            let mut quantile_reports = Vec::new();
            let descriptor_resolution = finite(sample_set, "descriptor_resolution")
                .expect("validated descriptor resolution");
            let mut all_quantiles_pass = true;
            if left_censored + right_censored > 0 {
                all_quantiles_pass = false;
                for target in sample_set["quantile_targets"].as_array().expect("validated quantile targets") {
                    let p = finite(target, "probability").expect("validated p");
                    quantile_reports.push(QuantileReport {
                        probability: p,
                        confidence_target: finite(target, "confidence_level").expect("validated confidence"),
                        achieved_confidence: 0.0,
                        estimate: None,
                        interval_lower: None,
                        interval_upper: None,
                        interval_width: None,
                        maximum_interval_width: finite(target, "maximum_interval_width").expect("validated width"),
                        disposition: "NOT_QUALIFIED_CENSORED_OBSERVATIONS".to_owned(),
                    });
                }
            } else {
                exact_values.sort_by(f64::total_cmp);
                for target in sample_set["quantile_targets"].as_array().expect("validated quantile targets") {
                    let probability = finite(target, "probability").expect("validated probability");
                    let confidence = finite(target, "confidence_level").expect("validated confidence");
                    let maximum_width = finite(target, "maximum_interval_width").expect("validated max width");
                    let estimate = empirical_quantile(&exact_values, probability);
                    let interval = nonparametric_quantile_interval(
                        &exact_values,
                        probability,
                        confidence,
                        finite(sample_set, "domain_lower_bound").expect("validated lower domain"),
                        sample_set.get("domain_upper_bound").and_then(Value::as_f64),
                    );
                    match interval {
                        Some((achieved, lower, upper)) => {
                            let width = upper - lower;
                            let pass = achieved + TOLERANCE >= confidence
                                && width <= maximum_width + TOLERANCE
                                && maximum_width + TOLERANCE >= descriptor_resolution;
                            all_quantiles_pass &= pass;
                            quantile_reports.push(QuantileReport {
                                probability,
                                confidence_target: confidence,
                                achieved_confidence: achieved,
                                estimate: Some(estimate),
                                interval_lower: Some(lower),
                                interval_upper: Some(upper),
                                interval_width: Some(width),
                                maximum_interval_width: maximum_width,
                                disposition: if pass {
                                    "PASS"
                                } else if width > maximum_width + TOLERANCE {
                                    "FAIL_INTERVAL_TOO_WIDE"
                                } else {
                                    "FAIL_CONFIDENCE"
                                }
                                .to_owned(),
                            });
                        }
                        None => {
                            all_quantiles_pass = false;
                            quantile_reports.push(QuantileReport {
                                probability,
                                confidence_target: confidence,
                                achieved_confidence: 0.0,
                                estimate: Some(estimate),
                                interval_lower: None,
                                interval_upper: None,
                                interval_width: None,
                                maximum_interval_width: maximum_width,
                                disposition: "INSUFFICIENT_SAMPLE_FOR_FINITE_INTERVAL".to_owned(),
                            });
                        }
                    }
                }
            }

            let all_stability_pass = sequential_reports
                .iter()
                .all(|report| report.disposition == "PASS_TERMINAL_STABILITY");
            let disposition = if left_censored + right_censored > 0 {
                "NOT_QUALIFIED_CENSORED_OBSERVATIONS"
            } else if all_quantiles_pass && all_stability_pass {
                "PASS_SAMPLE_ADEQUACY_AND_TERMINAL_STABILITY"
            } else if !all_stability_pass {
                "FAIL_SEQUENTIAL_STABILITY"
            } else {
                "FAIL_SAMPLE_ADEQUACY"
            };
            SampleSetReport {
                sample_set_id: text(sample_set, "sample_set_id").expect("validated sample set ID").to_owned(),
                parent_bin_index: uint(sample_set, "parent_bin_index").expect("validated parent bin") as usize,
                descriptor_type: text(sample_set, "descriptor_type").expect("validated descriptor").to_owned(),
                sample_count: samples.len(),
                batch_count,
                sequential_stability: sequential_reports,
                exact_sample_count: exact_values.len(),
                left_censored_count: left_censored,
                right_censored_count: right_censored,
                minimum_observed_feature_mm: if min_feature.is_finite() { Some(min_feature) } else { None },
                quantiles: quantile_reports,
                disposition: disposition.to_owned(),
            }
        })
        .collect()
}

/// Evaluate reference-material repeatability and resolution-aware quantile
/// precision. Passing applies only to the declared method, range and sample
/// design; synthetic evidence remains synthetic.
pub fn evaluate_against_parent(
    case: &Value,
    parent: &Value,
) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_against_parent(case, parent);
    if !errors.is_empty() {
        return Err(errors);
    }
    let reference_checks = reference_reports(case);
    let sample_sets = sample_set_reports(case);
    let all_references_pass = reference_checks.iter().all(|report| report.disposition == "PASS");
    let all_samples_pass = sample_sets.iter().all(|report| report.disposition == "PASS_SAMPLE_ADEQUACY");
    let overall = if all_references_pass && all_samples_pass {
        "PASS_FOR_DECLARED_SCOPE"
    } else {
        "NOT_QUALIFIED"
    };

    Ok(EvaluationReport {
        case_id: text(case, "case_id").expect("validated case ID").to_owned(),
        instrument_id: text(&case["instrument"], "instrument_id").expect("validated instrument").to_owned(),
        parent_morphology_case_id: text(&case["protocol"], "parent_morphology_case_id")
            .expect("validated parent morphology").to_owned(),
        parent_interval_id: text(&case["protocol"], "parent_interval_id")
            .expect("validated parent interval").to_owned(),
        reference_checks,
        sample_sets,
        evidence_disposition: if text(case, "evidence_class") == Some("SYNTHETIC_CONFORMANCE_FIXTURE") {
            "SYNTHETIC_ONLY_NOT_PHYSICAL_QUALIFICATION"
        } else {
            "MEASURED_INPUTS_REVIEW_REQUIRED"
        }
        .to_owned(),
        overall_disposition: overall.to_owned(),
        claim_boundary: "QUALIFICATION_IS_LIMITED_TO_THE_DECLARED_REFERENCE_METHOD_SIZE_RANGE_AND_IID_SAMPLE_DESIGN;_NOT_ISO_CERTIFICATION_OR_FIELD_PROOF".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parent() -> Value {
        serde_json::from_str(include_str!(
            "../fixtures/parent-morphology-case.v0_1.json"
        ))
        .expect("parent morphology fixture parses")
    }

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-measurement-qualification-case.v0_1.json"
        ))
        .expect("measurement fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn first_sample_set_mut(case: &mut Value) -> &mut Value {
        &mut case["sample_sets"][0]
    }

    fn evaluate() -> EvaluationReport {
        evaluate_against_parent(&fixture(), &parent()).expect("synthetic fixture evaluates")
    }

    #[test]
    fn synthetic_case_validates_against_exact_morphology_parent() {
        assert!(validate_against_parent(&fixture(), &parent()).is_empty());
    }

    #[test]
    fn reference_replicates_cover_sessions_operators_and_orientations() {
        let report = evaluate();
        let reference = &report.reference_checks[0];
        assert_eq!(reference.replicate_count, 6);
        assert_eq!(reference.session_count, 3);
        assert_eq!(reference.operator_count, 3);
        assert_eq!(reference.orientation_count, 6);
    }

    #[test]
    fn reference_bias_envelope_is_conservative_and_passes_fixture() {
        let reference = &evaluate().reference_checks[0];
        assert!(reference.absolute_bias > 0.0);
        assert!(reference.repeatability_envelope > 0.0);
        assert!((reference.conservative_error_envelope - 0.0175).abs() < 1.0e-9);
        assert_eq!(reference.coverage_factor, 2.0);
        assert!(reference.conservative_error_envelope < reference.maximum_abs_error);
        assert_eq!(reference.disposition, "PASS");
    }

    #[test]
    fn quantiles_use_exact_distribution_free_order_statistic_intervals() {
        let report = evaluate();
        let set = &report.sample_sets[0];
        assert_eq!(set.sample_count, 48);
        assert_eq!(set.quantiles.len(), 3);
        for quantile in &set.quantiles {
            assert!(quantile.achieved_confidence + TOLERANCE >= quantile.confidence_target);
            assert!(quantile.interval_width.is_some_and(|width| width >= 0.0));
        }
        assert!((set.quantiles[0].interval_lower.unwrap() - 0.42).abs() < 1.0e-9);
        assert!((set.quantiles[0].interval_upper.unwrap() - 0.475).abs() < 1.0e-9);
        assert!((set.quantiles[0].achieved_confidence - 0.9052716318374154).abs() < 1.0e-9);
        assert!((set.quantiles[1].interval_lower.unwrap() - 0.60).abs() < 1.0e-9);
        assert!((set.quantiles[1].interval_upper.unwrap() - 0.715).abs() < 1.0e-9);
        assert!((set.quantiles[1].achieved_confidence - 0.9145668668426059).abs() < 1.0e-9);
        assert!((set.quantiles[2].interval_lower.unwrap() - 0.845).abs() < 1.0e-9);
        assert!((set.quantiles[2].interval_upper.unwrap() - 0.915).abs() < 1.0e-9);
    }

    #[test]
    fn sequential_tail_stability_requires_the_predeclared_terminal_batches() {
        let report = evaluate();
        let set = &report.sample_sets[0];
        assert_eq!(set.batch_count, 6);
        assert_eq!(set.sequential_stability.len(), 3);
        assert!(set.sequential_stability.iter().all(|item| item.disposition == "PASS_TERMINAL_STABILITY"));
        assert_eq!(set.sequential_stability[1].last_instability_batch_index, Some(3));
        assert_eq!(set.sequential_stability[1].terminal_stable_transition_count, 2);
    }

    #[test]
    fn one_late_quantile_shift_resets_terminal_stability() {
        let mut case = fixture();
        for sample in case["sample_sets"][0]["samples"].as_array_mut().unwrap() {
            if sample["batch_index"] == 5 {
                sample["value"] = serde_json::json!(0.99);
            }
        }
        let report = evaluate_against_parent(&case, &parent()).unwrap();
        assert_eq!(report.sample_sets[0].sequential_stability[2].terminal_stable_transition_count, 0);
        assert_eq!(report.sample_sets[0].sequential_stability[2].disposition, "FAIL_TERMINAL_STABILITY");
        assert_eq!(report.overall_disposition, "NOT_QUALIFIED");
    }

    #[test]
    fn a_longer_stable_tail_can_fail_after_a_shorter_tail_passes() {
        let mut case = fixture();
        case["sample_sets"][0]["sequential_stability"]["stable_tail_batch_count"] =
            serde_json::json!(3);
        let report = evaluate_against_parent(&case, &parent()).unwrap();
        assert_eq!(report.sample_sets[0].sequential_stability[1].terminal_stable_transition_count, 2);
        assert_eq!(report.sample_sets[0].sequential_stability[1].disposition, "FAIL_TERMINAL_STABILITY");
    }

    #[test]
    fn noncontiguous_or_unsorted_batches_are_rejected() {
        let mut case = fixture();
        case["sample_sets"][0]["samples"][16]["batch_index"] = serde_json::json!(4);
        assert!(has(&validate_against_parent(&case, &parent()), "NONCONTIGUOUS_OR_UNSORTED_SAMPLE_BATCH"));
    }

    #[test]
    fn fixture_remains_synthetic_even_when_all_declared_gates_pass() {
        let report = evaluate();
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY_NOT_PHYSICAL_QUALIFICATION");
        assert_eq!(report.overall_disposition, "PASS_FOR_DECLARED_SCOPE");
    }

    #[test]
    fn duplicate_particle_id_is_rejected() {
        let mut case = fixture();
        let duplicate = case["sample_sets"][0]["samples"][0]["particle_id"].clone();
        case["sample_sets"][0]["samples"][1]["particle_id"] = duplicate;
        assert!(has(&validate_against_parent(&case, &parent()), "MISSING_OR_DUPLICATE_PARTICLE_ID"));
    }

    #[test]
    fn particle_size_interval_must_fit_exact_parent_bin() {
        let mut case = fixture();
        case["sample_sets"][0]["samples"][0]["particle_size_lower_mm"] = serde_json::json!(4.9);
        case["sample_sets"][0]["samples"][0]["particle_size_upper_mm"] = serde_json::json!(5.1);
        assert!(has(
            &validate_against_parent(&case, &parent()),
            "PARTICLE_SIZE_INTERVAL_CROSSES_PARENT_BIN"
        ));
    }

    #[test]
    fn duplicate_particle_source_artifact_is_rejected() {
        let mut case = fixture();
        let source = case["sample_sets"][0]["samples"][0]["source_artifact_ref"].clone();
        case["sample_sets"][0]["samples"][1]["source_artifact_ref"] = source;
        assert!(has(
            &validate_against_parent(&case, &parent()),
            "DUPLICATE_PARTICLE_SOURCE_ARTIFACT"
        ));
    }

    #[test]
    fn each_particle_requires_raw_and_segmentation_provenance() {
        let mut case = fixture();
        case["sample_sets"][0]["samples"][0]
            .as_object_mut().unwrap().remove("source_artifact_ref");
        assert!(has(
            &validate_against_parent(&case, &parent()),
            "MISSING_PARTICLE_ARTIFACT_PROVENANCE"
        ));
    }

    #[test]
    fn repeated_acquisition_block_is_not_treated_as_iid() {
        let mut case = fixture();
        let block = case["sample_sets"][0]["samples"][0]["acquisition_block_ref"].clone();
        case["sample_sets"][0]["samples"][1]["acquisition_block_ref"] = block;
        assert!(has(&validate_against_parent(&case, &parent()), "DUPLICATE_ACQUISITION_BLOCK"));
    }

    #[test]
    fn censored_observation_is_not_zero_filled() {
        let mut case = fixture();
        let sample = &mut case["sample_sets"][0]["samples"][0];
        sample["resolution_status"] = serde_json::json!("LEFT_CENSORED");
        sample.as_object_mut().unwrap().remove("value");
        sample["censor_bound"] = serde_json::json!(0.02);
        let report = evaluate_against_parent(&case, &parent()).unwrap();
        assert_eq!(report.sample_sets[0].left_censored_count, 1);
        assert_eq!(report.sample_sets[0].disposition, "NOT_QUALIFIED_CENSORED_OBSERVATIONS");
    }

    #[test]
    fn censored_observation_cannot_claim_an_exact_value() {
        let mut case = fixture();
        case["sample_sets"][0]["samples"][0]["resolution_status"] =
            serde_json::json!("LEFT_CENSORED");
        case["sample_sets"][0]["samples"][0]["censor_bound"] = serde_json::json!(0.01);
        assert!(has(&validate_against_parent(&case, &parent()), "CENSORED_SAMPLE_HAS_EXACT_VALUE"));
    }

    #[test]
    fn sample_below_spatial_resolution_is_rejected() {
        let mut case = fixture();
        case["sample_sets"][0]["samples"][0]["feature_dimension_mm"] =
            serde_json::json!(0.01);
        assert!(has(&validate_against_parent(&case, &parent()), "SAMPLE_BELOW_SPATIAL_RESOLUTION"));
    }

    #[test]
    fn reference_below_spatial_resolution_is_rejected() {
        let mut case = fixture();
        case["reference_checks"][0]["feature_dimension_mm"] =
            serde_json::json!(0.01);
        assert!(has(&validate_against_parent(&case, &parent()), "REFERENCE_BELOW_SPATIAL_RESOLUTION"));
    }

    #[test]
    fn descriptor_resolution_must_match_instrument() {
        let mut case = fixture();
        first_sample_set_mut(&mut case)["descriptor_resolution"] =
            serde_json::json!(0.01);
        assert!(has(&validate_against_parent(&case, &parent()), "DESCRIPTOR_RESOLUTION_MISMATCH"));
    }

    #[test]
    fn p05_precision_gate_can_fail_even_with_many_samples() {
        let mut case = fixture();
        case["sample_sets"][0]["quantile_targets"][0]["maximum_interval_width"] =
            serde_json::json!(0.01);
        let report = evaluate_against_parent(&case, &parent()).unwrap();
        assert_ne!(report.sample_sets[0].quantiles[0].disposition, "PASS");
        assert_eq!(report.overall_disposition, "NOT_QUALIFIED");
    }

    #[test]
    fn sample_size_cannot_exceed_parent_morphology_descriptor_count() {
        let mut case = fixture();
        let duplicate = case["sample_sets"][0]["samples"][0].clone();
        case["sample_sets"][0]["samples"].as_array_mut().unwrap().push(duplicate);
        assert!(has(&validate_against_parent(&case, &parent()), "SAMPLE_COUNT_EXCEEDS_PARENT_DECLARATION"));
    }

    #[test]
    fn parent_interval_must_match() {
        let mut case = fixture();
        case["protocol"]["parent_interval_id"] = serde_json::json!("OTHER-INTERVAL");
        assert!(has(&validate_against_parent(&case, &parent()), "PARENT_MORPHOLOGY_INTERVAL_MISMATCH"));
    }

    #[test]
    fn two_d_method_cannot_qualify_a_3d_reference() {
        let mut case = fixture();
        case["reference_checks"][0]["descriptor_type"] = serde_json::json!("SPHERICITY_3D");
        case["reference_checks"][0]["basis"] = serde_json::json!("RECONSTRUCTION_3D");
        assert!(has(&validate_against_parent(&case, &parent()), "METHOD_CANNOT_SUPPORT_3D"));
    }

    #[test]
    fn reference_replicates_need_multiple_sessions_operators_and_orientations() {
        let mut case = fixture();
        for replicate in case["reference_checks"][0]["replicates"].as_array_mut().unwrap() {
            replicate["session_ref"] = serde_json::json!("one-session");
        }
        assert!(has(&validate_against_parent(&case, &parent()), "INSUFFICIENT_REFERENCE_REPRODUCIBILITY_COVERAGE"));
    }

    #[test]
    fn reference_bias_can_fail_declared_error_limit_without_invalidating_schema() {
        let mut case = fixture();
        case["reference_checks"][0]["maximum_abs_error"] = serde_json::json!(0.001);
        let report = evaluate_against_parent(&case, &parent()).unwrap();
        assert_eq!(report.reference_checks[0].disposition, "FAIL_REFERENCE_ERROR_ENVELOPE");
        assert_eq!(report.overall_disposition, "NOT_QUALIFIED");
    }

    #[test]
    fn quantile_domain_must_include_all_sample_values() {
        let mut case = fixture();
        first_sample_set_mut(&mut case)["domain_upper_bound"] = serde_json::json!(0.5);
        assert!(has(&validate_against_parent(&case, &parent()), "INVALID_EXACT_SAMPLE_VALUE"));
    }

    #[test]
    fn one_sample_set_must_reference_a_parent_descriptor_definition() {
        let mut case = fixture();
        first_sample_set_mut(&mut case)["descriptor_definition_ref"] =
            serde_json::json!("unrelated-definition");
        assert!(has(&validate_against_parent(&case, &parent()), "PARENT_MORPHOLOGY_DESCRIPTOR_NOT_FOUND"));
    }

    #[test]
    fn exact_replay_is_deterministic() {
        assert_eq!(evaluate(), evaluate());
    }
}
