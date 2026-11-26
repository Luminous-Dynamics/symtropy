//! Fail-closed composition of an upstream feasible resource-plan receipt
//! with participant commitment projection receipts.
//!
//! This crate does not solve resource allocation, verify physical facts,
//! authenticate commitments, issue permissions, reconcile custody or execute
//! external actions. Those responsibilities remain with their owning systems.
//! It checks only that every declared reservation has exactly one eligible
//! commitment projection with matching terms at the same knowledge cutoff.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalDisposition {
    Feasible,
    Infeasible,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedReservation {
    pub reservation_id: String,
    pub resource_id: String,
    pub slot: u32,
    pub quantity: u64,
    pub unit: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePlanReceipt {
    pub schema_version: String,
    pub plan_id: String,
    pub scenario_digest: String,
    pub plan_digest: String,
    pub physical_disposition: PhysicalDisposition,
    pub reservations: Vec<PlannedReservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitmentRequirement {
    pub reservation_id: String,
    pub commitment_id: String,
    pub revision: u32,
    pub controller_id: String,
    pub accepting_participant_id: String,
    pub resource_id: String,
    pub slot: u32,
    pub quantity: u64,
    pub unit: String,
    pub terms_digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitmentState {
    Offered,
    Accepted,
    Rejected,
    Revoked,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitmentProjectionReceipt {
    pub schema_version: String,
    pub commitment_id: String,
    pub revision: u32,
    pub controller_id: String,
    pub accepting_participant_id: String,
    pub resource_id: String,
    pub slot: u32,
    pub quantity: u64,
    pub unit: String,
    pub expires_at: i64,
    pub terms_digest: String,
    pub state: CommitmentState,
    pub knowledge_cutoff: i64,
    pub accepted_at: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderError {
    Unavailable,
    InvalidReceipt,
    ConflictingProjection,
}

pub trait CommitmentProjectionProvider {
    fn project(
        &self,
        commitment_id: &str,
        knowledge_cutoff: i64,
    ) -> Result<Option<CommitmentProjectionReceipt>, ProviderError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequirementDisposition {
    Satisfied,
    MissingRequirement,
    OrphanRequirement,
    DuplicateRequirement,
    CommitmentMissing,
    CommitmentUnavailable,
    CommitmentInvalid,
    CommitmentNotAccepted,
    CommitmentExpired,
    TermsMismatch,
    DuplicateCommitmentReuse,
}

impl RequirementDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "SATISFIED",
            Self::MissingRequirement => "MISSING_REQUIREMENT",
            Self::OrphanRequirement => "ORPHAN_REQUIREMENT",
            Self::DuplicateRequirement => "DUPLICATE_REQUIREMENT",
            Self::CommitmentMissing => "COMMITMENT_MISSING",
            Self::CommitmentUnavailable => "COMMITMENT_UNAVAILABLE",
            Self::CommitmentInvalid => "COMMITMENT_INVALID",
            Self::CommitmentNotAccepted => "COMMITMENT_NOT_ACCEPTED",
            Self::CommitmentExpired => "COMMITMENT_EXPIRED",
            Self::TermsMismatch => "TERMS_MISMATCH",
            Self::DuplicateCommitmentReuse => "DUPLICATE_COMMITMENT_REUSE",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementResult {
    pub reservation_id: String,
    pub commitment_id: Option<String>,
    pub disposition: RequirementDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleDisposition {
    Complete,
    Incomplete,
    PhysicalPlanInfeasible,
    PhysicalPlanUnknown,
    InvalidPlanReceipt,
}

impl BundleDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "COMPLETE",
            Self::Incomplete => "INCOMPLETE",
            Self::PhysicalPlanInfeasible => "PHYSICAL_PLAN_INFEASIBLE",
            Self::PhysicalPlanUnknown => "PHYSICAL_PLAN_UNKNOWN",
            Self::InvalidPlanReceipt => "INVALID_PLAN_RECEIPT",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleReport {
    pub plan_id: String,
    pub plan_digest: String,
    pub knowledge_cutoff: i64,
    pub disposition: BundleDisposition,
    pub results: Vec<RequirementResult>,
}

impl BundleReport {
    pub fn is_complete(&self) -> bool {
        self.disposition == BundleDisposition::Complete
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BundleError {
    EmptyIdentifier,
    DuplicateReservation(String),
    DuplicateCommitmentIdentity(String),
    EmptyReservationSet,
    EmptyRequirementUnit,
    ZeroQuantity,
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BundleError {}

fn validate_plan_receipt(receipt: &ResourcePlanReceipt) -> Result<(), BundleError> {
    if receipt.schema_version != "maritime-resource-plan/0.1"
        || receipt.plan_id.is_empty()
        || receipt.scenario_digest.is_empty()
        || receipt.plan_digest.is_empty()
    {
        return Err(BundleError::EmptyIdentifier);
    }
    if receipt.reservations.is_empty() {
        return Err(BundleError::EmptyReservationSet);
    }

    let mut reservation_ids = BTreeSet::new();
    for reservation in &receipt.reservations {
        if reservation.reservation_id.is_empty() || reservation.resource_id.is_empty() {
            return Err(BundleError::EmptyIdentifier);
        }
        if reservation.unit.is_empty() {
            return Err(BundleError::EmptyRequirementUnit);
        }
        if reservation.quantity == 0 {
            return Err(BundleError::ZeroQuantity);
        }
        if !reservation_ids.insert(reservation.reservation_id.clone()) {
            return Err(BundleError::DuplicateReservation(
                reservation.reservation_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_requirements(
    requirements: &[CommitmentRequirement],
) -> Result<(), BundleError> {
    let mut identities = BTreeSet::new();
    for requirement in requirements {
        if requirement.reservation_id.is_empty()
            || requirement.commitment_id.is_empty()
            || requirement.controller_id.is_empty()
            || requirement.accepting_participant_id.is_empty()
            || requirement.resource_id.is_empty()
            || requirement.terms_digest.is_empty()
        {
            return Err(BundleError::EmptyIdentifier);
        }
        if requirement.unit.is_empty() {
            return Err(BundleError::EmptyRequirementUnit);
        }
        if requirement.quantity == 0 {
            return Err(BundleError::ZeroQuantity);
        }
        let identity = (requirement.commitment_id.clone(), requirement.revision);
        if !identities.insert(identity) {
            return Err(BundleError::DuplicateCommitmentIdentity(
                requirement.commitment_id.clone(),
            ));
        }
    }
    Ok(())
}

fn terms_match(
    reservation: &PlannedReservation,
    requirement: &CommitmentRequirement,
    projection: &CommitmentProjectionReceipt,
) -> bool {
    requirement.reservation_id == reservation.reservation_id
        && requirement.resource_id == reservation.resource_id
        && requirement.slot == reservation.slot
        && requirement.quantity == reservation.quantity
        && requirement.unit == reservation.unit
        && projection.commitment_id == requirement.commitment_id
        && projection.revision == requirement.revision
        && projection.controller_id == requirement.controller_id
        && projection.accepting_participant_id == requirement.accepting_participant_id
        && projection.resource_id == reservation.resource_id
        && projection.slot == reservation.slot
        && projection.quantity == reservation.quantity
        && projection.unit == reservation.unit
        && projection.terms_digest == requirement.terms_digest
}

fn check_requirement<P: CommitmentProjectionProvider>(
    reservation: &PlannedReservation,
    requirement: Option<&CommitmentRequirement>,
    knowledge_cutoff: i64,
    provider: &P,
) -> RequirementResult {
    let Some(requirement) = requirement else {
        return RequirementResult {
            reservation_id: reservation.reservation_id.clone(),
            commitment_id: None,
            disposition: RequirementDisposition::MissingRequirement,
        };
    };

    let commitment_id = Some(requirement.commitment_id.clone());
    let projection = match provider.project(&requirement.commitment_id, knowledge_cutoff) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return RequirementResult {
                reservation_id: reservation.reservation_id.clone(),
                commitment_id,
                disposition: RequirementDisposition::CommitmentMissing,
            }
        }
        Err(ProviderError::Unavailable) => {
            return RequirementResult {
                reservation_id: reservation.reservation_id.clone(),
                commitment_id,
                disposition: RequirementDisposition::CommitmentUnavailable,
            }
        }
        Err(ProviderError::InvalidReceipt | ProviderError::ConflictingProjection) => {
            return RequirementResult {
                reservation_id: reservation.reservation_id.clone(),
                commitment_id,
                disposition: RequirementDisposition::CommitmentInvalid,
            }
        }
    };

    if projection.schema_version != "maritime-commitment-projection/0.1"
        || projection.knowledge_cutoff != knowledge_cutoff
    {
        return RequirementResult {
            reservation_id: reservation.reservation_id.clone(),
            commitment_id,
            disposition: RequirementDisposition::CommitmentInvalid,
        };
    }

    if !terms_match(reservation, requirement, &projection) {
        return RequirementResult {
            reservation_id: reservation.reservation_id.clone(),
            commitment_id,
            disposition: RequirementDisposition::TermsMismatch,
        };
    }

    match projection.state {
        CommitmentState::Accepted => {
            if projection.accepted_at.is_none_or(|accepted_at| accepted_at > knowledge_cutoff) {
                RequirementResult {
                    reservation_id: reservation.reservation_id.clone(),
                    commitment_id,
                    disposition: RequirementDisposition::CommitmentInvalid,
                }
            } else if projection.expires_at <= knowledge_cutoff {
                RequirementResult {
                    reservation_id: reservation.reservation_id.clone(),
                    commitment_id,
                    disposition: RequirementDisposition::CommitmentExpired,
                }
            } else {
                RequirementResult {
                    reservation_id: reservation.reservation_id.clone(),
                    commitment_id,
                    disposition: RequirementDisposition::Satisfied,
                }
            }
        }
        CommitmentState::Revoked | CommitmentState::Rejected | CommitmentState::Expired => {
            RequirementResult {
                reservation_id: reservation.reservation_id.clone(),
                commitment_id,
                disposition: RequirementDisposition::CommitmentNotAccepted,
            }
        }
        CommitmentState::Offered => RequirementResult {
            reservation_id: reservation.reservation_id.clone(),
            commitment_id,
            disposition: RequirementDisposition::CommitmentNotAccepted,
        },
    }
}

/// Checks commitment coverage for a plan receipt that was already produced by
/// the physical resource owner. The physical receipt is consumed as a typed
/// input; no physical constraints are recomputed here.
///
/// Exactly one requirement must cover every planned reservation. The
/// provider must project the same commitment revision at the exact receiver
/// knowledge cutoff. A complete report is not execution authorization.
pub fn evaluate_bundle<P: CommitmentProjectionProvider>(
    receipt: &ResourcePlanReceipt,
    requirements: &[CommitmentRequirement],
    knowledge_cutoff: i64,
    provider: &P,
) -> Result<BundleReport, BundleError> {
    validate_plan_receipt(receipt)?;
    validate_requirements(requirements)?;

    let mut results = Vec::new();
    if receipt.physical_disposition != PhysicalDisposition::Feasible {
        let disposition = match receipt.physical_disposition {
            PhysicalDisposition::Feasible => unreachable!(),
            PhysicalDisposition::Infeasible => BundleDisposition::PhysicalPlanInfeasible,
            PhysicalDisposition::Unknown => BundleDisposition::PhysicalPlanUnknown,
        };
        return Ok(BundleReport {
            plan_id: receipt.plan_id.clone(),
            plan_digest: receipt.plan_digest.clone(),
            knowledge_cutoff,
            disposition,
            results,
        });
    }

    let mut grouped: BTreeMap<&str, Vec<&CommitmentRequirement>> = BTreeMap::new();
    for requirement in requirements {
        grouped
            .entry(requirement.reservation_id.as_str())
            .or_default()
            .push(requirement);
    }

    for reservation in &receipt.reservations {
        let matching = grouped
            .get(reservation.reservation_id.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[]);

        let result = match matching {
            [] => check_requirement(reservation, None, knowledge_cutoff, provider),
            [requirement] => check_requirement(
                reservation,
                Some(requirement),
                knowledge_cutoff,
                provider,
            ),
            _ => RequirementResult {
                reservation_id: reservation.reservation_id.clone(),
                commitment_id: None,
                disposition: RequirementDisposition::DuplicateRequirement,
            },
        };
        results.push(result);
    }

    let reservation_ids: BTreeSet<&str> = receipt
        .reservations
        .iter()
        .map(|reservation| reservation.reservation_id.as_str())
        .collect();
    for reservation_id in grouped
        .keys()
        .filter(|reservation_id| !reservation_ids.contains(**reservation_id))
    {
        results.push(RequirementResult {
            reservation_id: (*reservation_id).to_owned(),
            commitment_id: None,
            disposition: RequirementDisposition::OrphanRequirement,
        });
    }

    let all_satisfied = results
            .iter()
            .all(|result| result.disposition == RequirementDisposition::Satisfied);

    Ok(BundleReport {
        plan_id: receipt.plan_id.clone(),
        plan_digest: receipt.plan_digest.clone(),
        knowledge_cutoff,
        disposition: if all_satisfied {
            BundleDisposition::Complete
        } else {
            BundleDisposition::Incomplete
        },
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct StaticProvider {
        projections: BTreeMap<String, CommitmentProjectionReceipt>,
        unavailable: BTreeSet<String>,
        missing: BTreeSet<String>,
    }

    impl CommitmentProjectionProvider for StaticProvider {
        fn project(
            &self,
            commitment_id: &str,
            _knowledge_cutoff: i64,
        ) -> Result<Option<CommitmentProjectionReceipt>, ProviderError> {
            if self.unavailable.contains(commitment_id) {
                return Err(ProviderError::Unavailable);
            }
            if self.missing.contains(commitment_id) {
                return Ok(None);
            }
            Ok(self.projections.get(commitment_id).cloned())
        }
    }

    fn reservation(id: &str, resource: &str, slot: u32, quantity: u64, unit: &str) -> PlannedReservation {
        PlannedReservation {
            reservation_id: id.to_owned(),
            resource_id: resource.to_owned(),
            slot,
            quantity,
            unit: unit.to_owned(),
        }
    }

    fn requirement(
        reservation_id: &str,
        commitment_id: &str,
        resource_id: &str,
        slot: u32,
        quantity: u64,
        unit: &str,
    ) -> CommitmentRequirement {
        CommitmentRequirement {
            reservation_id: reservation_id.to_owned(),
            commitment_id: commitment_id.to_owned(),
            revision: 1,
            controller_id: "TERMINAL-A".to_owned(),
            accepting_participant_id: "CARRIER-B".to_owned(),
            resource_id: resource_id.to_owned(),
            slot,
            quantity,
            unit: unit.to_owned(),
            terms_digest: format!("terms-{commitment_id}"),
        }
    }

    fn projection(
        commitment_id: &str,
        resource_id: &str,
        slot: u32,
        quantity: u64,
        unit: &str,
        state: CommitmentState,
        cutoff: i64,
    ) -> CommitmentProjectionReceipt {
        CommitmentProjectionReceipt {
            schema_version: "maritime-commitment-projection/0.1".to_owned(),
            commitment_id: commitment_id.to_owned(),
            revision: 1,
            controller_id: "TERMINAL-A".to_owned(),
            accepting_participant_id: "CARRIER-B".to_owned(),
            resource_id: resource_id.to_owned(),
            slot,
            quantity,
            unit: unit.to_owned(),
            expires_at: 200,
            terms_digest: format!("terms-{commitment_id}"),
            state,
            knowledge_cutoff: cutoff,
            accepted_at: if state == CommitmentState::Accepted {
                Some(90)
            } else {
                None
            },
        }
    }

    fn plan() -> ResourcePlanReceipt {
        ResourcePlanReceipt {
            schema_version: "maritime-resource-plan/0.1".to_owned(),
            plan_id: "PLAN-1".to_owned(),
            scenario_digest: "scenario-digest".to_owned(),
            plan_digest: "plan-digest".to_owned(),
            physical_disposition: PhysicalDisposition::Feasible,
            reservations: vec![
                reservation("R-BERTH", "BERTH-2", 4, 12, "container"),
                reservation("R-RAIL", "RAIL-TRACK-A", 5, 12, "container"),
            ],
        }
    }

    fn provider() -> StaticProvider {
        let mut value = StaticProvider::default();
        value.projections.insert(
            "C-BERTH".to_owned(),
            projection(
                "C-BERTH",
                "BERTH-2",
                4,
                12,
                "container",
                CommitmentState::Accepted,
                120,
            ),
        );
        value.projections.insert(
            "C-RAIL".to_owned(),
            projection(
                "C-RAIL",
                "RAIL-TRACK-A",
                5,
                12,
                "container",
                CommitmentState::Accepted,
                120,
            ),
        );
        value
    }

    fn requirements() -> Vec<CommitmentRequirement> {
        vec![
            requirement("R-BERTH", "C-BERTH", "BERTH-2", 4, 12, "container"),
            requirement(
                "R-RAIL",
                "C-RAIL",
                "RAIL-TRACK-A",
                5,
                12,
                "container",
            ),
        ]
    }

    #[test]
    fn complete_berth_and_rail_bundle_is_satisfied() {
        let report = evaluate_bundle(&plan(), &requirements(), 120, &provider()).unwrap();
        assert!(report.is_complete());
        assert_eq!(report.results.len(), 2);
        assert!(report
            .results
            .iter()
            .all(|result| result.disposition == RequirementDisposition::Satisfied));
    }

    #[test]
    fn partial_acceptance_is_not_promoted_to_complete() {
        let mut value = provider();
        value.projections.insert(
            "C-RAIL".to_owned(),
            projection(
                "C-RAIL",
                "RAIL-TRACK-A",
                5,
                12,
                "container",
                CommitmentState::Offered,
                120,
            ),
        );
        let report = evaluate_bundle(&plan(), &requirements(), 120, &value).unwrap();
        assert_eq!(report.disposition, BundleDisposition::Incomplete);
        assert_eq!(
            report.results[1].disposition,
            RequirementDisposition::CommitmentNotAccepted
        );
    }

    #[test]
    fn te_u_and_container_units_do_not_match() {
        let mut value = provider();
        value.projections.insert(
            "C-RAIL".to_owned(),
            projection(
                "C-RAIL",
                "RAIL-TRACK-A",
                5,
                12,
                "TEU",
                CommitmentState::Accepted,
                120,
            ),
        );
        let report = evaluate_bundle(&plan(), &requirements(), 120, &value).unwrap();
        assert_eq!(
            report.results[1].disposition,
            RequirementDisposition::TermsMismatch
        );
        assert_eq!(report.disposition, BundleDisposition::Incomplete);
    }

    #[test]
    fn changed_quantity_or_slot_is_rejected() {
        let mut wrong = requirements();
        wrong[1].quantity = 11;
        wrong[1].slot = 6;
        let report = evaluate_bundle(&plan(), &wrong, 120, &provider()).unwrap();
        assert_eq!(
            report.results[1].disposition,
            RequirementDisposition::TermsMismatch
        );
    }

    #[test]
    fn revoked_commitment_is_not_eligible() {
        let mut value = provider();
        value.projections.insert(
            "C-BERTH".to_owned(),
            projection(
                "C-BERTH",
                "BERTH-2",
                4,
                12,
                "container",
                CommitmentState::Revoked,
                120,
            ),
        );
        let report = evaluate_bundle(&plan(), &requirements(), 120, &value).unwrap();
        assert_eq!(
            report.results[0].disposition,
            RequirementDisposition::CommitmentNotAccepted
        );
    }

    #[test]
    fn expiry_at_cutoff_is_not_eligible() {
        let mut value = provider();
        let mut expired = projection(
            "C-BERTH",
            "BERTH-2",
            4,
            12,
            "container",
            CommitmentState::Accepted,
            120,
        );
        expired.expires_at = 120;
        value.projections.insert("C-BERTH".to_owned(), expired);
        let report = evaluate_bundle(&plan(), &requirements(), 120, &value).unwrap();
        assert_eq!(
            report.results[0].disposition,
            RequirementDisposition::CommitmentExpired
        );
    }

    #[test]
    fn commitment_projection_must_use_same_knowledge_cutoff() {
        let mut value = provider();
        let mut stale = projection(
            "C-BERTH",
            "BERTH-2",
            4,
            12,
            "container",
            CommitmentState::Accepted,
            100,
        );
        stale.knowledge_cutoff = 100;
        value.projections.insert("C-BERTH".to_owned(), stale);
        let report = evaluate_bundle(&plan(), &requirements(), 120, &value).unwrap();
        assert_eq!(
            report.results[0].disposition,
            RequirementDisposition::CommitmentInvalid
        );
    }

    #[test]
    fn missing_commitment_is_incomplete() {
        let mut value = provider();
        value.missing.insert("C-RAIL".to_owned());
        let report = evaluate_bundle(&plan(), &requirements(), 120, &value).unwrap();
        assert_eq!(
            report.results[1].disposition,
            RequirementDisposition::CommitmentMissing
        );
        assert_eq!(report.disposition, BundleDisposition::Incomplete);
    }

    #[test]
    fn unknown_physical_plan_cannot_be_promoted_by_commitments() {
        let mut receipt = plan();
        receipt.physical_disposition = PhysicalDisposition::Unknown;
        let report = evaluate_bundle(&receipt, &requirements(), 120, &provider()).unwrap();
        assert_eq!(report.disposition, BundleDisposition::PhysicalPlanUnknown);
        assert!(report.results.is_empty());
    }

    #[test]
    fn duplicate_requirement_for_one_reservation_is_not_accepted() {
        let mut reqs = requirements();
        reqs.push(reqs[0].clone());
        let report = evaluate_bundle(&plan(), &reqs, 120, &provider()).unwrap();
        assert_eq!(
            report.results[0].disposition,
            RequirementDisposition::DuplicateRequirement
        );
        assert_eq!(report.disposition, BundleDisposition::Incomplete);
    }

    #[test]
    fn duplicate_commitment_identity_cannot_cover_two_reservations() {
        let mut reqs = requirements();
        reqs[1].commitment_id = reqs[0].commitment_id.clone();
        reqs[1].revision = reqs[0].revision;
        assert_eq!(
            evaluate_bundle(&plan(), &reqs, 120, &provider()),
            Err(BundleError::DuplicateCommitmentIdentity("C-BERTH".to_owned()))
        );
    }

    #[test]
    fn provider_unavailable_fails_closed() {
        let mut value = provider();
        value.unavailable.insert("C-RAIL".to_owned());
        let report = evaluate_bundle(&plan(), &requirements(), 120, &value).unwrap();
        assert_eq!(
            report.results[1].disposition,
            RequirementDisposition::CommitmentUnavailable
        );
    }
}
