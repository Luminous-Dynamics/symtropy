use aeroweift_assurance_composition_oracle::{
    compose, ClassOccupancy, CompositionInput, Decision, EvidenceIdentity, IdentityOwner,
    IntentDisposition, OccupancyInterpretation, PntDisposition, ReachabilityDisposition, Reason,
    SurveillanceDisposition, SurveillanceEvidence, TrafficClass,
};

const IDENTITY: EvidenceIdentity = EvidenceIdentity {
    time_scale_id: "SYNTHETIC-TIME-1",
    profile_id: "SYNTHETIC-PROFILE-1",
    frame_id: "SYNTHETIC-FRAME-1",
};

fn evidence(class: TrafficClass) -> SurveillanceEvidence {
    SurveillanceEvidence {
        traffic_class: class,
        disposition: SurveillanceDisposition::QualifiedTrack,
        coverage_qualification_ref: Some("synthetic-coverage-receipt"),
        observation_ref: Some("synthetic-observation-receipt"),
        dependency_snapshot_ref: Some("synthetic-dependency-snapshot"),
        independence_qualified: true,
        identity: IDENTITY,
    }
}

fn valid_input() -> CompositionInput {
    CompositionInput {
        expected_identity: IDENTITY,
        pnt_disposition: PntDisposition::AcceptedPrimaryHardBound,
        pnt_receipt_ref: Some("synthetic-pnt-receipt"),
        pnt_identity: IDENTITY,
        reachability_disposition: ReachabilityDisposition::QualifiedBounded,
        reachability_receipt_ref: Some("synthetic-reachability-receipt"),
        reachability_identity: IDENTITY,
        intent_disposition: IntentDisposition::Conforming,
        intent_receipt_ref: Some("synthetic-intent-receipt"),
        intent_identity: IDENTITY,
        required_traffic_classes: vec![
            TrafficClass::CooperativeCrewed,
            TrafficClass::NonCooperativeCrewed,
        ],
        surveillance: vec![
            evidence(TrafficClass::CooperativeCrewed),
            evidence(TrafficClass::NonCooperativeCrewed),
        ],
    }
}

#[test]
fn fully_qualified_positive_control_reaches_conflict_screen_only() {
    let result = compose(&valid_input());
    assert_eq!(result.decision, Decision::ReadyForConflictScreen);
    assert_eq!(result.reason, None);
    assert_eq!(result.class_occupancy.len(), 2);
    assert!(
        result
            .class_occupancy
            .iter()
            .all(|item| item.interpretation == OccupancyInterpretation::ObservedTrack)
    );
}

#[test]
fn qualified_no_report_remains_unresolved_possible_occupancy() {
    let mut input = valid_input();
    let non_coop = input
        .surveillance
        .iter_mut()
        .find(|item| item.traffic_class == TrafficClass::NonCooperativeCrewed)
        .expect("non-cooperative fixture exists");
    non_coop.disposition = SurveillanceDisposition::NoReportQualifiedCoverage;
    non_coop.observation_ref = None;

    let result = compose(&input);
    assert_eq!(result.decision, Decision::ReadyForConflictScreen);
    assert_eq!(
        result.class_occupancy[1].interpretation,
        OccupancyInterpretation::UnresolvedPossibleOccupancy
    );
}

#[test]
fn qualified_fallback_is_admissible_when_owner_has_accepted_it() {
    let mut input = valid_input();
    input.pnt_disposition = PntDisposition::AcceptedQualifiedFallback;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::ReadyForConflictScreen);
}

#[test]
fn stale_pnt_is_not_rescued_by_a_fresh_delivery_receipt() {
    let mut input = valid_input();
    input.pnt_disposition = PntDisposition::Stale;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::NotAssessable);
    assert_eq!(result.reason, Some(Reason::PntNotHardBound));
}

#[test]
fn statistical_only_pnt_cannot_be_promoted_to_a_hard_bound() {
    let mut input = valid_input();
    input.pnt_disposition = PntDisposition::StatisticalOnly;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::NotAssessable);
}

#[test]
fn pnt_time_scale_mismatch_blocks_composition() {
    let mut input = valid_input();
    input.pnt_identity.time_scale_id = "OTHER-TIME-SCALE";
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::IdentityMismatch(IdentityOwner::Pnt))
    );
}

#[test]
fn overlapping_or_unqualified_dual_clock_result_is_not_admissible() {
    let mut input = valid_input();
    input.pnt_disposition = PntDisposition::ClockUnqualified;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::NotAssessable);
}

#[test]
fn common_mode_receiver_multiplicity_is_not_recounted_here() {
    let mut input = valid_input();
    let non_coop = input
        .surveillance
        .iter_mut()
        .find(|item| item.traffic_class == TrafficClass::NonCooperativeCrewed)
        .expect("non-cooperative fixture exists");
    non_coop.independence_qualified = false;
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::IndependenceNotQualified(
            TrafficClass::NonCooperativeCrewed
        ))
    );
}

#[test]
fn cooperative_coverage_cannot_substitute_for_non_cooperative_coverage() {
    let mut input = valid_input();
    input
        .surveillance
        .retain(|item| item.traffic_class == TrafficClass::CooperativeCrewed);
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::MissingTrafficClass(
            TrafficClass::NonCooperativeCrewed
        ))
    );
}

#[test]
fn unknown_tdoa_clock_dependency_blocks_that_traffic_class() {
    let mut input = valid_input();
    let non_coop = input
        .surveillance
        .iter_mut()
        .find(|item| item.traffic_class == TrafficClass::NonCooperativeCrewed)
        .expect("non-cooperative fixture exists");
    non_coop.identity.time_scale_id = "UNKNOWN-TDOA-CLOCK";
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::IdentityMismatch(IdentityOwner::Surveillance))
    );
}

