//! Fail-closed admission rules for cross-simulator physics comparisons.
//!
//! This crate validates the semantic *mapping decision*. It does not parse JSON,
//! execute a simulator, certify a physical model, or select a source of truth.
//! Transport adapters deserialize the versioned receipt into these types before
//! calling [validate_receipt].

use jsonschema::validator_for;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use std::collections::BTreeSet;

/// The nine required comparison dimensions in schema order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Dimension {
    IdentityGeometry,
    MassProperties,
    Constraints,
    Contacts,
    Integration,
    Ccd,
    Actuation,
    Observations,
    NumericalProfile,
}

impl Dimension {
    pub const ALL: [Self; 9] = [
        Self::IdentityGeometry,
        Self::MassProperties,
        Self::Constraints,
        Self::Contacts,
        Self::Integration,
        Self::Ccd,
        Self::Actuation,
        Self::Observations,
        Self::NumericalProfile,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IdentityGeometry => "identity_geometry",
            Self::MassProperties => "mass_properties",
            Self::Constraints => "constraints",
            Self::Contacts => "contacts",
            Self::Integration => "integration",
            Self::Ccd => "ccd",
            Self::Actuation => "actuation",
            Self::Observations => "observations",
            Self::NumericalProfile => "numerical_profile",
        }
    }
}

/// Pairwise semantic disposition. Strict, bounded, and jointly-not-applicable
/// dimensions may be admitted; every unknown or incomplete state fails closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingStatus {
    StrictlyEquivalent,
    MappedEquivalentWithBound,
    NotApplicable,
    NotProvided,
    NonEquivalent,
    Unsupported,
    UnknownUnassessed,
}

