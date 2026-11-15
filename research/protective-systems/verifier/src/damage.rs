//! Conservative claim-state transitions after protective-system changes.
//!
//! This is a synthetic lifecycle semantics oracle, not an inspection system,
//! safety controller, certification mechanism, or authority to repair an asset.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimStatus {
    Qualified,
    Invalidated,
    NeedsReview,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Claim {
    pub claim_id: String,
    pub qualified_configuration_revision: String,
    pub component_ids: Vec<String>,
    pub material_lot_ids: Vec<String>,
    /// True only when the dependency inventory is complete for this claim.
    pub dependency_scope_complete: bool,
    pub status: ClaimStatus,
    /// New evidence required to requalify after invalidation.
    pub requalification_evidence_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Configuration {
    pub current_revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ChangeEvent {
    pub from_revision: String,
    pub to_revision: String,
    pub changed_component_ids: Vec<String>,
    pub changed_material_lot_ids: Vec<String>,
    /// True only when the event's impact/dependency mapping is complete.
    pub impact_map_complete: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Transition {
    pub claim_id: String,
    pub previous_status: ClaimStatus,
    pub new_status: ClaimStatus,
    pub reason_code: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateError {
    EmptyRevision,
    StaleOrConflictingChangeEvent,
    EmptyClaimIdentity,
    DuplicateClaimIdentity,
    EmptyRequalificationEvidence,
    ClaimAlreadyQualified,
    RequalificationRevisionMismatch,
    IndependentReviewRequired,
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StateError {}

/// Apply a configuration change using the declared dependency map.
///
/// Directly affected claims are invalidated. If the impact map is incomplete,
/// claims whose dependencies are not known to be complete move to NEEDS_REVIEW.
/// No claim is upgraded by this operation.
pub fn apply_change(
    configuration: &mut Configuration,
    claims: &mut [Claim],
    event: &ChangeEvent,
) -> Result<Vec<Transition>, StateError> {
    if configuration.current_revision.trim().is_empty()
        || event.to_revision.trim().is_empty()
        || event.from_revision.trim().is_empty()
    {
        return Err(StateError::EmptyRevision);
    }
    if event.from_revision != configuration.current_revision
        || event.from_revision == event.to_revision
    {
        return Err(StateError::StaleOrConflictingChangeEvent);
    }

    let mut ids = BTreeSet::new();
    for claim in claims.iter() {
        if claim.claim_id.trim().is_empty() {
            return Err(StateError::EmptyClaimIdentity);
        }
        if !ids.insert(claim.claim_id.clone()) {
            return Err(StateError::DuplicateClaimIdentity);
        }
    }

    let changed_components: BTreeSet<&str> = event
        .changed_component_ids
        .iter()
        .map(String::as_str)
        .collect();
    let changed_lots: BTreeSet<&str> = event
        .changed_material_lot_ids
        .iter()
        .map(String::as_str)
        .collect();

    let mut transitions = Vec::new();
    for claim in claims.iter_mut() {
        // INVALIDATED is monotonic until explicit requalification. A later
        // incomplete or unrelated event must never weaken it to NEEDS_REVIEW.
        if claim.status == ClaimStatus::Invalidated {
            continue;
        }

        let directly_affected = claim
            .component_ids
            .iter()
            .any(|id| changed_components.contains(id.as_str()))
            || claim
                .material_lot_ids
                .iter()
                .any(|id| changed_lots.contains(id.as_str()));

        let next = if directly_affected {
            Some((ClaimStatus::Invalidated, "DECLARED_DEPENDENCY_CHANGED"))
        } else if !event.impact_map_complete || !claim.dependency_scope_complete {
            Some((ClaimStatus::NeedsReview, "DEPENDENCY_IMPACT_UNRESOLVED"))
        } else {
            None
        };

        if let Some((new_status, reason_code)) = next
            && claim.status != new_status
        {
            transitions.push(Transition {
                claim_id: claim.claim_id.clone(),
                previous_status: claim.status.clone(),
                new_status: new_status.clone(),
                reason_code: reason_code.to_owned(),
            });
            claim.status = new_status;
            claim.requalification_evidence_id = None;
        }
    }

    configuration.current_revision = event.to_revision.clone();
    Ok(transitions)
}

/// Requalify one invalidated claim only against the current exact configuration
/// and an explicit independent-review evidence reference.
pub fn requalify_claim(
    claim: &mut Claim,
    current_configuration_revision: &str,
    evidence_configuration_revision: &str,
    evidence_id: &str,
    independent_review_complete: bool,
) -> Result<(), StateError> {
    if claim.status == ClaimStatus::Qualified {
        return Err(StateError::ClaimAlreadyQualified);
    }
    if current_configuration_revision.trim().is_empty()
        || evidence_configuration_revision != current_configuration_revision
    {
        return Err(StateError::RequalificationRevisionMismatch);
    }
    if evidence_id.trim().is_empty() {
        return Err(StateError::EmptyRequalificationEvidence);
    }
    if !independent_review_complete {
        return Err(StateError::IndependentReviewRequired);
    }

    claim.status = ClaimStatus::Qualified;
    claim.qualified_configuration_revision = current_configuration_revision.to_owned();
    claim.requalification_evidence_id = Some(evidence_id.to_owned());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(id: &str, components: &[&str], lots: &[&str], complete: bool) -> Claim {
        Claim {
            claim_id: id.to_owned(),
            qualified_configuration_revision: "R1".to_owned(),
            component_ids: components.iter().map(|s| (*s).to_owned()).collect(),
            material_lot_ids: lots.iter().map(|s| (*s).to_owned()).collect(),
            dependency_scope_complete: complete,
            status: ClaimStatus::Qualified,
            requalification_evidence_id: None,
        }
    }

    fn event(complete: bool) -> ChangeEvent {
        ChangeEvent {
            from_revision: "R1".to_owned(),
            to_revision: "R2".to_owned(),
            changed_component_ids: vec!["PANEL-A".to_owned()],
            changed_material_lot_ids: vec![],
            impact_map_complete: complete,
        }
    }

    #[test]
    fn only_claims_with_declared_changed_dependencies_are_invalidated() {
        let mut configuration = Configuration {
            current_revision: "R1".into(),
        };
        let mut claims = vec![
            claim("CLAIM-A", &["PANEL-A"], &[], true),
            claim("CLAIM-B", &["PANEL-B"], &["LOT-B"], true),
        ];

        let transitions = apply_change(&mut configuration, &mut claims, &event(true))
            .expect("valid change event");

        assert_eq!(configuration.current_revision, "R2");
        assert_eq!(claims[0].status, ClaimStatus::Invalidated);
        assert_eq!(claims[1].status, ClaimStatus::Qualified);
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].claim_id, "CLAIM-A");
    }

    #[test]
    fn incomplete_impact_map_does_not_leave_unknown_dependencies_qualified() {
        let mut configuration = Configuration {
            current_revision: "R1".into(),
        };
        let mut claims = vec![
            claim("DIRECT", &["PANEL-A"], &[], true),
            claim("UNKNOWN-DEPENDENCY", &["PANEL-B"], &[], false),
        ];

        apply_change(&mut configuration, &mut claims, &event(false))
            .expect("valid incomplete change event");

        assert_eq!(claims[0].status, ClaimStatus::Invalidated);
        assert_eq!(claims[1].status, ClaimStatus::NeedsReview);
    }

    #[test]
    fn a_repair_or_change_event_never_requalifies_a_claim() {
        let mut configuration = Configuration {
            current_revision: "R1".into(),
        };
        let mut claims = vec![claim("CLAIM-A", &["PANEL-A"], &[], true)];

        apply_change(&mut configuration, &mut claims, &event(true)).expect("change event");
        // Recording repair alone has no API that upgrades the claim.
        assert_eq!(claims[0].status, ClaimStatus::Invalidated);
        assert_eq!(claims[0].requalification_evidence_id, None);
    }

    #[test]
    fn requalification_requires_exact_current_revision_and_new_independent_evidence() {
        let mut claim = claim("CLAIM-A", &["PANEL-A"], &[], true);
        claim.status = ClaimStatus::Invalidated;

        assert_eq!(
            requalify_claim(&mut claim, "R2", "R2", "TEST-REPORT-NEW", false),
            Err(StateError::IndependentReviewRequired)
        );
        assert_eq!(
            requalify_claim(&mut claim, "R2", "R1", "STALE-REPORT", true),
            Err(StateError::RequalificationRevisionMismatch)
        );
        requalify_claim(&mut claim, "R2", "R2", "TEST-REPORT-NEW", true)
            .expect("correctly scoped requalification");
        assert_eq!(claim.status, ClaimStatus::Qualified);
        assert_eq!(claim.qualified_configuration_revision, "R2");
        assert_eq!(
            claim.requalification_evidence_id.as_deref(),
            Some("TEST-REPORT-NEW")
        );
    }

    #[test]
    fn stale_change_event_is_rejected_without_mutation() {
        let mut configuration = Configuration {
            current_revision: "R2".into(),
        };
        let mut claims = vec![claim("CLAIM-A", &["PANEL-A"], &[], true)];
        let before = claims.clone();

        let result = apply_change(&mut configuration, &mut claims, &event(true));
        assert_eq!(result, Err(StateError::StaleOrConflictingChangeEvent));
        assert_eq!(claims, before);
        assert_eq!(configuration.current_revision, "R2");
    }

    #[test]
    fn later_unknown_event_cannot_downgrade_invalidated_claim() {
        let mut configuration = Configuration {
            current_revision: "R1".into(),
        };
        let mut claims = vec![
            claim("ALREADY-INVALIDATED", &["PANEL-A"], &[], true),
            claim("OTHER", &["PANEL-B"], &[], false),
        ];

        apply_change(&mut configuration, &mut claims, &event(true))
            .expect("first event invalidates PANEL-A claim");
        assert_eq!(claims[0].status, ClaimStatus::Invalidated);
        assert_eq!(configuration.current_revision, "R2");

        let later = ChangeEvent {
            from_revision: "R2".into(),
            to_revision: "R3".into(),
            changed_component_ids: vec!["PANEL-C".into()],
            changed_material_lot_ids: vec![],
            impact_map_complete: false,
        };
        let transitions =
            apply_change(&mut configuration, &mut claims, &later).expect("second event");
        assert_eq!(claims[0].status, ClaimStatus::Invalidated);
        assert_eq!(claims[1].status, ClaimStatus::NeedsReview);
        assert!(
            !transitions
                .iter()
                .any(|t| t.claim_id == "ALREADY-INVALIDATED")
        );
    }

    #[test]
    fn material_lot_change_invalidates_dependent_claim() {
        let mut configuration = Configuration {
            current_revision: "R1".into(),
        };
        let mut claims = vec![
            claim("LOT-DEPENDENT", &["PANEL-B"], &["LOT-7"], true),
            claim("UNRELATED", &["PANEL-C"], &["LOT-8"], true),
        ];
        let mut lot_event = event(true);
        lot_event.changed_component_ids.clear();
        lot_event.changed_material_lot_ids = vec!["LOT-7".into()];

        apply_change(&mut configuration, &mut claims, &lot_event).expect("lot event");
        assert_eq!(claims[0].status, ClaimStatus::Invalidated);
        assert_eq!(claims[1].status, ClaimStatus::Qualified);
    }
}
