//! Fail-closed admission rules for cross-simulator physics comparisons.
//!
//! This crate validates the semantic *mapping decision*. It does not parse JSON,
//! execute a simulator, certify a physical model, or select a source of truth.
//! Transport adapters deserialize the versioned receipt into these types before
//! calling [validate_receipt].

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