impl MappingStatus {
    pub const fn is_admissible(self) -> bool {
        matches!(
            self,
            Self::StrictlyEquivalent | Self::MappedEquivalentWithBound | Self::NotApplicable
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ErrorBound {
    pub metric: String,
    pub maximum_absolute_error: f64,
    pub unit: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DimensionMapping {
    pub dimension: Dimension,
    pub status: MappingStatus,
    pub rationale: String,
    pub transformation: Option<String>,
    pub error_bounds: Vec<ErrorBound>,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackendIdentity {
    pub engine_id: String,
    pub version: String,
    pub adapter_commit: String,
    pub profile_id: String,
    pub profile_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioIdentity {
    pub id: String,
    pub digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReviewIdentity {
    pub author_id: String,
    pub reviewer_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappingReceipt {
    pub schema: String,
    pub scenario: ScenarioIdentity,
    pub left_backend: BackendIdentity,
    pub right_backend: BackendIdentity,
    pub dimensions: Vec<DimensionMapping>,
    pub review: ReviewIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationIssue {
    pub code: &'static str,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidationReport {
    pub score_eligible: bool,
    pub issues: Vec<ValidationIssue>,
    pub inadmissible_dimensions: Vec<Dimension>,
}

/// Validate a deserialized mapping receipt.
///
/// This function is intentionally conservative:
/// - every required dimension must appear exactly once;
/// - only StrictlyEquivalent, MappedEquivalentWithBound, and evidenced
///   NotApplicable dimensions may be admitted;
/// - bounded mappings require a non-empty transformation and explicit finite
///   non-negative error bounds;
/// - all comparisons require exact version/configuration identities;
/// - the author and reviewer must be distinct identities.
pub fn validate_receipt(receipt: &MappingReceipt) -> ValidationReport {
    let mut issues = Vec::new();
    let mut inadmissible_dimensions = Vec::new();

    if receipt.schema != "symtropy.semantic_mapping_receipt/0.1" {
        issues.push(issue("unsupported_schema", "schema"));
    }

    validate_non_empty(&mut issues, "scenario.id", &receipt.scenario.id);
    validate_digest(&mut issues, "scenario.digest", &receipt.scenario.digest);

    validate_backend(&mut issues, "left_backend", &receipt.left_backend);
    validate_backend(&mut issues, "right_backend", &receipt.right_backend);

    validate_non_empty(&mut issues, "review.author_id", &receipt.review.author_id);
    validate_non_empty(
        &mut issues,
        "review.reviewer_id",
        &receipt.review.reviewer_id,
    );
    if !receipt.review.author_id.is_empty()
        && receipt.review.author_id == receipt.review.reviewer_id
    {
        issues.push(issue("self_review_forbidden", "review.reviewer_id"));
    }

    let mut seen = BTreeSet::new();
    for (index, mapping) in receipt.dimensions.iter().enumerate() {
        let path = format!("dimensions[{index}]");
        if !seen.insert(mapping.dimension) {
            issues.push(issue("duplicate_dimension", &path));
        }

        validate_non_empty(
            &mut issues,
            &format!("{path}.rationale"),
            &mapping.rationale,
        );

        match mapping.status {
            MappingStatus::StrictlyEquivalent => {
                if mapping.evidence_refs.is_empty() {
                    issues.push(issue(
                        "missing_equivalence_evidence",
                        &format!("{path}.evidence_refs"),
                    ));
                }
            }
            MappingStatus::NotApplicable => {
                if mapping.evidence_refs.is_empty() {
                    issues.push(issue(
                        "missing_not_applicable_evidence",
                        &format!("{path}.evidence_refs"),
                    ));
                }
                if mapping.transformation.is_some() || !mapping.error_bounds.is_empty() {
                    issues.push(issue(
                        "not_applicable_has_mapping",
                        &format!("{path}.transformation"),
                    ));
                }
            }
            MappingStatus::MappedEquivalentWithBound => {
                if mapping.transformation.as_deref().is_none_or(str::is_empty) {
                    issues.push(issue(
                        "missing_mapping_transformation",
                        &format!("{path}.transformation"),
                    ));
                }
                if mapping.error_bounds.is_empty() {
                    issues.push(issue(
                        "missing_error_bound",
                        &format!("{path}.error_bounds"),
                    ));
                }
                if mapping.evidence_refs.is_empty() {
                    issues.push(issue(
                        "missing_mapping_evidence",
                        &format!("{path}.evidence_refs"),
                    ));
                }
            }
            MappingStatus::NonEquivalent | MappingStatus::Unsupported => {
                if mapping.evidence_refs.is_empty() {
                    issues.push(issue(
                        "missing_mapping_evidence",
                        &format!("{path}.evidence_refs"),
                    ));
                }
                inadmissible_dimensions.push(mapping.dimension);
            }
            MappingStatus::NotProvided | MappingStatus::UnknownUnassessed => {
                inadmissible_dimensions.push(mapping.dimension);
            }
        }

        for (bound_index, bound) in mapping.error_bounds.iter().enumerate() {
            let bound_path = format!("{path}.error_bounds[{bound_index}]");
            validate_non_empty(&mut issues, &format!("{bound_path}.metric"), &bound.metric);
            validate_non_empty(&mut issues, &format!("{bound_path}.unit"), &bound.unit);
            validate_non_empty(&mut issues, &format!("{bound_path}.source"), &bound.source);
            if !bound.maximum_absolute_error.is_finite() || bound.maximum_absolute_error < 0.0 {
                issues.push(issue(
                    "invalid_error_bound",
                    &format!("{bound_path}.maximum_absolute_error"),
                ));
            }
        }

        for (reference_index, reference) in mapping.evidence_refs.iter().enumerate() {
            validate_non_empty(
                &mut issues,
                &format!("{path}.evidence_refs[{reference_index}]"),
                reference,
            );
        }
    }

    for dimension in Dimension::ALL {
        if !seen.contains(&dimension) {
            issues.push(issue(
                "missing_dimension",
                &format!("dimensions.{}", dimension.as_str()),
            ));
            inadmissible_dimensions.push(dimension);
        }
    }

    // A known different backend is not a schema error, but a receipt comparing
    // a backend to itself is not a cross-simulator comparison.
    if receipt.left_backend.engine_id == receipt.right_backend.engine_id {
        issues.push(issue("same_engine_pair", "right_backend.engine_id"));
    }

    let score_eligible = issues.is_empty()
        && inadmissible_dimensions.is_empty()
        && receipt
            .dimensions
            .iter()
            .all(|mapping| mapping.status.is_admissible());

    ValidationReport {
        score_eligible,
        issues,
        inadmissible_dimensions,
    }
}

fn validate_backend(issues: &mut Vec<ValidationIssue>, prefix: &str, backend: &BackendIdentity) {
    validate_non_empty(issues, &format!("{prefix}.engine_id"), &backend.engine_id);
    validate_non_empty(issues, &format!("{prefix}.version"), &backend.version);
    validate_non_empty(issues, &format!("{prefix}.profile_id"), &backend.profile_id);
    validate_commit(
        issues,
        &format!("{prefix}.adapter_commit"),
        &backend.adapter_commit,
    );
    validate_digest(
        issues,
        &format!("{prefix}.profile_digest"),
        &backend.profile_digest,
    );
}

fn validate_non_empty(issues: &mut Vec<ValidationIssue>, path: &str, value: &str) {
    if value.trim().is_empty() {
        issues.push(issue("empty_required_value", path));
    }
}

fn validate_commit(issues: &mut Vec<ValidationIssue>, path: &str, value: &str) {
    let valid_len = value.len() == 40 || value.len() == 64;
    if !valid_len
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        issues.push(issue("invalid_commit_identity", path));
    }
}

fn validate_digest(issues: &mut Vec<ValidationIssue>, path: &str, value: &str) {
    let Some(hex) = value.strip_prefix("sha256:") else {
        issues.push(issue("invalid_sha256_digest", path));
        return;
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        issues.push(issue("invalid_sha256_digest", path));
    }
}

fn issue(code: &'static str, path: &str) -> ValidationIssue {
    ValidationIssue {
        code,
        path: path.to_owned(),
    }
}

/// JSON value wrapper whose map visitor rejects duplicate object keys recursively.
struct StrictJsonValue(JsonValue);

impl<'de> Deserialize<'de> for StrictJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = StrictJsonValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(JsonValue::Null))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(JsonValue::Null))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(JsonValue::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(JsonValue::Number(JsonNumber::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(JsonValue::Number(JsonNumber::from(value))))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        let number =
            JsonNumber::from_f64(value).ok_or_else(|| E::custom("JSON number is not finite"))?;
        Ok(StrictJsonValue(JsonValue::Number(number)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(JsonValue::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictJsonValue(JsonValue::String(value)))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<StrictJsonValue>()? {
            values.push(value.0);
        }
        Ok(StrictJsonValue(JsonValue::Array(values)))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = JsonMap::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object key {key:?}"
                )));
            }
            let value = object.next_value::<StrictJsonValue>()?;
            values.insert(key, value.0);
        }
        Ok(StrictJsonValue(JsonValue::Object(values)))
    }
}

/// Parse one complete JSON document, rejecting duplicate keys at every object depth.
fn parse_strict_json(input: &str) -> Result<JsonValue, String> {
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let value = StrictJsonValue::deserialize(&mut deserializer)
        .map_err(|error| format!("invalid receipt JSON: {error}"))?;
    deserializer
        .end()
        .map_err(|error| format!("invalid trailing receipt JSON: {error}"))?;
    Ok(value.0)
}

/// Validate raw receipt JSON against the checked-in Draft 2020-12 schema.
///
/// This is deliberately separate from the typed semantic admission validator:
/// schema validity establishes transport conformance, not physical equivalence.
pub fn validate_receipt_schema_json(input: &str) -> Result<(), String> {
    let instance = parse_strict_json(input)?;
    let schema = parse_strict_json(include_str!(
        "../../../schemas/physics/semantic_mapping_receipt_v0_1.schema.json"
    ))
    .map_err(|error| format!("checked-in receipt schema is invalid JSON: {error}"))?;
    let validator = validator_for(&schema)
        .map_err(|error| format!("cannot compile receipt JSON Schema: {error}"))?;
    let errors = validator
        .iter_errors(&instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// Validate JSON Schema conformance first, then run typed semantic admission.
pub fn validate_schema_and_json_receipt(input: &str) -> Result<ValidationReport, String> {
    validate_receipt_schema_json(input)?;
    validate_json_receipt(input)
}

/// Parse a versioned JSON receipt into the typed semantic contract.
///
/// This adapter is intentionally stricter than permissive JSON object decoding:
/// unknown fields are rejected, dimensions must contain exactly nine entries,
/// and enum spellings must match the versioned wire contract.
pub fn parse_receipt_json(input: &str) -> Result<MappingReceipt, String> {
    let value = parse_strict_json(input)?;
    let root = json_object(&value, "$")?;
    json_check_keys(
        root,
        "$",
        &[
            "schema",
            "scenario",
            "left_backend",
            "right_backend",
            "dimensions",
            "review",
        ],
        &[],
    )?;

    let scenario_value = json_required(root, "scenario", "$")?;
    let scenario = parse_scenario(scenario_value)?;
    let left_backend = parse_backend(json_required(root, "left_backend", "$")?, "$.left_backend")?;
    let right_backend = parse_backend(
        json_required(root, "right_backend", "$")?,
        "$.right_backend",
    )?;
    let review = parse_review(json_required(root, "review", "$")?)?;

    let dimensions_value = json_required(root, "dimensions", "$")?;
    let dimensions_array = dimensions_value
        .as_array()
        .ok_or_else(|| "$.dimensions must be an array".to_owned())?;
    if dimensions_array.len() != Dimension::ALL.len() {
        return Err(format!(
            "$.dimensions must contain exactly {} entries",
            Dimension::ALL.len()
        ));
    }
    let dimensions = dimensions_array
        .iter()
        .enumerate()
        .map(|(index, value)| parse_dimension_mapping(value, &format!("$.dimensions[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MappingReceipt {
        schema: json_string(root, "schema", "$")?,
        scenario,
        left_backend,
        right_backend,
        dimensions,
        review,
    })
}

/// Parse and validate a JSON receipt in one operation.
pub fn validate_json_receipt(input: &str) -> Result<ValidationReport, String> {
    let receipt = parse_receipt_json(input)?;
    Ok(validate_receipt(&receipt))
}

fn json_object<'a>(
    value: &'a JsonValue,
    path: &str,
) -> Result<&'a JsonMap<String, JsonValue>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))
}

fn json_check_keys(
    object: &JsonMap<String, JsonValue>,
    path: &str,
    required: &[&str],
    optional: &[&str],
) -> Result<(), String> {
    for key in required {
        if !object.contains_key(*key) {
            return Err(format!("{path}.{key} is required"));
        }
    }
    for key in object.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(format!(
                "{path}.{key} is not permitted by this schema version"
            ));
        }
    }
    Ok(())
}

fn json_required<'a>(
    object: &'a JsonMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<&'a JsonValue, String> {
    object
        .get(key)
        .ok_or_else(|| format!("{path}.{key} is required"))
}

fn json_string(
    object: &JsonMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<String, String> {
    json_required(object, key, path)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{path}.{key} must be a string"))
}

fn json_optional_string(
    object: &JsonMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<String>, String> {
    object
        .get(key)
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{path}.{key} must be a string"))
        })
        .transpose()
}

fn json_string_array(value: &JsonValue, path: &str) -> Result<Vec<String>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{path} must be an array"))?
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{path}[{index}] must be a string"))
        })
        .collect()
}

