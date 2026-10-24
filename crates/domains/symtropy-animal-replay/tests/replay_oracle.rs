// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::Deserialize;
use symtropy_animal_observation::{
    ClaimKind, DataOperation, GeneralizationTarget, LocationDisclosurePolicy, MeasurementField,
    ObservationModality, OperationPermission, PermissionDecision, ReferencePackManifest,
    SourceSnapshot, SplitRecord, SplitRole, TaxonomyBinding, TimeBasis,
};
use symtropy_animal_replay::{
    BaselineComparisonOutcome, BaselineReference, ComparisonBudgetPolicy, ConfidenceInterval,
    DecisionThreshold, EvaluationMode, MetricObjective, MetricObservation, MetricState,
    MetricStratumKind, PairedBaselineComparison, ReplayDisposition, ReplayEvaluationInput,
    ReplayReason, ReplayTaskSpec, RunIdentity, ThresholdDirection, ThresholdOutcome,
    build_replay_receipt,
};
use symtropy_fauna_core::{AnimalProfile, EvidenceClass, TaxonBinding};

const PROFILE_FIXTURE: &str =
    include_str!("../../symtropy-fauna-core/fixtures/animal-capability-profiles-v1.json");
const ORACLE_FIXTURE: &str = include_str!("../fixtures/replay-result-oracle-v1.json");

#[derive(Debug, Deserialize)]
struct ProfileFixture {
    synthetic_template: AnimalProfile,
}

#[derive(Debug, Deserialize)]
struct OracleCorpus {
    schema_version: String,
    note: String,
    cases: Vec<OracleCase>,
}

#[derive(Debug, Deserialize)]
struct OracleCase {
    id: String,
    mutation: String,
    expected_disposition: ReplayDisposition,
    expected_reason_codes: Vec<ReplayReason>,
    expected_threshold_outcome: ThresholdOutcome,
    #[serde(default)]
    expected_baseline_comparison_outcomes: Vec<BaselineComparisonOutcome>,
}

struct TestInputs {
    profile: AnimalProfile,
    manifest: ReferencePackManifest,
    mode: EvaluationMode,
    task: ReplayTaskSpec,
    run: RunIdentity,
    metrics: Vec<MetricObservation>,
    baselines: Vec<BaselineReference>,
    paired_comparisons: Vec<PairedBaselineComparison>,
}

fn profile_templates() -> ProfileFixture {
    serde_json::from_str(PROFILE_FIXTURE).unwrap()
}

fn synthetic_profile() -> AnimalProfile {
    profile_templates().synthetic_template
}

fn permission(operation: DataOperation) -> OperationPermission {
    OperationPermission {
        operation,
        decision: PermissionDecision::Granted,
        authority_ref: Some("test://authority/synthetic-fixture".to_owned()),
        scope_ref: Some("test://scope/local-evaluation".to_owned()),
        attribution: None,
        reviewed_at: Some("2026-01-01".to_owned()),
    }
}

