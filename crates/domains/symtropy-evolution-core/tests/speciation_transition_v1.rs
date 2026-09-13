include!("current_species_status_v1.rs");

mod wide_history_fixture {
    include!("lineage_divergence_history_v1.rs");

    #[derive(Clone, Copy)]
    pub(super) enum Case {
        Clean,
        RecontactAfter,
        FusionAfter,
    }

    fn wide_fixture() -> HistoryFixture {
        let schema = schema();
        let experiment = EvolutionExperimentId::new("lineage-history-wide-fixture").unwrap();
        let mut a_populations = Vec::new();
        let mut b_populations = Vec::new();
        let mut a_points = Vec::new();
        let mut b_points = Vec::new();

        for generation in 1..=5 {
            let a = population(&schema, "lineage-a-pop", 3, 1);
            let b = population(&schema, "lineage-b-pop", 1, 3);
            a_points.push(
                PopulationTrajectoryPoint::declare_reference_start(
                    &schema,
                    &a,
                    experiment.clone(),
                    PopulationGeneration(generation),
                )
                .unwrap(),
            );
            b_points.push(
                PopulationTrajectoryPoint::declare_reference_start(
                    &schema,
                    &b,
                    experiment.clone(),
                    PopulationGeneration(generation),
                )
                .unwrap(),
            );
            a_populations.push(a);
            b_populations.push(b);
        }

        HistoryFixture {
            schema,
            a_populations,
            b_populations,
            a_points,
            b_points,
        }
    }

    fn wide_design() -> LineageDivergenceHistoryDesign {
        LineageDivergenceHistoryDesign::declare(
            LineageDivergenceHistoryId::new("history-a-b-wide").unwrap(),
            authority("lineage-a", 20),
            authority("lineage-b", 21),
            PopulationGeneration(1),
            PopulationGeneration(5),
            protocols(),
            LineageHistoryContextPolicy::ExactAcrossInterval,
            LineageHistoryMissingPolicy::ReportInsufficientEvidence,
            authority("history-completeness", 22),
        )
        .unwrap()
    }

    fn wide_current_design<'a>(
        design: &'a LineageDivergenceHistoryDesign,
    ) -> ValidatedLineageDivergenceHistoryDesign<'a> {
        ValidatedLineageDivergenceHistoryDesign::validate_current(
            design,
            authority("lineage-a", 20),
            authority("lineage-b", 21),
            PopulationGeneration(1),
            PopulationGeneration(5),
            protocols(),
            LineageHistoryContextPolicy::ExactAcrossInterval,
            LineageHistoryMissingPolicy::ReportInsufficientEvidence,
            authority("history-completeness", 22),
        )
        .unwrap()
    }

    fn wide_inputs(
        fixture: &HistoryFixture,
        case: Case,
    ) -> Vec<LineageHistoryGenerationInput<'_>> {
        (1..=5)
            .map(|generation| {
                let kind = match (case, generation) {
                    (Case::RecontactAfter, 5) => GenerationKind::Recontact,
                    (Case::FusionAfter, 5) => GenerationKind::Fusion,
                    _ => GenerationKind::Clean,
                };
                observed_input(fixture, generation, kind)
            })
            .collect()
    }

    pub(super) fn with_current<R>(
        case: Case,
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ) -> R,
    ) -> R {
        let fixture = wide_fixture();
        let design = wide_design();
        let current = wide_current_design(&design);
        let history = LineageDivergenceHistory::capture(&current, wide_inputs(&fixture, case)).unwrap();
        let validated = ValidatedLineageDivergenceHistory::validate_current(
            &history,
            &current,
            wide_inputs(&fixture, case),
        )
        .unwrap();
        f(&current, &validated)
    }
}

use symtropy_evolution_core::{
    HistoricalCounterHistoryKind, PopulationGeneration, SpeciationTemporalCriterion,
    SpeciationTemporalEvidenceInput, SpeciationTransitionDesign, SpeciationTransitionDesignError,
    SpeciationTransitionDesignId, SpeciationTransitionEvidence, SpeciationTransitionEvidenceError,
    SpeciationTransitionEvidenceProtocols, SpeciationTransitionMissingPolicy,
    SpeciationTransitionStatus, TemporalEvidenceDisposition,
    ValidatedSpeciationTransitionDesign, ValidatedSpeciationTransitionEvidence,
};
use wide_history_fixture::Case as WideHistoryCase;

