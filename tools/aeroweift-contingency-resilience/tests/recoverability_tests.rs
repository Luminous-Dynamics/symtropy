use aeroweift_contingency_resilience::{
    Applicability, AuthorizationState, ContingencyCase, ContingencyKind, EvaluationError,
    EvidenceState, InventoryState, OptionBounds, OptionDisposition, OptionInventory,
    RecoverabilityPolicy, RecoverabilityStatus, RecoveryOption, SubjectScope,
    evaluate_recoverability,
};
use std::collections::BTreeSet;

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn scope() -> SubjectScope {
    SubjectScope {
        aircraft_id: "aircraft-pseudo-01".into(),
        flight_id: "flight-001".into(),
        operation_id: "mission-001".into(),
        intent_id: "intent-001".into(),
        intent_revision: 7,
        intent_sha256: digest('a'),
        authorization_id: "authorization-001".into(),
        authorization_revision: 4,
        authorization_sha256: digest('b'),
        model_artifact_sha256: digest('c'),
        operating_domain_sha256: digest('d'),
        validation_package_sha256: digest('e'),
        reference_basis_sha256: digest('f'),
    }
}

fn case() -> ContingencyCase {
    ContingencyCase {
        contingency_id: "c2-loss-case".into(),
        kind: ContingencyKind::LostCommandAndControl,
        subject: scope(),
        applicability: Applicability::InDomain,
        effective_at: 50,
        known_at: 50,
        failed_domains: vec![],
        response_deadline_ms: 10_000.0,
        required_energy_reserve: 10.0,
        minimum_containment: 0.95,
        minimum_separation_margin_m: 50.0,
        maximum_ground_risk: 0.10,
        required_independent_options: 2,
    }
}

fn policy() -> RecoverabilityPolicy {
    RecoverabilityPolicy {
        policy_sha256: digest('9'),
        supported_contingencies: BTreeSet::from([
            ContingencyKind::LostCommandAndControl,
            ContingencyKind::NavigationDegraded,
            ContingencyKind::TrafficConflict,
            ContingencyKind::EnergyOrPropulsionDegraded,
            ContingencyKind::DynamicRestriction,
            ContingencyKind::EnvironmentalDisturbance,
        ]),
        diversity_exempt_domains: BTreeSet::from(["AIRFRAME".to_string()]),
        maximum_determinate_option_count: 24,
    }
}

fn inventory() -> OptionInventory {
    OptionInventory {
        source_id: "recovery-planner-registry".into(),
        manifest_sha256: digest('8'),
        state: InventoryState::Complete,
        effective_from: 0,
        effective_through: 1_000,
        knowledge_from: 0,
        knowledge_through: 1_000,
    }
}

fn option(id: &str, dependencies: &[&str]) -> RecoveryOption {
    RecoveryOption {
        option_id: id.into(),
        contingency_id: case().contingency_id,
        subject: scope(),
        applicability: Applicability::InDomain,
        evidence_state: EvidenceState::Qualified,
        authorization_state: AuthorizationState::AuthorizedForContingency,
        evidence_valid_until: 100,
        evidence_binding_sha256: digest('7'),
        evidence_receipt_ids: vec![format!("receipt-{id}")],
        bounds: OptionBounds {
            response_time_upper_ms: 3_000.0,
            energy_available_lower: 1_000.0,
            energy_required_upper: 500.0,
            containment_lower: 0.99,
            separation_margin_lower_m: 100.0,
            ground_risk_upper: 0.05,
        },
        dependency_inventory_complete: true,
        dependency_domains: dependencies.iter().map(|value| (*value).into()).collect(),
    }
}

fn evaluate(
    contingency: &ContingencyCase,
    options: &[RecoveryOption],
    inv: &OptionInventory,
    pol: &RecoverabilityPolicy,
) -> Result<aeroweift_contingency_resilience::RecoverabilityAssessment, EvaluationError> {
    evaluate_recoverability(contingency, options, inv, pol)
}

