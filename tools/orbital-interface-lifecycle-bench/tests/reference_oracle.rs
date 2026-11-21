use orbital_interface_lifecycle_bench::{Corpus, Disposition, LifecycleCase, ServiceKind};

fn independent_reference(case: &LifecycleCase) -> (Disposition, &'static str) {
    use Disposition::{
        Indeterminate, QualifiedWithinDeclaredEnvelope, Rejected, RequalificationRequired,
        Restricted, Withdrawn,
    };

    if case.pair_id.is_empty() || case.qualified_pair_id.is_empty() {
        return (Indeterminate, "pair_identity_missing");
    }
    if case.pair_id != case.qualified_pair_id {
        return (Rejected, "pair_identity_mismatch");
    }
    if case.service_kind != case.qualification_service_kind {
        return (Rejected, "service_kind_mismatch");
    }
    if case.provider_configuration.is_empty() || case.qualified_configuration.is_empty() {
        return (Indeterminate, "configuration_identity_missing");
    }
    if case.provider_configuration != case.qualified_configuration {
        return (RequalificationRequired, "configuration_changed_since_qualification");
    }
    if case.current_procedure_revision.is_empty()
        || case.qualified_procedure_revision.is_empty()
    {
        return (Indeterminate, "procedure_revision_missing");
    }
    if case.current_procedure_revision != case.qualified_procedure_revision {
        return (RequalificationRequired, "procedure_revision_changed");
    }
    if case.qualification_revoked {
        return (Withdrawn, "qualification_revoked");
    }
    if case.critical_defect_for_service {
        return (Withdrawn, "unresolved_critical_defect");
    }
    if case.wear_within_limit == Some(false) {
        return (Withdrawn, "wear_exceeds_declared_limit");
    }
    if case.environment_within_envelope == Some(false) {
        return (
            RequalificationRequired,
            "environment_exceeds_qualified_envelope",
        );
    }
    if case.maintenance_after_qualification {
        return (
            RequalificationRequired,
            "post_qualification_maintenance_requires_requalification",
        );
    }
    match case.qualification_valid_until_tick {
        None => return (Indeterminate, "qualification_validity_missing"),
        Some(until) if case.at_tick > until => {
            return (RequalificationRequired, "qualification_expired");
        }
        Some(_) => {}
    }
    match case.qualified_cycle_limit {
        None => return (Indeterminate, "qualified_cycle_limit_missing"),
        Some(limit) if case.cycle_count > limit => {
            return (
                RequalificationRequired,
                "qualified_cycle_limit_exceeded",
            );
        }
        Some(_) => {}
    }
    let age = match case.inspection_age_ticks {
        Some(age) => age,
        None => return (Indeterminate, "inspection_age_missing"),
    };
    let interval = match case.inspection_interval_ticks {
        Some(interval) => interval,
        None => return (Indeterminate, "inspection_interval_missing"),
    };
    if age > interval {
        return (RequalificationRequired, "inspection_interval_expired");
    }
    if case.calibration_valid != Some(true) {
        return (Indeterminate, "calibration_evidence_not_valid");
    }
    if case.wear_within_limit != Some(true) {
        return (Indeterminate, "wear_observation_missing");
    }
    if case.environment_within_envelope != Some(true) {
        return (Indeterminate, "environment_evidence_missing");
    }
    if case.uncertainty_clears_margin != Some(true) {
        return (Indeterminate, "measurement_margin_unproven");
    }
    if case.inspection_coverage_complete != Some(true) {
        return (Indeterminate, "inspection_coverage_incomplete");
    }
    if case.lineage_reconciled != Some(true) {
        return (Indeterminate, "lifecycle_lineage_unreconciled");
    }
    if case.raw_artifact_valid != Some(true) {
        return (Indeterminate, "raw_artifact_not_admitted");
    }
    if case.service_kind == ServiceKind::PreparedStorableFluid
        && !case.fluid_interface_observation_present
    {
        return (Indeterminate, "fluid_interface_observation_missing");
    }
    if case.restricted {
        return (Restricted, "only_restricted_envelope_is_qualified");
    }
    (
        QualifiedWithinDeclaredEnvelope,
        "all_lifecycle_gates_satisfied",
    )
}

#[test]
fn independent_oracle_matches_all_frozen_expected_dispositions() {
    let raw = include_str!("../fixtures/v0.1/cases.json");
    let corpus: Corpus = serde_json::from_str(raw).expect("fixture corpus parses");
    assert_eq!(corpus.cases.len(), 43);

    for case in &corpus.cases {
        let (status, reason) = independent_reference(case);
        assert_eq!(status, case.expected_status, "{}", case.case_id);
        assert_eq!(reason, case.expected_reason, "{}", case.case_id);
    }
}

#[test]
fn product_evaluator_matches_the_independent_oracle_on_the_frozen_corpus() {
    let raw = include_str!("../fixtures/v0.1/cases.json");
    let corpus: Corpus = serde_json::from_str(raw).expect("fixture corpus parses");
    let report = orbital_interface_lifecycle_bench::evaluate(&corpus)
        .expect("lifecycle corpus is valid");
    assert_eq!(report.result, "pass");
    assert_eq!(report.summary.case_count, 42);
    assert_eq!(report.summary.expected_result_mismatches, 0);
    assert!(!report.authority_granted);
    assert!(
        report.summary.restricted > 0
            && report.summary.requalification_required > 0
            && report.summary.withdrawn > 0
            && report.summary.rejected > 0
            && report.summary.indeterminate > 0
    );
}

#[test]
fn critical_withdrawal_cannot_be_hidden_by_a_restricted_flag() {
    let raw = include_str!("../fixtures/v0.1/cases.json");
    let corpus: Corpus = serde_json::from_str(raw).expect("fixture corpus parses");
    let report = orbital_interface_lifecycle_bench::evaluate(&corpus)
        .expect("lifecycle corpus is valid");
    let case = report
        .cases
        .iter()
        .find(|item| item.case_id == "case-42-critical-negative-beats-aggregate-restriction")
        .expect("critical negative fixture exists");
    assert_eq!(case.actual_status, Disposition::Withdrawn);
}
