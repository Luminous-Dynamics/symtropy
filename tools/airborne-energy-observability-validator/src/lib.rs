// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite-sample observability verifier for synthetic aerial receiver pose maps.
//! A unique synthetic result is not a continuous physical envelope or a safety permit.

use serde_json::Value;
use std::collections::BTreeSet;

const TOP_KEYS: &[&str] = &[
    "schema_version",
    "case_id",
    "record_state",
    "evidence_class",
    "field",
    "receiver",
    "observation_contract",
    "pose_samples",
    "expected_disposition",
    "claim_ceiling",
];
const FIELD_KEYS: &[&str] = &["modality", "field_id", "field_sha256", "field_revision"];
const RECEIVER_KEYS: &[&str] = &[
    "receiver_id",
    "receiver_sha256",
    "sensor_kind",
    "calibration_sha256",
];
const CONTRACT_KEYS: &[&str] = &[
    "minimum_translation_separation_mm",
    "minimum_attitude_separation_mdeg",
    "measurement_noise_tolerance",
];
const SAMPLE_REQUIRED_KEYS: &[&str] = &[
    "pose_id",
    "position_mm",
    "attitude_mdeg",
    "observation_signature",
    "support_state",
];
const SAMPLE_OPTIONAL_KEYS: &[&str] = &["received_power_microwatt"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasPair {
    pub first_pose_id: String,
    pub second_pose_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservabilityReport {
    pub case_id: String,
    pub disposition: String,
    pub total_pose_samples: usize,
    pub in_domain_pose_samples: usize,
    pub out_of_domain_pose_samples: usize,
    pub uniquely_observable_pose_samples: usize,
    pub ambiguous_pose_samples: usize,
    pub alias_pairs: Vec<AliasPair>,
}

impl ObservabilityReport {
    pub fn unique_sample_numerator(&self) -> usize {
        self.uniquely_observable_pose_samples
    }

    pub fn unique_sample_denominator(&self) -> usize {
        self.in_domain_pose_samples
    }
}

fn fail(path: &str, reason: &str) -> String {
    format!("{path}: {reason}")
}

fn object(value: &Value, path: &str) -> Result<(), String> {
    if value.is_object() {
        Ok(())
    } else {
        Err(fail(path, "must be an object"))
    }
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| fail(path, "must be an array"))
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, &format!("missing property {key}")))
}

fn string<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| fail(path, &format!("{key} must be a non-empty string")))
}

fn unsigned(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(path, &format!("{key} must be a non-negative integer")))
}

fn exact_keys(value: &Value, expected: &[&str], path: &str) -> Result<(), String> {
    object(value, path)?;
    let map = value.as_object().expect("object checked");
    for key in expected {
        if !map.contains_key(*key) {
            return Err(fail(path, &format!("missing property {key}")));
        }
    }
    if let Some(key) = map.keys().find(|key| !expected.contains(&key.as_str())) {
        return Err(fail(path, &format!("unknown property {key}")));
    }
    Ok(())
}

fn exact_keys_with_optional(
    value: &Value,
    required: &[&str],
    optional: &[&str],
    path: &str,
) -> Result<(), String> {
    object(value, path)?;
    let map = value.as_object().expect("object checked");
    for key in required {
        if !map.contains_key(*key) {
            return Err(fail(path, &format!("missing property {key}")));
        }
    }
    if let Some(key) = map
        .keys()
        .find(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        return Err(fail(path, &format!("unknown property {key}")));
    }
    Ok(())
}

fn validate_sha256(value: &str, path: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(fail(path, "must be lowercase hexadecimal SHA-256"));
    }
    Ok(())
}

fn vector3_i64(value: &Value, path: &str) -> Result<[i64; 3], String> {
    let values = array(value, path)?;
    if values.len() != 3 {
        return Err(fail(path, "must contain exactly three integer components"));
    }
    let mut result = [0_i64; 3];
    for (index, item) in values.iter().enumerate() {
        result[index] = item
            .as_i64()
            .ok_or_else(|| fail(path, "components must be signed integers"))?;
    }
    Ok(result)
}

