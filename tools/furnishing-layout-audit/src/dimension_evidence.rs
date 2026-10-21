//! Structural audit for evidence attached to furnishing dimensions.
//!
//! This validates provenance shape and basic state/source consistency. It does
//! not authenticate external sources, determine whether evidence is fresh, or
//! connect evidence to layout promotion policy; callers must preserve those as
//! separate gates.

use serde_json::Value;
use std::collections::BTreeSet;

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object()?.get(key)
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    field(value, key)?.as_str()
}

fn nonblank(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn recognized_state(state: &str) -> bool {
    matches!(
        state,
        "measured" | "manufacturer_specified" | "seller_estimated" | "nominal" | "unknown"
    )
}

fn recognized_axis(axis: &str) -> bool {
    matches!(axis, "x" | "y" | "width" | "depth" | "height")
}

fn recognized_source_kind(kind: &str) -> bool {
    matches!(
        kind,
        "site_measurement"
            | "manufacturer_document"
            | "seller_listing"
            | "catalog_nominal"
            | "product_database"
            | "unknown"
    )
}

fn method_matches_state(state: &str, method: &str) -> bool {
    match state {
        "measured" => matches!(
            method,
            "tape_measure" | "laser_measure" | "phone_scan" | "survey"
        ),
        "manufacturer_specified" => {
            matches!(method, "manufacturer_specification" | "catalog_import")
        }
        "seller_estimated" => {
            matches!(method, "seller_estimate" | "seller_dimension_note" | "photo_estimate")
        }
        "nominal" => matches!(method, "design_nominal" | "catalog_nominal"),
        "unknown" => method == "unavailable",
        _ => false,
    }
}

fn source_matches_state(state: &str, source_kind: &str) -> bool {
    match state {
        "measured" => source_kind == "site_measurement",
        "manufacturer_specified" => {
            matches!(source_kind, "manufacturer_document" | "product_database")
        }
        "seller_estimated" => source_kind == "seller_listing",
        "nominal" => matches!(
            source_kind,
            "catalog_nominal" | "manufacturer_document" | "product_database"
        ),
        "unknown" => recognized_source_kind(source_kind),
        _ => false,
    }
}

/// Validate a dimension-evidence contract document.
///
/// Diagnostics are deterministic and ordered by input record, so they can be
/// included in reproducible audit receipts. A clean result means the record is
/// structurally coherent; it does not prove source authenticity or real-world
/// dimensional truth.
pub fn audit_dimension_evidence(document: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if text(document, "schema_version") != Some("0.1.0") {
        errors.push("document: unsupported or missing schema_version (expected 0.1.0)".to_owned());
    }
    if !nonblank(text(document, "benchmark_id")) {
        errors.push("document: missing or blank benchmark_id".to_owned());
    }

    let Some(records) = field(document, "records").and_then(Value::as_array) else {
        errors.push("document: records must be an array".to_owned());
        return errors;
    };
    if records.is_empty() {
        errors.push("document: records must not be empty".to_owned());
    }

    let mut seen_ids = BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        let prefix = format!("records[{index}]");
        let Some(evidence_id) = text(record, "evidence_id") else {
            errors.push(format!("{prefix}: missing string evidence_id"));
            continue;
        };
        if evidence_id.trim().is_empty() {
            errors.push(format!("{prefix}: evidence_id must not be blank"));
        }
        if !seen_ids.insert(evidence_id.to_owned()) {
            errors.push(format!("{prefix}: duplicate evidence_id {evidence_id:?}"));
        }

        let Some(subject) = field(record, "subject") else {
            errors.push(format!("{prefix}: missing subject object"));
            continue;
        };
        let subject_kind = text(subject, "kind");
        let subject_id = text(subject, "subject_id");
        if !matches!(subject_kind, Some("room" | "product" | "placement")) {
            errors.push(format!("{prefix}: subject.kind must be room, product, or placement"));
        }
        if !nonblank(subject_id) {
            errors.push(format!("{prefix}: subject.subject_id must be a nonblank string"));
        }

        let axis = text(record, "axis");
        if !axis.is_some_and(recognized_axis) {
            errors.push(format!("{prefix}: axis must be x, y, width, depth, or height"));
        }

        let state = text(record, "state");
        if !state.is_some_and(recognized_state) {
            errors.push(format!(
                "{prefix}: state must be measured, manufacturer_specified, seller_estimated, nominal, or unknown"
            ));
            continue;
        }

        let bounds = field(record, "bounds_m");
        if *state == "unknown" {
            if bounds.is_some() {
                errors.push(format!(
                    "{prefix}: unknown dimensions must not contain bounds_m"
                ));
            }
        } else {
            let Some(bounds) = bounds else {
                errors.push(format!("{prefix}: known state requires bounds_m"));
                validate_source_and_method(record, &prefix, state, &mut errors);
                continue;
            };
            let min = field(bounds, "min_m").and_then(Value::as_f64);
            let max = field(bounds, "max_m").and_then(Value::as_f64);
            match (min, max) {
                (Some(min), Some(max)) if min.is_finite() && max.is_finite() && min <= max => {
                    if matches!(*axis.unwrap_or(""), "width" | "depth" | "height") && min <= 0.0 {
                        errors.push(format!(
                            "{prefix}: extent axes require a strictly positive minimum"
                        ));
                    }
                }
                _ => errors.push(format!(
                    "{prefix}: bounds_m requires finite min_m <= max_m"
                )),
            }
            validate_source_and_method(record, &prefix, state, &mut errors);
        }

        if let Some(recorded_at) = text(record, "recorded_at") {
            if recorded_at.trim().is_empty() {
                errors.push(format!("{prefix}: recorded_at must not be blank"));
            }
        } else {
            errors.push(format!("{prefix}: missing recorded_at timestamp"));
        }

        if let Some(note) = text(record, "note")
            && note.trim().is_empty()
        {
            errors.push(format!("{prefix}: note must not be blank when provided"));
        }
    }

    errors
}