fn valid_manifest() -> ReferencePackManifest {
    ReferencePackManifest {
        schema_version: "ANIMAL-OBSERVATION-REFERENCE-PACK-V1".to_owned(),
        pack_id: "synthetic/valid-pack".to_owned(),
        source: SourceSnapshot {
            source_id: "synthetic-source".to_owned(),
            dataset_id: "synthetic-dataset".to_owned(),
            version: "v1".to_owned(),
            immutable_ref: "test://snapshot/v1".to_owned(),
            byte_sha256: Some("a".repeat(64)),
            digest_required: true,
        },
        taxonomy: TaxonomyBinding {
            source: "test-taxonomy".to_owned(),
            release: "test-release-1".to_owned(),
            taxon_ids: vec!["TEST:1".to_owned()],
            mapping_ref: Some("test://mapping/1".to_owned()),
        },
        provenance: EvidenceClass::Synthetic,
        modality: ObservationModality::VideoKeypoints2d,
        fields: vec![
            MeasurementField {
                id: "keypoints".to_owned(),
                quantity: true,
                unit: Some("pixel".to_owned()),
                coordinate: true,
                coordinate_frame: Some("image-pixel-v1".to_owned()),
                temporal: true,
            },
            MeasurementField {
                id: "frame-index".to_owned(),
                quantity: true,
                unit: Some("frame".to_owned()),
                coordinate: false,
                coordinate_frame: None,
                temporal: false,
            },
        ],
        time_basis: Some(TimeBasis::StudyRelativeMilliseconds),
        sample_interval_ms: Some(40),
        requested_operations: vec![DataOperation::LocalAnalysis, DataOperation::EvaluateModel],
        permissions: vec![
            permission(DataOperation::LocalAnalysis),
            permission(DataOperation::EvaluateModel),
        ],
        location_disclosure: Some(LocationDisclosurePolicy {
            sensitive_species_location: false,
            minimum_export_grid_m: None,
            actual_export_grid_m: None,
        }),
        declared_supported_claims: vec![ClaimKind::VisiblePose2d, ClaimKind::Kinematics2d],
        requested_claims: vec![ClaimKind::VisiblePose2d],
        generalization: GeneralizationTarget::UnseenSubject,
        minimum_temporal_gap: 0,
        records: vec![
            SplitRecord {
                record_id: "r-train".to_owned(),
                study_id: "study-train".to_owned(),
                subject_id: "animal-1".to_owned(),
                site_id: Some("site-a".to_owned()),
                session_id: Some("session-a".to_owned()),
                track_id: "track-1".to_owned(),
                duplicate_group: Some("dup-train".to_owned()),
                taxon_id: "TEST:1".to_owned(),
                role: SplitRole::Training,
                start_time: 0,
                end_time: 100,
                provenance: EvidenceClass::Synthetic,
            },
            SplitRecord {
                record_id: "r-test".to_owned(),
                study_id: "study-test".to_owned(),
                subject_id: "animal-2".to_owned(),
                site_id: Some("site-b".to_owned()),
                session_id: Some("session-b".to_owned()),
                track_id: "track-2".to_owned(),
                duplicate_group: Some("dup-test".to_owned()),
                taxon_id: "TEST:1".to_owned(),
                role: SplitRole::Test,
                start_time: 200,
                end_time: 300,
                provenance: EvidenceClass::Synthetic,
            },
        ],
    }
}

fn metric(metric_id: &str, value: f64, unit: &str) -> MetricObservation {
    MetricObservation {
        metric_id: metric_id.to_owned(),
        stratum_kind: MetricStratumKind::Overall,
        stratum_id: "all".to_owned(),
        unit: unit.to_owned(),
        state: MetricState::Reported,
        value: Some(value),
        sample_count: 10,
        missing_count: 0,
        abstained_count: 0,
        failure_reason: None,
        interval: Some(ConfidenceInterval {
            method_id: "synthetic-test-interval-v1".to_owned(),
            confidence_percent: 90,
            lower: (value - 0.05).max(0.0),
            upper: (value + 0.05).min(1.0),
        }),
    }
}

fn metric_for_stratum(
    metric_id: &str,
    value: f64,
    unit: &str,
    stratum_kind: MetricStratumKind,
    stratum_id: &str,
) -> MetricObservation {
    let mut result = metric(metric_id, value, unit);
    result.stratum_kind = stratum_kind;
    result.stratum_id = stratum_id.to_owned();
    result
}

