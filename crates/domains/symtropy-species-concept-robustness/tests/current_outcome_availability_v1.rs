include!("current_robustness_matrix_v1.rs");

use symtropy_evolution_core::{AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId};
use symtropy_species_concept_robustness::{
    CrossModelOutcomeAvailabilityLedger, ModelOutcomeAvailabilityDisposition,
    OutcomeAvailabilityError, OutcomeAvailabilityPolicy, OutcomeAvailabilityPolicyId,
    OutcomeUnavailabilityInput, ValidatedCrossModelOutcomeAvailability,
    ValidatedOutcomeAvailabilityPolicy,
};

fn availability_auth(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn unavailable_input(
    identity: symtropy_species_concept::OpenSpeciesConceptIdentity,
    observation_byte: u8,
    qualification_byte: u8,
) -> OutcomeUnavailabilityInput {
    OutcomeUnavailabilityInput {
        conceptual_identity: identity,
        observation_authority: availability_auth(
            "current-outcome-unavailability-observation",
            observation_byte,
        ),
        qualification_authority: availability_auth(
            "current-outcome-unavailability-qualification",
            qualification_byte,
        ),
    }
}

fn with_report<R>(
    missing_general: bool,
    f: impl FnOnce(
        &ValidatedOutcomeAvailabilityPolicy<'_>,
        &ValidatedCrossModelCurrentSpeciesReport<'_>,
        &SpeciesModelDesignRecord,
        &SpeciesModelDesignRecord,
    ) -> R,
) -> R {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
        |authority, strict_model, strict_status, general_model, general_status| {
            let missing_reason = availability_auth("general-current-result-missing", 201);
            let report = if missing_general {
                CrossModelCurrentSpeciesReport::evaluate(
                    authority,
                    [
                        CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                            .unwrap(),
                        CurrentModelOutcomeRow::missing_current_capability(
                            general_model,
                            missing_reason.clone(),
                        )
                        .unwrap(),
                    ],
                )
                .unwrap()
            } else {
                CrossModelCurrentSpeciesReport::evaluate(
                    authority,
                    [
                        CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                            .unwrap(),
                        CurrentModelOutcomeRow::general_lineage(general_model, general_status)
                            .unwrap(),
                    ],
                )
                .unwrap()
            };

            let current_report = if missing_general {
                ValidatedCrossModelCurrentSpeciesReport::validate_current(
                    &report,
                    authority,
                    [
                        CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                            .unwrap(),
                        CurrentModelOutcomeRow::missing_current_capability(
                            general_model,
                            missing_reason,
                        )
                        .unwrap(),
                    ],
                )
                .unwrap()
            } else {
                ValidatedCrossModelCurrentSpeciesReport::validate_current(
                    &report,
                    authority,
                    [
                        CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                            .unwrap(),
                        CurrentModelOutcomeRow::general_lineage(general_model, general_status)
                            .unwrap(),
                    ],
                )
                .unwrap()
            };

            let protocol = availability_auth("current-outcome-unavailability-protocol", 202);
            let raw_policy = OutcomeAvailabilityPolicy::declare(
                OutcomeAvailabilityPolicyId::new("e2c-current-outcome-availability").unwrap(),
                authority,
                protocol.clone(),
            )
            .unwrap();
            let current_policy = ValidatedOutcomeAvailabilityPolicy::validate_current(
                &raw_policy,
                authority,
                protocol,
            )
            .unwrap();
            f(
                &current_policy,
                &current_report,
                strict_model,
                general_model,
            )
        },
    )
}

#[test]
fn observed_current_rows_need_no_unavailability_assertion_and_replay() {
    with_report(false, |policy, report, _, _| {
        let ledger = CrossModelOutcomeAvailabilityLedger::capture(policy, report, []).unwrap();
        assert!(ledger.records.iter().all(|record| {
            record.availability == ModelOutcomeAvailabilityDisposition::ObservedCurrentOutcome
                && record.unavailability_observation_authority.is_none()
                && record.unavailability_qualification_authority.is_none()
        }));
        let current = ValidatedCrossModelOutcomeAvailability::validate_current(
            &ledger,
            policy,
            report,
            [],
        )
        .unwrap();
        assert_eq!(current.report_digest(), report.report_digest());
        assert_eq!(current.ledger_digest(), ledger.canonical_digest().unwrap());
    });
}

#[test]
fn missing_row_without_qualified_unavailability_fails_closed() {
    with_report(true, |policy, report, _, _| {
        let result = CrossModelOutcomeAvailabilityLedger::capture(policy, report, []);
        assert!(matches!(
            result,
            Err(OutcomeAvailabilityError::MissingQualifiedUnavailability)
        ));
    });
}

