use std::collections::BTreeMap;
use symtropy_evolution_core::{
    AlleleId, AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, EvolutionError,
    EvolutionExperimentId, HereditarySchema, HereditarySchemaId, LineageDivergenceHistory,
    LineageDivergenceHistoryDesign, LineageDivergenceHistoryError, LineageDivergenceHistoryId,
    LineageDivergenceHistoryStatus, LineageHistoryContextPolicy, LineageHistoryEpisodeId,
    LineageHistoryEpisodeInput, LineageHistoryEpisodeKind, LineageHistoryEvidenceProtocols,
    LineageHistoryGenerationInput, LineageHistoryGenerationRecord, LineageHistoryMissingPolicy,
    LineageObservationInput, LineagePersistenceInput, LocusDefinition, LocusId,
    ObservedLineageGenerationInput, PopulationGeneration, PopulationGeneticState, PopulationId,
    PopulationTrajectoryPoint, ValidatedLineageDivergenceHistory,
    ValidatedLineageDivergenceHistoryDesign,
};

fn authority(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("lineage-history-v1").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap()
}

fn population(
    schema: &HereditarySchema,
    id: &str,
    a_count: u64,
    b_count: u64,
) -> PopulationGeneticState {
    PopulationGeneticState::from_counts(
        PopulationId::new(id).unwrap(),
        schema,
        2,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele("a"), a_count), (allele("b"), b_count)]),
        )]),
    )
    .unwrap()
}

struct HistoryFixture {
    schema: HereditarySchema,
    a_populations: Vec<PopulationGeneticState>,
    b_populations: Vec<PopulationGeneticState>,
    a_points: Vec<PopulationTrajectoryPoint>,
    b_points: Vec<PopulationTrajectoryPoint>,
}

fn fixture() -> HistoryFixture {
    let schema = schema();
    let experiment = EvolutionExperimentId::new("lineage-history-fixture").unwrap();
    let mut a_populations = Vec::new();
    let mut b_populations = Vec::new();
    let mut a_points = Vec::new();
    let mut b_points = Vec::new();

    for generation in 1..=3 {
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

fn protocols() -> LineageHistoryEvidenceProtocols {
    LineageHistoryEvidenceProtocols {
        lineage_membership: authority("lineage-membership-protocol", 1),
        lineage_persistence: authority("lineage-persistence-protocol", 2),
        ancestry_relation: authority("ancestry-relation-protocol", 3),
        context: authority("context-protocol", 4),
        population_structure: authority("population-structure-protocol", 5),
        demographic_episode_census: authority("episode-census-protocol", 6),
        demographic_episode: authority("episode-protocol", 7),
        recontact: authority("recontact-protocol", 8),
        gene_flow: authority("gene-flow-protocol", 9),
        lineage_fusion: authority("fusion-protocol", 10),
    }
}

fn design_with(
    missing_policy: LineageHistoryMissingPolicy,
    context_policy: LineageHistoryContextPolicy,
) -> LineageDivergenceHistoryDesign {
    LineageDivergenceHistoryDesign::declare(
        LineageDivergenceHistoryId::new("history-a-b").unwrap(),
        authority("lineage-a", 20),
        authority("lineage-b", 21),
        PopulationGeneration(1),
        PopulationGeneration(3),
        protocols(),
        context_policy,
        missing_policy,
        authority("history-completeness", 22),
    )
    .unwrap()
}

fn current_design<'a>(
    design: &'a LineageDivergenceHistoryDesign,
) -> ValidatedLineageDivergenceHistoryDesign<'a> {
    ValidatedLineageDivergenceHistoryDesign::validate_current(
        design,
        authority("lineage-a", 20),
        authority("lineage-b", 21),
        PopulationGeneration(1),
        PopulationGeneration(3),
        protocols(),
        LineageHistoryContextPolicy::ExactAcrossInterval,
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        authority("history-completeness", 22),
    )
    .unwrap()
}

#[derive(Clone, Copy)]
enum GenerationKind {
    Clean,
    Recontact,
    Fusion,
    NotPersistent,
    ContextDrift,
}

