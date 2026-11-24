use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    QualifiedWithinDeclaredEnvelope,
    Incomplete,
    NotIdentifiable,
    NotEvaluable,
    Unknown,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QualifiedWithinDeclaredEnvelope => "QUALIFIED_WITHIN_DECLARED_ENVELOPE",
            Self::Incomplete => "INCOMPLETE",
            Self::NotIdentifiable => "NOT_IDENTIFIABLE",
            Self::NotEvaluable => "NOT_EVALUABLE",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureProfile {
    pub source_id: String,
    pub event_class: String,
    pub window_start: i64,
    pub window_end: i64,
    pub configuration_digest: String,
    pub expected_configuration_digest: String,
    pub sequence_semantics_qualified: bool,
    pub failure_domains: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceEvent {
    pub event_id: String,
    pub canonical_subject_id: String,
    pub event_class: String,
    pub event_time: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndependentReference {
    pub reference_id: String,
    pub event_class: String,
    pub window_start: i64,
    pub window_end: i64,
    pub coverage_complete: bool,
    pub independently_qualified: bool,
    pub failure_domains: BTreeSet<String>,
    pub events: Vec<ReferenceEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedEvent {
    pub event_id: String,
    /// Source-native row/record identity, not interchangeable with event_id.
    pub source_record_id: String,
    /// Canonical vessel/container/visit identity carried by the source row.
    pub canonical_subject_id: String,
    pub event_class: String,
    pub event_time: i64,
    pub published_at: i64,
    pub payload_digest: String,
    pub durably_persisted: bool,
    pub delivery_acknowledged: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureCase {
    pub profile: Option<CaptureProfile>,
    pub reference: Option<IndependentReference>,
    pub captured_events: Vec<CapturedEvent>,
    pub as_of: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureProvenance {
    pub source_id: String,
    pub event_class: String,
    pub configuration_digest: String,
    pub window_start: i64,
    pub window_end: i64,
    pub as_of: i64,
    pub reference_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciledCaptureEvent {
    pub source_id: String,
    pub reference_id: String,
    pub configuration_digest: String,
    pub event_id: String,
    pub source_record_id: String,
    pub canonical_subject_id: String,
    pub event_class: String,
    pub event_time: i64,
    pub published_at: i64,
    pub payload_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureReport {
    pub disposition: Disposition,
    pub expected_event_count: usize,
    pub reconciled_event_count: usize,
    pub missing_event_count: usize,
    pub non_durable_event_count: usize,
    pub unexpected_event_count: usize,
    pub conflicting_identity_count: usize,
    pub shared_failure_domains: Vec<String>,
    /// Available for qualified or incomplete reports with a known profile/reference.
    pub provenance: Option<CaptureProvenance>,
    /// Exact, durable event matches; a qualified window is still not enough
    /// unless a particular claim binds to one of these rows.
    pub reconciled_events: Vec<ReconciledCaptureEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureClaimBinding {
    pub source_id: String,
    pub event_class: String,
    pub configuration_digest: String,
    pub event_id: String,
    pub source_record_id: String,
    pub canonical_subject_id: String,
    pub event_time: i64,
    pub payload_digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureClaimAdmission {
    Qualified,
    Incomplete,
    NotIdentifiable,
    NotEvaluable,
    Unknown,
    EvidenceDoesNotMatchClaim,
}

impl CaptureClaimAdmission {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Qualified => "QUALIFIED_CAPTURE_BOUND_TO_CLAIM",
            Self::Incomplete => "CAPTURE_INCOMPLETE",
            Self::NotIdentifiable => "CAPTURE_NOT_IDENTIFIABLE",
            Self::NotEvaluable => "CAPTURE_NOT_EVALUABLE",
            Self::Unknown => "CAPTURE_UNKNOWN",
            Self::EvidenceDoesNotMatchClaim => "CAPTURE_EVIDENCE_DOES_NOT_MATCH_CLAIM",
        }
    }
}

impl CaptureReport {
    fn with_disposition(disposition: Disposition) -> Self {
        Self {
            disposition,
            expected_event_count: 0,
            reconciled_event_count: 0,
            missing_event_count: 0,
            non_durable_event_count: 0,
            unexpected_event_count: 0,
            conflicting_identity_count: 0,
            shared_failure_domains: Vec::new(),
            provenance: None,
            reconciled_events: Vec::new(),
        }
    }
}

fn valid_profile(profile: &CaptureProfile) -> bool {
    !profile.source_id.trim().is_empty()
        && !profile.event_class.trim().is_empty()
        && profile.window_start < profile.window_end
        && !profile.configuration_digest.trim().is_empty()
        && profile.configuration_digest == profile.expected_configuration_digest
        && profile.sequence_semantics_qualified
        && !profile.failure_domains.is_empty()
}

fn valid_reference(reference: &IndependentReference, profile: &CaptureProfile) -> bool {
    !reference.reference_id.trim().is_empty()
        && reference.event_class == profile.event_class
        && reference.window_start == profile.window_start
        && reference.window_end == profile.window_end
        && reference.window_start < reference.window_end
        && reference.coverage_complete
        && reference.independently_qualified
        && !reference.failure_domains.is_empty()
}

/// Binds a particular downstream source claim to a qualified capture receipt.
///
/// A qualified window alone is insufficient: the exact source, configuration,
/// event class, event ID, source-native record ID, canonical subject and event
/// time must match one durably reconciled event. A non-qualified window keeps
/// its stronger failure disposition and never admits an individual claim.
pub fn admit_claim_against_capture(
    report: &CaptureReport,
    claim: &CaptureClaimBinding,
) -> CaptureClaimAdmission {
    match report.disposition {
        Disposition::Incomplete => return CaptureClaimAdmission::Incomplete,
        Disposition::NotIdentifiable => return CaptureClaimAdmission::NotIdentifiable,
        Disposition::NotEvaluable => return CaptureClaimAdmission::NotEvaluable,
        Disposition::Unknown => return CaptureClaimAdmission::Unknown,
        Disposition::QualifiedWithinDeclaredEnvelope => {}
    }

    let Some(provenance) = report.provenance.as_ref() else {
        return CaptureClaimAdmission::NotEvaluable;
    };
    if provenance.source_id != claim.source_id
        || provenance.event_class != claim.event_class
        || provenance.configuration_digest != claim.configuration_digest
        || claim.event_time < provenance.window_start
        || claim.event_time >= provenance.window_end
        || claim.event_time > provenance.as_of
    {
        return CaptureClaimAdmission::EvidenceDoesNotMatchClaim;
    }

    let matched = report.reconciled_events.iter().any(|event| {
        event.source_id == claim.source_id
            && event.event_class == claim.event_class
            && event.configuration_digest == claim.configuration_digest
            && event.event_id == claim.event_id
            && event.source_record_id == claim.source_record_id
            && event.canonical_subject_id == claim.canonical_subject_id
            && event.event_time == claim.event_time
            && event.payload_digest == claim.payload_digest
            && event.reference_id == provenance.reference_id
            && event.published_at <= provenance.as_of
    });
    if matched {
        CaptureClaimAdmission::Qualified
    } else {
        CaptureClaimAdmission::EvidenceDoesNotMatchClaim
    }
}

pub fn evaluate(case: &CaptureCase) -> CaptureReport {
    let Some(profile) = case.profile.as_ref() else {
        return CaptureReport::with_disposition(Disposition::Unknown);
    };

    if !valid_profile(profile) {
        return CaptureReport::with_disposition(Disposition::NotEvaluable);
    }
    if case.as_of < profile.window_end {
        return CaptureReport::with_disposition(Disposition::NotEvaluable);
    }

    let Some(reference) = case.reference.as_ref() else {
        return CaptureReport::with_disposition(Disposition::NotIdentifiable);
    };

    if !valid_reference(reference, profile) {
        return CaptureReport::with_disposition(Disposition::NotIdentifiable);
    }

    let shared_failure_domains: Vec<String> = profile
        .failure_domains
        .intersection(&reference.failure_domains)
        .cloned()
        .collect();
    if !shared_failure_domains.is_empty() {
        let mut report = CaptureReport::with_disposition(Disposition::NotIdentifiable);
        report.shared_failure_domains = shared_failure_domains;
        return report;
    }

    let mut expected = BTreeMap::new();
    let mut seen_reference_ids = BTreeSet::new();
    for event in &reference.events {
        if event.event_id.trim().is_empty()
            || event.canonical_subject_id.trim().is_empty()
            || event.event_class != profile.event_class
            || event.event_time < profile.window_start
            || event.event_time >= profile.window_end
            || !seen_reference_ids.insert(event.event_id.clone())
        {
            return CaptureReport::with_disposition(Disposition::NotEvaluable);
        }
        if event.event_time <= case.as_of {
            expected.insert(event.event_id.clone(), event.clone());
        }
    }

    let mut visible: BTreeMap<String, CapturedEvent> = BTreeMap::new();
    let mut conflicting_identity_count = 0usize;
    for event in &case.captured_events {
        if event.event_class != profile.event_class
            || event.event_time < profile.window_start
            || event.event_time >= profile.window_end
        {
            continue;
        }
        if event.event_id.trim().is_empty()
            || event.source_record_id.trim().is_empty()
            || event.canonical_subject_id.trim().is_empty()
            || event.payload_digest.trim().is_empty()
        {
            return CaptureReport::with_disposition(Disposition::NotEvaluable);
        }
        if event.published_at < event.event_time {
            return CaptureReport::with_disposition(Disposition::NotEvaluable);
        }
        if event.event_time > case.as_of {
            if event.published_at <= case.as_of {
                return CaptureReport::with_disposition(Disposition::NotEvaluable);
            }
            continue;
        }
        if event.published_at > case.as_of {
            continue;
        }

        match visible.get(&event.event_id) {
            None => {
                visible.insert(event.event_id.clone(), event.clone());
            }
            Some(previous) if previous == event => {}
            Some(_) => conflicting_identity_count += 1,
        }
    }

    let mut report = CaptureReport::with_disposition(Disposition::QualifiedWithinDeclaredEnvelope);
    report.expected_event_count = expected.len();
    report.conflicting_identity_count = conflicting_identity_count;
    report.provenance = Some(CaptureProvenance {
        source_id: profile.source_id.clone(),
        event_class: profile.event_class.clone(),
        configuration_digest: profile.configuration_digest.clone(),
        window_start: profile.window_start,
        window_end: profile.window_end,
        as_of: case.as_of,
        reference_id: reference.reference_id.clone(),
    });

    for (event_id, expected_event) in &expected {
        match visible.get(event_id) {
            None => report.missing_event_count += 1,
            Some(record) if !record.durably_persisted => {
                report.non_durable_event_count += 1;
            }
            Some(record)
                if record.event_time != expected_event.event_time
                    || record.canonical_subject_id != expected_event.canonical_subject_id =>
            {
                report.conflicting_identity_count += 1;
            }
            Some(record) => {
                report.reconciled_event_count += 1;
                report.reconciled_events.push(ReconciledCaptureEvent {
                    source_id: profile.source_id.clone(),
                    reference_id: reference.reference_id.clone(),
                    configuration_digest: profile.configuration_digest.clone(),
                    event_id: record.event_id.clone(),
                    source_record_id: record.source_record_id.clone(),
                    canonical_subject_id: record.canonical_subject_id.clone(),
                    event_class: record.event_class.clone(),
                    event_time: record.event_time,
                    published_at: record.published_at,
                    payload_digest: record.payload_digest.clone(),
                });
            }
        }
    }

    report.unexpected_event_count = visible
        .keys()
        .filter(|event_id| !expected.contains_key(*event_id))
        .count();

    if report.missing_event_count > 0
        || report.non_durable_event_count > 0
        || report.unexpected_event_count > 0
        || report.conflicting_identity_count > 0
    {
        report.disposition = Disposition::Incomplete;
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn profile() -> CaptureProfile {
        CaptureProfile {
            source_id: "terminal-gate-1".to_owned(),
            event_class: "GATE_OUT".to_owned(),
            window_start: 0,
            window_end: 20,
            configuration_digest: "config-v1".to_owned(),
            expected_configuration_digest: "config-v1".to_owned(),
            sequence_semantics_qualified: true,
            failure_domains: set(&[
                "source-power",
                "source-clock",
                "source-reader",
                "source-network",
                "source-store",
            ]),
        }
    }

    fn reference(events: &[&str]) -> IndependentReference {
        IndependentReference {
            reference_id: "independent-counter-1".to_owned(),
            event_class: "GATE_OUT".to_owned(),
            window_start: 0,
            window_end: 20,
            coverage_complete: true,
            independently_qualified: true,
            failure_domains: set(&["reference-power", "reference-clock", "reference-sensor"]),
            events: events
                .iter()
                .map(|event_id| ReferenceEvent {
                    event_id: (*event_id).to_owned(),
                    canonical_subject_id: "urn:epc:id:sscc:0614141.1234567890".to_owned(),
                    event_class: "GATE_OUT".to_owned(),
                    event_time: 10,
                })
                .collect(),
        }
    }

    fn captured(event_id: &str) -> CapturedEvent {
        CapturedEvent {
            event_id: event_id.to_owned(),
            source_record_id: format!("source-record-{event_id}"),
            canonical_subject_id: "urn:epc:id:sscc:0614141.1234567890".to_owned(),
            event_class: "GATE_OUT".to_owned(),
            event_time: 10,
            published_at: 11,
            payload_digest: format!("digest-{event_id}"),
            durably_persisted: true,
            delivery_acknowledged: true,
        }
    }

    fn case(
        reference: Option<IndependentReference>,
        captured_events: Vec<CapturedEvent>,
    ) -> CaptureCase {
        CaptureCase {
            profile: Some(profile()),
            reference,
            captured_events,
            as_of: 20,
        }
    }

    fn known_case(case_id: &str) -> CaptureCase {
        match case_id {
            "complete" => case(
                Some(reference(&["e1", "e2"])),
                vec![captured("e1"), captured("e2")],
            ),
            "known_no_event" => case(Some(reference(&[])), vec![]),
            "missing_event" => case(
                Some(reference(&["e1", "e2", "e3"])),
                vec![captured("e1"), captured("e3")],
            ),
            "detected_but_not_durable" | "ack_without_durable_persistence" => {
                let mut event = captured("e1");
                event.durably_persisted = false;
                event.delivery_acknowledged = case_id == "ack_without_durable_persistence";
                case(Some(reference(&["e1"])), vec![event])
            }
            "late_publish_after_cutoff" => {
                let mut event = captured("e1");
                event.published_at = 21;
                case(Some(reference(&["e1"])), vec![event])
            }
            "exact_duplicate_delivery" => {
                let event = captured("e1");
                case(Some(reference(&["e1"])), vec![event.clone(), event])
            }
            "conflicting_event_identity" => {
                let mut conflict = captured("e1");
                conflict.payload_digest = "different-content".to_owned();
                case(Some(reference(&["e1"])), vec![captured("e1"), conflict])
            }
            "shared_clock_dependency" => {
                let mut p = profile();
                p.failure_domains.insert("reference-clock".to_owned());
                CaptureCase {
                    profile: Some(p),
                    reference: Some(reference(&["e1"])),
                    captured_events: vec![captured("e1")],
                    as_of: 20,
                }
            }
            "missing_independent_reference" => case(None, vec![captured("e1")]),
            "reference_capture_not_qualified" => {
                let mut r = reference(&["e1"]);
                r.coverage_complete = false;
                case(Some(r), vec![captured("e1")])
            }
            "profile_configuration_mismatch" => {
                let mut p = profile();
                p.configuration_digest = "config-v2".to_owned();
                CaptureCase {
                    profile: Some(p),
                    reference: Some(reference(&["e1"])),
                    captured_events: vec![captured("e1")],
                    as_of: 20,
                }
            }
            "unsupported_sequence_contract" => {
                let mut p = profile();
                p.sequence_semantics_qualified = false;
                CaptureCase {
                    profile: Some(p),
                    reference: Some(reference(&["e1"])),
                    captured_events: vec![captured("e1")],
                    as_of: 20,
                }
            }
            "missing_profile" => CaptureCase {
                profile: None,
                reference: Some(reference(&["e1"])),
                captured_events: vec![captured("e1")],
                as_of: 20,
            },
            "future_event_not_yet_occurred" => {
                let mut p = profile();
                p.window_end = 40;
                let mut r = reference(&["e1"]);
                r.window_end = 40;
                r.events[0].event_time = 30;
                CaptureCase {
                    profile: Some(p),
                    reference: Some(r),
                    captured_events: vec![],
                    as_of: 20,
                }
            }
            "future_dated_record_published_before_occurrence" => {
                let mut p = profile();
                p.window_end = 40;
                let mut r = reference(&["e1"]);
                r.window_end = 40;
                r.events[0].event_time = 30;
                let mut event = captured("e1");
                event.event_time = 30;
                event.published_at = 15;
                CaptureCase {
                    profile: Some(p),
                    reference: Some(r),
                    captured_events: vec![event],
                    as_of: 40,
                }
            }
            "as_of_before_window_end" => {
                let mut c = case(Some(reference(&[])), vec![]);
                c.as_of = 19;
                c
            }
            "as_of_before_window_start" => CaptureCase {
                profile: Some(profile()),
                reference: Some(reference(&[])),
                captured_events: vec![],
                as_of: -1,
            },
            _ => panic!("unknown fixture case: {case_id}"),
        }
    }

    #[test]
    fn frozen_expected_dispositions_match_independent_tsv_oracle() {
        let fixture = include_str!("../fixtures/capture-v0.1.tsv");
        let mut count = 0usize;
        for line in fixture
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 2, "malformed fixture row: {line}");
            let actual = evaluate(&known_case(fields[0]));
            assert_eq!(actual.disposition.as_str(), fields[1], "case {}", fields[0]);
            count += 1;
        }
        assert_eq!(count, 18);
    }

    fn claim_for_event(event_id: &str) -> CaptureClaimBinding {
        CaptureClaimBinding {
            source_id: "terminal-gate-1".to_owned(),
            event_class: "GATE_OUT".to_owned(),
            configuration_digest: "config-v1".to_owned(),
            event_id: event_id.to_owned(),
            source_record_id: format!("source-record-{event_id}"),
            canonical_subject_id: "urn:epc:id:sscc:0614141.1234567890".to_owned(),
            event_time: 10,
            payload_digest: format!("digest-{event_id}"),
        }
    }

    #[test]
    fn qualified_window_binds_exact_source_event_record_subject_and_time() {
        let report = evaluate(&case(
            Some(reference(&["e1", "e2"])),
            vec![captured("e1"), captured("e2")],
        ));
        assert_eq!(
            report.disposition,
            Disposition::QualifiedWithinDeclaredEnvelope
        );
        assert_eq!(report.reconciled_events.len(), 2);
        assert_eq!(
            admit_claim_against_capture(&report, &claim_for_event("e1")),
            CaptureClaimAdmission::Qualified
        );
    }

    #[test]
    fn claim_for_different_subject_is_not_admitted_by_same_capture_receipt() {
        let report = evaluate(&case(Some(reference(&["e1"])), vec![captured("e1")]));
        let mut claim = claim_for_event("e1");
        claim.canonical_subject_id = "urn:epc:id:sscc:0614141.9999999999".to_owned();
        assert_eq!(
            admit_claim_against_capture(&report, &claim),
            CaptureClaimAdmission::EvidenceDoesNotMatchClaim
        );
    }

    #[test]
    fn claim_for_different_source_record_or_configuration_is_not_admitted() {
        let report = evaluate(&case(
            Some(reference(&["e1"])),
            vec![captured("e1")],
        ));
        let mut claim = claim_for_event("e1");
        claim.source_record_id = "different-source-row".to_owned();
        assert_eq!(
            admit_claim_against_capture(&report, &claim),
            CaptureClaimAdmission::EvidenceDoesNotMatchClaim
        );
        claim = claim_for_event("e1");
        claim.configuration_digest = "config-v2".to_owned();
        assert_eq!(
            admit_claim_against_capture(&report, &claim),
            CaptureClaimAdmission::EvidenceDoesNotMatchClaim
        );
    }

    #[test]
    fn claim_from_different_source_or_event_class_is_not_admitted() {
        let report = evaluate(&case(Some(reference(&["e1"])), vec![captured("e1")]));
        let mut claim = claim_for_event("e1");
        claim.source_id = "different-terminal-gate".to_owned();
        assert_eq!(
            admit_claim_against_capture(&report, &claim),
            CaptureClaimAdmission::EvidenceDoesNotMatchClaim
        );
        claim = claim_for_event("e1");
        claim.event_class = "VESSEL_ARRIVAL".to_owned();
        assert_eq!(
            admit_claim_against_capture(&report, &claim),
            CaptureClaimAdmission::EvidenceDoesNotMatchClaim
        );
    }

    #[test]
    fn claim_with_wrong_source_payload_digest_is_not_admitted() {
        let report = evaluate(&case(Some(reference(&["e1"])), vec![captured("e1")]));
        let mut claim = claim_for_event("e1");
        claim.payload_digest = "different-payload-digest".to_owned();
        assert_eq!(
            admit_claim_against_capture(&report, &claim),
            CaptureClaimAdmission::EvidenceDoesNotMatchClaim
        );
    }

    #[test]
    fn claim_for_wrong_event_time_is_not_admitted() {
        let report = evaluate(&case(Some(reference(&["e1"])), vec![captured("e1")]));
        let mut claim = claim_for_event("e1");
        claim.event_time = 11;
        assert_eq!(
            admit_claim_against_capture(&report, &claim),
            CaptureClaimAdmission::EvidenceDoesNotMatchClaim
        );
    }

    #[test]
    fn malformed_report_without_provenance_cannot_admit_claim() {
        let report = CaptureReport::with_disposition(Disposition::QualifiedWithinDeclaredEnvelope);
        assert_eq!(
            admit_claim_against_capture(&report, &claim_for_event("e1")),
            CaptureClaimAdmission::NotEvaluable
        );
    }

    #[test]
    fn failed_capture_window_cannot_admit_an_individual_claim() {
        let report = evaluate(&case(Some(reference(&["e1", "e2"])), vec![captured("e1")]));
        assert_eq!(report.disposition, Disposition::Incomplete);
        assert_eq!(
            admit_claim_against_capture(&report, &claim_for_event("e1")),
            CaptureClaimAdmission::Incomplete
        );
    }

    #[test]
    fn independently_qualified_no_event_window_cannot_admit_an_unlisted_claim() {
        let report = evaluate(&case(Some(reference(&[])), vec![]));
        assert_eq!(
            admit_claim_against_capture(&report, &claim_for_event("not-present")),
            CaptureClaimAdmission::EvidenceDoesNotMatchClaim
        );
    }

    #[test]
    fn removing_a_persisted_event_cannot_improve_completeness() {
        let complete = case(
            Some(reference(&["e1", "e2"])),
            vec![captured("e1"), captured("e2")],
        );
        let incomplete = case(Some(reference(&["e1", "e2"])), vec![captured("e1")]);
        assert_eq!(
            evaluate(&complete).disposition,
            Disposition::QualifiedWithinDeclaredEnvelope
        );
        assert_eq!(evaluate(&incomplete).disposition, Disposition::Incomplete);
    }

    #[test]
    fn adding_a_shared_failure_domain_cannot_create_independence() {
        let mut p = profile();
        p.failure_domains.insert("reference-clock".to_owned());
        let test = CaptureCase {
            profile: Some(p),
            reference: Some(reference(&["e1"])),
            captured_events: vec![captured("e1")],
            as_of: 20,
        };
        let report = evaluate(&test);
        assert_eq!(report.disposition, Disposition::NotIdentifiable);
        assert_eq!(
            report.shared_failure_domains,
            vec!["reference-clock".to_owned()]
        );
    }

    #[test]
    fn delivery_acknowledgement_does_not_substitute_for_durable_persistence() {
        let report = evaluate(&known_case("ack_without_durable_persistence"));
        assert_eq!(report.disposition, Disposition::Incomplete);
        assert_eq!(report.non_durable_event_count, 1);
    }

    #[test]
    fn an_event_published_after_the_historical_cutoff_is_not_visible() {
        let report = evaluate(&known_case("late_publish_after_cutoff"));
        assert_eq!(report.disposition, Disposition::Incomplete);
        assert_eq!(report.missing_event_count, 1);
    }

    #[test]
    fn duplicate_identical_delivery_is_idempotent() {
        let report = evaluate(&known_case("exact_duplicate_delivery"));
        assert_eq!(
            report.disposition,
            Disposition::QualifiedWithinDeclaredEnvelope
        );
        assert_eq!(report.reconciled_event_count, 1);
    }

    #[test]
    fn conflicting_reuse_of_an_event_identity_is_incomplete() {
        let report = evaluate(&known_case("conflicting_event_identity"));
        assert_eq!(report.disposition, Disposition::Incomplete);
        assert_eq!(report.conflicting_identity_count, 1);
    }

    #[test]
    fn independently_observed_no_event_window_can_be_qualified() {
        let report = evaluate(&known_case("known_no_event"));
        assert_eq!(
            report.disposition,
            Disposition::QualifiedWithinDeclaredEnvelope
        );
        assert_eq!(report.expected_event_count, 0);
    }

    #[test]
    fn empty_or_unqualified_observation_is_not_identifiable() {
        assert_eq!(
            evaluate(&known_case("missing_independent_reference")).disposition,
            Disposition::NotIdentifiable
        );
        assert_eq!(
            evaluate(&known_case("reference_capture_not_qualified")).disposition,
            Disposition::NotIdentifiable
        );
    }

    #[test]
    fn profile_mismatch_or_unsupported_sequence_is_not_evaluable() {
        assert_eq!(
            evaluate(&known_case("profile_configuration_mismatch")).disposition,
            Disposition::NotEvaluable
        );
        assert_eq!(
            evaluate(&known_case("unsupported_sequence_contract")).disposition,
            Disposition::NotEvaluable
        );
    }

    #[test]
    fn missing_profile_is_unknown_not_complete() {
        assert_eq!(
            evaluate(&known_case("missing_profile")).disposition,
            Disposition::Unknown
        );
    }

    #[test]
    fn open_window_cannot_be_qualified_as_complete() {
        assert_eq!(
            evaluate(&known_case("future_event_not_yet_occurred")).disposition,
            Disposition::NotEvaluable
        );
        assert_eq!(
            evaluate(&known_case("as_of_before_window_end")).disposition,
            Disposition::NotEvaluable
        );
    }

    #[test]
    fn future_event_published_before_occurrence_is_not_evaluable() {
        assert_eq!(
            evaluate(&known_case(
                "future_dated_record_published_before_occurrence"
            ))
            .disposition,
            Disposition::NotEvaluable
        );
    }

    #[test]
    fn as_of_before_capture_window_is_not_evaluable() {
        assert_eq!(
            evaluate(&known_case("as_of_before_window_start")).disposition,
            Disposition::NotEvaluable
        );
    }
}
