use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Component {
    EarthOrbitChain,
    OrbitalCapture,
    OrbitalFluid,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    LaunchAttemptSucceeded,
    ApproachObserved,
    PayloadDeliveryAccepted,
    CaptureHandoffAccepted,
    FluidReceiverAccepted,
    CustomerAcknowledged,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    AcceptedForSyntheticComposition,
    Rejected,
    Indeterminate,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub component: Component,
    pub receipt_schema: String,
    pub source_commit: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub component: Component,
    pub receipt_schema: String,
    pub source_commit: String,
    pub reference: String,
    /// SHA-256 claim for the referenced source artifact; bytes are not fetched here.
    pub artifact_sha256: String,
    pub producer_id: String,
    pub authority_scope: String,
    pub context_of_use: String,
    pub claim_ceiling: String,
    pub decision: String,
    pub evidence_class: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub event_id: String,
    pub sequence: u64,
    pub kind: EventKind,
    #[serde(default)]
    pub subject_id: Option<String>,
    #[serde(default)]
    pub target_orbit_id: Option<String>,
    #[serde(default)]
    pub receiver_id: Option<String>,
    #[serde(default)]
    pub interface_id: Option<String>,
    #[serde(default)]
    pub interface_revision: Option<u64>,
    #[serde(default)]
    pub fluid_id: Option<String>,
    #[serde(default)]
    pub accepted_mass_g: Option<u64>,
    #[serde(default)]
    pub evidence: Option<EvidenceRef>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProfile {
    pub subject_id: String,
    pub target_orbit_id: String,
    pub interface_id: String,
    pub interface_revision: u64,
    pub fluid_id: String,
    pub minimum_accepted_mass_g: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub expected_status: GateStatus,
    pub profile: ServiceProfile,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub suite_id: String,
    pub suite_version: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub source_pins: Vec<SourcePin>,
    pub cases: Vec<Case>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub expected_status: GateStatus,
    pub actual_status: GateStatus,
    pub reason_codes: Vec<String>,
    pub required_stage_count: u64,
    pub observed_stage_count: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub case_count: u64,
    pub accepted_for_synthetic_composition: u64,
    pub rejected: u64,
    pub indeterminate: u64,
    pub expected_result_mismatches: u64,
    pub operational_authority_grants: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Report {
    pub suite_id: String,
    pub suite_version: String,
    pub evidence_class: String,
    pub result: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
    pub claim_ceiling: String,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_source_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn expected_component(kind: EventKind) -> Option<(Component, &'static str, &'static str)> {
    match kind {
        EventKind::PayloadDeliveryAccepted => Some((
            Component::EarthOrbitChain,
            "luminous.earth-orbit-chain.receipt.v1",
            "ready",
        )),
        EventKind::CaptureHandoffAccepted => Some((
            Component::OrbitalCapture,
            "luminous.orbital-capture-handoff.receipt.v0.1",
            "accepted_for_handoff",
        )),
        EventKind::FluidReceiverAccepted => Some((
            Component::OrbitalFluid,
            "luminous.orbital-fluid-transfer.receipt.v0.1",
            "transfer_accepted",
        )),
        EventKind::LaunchAttemptSucceeded
        | EventKind::ApproachObserved
        | EventKind::CustomerAcknowledged => None,
    }
}

fn push_missing(reasons: &mut Vec<String>, code: &str) {
    reasons.push(format!("missing_{code}"));
}

fn push_violation(reasons: &mut Vec<String>, code: &str) {
    reasons.push(format!("limit_{code}"));
}

fn validate_evidence(
    event: &Event,
    kind: EventKind,
    pins: &BTreeMap<Component, &SourcePin>,
    reasons: &mut Vec<String>,
) {
    let Some((component, schema, decision)) = expected_component(kind) else {
        return;
    };
    let Some(evidence) = event.evidence.as_ref() else {
        push_missing(reasons, "component_receipt");
        return;
    };
    if evidence.component != component {
        push_violation(reasons, "component_identity_mismatch");
    }
    if evidence.receipt_schema != schema {
        push_violation(reasons, "receipt_schema_mismatch");
    }
    if !non_empty(&evidence.reference) {
        push_missing(reasons, "receipt_reference");
    }
    if !non_empty(&evidence.artifact_sha256) {
        push_missing(reasons, "artifact_digest");
    } else if !is_sha256(&evidence.artifact_sha256) {
        push_violation(reasons, "artifact_digest_malformed");
    }
    if !non_empty(&evidence.producer_id) {
        push_missing(reasons, "producer_identity");
    }
    if !non_empty(&evidence.authority_scope) {
        push_missing(reasons, "authority_scope");
    }
    if !non_empty(&evidence.context_of_use) {
        push_missing(reasons, "context_of_use");
    }
    if !non_empty(&evidence.claim_ceiling) {
        push_missing(reasons, "claim_ceiling");
    }
    if evidence.evidence_class != "synthetic" {
        push_violation(reasons, "non_synthetic_evidence_in_v0_1");
    }
    match pins.get(&component) {
        Some(pin) => {
            if evidence.receipt_schema != pin.receipt_schema
                || evidence.source_commit != pin.source_commit
            {
                push_violation(reasons, "source_pin_mismatch");
            }
        }
        None => push_missing(reasons, "source_pin"),
    }
    if !is_source_commit(&evidence.source_commit) {
        push_violation(reasons, "source_commit_malformed");
    }
    if evidence.decision == "storage_only_not_transfer_evidence"
        && kind == EventKind::FluidReceiverAccepted
    {
        push_missing(reasons, "fluid_transfer_demonstration");
    } else if evidence.decision != decision {
        push_violation(reasons, "component_decision_mismatch");
    }
}

pub fn evaluate_case(case: &Case, pins: &BTreeMap<Component, &SourcePin>) -> CaseResult {
    let mut reasons = Vec::new();
    let profile = &case.profile;
    if !non_empty(&case.case_id)
        || !non_empty(&profile.subject_id)
        || !non_empty(&profile.target_orbit_id)
        || !non_empty(&profile.interface_id)
        || !non_empty(&profile.fluid_id)
        || profile.interface_revision == 0
        || profile.minimum_accepted_mass_g == 0
    {
        push_violation(&mut reasons, "profile_invalid_or_incomplete");
    }

    let mut event_ids = BTreeSet::new();
    let mut previous_sequence = None;
    let mut previous_stage = 0_u8;
    let mut stages = BTreeMap::<u8, &Event>::new();
    let mut observed_stage_count = 0_u64;

    for event in &case.events {
        if !non_empty(&event.event_id) || !event_ids.insert(event.event_id.as_str()) {
            push_violation(&mut reasons, "event_id_empty_or_duplicate");
        }
        if previous_sequence.is_some_and(|previous| event.sequence <= previous) {
            push_violation(&mut reasons, "event_sequence_not_strictly_increasing");
        }
        previous_sequence = Some(event.sequence);

        let stage = match event.kind {
            EventKind::PayloadDeliveryAccepted => Some(1_u8),
            EventKind::CaptureHandoffAccepted => Some(2_u8),
            EventKind::FluidReceiverAccepted => Some(3_u8),
            EventKind::LaunchAttemptSucceeded
            | EventKind::ApproachObserved
            | EventKind::CustomerAcknowledged => None,
        };
        if let Some(stage) = stage {
            observed_stage_count += 1;
            if stage <= previous_stage {
                push_violation(&mut reasons, "service_stage_order_or_duplicate");
            }
            previous_stage = stage;
            if stages.insert(stage, event).is_some() {
                push_violation(&mut reasons, "duplicate_service_stage");
            }
            validate_evidence(event, event.kind, pins, &mut reasons);
        }
    }

    let required_stage_count = 3_u64;
    for stage in 1..=3 {
        if !stages.contains_key(&stage) {
            push_missing(&mut reasons, match stage {
                1 => "payload_delivery",
                2 => "capture_handoff",
                _ => "fluid_receiver_acceptance",
            });
        }
    }

    if let Some(event) = stages.get(&1) {
        match event.subject_id.as_deref() {
            Some(value) if value == profile.subject_id => {}
            Some(_) => push_violation(&mut reasons, "payload_subject_mismatch"),
            None => push_missing(&mut reasons, "payload_subject_identity"),
        }
        match event.target_orbit_id.as_deref() {
            Some(value) if value == profile.target_orbit_id => {}
            Some(_) => push_violation(&mut reasons, "target_orbit_mismatch"),
            None => push_missing(&mut reasons, "target_orbit_acceptance"),
        }
    }

    if let Some(event) = stages.get(&2) {
        match event.subject_id.as_deref() {
            Some(value) if value == profile.subject_id => {}
            Some(_) => push_violation(&mut reasons, "capture_subject_mismatch"),
            None => push_missing(&mut reasons, "capture_subject_identity"),
        }
        match event.interface_id.as_deref() {
            Some(value) if value == profile.interface_id => {}
            Some(_) => push_violation(&mut reasons, "capture_interface_mismatch"),
            None => push_missing(&mut reasons, "capture_interface_identity"),
        }
        match event.interface_revision {
            Some(value) if value == profile.interface_revision => {}
            Some(_) => push_violation(&mut reasons, "capture_interface_revision_mismatch"),
            None => push_missing(&mut reasons, "capture_interface_revision"),
        }
    }

    if let Some(event) = stages.get(&3) {
        match event.subject_id.as_deref() {
            Some(value) if value == profile.subject_id => {}
            Some(_) => push_violation(&mut reasons, "fluid_receiver_subject_mismatch"),
            None => push_missing(&mut reasons, "fluid_receiver_identity"),
        }
        match event.receiver_id.as_deref() {
            Some(value) if value == profile.subject_id => {}
            Some(_) => push_violation(&mut reasons, "receiver_id_mismatch"),
            None => push_missing(&mut reasons, "receiver_id"),
        }
        match event.interface_id.as_deref() {
            Some(value) if value == profile.interface_id => {}
            Some(_) => push_violation(&mut reasons, "fluid_interface_mismatch"),
            None => push_missing(&mut reasons, "fluid_interface_identity"),
        }
        match event.interface_revision {
            Some(value) if value == profile.interface_revision => {}
            Some(_) => push_violation(&mut reasons, "fluid_interface_revision_mismatch"),
            None => push_missing(&mut reasons, "fluid_interface_revision"),
        }
        match event.fluid_id.as_deref() {
            Some(value) if value == profile.fluid_id => {}
            Some(_) => push_violation(&mut reasons, "fluid_identity_mismatch"),
            None => push_missing(&mut reasons, "fluid_identity"),
        }
        let storage_only = event
            .evidence
            .as_ref()
            .is_some_and(|evidence| evidence.decision == "storage_only_not_transfer_evidence");
        if !storage_only {
            match event.accepted_mass_g {
                Some(value) if value >= profile.minimum_accepted_mass_g => {}
                Some(_) => push_violation(&mut reasons, "accepted_mass_below_requirement"),
                None => push_missing(&mut reasons, "receiver_accepted_mass"),
            }
        }
    }

    reasons.sort();
    reasons.dedup();
    let has_violation = reasons.iter().any(|reason| reason.starts_with("limit_"));
    let has_missing = reasons.iter().any(|reason| reason.starts_with("missing_"));
    let actual_status = if has_violation {
        GateStatus::Rejected
    } else if has_missing {
        GateStatus::Indeterminate
    } else {
        GateStatus::AcceptedForSyntheticComposition
    };

    CaseResult {
        case_id: case.case_id.clone(),
        expected_status: case.expected_status,
        actual_status,
        reason_codes: reasons,
        required_stage_count,
        observed_stage_count,
    }
}

pub fn evaluate(fixture: &Fixture) -> Result<Report, String> {
    if fixture.suite_id != "orbital-service-composition"
        || fixture.suite_version != "0.1.0"
        || fixture.evidence_class != "synthetic"
        || !non_empty(&fixture.claim_ceiling)
    {
        return Err("unsupported suite identity, version, evidence class or claim ceiling".to_owned());
    }
    if fixture.cases.is_empty() {
        return Err("fixture must contain cases".to_owned());
    }
    let mut pins = BTreeMap::new();
    for pin in &fixture.source_pins {
        if !is_source_commit(&pin.source_commit)
            || !non_empty(&pin.receipt_schema)
            || pins.insert(pin.component, pin).is_some()
        {
            return Err("source pins must be unique, complete and use full commit SHAs".to_owned());
        }
    }
    if pins.len() != 3 {
        return Err("v0.1 requires launch, capture and fluid source pins".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(fixture.cases.len());
    let mut mismatches = 0_u64;
    for case in &fixture.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("case IDs must be unique".to_owned());
        }
        let result = evaluate_case(case, &pins);
        if result.expected_status != result.actual_status {
            mismatches += 1;
        }
        results.push(result);
    }

    let accepted = results.iter().filter(|item| {
        item.actual_status == GateStatus::AcceptedForSyntheticComposition
    }).count() as u64;
    let rejected = results.iter().filter(|item| item.actual_status == GateStatus::Rejected).count() as u64;
    let indeterminate = results.iter().filter(|item| item.actual_status == GateStatus::Indeterminate).count() as u64;
    let result = if mismatches == 0 { "pass" } else { "fail" };

    Ok(Report {
        suite_id: fixture.suite_id.clone(),
        suite_version: fixture.suite_version.clone(),
        evidence_class: fixture.evidence_class.clone(),
        result: result.to_owned(),
        summary: Summary {
            case_count: results.len() as u64,
            accepted_for_synthetic_composition: accepted,
            rejected,
            indeterminate,
            expected_result_mismatches: mismatches,
            operational_authority_grants: 0,
        },
        cases: results,
        claim_ceiling: fixture.claim_ceiling.clone(),
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String is infallible");
    }
    output
}

pub fn is_valid_source_commit(value: &str) -> bool {
    is_source_commit(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../fixtures/v0.1/cases.json"))
            .expect("frozen fixture must parse")
    }

    fn pins(data: &Fixture) -> BTreeMap<Component, &SourcePin> {
        data.source_pins.iter().map(|pin| (pin.component, pin)).collect()
    }

    #[test]
    fn frozen_corpus_matches_every_expected_disposition() {
        let data = fixture();
        let report = evaluate(&data).expect("fixture should evaluate");
        assert_eq!(report.result, "pass");
        assert_eq!(report.summary.case_count, 14);
        assert_eq!(report.summary.accepted_for_synthetic_composition, 1);
        assert_eq!(report.summary.rejected, 7);
        assert_eq!(report.summary.indeterminate, 6);
        assert_eq!(report.summary.expected_result_mismatches, 0);
        assert_eq!(report.summary.operational_authority_grants, 0);
    }

    #[test]
    fn nominal_chain_requires_delivery_capture_and_receiver_acceptance() {
        let data = fixture();
        let result = evaluate_case(&data.cases[0], &pins(&data));
        assert_eq!(result.actual_status, GateStatus::AcceptedForSyntheticComposition);
        assert_eq!(result.observed_stage_count, 3);
    }

    #[test]
    fn a_launch_attempt_is_not_payload_delivery() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "LAUNCH_ATTEMPT_ONLY").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn approach_observation_is_not_mechanical_capture() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "APPROACH_ONLY").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn wrong_payload_identity_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "CAPTURE_WRONG_SUBJECT").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn interface_revision_mismatch_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "INTERFACE_REVISION_MISMATCH").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn storage_only_is_not_receiver_accepted_transfer() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "STORAGE_ONLY_NOT_TRANSFER_EVIDENCE").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn receiver_mass_shortfall_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "RECEIVER_MASS_SHORTFALL").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn stale_component_commit_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "STALE_SOURCE_COMMIT").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn duplicate_event_identity_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "DUPLICATE_EVENT_ID").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn customer_acknowledgement_alone_is_not_technical_acceptance() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "CUSTOMER_ACK_ONLY").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn out_of_order_service_stages_are_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "OUT_OF_ORDER").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn missing_fluid_evidence_is_indeterminate() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "MISSING_FLUID_EVIDENCE").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn missing_receiver_identity_is_indeterminate() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[2].receiver_id = None;
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn wrong_receiver_identity_is_rejected() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[2].receiver_id = Some("unrelated-receiver".to_owned());
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn wrong_target_orbit_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "WRONG_TARGET_ORBIT").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn source_commit_digest_shape_is_checked() {
        assert!(is_valid_source_commit("0123456789abcdef0123456789abcdef01234567"));
        assert!(!is_valid_source_commit("main"));
    }

    #[test]
    fn missing_artifact_digest_is_indeterminate() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[0].evidence.as_mut().unwrap().artifact_sha256.clear();
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn missing_producer_or_authority_scope_is_indeterminate() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        let evidence = case.events[1].evidence.as_mut().unwrap();
        evidence.producer_id.clear();
        evidence.authority_scope.clear();
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn malformed_artifact_digest_is_rejected() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[2].evidence.as_mut().unwrap().artifact_sha256 = "not-a-sha256".to_owned();
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn evidence_component_mismatch_is_rejected() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[1].evidence.as_mut().unwrap().component = Component::EarthOrbitChain;
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Rejected);
    }
}
