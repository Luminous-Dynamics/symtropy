use civbench_event_evaluator::decision_closure::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt_binding(
        owner: ReceiptOwner,
        receipt_id: &str,
        decision_episode_id: &str,
        paired_block_id: &str,
        seed: u64,
    ) -> ReceiptBinding {
        ReceiptBinding {
            owner,
            receipt_id: receipt_id.into(),
            decision_episode_id: decision_episode_id.into(),
            paired_block_id: paired_block_id.into(),
            payload_digest: format!("{seed:064x}"),
        }
    }

    fn positive_bindings() -> Vec<ReceiptBinding> {
        vec![
            receipt_binding(
                ReceiptOwner::EventEvaluator,
                "event-receipt-1",
                "decision-1",
                "block-1",
                1,
            ),
            receipt_binding(
                ReceiptOwner::AcquisitionPlanner,
                "acquisition-1",
                "decision-1",
                "block-1",
                2,
            ),
            receipt_binding(
                ReceiptOwner::HumanAuthority,
                "authority-1",
                "decision-1",
                "block-1",
                3,
            ),
            receipt_binding(
                ReceiptOwner::ResourceLedger,
                "resource-1",
                "decision-1",
                "block-1",
                4,
            ),
            receipt_binding(
                ReceiptOwner::RepairWorkflow,
                "workflow-1",
                "decision-1",
                "block-1",
                5,
            ),
            receipt_binding(
                ReceiptOwner::IndependentVerifier,
                "verification-1",
                "decision-1",
                "block-1",
                6,
            ),
            receipt_binding(
                ReceiptOwner::ServiceOutcome,
                "service-outcome-1",
                "decision-1",
                "block-1",
                7,
            ),
            receipt_binding(
                ReceiptOwner::PairedOutcome,
                "paired-outcome-1",
                "decision-1",
                "block-1",
                8,
            ),
        ]
    }

    fn positive_record() -> DecisionClosureRecord {
        DecisionClosureRecord {
            decision_episode_id: "decision-1".into(),
            paired_block_id: "block-1".into(),
            event_receipt_id: "event-receipt-1".into(),
            acquisition_receipt_id: Some("acquisition-1".into()),
            decision_changed_after_acquisition: Some(true),
            human_disposition: HumanDisposition::Authorized,
            human_authority_receipt_id: Some("authority-1".into()),
            human_actor_ref: Some("principal-operator-1".into()),
            resource_feasibility: ResourceFeasibility::FeasibleAndReserved,
            resource_settlement: ResourceSettlement::Settled,
            executor_identity_ref: Some("principal-crew-1".into()),
            verifier_identity_ref: Some("principal-inspector-2".into()),
            receipt_bindings: positive_bindings(),
            action_progress: ActionProgress::IndependentlyVerified,
            resource_receipt_id: Some("resource-1".into()),
            workflow_receipt_id: Some("workflow-1".into()),
            verification_receipt_id: Some("verification-1".into()),
            service_outcome_receipt_id: Some("service-outcome-1".into()),
            paired_outcome_receipt_id: Some("paired-outcome-1".into()),
            service_evidence_class: Some(ServiceEvidenceClass::Modelled),
            service_effect: ServiceEffect::ModelledImprovement,
        }
    }

    #[test]
    fn independently_verified_modelled_improvement_is_valid() {
        let record = positive_record();
        assert_eq!(record.validate(), Ok(()));
        assert!(
            record
                .canonical_text()
                .contains("service_effect=modelled_improvement")
        );
    }

    #[test]
    fn acquisition_cannot_change_decision_without_an_acquisition_receipt() {
        let mut record = positive_record();
        record.acquisition_receipt_id = None;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::DecisionChangeWithoutAcquisition)
        );
    }

    #[test]
    fn rejected_recommendation_cannot_become_a_work_order() {
        let mut record = positive_record();
        record.human_disposition = HumanDisposition::Rejected;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ActionAfterNonAuthorization)
        );
    }

    #[test]
    fn deferred_recommendation_cannot_claim_a_repair() {
        let mut record = positive_record();
        record.human_disposition = HumanDisposition::Deferred;
        record.action_progress = ActionProgress::RepairClaimed;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ActionAfterNonAuthorization)
        );
    }

    #[test]
    fn rejected_or_deferred_recommendations_cannot_hold_resource_reservations() {
        let mut record = positive_record();
        record.human_disposition = HumanDisposition::Deferred;
        record.action_progress = ActionProgress::NotStarted;
        record.verification_receipt_id = None;
        record.verifier_identity_ref = None;
        record
            .receipt_bindings
            .retain(|binding| binding.owner != ReceiptOwner::IndependentVerifier);
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ResourceReservationAfterNonAuthorization)
        );

        // A reservation explicitly released after a deferred decision is safe:
        // it must not be mistaken for resources still held or settled.
        record.resource_settlement = ResourceSettlement::Released;
        assert_eq!(record.validate(), Ok(()));
    }

    #[test]
    fn authorized_but_infeasible_action_cannot_be_planned() {
        let mut record = positive_record();
        record.resource_feasibility = ResourceFeasibility::RejectedMissingPart;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ActionAfterFeasibilityRejection)
        );
    }

    #[test]
    fn verified_work_requires_a_distinct_verification_receipt() {
        let mut record = positive_record();
        record.verification_receipt_id = None;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::VerifiedWithoutVerification)
        );
    }

    #[test]
    fn verification_receipt_cannot_upgrade_a_repair_claim() {
        let mut record = positive_record();
        record.action_progress = ActionProgress::RepairClaimed;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::VerificationWithoutVerifiedStage)
        );
    }

    #[test]
    fn improvement_requires_authorization_feasibility_and_verification() {
        let mut record = positive_record();
        record.action_progress = ActionProgress::RepairClaimed;
        record.verification_receipt_id = None;
        record.verifier_identity_ref = None;
        record.resource_settlement = ResourceSettlement::Settled;
        record
            .receipt_bindings
            .retain(|binding| binding.owner != ReceiptOwner::IndependentVerifier);
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ImprovementWithoutVerifiedAction)
        );
    }

    #[test]
    fn a_service_claim_requires_both_single_and_paired_outcome_receipts() {
        let mut record = positive_record();
        record.paired_outcome_receipt_id = None;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ServiceEffectWithoutPairedOutcome)
        );
    }

    #[test]
    fn source_evidence_class_must_match_the_effect_claim() {
        let mut record = positive_record();
        record.service_evidence_class = Some(ServiceEvidenceClass::Observed);
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ServiceEffectEvidenceMismatch)
        );
    }

    #[test]
    fn no_value_sensor_can_resolve_without_becoming_a_service_gain() {
        let record = DecisionClosureRecord {
            decision_episode_id: "decision-no-value".into(),
            paired_block_id: "block-no-value".into(),
            event_receipt_id: "event-receipt-no-value".into(),
            acquisition_receipt_id: Some("acquisition-no-value".into()),
            decision_changed_after_acquisition: Some(false),
            human_disposition: HumanDisposition::Deferred,
            human_authority_receipt_id: Some("authority-no-value".into()),
            human_actor_ref: Some("principal-operator-no-value".into()),
            resource_feasibility: ResourceFeasibility::NotAttempted,
            resource_settlement: ResourceSettlement::NotAttempted,
            executor_identity_ref: None,
            verifier_identity_ref: None,
            receipt_bindings: vec![
                receipt_binding(
                    ReceiptOwner::EventEvaluator,
                    "event-receipt-no-value",
                    "decision-no-value",
                    "block-no-value",
                    11,
                ),
                receipt_binding(
                    ReceiptOwner::AcquisitionPlanner,
                    "acquisition-no-value",
                    "decision-no-value",
                    "block-no-value",
                    12,
                ),
                receipt_binding(
                    ReceiptOwner::HumanAuthority,
                    "authority-no-value",
                    "decision-no-value",
                    "block-no-value",
                    13,
                ),
                receipt_binding(
                    ReceiptOwner::ServiceOutcome,
                    "service-no-value",
                    "decision-no-value",
                    "block-no-value",
                    14,
                ),
                receipt_binding(
                    ReceiptOwner::PairedOutcome,
                    "paired-no-value",
                    "decision-no-value",
                    "block-no-value",
                    15,
                ),
            ],
            action_progress: ActionProgress::NotStarted,
            resource_receipt_id: None,
            workflow_receipt_id: None,
            verification_receipt_id: None,
            service_outcome_receipt_id: Some("service-no-value".into()),
            paired_outcome_receipt_id: Some("paired-no-value".into()),
            service_evidence_class: Some(ServiceEvidenceClass::Modelled),
            service_effect: ServiceEffect::ModelledNoImprovement,
        };
        assert_eq!(record.validate(), Ok(()));
    }

    #[test]
    fn resource_exhaustion_is_auditable_but_not_executable() {
        let record = DecisionClosureRecord {
            decision_episode_id: "decision-no-part".into(),
            paired_block_id: "block-no-part".into(),
            event_receipt_id: "event-receipt-no-part".into(),
            acquisition_receipt_id: None,
            decision_changed_after_acquisition: None,
            human_disposition: HumanDisposition::Authorized,
            human_authority_receipt_id: Some("authority-no-part".into()),
            human_actor_ref: Some("principal-operator-no-part".into()),
            resource_feasibility: ResourceFeasibility::RejectedMissingPart,
            resource_settlement: ResourceSettlement::Rejected,
            executor_identity_ref: None,
            verifier_identity_ref: None,
            receipt_bindings: vec![
                receipt_binding(
                    ReceiptOwner::EventEvaluator,
                    "event-receipt-no-part",
                    "decision-no-part",
                    "block-no-part",
                    21,
                ),
                receipt_binding(
                    ReceiptOwner::HumanAuthority,
                    "authority-no-part",
                    "decision-no-part",
                    "block-no-part",
                    22,
                ),
                receipt_binding(
                    ReceiptOwner::ResourceLedger,
                    "resource-rejection-no-part",
                    "decision-no-part",
                    "block-no-part",
                    23,
                ),
            ],
            action_progress: ActionProgress::NotStarted,
            resource_receipt_id: Some("resource-rejection-no-part".into()),
            workflow_receipt_id: None,
            verification_receipt_id: None,
            service_outcome_receipt_id: None,
            paired_outcome_receipt_id: None,
            service_evidence_class: None,
            service_effect: ServiceEffect::Indeterminate,
        };
        assert_eq!(record.validate(), Ok(()));
    }

    #[test]
    fn unverified_repair_remains_indeterminate() {
        let mut record = positive_record();
        record.action_progress = ActionProgress::RepairClaimed;
        record.verification_receipt_id = None;
        record.service_outcome_receipt_id = None;
        record.paired_outcome_receipt_id = None;
        record.service_evidence_class = None;
        record.verifier_identity_ref = None;
        record.receipt_bindings.retain(|binding| {
            !matches!(
                binding.owner,
                ReceiptOwner::IndependentVerifier
                    | ReceiptOwner::ServiceOutcome
                    | ReceiptOwner::PairedOutcome
            )
        });
        record.service_effect = ServiceEffect::Indeterminate;
        assert_eq!(record.validate(), Ok(()));
    }

    #[test]
    fn unclaimed_service_effect_cannot_carry_effect_receipts() {
        let mut record = positive_record();
        record.service_effect = ServiceEffect::NotClaimed;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ReferencesWithoutServiceClaim)
        );
    }

    #[test]
    fn whitespace_and_empty_references_fail_closed() {
        let mut record = positive_record();
        record.paired_block_id = "block with spaces".into();
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::InvalidReference("paired_block_id"))
        );
    }

    #[test]
    fn verifier_must_be_a_different_principal_from_the_executor() {
        let mut record = positive_record();
        record.verifier_identity_ref = record.executor_identity_ref.clone();
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::VerifierNotIndependent)
        );
    }

    #[test]
    fn receipt_ids_cannot_be_reused_for_workflow_and_verification() {
        let mut record = positive_record();
        record.verification_receipt_id = record.workflow_receipt_id.clone();
        for binding in &mut record.receipt_bindings {
            if binding.owner == ReceiptOwner::IndependentVerifier {
                binding.receipt_id = "workflow-1".into();
            }
        }
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::DuplicateReceiptId)
        );
    }

    #[test]
    fn every_owner_receipt_must_bind_the_same_decision_and_paired_block() {
        let mut record = positive_record();
        let paired = record
            .receipt_bindings
            .iter_mut()
            .find(|binding| binding.owner == ReceiptOwner::PairedOutcome)
            .expect("paired outcome binding");
        paired.paired_block_id = "other-block".into();
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ReceiptBindingMismatch)
        );
    }

    #[test]
    fn receipt_bindings_require_a_nonzero_sha256_digest() {
        let mut record = positive_record();
        let event = record
            .receipt_bindings
            .iter_mut()
            .find(|binding| binding.owner == ReceiptOwner::EventEvaluator)
            .expect("event binding");
        event.payload_digest = "0".repeat(64);
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::InvalidReceiptDigest)
        );
    }

    #[test]
    fn authorized_disposition_requires_an_owner_receipt() {
        let mut record = positive_record();
        record.human_authority_receipt_id = None;
        record
            .receipt_bindings
            .retain(|binding| binding.owner != ReceiptOwner::HumanAuthority);
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::MissingAuthorityReceipt)
        );
    }

    #[test]
    fn cancelled_work_must_release_its_resource_reservation() {
        let mut record = positive_record();
        record.action_progress = ActionProgress::Cancelled;
        record.verification_receipt_id = None;
        record.verifier_identity_ref = None;
        record
            .receipt_bindings
            .retain(|binding| binding.owner != ReceiptOwner::IndependentVerifier);
        record.resource_settlement = ResourceSettlement::Settled;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ResourceSettlementMismatch)
        );
    }

    #[test]
    fn in_progress_work_cannot_claim_a_settled_reservation() {
        let mut record = positive_record();
        record.action_progress = ActionProgress::InProgress;
        record.verification_receipt_id = None;
        record.verifier_identity_ref = None;
        record
            .receipt_bindings
            .retain(|binding| binding.owner != ReceiptOwner::IndependentVerifier);
        record.resource_settlement = ResourceSettlement::Settled;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ResourceSettlementMismatch)
        );
    }

    #[test]
    fn receipt_binding_input_order_does_not_change_canonical_output() {
        let first = positive_record();
        let mut second = positive_record();
        second.receipt_bindings.reverse();
        assert_eq!(first.canonical_text(), second.canonical_text());
    }

    #[test]
    fn canonical_output_is_deterministic_for_identical_references() {
        let first = positive_record();
        let second = positive_record();
        assert_eq!(first.canonical_text(), second.canonical_text());
    }
}
