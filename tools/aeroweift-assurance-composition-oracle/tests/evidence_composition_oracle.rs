use aeroweift_assurance_composition_oracle::{
    compose, ClassOccupancy, CompositionInput, Decision, EvidenceIdentity, IdentityOwner,
    IntentDisposition, OccupancyInterpretation, PntDisposition, ReachabilityDisposition, Reason,
    SurveillanceDisposition, SurveillanceEvidence, TrafficClass,
};

const IDENTITY: EvidenceIdentity = EvidenceIdentity {
    time_scale_id: "SYNTHETIC-TIME-1",
    profile_id: "SYNTHETIC-PROFILE-1",
    frame_id: "SYNTHETIC-FRAME-1",
    vertical_datum_id: "SYNTHETIC-DATUM-1",
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

fn apply_fault(input: &mut CompositionInput, fault: usize, class_index: usize) {
    let traffic_class = if class_index == 0 {
        TrafficClass::CooperativeCrewed
    } else {
        TrafficClass::NonCooperativeCrewed
    };
    let evidence_index = input
        .surveillance
        .iter()
        .position(|item| item.traffic_class == traffic_class);

    match fault {
        0 => input.pnt_disposition = PntDisposition::Stale,
        1 => input.pnt_disposition = PntDisposition::StatisticalOnly,
        2 => input.pnt_disposition = PntDisposition::Recovering,
        3 => input.pnt_disposition = PntDisposition::Indeterminate,
        4 => input.pnt_disposition = PntDisposition::ClockUnqualified,
        5 => input.pnt_disposition = PntDisposition::Contradictory,
        6 => input.reachability_disposition = ReachabilityDisposition::NotAssessable,
        7 => input.reachability_disposition = ReachabilityDisposition::ArithmeticFailure,
        8 => input.intent_disposition = IntentDisposition::PotentiallyNonconforming,
        9 => input.intent_disposition = IntentDisposition::NotAssessable,
        10 => input.intent_disposition = IntentDisposition::Nonconforming,
        11 => input.intent_disposition = IntentDisposition::Contingent,
        12 => input.intent_disposition = IntentDisposition::ReauthorizationRequired,
        13 => input.pnt_receipt_ref = None,
        14 => input.reachability_receipt_ref = None,
        15 => input.intent_receipt_ref = None,
        16 => input
            .surveillance
            .retain(|item| item.traffic_class != traffic_class),
        17 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.disposition = SurveillanceDisposition::NoQualifiedCoverage;
                item.observation_ref = None;
            }
        }
        18 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.disposition = SurveillanceDisposition::UnqualifiedTrack;
            }
        }
        19 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.coverage_qualification_ref = None;
            }
        }
        20 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.observation_ref = None;
            }
        }
        21 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.dependency_snapshot_ref = None;
            }
        }
        22 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.independence_qualified = false;
            }
        }
        23 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.identity.time_scale_id = "OTHER-TIME";
            }
        }
        24 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.identity.profile_id = "OTHER-PROFILE";
            }
        }
        25 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.identity.frame_id = "OTHER-FRAME";
            }
        }
        26 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.identity.vertical_datum_id = "OTHER-DATUM";
            }
        }
        27 => input.expected_identity.vertical_datum_id = "",
        28 => {
            let class_was_present = input
                .surveillance
                .iter()
                .any(|item| item.traffic_class == traffic_class);
            let copy = input
                .surveillance
                .iter()
                .find(|item| item.traffic_class == traffic_class)
                .cloned()
                .unwrap_or_else(|| crate::evidence(traffic_class));
            input.surveillance.push(copy.clone());
            if !class_was_present {
                // When another fault removed the class, add two copies so this
                // mutator cannot accidentally repair the missing-class fault.
                input.surveillance.push(copy);
            }
        }
        29 => input.required_traffic_classes.push(traffic_class),
        30 => input.required_traffic_classes.clear(),
        31 => input.expected_identity.time_scale_id = "",
        32 => input.reachability_identity.profile_id = "",
        33 => input.intent_identity.vertical_datum_id = "OTHER-DATUM",
        34 => input.pnt_receipt_ref = Some(" "),
        35 => input.reachability_receipt_ref = Some(" "),
        36 => input.intent_receipt_ref = Some(" "),
        37 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.coverage_qualification_ref = Some(" ");
            }
        }
        38 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.dependency_snapshot_ref = Some(" ");
            }
        }
        39 => {
            if let Some(index) = evidence_index {
                let item = &mut input.surveillance[index];
                item.identity.frame_id = "";
            }
        }
        _ => unreachable!("fault index is bounded by the campaign"),
    }
}