fn parse_scenario(value: &JsonValue) -> Result<ScenarioIdentity, String> {
    let object = json_object(value, "$.scenario")?;
    json_check_keys(object, "$.scenario", &["id", "digest"], &[])?;
    Ok(ScenarioIdentity {
        id: json_string(object, "id", "$.scenario")?,
        digest: json_string(object, "digest", "$.scenario")?,
    })
}

fn parse_backend(value: &JsonValue, path: &str) -> Result<BackendIdentity, String> {
    let object = json_object(value, path)?;
    json_check_keys(
        object,
        path,
        &[
            "engine_id",
            "version",
            "adapter_commit",
            "profile_id",
            "profile_digest",
        ],
        &[],
    )?;
    Ok(BackendIdentity {
        engine_id: json_string(object, "engine_id", path)?,
        version: json_string(object, "version", path)?,
        adapter_commit: json_string(object, "adapter_commit", path)?,
        profile_id: json_string(object, "profile_id", path)?,
        profile_digest: json_string(object, "profile_digest", path)?,
    })
}

fn parse_review(value: &JsonValue) -> Result<ReviewIdentity, String> {
    let object = json_object(value, "$.review")?;
    json_check_keys(object, "$.review", &["author_id", "reviewer_id"], &[])?;
    Ok(ReviewIdentity {
        author_id: json_string(object, "author_id", "$.review")?,
        reviewer_id: json_string(object, "reviewer_id", "$.review")?,
    })
}