fn transition_protocols() -> SpeciationTransitionEvidenceProtocols {
    SpeciationTransitionEvidenceProtocols {
        pre_transition_common_source: authority("transition-common-source-protocol", 120),
        divergence_timing: authority("transition-divergence-timing-protocol", 121),
        reproductive_barrier_timing: authority("transition-barrier-timing-protocol", 122),
        demographic_history: authority("transition-demographic-history-protocol", 123),
        interval_completeness: authority("transition-interval-completeness-protocol", 124),
        model_applicability: authority("transition-model-applicability-protocol", 125),
        later_counter_history: authority("transition-counter-history-protocol", 126),
        qualification: authority("transition-evidence-qualification-protocol", 127),
    }
}

fn temporal_input(
    criterion: SpeciationTemporalCriterion,
    start: u64,
    end: u64,
    disposition: TemporalEvidenceDisposition,
    byte: u8,
) -> SpeciationTemporalEvidenceInput {
    SpeciationTemporalEvidenceInput {
        criterion,
        start_generation: PopulationGeneration(start),
        end_generation: PopulationGeneration(end),
        disposition,
        evidence: authority(&format!("transition-{criterion:?}-evidence"), byte),
        qualification: authority(
            &format!("transition-{criterion:?}-qualification"),
            byte.wrapping_add(20),
        ),
    }
}

fn supported_temporal_inputs() -> Vec<SpeciationTemporalEvidenceInput> {
    vec![
        temporal_input(
            SpeciationTemporalCriterion::LaterCounterHistory,
            4,
            5,
            TemporalEvidenceDisposition::Supports,
            140,
        ),
        temporal_input(
            SpeciationTemporalCriterion::ModelApplicability,
            2,
            3,
            TemporalEvidenceDisposition::Supports,
            139,
        ),
        temporal_input(
            SpeciationTemporalCriterion::IntervalCompleteness,
            2,
            3,
            TemporalEvidenceDisposition::Supports,
            138,
        ),
        temporal_input(
            SpeciationTemporalCriterion::DemographicHistory,
            1,
            4,
            TemporalEvidenceDisposition::Supports,
            137,
        ),
        temporal_input(
            SpeciationTemporalCriterion::ReproductiveBarrierTiming,
            2,
            4,
            TemporalEvidenceDisposition::Supports,
            136,
        ),
        temporal_input(
            SpeciationTemporalCriterion::DivergenceTiming,
            2,
            3,
            TemporalEvidenceDisposition::Supports,
            135,
        ),
        temporal_input(
            SpeciationTemporalCriterion::PreTransitionCommonSource,
            1,
            1,
            TemporalEvidenceDisposition::Supports,
            134,
        ),
    ]
}

fn replace_disposition(
    inputs: &mut [SpeciationTemporalEvidenceInput],
    criterion: SpeciationTemporalCriterion,
    disposition: TemporalEvidenceDisposition,
) {
    inputs
        .iter_mut()
        .find(|input| input.criterion == criterion)
        .unwrap()
        .disposition = disposition;
}

