use maritime_capability_to_service::kite::{
    assess_action, select_as_of, ActionCandidate, AuthorityState, ClockQuality, EvidenceState,
    EvidenceVintage, Gate, RightsState, VintageDisposition,
};

fn evidence(
    logical_id: &str,
    vintage_id: &str,
    revision: u64,
    supersedes: Option<&str>,
    available_at: u64,
    authority: AuthorityState,
) -> EvidenceVintage {
    EvidenceVintage {
        logical_id: logical_id.to_owned(),
        vintage_id: vintage_id.to_owned(),
        revision,
        supersedes_vintage_id: supersedes.map(str::to_owned),
        event_time: 40,
        source_observed_at: available_at - 1,
        first_available_at: available_at,
        ingested_at: available_at + 1,
        validated_at: available_at + 2,
        valid_until: None,
        rights: RightsState::Granted,
        authority,
        evidence_state: EvidenceState::Observed,
        clock_quality: ClockQuality::Trusted,
        feature_uses_future_data: false,
    }
}

#[test]
fn public_api_replays_a_valid_historical_evidence_vintage() {
    let v1 = evidence("FEEDER", "FEEDER-V1", 1, None, 60, AuthorityState::Valid);
    let v2 = evidence(
        "FEEDER",
        "FEEDER-V2",
        2,
        Some("FEEDER-V1"),
        106,
        AuthorityState::Revoked,
    );
    let snapshot = select_as_of(&[v1, v2], 100).expect("as-of snapshot");

    assert_eq!(snapshot.selected["FEEDER"].vintage_id, "FEEDER-V1");
    assert_eq!(snapshot.selected["FEEDER"].admissible, Gate::Yes);
    assert_eq!(snapshot.withheld[0].disposition, VintageDisposition::NotYetAvailable);

    let action = ActionCandidate {
        action_id: "RECOVER-FEEDER".to_owned(),
        required_evidence: vec!["FEEDER".to_owned()],
        plan_published_at: Some(90),
        authorized_at: Some(95),
        action_started_at: Some(102),
        service_deadline: Some(120),
    };
    assert_eq!(
        assess_action(&snapshot, &action)
            .expect("historical action assessment")
            .executable_at_cutoff,
        Gate::Yes
    );

    let current = select_as_of(&[vintage_for_current()], 120).expect("current snapshot");
    assert_eq!(current.selected["FEEDER"].admissible, Gate::No);
}

fn vintage_for_current() -> EvidenceVintage {
    evidence(
        "FEEDER",
        "FEEDER-V1",
        1,
        None,
        60,
        AuthorityState::Revoked,
    )
}

#[test]
fn missing_censored_and_duplicate_prerequisites_never_make_an_action_executable() {
    let mut censored = evidence("CENSORED", "CENSORED-V1", 1, None, 60, AuthorityState::Valid);
    censored.evidence_state = EvidenceState::Censored;
    let mut duplicate = evidence("DUPLICATE", "DUPLICATE-V1", 1, None, 60, AuthorityState::Valid);
    duplicate.evidence_state = EvidenceState::Duplicate;

    let snapshot = select_as_of(&[censored, duplicate], 100).expect("as-of snapshot");
    let base = ActionCandidate {
        action_id: "ACTION".to_owned(),
        required_evidence: vec!["CENSORED".to_owned()],
        plan_published_at: Some(90),
        authorized_at: Some(95),
        action_started_at: None,
        service_deadline: Some(120),
    };

    assert_eq!(
        assess_action(&snapshot, &base)
            .expect("censored action")
            .executable_at_cutoff,
        Gate::Indeterminate
    );

    let mut duplicate_action = base.clone();
    duplicate_action.required_evidence = vec!["DUPLICATE".to_owned()];
    assert_eq!(
        assess_action(&snapshot, &duplicate_action)
            .expect("duplicate action")
            .executable_at_cutoff,
        Gate::No
    );

    let mut missing_action = base;
    missing_action.required_evidence = vec!["NOT-PRESENT".to_owned()];
    assert_eq!(
        assess_action(&snapshot, &missing_action)
            .expect("missing action")
            .executable_at_cutoff,
        Gate::No
    );
}
