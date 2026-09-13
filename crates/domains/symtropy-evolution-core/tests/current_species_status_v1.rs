mod history_fixture {
    include!("lineage_divergence_history_v1.rs");

    #[derive(Clone, Copy)]
    pub(super) enum Case {
        Clean,
        Recontact,
        Fusion,
        NotPersistent,
        Unavailable,
    }

    pub(super) fn with_current<R>(
        case: Case,
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ) -> R,
    ) -> R {
        let fixture = fixture();
        let design = design_with(
            LineageHistoryMissingPolicy::ReportInsufficientEvidence,
            LineageHistoryContextPolicy::ExactAcrossInterval,
        );
        let current = current_design(&design);
        let inputs = || match case {
            Case::Clean => clean_inputs(&fixture),
            Case::Recontact => vec![
                observed_input(&fixture, 1, GenerationKind::Clean),
                observed_input(&fixture, 2, GenerationKind::Recontact),
                observed_input(&fixture, 3, GenerationKind::Clean),
            ],
            Case::Fusion => vec![
                observed_input(&fixture, 1, GenerationKind::Clean),
                observed_input(&fixture, 2, GenerationKind::Clean),
                observed_input(&fixture, 3, GenerationKind::Fusion),
            ],
            Case::NotPersistent => vec![
                observed_input(&fixture, 1, GenerationKind::Clean),
                observed_input(&fixture, 2, GenerationKind::NotPersistent),
                observed_input(&fixture, 3, GenerationKind::Clean),
            ],
            Case::Unavailable => vec![
                observed_input(&fixture, 1, GenerationKind::Clean),
                LineageHistoryGenerationInput::Unavailable {
                    generation: PopulationGeneration(2),
                    reason: authority("generation-two-unavailable", 70),
                },
                observed_input(&fixture, 3, GenerationKind::Clean),
            ],
        };
        let history = LineageDivergenceHistory::capture(&current, inputs()).unwrap();
        let validated =
            ValidatedLineageDivergenceHistory::validate_current(&history, &current, inputs())
                .unwrap();
        f(&current, &validated)
    }
}

mod isolation_fixture {
    include!("reproductive_isolation_evidence_v1.rs");

    #[derive(Clone, Copy)]
    pub(super) enum Case {
        Supported,
        Contradicted,
        NotSupported,
        Insufficient,
    }

    pub(super) fn with_current<R>(
        case: Case,
        f: impl FnOnce(
            &ValidatedReproductiveIsolationDesign<'_>,
            &ValidatedReproductiveIsolationEvidence<'_>,
        ) -> R,
    ) -> R {
        // Seed 20 deliberately matches SEL-10A fixture lineage authorities:
        // lineage-a=[20;32], lineage-b=[21;32].
        let ctx = context(150);
        let auth = authorities(20);
        let a = iso_contact(&auth, ctx, "status-a", 10);
        let b = iso_contact(&auth, ctx, "status-b", 20);
        let a_design = iso_current_design(&a, &auth, ctx);
        let b_design = iso_current_design(&b, &auth, ctx);
        let design = iso_design(&a_design, &b_design, 30);
        let current_design =
            iso_current_isolation_design(&design, &a_design, &b_design, 30);

        let (a_kind, b_kind, a_byte, b_byte) = match case {
            Case::Supported => (ContactKind::Barrier, ContactKind::Barrier, 40, 60),
            Case::Contradicted => (ContactKind::Barrier, ContactKind::Fertile, 40, 80),
            Case::NotSupported => (ContactKind::Barrier, ContactKind::NoContact, 40, 80),
            Case::Insufficient => (ContactKind::NoContact, ContactKind::NoContact, 40, 80),
        };
        let a_study = iso_study(&a, &auth, ctx, a_kind, a_byte);
        let b_study = iso_study(&b, &auth, ctx, b_kind, b_byte);
        let current_a = iso_current_study(&a, &a_study, &auth, ctx, a_kind, a_byte);
        let current_b = iso_current_study(&b, &b_study, &auth, ctx, b_kind, b_byte);
        let evidence = two_study_evidence(&current_design, &current_a, &current_b);
        let validated = ValidatedReproductiveIsolationEvidence::validate_current(
            &evidence,
            &current_design,
            vec![
                IsolationStudyEvidenceInput {
                    unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                    study: &current_a,
                },
                IsolationStudyEvidenceInput {
                    unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                    study: &current_b,
                },
            ],
        )
        .unwrap();
        f(&current_design, &validated)
    }
}

use history_fixture::Case as HistoryCase;
use isolation_fixture::Case as IsolationCase;
use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, BiologicalSpeciesModel,
    CurrentSpeciesClassificationDesign, CurrentSpeciesClassificationId, CurrentSpeciesStatus,
    CurrentSpeciesStatusEvidence, CurrentSpeciesStatusError, SpeciesModelApplicabilityInput,
    SpeciesModelId, SpeciesModelValidityDomainId, SpeciesModelValidityDomainRef,
    ValidatedBiologicalSpeciesModel, ValidatedCurrentSpeciesClassificationDesign,
    ValidatedCurrentSpeciesStatus,
};

