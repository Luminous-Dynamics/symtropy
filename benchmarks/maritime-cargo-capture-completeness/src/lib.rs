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
pub struct CaptureReport {
    pub disposition: Disposition,
    pub expected_event_count: usize,
    pub reconciled_event_count: usize,
    pub missing_event_count: usize,
    pub non_durable_event_count: usize,
    pub unexpected_event_count: usize,
    pub conflicting_identity_count: usize,
    pub shared_failure_domains: Vec<String>,
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

pub fn evaluate(case: &CaptureCase) -> CaptureReport {
    let Some(profile) = case.profile.as_ref() else {
        return CaptureReport::with_disposition(Disposition::Unknown);
    };

    if !valid_profile(profile) {
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
    for event in &reference.events {
        if event.event_id.trim().is_empty()
            || event.event_class != profile.event_class
            || event.event_time < profile.window_start
            || event.event_time >= profile.window_end
            || expected
                .insert(event.event_id.clone(), event.event_time)
                .is_some()
        {
            return CaptureReport::with_disposition(Disposition::NotEvaluable);
        }
    }

    let mut visible: BTreeMap<String, CapturedEvent> = BTreeMap::new();
    let mut conflicting_identity_count = 0usize;
    for event in &case.captured_events {
        if event.event_class != profile.event_class
            || event.event_time < profile.window_start
            || event.event_time >= profile.window_end
            || event.published_at > case.as_of
        {
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

    for (event_id, event_time) in &expected {
        match visible.get(event_id) {
            None => report.missing_event_count += 1,
            Some(record) if !record.durably_persisted => {
                report.non_durable_event_count += 1;
            }
            Some(record) if record.event_time != *event_time => {
                report.conflicting_identity_count += 1;
            }
            Some(_) => report.reconciled_event_count += 1,
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
            window_end: 100,
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
            window_end: 100,
            coverage_complete: true,
            independently_qualified: true,
            failure_domains: set(&["reference-power", "reference-clock", "reference-sensor"]),
            events: events
                .iter()
                .map(|event_id| ReferenceEvent {
                    event_id: (*event_id).to_owned(),
                    event_class: "GATE_OUT".to_owned(),
                    event_time: 10,
                })
                .collect(),
        }
    }

    fn captured(event_id: &str) -> CapturedEvent {
        CapturedEvent {
            event_id: event_id.to_owned(),
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
                CaptureCase { profile: Some(p), reference: Some(reference(&["e1"])), captured_events: vec![captured("e1")], as_of: 20 }
            }
            "missing_profile" => CaptureCase {
                profile: None,
                reference: Some(reference(&["e1"])),
                captured_events: vec![captured("e1")],
                as_of: 20,
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
        assert_eq!(count, 14);
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
}
