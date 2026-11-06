//! Measurement-qualified, size-conditioned particle morphology validation.
//!
//! The parent fragment/transport case remains authoritative for PSD boundaries
//! and mass. This crate validates morphology samples against the exact parent
//! case + interval + trial + machine configuration + material cohort. It does
//! not predict transport or replace independent physical measurements.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DescriptorDistribution {
    pub descriptor_type: String,
    pub basis: String,
    pub sample_count: u64,
    pub p05: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p95: f64,
    pub p50_standard_uncertainty: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SizeStratumReport {
    pub parent_bin_index: usize,
    pub lower_mm: f64,
    pub upper_mm: Option<f64>,
    pub sampled_particle_count: u64,
    pub valid_particle_count: u64,
    pub occluded_particle_count: u64,
    pub ambiguous_particle_count: u64,
    pub rejected_particle_count: u64,
    pub descriptors: Vec<DescriptorDistribution>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphologyReport {
    pub case_id: String,
    pub parent_case_id: String,
    pub parent_interval_id: String,
    pub trial_id: String,
    pub configuration_ref: String,
    pub material_cohort_ref: String,
    pub acquisition_method: String,
    pub orientation_protocol: String,
    pub parent_psd_bin_count: usize,
    pub sampled_psd_bin_count: usize,
    pub unsampled_parent_bin_indices: Vec<usize>,
    pub size_bin_coverage_fraction: f64,
    pub sampled_particle_count: u64,
    pub valid_particle_count: u64,
    pub occluded_particle_count: u64,
    pub ambiguous_particle_count: u64,
    pub rejected_particle_count: u64,
    pub orientation_bias_warning: bool,
    pub warnings: Vec<String>,
    pub strata: Vec<SizeStratumReport>,
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
        .filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn count(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn same_upper(value: &Value, key: &str, parent: &Value) -> bool {
    match (value.get(key), parent.get(key)) {
        (Some(Value::Null), Some(Value::Null)) => true,
        (Some(a), Some(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() => close(x, y),
            _ => false,
        },
        _ => false,
    }
}

fn valid_method(method: &str) -> bool {
    matches!(
        method,
        "STATIC_2D_IMAGE"
            | "DYNAMIC_2D_IMAGE"
            | "LIDAR_POINT_CLOUD"
            | "PHOTOGRAMMETRY_3D"
            | "STRUCTURED_LIGHT_3D"
            | "MULTIVIEW_RECONSTRUCTION"
    )
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

fn descriptor_value_in_domain(descriptor: &str, value: f64) -> bool {
    match descriptor {
        "FERET_ASPECT_RATIO_2D" | "ASPECT_RATIO_3D" => value >= 1.0,
        "CIRCULARITY_2D"
        | "SOLIDITY_2D"
        | "SPHERICITY_3D"
        | "CONVEXITY_3D" => value > 0.0 && value <= 1.0,
        "FOURIER_ANGULARITY_2D" | "FOURIER_ROUGHNESS_2D" => value >= 0.0,
        _ => false,
    }
}

fn validate_descriptor(
    descriptor: &Value,
    path: &str,
    method: &str,
    valid_particles: u64,
    errors: &mut Vec<Violation>,
) {
    let name = text(descriptor, "descriptor_type");
    let basis = text(descriptor, "basis");

    let Some(name) = name else {
        push(
            errors,
            "MISSING_DESCRIPTOR_TYPE",
            &format!("{path}.descriptor_type"),
            "descriptor_type is required",
        );
        return;
    };

    let Some(expected_basis) = descriptor_basis(name) else {
        push(
            errors,
            "UNKNOWN_DESCRIPTOR_TYPE",
            &format!("{path}.descriptor_type"),
            "descriptor type is outside the frozen v0.1 vocabulary",
        );
        return;
    };

    if basis != Some(expected_basis) {
        push(
            errors,
            "DESCRIPTOR_DIMENSION_MISMATCH",
            &format!("{path}.basis"),
            format!(
                "{name} requires basis {expected_basis}; a 2D projection cannot be promoted to a 3D descriptor"
            ),
        );
    }

    if matches!(method, "STATIC_2D_IMAGE" | "DYNAMIC_2D_IMAGE")
        && expected_basis == "RECONSTRUCTION_3D"
    {
        push(
            errors,
            "METHOD_CANNOT_SUPPORT_3D_DESCRIPTOR",
            &format!("{path}.descriptor_type"),
            "a 2D-only acquisition method cannot support a 3D reconstruction descriptor",
        );
    }

    if text(descriptor, "unit") != Some("1") {
        push(
            errors,
            "INVALID_DESCRIPTOR_UNIT",
            &format!("{path}.unit"),
            "v0.1 descriptor values in this vocabulary are dimensionless and require unit '1'",
        );
    }

    if text(descriptor, "descriptor_definition_ref").is_none() {
        push(
            errors,
            "MISSING_DESCRIPTOR_DEFINITION",
            &format!("{path}.descriptor_definition_ref"),
            "descriptor formula/version reference is required for inter-method reproducibility",
        );
    }

    match count(descriptor, "sample_count") {
        Some(n) if n > 0 && n <= valid_particles => {}
        _ => push(
            errors,
            "INVALID_DESCRIPTOR_SAMPLE_COUNT",
            &format!("{path}.sample_count"),
            "descriptor sample_count must be positive and cannot exceed valid particles in its size stratum",
        ),
    }

    match finite(descriptor, "p50_standard_uncertainty") {
        Some(u) if u >= 0.0 => {}
        _ => push(
            errors,
            "INVALID_DESCRIPTOR_UNCERTAINTY",
            &format!("{path}.p50_standard_uncertainty"),
            "p50 standard uncertainty must be finite and non-negative",
        ),
    }

    let Some(quantiles) = descriptor.get("quantiles") else {
        push(
            errors,
            "MISSING_DESCRIPTOR_QUANTILES",
            &format!("{path}.quantiles"),
            "p05/p25/p50/p75/p95 distribution is required; a single mean is insufficient",
        );
        return;
    };

    let keys = ["p05", "p25", "p50", "p75", "p95"];
    let mut values = Vec::with_capacity(keys.len());
    for key in keys {
        match finite(quantiles, key) {
            Some(v) => {
                if !descriptor_value_in_domain(name, v) {
                    push(
                        errors,
                        "DESCRIPTOR_VALUE_OUT_OF_DOMAIN",
                        &format!("{path}.quantiles.{key}"),
                        format!("{name} value is outside its declared domain"),
                    );
                }
                values.push(v);
            }
            None => {
                push(
                    errors,
                    "INVALID_DESCRIPTOR_QUANTILE",
                    &format!("{path}.quantiles.{key}"),
                    "each quantile must be a finite numeric value",
                );
            }
        }
    }

    if values.len() == keys.len() && values.windows(2).any(|pair| pair[0] > pair[1]) {
        push(
            errors,
            "NON_MONOTONIC_DESCRIPTOR_QUANTILES",
            &format!("{path}.quantiles"),
            "p05 <= p25 <= p50 <= p75 <= p95 is required",
        );
    }
}

/// Validate morphology case structure and its exact mapping to one parent
/// fragmentation/transport case. The parent remains the PSD and mass authority.
pub fn validate_against_parent(case: &Value, parent: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(
            &mut errors,
            "UNSUPPORTED_SCHEMA_VERSION",
            "$.schema_version",
            "schema_version must be exactly 1",
        );
    }

    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }

    if !matches!(
        text(case, "evidence_class"),
        Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")
    ) {
        push(
            &mut errors,
            "INVALID_EVIDENCE_CLASS",
            "$.evidence_class",
            "evidence_class must be SYNTHETIC_CONFORMANCE_FIXTURE or MEASURED",
        );
    }

    let Some(acquisition) = case.get("acquisition") else {
        push(
            &mut errors,
            "MISSING_ACQUISITION",
            "$.acquisition",
            "acquisition metadata is required",
        );
        return errors;
    };

    let reference_fields = [
        "acquisition_id",
        "parent_case_id",
        "parent_interval_id",
        "trial_id",
        "configuration_ref",
        "material_cohort_ref",
        "instrument_ref",
        "calibration_ref",
        "reference_material_ref",
        "sampling_protocol_ref",
        "segmentation_pipeline_ref",
        "orientation_protocol_ref",
        "repeatability_ref",
        "source_artifact_ref",
    ];

    for key in reference_fields {
        if text(acquisition, key).is_none() {
            push(
                &mut errors,
                "MISSING_ACQUISITION_PROVENANCE",
                &format!("$.acquisition.{key}"),
                "non-empty acquisition provenance reference is required",
            );
        }
    }

    let Some(method) = text(acquisition, "method") else {
        push(
            &mut errors,
            "MISSING_ACQUISITION_METHOD",
            "$.acquisition.method",
            "acquisition method is required",
        );
        return errors;
    };

    if !valid_method(method) {
        push(
            &mut errors,
            "UNSUPPORTED_ACQUISITION_METHOD",
            "$.acquisition.method",
            "method must be an explicitly supported v0.1 image/point-cloud acquisition path",
        );
    }

    let orientation = text(acquisition, "orientation_protocol");
    if !matches!(
        orientation,
        Some("DYNAMIC_ROTATION"
            | "MULTI_VIEW"
            | "RANDOM_ORIENTATION_SAMPLE"
            | "SINGLE_VIEW_UNCONTROLLED")
    ) {
        push(
            &mut errors,
            "INVALID_ORIENTATION_PROTOCOL",
            "$.acquisition.orientation_protocol",
            "orientation coverage protocol must be explicit",
        );
    }

    let parent_case_id = text(parent, "case_id");
    if text(acquisition, "parent_case_id") != parent_case_id || parent_case_id.is_none() {
        push(
            &mut errors,
            "PARENT_CASE_MISMATCH",
            "$.acquisition.parent_case_id",
            "morphology must reference the exact parent fragment/transport case supplied for evaluation",
        );
    }

    let interval_id = text(acquisition, "parent_interval_id");
    let intervals = parent.get("intervals").and_then(Value::as_array);
    let matching_intervals: Vec<&Value> = intervals
        .into_iter()
        .flatten()
        .filter(|interval| text(interval, "interval_id") == interval_id)
        .collect();

    if matching_intervals.len() != 1 {
        push(
            &mut errors,
            "PARENT_INTERVAL_NOT_UNIQUE",
            "$.acquisition.parent_interval_id",
            "parent interval reference must resolve to exactly one interval",
        );
    }

    let parent_profile_cohort = parent
        .get("profile")
        .and_then(|profile| text(profile, "material_cohort_ref"));

    if text(acquisition, "material_cohort_ref") != parent_profile_cohort
        || parent_profile_cohort.is_none()
    {
        push(
            &mut errors,
            "PARENT_MATERIAL_COHORT_MISMATCH",
            "$.acquisition.material_cohort_ref",
            "morphology material cohort must match the parent profile's exact cohort",
        );
    }

    let parent_interval = matching_intervals.first().copied();
    if let Some(interval) = parent_interval {
        for key in ["trial_id", "configuration_ref"] {
            if text(acquisition, key) != text(interval, key) {
                push(
                    &mut errors,
                    "PARENT_INTERVAL_CONTEXT_MISMATCH",
                    &format!("$.acquisition.{key}"),
                    format!("morphology {key} must match the referenced parent interval"),
                );
            }
        }
    }

    let Some(parent_bins) = parent_interval
        .and_then(|interval| interval.get("fragment_bins"))
        .and_then(Value::as_array)
    else {
        push(
            &mut errors,
            "PARENT_PSD_UNAVAILABLE",
            "$.acquisition.parent_interval_id",
            "referenced parent interval must contain an authoritative fragment_bins array",
        );
        return errors;
    };

    let Some(strata) = case.get("size_strata").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_SIZE_STRATA",
            "$.size_strata",
            "size_strata must be an array",
        );
        return errors;
    };

    if strata.is_empty() {
        push(
            &mut errors,
            "EMPTY_SIZE_STRATA",
            "$.size_strata",
            "at least one measured size stratum is required",
        );
        return errors;
    }

    let mut seen_bins = BTreeSet::new();
    let mut previous_index: Option<usize> = None;

    for (index, stratum) in strata.iter().enumerate() {
        let path = format!("$.size_strata[{index}]");
        let Some(bin_index_u64) = count(stratum, "parent_bin_index") else {
            push(
                &mut errors,
                "INVALID_PARENT_BIN_INDEX",
                &format!("{path}.parent_bin_index"),
                "parent_bin_index must be a non-negative integer",
            );
            continue;
        };
        let Ok(bin_index) = usize::try_from(bin_index_u64) else {
            push(
                &mut errors,
                "INVALID_PARENT_BIN_INDEX",
                &format!("{path}.parent_bin_index"),
                "parent_bin_index exceeds this platform's addressable range",
            );
            continue;
        };

        if !seen_bins.insert(bin_index) {
            push(
                &mut errors,
                "DUPLICATE_PARENT_BIN",
                &format!("{path}.parent_bin_index"),
                "one morphology stratum per parent PSD bin is permitted",
            );
        }
        if previous_index.is_some_and(|previous| bin_index <= previous) {
            push(
                &mut errors,
                "UNSORTED_PARENT_BINS",
                &format!("{path}.parent_bin_index"),
                "morphology strata must follow ascending parent PSD bin index",
            );
        }
        previous_index = Some(bin_index);

        let Some(parent_bin) = parent_bins.get(bin_index) else {
            push(
                &mut errors,
                "PARENT_BIN_OUT_OF_RANGE",
                &format!("{path}.parent_bin_index"),
                "parent_bin_index does not exist in the referenced parent PSD",
            );
            continue;
        };

        match (finite(stratum, "lower_mm"), finite(parent_bin, "lower_mm")) {
            (Some(actual), Some(expected)) if close(actual, expected) => {}
            _ => push(
                &mut errors,
                "PARENT_BIN_LOWER_BOUND_MISMATCH",
                &format!("{path}.lower_mm"),
                "morphology stratum lower_mm must exactly match the referenced parent PSD bin",
            ),
        }

        if !same_upper(stratum, "upper_mm", parent_bin) {
            push(
                &mut errors,
                "PARENT_BIN_UPPER_BOUND_MISMATCH",
                &format!("{path}.upper_mm"),
                "morphology stratum upper_mm (including final open-ended null) must match the parent PSD bin",
            );
        }

        let sampled = count(stratum, "sampled_particle_count");
        let valid = count(stratum, "valid_particle_count");
        let occluded = count(stratum, "occluded_particle_count");
        let ambiguous = count(stratum, "ambiguous_particle_count");
        let rejected = count(stratum, "rejected_particle_count");

        match (sampled, valid, occluded, ambiguous, rejected) {
            (Some(s), Some(v), Some(o), Some(a), Some(r)) if v > 0 => {
                let partition = v
                    .checked_add(o)
                    .and_then(|n| n.checked_add(a))
                    .and_then(|n| n.checked_add(r));
                if partition != Some(s) {
                    push(
                        &mut errors,
                        "PARTICLE_SAMPLE_ACCOUNTING_MISMATCH",
                        &path,
                        "sampled count must equal valid + occluded + ambiguous + rejected counts",
                    );
                }
            }
            _ => push(
                &mut errors,
                "INVALID_PARTICLE_SAMPLE_COUNTS",
                &path,
                "all counts must be non-negative integers and valid_particle_count must be positive",
            ),
        }

        let role = text(stratum, "evidence_role");
        if !matches!(
            role,
            Some("INDEPENDENT_MEASUREMENT" | "DERIVED_FROM_INDEPENDENT_MEASUREMENTS")
        ) {
            push(
                &mut errors,
                "NON_INDEPENDENT_MORPHOLOGY_EVIDENCE",
                &format!("{path}.evidence_role"),
                "only independent measurement or a declared derivation from independent measurement is admissible",
            );
        }
        if role == Some("DERIVED_FROM_INDEPENDENT_MEASUREMENTS")
            && text(stratum, "lineage_ref").is_none()
        {
            push(
                &mut errors,
                "MISSING_MORPHOLOGY_LINEAGE",
                &format!("{path}.lineage_ref"),
                "derived descriptors require lineage to the independent image/point-cloud observations",
            );
        }
        if text(stratum, "source_ref").is_none() {
            push(
                &mut errors,
                "MISSING_STRATUM_SOURCE",
                &format!("{path}.source_ref"),
                "source observation reference is required",
            );
        }

        let Some(descriptors) = stratum.get("descriptors").and_then(Value::as_array) else {
            push(
                &mut errors,
                "MISSING_DESCRIPTORS",
                &format!("{path}.descriptors"),
                "descriptors must be an array",
            );
            continue;
        };

        if descriptors.is_empty() {
            push(
                &mut errors,
                "EMPTY_DESCRIPTORS",
                &format!("{path}.descriptors"),
                "at least one distribution descriptor is required for a sampled stratum",
            );
        }

        let valid_count = valid.unwrap_or(0);
        let mut descriptor_ids = BTreeSet::new();
        for (descriptor_index, descriptor) in descriptors.iter().enumerate() {
            let descriptor_path = format!("{path}.descriptors[{descriptor_index}]");
            let descriptor_id = text(descriptor, "descriptor_type");
            if let Some(id) = descriptor_id {
                if !descriptor_ids.insert(id.to_owned()) {
                    push(
                        &mut errors,
                        "DUPLICATE_DESCRIPTOR",
                        &format!("{descriptor_path}.descriptor_type"),
                        "descriptor type may appear only once per size stratum",
                    );
                }
            }
            validate_descriptor(
                descriptor,
                &descriptor_path,
                method,
                valid_count,
                &mut errors,
            );
        }
    }

    errors
}

/// Evaluate a valid morphology case against its authoritative parent PSD.
/// Unsampled PSD bins are returned explicitly; no morphology is interpolated.
pub fn evaluate_against_parent(
    case: &Value,
    parent: &Value,
) -> Result<MorphologyReport, Vec<Violation>> {
    let errors = validate_against_parent(case, parent);
    if !errors.is_empty() {
        return Err(errors);
    }

    let acquisition = case.get("acquisition").expect("validated acquisition");
    let interval_id = text(acquisition, "parent_interval_id").expect("validated parent interval");
    let parent_interval = parent["intervals"]
        .as_array()
        .expect("validated parent intervals")
        .iter()
        .find(|interval| text(interval, "interval_id") == Some(interval_id))
        .expect("validated parent interval identity");
    let parent_bins = parent_interval["fragment_bins"]
        .as_array()
        .expect("validated parent bins");
    let strata = case["size_strata"]
        .as_array()
        .expect("validated morphology strata");

    let mut reports = Vec::with_capacity(strata.len());
    let mut sampled_indices = BTreeSet::new();
    let mut totals = [0_u64; 5];

    for stratum in strata {
        let bin_index = count(stratum, "parent_bin_index")
            .expect("validated bin index") as usize;
        sampled_indices.insert(bin_index);

        let counts = [
            count(stratum, "sampled_particle_count").expect("validated sampled count"),
            count(stratum, "valid_particle_count").expect("validated valid count"),
            count(stratum, "occluded_particle_count").expect("validated occluded count"),
            count(stratum, "ambiguous_particle_count").expect("validated ambiguous count"),
            count(stratum, "rejected_particle_count").expect("validated rejected count"),
        ];
        for (total, value) in totals.iter_mut().zip(counts) {
            *total = total.saturating_add(value);
        }

        let mut descriptor_reports = Vec::new();
        for descriptor in stratum["descriptors"].as_array().expect("validated descriptors") {
            let quantiles = &descriptor["quantiles"];
            descriptor_reports.push(DescriptorDistribution {
                descriptor_type: text(descriptor, "descriptor_type")
                    .expect("validated descriptor type")
                    .to_owned(),
                basis: text(descriptor, "basis")
                    .expect("validated descriptor basis")
                    .to_owned(),
                sample_count: count(descriptor, "sample_count")
                    .expect("validated descriptor sample count"),
                p05: finite(quantiles, "p05").expect("validated p05"),
                p25: finite(quantiles, "p25").expect("validated p25"),
                p50: finite(quantiles, "p50").expect("validated p50"),
                p75: finite(quantiles, "p75").expect("validated p75"),
                p95: finite(quantiles, "p95").expect("validated p95"),
                p50_standard_uncertainty: finite(descriptor, "p50_standard_uncertainty")
                    .expect("validated p50 uncertainty"),
            });
        }

        let parent_bin = &parent_bins[bin_index];
        reports.push(SizeStratumReport {
            parent_bin_index: bin_index,
            lower_mm: finite(parent_bin, "lower_mm").expect("validated parent lower bin"),
            upper_mm: parent_bin.get("upper_mm").and_then(Value::as_f64),
            sampled_particle_count: counts[0],
            valid_particle_count: counts[1],
            occluded_particle_count: counts[2],
            ambiguous_particle_count: counts[3],
            rejected_particle_count: counts[4],
            descriptors: descriptor_reports,
        });
    }

    let unsampled: Vec<usize> = (0..parent_bins.len())
        .filter(|index| !sampled_indices.contains(index))
        .collect();
    let orientation_protocol =
        text(acquisition, "orientation_protocol").expect("validated orientation protocol");
    let orientation_bias_warning = orientation_protocol == "SINGLE_VIEW_UNCONTROLLED";
    let warnings = if orientation_bias_warning {
        vec![
            "SINGLE_VIEW_UNCONTROLLED: observed 2D descriptors may be orientation-biased and do not establish a representative 3D shape distribution".to_owned(),
        ]
    } else {
        Vec::new()
    };

    Ok(MorphologyReport {
        case_id: text(case, "case_id").expect("validated case id").to_owned(),
        parent_case_id: text(acquisition, "parent_case_id")
            .expect("validated parent case id")
            .to_owned(),
        parent_interval_id: interval_id.to_owned(),
        trial_id: text(acquisition, "trial_id")
            .expect("validated trial id")
            .to_owned(),
        configuration_ref: text(acquisition, "configuration_ref")
            .expect("validated configuration")
            .to_owned(),
        material_cohort_ref: text(acquisition, "material_cohort_ref")
            .expect("validated cohort")
            .to_owned(),
        acquisition_method: text(acquisition, "method")
            .expect("validated method")
            .to_owned(),
        orientation_protocol: orientation_protocol.to_owned(),
        parent_psd_bin_count: parent_bins.len(),
        sampled_psd_bin_count: sampled_indices.len(),
        unsampled_parent_bin_indices: unsampled,
        size_bin_coverage_fraction: sampled_indices.len() as f64 / parent_bins.len() as f64,
        sampled_particle_count: totals[0],
        valid_particle_count: totals[1],
        occluded_particle_count: totals[2],
        ambiguous_particle_count: totals[3],
        rejected_particle_count: totals[4],
        orientation_bias_warning,
        warnings,
        strata: reports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parent_case() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-fragment-transport-case.v0_1.json"
        ))
        .expect("parent fixture parses")
    }

    fn morphology_case() -> Value {
        serde_json::from_str(include_str!(
            "../fixtures/synthetic-fragment-morphology-case.v0_1.json"
        ))
        .expect("morphology fixture parses")
    }

    fn stratum_mut(case: &mut Value, index: usize) -> &mut Value {
        &mut case["size_strata"][index]
    }

    fn first_descriptor_mut(case: &mut Value, stratum: usize) -> &mut Value {
        &mut case["size_strata"][stratum]["descriptors"][0]
    }

    fn has_code(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    #[test]
    fn synthetic_case_validates_against_exact_parent_fixture() {
        assert!(validate_against_parent(&morphology_case(), &parent_case()).is_empty());
    }

    #[test]
    fn structural_replay_reports_size_conditioned_shape_distributions() {
        let report = evaluate_against_parent(&morphology_case(), &parent_case())
            .expect("synthetic morphology evaluates");
        assert_eq!(report.parent_psd_bin_count, 4);
        assert_eq!(report.sampled_psd_bin_count, 4);
        assert!(report.unsampled_parent_bin_indices.is_empty());
        assert_eq!(report.size_bin_coverage_fraction, 1.0);
        assert_eq!(report.sampled_particle_count, 208);
        assert_eq!(report.valid_particle_count, 192);
        assert_eq!(report.occluded_particle_count, 8);
        assert_eq!(report.ambiguous_particle_count, 4);
        assert_eq!(report.rejected_particle_count, 4);
        assert!(!report.orientation_bias_warning);
        assert_eq!(report.strata.len(), 4);
    }

    #[test]
    fn morphology_bins_reference_exact_parent_psd_boundaries() {
        let report = evaluate_against_parent(&morphology_case(), &parent_case())
            .expect("exact PSD mapping");
        assert_eq!(report.strata[0].lower_mm, 0.0);
        assert_eq!(report.strata[0].upper_mm, Some(5.0));
        assert_eq!(report.strata[3].lower_mm, 20.0);
        assert_eq!(report.strata[3].upper_mm, None);
    }

    #[test]
    fn parent_case_identity_mismatch_is_rejected() {
        let mut case = morphology_case();
        case["acquisition"]["parent_case_id"] = serde_json::json!("OTHER-PARENT");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "PARENT_CASE_MISMATCH"
        ));
    }

    #[test]
    fn parent_interval_must_resolve_uniquely() {
        let mut case = morphology_case();
        case["acquisition"]["parent_interval_id"] = serde_json::json!("DOES-NOT-EXIST");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "PARENT_INTERVAL_NOT_UNIQUE"
        ));
    }

    #[test]
    fn duplicate_parent_interval_ids_are_rejected() {
        let mut parent = parent_case();
        let first = parent["intervals"][0].clone();
        parent["intervals"].as_array_mut().unwrap().push(first);
        assert!(has_code(
            &validate_against_parent(&morphology_case(), &parent),
            "PARENT_INTERVAL_NOT_UNIQUE"
        ));
    }

    #[test]
    fn trial_identity_must_match_parent_interval() {
        let mut case = morphology_case();
        case["acquisition"]["trial_id"] = serde_json::json!("OTHER-TRIAL");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "PARENT_INTERVAL_CONTEXT_MISMATCH"
        ));
    }

    #[test]
    fn machine_configuration_must_match_parent_interval() {
        let mut case = morphology_case();
        case["acquisition"]["configuration_ref"] = serde_json::json!("OTHER-MACHINE");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "PARENT_INTERVAL_CONTEXT_MISMATCH"
        ));
    }

    #[test]
    fn material_cohort_must_match_parent_profile() {
        let mut case = morphology_case();
        case["acquisition"]["material_cohort_ref"] = serde_json::json!("OTHER-COHORT");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "PARENT_MATERIAL_COHORT_MISMATCH"
        ));
    }

    #[test]
    fn source_bin_index_out_of_range_is_rejected() {
        let mut case = morphology_case();
        stratum_mut(&mut case, 0)["parent_bin_index"] = serde_json::json!(99);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "PARENT_BIN_OUT_OF_RANGE"
        ));
    }

    #[test]
    fn duplicate_parent_bin_is_rejected() {
        let mut case = morphology_case();
        stratum_mut(&mut case, 1)["parent_bin_index"] = serde_json::json!(0);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "DUPLICATE_PARENT_BIN"
        ));
    }

    #[test]
    fn strata_must_be_in_parent_bin_order() {
        let mut case = morphology_case();
        case["size_strata"].as_array_mut().unwrap().swap(0, 1);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "UNSORTED_PARENT_BINS"
        ));
    }

    #[test]
    fn lower_size_boundary_must_match_parent_bin() {
        let mut case = morphology_case();
        stratum_mut(&mut case, 0)["lower_mm"] = serde_json::json!(0.25);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "PARENT_BIN_LOWER_BOUND_MISMATCH"
        ));
    }

    #[test]
    fn upper_size_boundary_must_match_parent_bin() {
        let mut case = morphology_case();
        stratum_mut(&mut case, 3)["upper_mm"] = serde_json::json!(100.0);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "PARENT_BIN_UPPER_BOUND_MISMATCH"
        ));
    }

    #[test]
    fn sample_population_must_reconcile() {
        let mut case = morphology_case();
        stratum_mut(&mut case, 0)["sampled_particle_count"] = serde_json::json!(999);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "PARTICLE_SAMPLE_ACCOUNTING_MISMATCH"
        ));
    }

    #[test]
    fn zero_valid_particles_is_rejected() {
        let mut case = morphology_case();
        stratum_mut(&mut case, 0)["valid_particle_count"] = serde_json::json!(0);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "INVALID_PARTICLE_SAMPLE_COUNTS"
        ));
    }

    #[test]
    fn model_estimated_shape_cannot_become_measurement_truth() {
        let mut case = morphology_case();
        stratum_mut(&mut case, 0)["evidence_role"] = serde_json::json!("MODEL_ESTIMATE");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "NON_INDEPENDENT_MORPHOLOGY_EVIDENCE"
        ));
    }

    #[test]
    fn derived_morphology_requires_measurement_lineage() {
        let mut case = morphology_case();
        stratum_mut(&mut case, 0)["evidence_role"] =
            serde_json::json!("DERIVED_FROM_INDEPENDENT_MEASUREMENTS");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "MISSING_MORPHOLOGY_LINEAGE"
        ));
    }

    #[test]
    fn descriptor_sample_count_cannot_exceed_valid_particles() {
        let mut case = morphology_case();
        first_descriptor_mut(&mut case, 0)["sample_count"] = serde_json::json!(999);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "INVALID_DESCRIPTOR_SAMPLE_COUNT"
        ));
    }

    #[test]
    fn duplicate_descriptor_type_is_rejected() {
        let mut case = morphology_case();
        let first = case["size_strata"][0]["descriptors"][0].clone();
        case["size_strata"][0]["descriptors"]
            .as_array_mut()
            .unwrap()
            .push(first);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "DUPLICATE_DESCRIPTOR"
        ));
    }

    #[test]
    fn quantiles_must_be_monotonic() {
        let mut case = morphology_case();
        first_descriptor_mut(&mut case, 0)["quantiles"]["p95"] = serde_json::json!(0.1);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "NON_MONOTONIC_DESCRIPTOR_QUANTILES"
        ));
    }

    #[test]
    fn aspect_ratio_must_be_at_least_one() {
        let mut case = morphology_case();
        first_descriptor_mut(&mut case, 0)["quantiles"]["p50"] = serde_json::json!(0.8);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "DESCRIPTOR_VALUE_OUT_OF_DOMAIN"
        ));
    }

    #[test]
    fn circularity_cannot_exceed_one() {
        let mut case = morphology_case();
        case["size_strata"][0]["descriptors"][1]["quantiles"]["p95"] =
            serde_json::json!(1.2);
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "DESCRIPTOR_VALUE_OUT_OF_DOMAIN"
        ));
    }

    #[test]
    fn two_dimensional_descriptor_cannot_claim_3d_basis() {
        let mut case = morphology_case();
        first_descriptor_mut(&mut case, 0)["basis"] =
            serde_json::json!("RECONSTRUCTION_3D");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "DESCRIPTOR_DIMENSION_MISMATCH"
        ));
    }

    #[test]
    fn two_dimensional_method_cannot_emit_sphericity_3d() {
        let mut case = morphology_case();
        first_descriptor_mut(&mut case, 0)["descriptor_type"] =
            serde_json::json!("SPHERICITY_3D");
        first_descriptor_mut(&mut case, 0)["basis"] =
            serde_json::json!("RECONSTRUCTION_3D");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "METHOD_CANNOT_SUPPORT_3D_DESCRIPTOR"
        ));
    }

    #[test]
    fn 3d_acquisition_can_support_sphericity_when_basis_is_explicit() {
        let mut case = morphology_case();
        case["acquisition"]["method"] = serde_json::json!("LIDAR_POINT_CLOUD");
        for stratum in case["size_strata"].as_array_mut().unwrap() {
            for descriptor in stratum["descriptors"].as_array_mut().unwrap() {
                descriptor["descriptor_type"] = serde_json::json!("SPHERICITY_3D");
                descriptor["basis"] = serde_json::json!("RECONSTRUCTION_3D");
                descriptor["quantiles"] = serde_json::json!({
                    "p05":0.42,"p25":0.55,"p50":0.68,"p75":0.79,"p95":0.91
                });
            }
        }
        assert!(validate_against_parent(&case, &parent_case()).is_empty());
    }

    #[test]
    fn single_view_uncontrolled_is_flagged_not_silently_accepted_as_3d() {
        let mut case = morphology_case();
        case["acquisition"]["orientation_protocol"] =
            serde_json::json!("SINGLE_VIEW_UNCONTROLLED");
        let report = evaluate_against_parent(&case, &parent_case()).expect("valid with warning");
        assert!(report.orientation_bias_warning);
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn unsampled_parent_bins_remain_explicitly_unknown() {
        let mut case = morphology_case();
        case["size_strata"].as_array_mut().unwrap().pop();
        let report = evaluate_against_parent(&case, &parent_case()).expect("partial coverage");
        assert_eq!(report.sampled_psd_bin_count, 3);
        assert_eq!(report.unsampled_parent_bin_indices, vec![3]);
        assert_eq!(report.size_bin_coverage_fraction, 0.75);
    }

    #[test]
    fn no_morphology_is_imputed_for_missing_size_bins() {
        let mut case = morphology_case();
        case["size_strata"].as_array_mut().unwrap().remove(1);
        let report = evaluate_against_parent(&case, &parent_case()).expect("partial coverage");
        assert_eq!(report.unsampled_parent_bin_indices, vec![1]);
        assert_eq!(report.strata.len(), 3);
    }

    #[test]
    fn missing_acquisition_calibration_is_rejected() {
        let mut case = morphology_case();
        case["acquisition"].as_object_mut().unwrap().remove("calibration_ref");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "MISSING_ACQUISITION_PROVENANCE"
        ));
    }

    #[test]
    fn missing_orientation_protocol_is_rejected() {
        let mut case = morphology_case();
        case["acquisition"].as_object_mut().unwrap().remove("orientation_protocol");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "INVALID_ORIENTATION_PROTOCOL"
        ));
    }

    #[test]
    fn unsupported_acquisition_method_is_rejected() {
        let mut case = morphology_case();
        case["acquisition"]["method"] = serde_json::json!("PHONE_PHOTO");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "UNSUPPORTED_ACQUISITION_METHOD"
        ));
    }

    #[test]
    fn descriptor_unit_must_be_explicitly_dimensionless() {
        let mut case = morphology_case();
        first_descriptor_mut(&mut case, 0)["unit"] = serde_json::json!("mm");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "INVALID_DESCRIPTOR_UNIT"
        ));
    }

    #[test]
    fn replay_is_deterministic() {
        let case = morphology_case();
        let parent = parent_case();
        let first = evaluate_against_parent(&case, &parent).expect("first replay");
        let second = evaluate_against_parent(&case, &parent).expect("second replay");
        assert_eq!(first, second);
    }

    #[test]
    fn every_descriptor_retains_its_definition_reference() {
        let mut case = morphology_case();
        first_descriptor_mut(&mut case, 0)["descriptor_definition_ref"] =
            serde_json::json!("");
        assert!(has_code(
            &validate_against_parent(&case, &parent_case()),
            "MISSING_DESCRIPTOR_DEFINITION"
        ));
    }

    #[test]
    fn parent_without_psd_bins_is_rejected() {
        let mut parent = parent_case();
        parent["intervals"][0]
            .as_object_mut()
            .unwrap()
            .remove("fragment_bins");
        assert!(has_code(
            &validate_against_parent(&morphology_case(), &parent),
            "PARENT_PSD_UNAVAILABLE"
        ));
    }
}