fn observed_input<'a>(
    fixture: &'a HistoryFixture,
    generation: u64,
    kind: GenerationKind,
) -> LineageHistoryGenerationInput<'a> {
    let index = (generation - 1) as usize;
    let context_byte = if matches!(kind, GenerationKind::ContextDrift) {
        91
    } else {
        90
    };
    let persistence_b = if matches!(kind, GenerationKind::NotPersistent) {
        LineagePersistenceInput::NotPersistent {
            evidence: authority("lineage-b-not-persistent", 41),
        }
    } else {
        LineagePersistenceInput::Persistent {
            evidence: authority("lineage-b-persistent", 40),
        }
    };
    let recontact = if matches!(kind, GenerationKind::Recontact) {
        LineageObservationInput::Observed {
            evidence: authority("recontact-observed", 50),
        }
    } else {
        LineageObservationInput::NoneObserved {
            evidence: authority("recontact-none", 51),
        }
    };
    let gene_flow = if matches!(kind, GenerationKind::Recontact) {
        LineageObservationInput::Observed {
            evidence: authority("gene-flow-observed", 52),
        }
    } else {
        LineageObservationInput::NoneObserved {
            evidence: authority("gene-flow-none", 53),
        }
    };
    let fusion = if matches!(kind, GenerationKind::Fusion) {
        LineageObservationInput::Observed {
            evidence: authority("fusion-observed", 54),
        }
    } else {
        LineageObservationInput::NoneObserved {
            evidence: authority("fusion-none", 55),
        }
    };
    let episodes = if matches!(kind, GenerationKind::Recontact) {
        vec![LineageHistoryEpisodeInput {
            episode_id: LineageHistoryEpisodeId::new("recontact-episode").unwrap(),
            kind: LineageHistoryEpisodeKind::Recontact,
            proof_bundle_digest: None,
            evidence: authority("recontact-episode-evidence", 56),
        }]
    } else {
        vec![]
    };

    LineageHistoryGenerationInput::Observed(ObservedLineageGenerationInput {
        generation: PopulationGeneration(generation),
        lineage_a_point: &fixture.a_points[index],
        lineage_a_schema: &fixture.schema,
        lineage_a_population: &fixture.a_populations[index],
        lineage_b_point: &fixture.b_points[index],
        lineage_b_schema: &fixture.schema,
        lineage_b_population: &fixture.b_populations[index],
        lineage_a_membership_evidence: authority("lineage-a-membership", 30),
        lineage_b_membership_evidence: authority("lineage-b-membership", 31),
        lineage_a_persistence: LineagePersistenceInput::Persistent {
            evidence: authority("lineage-a-persistent", 39),
        },
        lineage_b_persistence: persistence_b,
        ancestry_relation_evidence: authority("ancestry-relation", 32),
        context_evidence: authority("context-observation", context_byte),
        population_structure_evidence: authority("structure-observation", 33),
        demographic_episode_census_evidence: authority("episode-census", 34),
        recontact,
        gene_flow,
        fusion,
        episodes,
    })
}

fn clean_inputs(fixture: &HistoryFixture) -> Vec<LineageHistoryGenerationInput<'_>> {
    vec![
        observed_input(fixture, 3, GenerationKind::Clean),
        observed_input(fixture, 1, GenerationKind::Clean),
        observed_input(fixture, 2, GenerationKind::Clean),
    ]
}