#[allow(clippy::too_many_arguments)]
fn with_transition<R>(
    history_case: WideHistoryCase,
    isolation_case: IsolationCase,
    current_applicability: SpeciesModelApplicabilityInput,
    missing_policy: SpeciationTransitionMissingPolicy,
    temporal_inputs: Vec<SpeciationTemporalEvidenceInput>,
    f: impl FnOnce(
        &SpeciationTransitionDesign,
        &ValidatedSpeciationTransitionDesign<'_>,
        &SpeciationTransitionEvidence,
        &ValidatedSpeciationTransitionEvidence<'_>,
        &CurrentSpeciesStatusEvidence,
    ) -> R,
) -> R {
    wide_history_fixture::with_current(history_case, |history_design, history| {
        isolation_fixture::with_current(isolation_case, |isolation_design, isolation| {
            let model = model(90, 91);
            let current_model = ValidatedBiologicalSpeciesModel::validate_current(
                &model,
                model.validity_domain.clone(),
                authority("species-model-qualification", 91),
            )
            .unwrap();
            let current_species_design = CurrentSpeciesClassificationDesign::declare(
                CurrentSpeciesClassificationId::new("current-status-a-b-wide").unwrap(),
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
                current_applicability.clone(),
            )
            .unwrap();
            let current_species = ValidatedCurrentSpeciesStatus::validate_current(
                &current_species_evidence,
                &current_current_species_design,
                history,
                isolation,
                &current_model,
                current_applicability,
            )
            .unwrap();

            let transition_design = SpeciationTransitionDesign::declare(
                SpeciationTransitionDesignId::new("historical-transition-a-b").unwrap(),
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
            let evidence = SpeciationTransitionEvidence::evaluate(
                &current_transition_design,
                history,
                isolation,
                &current_species,
                &current_model,
                temporal_inputs.clone(),
            )
            .unwrap();
            let validated = ValidatedSpeciationTransitionEvidence::validate_current(
                &evidence,
                &current_transition_design,
                history,
                isolation,
                &current_species,
                &current_model,
                temporal_inputs,
            )
            .unwrap();
            f(
                &transition_design,
                &current_transition_design,
                &evidence,
                &validated,
                &current_species_evidence,
            )
        })
    })
}

#[test]
fn complete_temporal_evidence_supports_interval_and_replays_without_exact_event_time() {
    with_transition(
        WideHistoryCase::Clean,
        IsolationCase::Supported,
        inside(150),
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        supported_temporal_inputs(),
        |design, _, evidence, validated, _| {
            assert_eq!(
                evidence.status,
                SpeciationTransitionStatus::TransitionSupportedUnderModel
            );
            assert_eq!(design.candidate_start_generation, PopulationGeneration(2));
            assert_eq!(design.candidate_end_generation, PopulationGeneration(3));
            assert_eq!(design.candidate_generation_count, 2);
            assert_eq!(validated.evidence_digest(), evidence.canonical_digest().unwrap());
        },
    );
}

#[test]
fn one_generation_exact_transition_claim_is_structurally_rejected() {
    wide_history_fixture::with_current(WideHistoryCase::Clean, |history_design, _| {
        isolation_fixture::with_current(IsolationCase::Supported, |isolation_design, _| {
            let model = model(90, 91);
            let current_model = ValidatedBiologicalSpeciesModel::validate_current(
                &model,
                model.validity_domain.clone(),
                authority("species-model-qualification", 91),
            )
            .unwrap();
            let current_species_design = CurrentSpeciesClassificationDesign::declare(
                CurrentSpeciesClassificationId::new("current-status-exact-reject").unwrap(),
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
            assert!(matches!(
                SpeciationTransitionDesign::declare(
                    SpeciationTransitionDesignId::new("exact-forbidden").unwrap(),
                    history_design,
                    isolation_design,
                    &current_current_species_design,
                    &current_model,
                    PopulationGeneration(2),
                    PopulationGeneration(2),
                    transition_protocols(),
                    SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
                ),
                Err(SpeciationTransitionDesignError::ExactTransitionGenerationForbidden)
            ));
        })
    });
}

#[test]
fn qualified_bracket_without_mechanism_level_barrier_timing_is_interval_only() {
    let mut inputs = supported_temporal_inputs();
    replace_disposition(
        &mut inputs,
        SpeciationTemporalCriterion::ReproductiveBarrierTiming,
        TemporalEvidenceDisposition::Unavailable,
    );
    with_transition(
        WideHistoryCase::Clean,
        IsolationCase::Supported,
        inside(151),
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        inputs,
        |_, _, evidence, _, _| {
            assert_eq!(evidence.status, SpeciationTransitionStatus::TransitionIntervalOnly)
        },
    );
}

#[test]
fn historical_model_applicability_is_independent_of_current_applicability() {
    with_transition(
        WideHistoryCase::Clean,
        IsolationCase::Supported,
        SpeciesModelApplicabilityInput::OutsideValidityDomain {
            evidence: authority("current-outside-domain", 152),
        },
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        supported_temporal_inputs(),
        |_, _, evidence, _, current| {
            assert_eq!(current.status, CurrentSpeciesStatus::OutsideModelValidityDomain);
            assert_eq!(
                evidence.status,
                SpeciationTransitionStatus::TransitionSupportedUnderModel
            );
        },
    );

    let mut historical_outside = supported_temporal_inputs();
    replace_disposition(
        &mut historical_outside,
        SpeciationTemporalCriterion::ModelApplicability,
        TemporalEvidenceDisposition::OutsideValidityDomain,
    );
    with_transition(
        WideHistoryCase::Clean,
        IsolationCase::Supported,
        inside(153),
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        historical_outside,
        |_, _, evidence, _, current| {
            assert_eq!(current.status, CurrentSpeciesStatus::SupportedUnderModel);
            assert_eq!(
                evidence.status,
                SpeciationTransitionStatus::OutsideModelValidityDomain
            );
        },
    );
}

#[test]
fn later_recontact_or_fusion_is_preserved_without_erasing_supported_historical_transition() {
    for (case, expected_kind, expected_current) in [
        (
            WideHistoryCase::RecontactAfter,
            HistoricalCounterHistoryKind::Recontact,
            CurrentSpeciesStatus::SupportedUnderModel,
        ),
        (
            WideHistoryCase::FusionAfter,
            HistoricalCounterHistoryKind::LineageFusionOrRemerger,
            CurrentSpeciesStatus::ContradictedUnderModel,
        ),
    ] {
        with_transition(
            case,
            IsolationCase::Supported,
            inside(154),
            SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
            supported_temporal_inputs(),
            |_, _, evidence, _, current| {
                assert_eq!(current.status, expected_current);
                assert_eq!(
                    evidence.status,
                    SpeciationTransitionStatus::TransitionSupportedUnderModel
                );
                assert!(evidence
                    .later_counter_history
                    .iter()
                    .any(|observation| observation.kind == expected_kind));
            },
        );
    }
}

#[test]
fn serialized_status_or_temporal_subject_tampering_fails_local_validation() {
    with_transition(
        WideHistoryCase::Clean,
        IsolationCase::Supported,
        inside(156),
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        supported_temporal_inputs(),
        |_, _, evidence, _, _| {
            let mut status = serde_json::to_value(evidence).unwrap();
            status["status"] = serde_json::json!("TransitionNotSupportedUnderModel");
            let changed: SpeciationTransitionEvidence = serde_json::from_value(status).unwrap();
            assert!(matches!(
                changed.canonical_digest(),
                Err(SpeciationTransitionEvidenceError::StatusInvariant)
            ));

            let mut subject = serde_json::to_value(evidence).unwrap();
            subject["temporal_evidence"][0]["species_model_digest"][0] = serde_json::json!(255);
            let changed: SpeciationTransitionEvidence = serde_json::from_value(subject).unwrap();
            assert!(matches!(
                changed.canonical_digest(),
                Err(SpeciationTransitionEvidenceError::TemporalSubjectBindingMismatch(_))
            ));
        },
    );
}

#[test]
fn wire_shape_contains_interval_bounds_but_no_exact_event_generation_or_time() {
    with_transition(
        WideHistoryCase::Clean,
        IsolationCase::Supported,
        inside(157),
        SpeciationTransitionMissingPolicy::ReportInsufficientTemporalEvidence,
        supported_temporal_inputs(),
        |_, _, evidence, _, _| {
            let value = serde_json::to_value(evidence).unwrap();
            let mut keys = Vec::new();
            collect_keys(&value, &mut keys);
            assert!(keys.iter().any(|key| key == "candidate_start_generation"));
            assert!(keys.iter().any(|key| key == "candidate_end_generation"));
            for forbidden in [
                "transition_generation",
                "exact_transition_generation",
                "speciation_generation",
                "event_generation",
                "transition_time",
                "event_time",
                "speciation_time",
            ] {
                assert!(!keys.iter().any(|key| key == forbidden));
            }
        },
    );
}