fn valid_inputs() -> TestInputs {
    let task = ReplayTaskSpec {
        task_id: "animal-pose-synthetic-oracle".to_owned(),
        task_version: "v1".to_owned(),
        claim: ClaimKind::VisiblePose2d,
        target_taxon_id: Some("TEST:1".to_owned()),
        generalization: GeneralizationTarget::UnseenSubject,
        primary_metric_id: "pose_pck".to_owned(),
        budget_policy: ComparisonBudgetPolicy::MatchedBudget,
        required_confidence_percent: 90,
        metric_uncertainty_method_id: "synthetic-test-interval-v1".to_owned(),
        paired_comparison_method_id: "synthetic-test-interval-v1".to_owned(),
        comparison_group: "synthetic-protocol-v1".to_owned(),
        threshold: Some(DecisionThreshold {
            metric_id: "pose_pck".to_owned(),
            direction: ThresholdDirection::AtLeast,
            value: 0.8,
        }),
        claim_ceiling: "synthetic evaluator behavior only; no empirical claim".to_owned(),
    };
    let run = RunIdentity {
        run_id: "test-run-0001".to_owned(),
        code_head: "a".repeat(40),
        config_sha256: "b".repeat(64),
        working_tree_clean: true,
        working_tree_sha256: None,
        evaluator_version: "animal-replay-evaluator-v1".to_owned(),
        toolchain: "rustc-test-fixture".to_owned(),
        platform: "test-platform".to_owned(),
        comparison_group: "synthetic-protocol-v1".to_owned(),
        training_data_scope: "synthetic-fixture-only".to_owned(),
        compute_budget: "unit-test".to_owned(),
        deterministic_replay: true,
        seed: Some(1),
    };
    TestInputs {
        profile: synthetic_profile(),
        manifest: valid_manifest(),
        mode: EvaluationMode::SyntheticOracle,
        task,
        run,
        metrics: vec![metric("pose_pck", 0.9, "fraction")],
        baselines: vec![BaselineReference {
            baseline_id: "persistence".to_owned(),
            revision: "v1".to_owned(),
            run_id: "persistence-run-v1".to_owned(),
            code_head: "c".repeat(40),
            config_sha256: "d".repeat(64),
            evaluator_version: "animal-replay-evaluator-v1".to_owned(),
            toolchain: "rustc-test-fixture".to_owned(),
            platform: "test-platform".to_owned(),
            comparison_group: "synthetic-protocol-v1".to_owned(),
            training_data_scope: "synthetic-fixture-only".to_owned(),
            compute_budget: "unit-test".to_owned(),
            primary_metric: metric("pose_pck", 0.4, "fraction"),
            additional_metrics: Vec::new(),
        }],
        paired_comparisons: vec![PairedBaselineComparison {
            baseline_id: "persistence".to_owned(),
            metric_id: "pose_pck".to_owned(),
            stratum_kind: MetricStratumKind::Overall,
            stratum_id: "all".to_owned(),
            comparison_group: "synthetic-protocol-v1".to_owned(),
            paired_unit_manifest_sha256: "e".repeat(64),
            objective: MetricObjective::HigherIsBetter,
            pairing_unit: "animal".to_owned(),
            independent_unit_count: 10,
            mean_difference: Some(0.5),
            difference_interval: Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: 0.35,
                upper: 0.65,
            }),
            minimum_effect_size: 0.1,
            non_inferiority_margin: 0.05,
        }],
    }
}

