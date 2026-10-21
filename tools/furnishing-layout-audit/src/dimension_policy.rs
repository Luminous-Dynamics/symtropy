//! Policy-relative decisions for evidence-backed furnishing dimensions.
//!
//! A result accepted by this module means only that a structurally coherent
//! claim is permitted by the caller's explicit state policy. It does not
//! authenticate source bytes, determine freshness, certify a product, or prove
//! a room is accessible. Missing and conflicting evidence never becomes a pass.

use super::dimension_evidence::audit_dimension_evidence;
use serde_json::Value;
use std::collections::BTreeSet;

fn recognized_state(state: &str) -> bool {
    matches!(
        state,
        "measured" | "manufacturer_specified" | "seller_estimated" | "nominal" | "unknown"
    )
}

fn recognized_axis(axis: &str) -> bool {
    matches!(axis, "x" | "y" | "width" | "depth" | "height")
}

fn recognized_subject(kind: &str) -> bool {
    matches!(kind, "room" | "product" | "placement")
}

/// One caller-owned dimension requirement.
///
/// accepted_states must be nonempty and may contain only known states. The
/// list is use-case policy, not a universal ranking of evidence quality.
#[derive(Clone, Copy, Debug)]
pub struct DimensionRequirement<'a> {
    pub subject_kind: &'a str,
    pub subject_id: &'a str,
    pub axis: &'a str,
    pub accepted_states: &'a [&'a str],
    pub mandatory: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceDisposition {
    /// The single matching claim is structurally valid and allowed by policy.
    /// Source authenticity and freshness remain separate, unevaluated claims.
    AcceptedByPolicy,
    /// The claim can inform an explicitly provisional preview, not a hard pass.
    Provisional,
    /// Evidence is missing, unknown, or ambiguous; no numeric claim is selected.
    Indeterminate,
    /// A mandatory claim violates the caller's evidence-state policy.
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromotionDisposition {
    /// Every requested dimension is admissible under its explicit state policy.
    /// This is not a complete-layout, code, accessibility, or safety certificate.
    ReadyByPolicy,
    /// No mandatory requirement failed, but one or more dimensions are provisional.
    PreviewOnly,
    /// At least one mandatory dimension remains unknown or ambiguous.
    Indeterminate,
    /// Structural validation, policy configuration, or a mandatory requirement failed.
    Blocked,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DimensionAssessment {
    pub requirement_key: String,
    pub mandatory: bool,
    pub disposition: EvidenceDisposition,
    /// IDs are preserved in deterministic input order; ambiguous claims are not selected.
    pub evidence_ids: Vec<String>,
    /// Present only when the one matching record has finite, ordered numeric bounds.
    pub bounds_m: Option<(f64, f64)>,
    pub reason: String,
}

impl DimensionAssessment {
    /// Numeric bounds are exposed for hard-gate composition only after policy acceptance.
    pub fn accepted_bounds_m(&self) -> Option<(f64, f64)> {
        (self.disposition == EvidenceDisposition::AcceptedByPolicy)
            .then_some(self.bounds_m)
            .flatten()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DimensionPolicyReport {
    pub disposition: PromotionDisposition,
    pub structural_errors: Vec<String>,
    pub policy_errors: Vec<String>,
    pub dimensions: Vec<DimensionAssessment>,
}

fn record_string<'a>(record: &'a Value, parent: Option<&str>, key: &str) -> Option<&'a str> {
    let value = match parent {
        Some(parent) => record.get(parent)?.get(key)?,
        None => record.get(key)?,
    };
    value.as_str()
}

fn requirement_key(requirement: &DimensionRequirement<'_>) -> String {
    format!(
        "{}:{}:{}",
        requirement.subject_kind, requirement.subject_id, requirement.axis
    )
}

fn validate_requirement(requirement: &DimensionRequirement<'_>) -> Result<(), String> {
    if !recognized_subject(requirement.subject_kind) {
        return Err("subject_kind must be room, product, or placement".to_owned());
    }
    if requirement.subject_id.trim().is_empty() {
        return Err("subject_id must not be blank".to_owned());
    }
    if !recognized_axis(requirement.axis) {
        return Err("axis must be x, y, width, depth, or height".to_owned());
    }
    if requirement.accepted_states.is_empty() {
        return Err("accepted_states must not be empty".to_owned());
    }

    let mut seen = BTreeSet::new();
    for state in requirement.accepted_states {
        if !recognized_state(state) || *state == "unknown" {
            return Err(format!(
                "accepted state {state:?} is not a permitted known evidence state"
            ));
        }
        if !seen.insert(*state) {
            return Err(format!("accepted state {state:?} is duplicated"));
        }
    }
    Ok(())
}

fn matches_requirement(record: &Value, requirement: &DimensionRequirement<'_>) -> bool {
    record_string(record, Some("subject"), "kind") == Some(requirement.subject_kind)
        && record_string(record, Some("subject"), "subject_id") == Some(requirement.subject_id)
        && record_string(record, None, "axis") == Some(requirement.axis)
}

fn bounds(record: &Value) -> Option<(f64, f64)> {
    let bounds = record.get("bounds_m")?;
    let min = bounds.get("min_m")?.as_f64()?;
    let max = bounds.get("max_m")?.as_f64()?;
    (min.is_finite() && max.is_finite() && min <= max).then_some((min, max))
}

fn disposition_for_missing(mandatory: bool) -> EvidenceDisposition {
    if mandatory {
        EvidenceDisposition::Indeterminate
    } else {
        EvidenceDisposition::Provisional
    }
}

/// Evaluate required room/product/placement axes against explicit caller policy.
///
/// The existing structural audit runs first. If it fails, no dimension is
/// surfaced as policy-accepted. Multiple records for the same subject and axis
/// are always indeterminate in v0.1: this function deliberately does not merge,
/// average, or select conflicting observations based on timestamps or precision.
pub fn assess_dimensions(
    document: &Value,
    requirements: &[DimensionRequirement<'_>],
) -> DimensionPolicyReport {
    let structural_errors = audit_dimension_evidence(document);
    let mut policy_errors = Vec::new();
    let mut dimensions = Vec::new();

    if !structural_errors.is_empty() {
        return DimensionPolicyReport {
            disposition: PromotionDisposition::Blocked,
            structural_errors,
            policy_errors,
            dimensions,
        };
    }

    if requirements.is_empty() {
        policy_errors.push("requirements must not be empty".to_owned());
    }

    let Some(records) = document.get("records").and_then(Value::as_array) else {
        return DimensionPolicyReport {
            disposition: PromotionDisposition::Blocked,
            structural_errors: vec!["records must be an array".to_owned()],
            policy_errors,
            dimensions,
        };
    };

    for requirement in requirements {
        let key = requirement_key(requirement);
        if let Err(error) = validate_requirement(requirement) {
            policy_errors.push(format!("{key}: {error}"));
            dimensions.push(DimensionAssessment {
                requirement_key: key,
                mandatory: requirement.mandatory,
                disposition: EvidenceDisposition::Blocked,
                evidence_ids: Vec::new(),
                bounds_m: None,
                reason: error,
            });
            continue;
        }

        let matching = records
            .iter()
            .filter(|record| matches_requirement(record, requirement))
            .collect::<Vec<_>>();
        let evidence_ids = matching
            .iter()
            .filter_map(|record| record_string(record, None, "evidence_id"))
            .map(str::to_owned)
            .collect::<Vec<_>>();

        if matching.is_empty() {
            let disposition = disposition_for_missing(requirement.mandatory);
            dimensions.push(DimensionAssessment {
                requirement_key: key,
                mandatory: requirement.mandatory,
                disposition,
                evidence_ids,
                bounds_m: None,
                reason: "no evidence record matches this subject and axis".to_owned(),
            });
            continue;
        }

        if matching.len() > 1 {
            dimensions.push(DimensionAssessment {
                requirement_key: key,
                mandatory: requirement.mandatory,
                disposition: EvidenceDisposition::Indeterminate,
                evidence_ids,
                bounds_m: None,
                reason: "multiple claims match this subject and axis; v0.1 preserves the conflict instead of selecting one".to_owned(),
            });
            continue;
        }

        let record = matching[0];
        let state = record_string(record, None, "state").unwrap_or("");
        if state == "unknown" {
            dimensions.push(DimensionAssessment {
                requirement_key: key,
                mandatory: requirement.mandatory,
                disposition: disposition_for_missing(requirement.mandatory),
                evidence_ids,
                bounds_m: None,
                reason: "the matching record explicitly declares the dimension unknown".to_owned(),
            });
            continue;
        }

        let claim_bounds = bounds(record);
        if requirement.accepted_states.contains(&state) {
            dimensions.push(DimensionAssessment {
                requirement_key: key,
                mandatory: requirement.mandatory,
                disposition: EvidenceDisposition::AcceptedByPolicy,
                evidence_ids,
                bounds_m: claim_bounds,
                reason: "one structurally valid claim matches an allowed state; source authenticity and freshness were not evaluated".to_owned(),
            });
        } else if requirement.mandatory {
            dimensions.push(DimensionAssessment {
                requirement_key: key,
                mandatory: true,
                disposition: EvidenceDisposition::Blocked,
                evidence_ids,
                bounds_m: claim_bounds,
                reason: format!(
                    "evidence state {state:?} is not allowed for this mandatory requirement"
                ),
            });
        } else {
            dimensions.push(DimensionAssessment {
                requirement_key: key,
                mandatory: false,
                disposition: EvidenceDisposition::Provisional,
                evidence_ids,
                bounds_m: claim_bounds,
                reason: format!(
                    "evidence state {state:?} is not in the allowed set; retained for preview only"
                ),
            });
        }
    }

    let disposition = if !policy_errors.is_empty()
        || dimensions
            .iter()
            .any(|item| item.mandatory && item.disposition == EvidenceDisposition::Blocked)
    {
        PromotionDisposition::Blocked
    } else if dimensions
        .iter()
        .any(|item| item.mandatory && item.disposition == EvidenceDisposition::Indeterminate)
    {
        PromotionDisposition::Indeterminate
    } else if dimensions
        .iter()
        .any(|item| item.disposition != EvidenceDisposition::AcceptedByPolicy)
    {
        PromotionDisposition::PreviewOnly
    } else {
        PromotionDisposition::ReadyByPolicy
    };

    DimensionPolicyReport {
        disposition,
        structural_errors,
        policy_errors,
        dimensions,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DimensionRequirement, EvidenceDisposition, PromotionDisposition, assess_dimensions,
    };
    use serde_json::{Value, json};
    use std::{fs, path::PathBuf};

    const MEASURED_ONLY: &[&str] = &["measured"];
    const HARD_FIT_STATES: &[&str] = &["measured", "manufacturer_specified"];

    fn fixture() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/interiors/furnishing_dimension_evidence_v0_1.json");
        let bytes = fs::read(&path).expect("dimension evidence fixture must exist");
        serde_json::from_slice(&bytes).expect("dimension evidence fixture must be JSON")
    }

    fn requirement<'a>(
        subject_kind: &'a str,
        subject_id: &'a str,
        axis: &'a str,
        accepted_states: &'a [&'a str],
        mandatory: bool,
    ) -> DimensionRequirement<'a> {
        DimensionRequirement {
            subject_kind,
            subject_id,
            axis,
            accepted_states,
            mandatory,
        }
    }

    #[test]
    fn measured_room_width_is_accepted_by_explicit_hard_fit_policy() {
        let document = fixture();
        let requirement = requirement("room", "fixture-room-001", "width", MEASURED_ONLY, true);
        let report = assess_dimensions(&document, &[requirement]);
        assert_eq!(report.disposition, PromotionDisposition::ReadyByPolicy);
        let dimension = &report.dimensions[0];
        assert_eq!(dimension.disposition, EvidenceDisposition::AcceptedByPolicy);
        assert_eq!(dimension.bounds_m, Some((3.98, 4.02)));
        assert_eq!(dimension.accepted_bounds_m(), Some((3.98, 4.02)));
        assert!(
            dimension
                .reason
                .contains("authenticity and freshness were not evaluated")
        );
    }

    #[test]
    fn manufacturer_product_dimension_can_be_allowed_for_fit_policy() {
        let document = fixture();
        let requirement = requirement(
            "product",
            "fixture-cabinet-revision-a",
            "width",
            HARD_FIT_STATES,
            true,
        );
        let report = assess_dimensions(&document, &[requirement]);
        assert_eq!(report.disposition, PromotionDisposition::ReadyByPolicy);
        assert_eq!(report.dimensions[0].bounds_m, Some((0.792, 0.808)));
    }

    #[test]
    fn seller_estimate_is_blocked_when_mandatory_policy_disallows_it() {
        let document = fixture();
        let requirement = requirement(
            "product",
            "fixture-chair-listing-17",
            "depth",
            HARD_FIT_STATES,
            true,
        );
        let report = assess_dimensions(&document, &[requirement]);
        assert_eq!(report.disposition, PromotionDisposition::Blocked);
        assert_eq!(
            report.dimensions[0].disposition,
            EvidenceDisposition::Blocked
        );
        assert_eq!(report.dimensions[0].accepted_bounds_m(), None);
        assert!(report.dimensions[0].bounds_m.is_some());
    }

    #[test]
    fn explicitly_unknown_required_dimension_remains_indeterminate() {
        let document = fixture();
        let requirement = requirement(
            "product",
            "fixture-unverified-stool",
            "width",
            HARD_FIT_STATES,
            true,
        );
        let report = assess_dimensions(&document, &[requirement]);
        assert_eq!(report.disposition, PromotionDisposition::Indeterminate);
        assert_eq!(
            report.dimensions[0].disposition,
            EvidenceDisposition::Indeterminate
        );
        assert_eq!(report.dimensions[0].bounds_m, None);
    }

    #[test]
    fn missing_mandatory_dimension_is_indeterminate_not_clear() {
        let document = fixture();
        let requirement = requirement("placement", "missing-placement", "x", MEASURED_ONLY, true);
        let report = assess_dimensions(&document, &[requirement]);
        assert_eq!(report.disposition, PromotionDisposition::Indeterminate);
        assert_eq!(report.dimensions[0].accepted_bounds_m(), None);
    }

    #[test]
    fn missing_nonmandatory_dimension_is_preview_only() {
        let document = fixture();
        let requirement = requirement("placement", "optional-placement", "y", MEASURED_ONLY, false);
        let report = assess_dimensions(&document, &[requirement]);
        assert_eq!(report.disposition, PromotionDisposition::PreviewOnly);
        assert_eq!(
            report.dimensions[0].disposition,
            EvidenceDisposition::Provisional
        );
    }

    #[test]
    fn multiple_claims_for_one_axis_remain_unresolved() {
        let mut document = fixture();
        let mut duplicate = document["records"][0].clone();
        duplicate["evidence_id"] = json!("DIM-EVID-005");
        document["records"]
            .as_array_mut()
            .expect("records array")
            .push(duplicate);
        let requirement = requirement(
            "product",
            "fixture-cabinet-revision-a",
            "width",
            HARD_FIT_STATES,
            true,
        );
        let report = assess_dimensions(&document, &[requirement]);
        assert!(report.structural_errors.is_empty());
        assert_eq!(report.disposition, PromotionDisposition::Indeterminate);
        assert_eq!(report.dimensions[0].disposition, EvidenceDisposition::Indeterminate);
        assert_eq!(report.dimensions[0].evidence_ids.len(), 2);
        assert_eq!(report.dimensions[0].accepted_bounds_m(), None);
    }

    #[test]
    fn invalid_document_blocks_every_policy_promotion() {
        let mut document = fixture();
        document["units"]["length"] = json!("feet");
        let requirement = requirement("room", "fixture-room-001", "width", MEASURED_ONLY, true);
        let report = assess_dimensions(&document, &[requirement]);
        assert_eq!(report.disposition, PromotionDisposition::Blocked);
        assert!(!report.structural_errors.is_empty());
        assert!(report.dimensions.is_empty());
    }

    #[test]
    fn empty_or_unknown_acceptance_policy_is_rejected() {
        let document = fixture();
        let empty = requirement("room", "fixture-room-001", "width", &[], true);
        let unknown = requirement("room", "fixture-room-001", "width", &["unknown"], true);
        let empty_report = assess_dimensions(&document, &[empty]);
        let unknown_report = assess_dimensions(&document, &[unknown]);
        assert_eq!(empty_report.disposition, PromotionDisposition::Blocked);
        assert_eq!(unknown_report.disposition, PromotionDisposition::Blocked);
        assert!(!unknown_report.policy_errors.is_empty());
    }
}