fn validate_source_and_method(
    record: &Value,
    prefix: &str,
    state: &str,
    errors: &mut Vec<String>,
) {
    let Some(source) = field(record, "source") else {
        if state != "unknown" {
            errors.push(format!("{prefix}: known state requires source"));
        }
        if state != "unknown" {
            errors.push(format!("{prefix}: known state requires method"));
        }
        return;
    };

    let source_id = text(source, "source_id");
    let source_kind = text(source, "source_kind");
    let captured_at = text(source, "captured_at");
    if !nonblank(source_id) {
        errors.push(format!("{prefix}: source.source_id must be nonblank"));
    }
    if !source_kind.is_some_and(recognized_source_kind) {
        errors.push(format!("{prefix}: unrecognized source.source_kind"));
    } else if !source_matches_state(state, source_kind.unwrap_or("")) {
        errors.push(format!(
            "{prefix}: source.source_kind is inconsistent with state {state:?}"
        ));
    }
    if !nonblank(captured_at) {
        errors.push(format!("{prefix}: source.captured_at must be nonblank"));
    }

    if let Some(digest) = text(source, "content_sha256")
        && !(digest.len() == 64 && digest.chars().all(|character| character.is_ascii_hexdigit()))
    {
        errors.push(format!(
            "{prefix}: source.content_sha256 must be 64 hexadecimal characters"
        ));
    }

    let method = text(record, "method");
    if state != "unknown" && !nonblank(method) {
        errors.push(format!("{prefix}: known state requires method"));
    } else if let Some(method) = method
        && !method_matches_state(state, method)
    {
        errors.push(format!(
            "{prefix}: method {method:?} is inconsistent with state {state:?}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::audit_dimension_evidence;
    use serde_json::{Value, json};
    use std::{fs, path::PathBuf};

    fn fixture() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/interiors/furnishing_dimension_evidence_v0_1.json");
        let bytes = fs::read(&path).expect("dimension evidence fixture must exist");
        serde_json::from_slice(&bytes).expect("dimension evidence fixture must be JSON")
    }

    #[test]
    fn dimension_evidence_fixture_passes_structural_audit() {
        let errors = audit_dimension_evidence(&fixture());
        assert!(errors.is_empty(), "dimension evidence errors: {errors:#?}");
    }

    #[test]
    fn unknown_dimension_cannot_carry_numeric_bounds() {
        let mut doc = fixture();
        doc["records"][3]["bounds_m"] = json!({"min_m": 0.5, "max_m": 0.7});
        let errors = audit_dimension_evidence(&doc);
        assert!(errors.iter().any(|error| error.contains("unknown dimensions must not contain bounds_m")));
    }

    #[test]
    fn seller_estimate_cannot_claim_manufacturer_source() {
        let mut doc = fixture();
        doc["records"][2]["source"]["source_kind"] = json!("manufacturer_document");
        let errors = audit_dimension_evidence(&doc);
        assert!(errors.iter().any(|error| error.contains("inconsistent with state")));
    }

    #[test]
    fn duplicate_evidence_ids_are_rejected() {
        let mut doc = fixture();
        doc["records"][1]["evidence_id"] = doc["records"][0]["evidence_id"].clone();
        let errors = audit_dimension_evidence(&doc);
        assert!(errors.iter().any(|error| error.contains("duplicate evidence_id")));
    }

    #[test]
    fn nonpositive_product_extent_bounds_are_rejected() {
        let mut doc = fixture();
        doc["records"][0]["bounds_m"]["min_m"] = json!(0.0);
        let errors = audit_dimension_evidence(&doc);
        assert!(errors.iter().any(|error| error.contains("strictly positive minimum")));
    }

    #[test]
    fn malformed_source_digest_is_rejected() {
        let mut doc = fixture();
        doc["records"][0]["source"]["content_sha256"] = json!("not-a-sha256");
        let errors = audit_dimension_evidence(&doc);
        assert!(errors.iter().any(|error| error.contains("64 hexadecimal characters")));
    }
}