fn vector_i64(value: &Value, path: &str) -> Result<Vec<i64>, String> {
    let values = array(value, path)?;
    if values.is_empty() || values.len() > 32 {
        return Err(fail(path, "must contain 1..=32 integer features"));
    }
    values
        .iter()
        .map(|item| {
            item.as_i64()
                .ok_or_else(|| fail(path, "features must be signed integers"))
        })
        .collect()
}

#[derive(Debug, Clone)]
struct PoseSample {
    id: String,
    position: [i64; 3],
    attitude: [i64; 3],
    signature: Vec<i64>,
    in_domain: bool,
}

fn squared_translation(left: [i64; 3], right: [i64; 3]) -> i128 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| {
            let delta = *a as i128 - *b as i128;
            delta * delta
        })
        .sum()
}

fn wrapped_yaw_delta_mdeg(left: i64, right: i64) -> i128 {
    let raw = (left as i128 - right as i128).abs();
    raw.min(360_000 - raw)
}

fn squared_attitude(left: [i64; 3], right: [i64; 3]) -> i128 {
    let deltas = [
        wrapped_yaw_delta_mdeg(left[0], right[0]),
        (left[1] as i128 - right[1] as i128).abs(),
        (left[2] as i128 - right[2] as i128).abs(),
    ];
    deltas.iter().map(|delta| delta * delta).sum()
}

fn poses_are_distinct(
    left: &PoseSample,
    right: &PoseSample,
    minimum_translation_mm: u64,
    minimum_attitude_mdeg: u64,
) -> bool {
    let translation_limit = (minimum_translation_mm as i128).pow(2);
    let attitude_limit = (minimum_attitude_mdeg as i128).pow(2);
    squared_translation(left.position, right.position) >= translation_limit
        || squared_attitude(left.attitude, right.attitude) >= attitude_limit
}

fn signatures_are_indistinguishable(
    left: &[i64],
    right: &[i64],
    noise_tolerance: &[u64],
) -> Result<bool, String> {
    if left.len() != right.len() || left.len() != noise_tolerance.len() {
        return Err("observation signature length does not match the noise contract".to_string());
    }
    for ((a, b), tolerance) in left.iter().zip(right.iter()).zip(noise_tolerance.iter()) {
        let twice_tolerance = (*tolerance as u128) * 2;
        if (a.abs_diff(*b) as u128) > twice_tolerance {
            return Ok(false);
        }
    }
    Ok(true)
}

fn validate_not_run(value: &Value, case_id: &str) -> Result<ObservabilityReport, String> {
    if case_id != "NOT_RUN" {
        return Err(fail(
            "$.case_id",
            "NOT_RUN template case_id must be NOT_RUN",
        ));
    }
    if string(value, "evidence_class", "$")? != "NOT_RUN" {
        return Err(fail(
            "$.evidence_class",
            "NOT_RUN record requires NOT_RUN evidence class",
        ));
    }
    for key in ["field", "receiver", "observation_contract"] {
        if !field(value, key, "$")?.is_null() {
            return Err(fail("$", &format!("NOT_RUN requires {key}=null")));
        }
    }
    if !array(field(value, "pose_samples", "$")?, "$.pose_samples")?.is_empty() {
        return Err(fail(
            "$.pose_samples",
            "NOT_RUN requires an empty pose sample array",
        ));
    }
    if string(value, "expected_disposition", "$")? != "NOT_RUN" {
        return Err(fail(
            "$.expected_disposition",
            "NOT_RUN template requires NOT_RUN disposition",
        ));
    }
    Ok(ObservabilityReport {
        case_id: "NOT_RUN".to_string(),
        disposition: "NOT_RUN".to_string(),
        total_pose_samples: 0,
        in_domain_pose_samples: 0,
        out_of_domain_pose_samples: 0,
        uniquely_observable_pose_samples: 0,
        ambiguous_pose_samples: 0,
        alias_pairs: Vec::new(),
    })
}