#[test]
fn stale_track_is_not_rescued_by_fresh_receipt_delivery() {
    let mut input = valid_input();
    let coop = input
        .surveillance
        .iter_mut()
        .find(|item| item.traffic_class == TrafficClass::CooperativeCrewed)
        .expect("cooperative fixture exists");
    coop.disposition = SurveillanceDisposition::UnqualifiedTrack;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::NotAssessable);
}

#[test]
fn missing_surveillance_qualification_reference_blocks_admission() {
    let mut input = valid_input();
    input.surveillance[0].coverage_qualification_ref = None;
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::MissingSurveillanceReceipt(
            TrafficClass::CooperativeCrewed
        ))
    );
}

#[test]
fn unqualified_reachability_cannot_enter_conflict_screening() {
    let mut input = valid_input();
    input.reachability_disposition = ReachabilityDisposition::NotAssessable;
    let result = compose(&input);
    assert_eq!(result.reason, Some(Reason::ReachabilityNotQualified));
}

#[test]
fn expired_or_superseded_authorization_requires_reauthorization() {
    let mut input = valid_input();
    input.intent_disposition = IntentDisposition::ReauthorizationRequired;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::ReauthorizationRequired);
}

#[test]
fn intersecting_dynamic_restriction_requires_reauthorization() {
    let mut input = valid_input();
    input.intent_disposition = IntentDisposition::ReauthorizationRequired;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::ReauthorizationRequired);
}

#[test]
fn non_intersecting_restriction_positive_control_remains_assessable() {
    let result = compose(&valid_input());
    assert_eq!(result.decision, Decision::ReadyForConflictScreen);
}

#[test]
fn definite_nonconformance_is_not_hidden_by_stale_pnt() {
    let mut input = valid_input();
    input.intent_disposition = IntentDisposition::Nonconforming;
    input.pnt_disposition = PntDisposition::Stale;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::Nonconforming);
}

#[test]
fn contingent_intent_is_a_handoff_not_a_positive_result() {
    let mut input = valid_input();
    input.intent_disposition = IntentDisposition::Contingent;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::ContingencyHandoff);
}

#[test]
fn potentially_nonconforming_intent_is_not_promoted_to_conforming() {
    let mut input = valid_input();
    input.intent_disposition = IntentDisposition::PotentiallyNonconforming;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::NotAssessable);
}

#[test]
fn missing_pnt_receipt_blocks_a_positive_status() {
    let mut input = valid_input();
    input.pnt_receipt_ref = None;
    let result = compose(&input);
    assert_eq!(result.reason, Some(Reason::MissingPntReceipt));
}

#[test]
fn missing_reachability_receipt_blocks_a_positive_status() {
    let mut input = valid_input();
    input.reachability_receipt_ref = None;
    let result = compose(&input);
    assert_eq!(result.reason, Some(Reason::MissingReachabilityReceipt));
}

#[test]
fn missing_intent_receipt_prevents_trusting_any_intent_state() {
    let mut input = valid_input();
    input.intent_receipt_ref = None;
    let result = compose(&input);
    assert_eq!(result.reason, Some(Reason::MissingIntentReceipt));
}

#[test]
fn frame_profile_or_datum_mismatch_blocks_surveillance_evidence() {
    let mut input = valid_input();
    input.surveillance[0].identity.frame_id = "OTHER-FRAME";
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::IdentityMismatch(IdentityOwner::Surveillance))
    );
}

#[test]
fn duplicate_class_evidence_is_not_silently_selected() {
    let mut input = valid_input();
    input
        .surveillance
        .push(evidence(TrafficClass::CooperativeCrewed));
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::DuplicateTrafficClass(
            TrafficClass::CooperativeCrewed
        ))
    );
}

#[test]
fn input_order_permutation_preserves_semantic_result_and_canonical_order() {
    let baseline = compose(&valid_input());
    let mut permuted = valid_input();
    permuted.required_traffic_classes.reverse();
    permuted.surveillance.reverse();
    let replay = compose(&permuted);

    assert_eq!(baseline, replay);
    assert_eq!(
        replay.class_occupancy,
        vec![
            ClassOccupancy {
                traffic_class: TrafficClass::CooperativeCrewed,
                interpretation: OccupancyInterpretation::ObservedTrack,
                coverage_qualification_ref: "synthetic-coverage-receipt",
                observation_ref: Some("synthetic-observation-receipt"),
                dependency_snapshot_ref: "synthetic-dependency-snapshot",
            },
            ClassOccupancy {
                traffic_class: TrafficClass::NonCooperativeCrewed,
                interpretation: OccupancyInterpretation::ObservedTrack,
                coverage_qualification_ref: "synthetic-coverage-receipt",
                observation_ref: Some("synthetic-observation-receipt"),
                dependency_snapshot_ref: "synthetic-dependency-snapshot",
            },
        ]
    );
}

#[test]
fn degrading_any_positive_pnt_state_cannot_improve_admissibility() {
    let baseline = compose(&valid_input());
    assert_eq!(baseline.decision, Decision::ReadyForConflictScreen);

    let mut degraded = valid_input();
    degraded.pnt_disposition = PntDisposition::Indeterminate;
    assert_eq!(compose(&degraded).decision, Decision::NotAssessable);
}

#[test]
fn empty_required_class_set_is_not_a_vacuously_valid_airspace_assessment() {
    let mut input = valid_input();
    input.required_traffic_classes.clear();
    let result = compose(&input);
    assert_eq!(result.reason, Some(Reason::NoRequiredTrafficClasses));
}