fn apply_mutation(inputs: &mut TestInputs, mutation: &str) {
    match mutation {
        "none" => {}
        "empty_task_id" => inputs.task.task_id.clear(),
        "empty_task_version" => inputs.task.task_version.clear(),
        "empty_claim_ceiling" => inputs.task.claim_ceiling.clear(),
        "task_claim_not_requested" => inputs.task.claim = ClaimKind::Kinematics2d,
        "task_generalization_mismatch" => {
            inputs.task.generalization = GeneralizationTarget::UnseenSite;
        }
        "target_taxon_unbound" => {
            inputs.task.target_taxon_id = Some("TEST:404".to_owned());
        }
        "empirical_synthetic_inputs" => inputs.mode = EvaluationMode::EmpiricalReference,
        "profile_without_capability_claims" => {
            inputs.profile.capabilities.clear();
        }
        "profile_taxon_for_synthetic" => {
            inputs.profile.taxon = Some(TaxonBinding {
                source: "test-taxonomy".to_owned(),
                release: "test-release-1".to_owned(),
                taxon_id: "TEST:1".to_owned(),
            });
        }
        "manifest_unknown_permission" => {
            inputs.manifest.permissions[0].decision = PermissionDecision::Unknown;
        }
        "manifest_restricted_permission" => {
            inputs.manifest.permissions[0].decision = PermissionDecision::Restricted;
        }
        "malformed_code_head" => inputs.run.code_head = "not-a-commit".to_owned(),
        "malformed_config_digest" => inputs.run.config_sha256 = "xyz".to_owned(),
        "dirty_tree_missing_digest" => {
            inputs.run.working_tree_clean = false;
            inputs.run.working_tree_sha256 = None;
        }
        "deterministic_seed_missing" => inputs.run.seed = None,
        "empty_run_id" => inputs.run.run_id.clear(),
        "run_comparison_group_mismatch" => {
            inputs.run.comparison_group = "different-protocol".to_owned();
        }
        "missing_primary_metric" => inputs.metrics.clear(),
        "duplicate_metric_id" => inputs.metrics.push(inputs.metrics[0].clone()),
        "nonfinite_metric_value" => inputs.metrics[0].value = Some(f64::NAN),
        "missing_metric_interval" => inputs.metrics[0].interval = None,
        "metric_missing_state" => {
            let primary = &mut inputs.metrics[0];
            primary.state = MetricState::Missing;
            primary.value = None;
            primary.sample_count = 0;
            primary.missing_count = 10;
            primary.interval = None;
            inputs.paired_comparisons[0].independent_unit_count = 0;
        }
        "empty_metric_stratum" => inputs.metrics[0].stratum_id.clear(),
        "invalid_overall_stratum_id" => inputs.metrics[0].stratum_id = "pooled".to_owned(),
        "taxon_stratum_unbound" => inputs.metrics.push(metric_for_stratum(
            "pose_pck",
            0.7,
            "fraction",
            MetricStratumKind::Taxon,
            "TEST:404",
        )),
        "same_metric_across_taxa" => {
            inputs.metrics.push(metric_for_stratum(
                "pose_pck",
                0.7,
                "fraction",
                MetricStratumKind::Taxon,
                "TEST:1",
            ));
            inputs.baselines[0]
                .additional_metrics
                .push(metric_for_stratum(
                    "pose_pck",
                    0.5,
                    "fraction",
                    MetricStratumKind::Taxon,
                    "TEST:1",
                ));
            inputs.paired_comparisons.push(PairedBaselineComparison {
                baseline_id: "persistence".to_owned(),
                metric_id: "pose_pck".to_owned(),
                stratum_kind: MetricStratumKind::Taxon,
                stratum_id: "TEST:1".to_owned(),
                comparison_group: "synthetic-protocol-v1".to_owned(),
                paired_unit_manifest_sha256: "e".repeat(64),
                objective: MetricObjective::HigherIsBetter,
                pairing_unit: "animal".to_owned(),
                independent_unit_count: 10,
                mean_difference: Some(0.2),
                difference_interval: Some(ConfidenceInterval {
                    method_id: "synthetic-test-interval-v1".to_owned(),
                    confidence_percent: 90,
                    lower: 0.15,
                    upper: 0.25,
                }),
                minimum_effect_size: 0.1,
                non_inferiority_margin: 0.05,
            });
        }
        "baseline_stratum_mismatch" => {
            inputs.baselines[0].primary_metric = metric_for_stratum(
                "pose_pck",
                0.4,
                "fraction",
                MetricStratumKind::Taxon,
                "TEST:1",
            );
        }
        "invalid_interval_bounds" => {
            inputs.metrics[0].interval = Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: 0.95,
                upper: 0.8,
            });
        }
        "nonfinite_threshold" => {
            inputs.task.threshold.as_mut().unwrap().value = f64::NAN;
        }
        "threshold_metric_mismatch" => {
            inputs.task.threshold.as_mut().unwrap().metric_id = "another_metric".to_owned();
        }
        "empty_uncertainty_method" => inputs.task.paired_comparison_method_id.clear(),
        "metric_confidence_protocol_mismatch" => {
            inputs.metrics[0]
                .interval
                .as_mut()
                .unwrap()
                .confidence_percent = 95;
        }
        "paired_confidence_protocol_mismatch" => {
            inputs.paired_comparisons[0]
                .difference_interval
                .as_mut()
                .unwrap()
                .confidence_percent = 95;
        }
        "paired_interval_method_mismatch" => {
            inputs.paired_comparisons[0]
                .difference_interval
                .as_mut()
                .unwrap()
                .method_id = "unregistered-paired-method".to_owned();
        }
        "missing_paired_comparison" => inputs.paired_comparisons.clear(),
        "duplicate_paired_comparison" => {
            inputs
                .paired_comparisons
                .push(inputs.paired_comparisons[0].clone());
        }
        "unknown_baseline_comparison" => {
            inputs.paired_comparisons[0].baseline_id = "missing-baseline".to_owned();
        }
        "comparison_group_mismatch" => {
            inputs.paired_comparisons[0].comparison_group = "different-protocol".to_owned();
        }
        "empty_pairing_unit" => inputs.paired_comparisons[0].pairing_unit.clear(),
        "zero_paired_samples" => inputs.paired_comparisons[0].independent_unit_count = 0,
        "missing_paired_difference" => inputs.paired_comparisons[0].mean_difference = None,
        "missing_paired_interval" => inputs.paired_comparisons[0].difference_interval = None,
        "invalid_paired_interval" => {
            inputs.paired_comparisons[0].difference_interval = Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: 0.8,
                upper: 0.2,
            });
        }
        "invalid_effect_margins" => inputs.paired_comparisons[0].minimum_effect_size = -0.1,
        "paired_difference_does_not_match_scores" => {
            inputs.paired_comparisons[0].mean_difference = Some(0.2);
            inputs.paired_comparisons[0].difference_interval = Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: 0.15,
                upper: 0.25,
            });
        }
        "paired_samples_exceed_available" => {
            inputs.paired_comparisons[0].independent_unit_count = 11;
        }
        "comparison_stratum_missing" => {
            inputs.paired_comparisons[0].stratum_kind = MetricStratumKind::Taxon;
            inputs.paired_comparisons[0].stratum_id = "TEST:404".to_owned();
        }
        "uncertain_paired_effect" => {
            inputs.metrics[0] = metric("pose_pck", 0.45, "fraction");
            inputs.paired_comparisons[0].mean_difference = Some(0.05);
            inputs.paired_comparisons[0].difference_interval = Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: -0.1,
                upper: 0.2,
            });
        }
        "noninferior_paired_effect" => {
            inputs.metrics[0] = metric("pose_pck", 0.42, "fraction");
            inputs.paired_comparisons[0].mean_difference = Some(0.02);
            inputs.paired_comparisons[0].difference_interval = Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: -0.02,
                upper: 0.06,
            });
        }
        "regression_paired_effect" => {
            inputs.metrics[0] = metric("pose_pck", 0.2, "fraction");
            inputs.paired_comparisons[0].mean_difference = Some(-0.2);
            inputs.paired_comparisons[0].difference_interval = Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: -0.3,
                upper: -0.1,
            });
        }
        "lower_is_better_outperforms" => {
            inputs.metrics[0] = metric("pose_pck", 0.2, "fraction");
            inputs.paired_comparisons[0].objective = MetricObjective::LowerIsBetter;
            inputs.paired_comparisons[0].mean_difference = Some(-0.2);
            inputs.paired_comparisons[0].difference_interval = Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: -0.3,
                upper: -0.1,
            });
            inputs.paired_comparisons[0].minimum_effect_size = 0.05;
        }
        "no_baseline" => {
            inputs.baselines.clear();
            inputs.paired_comparisons.clear();
        }
        "duplicate_baseline_id" => inputs.baselines.push(inputs.baselines[0].clone()),
        "baseline_metric_mismatch" => {
            inputs.baselines[0].primary_metric.metric_id = "another_metric".to_owned();
        }
        "baseline_unit_mismatch" => {
            inputs.baselines[0].primary_metric.unit = "pixels".to_owned();
        }
        "baseline_missing_metric" => {
            let primary = &mut inputs.baselines[0].primary_metric;
            primary.state = MetricState::Missing;
            primary.value = None;
            primary.sample_count = 0;
            primary.missing_count = 10;
            primary.interval = None;
            inputs.paired_comparisons[0].independent_unit_count = 0;
        }
        "below_threshold" => {
            inputs.metrics[0] = metric("pose_pck", 0.6, "fraction");
            inputs.paired_comparisons[0].mean_difference = Some(0.2);
            inputs.paired_comparisons[0].difference_interval = Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: 0.15,
                upper: 0.25,
            });
        }
        "threshold_interval_overlaps" => {
            inputs.metrics[0] = metric("pose_pck", 0.79, "fraction");
            inputs.paired_comparisons[0].mean_difference = Some(0.39);
            inputs.paired_comparisons[0].difference_interval = Some(ConfidenceInterval {
                method_id: "synthetic-test-interval-v1".to_owned(),
                confidence_percent: 90,
                lower: 0.34,
                upper: 0.44,
            });
        }
        "no_threshold" => inputs.task.threshold = None,
        "target_taxon_absent_from_test" => {
            inputs.manifest.taxonomy.taxon_ids.push("TEST:2".to_owned());
            inputs.manifest.records[1].taxon_id = "TEST:2".to_owned();
        }
        "unseen_taxon_split_leak" => {
            inputs.task.generalization = GeneralizationTarget::UnseenTaxon;
            inputs.manifest.generalization = GeneralizationTarget::UnseenTaxon;
        }
        "baseline_invalid_code_head" => inputs.baselines[0].code_head = "not-a-commit".to_owned(),
        "baseline_invalid_config_digest" => inputs.baselines[0].config_sha256 = "bad".to_owned(),
        "baseline_training_scope_mismatch" => {
            inputs.baselines[0].training_data_scope = "different-training-data".to_owned();
        }
        "baseline_compute_budget_mismatch" => {
            inputs.baselines[0].compute_budget = "larger-budget".to_owned();
        }
        "declared_budget_tradeoff_allows_scope_mismatch" => {
            inputs.task.budget_policy = ComparisonBudgetPolicy::DeclaredBudgetTradeoff;
            inputs.baselines[0].training_data_scope = "different-training-data".to_owned();
        }
        "missing_paired_unit_manifest_digest" => {
            inputs.paired_comparisons[0]
                .paired_unit_manifest_sha256
                .clear();
        }
        "malformed_paired_unit_manifest_digest" => {
            inputs.paired_comparisons[0].paired_unit_manifest_sha256 = "bad-digest".to_owned();
        }
        "empty_baseline_compute_budget" => inputs.baselines[0].compute_budget.clear(),
        "empty_platform" => inputs.run.platform.clear(),
        "empty_evaluator_version" => inputs.run.evaluator_version.clear(),
        other => panic!("unknown fixture mutation: {other}"),
    }
}