fn validate_fixture(value: &Value, case_id: &str) -> Result<ObservabilityReport, String> {
    let evidence_class = string(value, "evidence_class", "$")?;
    if !matches!(
        evidence_class,
        "SYNTHETIC_KNOWN_TRUTH" | "SYNTHETIC_ADVERSARIAL"
    ) {
        return Err(fail(
            "$.evidence_class",
            "FIXTURE requires synthetic known-truth or adversarial evidence",
        ));
    }

    let field_value = field(value, "field", "$")?;
    exact_keys(field_value, FIELD_KEYS, "$.field")?;
    let modality = string(field_value, "modality", "$.field")?;
    if !matches!(modality, "RF_MICROWAVE" | "OPTICAL") {
        return Err(fail(
            "$.field.modality",
            "v0 supports RF_MICROWAVE and OPTICAL only",
        ));
    }
    string(field_value, "field_id", "$.field")?;
    validate_sha256(
        string(field_value, "field_sha256", "$.field")?,
        "$.field.field_sha256",
    )?;
    string(field_value, "field_revision", "$.field")?;

    let receiver_value = field(value, "receiver", "$")?;
    exact_keys(receiver_value, RECEIVER_KEYS, "$.receiver")?;
    string(receiver_value, "receiver_id", "$.receiver")?;
    validate_sha256(
        string(receiver_value, "receiver_sha256", "$.receiver")?,
        "$.receiver.receiver_sha256",
    )?;
    let sensor_kind = string(receiver_value, "sensor_kind", "$.receiver")?;
    if !matches!(
        sensor_kind,
        "SEGMENTED_RF_POWER" | "PHOTODIODE_SPOT" | "SYNTHETIC_OTHER"
    ) {
        return Err(fail("$.receiver.sensor_kind", "unsupported v0 sensor kind"));
    }

    let sensor_matches_modality = match modality {
        "RF_MICROWAVE" => matches!(sensor_kind, "SEGMENTED_RF_POWER" | "SYNTHETIC_OTHER"),
        "OPTICAL" => matches!(sensor_kind, "PHOTODIODE_SPOT" | "SYNTHETIC_OTHER"),
        _ => false,
    };
    if !sensor_matches_modality {
        return Err(fail(
            "$.receiver.sensor_kind",
            "sensor kind does not match the declared field modality",
        ));
    }
    validate_sha256(
        string(receiver_value, "calibration_sha256", "$.receiver")?,
        "$.receiver.calibration_sha256",
    )?;

    let contract = field(value, "observation_contract", "$")?;
    exact_keys(contract, CONTRACT_KEYS, "$.observation_contract")?;
    let min_translation = unsigned(
        contract,
        "minimum_translation_separation_mm",
        "$.observation_contract",
    )?;
    let min_attitude = unsigned(
        contract,
        "minimum_attitude_separation_mdeg",
        "$.observation_contract",
    )?;
    if min_translation == 0
        || min_translation > 1_000_000_000
        || min_attitude == 0
        || min_attitude > 360_000
    {
        return Err(fail(
            "$.observation_contract",
            "pose separation thresholds are outside the supported positive bounds",
        ));
    }
    let tolerance_values = array(
        field(
            contract,
            "measurement_noise_tolerance",
            "$.observation_contract",
        )?,
        "$.observation_contract.measurement_noise_tolerance",
    )?;
    if tolerance_values.is_empty() || tolerance_values.len() > 32 {
        return Err(fail(
            "$.observation_contract.measurement_noise_tolerance",
            "must contain 1..=32 tolerances",
        ));
    }
    let mut noise_tolerance = Vec::with_capacity(tolerance_values.len());
    for tolerance in tolerance_values {
        noise_tolerance.push(tolerance.as_u64().ok_or_else(|| {
            fail(
                "$.observation_contract.measurement_noise_tolerance",
                "tolerances must be non-negative integers",
            )
        })?);
    }

    let sample_values = array(field(value, "pose_samples", "$")?, "$.pose_samples")?;
    if !(2..=10_000).contains(&sample_values.len()) {
        return Err(fail(
            "$.pose_samples",
            "must contain 2..=10000 pose samples",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut samples = Vec::with_capacity(sample_values.len());
    for (index, sample) in sample_values.iter().enumerate() {
        let path = format!("$.pose_samples[{index}]");
        exact_keys_with_optional(sample, SAMPLE_REQUIRED_KEYS, SAMPLE_OPTIONAL_KEYS, &path)?;
        let id = string(sample, "pose_id", &path)?.to_string();
        if !ids.insert(id.clone()) {
            return Err(fail(&path, "duplicate pose_id"));
        }
        let position = vector3_i64(
            field(sample, "position_mm", &path)?,
            &format!("{path}.position_mm"),
        )?;
        if position
            .iter()
            .any(|value| value.unsigned_abs() > 1_000_000_000_u64)
        {
            return Err(fail(
                &path,
                "position components must be within +/-1000000000 mm",
            ));
        }
        let attitude = vector3_i64(
            field(sample, "attitude_mdeg", &path)?,
            &format!("{path}.attitude_mdeg"),
        )?;
        if attitude
            .iter()
            .any(|value| !(-180_000..=180_000).contains(value))
        {
            return Err(fail(
                &path,
                "attitude components must be within -180000..=180000 millidegrees",
            ));
        }
        let signature = vector_i64(
            field(sample, "observation_signature", &path)?,
            &format!("{path}.observation_signature"),
        )?;
        if signature.len() != noise_tolerance.len() {
            return Err(fail(
                &path,
                "observation_signature length differs from measurement_noise_tolerance",
            ));
        }
        if let Some(power) = sample.get("received_power_microwatt") {
            if power.as_u64().is_none() {
                return Err(fail(
                    &path,
                    "received_power_microwatt must be a non-negative integer",
                ));
            }
        }
        let support_state = string(sample, "support_state", &path)?;
        let in_domain = match support_state {
            "IN_DOMAIN" => true,
            "OUT_OF_DOMAIN" => false,
            _ => {
                return Err(fail(
                    &path,
                    "support_state must be IN_DOMAIN or OUT_OF_DOMAIN",
                ));
            }
        };
        samples.push(PoseSample {
            id,
            position,
            attitude,
            signature,
            in_domain,
        });
    }

    let in_domain: Vec<&PoseSample> = samples.iter().filter(|sample| sample.in_domain).collect();
    let out_of_domain_count = samples.len() - in_domain.len();
    if in_domain.len() < 2 {
        return Err(fail(
            "$.pose_samples",
            "at least two in-domain pose samples are required",
        ));
    }

    let mut alias_pairs = Vec::new();
    let mut ambiguous_ids = BTreeSet::new();
    for left_index in 0..in_domain.len() {
        for right_index in (left_index + 1)..in_domain.len() {
            let left = in_domain[left_index];
            let right = in_domain[right_index];
            if poses_are_distinct(left, right, min_translation, min_attitude)
                && signatures_are_indistinguishable(
                    &left.signature,
                    &right.signature,
                    &noise_tolerance,
                )?
            {
                let (first, second) = if left.id <= right.id {
                    (left.id.clone(), right.id.clone())
                } else {
                    (right.id.clone(), left.id.clone())
                };
                alias_pairs.push(AliasPair {
                    first_pose_id: first.clone(),
                    second_pose_id: second.clone(),
                });
                ambiguous_ids.insert(first);
                ambiguous_ids.insert(second);
            }
        }
    }
    alias_pairs.sort_by(|left, right| {
        (&left.first_pose_id, &left.second_pose_id)
            .cmp(&(&right.first_pose_id, &right.second_pose_id))
    });
    let unique_count = in_domain
        .iter()
        .filter(|sample| !ambiguous_ids.contains(&sample.id))
        .count();
    let disposition = if alias_pairs.is_empty() {
        "UNIQUE_WITHIN_DECLARED_SAMPLE_SET"
    } else {
        "AMBIGUOUS"
    };
    let expected = string(value, "expected_disposition", "$")?;
    if expected != disposition {
        return Err(fail(
            "$.expected_disposition",
            &format!("expected {expected}, independently computed {disposition}"),
        ));
    }

    Ok(ObservabilityReport {
        case_id: case_id.to_string(),
        disposition: disposition.to_string(),
        total_pose_samples: samples.len(),
        in_domain_pose_samples: in_domain.len(),
        out_of_domain_pose_samples: out_of_domain_count,
        uniquely_observable_pose_samples: unique_count,
        ambiguous_pose_samples: ambiguous_ids.len(),
        alias_pairs,
    })
}

/// Evaluate one finite, synthetic pose/signature map. Pairwise signature
/// uncertainty regions are treated as overlapping when every feature differs
/// by no more than twice its declared per-observation noise tolerance.
pub fn validate_case(value: &Value) -> Result<ObservabilityReport, String> {
    exact_keys(value, TOP_KEYS, "$")?;
    if string(value, "schema_version", "$")? != "airborne-energy-observability-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let case_id = string(value, "case_id", "$")?.to_string();
    string(value, "claim_ceiling", "$")?;
    match string(value, "record_state", "$")? {
        "NOT_RUN" => validate_not_run(value, &case_id),
        "FIXTURE" => validate_fixture(value, &case_id),
        other => Err(fail(
            "$.record_state",
            &format!("unsupported state {other}"),
        )),
    }
}

/// Parse and validate an observability case encoded as JSON.
pub fn validate_json(input: &str) -> Result<ObservabilityReport, String> {
    let value: Value =
        serde_json::from_str(input).map_err(|error| format!("invalid JSON: {error}"))?;
    validate_case(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const POSITIVE: &str = include_str!(
        "../../../research/airborne-energy-observability-v0/observability.unique.synthetic.json"
    );
    const AMBIGUOUS: &str = include_str!(
        "../../../research/airborne-energy-observability-v0/observability.ambiguous.synthetic.json"
    );
    const NOT_RUN: &str = include_str!(
        "../../../research/airborne-energy-observability-v0/observability.NOT_RUN.template.json"
    );

    fn positive_value() -> Value {
        serde_json::from_str(POSITIVE).expect("positive fixture JSON")
    }

    #[test]
    fn unique_map_excludes_out_of_domain_alias_and_reports_sample_denominator() {
        let report = validate_json(POSITIVE).unwrap();
        assert_eq!(report.disposition, "UNIQUE_WITHIN_DECLARED_SAMPLE_SET");
        assert_eq!(report.total_pose_samples, 4);
        assert_eq!(report.in_domain_pose_samples, 3);
        assert_eq!(report.out_of_domain_pose_samples, 1);
        assert_eq!(report.uniquely_observable_pose_samples, 3);
        assert!(report.alias_pairs.is_empty());
        assert_eq!(report.unique_sample_numerator(), 3);
        assert_eq!(report.unique_sample_denominator(), 3);
    }

    #[test]
    fn high_received_power_does_not_resolve_ambiguous_pose_signatures() {
        let report = validate_json(AMBIGUOUS).unwrap();
        assert_eq!(report.disposition, "AMBIGUOUS");
        assert_eq!(report.in_domain_pose_samples, 3);
        assert_eq!(report.out_of_domain_pose_samples, 1);
        assert_eq!(report.ambiguous_pose_samples, 3);
        assert_eq!(report.alias_pairs.len(), 3);
    }

    #[test]
    fn not_run_template_is_not_a_pass() {
        let report = validate_json(NOT_RUN).unwrap();
        assert_eq!(report.disposition, "NOT_RUN");
        assert_eq!(report.in_domain_pose_samples, 0);
        assert!(report.alias_pairs.is_empty());
    }

    #[test]
    fn duplicate_pose_ids_fail_closed() {
        let mut value = positive_value();
        value["pose_samples"][1]["pose_id"] = json!("pose-left");
        assert!(
            validate_case(&value)
                .unwrap_err()
                .contains("duplicate pose_id")
        );
    }

    #[test]
    fn signature_width_must_match_noise_contract() {
        let mut value = positive_value();
        value["pose_samples"][1]["observation_signature"] = json!([10, 20]);
        assert!(
            validate_case(&value)
                .unwrap_err()
                .contains("length differs")
        );
    }

    #[test]
    fn expected_disposition_cannot_override_computed_ambiguity() {
        let mut value: Value = serde_json::from_str(AMBIGUOUS).unwrap();
        value["expected_disposition"] = json!("UNIQUE_WITHIN_DECLARED_SAMPLE_SET");
        assert!(
            validate_case(&value)
                .unwrap_err()
                .contains("independently computed AMBIGUOUS")
        );
    }

    #[test]
    fn changed_field_digest_is_rejected_when_missing_or_malformed() {
        let mut value = positive_value();
        value["field"]["field_sha256"] = json!("not-a-digest");
        assert!(
            validate_case(&value)
                .unwrap_err()
                .contains("lowercase hexadecimal SHA-256")
        );
    }

    #[test]
    fn row_order_does_not_change_sorted_alias_witnesses() {
        let original = validate_json(AMBIGUOUS).unwrap();
        let mut reversed = serde_json::from_str::<Value>(AMBIGUOUS).unwrap();
        reversed["pose_samples"].as_array_mut().unwrap().reverse();
        let replayed = validate_case(&reversed).unwrap();
        assert_eq!(original.alias_pairs, replayed.alias_pairs);
        assert_eq!(original.disposition, replayed.disposition);
        assert_eq!(
            original.ambiguous_pose_samples,
            replayed.ambiguous_pose_samples
        );
    }

    #[test]
    fn signatures_of_poses_inside_declared_resolution_are_not_counted_as_aliases() {
        let mut value = positive_value();
        value["pose_samples"][1]["position_mm"] = json!([-900, 0, 1000]);
        let repeated_signature = value["pose_samples"][0]["observation_signature"].clone();
        value["pose_samples"][1]["observation_signature"] = repeated_signature;
        value["pose_samples"][2]["position_mm"] = json!([1000, 0, 1000]);
        // The first two poses differ by only 100 mm, below the 500 mm
        // declared resolution. Their matching signatures are not a false alias.
        let report = validate_case(&value).unwrap();
        assert_eq!(report.disposition, "UNIQUE_WITHIN_DECLARED_SAMPLE_SET");
    }

    #[test]
    fn sensor_kind_must_match_field_modality() {
        let mut value = positive_value();
        value["field"]["modality"] = json!("OPTICAL");
        assert!(
            validate_case(&value)
                .unwrap_err()
                .contains("sensor kind does not match")
        );
    }

    #[test]
    fn missing_claim_ceiling_is_rejected() {
        let mut value = positive_value();
        value.as_object_mut().unwrap().remove("claim_ceiling");
        assert!(
            validate_case(&value)
                .unwrap_err()
                .contains("missing property claim_ceiling")
        );
    }

    #[test]
    fn extreme_signed_position_is_rejected_without_abs_overflow() {
        let mut value = positive_value();
        value["pose_samples"][0]["position_mm"] = json!([i64::MIN, 0, 1000]);
        assert!(
            validate_case(&value)
                .unwrap_err()
                .contains("position components must be within")
        );
    }
}
