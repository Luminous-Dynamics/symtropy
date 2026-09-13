include!("speciation_transition_v1.rs");

fn with_transition_context<R>(
    missing_policy: SpeciationTransitionMissingPolicy,
    f: impl FnOnce(
        &ValidatedSpeciationTransitionDesign<'_>,
        &ValidatedLineageDivergenceHistory<'_>,
        &ValidatedReproductiveIsolationEvidence<'_>,
        &ValidatedCurrentSpeciesStatus<'_>,
        &ValidatedBiologicalSpeciesModel<'_>,
    ) -> R,
) -> R {
    wide_history_fixture::with_current(WideHistoryCase::Clean, |history_design, history| {
        isolation_fixture::with_current(IsolationCase::Supported, |isolation_design, isolation| {
            let model = model(90, 91);
            let current_model = ValidatedBiologicalSpeciesModel::validate_current(
                &model,
                model.validity_domain.clone(),
                authority("species-model-qualification", 91),
            )
            .unwrap();
            let current_species_design = CurrentSpeciesClassificationDesign::declare(
                CurrentSpeciesClassificationId::new("current-status-hardening").unwrap(),
                history_design,
                isolation_design,
                &current_model,
                authority("target-model-applicability", 92),
            )
            .unwrap();
            let current_current_species_design =
                ValidatedCurrentSpeciesClassificationDesign::validate_current(
                    &current_species_design,
                    history_design,
                    isolation_design,
                    &current_model,
                    authority("target-model-applicability", 92),
                )
                .unwrap();
            let current_species_evidence = CurrentSpeciesStatusEvidence::evaluate(
                &current_current_species_design,
                history,
                isolation,
                &current_model,
                inside(170),
            )
            .unwrap();
            let current_species = ValidatedCurrentSpeciesStatus::validate_current(
                &current_species_evidence,
                &current_current_species_design,
                history,
                isolation,
                &current_model,
                inside(170),
            )
            .unwrap();
            let transition_design = SpeciationTransitionDesign::declare(
                SpeciationTransitionDesignId::new("historical-transition-hardening").unwrap(),
                history_design,
                isolation_design,
                &current_current_species_design,
                &current_model,
                PopulationGeneration(2),
                PopulationGeneration(3),
                transition_protocols(),
                missing_policy,
            )
            .unwrap();
            let current_transition_design = ValidatedSpeciationTransitionDesign::validate_current(
                &transition_design,
                history_design,
                isolation_design,
                &current_current_species_design,
                &current_model,
                transition_protocols(),
                missing_policy,
            )
            .unwrap();
            f(
                &current_transition_design,
                history,
                isolation,
                &current_species,
                &current_model,
            )
        })
    })
}

#[test]
fn duplicate_temporal_criterion_is_rejected_before_classification() {
    with_transition_context(
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        |design, history, isolation, current, model| {
            let mut inputs = supported_temporal_inputs();
            inputs.push(temporal_input(
                SpeciationTemporalCriterion::DivergenceTiming,
                2,
                3,
                TemporalEvidenceDisposition::Supports,
                180,
            ));
            assert!(matches!(
                SpeciationTransitionEvidence::evaluate(
                    design, history, isolation, current, model, inputs,
                ),
                Err(SpeciationTransitionEvidenceError::DuplicateTemporalCriterion(
                    SpeciationTemporalCriterion::DivergenceTiming
                ))
            ));
        },
    );
}

#[test]
fn caller_cannot_cherry_pick_temporal_windows() {
    with_transition_context(
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        |design, history, isolation, current, model| {
            let mut inputs = supported_temporal_inputs();
            let divergence = inputs
                .iter_mut()
                .find(|input| input.criterion == SpeciationTemporalCriterion::DivergenceTiming)
                .unwrap();
            divergence.start_generation = PopulationGeneration(1);
            divergence.end_generation = PopulationGeneration(3);
            assert!(matches!(
                SpeciationTransitionEvidence::evaluate(
                    design, history, isolation, current, model, inputs,
                ),
                Err(SpeciationTransitionEvidenceError::TemporalWindowMismatch(
                    SpeciationTemporalCriterion::DivergenceTiming
                ))
            ));
        },
    );
}

#[test]
fn outside_domain_disposition_is_reserved_for_historical_model_applicability() {
    with_transition_context(
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        |design, history, isolation, current, model| {
            let mut inputs = supported_temporal_inputs();
            replace_disposition(
                &mut inputs,
                SpeciationTemporalCriterion::DivergenceTiming,
                TemporalEvidenceDisposition::OutsideValidityDomain,
            );
            assert!(matches!(
                SpeciationTransitionEvidence::evaluate(
                    design, history, isolation, current, model, inputs,
                ),
                Err(
                    SpeciationTransitionEvidenceError::OutsideValidityDomainDispositionForbidden(
                        SpeciationTemporalCriterion::DivergenceTiming
                    )
                )
            ));
        },
    );
}

#[test]
fn fail_closed_policy_rejects_unavailable_temporal_evidence() {
    with_transition_context(
        SpeciationTransitionMissingPolicy::FailClosed,
        |design, history, isolation, current, model| {
            let mut inputs = supported_temporal_inputs();
            replace_disposition(
                &mut inputs,
                SpeciationTemporalCriterion::ReproductiveBarrierTiming,
                TemporalEvidenceDisposition::Unavailable,
            );
            assert!(matches!(
                SpeciationTransitionEvidence::evaluate(
                    design, history, isolation, current, model, inputs,
                ),
                Err(
                    SpeciationTransitionEvidenceError::UnavailableTemporalEvidenceForbidden(
                        SpeciationTemporalCriterion::ReproductiveBarrierTiming
                    )
                )
            ));
        },
    );
}

#[test]
fn qualification_identity_is_evidence_identity_not_a_comment() {
    with_transition_context(
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        |design, history, isolation, current, model| {
            let first = SpeciationTransitionEvidence::evaluate(
                design,
                history,
                isolation,
                current,
                model,
                supported_temporal_inputs(),
            )
            .unwrap();
            let mut changed_inputs = supported_temporal_inputs();
            changed_inputs
                .iter_mut()
                .find(|input| input.criterion == SpeciationTemporalCriterion::DivergenceTiming)
                .unwrap()
                .qualification = authority("changed-transition-qualification", 201);
            let second = SpeciationTransitionEvidence::evaluate(
                design,
                history,
                isolation,
                current,
                model,
                changed_inputs,
            )
            .unwrap();
            assert_eq!(first.status, second.status);
            assert_ne!(
                first.canonical_digest().unwrap(),
                second.canonical_digest().unwrap()
            );
        },
    );
}

#[test]
fn serialized_counter_history_cannot_be_deleted_to_make_history_look_cleaner() {
    with_transition(
        WideHistoryCase::RecontactAfter,
        IsolationCase::Supported,
        inside(171),
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        supported_temporal_inputs(),
        |_, _, evidence, _, _| {
            assert!(!evidence.later_counter_history.is_empty());
            let mut value = serde_json::to_value(evidence).unwrap();
            value["later_counter_history"] = serde_json::json!([]);
            let changed: SpeciationTransitionEvidence = serde_json::from_value(value).unwrap();
            assert!(matches!(
                changed.canonical_digest(),
                Err(SpeciationTransitionEvidenceError::CounterHistoryInvariant)
            ));
        },
    );
}