fn build(inputs: TestInputs) -> symtropy_animal_replay::ReplayReceipt {
    build_replay_receipt(ReplayEvaluationInput {
        profile: &inputs.profile,
        manifest: &inputs.manifest,
        mode: inputs.mode,
        task: inputs.task,
        run: inputs.run,
        metrics: inputs.metrics,
        baselines: inputs.baselines,
        paired_baseline_comparisons: inputs.paired_comparisons,
    })
    .unwrap()
}

#[test]
fn frozen_replay_result_oracle_matches_all_expected_outcomes() {
    let corpus: OracleCorpus = serde_json::from_str(ORACLE_FIXTURE).unwrap();
    assert_eq!(corpus.schema_version, "ANIMAL-REPLAY-RESULT-ORACLE-V1");
    assert!(corpus.note.contains("synthetic"));
    assert_eq!(corpus.cases.len(), 72);
    for case in corpus.cases {
        let mut inputs = valid_inputs();
        apply_mutation(&mut inputs, &case.mutation);
        let receipt = build(inputs);
        assert_eq!(
            receipt.payload.assessment.disposition, case.expected_disposition,
            "case {}",
            case.id
        );
        assert_eq!(
            receipt.payload.assessment.reason_codes, case.expected_reason_codes,
            "case {}",
            case.id
        );
        assert_eq!(
            receipt.payload.threshold_outcome, case.expected_threshold_outcome,
            "case {}",
            case.id
        );
        assert_eq!(
            receipt
                .payload
                .baseline_comparison_results
                .iter()
                .map(|result| result.outcome)
                .collect::<Vec<_>>(),
            case.expected_baseline_comparison_outcomes,
            "baseline comparisons for case {}",
            case.id
        );
        assert_eq!(receipt.payload.qualification_scope, "synthetic_oracle_only");
        assert_eq!(receipt.receipt_sha256.len(), 64);
    }
}

