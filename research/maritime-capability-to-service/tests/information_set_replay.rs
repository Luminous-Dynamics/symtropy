use std::collections::BTreeSet;

const MANIFEST: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/information-set-replay-v0.tsv"
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Truth {
    Yes,
    No,
    Indeterminate,
    NotApplicable,
}

impl Truth {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "YES" => Ok(Self::Yes),
            "NO" => Ok(Self::No),
            "INDETERMINATE" => Ok(Self::Indeterminate),
            "NOT_APPLICABLE" => Ok(Self::NotApplicable),
            other => Err(format!("unsupported truth state '{other}'")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RightsState {
    Granted,
    Denied,
}

impl RightsState {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "GRANTED" => Ok(Self::Granted),
            "DENIED" => Ok(Self::Denied),
            other => Err(format!("unsupported rights state '{other}'")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuthorityState {
    Valid,
    Revoked,
    Conflict,
}

impl AuthorityState {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "VALID" => Ok(Self::Valid),
            "REVOKED" => Ok(Self::Revoked),
            "CONFLICT" => Ok(Self::Conflict),
            other => Err(format!("unsupported authority state '{other}'")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvidenceState {
    Observed,
    Censored,
    Conflict,
    Duplicate,
}

impl EvidenceState {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "OBSERVED" => Ok(Self::Observed),
            "CENSORED" => Ok(Self::Censored),
            "CONFLICT" => Ok(Self::Conflict),
            "DUPLICATE" => Ok(Self::Duplicate),
            other => Err(format!("unsupported evidence state '{other}'")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClockQuality {
    Trusted,
    Uncertain,
}

impl ClockQuality {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "TRUSTED" => Ok(Self::Trusted),
            "UNCERTAIN" => Ok(Self::Uncertain),
            other => Err(format!("unsupported clock quality '{other}'")),
        }
    }
}

#[derive(Clone, Debug)]
struct ReplayCase {
    id: String,
    control: String,
    event_time: u64,
    source_observed_at: u64,
    first_available_at: u64,
    ingested_at: u64,
    validated_at: u64,
    valid_until: Option<u64>,
    rights: RightsState,
    authority_at_cutoff: AuthorityState,
    authority_current: AuthorityState,
    authority_current_available_at: Option<u64>,
    evidence_state: EvidenceState,
    clock_quality: ClockQuality,
    feature_uses_future_data: bool,
    action_requested: bool,
    decision_cutoff: u64,
    plan_published_at: Option<u64>,
    authorized_at: Option<u64>,
    service_deadline: Option<u64>,
    expected_visible: Truth,
    expected_admissible: Truth,
    expected_executable: Truth,
    expected_disposition: String,
}

fn parse_u64(value: &str, field: &str, case_id: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{case_id}: invalid {field} value '{value}'"))
}

fn parse_optional_u64(value: &str, field: &str, case_id: &str) -> Result<Option<u64>, String> {
    if value == "-" {
        Ok(None)
    } else {
        parse_u64(value, field, case_id).map(Some)
    }
}

fn parse_bool(value: &str, field: &str, case_id: &str) -> Result<bool, String> {
    match value {
        "YES" => Ok(true),
        "NO" => Ok(false),
        other => Err(format!("{case_id}: invalid {field} value '{other}'")),
    }
}

fn parse_manifest(input: &str) -> Result<Vec<ReplayCase>, String> {
    const HEADER: &str = "case_id\tcontrol\tevent_time_s\tsource_observed_at_s\tfirst_available_at_s\tingested_at_s\tvalidated_at_s\tvalid_until_s\trights_state\tauthority_state_at_cutoff\tauthority_state_current\tcurrent_authority_available_at_s\tevidence_state\tclock_quality\tfeature_uses_future_data\taction_requested\tdecision_cutoff_s\tplan_published_at_s\tauthorized_at_s\tservice_deadline_s\tvisible_at_cutoff\tevidence_admissible_at_cutoff\taction_executable_at_cutoff\texpected_disposition";
    let mut lines = input.lines().filter(|line| !line.trim().is_empty() && !line.starts_with('#'));
    if lines.next() != Some(HEADER) {
        return Err("KITE manifest header does not match v0.1 schema".to_owned());
    }

    let mut cases = Vec::new();
    let mut seen_ids = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 24 {
            return Err(format!(
                "manifest row {} has {} fields; expected 24",
                index + 2,
                fields.len()
            ));
        }
        let id = fields[0].to_owned();
        if id.is_empty() || !seen_ids.insert(id.clone()) {
            return Err(format!("manifest row {} has empty or duplicate case ID", index + 2));
        }

        let case = ReplayCase {
            id: id.clone(),
            control: fields[1].to_owned(),
            event_time: parse_u64(fields[2], "event time", &id)?,
            source_observed_at: parse_u64(fields[3], "source observation time", &id)?,
            first_available_at: parse_u64(fields[4], "first availability", &id)?,
            ingested_at: parse_u64(fields[5], "ingestion time", &id)?,
            validated_at: parse_u64(fields[6], "validation time", &id)?,
            valid_until: parse_optional_u64(fields[7], "valid-until time", &id)?,
            rights: RightsState::parse(fields[8])?,
            authority_at_cutoff: AuthorityState::parse(fields[9])?,
            authority_current: AuthorityState::parse(fields[10])?,
            authority_current_available_at: parse_optional_u64(
                fields[11],
                "current authority availability",
                &id,
            )?,
            evidence_state: EvidenceState::parse(fields[12])?,
            clock_quality: ClockQuality::parse(fields[13])?,
            feature_uses_future_data: parse_bool(fields[14], "future-feature flag", &id)?,
            action_requested: parse_bool(fields[15], "action-requested flag", &id)?,
            decision_cutoff: parse_u64(fields[16], "decision cutoff", &id)?,
            plan_published_at: parse_optional_u64(fields[17], "plan publication time", &id)?,
            authorized_at: parse_optional_u64(fields[18], "authorization time", &id)?,
            service_deadline: parse_optional_u64(fields[19], "service deadline", &id)?,
            expected_visible: Truth::parse(fields[20])?,
            expected_admissible: Truth::parse(fields[21])?,
            expected_executable: Truth::parse(fields[22])?,
            expected_disposition: fields[23].to_owned(),
        };

        if case.source_observed_at > case.first_available_at
            || case.first_available_at > case.ingested_at
            || case.ingested_at > case.validated_at
        {
            return Err(format!("{id}: evidence clock order is inconsistent"));
        }
        if case.authority_current != case.authority_at_cutoff {
            let known_at = case
                .authority_current_available_at
                .ok_or_else(|| format!("{id}: changed authority state lacks availability time"))?;
            if known_at <= case.decision_cutoff {
                return Err(format!("{id}: current authority change predates decision cutoff"));
            }
        }
        if !case.expected_disposition.is_empty() {
            cases.push(case);
        } else {
            return Err(format!("{id}: missing expected disposition"));
        }
    }
    Ok(cases)
}

fn visibility(case: &ReplayCase) -> Truth {
    if case.clock_quality == ClockQuality::Uncertain {
        Truth::Indeterminate
    } else if case.first_available_at <= case.decision_cutoff {
        Truth::Yes
    } else {
        Truth::No
    }
}

fn evidence_admissibility(case: &ReplayCase) -> Truth {
    match visibility(case) {
        Truth::No => return Truth::No,
        Truth::Indeterminate => return Truth::Indeterminate,
        Truth::Yes | Truth::NotApplicable => {}
    }

    if case.ingested_at > case.decision_cutoff
        || case.validated_at > case.decision_cutoff
        || case.valid_until.is_some_and(|expiry| expiry <= case.decision_cutoff)
        || case.rights == RightsState::Denied
        || case.authority_at_cutoff == AuthorityState::Revoked
        || case.feature_uses_future_data
    {
        return Truth::No;
    }
    if case.authority_at_cutoff == AuthorityState::Conflict
        || case.evidence_state == EvidenceState::Censored
        || case.evidence_state == EvidenceState::Conflict
    {
        return Truth::Indeterminate;
    }
    Truth::Yes
}

fn action_executability(case: &ReplayCase) -> Truth {
    if !case.action_requested {
        return Truth::NotApplicable;
    }
    if case.clock_quality == ClockQuality::Uncertain {
        return Truth::Indeterminate;
    }
    if evidence_admissibility(case) != Truth::Yes
        || case.rights != RightsState::Granted
        || case.authority_at_cutoff != AuthorityState::Valid
        || case.feature_uses_future_data
    {
        return Truth::No;
    }
    let (Some(published), Some(authorized), Some(deadline)) = (
        case.plan_published_at,
        case.authorized_at,
        case.service_deadline,
    ) else {
        return Truth::No;
    };
    if published > case.decision_cutoff
        || authorized > case.decision_cutoff
        || published > deadline
        || authorized < published
    {
        return Truth::No;
    }
    Truth::Yes
}

fn by_id(cases: &[ReplayCase], id: &str) -> &ReplayCase {
    cases
        .iter()
        .find(|case| case.id == id)
        .unwrap_or_else(|| panic!("missing KITE case {id}"))
}

#[test]
fn manifest_schema_and_evidence_clocks_are_well_formed() {
    let cases = parse_manifest(MANIFEST).expect("KITE fixture parses");
    assert_eq!(cases.len(), 20);
    assert_eq!(cases.iter().map(|case| case.id.as_str()).collect::<BTreeSet<_>>().len(), 20);
    for case in &cases {
        assert_eq!(visibility(case), case.expected_visible, "{}", case.id);
        assert_eq!(
            evidence_admissibility(case),
            case.expected_admissible,
            "{}: {}",
            case.id,
            case.control
        );
    }
}

#[test]
fn knowledge_visibility_uses_first_availability_not_event_time() {
    let cases = parse_manifest(MANIFEST).expect("KITE fixture parses");
    let backfilled = by_id(&cases, "KITE-005");
    assert!(backfilled.event_time < backfilled.decision_cutoff);
    assert!(backfilled.first_available_at > backfilled.decision_cutoff);
    assert_eq!(visibility(backfilled), Truth::No);

    let mutated_event_time = by_id(&cases, "KITE-008");
    assert!(mutated_event_time.event_time < mutated_event_time.decision_cutoff);
    assert_eq!(visibility(mutated_event_time), Truth::No);

    let late_ingestion = by_id(&cases, "KITE-003");
    assert_eq!(visibility(late_ingestion), Truth::Yes);
    assert!(late_ingestion.ingested_at > late_ingestion.decision_cutoff);
    assert_eq!(evidence_admissibility(late_ingestion), Truth::No);

    let future_feature = by_id(&cases, "KITE-018");
    assert!(future_feature.event_time > future_feature.decision_cutoff);
    assert_eq!(visibility(future_feature), Truth::Yes);
    assert_eq!(evidence_admissibility(future_feature), Truth::No);
}

#[test]
fn evidence_admission_fails_closed_on_rights_expiry_authority_and_future_features() {
    let cases = parse_manifest(MANIFEST).expect("KITE fixture parses");
    for id in ["KITE-009", "KITE-011", "KITE-012", "KITE-018"] {
        assert_eq!(evidence_admissibility(by_id(&cases, id)), Truth::No, "{id}");
    }
    assert_eq!(evidence_admissibility(by_id(&cases, "KITE-013")), Truth::Indeterminate);
    assert_eq!(evidence_admissibility(by_id(&cases, "KITE-014")), Truth::Indeterminate);
    assert_eq!(evidence_admissibility(by_id(&cases, "KITE-017")), Truth::Yes);
}

#[test]
fn late_revocation_changes_current_state_without_rewriting_historical_admission() {
    let cases = parse_manifest(MANIFEST).expect("KITE fixture parses");
    let late_revocation = by_id(&cases, "KITE-010");
    assert_eq!(late_revocation.authority_at_cutoff, AuthorityState::Valid);
    assert_eq!(late_revocation.authority_current, AuthorityState::Revoked);
    assert_eq!(late_revocation.authority_current_available_at, Some(106));
    assert!(late_revocation.authority_current_available_at.unwrap() > late_revocation.decision_cutoff);
    assert_eq!(evidence_admissibility(late_revocation), Truth::Yes);
    assert_eq!(action_executability(late_revocation), Truth::Yes);

    let known_revocation = by_id(&cases, "KITE-009");
    assert_eq!(known_revocation.authority_at_cutoff, AuthorityState::Revoked);
    assert_eq!(action_executability(known_revocation), Truth::No);
}

#[test]
fn a_plan_is_not_executable_before_publication_authorization_and_service_deadline() {
    let cases = parse_manifest(MANIFEST).expect("KITE fixture parses");
    for id in ["KITE-001", "KITE-015", "KITE-016", "KITE-009", "KITE-011", "KITE-012"] {
        assert_eq!(
            action_executability(by_id(&cases, id)),
            by_id(&cases, id).expected_executable,
            "{id}"
        );
    }
    assert_eq!(action_executability(by_id(&cases, "KITE-001")), Truth::Yes);
    assert_eq!(action_executability(by_id(&cases, "KITE-015")), Truth::No);
    assert_eq!(action_executability(by_id(&cases, "KITE-016")), Truth::No);
}

#[test]
fn censored_conflicting_and_duplicate_evidence_remain_distinct_states() {
    let cases = parse_manifest(MANIFEST).expect("KITE fixture parses");
    assert_eq!(by_id(&cases, "KITE-013").evidence_state, EvidenceState::Conflict);
    assert_eq!(by_id(&cases, "KITE-014").evidence_state, EvidenceState::Censored);
    assert_eq!(by_id(&cases, "KITE-017").evidence_state, EvidenceState::Duplicate);
    assert_eq!(action_executability(by_id(&cases, "KITE-019")), Truth::Indeterminate);
    assert_eq!(by_id(&cases, "KITE-014").expected_disposition, "UNKNOWN_NOT_ZERO_OR_SUCCESS");
}

#[test]
fn recorded_action_dispositions_match_recomputed_policy_gates() {
    let cases = parse_manifest(MANIFEST).expect("KITE fixture parses");
    for case in &cases {
        assert_eq!(
            action_executability(case),
            case.expected_executable,
            "{}: {}",
            case.id,
            case.control
        );
    }
}