fn authority(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn model(domain_byte: u8, qualification_byte: u8) -> BiologicalSpeciesModel {
    BiologicalSpeciesModel::qualify(
        SpeciesModelId::new("strict-biological-species-v1").unwrap(),
        SpeciesModelValidityDomainRef::new(
            SpeciesModelValidityDomainId::new("diploid-sexual-validity-domain").unwrap(),
            authority("species-validity-domain-authority", domain_byte),
        ),
        authority("species-model-qualification", qualification_byte),
    )
    .unwrap()
}

fn with_status<R>(
    history_case: HistoryCase,
    isolation_case: IsolationCase,
    applicability: SpeciesModelApplicabilityInput,
    f: impl FnOnce(&CurrentSpeciesStatusEvidence, &ValidatedCurrentSpeciesStatus<'_>) -> R,
) -> R {
    history_fixture::with_current(history_case, |history_design, history| {
        isolation_fixture::with_current(isolation_case, |isolation_design, isolation| {
            let model = model(90, 91);
            let current_model = ValidatedBiologicalSpeciesModel::validate_current(
                &model,
                model.validity_domain.clone(),
                authority("species-model-qualification", 91),
            )
            .unwrap();
            let design = CurrentSpeciesClassificationDesign::declare(
                CurrentSpeciesClassificationId::new("current-status-a-b").unwrap(),
                history_design,
                isolation_design,
                &current_model,
                authority("target-model-applicability", 92),
            )
            .unwrap();
            let current_design = ValidatedCurrentSpeciesClassificationDesign::validate_current(
                &design,
                history_design,
                isolation_design,
                &current_model,
                authority("target-model-applicability", 92),
            )
            .unwrap();
            let evidence = CurrentSpeciesStatusEvidence::evaluate(
                &current_design,
                history,
                isolation,
                &current_model,
                applicability.clone(),
            )
            .unwrap();
            let validated = ValidatedCurrentSpeciesStatus::validate_current(
                &evidence,
                &current_design,
                history,
                isolation,
                &current_model,
                applicability,
            )
            .unwrap();
            f(&evidence, &validated)
        })
    })
}

fn inside(byte: u8) -> SpeciesModelApplicabilityInput {
    SpeciesModelApplicabilityInput::InsideValidityDomain {
        evidence: authority("inside-model-domain", byte),
    }
}

#[test]
fn strict_model_supports_complete_isolation_with_persistent_or_recontact_history() {
    for history in [HistoryCase::Clean, HistoryCase::Recontact] {
        with_status(history, IsolationCase::Supported, inside(100), |evidence, validated| {
            assert_eq!(evidence.status, CurrentSpeciesStatus::SupportedUnderModel);
            assert_eq!(validated.evidence_digest(), evidence.canonical_digest().unwrap());
        });
    }
}

#[test]
fn all_five_current_statuses_have_distinct_evidence_paths() {
    with_status(
        HistoryCase::Clean,
        IsolationCase::NotSupported,
        inside(101),
        |evidence, _| assert_eq!(evidence.status, CurrentSpeciesStatus::NotSupportedUnderModel),
    );
    with_status(
        HistoryCase::Clean,
        IsolationCase::Contradicted,
        inside(102),
        |evidence, _| assert_eq!(evidence.status, CurrentSpeciesStatus::ContradictedUnderModel),
    );
    with_status(
        HistoryCase::Unavailable,
        IsolationCase::Supported,
        inside(103),
        |evidence, _| assert_eq!(evidence.status, CurrentSpeciesStatus::InsufficientEvidence),
    );
    with_status(
        HistoryCase::Clean,
        IsolationCase::Supported,
        SpeciesModelApplicabilityInput::OutsideValidityDomain {
            evidence: authority("outside-model-domain", 104),
        },
        |evidence, _| {
            assert_eq!(evidence.status, CurrentSpeciesStatus::OutsideModelValidityDomain)
        },
    );
    with_status(
        HistoryCase::Clean,
        IsolationCase::Supported,
        SpeciesModelApplicabilityInput::Unavailable {
            reason: authority("applicability-unavailable", 105),
        },
        |evidence, _| assert_eq!(evidence.status, CurrentSpeciesStatus::InsufficientEvidence),
    );
}

#[test]
fn fusion_or_persistence_loss_contradicts_even_with_supported_isolation() {
    for history in [HistoryCase::Fusion, HistoryCase::NotPersistent] {
        with_status(history, IsolationCase::Supported, inside(106), |evidence, _| {
            assert_eq!(evidence.status, CurrentSpeciesStatus::ContradictedUnderModel)
        });
    }
}

#[test]
fn applicability_evidence_is_identity_bearing_and_required_for_current_replay() {
    let mut first = None;
    with_status(HistoryCase::Clean, IsolationCase::Supported, inside(107), |evidence, _| {
        first = Some(evidence.canonical_digest().unwrap());
    });
    with_status(HistoryCase::Clean, IsolationCase::Supported, inside(108), |evidence, _| {
        assert_ne!(Some(evidence.canonical_digest().unwrap()), first);
    });
}

#[test]
fn serialized_status_tampering_fails_local_canonicalization() {
    with_status(HistoryCase::Clean, IsolationCase::Supported, inside(109), |evidence, _| {
        let mut value = serde_json::to_value(evidence).unwrap();
        value["status"] = serde_json::json!("NotSupportedUnderModel");
        let changed: CurrentSpeciesStatusEvidence = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(CurrentSpeciesStatusError::StatusInvariant)
        ));
    });
}

fn collect_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn current_species_status_wire_has_no_historical_speciation_event_claim() {
    with_status(HistoryCase::Clean, IsolationCase::Supported, inside(110), |evidence, _| {
        let mut keys = Vec::new();
        collect_keys(&serde_json::to_value(evidence).unwrap(), &mut keys);
        for forbidden in [
            "speciation_event",
            "speciation_transition",
            "transition_generation",
            "transition_time",
            "transition_interval",
            "divergence_time",
        ] {
            assert!(!keys.iter().any(|key| key == forbidden));
        }
    });
}