#[test]
fn receipt_digest_is_invariant_to_input_vector_order() {
    let mut first = valid_inputs();
    first
        .metrics
        .push(metric("turn_rate", 0.3, "rad_per_second"));
    first.baselines.push(BaselineReference {
        baseline_id: "constant-velocity".to_owned(),
        revision: "v1".to_owned(),
        run_id: "constant-velocity-run-v1".to_owned(),
        code_head: "c".repeat(40),
        config_sha256: "d".repeat(64),
        evaluator_version: "animal-replay-evaluator-v1".to_owned(),
        toolchain: "rustc-test-fixture".to_owned(),
        platform: "test-platform".to_owned(),
        comparison_group: "synthetic-protocol-v1".to_owned(),
        training_data_scope: "synthetic-fixture-only".to_owned(),
        compute_budget: "unit-test".to_owned(),
        primary_metric: metric("pose_pck", 0.5, "fraction"),
        additional_metrics: Vec::new(),
    });
    first.paired_comparisons.push(PairedBaselineComparison {
        baseline_id: "constant-velocity".to_owned(),
        metric_id: "pose_pck".to_owned(),
        stratum_kind: MetricStratumKind::Overall,
        stratum_id: "all".to_owned(),
        comparison_group: "synthetic-protocol-v1".to_owned(),
        paired_unit_manifest_sha256: "e".repeat(64),
        objective: MetricObjective::HigherIsBetter,
        pairing_unit: "animal".to_owned(),
        independent_unit_count: 10,
        mean_difference: Some(0.4),
        difference_interval: Some(ConfidenceInterval {
            method_id: "synthetic-test-interval-v1".to_owned(),
            confidence_percent: 90,
            lower: 0.3,
            upper: 0.5,
        }),
        minimum_effect_size: 0.1,
        non_inferiority_margin: 0.05,
    });
    let expected = build(first);

    let mut second = valid_inputs();
    second
        .metrics
        .push(metric("turn_rate", 0.3, "rad_per_second"));
    second.baselines.push(BaselineReference {
        baseline_id: "constant-velocity".to_owned(),
        revision: "v1".to_owned(),
        run_id: "constant-velocity-run-v1".to_owned(),
        code_head: "c".repeat(40),
        config_sha256: "d".repeat(64),
        evaluator_version: "animal-replay-evaluator-v1".to_owned(),
        toolchain: "rustc-test-fixture".to_owned(),
        platform: "test-platform".to_owned(),
        comparison_group: "synthetic-protocol-v1".to_owned(),
        training_data_scope: "synthetic-fixture-only".to_owned(),
        compute_budget: "unit-test".to_owned(),
        primary_metric: metric("pose_pck", 0.5, "fraction"),
        additional_metrics: Vec::new(),
    });
    second.paired_comparisons.push(PairedBaselineComparison {
        baseline_id: "constant-velocity".to_owned(),
        metric_id: "pose_pck".to_owned(),
        stratum_kind: MetricStratumKind::Overall,
        stratum_id: "all".to_owned(),
        comparison_group: "synthetic-protocol-v1".to_owned(),
        paired_unit_manifest_sha256: "e".repeat(64),
        objective: MetricObjective::HigherIsBetter,
        pairing_unit: "animal".to_owned(),
        independent_unit_count: 10,
        mean_difference: Some(0.4),
        difference_interval: Some(ConfidenceInterval {
            method_id: "synthetic-test-interval-v1".to_owned(),
            confidence_percent: 90,
            lower: 0.3,
            upper: 0.5,
        }),
        minimum_effect_size: 0.1,
        non_inferiority_margin: 0.05,
    });
    second.metrics.reverse();
    second.baselines.reverse();
    second.paired_comparisons.reverse();
    second.manifest.records.reverse();
    second.manifest.fields.reverse();
    second.manifest.permissions.reverse();
    second.manifest.taxonomy.taxon_ids.reverse();
    second.profile.capabilities.reverse();
    let actual = build(second);

    assert_eq!(expected.receipt_sha256, actual.receipt_sha256);
    assert_eq!(
        expected.payload.manifest_sha256,
        actual.payload.manifest_sha256
    );
    assert_eq!(expected.payload.split_sha256, actual.payload.split_sha256);
}

