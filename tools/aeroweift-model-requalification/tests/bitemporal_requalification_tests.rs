use aeroweift_model_requalification::{
    BitemporalCutoff, ChangeClass, ChangeEvent, ChangeEvidenceState, ChangeFeedCoverage,
    ClaimStatus, CoverageCompleteness, DependencyBinding, DependencyKind, EvaluationError,
    ModelEvidenceClaim, RequalificationPolicy, evaluate_claim,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn claim() -> ModelEvidenceClaim {
    ModelEvidenceClaim {
        claim_id: "claim-aero-001".into(),
        validation_package_sha256: digest('a'),
        validated_at_effective: 10,
        recorded_at: 10,
        dependencies: vec![
            DependencyBinding {
                kind: DependencyKind::ModelArtifact,
                digest: digest('b'),
                change_source_id: "model-registry".into(),
            },
            DependencyBinding {
                kind: DependencyKind::SensorStack,
                digest: digest('c'),
                change_source_id: "sensor-registry".into(),
            },
        ],
    }
}

fn coverage(
    dependency: DependencyKind,
    source: &str,
    completeness: CoverageCompleteness,
    state: ChangeEvidenceState,
) -> ChangeFeedCoverage {
    ChangeFeedCoverage {
        dependency,
        source_id: source.into(),
        effective_from: 0,
        effective_through: 100,
        knowledge_from: 0,
        knowledge_through: 100,
        first_sequence: 1,
        last_sequence: 100,
        completeness,
        evidence_state: state,
        manifest_sha256: digest('d'),
    }
}

fn complete_coverage() -> Vec<ChangeFeedCoverage> {
    vec![
        coverage(
            DependencyKind::ModelArtifact,
            "model-registry",
            CoverageCompleteness::Complete,
            ChangeEvidenceState::OwnerQualified,
        ),
        coverage(
            DependencyKind::SensorStack,
            "sensor-registry",
            CoverageCompleteness::Complete,
            ChangeEvidenceState::OwnerQualified,
        ),
    ]
}

macro_rules! event {
    (
        $id:expr,
        $source:expr,
        $sequence:expr,
        $dependency:expr,
        $prior:expr,
        $successor:expr,
        $effective_at:expr,
        $recorded_at:expr,
        $class:expr,
        $state:expr $(,)?
    ) => {
        ChangeEvent {
            event_id: $id.into(),
            source_id: $source.into(),
            source_sequence: $sequence,
            dependency: $dependency,
            prior_digest: digest($prior),
            successor_digest: digest($successor),
            effective_at: $effective_at,
            recorded_at: $recorded_at,
            change_class: $class,
            evidence_state: $state,
            event_sha256: digest('e'),
        }
    };
}

fn policy() -> RequalificationPolicy {
    RequalificationPolicy {
        policy_sha256: digest('f'),
    }
}

fn cutoff(effective_at: i64, known_at: i64) -> BitemporalCutoff {
    BitemporalCutoff {
        effective_at,
        known_at,
    }
}

fn assess(
    claim: &ModelEvidenceClaim,
    events: &[ChangeEvent],
    coverage: &[ChangeFeedCoverage],
    at: BitemporalCutoff,
) -> Result<aeroweift_model_requalification::ClaimAssessment, EvaluationError> {
    evaluate_claim(claim, events, coverage, at, &policy())
}

#[test]
fn positive_control_is_still_applicable_under_complete_coverage() {
    let result = assess(&claim(), &[], &complete_coverage(), cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::StillApplicable);
    assert!(result.affected_dimensions.is_empty());
    assert!(result.event_ids.is_empty());
    assert!(result.indeterminate_dimensions.is_empty());
    assert_eq!(result.evaluated_dependencies, claim().dependencies);
}

#[test]
fn sensor_calibration_change_invalidates_sensor_claim_not_model_artifact() {
    let change = event!(
        "sensor-cal-rotate",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(&claim(), &[change], &complete_coverage(), cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::RequalificationRequired);
    assert_eq!(
        result.affected_dimensions,
        vec![DependencyKind::SensorStack]
    );
    assert_eq!(
        result.requalification_dimensions,
        vec![DependencyKind::SensorStack]
    );
    assert_eq!(result.evaluated_dependencies[0].digest, digest('b'));
    assert_eq!(result.evaluated_dependencies[1].digest, digest('7'));
}

#[test]
fn unrelated_model_change_does_not_mark_sensor_dimension_as_changed() {
    let change = event!(
        "model-v2",
        "model-registry",
        2,
        DependencyKind::ModelArtifact,
        'b',
        '8',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(&claim(), &[change], &complete_coverage(), cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::RequalificationRequired);
    assert_eq!(
        result.affected_dimensions,
        vec![DependencyKind::ModelArtifact]
    );
    assert_eq!(
        result.requalification_dimensions,
        vec![DependencyKind::ModelArtifact]
    );
    assert_eq!(result.evaluated_dependencies[1].digest, digest('c'));
}

#[test]
fn late_recorded_event_does_not_rewrite_earlier_knowledge_cutoff() {
    let late = event!(
        "late-sensor-recal",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        20,
        30,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let historical = assess(
        &claim(),
        std::slice::from_ref(&late),
        &complete_coverage(),
        cutoff(25, 25),
    )
    .unwrap();
    let later = assess(&claim(), &[late], &complete_coverage(), cutoff(25, 35)).unwrap();

    assert_eq!(historical.status, ClaimStatus::StillApplicable);
    assert_eq!(historical.event_ids, Vec::<String>::new());
    assert_eq!(later.status, ClaimStatus::RequalificationRequired);
    assert_eq!(later.event_ids, vec!["late-sensor-recal".to_string()]);
}

#[test]
fn event_not_yet_effective_does_not_invalidate_earlier_effective_cutoff() {
    let scheduled = event!(
        "future-control-change",
        "model-registry",
        2,
        DependencyKind::ModelArtifact,
        'b',
        '8',
        60,
        30,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(&claim(), &[scheduled], &complete_coverage(), cutoff(50, 35)).unwrap();
    assert_eq!(result.status, ClaimStatus::StillApplicable);
}

#[test]
fn missing_dependency_feed_coverage_is_indeterminate() {
    let only_sensor = vec![coverage(
        DependencyKind::SensorStack,
        "sensor-registry",
        CoverageCompleteness::Complete,
        ChangeEvidenceState::OwnerQualified,
    )];
    let result = assess(&claim(), &[], &only_sensor, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::ModelArtifact]
    );
}

#[test]
fn partial_feed_coverage_is_not_evidence_of_no_change() {
    let mut feeds = complete_coverage();
    feeds[0].completeness = CoverageCompleteness::Partial;
    let result = assess(&claim(), &[], &feeds, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::ModelArtifact]
    );
}

#[test]
fn feed_coverage_that_ends_before_effective_cutoff_is_indeterminate() {
    let mut feeds = complete_coverage();
    feeds[1].effective_through = 49;
    let result = assess(&claim(), &[], &feeds, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::SensorStack]
    );
}

#[test]
fn feed_knowledge_frontier_must_reach_requested_knowledge_cutoff() {
    let mut feeds = complete_coverage();
    feeds[1].knowledge_through = 49;
    let result = assess(&claim(), &[], &feeds, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
}

#[test]
fn owner_unqualified_change_event_is_indeterminate_not_ignored() {
    let uncertain = event!(
        "sensor-event-untrusted",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerUnqualified,
    );
    let result = assess(&claim(), &[uncertain], &complete_coverage(), cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::SensorStack]
    );
    assert_eq!(
        result.affected_dimensions,
        vec![DependencyKind::SensorStack]
    );
}

#[test]
fn corrective_reference_event_contradicts_dependent_claim() {
    let correction = event!(
        "reference-correction",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::CorrectiveChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(
        &claim(),
        &[correction],
        &complete_coverage(),
        cutoff(50, 50),
    )
    .unwrap();
    assert_eq!(result.status, ClaimStatus::Contradicted);
}

#[test]
fn retirement_preserves_historical_claim_without_current_reliance() {
    let retired = event!(
        "sensor-retired",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::Retirement,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(&claim(), &[retired], &complete_coverage(), cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::HistoricalOnly);
    assert!(result.requalification_dimensions.is_empty());
}

#[test]
fn unsupported_change_class_does_not_receive_automatic_pass() {
    let unsupported = event!(
        "new-change-semantics",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::Unsupported,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(
        &claim(),
        &[unsupported],
        &complete_coverage(),
        cutoff(50, 50),
    )
    .unwrap();
    assert_eq!(result.status, ClaimStatus::UnsupportedChangeType);
}

#[test]
fn broken_dependency_digest_chain_is_indeterminate() {
    let broken = event!(
        "sensor-chain-break",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        '9',
        '7',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(&claim(), &[broken], &complete_coverage(), cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
}

#[test]
fn two_changes_must_form_exact_digest_chain() {
    let events = vec![
        event!(
            "sensor-step-1",
            "sensor-registry",
            2,
            DependencyKind::SensorStack,
            'c',
            '7',
            20,
            21,
            ChangeClass::MaterialChange,
            ChangeEvidenceState::OwnerQualified,
        ),
        event!(
            "sensor-step-2",
            "sensor-registry",
            3,
            DependencyKind::SensorStack,
            '7',
            '8',
            40,
            41,
            ChangeClass::MaterialChange,
            ChangeEvidenceState::OwnerQualified,
        ),
    ];
    let result = assess(&claim(), &events, &complete_coverage(), cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::RequalificationRequired);
    assert_eq!(result.evaluated_dependencies[1].digest, digest('8'));
    assert_eq!(
        result.event_ids,
        vec!["sensor-step-1".to_string(), "sensor-step-2".to_string()]
    );
}

#[test]
fn events_already_known_at_claim_creation_are_part_of_its_baseline() {
    let mut baseline_claim = claim();
    baseline_claim.dependencies[1].digest = digest('7');
    let baseline = event!(
        "sensor-before-claim",
        "sensor-registry",
        1,
        DependencyKind::SensorStack,
        'c',
        '7',
        5,
        6,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(
        &baseline_claim,
        &[baseline],
        &complete_coverage(),
        cutoff(50, 50),
    )
    .unwrap();
    assert_eq!(result.status, ClaimStatus::StillApplicable);
    assert_eq!(result.event_ids, Vec::<String>::new());
}

#[test]
fn late_recorded_correction_changes_later_view_but_not_earlier_view() {
    let correction = event!(
        "late-correction",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        5,
        30,
        ChangeClass::CorrectiveChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let before = assess(
        &claim(),
        std::slice::from_ref(&correction),
        &complete_coverage(),
        cutoff(50, 25),
    )
    .unwrap();
    let after = assess(
        &claim(),
        &[correction],
        &complete_coverage(),
        cutoff(50, 35),
    )
    .unwrap();
    assert_eq!(before.status, ClaimStatus::StillApplicable);
    assert_eq!(after.status, ClaimStatus::Contradicted);
}

#[test]
fn duplicate_event_id_is_rejected() {
    let e = event!(
        "duplicate",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    assert_eq!(
        assess(
            &claim(),
            &[e.clone(), e],
            &complete_coverage(),
            cutoff(50, 50)
        ),
        Err(EvaluationError::DuplicateEventId)
    );
}

#[test]
fn reused_source_sequence_is_rejected_as_fork_or_duplicate() {
    let events = vec![
        event!(
            "source-event-a",
            "sensor-registry",
            2,
            DependencyKind::SensorStack,
            'c',
            '7',
            40,
            42,
            ChangeClass::MaterialChange,
            ChangeEvidenceState::OwnerQualified,
        ),
        event!(
            "source-event-b",
            "sensor-registry",
            2,
            DependencyKind::SensorStack,
            '7',
            '8',
            41,
            43,
            ChangeClass::MaterialChange,
            ChangeEvidenceState::OwnerQualified,
        ),
    ];
    assert_eq!(
        assess(&claim(), &events, &complete_coverage(), cutoff(50, 50)),
        Err(EvaluationError::ForkedSourceSequence)
    );
}

#[test]
fn duplicate_coverage_for_one_dependency_source_is_rejected() {
    let mut feeds = complete_coverage();
    feeds.push(feeds[0].clone());
    assert_eq!(
        assess(&claim(), &[], &feeds, cutoff(50, 50)),
        Err(EvaluationError::DuplicateCoverage)
    );
}

#[test]
fn event_array_permutation_preserves_the_exact_result() {
    let events = vec![
        event!(
            "sensor-step-1",
            "sensor-registry",
            2,
            DependencyKind::SensorStack,
            'c',
            '7',
            20,
            21,
            ChangeClass::MaterialChange,
            ChangeEvidenceState::OwnerQualified,
        ),
        event!(
            "model-step-1",
            "model-registry",
            2,
            DependencyKind::ModelArtifact,
            'b',
            '8',
            30,
            31,
            ChangeClass::MaterialChange,
            ChangeEvidenceState::OwnerQualified,
        ),
    ];
    let expected = assess(&claim(), &events, &complete_coverage(), cutoff(50, 50)).unwrap();
    let mut permuted = events;
    permuted.reverse();
    assert_eq!(
        assess(&claim(), &permuted, &complete_coverage(), cutoff(50, 50)).unwrap(),
        expected
    );
}

#[test]
fn claim_not_yet_known_at_knowledge_cutoff_cannot_be_evaluated() {
    let mut future_claim = claim();
    future_claim.recorded_at = 60;
    assert_eq!(
        assess(&future_claim, &[], &complete_coverage(), cutoff(70, 50)),
        Err(EvaluationError::ClaimNotAvailableAtCutoff)
    );
}

#[test]
fn claim_not_yet_valid_at_effective_cutoff_cannot_be_evaluated() {
    let mut future_claim = claim();
    future_claim.validated_at_effective = 60;
    assert_eq!(
        assess(&future_claim, &[], &complete_coverage(), cutoff(50, 70)),
        Err(EvaluationError::ClaimNotAvailableAtCutoff)
    );
}

#[test]
fn evidence_coverage_deletion_cannot_improve_a_claim_status() {
    let full = assess(&claim(), &[], &complete_coverage(), cutoff(50, 50)).unwrap();
    let mut reduced = complete_coverage();
    reduced.retain(|item| item.dependency != DependencyKind::SensorStack);
    let partial = assess(&claim(), &[], &reduced, cutoff(50, 50)).unwrap();
    assert_eq!(full.status, ClaimStatus::StillApplicable);
    assert_eq!(partial.status, ClaimStatus::Indeterminate);
}

#[test]
fn material_change_plus_incomplete_other_feed_remains_indeterminate_but_explains_known_impact() {
    let change = event!(
        "sensor-known-change",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let only_sensor = vec![coverage(
        DependencyKind::SensorStack,
        "sensor-registry",
        CoverageCompleteness::Complete,
        ChangeEvidenceState::OwnerQualified,
    )];
    let result = assess(&claim(), &[change], &only_sensor, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
    assert_eq!(
        result.requalification_dimensions,
        vec![DependencyKind::SensorStack]
    );
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::ModelArtifact]
    );
}

#[test]
fn known_contradiction_is_not_hidden_by_an_unrelated_coverage_gap() {
    let correction = event!(
        "sensor-correction-known",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::CorrectiveChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let only_sensor = vec![coverage(
        DependencyKind::SensorStack,
        "sensor-registry",
        CoverageCompleteness::Complete,
        ChangeEvidenceState::OwnerQualified,
    )];
    let result = assess(&claim(), &[correction], &only_sensor, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Contradicted);
    assert_eq!(
        result.contradicted_dimensions,
        vec![DependencyKind::SensorStack]
    );
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::ModelArtifact]
    );
}

#[test]
fn historical_retirement_dimension_is_reported_explicitly() {
    let retired = event!(
        "solver-retired",
        "model-registry",
        2,
        DependencyKind::ModelArtifact,
        'b',
        '8',
        40,
        42,
        ChangeClass::Retirement,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(&claim(), &[retired], &complete_coverage(), cutoff(50, 50)).unwrap();
    assert_eq!(
        result.historical_only_dimensions,
        vec![DependencyKind::ModelArtifact]
    );
    assert!(result.contradicted_dimensions.is_empty());
}

#[test]
fn unsupported_change_dimension_is_reported_explicitly() {
    let unsupported = event!(
        "unknown-sensor-change",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::Unsupported,
        ChangeEvidenceState::OwnerQualified,
    );
    let result = assess(
        &claim(),
        &[unsupported],
        &complete_coverage(),
        cutoff(50, 50),
    )
    .unwrap();
    assert_eq!(
        result.unsupported_dimensions,
        vec![DependencyKind::SensorStack]
    );
}

#[test]
fn broken_preclaim_baseline_chain_is_indeterminate_even_when_final_digest_matches() {
    let mut baseline_claim = claim();
    baseline_claim.dependencies[1].digest = digest('8');
    let events = vec![
        event!(
            "baseline-sensor-step-1",
            "sensor-registry",
            1,
            DependencyKind::SensorStack,
            'c',
            '7',
            5,
            6,
            ChangeClass::MaterialChange,
            ChangeEvidenceState::OwnerQualified,
        ),
        event!(
            "baseline-sensor-step-2",
            "sensor-registry",
            2,
            DependencyKind::SensorStack,
            '9',
            '8',
            7,
            8,
            ChangeClass::MaterialChange,
            ChangeEvidenceState::OwnerQualified,
        ),
    ];
    let result = assess(
        &baseline_claim,
        &events,
        &complete_coverage(),
        cutoff(50, 50),
    )
    .unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::SensorStack]
    );
}

#[test]
fn source_sequence_not_effective_time_orders_causal_digest_succession() {
    let events = vec![
        event!(
            "sensor-change-seq-1",
            "sensor-registry",
            2,
            DependencyKind::SensorStack,
            'c',
            '7',
            30,
            31,
            ChangeClass::MaterialChange,
            ChangeEvidenceState::OwnerQualified,
        ),
        event!(
            "sensor-correction-seq-2",
            "sensor-registry",
            3,
            DependencyKind::SensorStack,
            '7',
            '8',
            20,
            32,
            ChangeClass::CorrectiveChange,
            ChangeEvidenceState::OwnerQualified,
        ),
    ];
    let result = assess(&claim(), &events, &complete_coverage(), cutoff(40, 40)).unwrap();
    assert_eq!(result.status, ClaimStatus::Contradicted);
    assert_eq!(
        result.event_ids,
        vec![
            "sensor-change-seq-1".to_string(),
            "sensor-correction-seq-2".to_string()
        ]
    );
    assert_eq!(result.evaluated_dependencies[1].digest, digest('8'));
}

#[test]
fn sequence_outside_declared_coverage_frontier_is_indeterminate() {
    let change = event!(
        "sensor-outside-sequence-window",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let mut feeds = complete_coverage();
    feeds[1].last_sequence = 1;
    let result = assess(&claim(), &[change], &feeds, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::SensorStack]
    );
}

#[test]
fn effective_coverage_must_include_claim_validation_time() {
    let mut feeds = complete_coverage();
    feeds[0].effective_from = 11;
    let result = assess(&claim(), &[], &feeds, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::ModelArtifact]
    );
}

#[test]
fn knowledge_coverage_must_include_claim_recorded_time() {
    let mut feeds = complete_coverage();
    feeds[1].knowledge_from = 11;
    let result = assess(&claim(), &[], &feeds, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::Indeterminate);
    assert_eq!(
        result.indeterminate_dimensions,
        vec![DependencyKind::SensorStack]
    );
}

#[test]
fn future_recorded_duplicate_does_not_poison_earlier_knowledge_snapshot() {
    let first = event!(
        "same-event-id",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let mut late_duplicate = first.clone();
    late_duplicate.recorded_at = 60;

    let earlier = assess(
        &claim(),
        &[first.clone(), late_duplicate.clone()],
        &complete_coverage(),
        cutoff(50, 50),
    )
    .unwrap();
    assert_eq!(earlier.status, ClaimStatus::RequalificationRequired);

    let later = assess(
        &claim(),
        &[first, late_duplicate],
        &complete_coverage(),
        cutoff(50, 70),
    );
    assert_eq!(later, Err(EvaluationError::DuplicateEventId));
}

#[test]
fn unrelated_dependency_feed_does_not_poison_claim_coverage() {
    let mut feeds = complete_coverage();
    let mut unrelated = coverage(
        DependencyKind::ControlStack,
        "unrelated-registry",
        CoverageCompleteness::Complete,
        ChangeEvidenceState::OwnerQualified,
    );
    unrelated.manifest_sha256 = digest('7');
    feeds.push(unrelated.clone());
    feeds.push(unrelated);

    let result = assess(&claim(), &[], &feeds, cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::StillApplicable);
}

#[test]
fn future_effective_coverage_receipt_does_not_poison_earlier_cutoff() {
    let mut feeds = complete_coverage();
    let mut future = feeds[1].clone();
    future.effective_from = 60;
    future.manifest_sha256 = digest('7');
    feeds.push(future);

    let earlier = assess(&claim(), &[], &feeds, cutoff(50, 50)).unwrap();
    assert_eq!(earlier.status, ClaimStatus::StillApplicable);

    let later = assess(&claim(), &[], &feeds, cutoff(70, 70));
    assert_eq!(later, Err(EvaluationError::DuplicateCoverage));
}

#[test]
fn unrelated_dependency_event_integrity_does_not_poison_claim() {
    let mut unrelated = event!(
        "unrelated-invalid-event",
        "control-registry",
        1,
        DependencyKind::ControlStack,
        '1',
        '2',
        20,
        21,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    unrelated.event_sha256 = "not-a-sha".into();

    let result = assess(&claim(), &[unrelated], &complete_coverage(), cutoff(50, 50)).unwrap();
    assert_eq!(result.status, ClaimStatus::StillApplicable);
}

#[test]
fn wrong_source_for_claim_dependency_is_rejected() {
    let wrong_source = event!(
        "sensor-event-wrong-source",
        "rogue-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );

    assert_eq!(
        assess(
            &claim(),
            &[wrong_source],
            &complete_coverage(),
            cutoff(50, 50)
        ),
        Err(EvaluationError::ChangeSourceMismatch)
    );
}

#[test]
fn future_recorded_source_sequence_fork_does_not_poison_earlier_knowledge_snapshot() {
    let first = event!(
        "sequence-event-a",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '7',
        40,
        42,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );
    let mut late_fork = event!(
        "sequence-event-b",
        "sensor-registry",
        2,
        DependencyKind::SensorStack,
        'c',
        '8',
        41,
        60,
        ChangeClass::MaterialChange,
        ChangeEvidenceState::OwnerQualified,
    );

    let earlier = assess(
        &claim(),
        &[first.clone(), late_fork.clone()],
        &complete_coverage(),
        cutoff(50, 50),
    )
    .unwrap();
    assert_eq!(earlier.status, ClaimStatus::RequalificationRequired);

    late_fork.recorded_at = 60;
    let later = assess(
        &claim(),
        &[first, late_fork],
        &complete_coverage(),
        cutoff(50, 70),
    );
    assert_eq!(later, Err(EvaluationError::ForkedSourceSequence));
}

#[test]
fn future_recorded_coverage_duplicate_does_not_poison_earlier_knowledge_snapshot() {
    let mut feeds = complete_coverage();
    let mut late_duplicate = feeds[1].clone();
    late_duplicate.recorded_at = 60;
    late_duplicate.manifest_sha256 = digest('7');
    feeds.push(late_duplicate);

    let earlier = assess(&claim(), &[], &feeds, cutoff(50, 50)).unwrap();
    assert_eq!(earlier.status, ClaimStatus::StillApplicable);

    let later = assess(&claim(), &[], &feeds, cutoff(50, 70));
    assert_eq!(later, Err(EvaluationError::DuplicateCoverage));
}