fn parse_dimension_mapping(value: &JsonValue, path: &str) -> Result<DimensionMapping, String> {
    let object = json_object(value, path)?;
    json_check_keys(
        object,
        path,
        &["dimension", "status", "rationale", "evidence_refs"],
        &["transformation", "error_bounds"],
    )?;

    let dimension_text = json_string(object, "dimension", path)?;
    let dimension = match dimension_text.as_str() {
        "identity_geometry" => Dimension::IdentityGeometry,
        "mass_properties" => Dimension::MassProperties,
        "constraints" => Dimension::Constraints,
        "contacts" => Dimension::Contacts,
        "integration" => Dimension::Integration,
        "ccd" => Dimension::Ccd,
        "actuation" => Dimension::Actuation,
        "observations" => Dimension::Observations,
        "numerical_profile" => Dimension::NumericalProfile,
        _ => {
            return Err(format!(
                "{path}.dimension has unknown value {dimension_text:?}"
            ));
        }
    };

    let status_text = json_string(object, "status", path)?;
    let status = match status_text.as_str() {
        "strictly_equivalent" => MappingStatus::StrictlyEquivalent,
        "mapped_equivalent_with_bound" => MappingStatus::MappedEquivalentWithBound,
        "not_applicable" => MappingStatus::NotApplicable,
        "not_provided" => MappingStatus::NotProvided,
        "non_equivalent" => MappingStatus::NonEquivalent,
        "unsupported" => MappingStatus::Unsupported,
        "unknown_unassessed" => MappingStatus::UnknownUnassessed,
        _ => return Err(format!("{path}.status has unknown value {status_text:?}")),
    };

    let transformation = json_optional_string(object, "transformation", path)?;
    if transformation.as_deref().is_some_and(str::is_empty) {
        return Err(format!(
            "{path}.transformation must not be empty when present"
        ));
    }
    if status == MappingStatus::NotApplicable
        && (object.contains_key("transformation") || object.contains_key("error_bounds"))
    {
        return Err(format!(
            "{path} must omit transformation and error_bounds when status is not_applicable"
        ));
    }
    let error_bounds = match object.get("error_bounds") {
        Some(value) => value
            .as_array()
            .ok_or_else(|| format!("{path}.error_bounds must be an array"))?
            .iter()
            .enumerate()
            .map(|(index, bound)| {
                parse_error_bound(bound, &format!("{path}.error_bounds[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    let evidence_refs = json_string_array(
        json_required(object, "evidence_refs", path)?,
        &format!("{path}.evidence_refs"),
    )?;

    Ok(DimensionMapping {
        dimension,
        status,
        rationale: json_string(object, "rationale", path)?,
        transformation,
        error_bounds,
        evidence_refs,
    })
}

fn parse_error_bound(value: &JsonValue, path: &str) -> Result<ErrorBound, String> {
    let object = json_object(value, path)?;
    json_check_keys(
        object,
        path,
        &["metric", "maximum_absolute_error", "unit", "source"],
        &[],
    )?;
    let maximum_absolute_error = json_required(object, "maximum_absolute_error", path)?
        .as_f64()
        .ok_or_else(|| format!("{path}.maximum_absolute_error must be a JSON number"))?;
    Ok(ErrorBound {
        metric: json_string(object, "metric", path)?,
        maximum_absolute_error,
        unit: json_string(object, "unit", path)?,
        source: json_string(object, "source", path)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend(engine_id: &str) -> BackendIdentity {
        BackendIdentity {
            engine_id: engine_id.to_owned(),
            version: "1.0.0".to_owned(),
            adapter_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            profile_id: format!("{engine_id}-default"),
            profile_digest: format!("sha256:{}", "a".repeat(64)),
        }
    }

    fn mapping(dimension: Dimension, status: MappingStatus) -> DimensionMapping {
        let mapped = status == MappingStatus::MappedEquivalentWithBound;
        DimensionMapping {
            dimension,
            status,
            rationale: format!("Frozen comparison for {}", dimension.as_str()),
            transformation: mapped
                .then(|| "Convert values to SI and the declared world frame".to_owned()),
            error_bounds: if mapped {
                vec![ErrorBound {
                    metric: "position_error".to_owned(),
                    maximum_absolute_error: 1.0e-6,
                    unit: "m".to_owned(),
                    source: "analytic oracle case A".to_owned(),
                }]
            } else {
                Vec::new()
            },
            evidence_refs: if status == MappingStatus::UnknownUnassessed
                || status == MappingStatus::NotProvided
            {
                Vec::new()
            } else {
                vec![format!("fixture:{}", dimension.as_str())]
            },
        }
    }

    fn receipt(status: MappingStatus) -> MappingReceipt {
        MappingReceipt {
            schema: "symtropy.semantic_mapping_receipt/0.1".to_owned(),
            scenario: ScenarioIdentity {
                id: "free-fall-si-001".to_owned(),
                digest: format!("sha256:{}", "b".repeat(64)),
            },
            left_backend: backend("symtropy"),
            right_backend: backend("rapier"),
            dimensions: Dimension::ALL
                .into_iter()
                .map(|dimension| mapping(dimension, status))
                .collect(),
            review: ReviewIdentity {
                author_id: "author-A".to_owned(),
                reviewer_id: "reviewer-B".to_owned(),
            },
        }
    }

    #[test]
    fn strictly_equivalent_receipt_is_admitted() {
        let report = validate_receipt(&receipt(MappingStatus::StrictlyEquivalent));
        assert!(report.score_eligible, "{:?}", report.issues);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn mapped_receipt_with_a_bound_is_admitted() {
        let report = validate_receipt(&receipt(MappingStatus::MappedEquivalentWithBound));
        assert!(report.score_eligible, "{:?}", report.issues);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn mapped_receipt_requires_explicit_transformation_and_bound() {
        let mut candidate = receipt(MappingStatus::MappedEquivalentWithBound);
        let contact = candidate
            .dimensions
            .iter_mut()
            .find(|mapping| mapping.dimension == Dimension::Contacts)
            .expect("contact dimension exists");
        contact.transformation = None;
        contact.error_bounds.clear();

        let report = validate_receipt(&candidate);
        assert!(!report.score_eligible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "missing_mapping_transformation")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "missing_error_bound")
        );
    }

    #[test]
    fn not_applicable_dimension_is_admissible_only_with_evidence() {
        let mut candidate = receipt(MappingStatus::StrictlyEquivalent);
        {
            let contact = candidate
                .dimensions
                .iter_mut()
                .find(|mapping| mapping.dimension == Dimension::Contacts)
                .expect("contact dimension exists");
            contact.status = MappingStatus::NotApplicable;
            contact.evidence_refs = vec!["scenario:no-contact-001".to_owned()];
        }

        let report = validate_receipt(&candidate);
        assert!(report.score_eligible, "{:?}", report.issues);

        {
            let contact = candidate
                .dimensions
                .iter_mut()
                .find(|mapping| mapping.dimension == Dimension::Contacts)
                .expect("contact dimension exists");
            contact.evidence_refs.clear();
        }
        let report = validate_receipt(&candidate);
        assert!(!report.score_eligible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "missing_not_applicable_evidence")
        );
    }

    #[test]
    fn not_provided_dimension_blocks_admission() {
        let mut candidate = receipt(MappingStatus::StrictlyEquivalent);
        let ccd = candidate
            .dimensions
            .iter_mut()
            .find(|mapping| mapping.dimension == Dimension::Ccd)
            .expect("CCD dimension exists");
        ccd.status = MappingStatus::NotProvided;

        let report = validate_receipt(&candidate);
        assert!(!report.score_eligible);
        assert_eq!(report.inadmissible_dimensions, vec![Dimension::Ccd]);
    }

    #[test]
    fn known_non_equivalence_never_enters_matched_score() {
        let mut candidate = receipt(MappingStatus::StrictlyEquivalent);
        let contact = candidate
            .dimensions
            .iter_mut()
            .find(|mapping| mapping.dimension == Dimension::Contacts)
            .expect("contact dimension exists");
        contact.status = MappingStatus::NonEquivalent;

        let report = validate_receipt(&candidate);
        assert!(!report.score_eligible);
        assert_eq!(report.inadmissible_dimensions, vec![Dimension::Contacts]);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn unknown_ccd_never_defaults_to_no_mismatch() {
        let mut candidate = receipt(MappingStatus::StrictlyEquivalent);
        let ccd = candidate
            .dimensions
            .iter_mut()
            .find(|mapping| mapping.dimension == Dimension::Ccd)
            .expect("CCD dimension exists");
        ccd.status = MappingStatus::UnknownUnassessed;

        let report = validate_receipt(&candidate);
        assert!(!report.score_eligible);
        assert_eq!(report.inadmissible_dimensions, vec![Dimension::Ccd]);
    }

    #[test]
    fn missing_and_duplicate_dimensions_are_rejected() {
        let mut candidate = receipt(MappingStatus::StrictlyEquivalent);
        candidate.dimensions.pop();
        candidate.dimensions.push(mapping(
            Dimension::Contacts,
            MappingStatus::StrictlyEquivalent,
        ));

        let report = validate_receipt(&candidate);
        assert!(!report.score_eligible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "duplicate_dimension")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "missing_dimension")
        );
    }

    #[test]
    fn self_review_is_rejected() {
        let mut candidate = receipt(MappingStatus::StrictlyEquivalent);
        candidate.review.reviewer_id = candidate.review.author_id.clone();

        let report = validate_receipt(&candidate);
        assert!(!report.score_eligible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "self_review_forbidden")
        );
    }

    #[test]
    fn non_finite_error_bound_is_rejected() {
        let mut candidate = receipt(MappingStatus::MappedEquivalentWithBound);
        candidate.dimensions[0].error_bounds[0].maximum_absolute_error = f64::NAN;

        let report = validate_receipt(&candidate);
        assert!(!report.score_eligible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "invalid_error_bound")
        );
    }

    #[test]
    fn json_strict_fixture_is_admitted() {
        let report =
            validate_json_receipt(include_str!("../fixtures/strict-equivalent.example.json"))
                .expect("strict fixture parses");
        assert!(report.score_eligible, "{:?}", report);
    }

    #[test]
    fn json_mapped_fixture_is_admitted() {
        let report =
            validate_json_receipt(include_str!("../fixtures/mapped-equivalent.example.json"))
                .expect("mapped fixture parses");
        assert!(report.score_eligible, "{:?}", report);
    }

    #[test]
    fn json_non_equivalent_friction_fixture_is_not_score_eligible() {
        let report = validate_json_receipt(include_str!(
            "../fixtures/non-equivalent-friction.example.json"
        ))
        .expect("non-equivalent fixture parses");
        assert!(!report.score_eligible);
        assert_eq!(report.inadmissible_dimensions, vec![Dimension::Contacts]);
    }

    #[test]
    fn json_unknown_ccd_fixture_fails_closed() {
        let report = validate_json_receipt(include_str!("../fixtures/unknown-ccd.example.json"))
            .expect("unknown CCD fixture parses");
        assert!(!report.score_eligible);
        assert_eq!(report.inadmissible_dimensions, vec![Dimension::Ccd]);
    }

    #[test]
    fn json_duplicate_dimension_fixture_fails_closed() {
        let report =
            validate_json_receipt(include_str!("../fixtures/duplicate-dimension.invalid.json"))
                .expect("duplicate dimension fixture parses");
        assert!(!report.score_eligible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "duplicate_dimension")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "missing_dimension")
        );
    }

    #[test]
    fn json_adapter_rejects_duplicate_object_keys() {
        let duplicate = r#"{"schema":"symtropy.semantic_mapping_receipt/0.1","schema":"symtropy.semantic_mapping_receipt/0.1"}"#;
        let error = parse_receipt_json(duplicate).expect_err("duplicate keys must fail");
        assert!(error.contains("duplicate JSON object key"));
    }

    #[test]
    fn json_adapter_rejects_nested_duplicate_object_keys() {
        let source = include_str!("../fixtures/strict-equivalent.example.json");
        let mutated = source.replacen(
            "\"engine_id\": \"symtropy\",",
            "\"engine_id\": \"symtropy\",\n    \"engine_id\": \"symtropy\",",
            1,
        );
        let error = parse_receipt_json(&mutated).expect_err("nested duplicate keys must fail");
        assert!(error.contains("duplicate JSON object key"));
    }

    #[test]
    fn draft_2020_12_schema_rejects_not_applicable_with_empty_error_bounds() {
        let mut value: JsonValue =
            serde_json::from_str(include_str!("../fixtures/strict-equivalent.example.json"))
                .expect("strict fixture is valid JSON");
        let dimensions = value["dimensions"]
            .as_array_mut()
            .expect("dimensions array");
        let contacts = dimensions
            .iter_mut()
            .find(|item| item["dimension"] == "contacts")
            .expect("contacts dimension");
        contacts["status"] = JsonValue::String("not_applicable".to_owned());
        contacts["error_bounds"] = JsonValue::Array(Vec::new());

        let encoded = serde_json::to_string(&value).expect("serialize mutated receipt");
        assert!(validate_receipt_schema_json(&encoded).is_err());
    }

    #[test]
    fn schema_gate_rejects_duplicate_object_keys() {
        let duplicate = r#"{"schema":"symtropy.semantic_mapping_receipt/0.1","schema":"symtropy.semantic_mapping_receipt/0.1"}"#;
        let error = validate_receipt_schema_json(duplicate)
            .expect_err("duplicate keys must fail schema gate");
        assert!(error.contains("duplicate JSON object key"));
    }

    #[test]
    fn draft_2020_12_schema_admits_positive_fixtures() {
        for fixture in [
            include_str!("../fixtures/strict-equivalent.example.json"),
            include_str!("../fixtures/mapped-equivalent.example.json"),
            include_str!("../fixtures/non-equivalent-friction.example.json"),
            include_str!("../fixtures/unknown-ccd.example.json"),
        ] {
            validate_receipt_schema_json(fixture).expect("schema accepts fixture");
        }
    }

    #[test]
    fn draft_2020_12_schema_rejects_unknown_fields() {
        let source = include_str!("../fixtures/strict-equivalent.example.json");
        let mutated = source.replacen("{", "{\n  \"unrecognized\": true,", 1);
        assert!(validate_receipt_schema_json(&mutated).is_err());
    }

    #[test]
    fn draft_2020_12_schema_rejects_duplicate_dimension_fixture() {
        assert!(
            validate_receipt_schema_json(include_str!(
                "../fixtures/duplicate-dimension.invalid.json"
            ))
            .is_err()
        );
    }

    #[test]
    fn schema_gate_precedes_semantic_admission() {
        let report = validate_schema_and_json_receipt(include_str!(
            "../fixtures/non-equivalent-friction.example.json"
        ))
        .expect("schema-valid exploratory receipt is structurally valid");
        assert!(!report.score_eligible);
        assert_eq!(report.inadmissible_dimensions, vec![Dimension::Contacts]);
    }

    #[test]
    fn json_adapter_rejects_not_applicable_with_empty_error_bounds() {
        let mut value: JsonValue =
            serde_json::from_str(include_str!("../fixtures/strict-equivalent.example.json"))
                .expect("strict fixture is valid JSON");
        let dimensions = value["dimensions"]
            .as_array_mut()
            .expect("dimensions array");
        let contacts = dimensions
            .iter_mut()
            .find(|item| item["dimension"] == "contacts")
            .expect("contacts dimension");
        contacts["status"] = JsonValue::String("not_applicable".to_owned());
        contacts["error_bounds"] = JsonValue::Array(Vec::new());

        let encoded = serde_json::to_string(&value).expect("serialize mutated receipt");
        assert!(parse_receipt_json(&encoded).is_err());
    }

    #[test]
    fn json_adapter_rejects_empty_optional_transformation() {
        let mut value: JsonValue =
            serde_json::from_str(include_str!("../fixtures/strict-equivalent.example.json"))
                .expect("strict fixture is valid JSON");
        let dimensions = value["dimensions"]
            .as_array_mut()
            .expect("dimensions array");
        dimensions[0]["transformation"] = JsonValue::String(String::new());

        let encoded = serde_json::to_string(&value).expect("serialize mutated receipt");
        assert!(parse_receipt_json(&encoded).is_err());
    }

    #[test]
    fn json_adapter_rejects_unknown_fields() {
        let source = include_str!("../fixtures/strict-equivalent.example.json");
        let mutated = source.replacen("{", "{\n  \"unrecognized\": true,", 1);
        assert!(validate_json_receipt(&mutated).is_err());
    }

    #[test]
    fn json_adapter_rejects_malformed_input() {
        assert!(validate_json_receipt("{ definitely not JSON").is_err());
    }

    #[test]
    fn invalid_configuration_digest_is_rejected() {
        let mut candidate = receipt(MappingStatus::StrictlyEquivalent);
        candidate.left_backend.profile_digest = "sha256:broken".to_owned();

        let report = validate_receipt(&candidate);
        assert!(!report.score_eligible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "invalid_sha256_digest")
        );
    }
}