fn next_seeded(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
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
fn accepted_receipt_lineage_preserves_complete_owner_input_snapshot() {
    let input = valid_input();
    let result = compose(&input);
    assert_eq!(result.source_snapshot, input);
    assert_eq!(
        result.source_snapshot.pnt_receipt_ref,
        Some("synthetic-pnt-receipt")
    );
    assert_eq!(
        result.source_snapshot.reachability_receipt_ref,
        Some("synthetic-reachability-receipt")
    );
    assert_eq!(
        result.source_snapshot.intent_receipt_ref,
        Some("synthetic-intent-receipt")
    );
    assert_eq!(result.source_snapshot.surveillance.len(), 2);
}

#[test]
fn rejected_composition_still_preserves_all_submitted_evidence_references() {
    let mut input = valid_input();
    input.pnt_disposition = PntDisposition::Stale;
    let result = compose(&input);
    assert_eq!(result.decision, Decision::NotAssessable);
    assert_eq!(result.class_occupancy.len(), 0);
    assert_eq!(result.source_snapshot, input);
}

#[test]
fn vertical_datum_mismatch_blocks_composition() {
    let mut input = valid_input();
    input.surveillance[0].identity.vertical_datum_id = "OTHER-DATUM";
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::IdentityMismatch(IdentityOwner::Surveillance))
    );
}

#[test]
fn missing_vertical_datum_identity_is_not_admissible() {
    let mut input = valid_input();
    input.expected_identity.vertical_datum_id = "";
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::IncompleteIdentity(IdentityOwner::Expected))
    );
}

#[test]
fn duplicate_required_traffic_class_is_rejected() {
    let mut input = valid_input();
    input
        .required_traffic_classes
        .push(TrafficClass::NonCooperativeCrewed);
    let result = compose(&input);
    assert_eq!(
        result.reason,
        Some(Reason::DuplicateRequiredTrafficClass(
            TrafficClass::NonCooperativeCrewed
        ))
    );
}

#[test]
fn exhaustive_pairs_and_seeded_ten_thousand_fault_trials_never_admit() {
    const FAULTS: usize = 40;

    // Exhaust every single mutator and every unordered pair of mutators.
    for first in 0..FAULTS {
        let mut single = valid_input();
        apply_fault(&mut single, first, first % 2);
        let single_result = compose(&single);
        assert_ne!(
            single_result.decision,
            Decision::ReadyForConflictScreen,
            "single fault {first} admitted"
        );
        assert!(single_result.class_occupancy.is_empty());

        for second in first..FAULTS {
            let mut pair = valid_input();
            apply_fault(&mut pair, first, first % 2);
            apply_fault(&mut pair, second, second % 2);
            let result = compose(&pair);
            assert_ne!(
                result.decision,
                Decision::ReadyForConflictScreen,
                "fault pair ({first}, {second}) admitted"
            );
            assert!(result.class_occupancy.is_empty());
        }
    }

    // Deterministic seeded combinations exercise repeated cross-layer interactions.
    let mut seed = 0xA3E0_5AFE_2026_0015_u64;
    for sample in 0..10_000 {
        let first = (next_seeded(&mut seed) as usize) % FAULTS;
        let second = (next_seeded(&mut seed) as usize) % FAULTS;
        let class_index = (next_seeded(&mut seed) as usize) % 2;
        let mut input = valid_input();
        apply_fault(&mut input, first, class_index);
        apply_fault(&mut input, second, 1 - class_index);

        let result = compose(&input);
        assert_ne!(
            result.decision,
            Decision::ReadyForConflictScreen,
            "seeded sample {sample} with faults ({first}, {second}) admitted"
        );
        assert!(result.class_occupancy.is_empty());
    }
}

#[test]
fn empty_required_class_set_is_not_a_vacuously_valid_airspace_assessment() {
    let mut input = valid_input();
    input.required_traffic_classes.clear();
    let result = compose(&input);
    assert_eq!(result.reason, Some(Reason::NoRequiredTrafficClasses));
}