#[test]
fn exact_qualified_unavailability_binds_missing_row_without_changing_report_status() {
    with_report(true, |policy, report, _, general_model| {
        let input = unavailable_input(general_model.conceptual_identity().clone(), 203, 204);
        let original_status = report.report().status;
        let ledger = CrossModelOutcomeAvailabilityLedger::capture(
            policy,
            report,
            [input.clone()],
        )
        .unwrap();
        assert_eq!(ledger.report.status, original_status);
        let unavailable = ledger
            .records
            .iter()
            .find(|record| record.conceptual_identity == *general_model.conceptual_identity())
            .unwrap();
        assert_eq!(
            unavailable.availability,
            ModelOutcomeAvailabilityDisposition::QualifiedUnavailable
        );
        assert!(unavailable.unavailability_observation_authority.is_some());
        assert!(unavailable.unavailability_qualification_authority.is_some());

        let current = ValidatedCrossModelOutcomeAvailability::validate_current(
            &ledger,
            policy,
            report,
            [input],
        )
        .unwrap();
        assert_eq!(current.ledger().report.status, original_status);
    });
}

#[test]
fn unavailability_cannot_replace_an_observed_current_result() {
    with_report(false, |policy, report, _, general_model| {
        let result = CrossModelOutcomeAvailabilityLedger::capture(
            policy,
            report,
            [unavailable_input(
                general_model.conceptual_identity().clone(),
                205,
                206,
            )],
        );
        assert!(matches!(
            result,
            Err(OutcomeAvailabilityError::UnavailabilityForObservedRow)
        ));
    });
}

#[test]
fn qualification_only_drift_changes_identity_and_cannot_regain_current_authority() {
    with_report(true, |policy, report, _, general_model| {
        let first = unavailable_input(general_model.conceptual_identity().clone(), 207, 208);
        let ledger = CrossModelOutcomeAvailabilityLedger::capture(
            policy,
            report,
            [first.clone()],
        )
        .unwrap();
        let first_digest = ledger.canonical_digest().unwrap();

        let changed = unavailable_input(general_model.conceptual_identity().clone(), 207, 209);
        let changed_ledger = CrossModelOutcomeAvailabilityLedger::capture(
            policy,
            report,
            [changed.clone()],
        )
        .unwrap();
        assert_ne!(first_digest, changed_ledger.canonical_digest().unwrap());

        let replay = ValidatedCrossModelOutcomeAvailability::validate_current(
            &ledger,
            policy,
            report,
            [changed],
        );
        assert!(matches!(
            replay,
            Err(OutcomeAvailabilityError::LedgerReplayMismatch)
        ));
    });
}

#[test]
fn wrong_subject_unavailability_input_cannot_cover_missing_row() {
    with_report(true, |policy, report, strict_model, _| {
        let result = CrossModelOutcomeAvailabilityLedger::capture(
            policy,
            report,
            [unavailable_input(
                strict_model.conceptual_identity().clone(),
                210,
                211,
            )],
        );
        assert!(matches!(
            result,
            Err(OutcomeAvailabilityError::UnavailabilityForObservedRow)
                | Err(OutcomeAvailabilityError::MissingQualifiedUnavailability)
        ));
    });
}

#[test]
fn forged_availability_disposition_fails_local_recomputation() {
    with_report(true, |policy, report, _, general_model| {
        let input = unavailable_input(general_model.conceptual_identity().clone(), 212, 213);
        let mut ledger =
            CrossModelOutcomeAvailabilityLedger::capture(policy, report, [input]).unwrap();
        let record = ledger
            .records
            .iter_mut()
            .find(|record| record.conceptual_identity == *general_model.conceptual_identity())
            .unwrap();
        record.availability = ModelOutcomeAvailabilityDisposition::ObservedCurrentOutcome;
        assert!(matches!(
            ledger.canonical_digest(),
            Err(OutcomeAvailabilityError::MissingQualifiedUnavailability)
                | Err(OutcomeAvailabilityError::AvailabilityEvidenceShapeMismatch)
        ));
    });
}

#[test]
fn policy_protocol_drift_stales_current_policy_authority() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
        |authority, _, _, _, _| {
            let original_protocol = availability_auth("current-outcome-unavailability-protocol", 220);
            let policy = OutcomeAvailabilityPolicy::declare(
                OutcomeAvailabilityPolicyId::new("e2c-policy-drift").unwrap(),
                authority,
                original_protocol,
            )
            .unwrap();
            let replay = ValidatedOutcomeAvailabilityPolicy::validate_current(
                &policy,
                authority,
                availability_auth("current-outcome-unavailability-protocol", 221),
            );
            assert!(matches!(
                replay,
                Err(OutcomeAvailabilityError::PolicyReplayMismatch)
            ));
        },
    );
}