#[test]
fn receipt_digest_binds_metrics_seed_and_permission_scope() {
    let original = build(valid_inputs());

    let mut changed_metric = valid_inputs();
    changed_metric.metrics[0] = metric("pose_pck", 0.85, "fraction");
    changed_metric.paired_comparisons[0].mean_difference = Some(0.45);
    changed_metric.paired_comparisons[0].difference_interval = Some(ConfidenceInterval {
        method_id: "synthetic-test-interval-v1".to_owned(),
        confidence_percent: 90,
        lower: 0.4,
        upper: 0.5,
    });
    assert_ne!(
        original.receipt_sha256,
        build(changed_metric).receipt_sha256
    );

    let mut changed_seed = valid_inputs();
    changed_seed.run.seed = Some(2);
    assert_ne!(original.receipt_sha256, build(changed_seed).receipt_sha256);

    let mut changed_scope = valid_inputs();
    changed_scope.manifest.permissions[0].scope_ref =
        Some("test://scope/different-operation".to_owned());
    let changed_scope_receipt = build(changed_scope);
    assert_ne!(
        original.receipt_sha256,
        changed_scope_receipt.receipt_sha256
    );
    assert_ne!(
        original.payload.manifest_sha256,
        changed_scope_receipt.payload.manifest_sha256
    );
}

#[test]
fn synthetic_receipt_never_claims_empirical_qualification() {
    let receipt = build(valid_inputs());
    assert_eq!(receipt.payload.qualification_scope, "synthetic_oracle_only");
    assert_ne!(
        receipt.payload.qualification_scope,
        "empirical_reference_evaluation"
    );
}