#[test]
fn complete_clean_history_is_persistent_and_replays_from_current_population_states() {
    let fixture = fixture();
    let design = design_with(
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current = current_design(&design);
    let history = LineageDivergenceHistory::capture(&current, clean_inputs(&fixture)).unwrap();
    assert_eq!(
        history.status,
        LineageDivergenceHistoryStatus::PersistentDivergenceObserved
    );
    assert_eq!(
        history
            .generations
            .iter()
            .map(|record| match record {
                LineageHistoryGenerationRecord::Observed(record) => record.generation.0,
                LineageHistoryGenerationRecord::Unavailable { generation, .. } => generation.0,
            })
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );

    let restored_design: LineageDivergenceHistoryDesign =
        serde_json::from_slice(&serde_json::to_vec(&design).unwrap()).unwrap();
    let restored_current = current_design(&restored_design);
    let restored: LineageDivergenceHistory =
        serde_json::from_slice(&serde_json::to_vec(&history).unwrap()).unwrap();
    let validated = ValidatedLineageDivergenceHistory::validate_current(
        &restored,
        &restored_current,
        clean_inputs(&fixture),
    )
    .unwrap();
    assert_eq!(
        validated.history_digest(),
        history.canonical_digest().unwrap()
    );
}

#[test]
fn recontact_and_gene_flow_remain_explicit_without_erasing_divergence() {
    let fixture = fixture();
    let design = design_with(
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current = current_design(&design);
    let history = LineageDivergenceHistory::capture(
        &current,
        vec![
            observed_input(&fixture, 1, GenerationKind::Clean),
            observed_input(&fixture, 2, GenerationKind::Recontact),
            observed_input(&fixture, 3, GenerationKind::Clean),
        ],
    )
    .unwrap();
    assert_eq!(
        history.status,
        LineageDivergenceHistoryStatus::DivergenceWithRecontact
    );
}

#[test]
fn fusion_and_loss_of_persistence_are_stronger_counterhistory_states() {
    let fixture = fixture();
    let design = design_with(
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current = current_design(&design);

    let fused = LineageDivergenceHistory::capture(
        &current,
        vec![
            observed_input(&fixture, 1, GenerationKind::Clean),
            observed_input(&fixture, 2, GenerationKind::Clean),
            observed_input(&fixture, 3, GenerationKind::Fusion),
        ],
    )
    .unwrap();
    assert_eq!(
        fused.status,
        LineageDivergenceHistoryStatus::LineageFusionObserved
    );

    let not_persistent = LineageDivergenceHistory::capture(
        &current,
        vec![
            observed_input(&fixture, 1, GenerationKind::Clean),
            observed_input(&fixture, 2, GenerationKind::NotPersistent),
            observed_input(&fixture, 3, GenerationKind::Clean),
        ],
    )
    .unwrap();
    assert_eq!(
        not_persistent.status,
        LineageDivergenceHistoryStatus::NotPersistent
    );
}

#[test]
fn unavailable_history_is_explicit_or_fail_closed_under_frozen_policy() {
    let fixture = fixture();
    let design = design_with(
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current = current_design(&design);
    let history = LineageDivergenceHistory::capture(
        &current,
        vec![
            observed_input(&fixture, 1, GenerationKind::Clean),
            LineageHistoryGenerationInput::Unavailable {
                generation: PopulationGeneration(2),
                reason: authority("generation-two-unavailable", 70),
            },
            observed_input(&fixture, 3, GenerationKind::Clean),
        ],
    )
    .unwrap();
    assert_eq!(
        history.status,
        LineageDivergenceHistoryStatus::InsufficientEvidence
    );

    let fail_closed = design_with(
        LineageHistoryMissingPolicy::FailClosed,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current_fail_closed = ValidatedLineageDivergenceHistoryDesign::validate_current(
        &fail_closed,
        authority("lineage-a", 20),
        authority("lineage-b", 21),
        PopulationGeneration(1),
        PopulationGeneration(3),
        protocols(),
        LineageHistoryContextPolicy::ExactAcrossInterval,
        LineageHistoryMissingPolicy::FailClosed,
        authority("history-completeness", 22),
    )
    .unwrap();
    assert!(matches!(
        LineageDivergenceHistory::capture(
            &current_fail_closed,
            vec![
                observed_input(&fixture, 1, GenerationKind::Clean),
                LineageHistoryGenerationInput::Unavailable {
                    generation: PopulationGeneration(2),
                    reason: authority("generation-two-unavailable", 70),
                },
                observed_input(&fixture, 3, GenerationKind::Clean),
            ],
        ),
        Err(LineageDivergenceHistoryError::UnavailableEvidenceForbidden)
    ));
}

#[test]
fn omitted_duplicate_generation_and_context_drift_fail_closed() {
    let fixture = fixture();
    let design = design_with(
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current = current_design(&design);

    assert!(matches!(
        LineageDivergenceHistory::capture(
            &current,
            vec![
                observed_input(&fixture, 1, GenerationKind::Clean),
                observed_input(&fixture, 3, GenerationKind::Clean),
            ],
        ),
        Err(LineageDivergenceHistoryError::IncompleteGenerationCoverage)
    ));

    assert!(matches!(
        LineageDivergenceHistory::capture(
            &current,
            vec![
                observed_input(&fixture, 1, GenerationKind::Clean),
                observed_input(&fixture, 2, GenerationKind::Clean),
                observed_input(&fixture, 2, GenerationKind::Clean),
            ],
        ),
        Err(LineageDivergenceHistoryError::DuplicateGeneration(
            PopulationGeneration(2)
        ))
    ));

    assert!(matches!(
        LineageDivergenceHistory::capture(
            &current,
            vec![
                observed_input(&fixture, 1, GenerationKind::Clean),
                observed_input(&fixture, 2, GenerationKind::ContextDrift),
                observed_input(&fixture, 3, GenerationKind::Clean),
            ],
        ),
        Err(LineageDivergenceHistoryError::ContextMismatch)
    ));
}

#[test]
fn stale_population_state_cannot_replay_a_persisted_trajectory_point() {
    let fixture = fixture();
    let design = design_with(
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current = current_design(&design);
    let changed = population(&fixture.schema, "lineage-a-pop", 2, 2);
    let input = LineageHistoryGenerationInput::Observed(ObservedLineageGenerationInput {
        generation: PopulationGeneration(1),
        lineage_a_point: &fixture.a_points[0],
        lineage_a_schema: &fixture.schema,
        lineage_a_population: &changed,
        lineage_b_point: &fixture.b_points[0],
        lineage_b_schema: &fixture.schema,
        lineage_b_population: &fixture.b_populations[0],
        lineage_a_membership_evidence: authority("lineage-a-membership", 30),
        lineage_b_membership_evidence: authority("lineage-b-membership", 31),
        lineage_a_persistence: LineagePersistenceInput::Persistent {
            evidence: authority("lineage-a-persistent", 39),
        },
        lineage_b_persistence: LineagePersistenceInput::Persistent {
            evidence: authority("lineage-b-persistent", 40),
        },
        ancestry_relation_evidence: authority("ancestry-relation", 32),
        context_evidence: authority("context-observation", 90),
        population_structure_evidence: authority("structure-observation", 33),
        demographic_episode_census_evidence: authority("episode-census", 34),
        recontact: LineageObservationInput::NoneObserved {
            evidence: authority("recontact-none", 51),
        },
        gene_flow: LineageObservationInput::NoneObserved {
            evidence: authority("gene-flow-none", 53),
        },
        fusion: LineageObservationInput::NoneObserved {
            evidence: authority("fusion-none", 55),
        },
        episodes: vec![],
    });
    assert!(matches!(
        LineageDivergenceHistory::capture(
            &current,
            vec![
                input,
                observed_input(&fixture, 2, GenerationKind::Clean),
                observed_input(&fixture, 3, GenerationKind::Clean),
            ],
        ),
        Err(LineageDivergenceHistoryError::Population(
            EvolutionError::PopulationTrajectoryStateMismatch
        ))
    ));
}

#[test]
fn serialized_status_tampering_is_locally_invalid() {
    let fixture = fixture();
    let design = design_with(
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current = current_design(&design);
    let history = LineageDivergenceHistory::capture(&current, clean_inputs(&fixture)).unwrap();
    let mut value = serde_json::to_value(history).unwrap();
    value["status"] = serde_json::json!("LineageFusionObserved");
    let changed: LineageDivergenceHistory = serde_json::from_value(value).unwrap();
    assert!(matches!(
        changed.canonical_digest(),
        Err(LineageDivergenceHistoryError::StatusInvariant)
    ));
}

#[test]
fn serialized_lineage_subject_transplant_is_locally_invalid() {
    let fixture = fixture();
    let design = design_with(
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current = current_design(&design);
    let history = LineageDivergenceHistory::capture(&current, clean_inputs(&fixture)).unwrap();
    let mut value = serde_json::to_value(history).unwrap();
    value["generations"][0]["Observed"]["lineage_a_membership"]["lineage"] =
        serde_json::to_value(authority("lineage-b", 21)).unwrap();
    let changed: LineageDivergenceHistory = serde_json::from_value(value).unwrap();
    assert!(matches!(
        changed.canonical_digest(),
        Err(LineageDivergenceHistoryError::LineageSubjectBindingMismatch)
    ));
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
fn lineage_history_wire_shape_contains_no_species_or_speciation_claim() {
    let fixture = fixture();
    let design = design_with(
        LineageHistoryMissingPolicy::ReportInsufficientEvidence,
        LineageHistoryContextPolicy::ExactAcrossInterval,
    );
    let current = current_design(&design);
    let history = LineageDivergenceHistory::capture(&current, clean_inputs(&fixture)).unwrap();
    let mut keys = Vec::new();
    collect_keys(&serde_json::to_value(history).unwrap(), &mut keys);
    for forbidden in [
        "species",
        "species_status",
        "species_concept",
        "speciation",
        "speciation_event",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