#[test]
fn positive_control_proves_two_independent_recovery_options() {
    let result = evaluate(
        &case(),
        &[
            option("return", &["C2", "AIRFRAME"]),
            option("divert", &["GNSS", "AIRFRAME"]),
        ],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(
        result.status,
        RecoverabilityStatus::CoveredWithRequiredDiversity
    );
    assert_eq!(result.admissible_option_ids, vec!["divert", "return"]);
    assert_eq!(result.independent_option_witness, vec!["divert", "return"]);
    assert!(result.inventory_complete);
}

#[test]
fn common_c2_dependency_prevents_false_diversity_credit() {
    let result = evaluate(
        &case(),
        &[option("route-east", &["C2"]), option("route-west", &["C2"])],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::CoveredButCorrelated);
    assert_eq!(result.independent_option_witness.len(), 1);
}

#[test]
fn different_geometry_does_not_override_shared_gnss_dependency() {
    let result = evaluate(
        &case(),
        &[
            option("geometry-a", &["GNSS"]),
            option("geometry-b", &["GNSS"]),
        ],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::CoveredButCorrelated);
}

#[test]
fn known_failed_dependency_rejects_the_option() {
    let mut contingency = case();
    contingency.failed_domains = vec!["C2".into()];
    let result = evaluate(
        &contingency,
        &[option("c2-dependent", &["C2"])],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
    assert!(
        result.options[0]
            .rejection_reasons
            .contains(&aeroweift_contingency_resilience::RejectionReason::FailedDependency)
    );
}

#[test]
fn incomplete_option_dependency_inventory_is_indeterminate() {
    let mut candidate = option("unknown-dependencies", &["C2"]);
    candidate.dependency_inventory_complete = false;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
    assert_eq!(
        result.options[0].disposition,
        OptionDisposition::Indeterminate
    );
}

#[test]
fn insufficient_energy_reserve_rejects_option() {
    let mut candidate = option("low-energy", &["GNSS"]);
    candidate.bounds.energy_available_lower = 509.0;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
    assert!(result.options[0].margins.energy_slack < 0.0);
}

#[test]
fn response_deadline_miss_rejects_option() {
    let mut candidate = option("too-slow", &["GNSS"]);
    candidate.bounds.response_time_upper_ms = 10_001.0;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
    assert!(result.options[0].margins.response_slack_ms < 0.0);
}

#[test]
fn occupancy_containment_lower_bound_must_pass() {
    let mut candidate = option("low-containment", &["GNSS"]);
    candidate.bounds.containment_lower = 0.94;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
}

#[test]
fn separation_margin_lower_bound_must_pass() {
    let mut candidate = option("low-separation", &["GNSS"]);
    candidate.bounds.separation_margin_lower_m = 49.9;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
}

#[test]
fn ground_risk_upper_bound_must_pass() {
    let mut candidate = option("high-ground-risk", &["GNSS"]);
    candidate.bounds.ground_risk_upper = 0.11;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
}

#[test]
fn exact_threshold_equality_is_admissible() {
    let mut candidate = option("at-threshold", &["GNSS"]);
    candidate.bounds.response_time_upper_ms = 10_000.0;
    candidate.bounds.energy_available_lower = 510.0;
    candidate.bounds.containment_lower = 0.95;
    candidate.bounds.separation_margin_lower_m = 50.0;
    candidate.bounds.ground_risk_upper = 0.10;
    let mut contingency = case();
    contingency.required_independent_options = 1;
    let result = evaluate(&contingency, &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::CoveredSingleOption);
}

#[test]
fn reauthorization_required_is_not_counted_as_admissible() {
    let mut candidate = option("needs-authority", &["GNSS"]);
    candidate.authorization_state = AuthorizationState::ReauthorizationRequired;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::ReauthorizationRequired);
    assert!(result.admissible_option_ids.is_empty());
}

#[test]
fn explicitly_unauthorized_option_is_rejected() {
    let mut candidate = option("unauthorized", &["GNSS"]);
    candidate.authorization_state = AuthorizationState::NotAuthorized;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
}

#[test]
fn unknown_authority_state_is_indeterminate() {
    let mut candidate = option("authority-unknown", &["GNSS"]);
    candidate.authorization_state = AuthorizationState::Unknown;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
}

#[test]
fn unknown_evidence_state_is_indeterminate() {
    let mut candidate = option("evidence-unknown", &["GNSS"]);
    candidate.evidence_state = EvidenceState::Unknown;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
}

#[test]
fn rejected_evidence_cannot_be_revived_by_an_option_wrapper() {
    let mut candidate = option("evidence-rejected", &["GNSS"]);
    candidate.evidence_state = EvidenceState::Rejected;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
}

#[test]
fn expired_evidence_cannot_be_revived_by_a_fresh_delivery() {
    let mut candidate = option("expired", &["GNSS"]);
    candidate.evidence_valid_until = 50;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
}

#[test]
fn subject_authorization_revision_must_match_exactly() {
    let mut candidate = option("wrong-auth-revision", &["GNSS"]);
    candidate.subject.authorization_revision += 1;
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
    assert!(
        result.options[0]
            .rejection_reasons
            .contains(&aeroweift_contingency_resilience::RejectionReason::SubjectScopeMismatch)
    );
}

#[test]
fn model_revision_mismatch_cannot_inherit_old_recoverability_credit() {
    let mut candidate = option("wrong-model", &["GNSS"]);
    candidate.subject.model_artifact_sha256 = digest('6');
    let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
}

#[test]
fn out_of_domain_case_is_not_credited() {
    let mut contingency = case();
    contingency.applicability = Applicability::OutOfDomain;
    let result = evaluate(
        &contingency,
        &[option("route", &["GNSS"])],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::OutOfDomain);
}

#[test]
fn unknown_case_applicability_is_indeterminate() {
    let mut contingency = case();
    contingency.applicability = Applicability::Unknown;
    let result = evaluate(
        &contingency,
        &[option("route", &["GNSS"])],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
}

#[test]
fn unsupported_contingency_class_is_explicit() {
    let mut contingency = case();
    contingency.kind = ContingencyKind::Other;
    let result = evaluate(&contingency, &[], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::UnsupportedContingency);
}

#[test]
fn partial_inventory_never_becomes_no_admissible_option() {
    let mut inv = inventory();
    inv.state = InventoryState::Partial;
    let result = evaluate(&case(), &[], &inv, &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
}

#[test]
fn unknown_inventory_never_becomes_no_admissible_option() {
    let mut inv = inventory();
    inv.state = InventoryState::Unknown;
    let result = evaluate(&case(), &[], &inv, &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
}

#[test]
fn empty_complete_inventory_establishes_no_admissible_option() {
    let result = evaluate(&case(), &[], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
}

#[test]
fn known_failure_plus_unknown_candidate_preserves_indeterminate_status() {
    let mut failed = option("failed", &["C2"]);
    let mut unknown = option("unknown", &["GNSS"]);
    failed.bounds.energy_available_lower = 1.0;
    unknown.evidence_state = EvidenceState::Unknown;
    let result = evaluate(&case(), &[failed, unknown], &inventory(), &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
    assert!(
        result
            .options
            .iter()
            .any(|item| !item.rejection_reasons.is_empty())
    );
    assert!(
        result
            .options
            .iter()
            .any(|item| !item.uncertainty_reasons.is_empty())
    );
}

#[test]
fn common_airframe_dependency_can_be_explicitly_exempted_from_diversity() {
    let result = evaluate(
        &case(),
        &[
            option("route-a", &["AIRFRAME", "C2"]),
            option("route-b", &["AIRFRAME", "GNSS"]),
        ],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(
        result.status,
        RecoverabilityStatus::CoveredWithRequiredDiversity
    );
    assert_eq!(
        result.options[0].diversity_dependencies,
        vec!["C2".to_string()]
    );
}

#[test]
fn failed_dependency_is_still_rejected_when_exempt_for_diversity() {
    let mut contingency = case();
    contingency.failed_domains = vec!["AIRFRAME".into()];
    let result = evaluate(
        &contingency,
        &[option("route-a", &["AIRFRAME", "C2"])],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::NoAdmissibleOption);
}

#[test]
fn same_scope_and_bounds_with_three_required_options_abstains_if_witness_is_too_small() {
    let mut contingency = case();
    contingency.required_independent_options = 3;
    let result = evaluate(
        &contingency,
        &[option("route-a", &["C2"]), option("route-b", &["GNSS"])],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
    assert_eq!(result.independent_option_witness.len(), 2);
}

#[test]
fn partial_inventory_with_one_known_option_does_not_claim_required_diversity() {
    let mut inv = inventory();
    inv.state = InventoryState::Partial;
    let result = evaluate(&case(), &[option("route-a", &["C2"])], &inv, &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
}

#[test]
fn option_permutation_preserves_canonical_assessment() {
    let options = vec![
        option("return", &["C2", "AIRFRAME"]),
        option("divert", &["GNSS", "AIRFRAME"]),
        option("hold", &["C2", "GNSS"]),
    ];
    let expected = evaluate(&case(), &options, &inventory(), &policy()).unwrap();
    let mut permuted = options;
    permuted.reverse();
    assert_eq!(
        evaluate(&case(), &permuted, &inventory(), &policy()).unwrap(),
        expected
    );
}

#[test]
fn duplicate_option_identity_is_rejected() {
    let candidate = option("duplicate", &["GNSS"]);
    assert_eq!(
        evaluate(
            &case(),
            &[candidate.clone(), candidate],
            &inventory(),
            &policy()
        ),
        Err(EvaluationError::DuplicateOptionId)
    );
}

#[test]
fn duplicate_receipt_identity_is_rejected() {
    let mut candidate = option("duplicate-receipt", &["GNSS"]);
    candidate
        .evidence_receipt_ids
        .push(candidate.evidence_receipt_ids[0].clone());
    assert_eq!(
        evaluate(&case(), &[candidate], &inventory(), &policy()),
        Err(EvaluationError::DuplicateReceiptId)
    );
}

#[test]
fn duplicate_dependency_domain_is_rejected() {
    let candidate = option("duplicate-domain", &["GNSS", "GNSS"]);
    assert_eq!(
        evaluate(&case(), &[candidate], &inventory(), &policy()),
        Err(EvaluationError::InvalidDependencySet)
    );
}

#[test]
fn malformed_evidence_binding_digest_is_rejected() {
    let mut candidate = option("bad-digest", &["GNSS"]);
    candidate.evidence_binding_sha256 = "not-a-digest".into();
    assert_eq!(
        evaluate(&case(), &[candidate], &inventory(), &policy()),
        Err(EvaluationError::InvalidOption)
    );
}

#[test]
fn unknown_dependency_inventory_never_counts_as_independent() {
    let mut candidate = option("dependency-unknown", &["GNSS"]);
    candidate.dependency_inventory_complete = false;
    let result = evaluate(
        &case(),
        &[option("known", &["C2"]), candidate],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
    assert_eq!(result.independent_option_witness.len(), 1);
}

#[test]
fn reauthorization_gap_is_reported_when_one_known_option_is_not_enough() {
    let mut second = option("needs-reauthorization", &["GNSS"]);
    second.authorization_state = AuthorizationState::ReauthorizationRequired;
    let result = evaluate(
        &case(),
        &[option("known", &["C2"]), second],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::ReauthorizationRequired);
}

#[test]
fn one_option_is_not_misreported_as_meeting_two_option_requirement() {
    let result = evaluate(
        &case(),
        &[option("single", &["C2"])],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::CoveredSingleOption);
    assert_eq!(result.required_independent_options, 2);
    assert!(!result.required_diversity_met);
    assert_eq!(
        result.independent_option_witness,
        vec!["single".to_string()]
    );
}

#[test]
fn empty_complete_dependency_inventory_is_rejected() {
    let mut candidate = option("empty-dependency-set", &["GNSS"]);
    candidate.dependency_domains.clear();
    assert_eq!(
        evaluate(&case(), &[candidate], &inventory(), &policy()),
        Err(EvaluationError::InvalidDependencySet)
    );
}

#[test]
fn inventory_effective_history_must_start_before_requested_cutoff() {
    let mut inv = inventory();
    inv.effective_from = 51;
    let result = evaluate(&case(), &[], &inv, &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
    assert!(!result.inventory_complete);
}

#[test]
fn inventory_knowledge_history_must_start_before_requested_cutoff() {
    let mut inv = inventory();
    inv.knowledge_from = 51;
    let result = evaluate(&case(), &[], &inv, &policy()).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
    assert!(!result.inventory_complete);
}

#[test]
fn known_diverse_witness_is_not_erased_by_an_unresolved_extra_option() {
    let mut unknown = option("unknown-extra", &["LANDING-SITE"]);
    unknown.evidence_state = EvidenceState::Unknown;
    let result = evaluate(
        &case(),
        &[
            option("return", &["C2"]),
            option("divert", &["GNSS"]),
            unknown,
        ],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(
        result.status,
        RecoverabilityStatus::CoveredWithRequiredDiversity
    );
    assert!(result.required_diversity_met);
}

#[test]
fn adding_failed_dependency_never_increases_admissible_option_count() {
    for index in 0..50 {
        let mut contingency = case();
        let domain = format!("FAILURE-{index}");
        let candidate = option("candidate", &[&domain]);
        let baseline = evaluate(
            &contingency,
            std::slice::from_ref(&candidate),
            &inventory(),
            &policy(),
        )
        .unwrap();
        contingency.failed_domains.push(domain);
        let after = evaluate(&contingency, &[candidate], &inventory(), &policy()).unwrap();
        assert_eq!(baseline.admissible_option_ids.len(), 1);
        assert!(after.admissible_option_ids.len() <= baseline.admissible_option_ids.len());
    }
}

#[test]
fn decreasing_containment_bound_never_improves_option_admissibility() {
    for step in 0..20 {
        let mut candidate = option("containment-monotone", &["GNSS"]);
        candidate.bounds.containment_lower = 0.95 - (step as f64 * 0.001);
        let result = evaluate(&case(), &[candidate], &inventory(), &policy()).unwrap();
        if step == 0 {
            assert_eq!(result.admissible_option_ids.len(), 1);
        } else {
            assert_eq!(result.admissible_option_ids.len(), 0);
        }
    }
}

#[test]
fn empty_dependency_inventory_cannot_manufacture_independence() {
    let candidate = option("empty-dependencies", &[]);
    assert_eq!(
        evaluate(&case(), &[candidate], &inventory(), &policy()),
        Err(EvaluationError::InvalidDependencySet)
    );
}

#[test]
fn exact_maximum_independent_set_finds_pair_missed_by_greedy_order() {
    let result = evaluate(
        &case(),
        &[
            option("a-blocker", &["X", "Y"]),
            option("b-left", &["X", "Z"]),
            option("c-right", &["Y", "W"]),
        ],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(
        result.status,
        RecoverabilityStatus::CoveredWithRequiredDiversity
    );
    assert!(result.independent_option_witness_exact);
    assert_eq!(
        result.independent_option_witness,
        vec!["b-left".to_string(), "c-right".to_string()]
    );
}

#[test]
fn options_with_only_exempt_common_dependencies_are_not_independent() {
    let result = evaluate(
        &case(),
        &[
            option("airframe-a", &["AIRFRAME"]),
            option("airframe-b", &["AIRFRAME"]),
        ],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert_eq!(result.status, RecoverabilityStatus::CoveredButCorrelated);
    assert!(result.independent_option_witness_exact);
    assert_eq!(result.independent_option_witness.len(), 1);
    assert!(!result.required_diversity_met);
}

#[test]
fn option_count_above_exact_limit_returns_non_exact_witness_and_abstains() {
    let mut frozen = policy();
    frozen.maximum_determinate_option_count = 2;
    let options = vec![
        option("option-a", &["SHARED-C2"]),
        option("option-b", &["SHARED-C2"]),
        option("option-c", &["SHARED-C2"]),
    ];
    let result = evaluate(&case(), &options, &inventory(), &frozen).unwrap();
    assert_eq!(result.status, RecoverabilityStatus::Indeterminate);
    assert!(!result.independent_option_witness_exact);
    assert_eq!(result.independent_option_witness.len(), 1);
    assert!(!result.required_diversity_met);
}

#[test]
fn exact_independent_set_ties_use_canonical_lexicographic_witness() {
    let result = evaluate(
        &case(),
        &[
            option("z-route", &["Z"]),
            option("a-route", &["A"]),
            option("m-route", &["M"]),
        ],
        &inventory(),
        &policy(),
    )
    .unwrap();
    assert!(result.independent_option_witness_exact);
    assert_eq!(
        result.independent_option_witness,
        vec!["a-route".to_string(), "m-route".to_string()]
    );
}

#[test]
fn determinate_option_limit_cannot_exceed_implementation_cap() {
    let mut frozen = policy();
    frozen.maximum_determinate_option_count = 25;
    assert_eq!(
        evaluate(&case(), &[], &inventory(), &frozen),
        Err(EvaluationError::InvalidPolicy)
    );
}
