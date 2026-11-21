use std::collections::BTreeMap;
use std::fmt;

pub const ANNOUNCEMENT_SOURCE_ID: &str = "TRANSNET_STATEMENT_2026_11_18";
const ANNOUNCEMENT_HEADER: &str = "fixture_id|intervention_id|resource_id|claim|source_id|stage";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LifecycleStage {
    Announced,
    OfferedScheduled,
    Committed,
    Available,
    Activated,
    Realized,
    Delivered,
}

impl LifecycleStage {
    pub const ORDERED: [Self; 7] = [
        Self::Announced,
        Self::OfferedScheduled,
        Self::Committed,
        Self::Available,
        Self::Activated,
        Self::Realized,
        Self::Delivered,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Announced => "ANNOUNCED",
            Self::OfferedScheduled => "OFFERED_SCHEDULED",
            Self::Committed => "COMMITTED",
            Self::Available => "AVAILABLE",
            Self::Activated => "ACTIVATED",
            Self::Realized => "REALIZED",
            Self::Delivered => "DELIVERED",
        }
    }

    fn rank(self) -> usize {
        Self::ORDERED
            .iter()
            .position(|candidate| *candidate == self)
            .expect("stage is in the ordered lifecycle")
    }

    pub fn required_evidence(self) -> EvidenceKind {
        match self {
            Self::Announced => EvidenceKind::PublicStatement,
            Self::OfferedScheduled => EvidenceKind::ProviderSchedule,
            Self::Committed => EvidenceKind::AuthorizedCommitment,
            Self::Available => EvidenceKind::CapabilityInspection,
            Self::Activated => EvidenceKind::OperatorActivation,
            Self::Realized => EvidenceKind::IndependentPhysicalObservation,
            Self::Delivered => EvidenceKind::IndependentHandoffReceipt,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "ANNOUNCED" => Ok(Self::Announced),
            "OFFERED_SCHEDULED" => Ok(Self::OfferedScheduled),
            "COMMITTED" => Ok(Self::Committed),
            "AVAILABLE" => Ok(Self::Available),
            "ACTIVATED" => Ok(Self::Activated),
            "REALIZED" => Ok(Self::Realized),
            "DELIVERED" => Ok(Self::Delivered),
            other => Err(format!("unknown lifecycle stage {other:?}")),
        }
    }
}

impl fmt::Display for LifecycleStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceKind {
    PublicStatement,
    ProviderSchedule,
    AuthorizedCommitment,
    CapabilityInspection,
    OperatorActivation,
    IndependentPhysicalObservation,
    IndependentHandoffReceipt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IssuerRole {
    PublicSource,
    Provider,
    AuthorizedOperator,
    IndependentObserver,
    ReceivingParty,
}

impl EvidenceKind {
    fn permits_issuer(self, issuer: IssuerRole) -> bool {
        match self {
            Self::PublicStatement => {
                matches!(issuer, IssuerRole::PublicSource | IssuerRole::Provider)
            }
            Self::ProviderSchedule => {
                matches!(
                    issuer,
                    IssuerRole::Provider | IssuerRole::AuthorizedOperator
                )
            }
            Self::AuthorizedCommitment | Self::OperatorActivation => {
                issuer == IssuerRole::AuthorizedOperator
            }
            Self::CapabilityInspection => matches!(
                issuer,
                IssuerRole::AuthorizedOperator | IssuerRole::IndependentObserver
            ),
            Self::IndependentPhysicalObservation => issuer == IssuerRole::IndependentObserver,
            Self::IndependentHandoffReceipt => {
                matches!(
                    issuer,
                    IssuerRole::ReceivingParty | IssuerRole::IndependentObserver
                )
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub source_id: String,
    pub source_uri: String,
    pub artifact_sha256: String,
    pub issuer_id: String,
    pub issuer_role: IssuerRole,
    pub authority_ref: String,
    pub resource_id: String,
    pub event_time_utc: i64,
    pub publication_time_utc: i64,
    pub received_time_utc: i64,
    pub kind: EvidenceKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageAssertion {
    pub assertion_id: String,
    pub intervention_id: String,
    pub stage: LifecycleStage,
    pub evidence: EvidenceRecord,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplyResult {
    Applied {
        from: Option<LifecycleStage>,
        to: LifecycleStage,
        unobserved_predecessors: Vec<LifecycleStage>,
    },
    Corroborated {
        stage: LifecycleStage,
    },
    Replayed,
}

#[derive(Clone, Debug, Default)]
pub struct InterventionLedger {
    assertions: BTreeMap<String, StageAssertion>,
    current_stage: BTreeMap<String, LifecycleStage>,
    resource_by_intervention: BTreeMap<String, String>,
}

impl InterventionLedger {
    pub fn apply(&mut self, assertion: StageAssertion) -> Result<ApplyResult, String> {
        validate_assertion(&assertion)?;

        if let Some(existing) = self.assertions.get(&assertion.assertion_id) {
            return if existing == &assertion {
                Ok(ApplyResult::Replayed)
            } else {
                Err(format!(
                    "assertion ID {:?} was reused with conflicting content",
                    assertion.assertion_id
                ))
            };
        }

        if let Some(resource_id) = self
            .resource_by_intervention
            .get(&assertion.intervention_id)
            && resource_id != &assertion.evidence.resource_id
        {
            return Err(format!(
                "resource identity changed for intervention {:?}: {:?} -> {:?}",
                assertion.intervention_id, resource_id, assertion.evidence.resource_id
            ));
        }

        let previous = self.current_stage.get(&assertion.intervention_id).copied();
        if let Some(previous_stage) = previous
            && assertion.stage.rank() < previous_stage.rank()
        {
            return Err(format!(
                "lifecycle regression for {:?}: {} -> {}",
                assertion.intervention_id, previous_stage, assertion.stage
            ));
        }

        let result = match previous {
            Some(stage) if stage == assertion.stage => ApplyResult::Corroborated { stage },
            from => {
                let lower_rank = from.map_or(0, |stage| stage.rank() + 1);
                let unobserved_predecessors = LifecycleStage::ORDERED
                    .iter()
                    .copied()
                    .filter(|stage| {
                        stage.rank() >= lower_rank && stage.rank() < assertion.stage.rank()
                    })
                    .collect();
                ApplyResult::Applied {
                    from,
                    to: assertion.stage,
                    unobserved_predecessors,
                }
            }
        };

        self.resource_by_intervention
            .entry(assertion.intervention_id.clone())
            .or_insert_with(|| assertion.evidence.resource_id.clone());
        self.current_stage
            .entry(assertion.intervention_id.clone())
            .and_modify(|stage| {
                if assertion.stage.rank() > stage.rank() {
                    *stage = assertion.stage;
                }
            })
            .or_insert(assertion.stage);
        self.assertions
            .insert(assertion.assertion_id.clone(), assertion);
        Ok(result)
    }

    pub fn current_stage(&self, intervention_id: &str) -> Option<LifecycleStage> {
        self.current_stage.get(intervention_id).copied()
    }

    pub fn assertion_count(&self) -> usize {
        self.assertions.len()
    }

    pub fn replay_as_of(
        assertions: &[StageAssertion],
        publication_cutoff_utc: i64,
    ) -> Result<Self, String> {
        let mut ordered = assertions.to_vec();
        for assertion in &ordered {
            if assertion.evidence.publication_time_utc > publication_cutoff_utc {
                return Err(format!(
                    "post-cutoff evidence {:?} published at {} exceeds cutoff {}",
                    assertion.evidence.evidence_id,
                    assertion.evidence.publication_time_utc,
                    publication_cutoff_utc
                ));
            }
        }
        ordered.sort_by(|left, right| {
            (
                left.evidence.publication_time_utc,
                left.intervention_id.as_str(),
                left.stage.rank(),
                left.evidence.evidence_id.as_str(),
            )
                .cmp(&(
                    right.evidence.publication_time_utc,
                    right.intervention_id.as_str(),
                    right.stage.rank(),
                    right.evidence.evidence_id.as_str(),
                ))
        });

        let mut ledger = Self::default();
        for assertion in ordered {
            ledger.apply(assertion)?;
        }
        Ok(ledger)
    }
}

fn validate_assertion(assertion: &StageAssertion) -> Result<(), String> {
    let evidence = &assertion.evidence;
    for (name, value) in [
        ("assertion ID", assertion.assertion_id.as_str()),
        ("intervention ID", assertion.intervention_id.as_str()),
        ("evidence ID", evidence.evidence_id.as_str()),
        ("source ID", evidence.source_id.as_str()),
        ("source URI", evidence.source_uri.as_str()),
        ("issuer ID", evidence.issuer_id.as_str()),
        ("authority reference", evidence.authority_ref.as_str()),
        ("resource ID", evidence.resource_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name} must be non-empty"));
        }
    }

    if evidence.artifact_sha256.len() != 64
        || !evidence
            .artifact_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("artifact SHA-256 must be 64 lowercase hexadecimal characters".to_string());
    }
    if evidence.event_time_utc <= 0
        || evidence.publication_time_utc <= 0
        || evidence.received_time_utc <= 0
    {
        return Err("event, publication and receipt times must be positive UTC epochs".to_string());
    }
    if evidence.received_time_utc < evidence.publication_time_utc {
        return Err("receipt time precedes source publication time".to_string());
    }
    if assertion.stage.rank() >= LifecycleStage::Available.rank()
        && evidence.publication_time_utc < evidence.event_time_utc
    {
        return Err("physical-stage evidence was published before its event time".to_string());
    }
    if evidence.kind != assertion.stage.required_evidence() {
        return Err(format!(
            "{} requires {:?}, not {:?}",
            assertion.stage,
            assertion.stage.required_evidence(),
            evidence.kind
        ));
    }
    if !evidence.kind.permits_issuer(evidence.issuer_role) {
        return Err(format!(
            "issuer role {:?} is not permitted for evidence kind {:?}",
            evidence.issuer_role, evidence.kind
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnouncementFixtureRow {
    pub fixture_id: String,
    pub intervention_id: String,
    pub resource_id: String,
    pub claim: String,
    pub source_id: String,
    pub stage: LifecycleStage,
}

pub fn parse_announcement_fixture(input: &str) -> Result<Vec<AnnouncementFixtureRow>, String> {
    let mut lines = input.lines();
    if lines.next() != Some(ANNOUNCEMENT_HEADER) {
        return Err("unexpected announcement fixture header".to_string());
    }

    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!("blank fixture row at line {}", index + 2));
        }
        let fields: Vec<&str> = line.split('|').map(str::trim).collect();
        if fields.len() != 6 || fields.iter().any(|field| field.is_empty()) {
            return Err(format!(
                "invalid six-field fixture row at line {}",
                index + 2
            ));
        }
        let stage = LifecycleStage::parse(fields[5])?;
        if stage != LifecycleStage::Announced {
            return Err(format!(
                "public Transnet statement fixture may only assert ANNOUNCED, found {stage}"
            ));
        }
        if fields[4] != ANNOUNCEMENT_SOURCE_ID {
            return Err(format!(
                "unexpected source ID {:?} in fixture row",
                fields[4]
            ));
        }
        rows.push(AnnouncementFixtureRow {
            fixture_id: fields[0].to_string(),
            intervention_id: fields[1].to_string(),
            resource_id: fields[2].to_string(),
            claim: fields[3].to_string(),
            source_id: fields[4].to_string(),
            stage,
        });
    }
    if rows.is_empty() {
        return Err("announcement fixture has no data rows".to_string());
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(
        evidence_id: &str,
        kind: EvidenceKind,
        issuer_role: IssuerRole,
        resource_id: &str,
        event_time: i64,
        publication_time: i64,
    ) -> EvidenceRecord {
        EvidenceRecord {
            evidence_id: evidence_id.to_string(),
            source_id: format!("SOURCE_{evidence_id}"),
            source_uri: format!("https://evidence.invalid/{evidence_id}"),
            artifact_sha256: "a".repeat(64),
            issuer_id: format!("ISSUER_{evidence_id}"),
            issuer_role,
            authority_ref: format!("AUTHORITY_{evidence_id}"),
            resource_id: resource_id.to_string(),
            event_time_utc: event_time,
            publication_time_utc: publication_time,
            received_time_utc: publication_time + 5,
            kind,
        }
    }

    fn assertion(
        assertion_id: &str,
        intervention_id: &str,
        stage: LifecycleStage,
        evidence: EvidenceRecord,
    ) -> StageAssertion {
        StageAssertion {
            assertion_id: assertion_id.to_string(),
            intervention_id: intervention_id.to_string(),
            stage,
            evidence,
        }
    }

    #[test]
    fn public_statement_fixture_contains_six_announced_measures_only() {
        let rows =
            parse_announcement_fixture(include_str!("../data/dgt-package-announcements.csv"))
                .expect("frozen public statement claims");
        assert_eq!(rows.len(), 6);
        assert!(
            rows.iter()
                .all(|row| row.stage == LifecycleStage::Announced)
        );
        assert!(
            rows.iter()
                .all(|row| row.source_id == ANNOUNCEMENT_SOURCE_ID)
        );
    }

    #[test]
    fn public_statement_cannot_prove_physical_realization() {
        let announced = assertion(
            "A1",
            "CRANE_1",
            LifecycleStage::Announced,
            evidence(
                "E1",
                EvidenceKind::PublicStatement,
                IssuerRole::PublicSource,
                "CRANE_1",
                100,
                110,
            ),
        );
        let unrealized = assertion(
            "A2",
            "CRANE_1",
            LifecycleStage::Realized,
            announced.evidence.clone(),
        );
        let mut ledger = InterventionLedger::default();
        ledger.apply(announced).expect("announcement is admissible");
        assert!(ledger.apply(unrealized).unwrap_err().contains("requires"));
        assert_eq!(
            ledger.current_stage("CRANE_1"),
            Some(LifecycleStage::Announced)
        );
    }

    #[test]
    fn provider_cannot_impersonate_an_independent_physical_observer() {
        let record = evidence(
            "E_PROVIDER",
            EvidenceKind::IndependentPhysicalObservation,
            IssuerRole::Provider,
            "CRANE_1",
            100,
            110,
        );
        let result = InterventionLedger::default().apply(assertion(
            "A_PROVIDER",
            "CRANE_1",
            LifecycleStage::Realized,
            record,
        ));
        assert!(result.unwrap_err().contains("not permitted"));
    }

    #[test]
    fn receiving_party_handoff_receipt_can_prove_delivery() {
        let record = evidence(
            "E_DELIVERY",
            EvidenceKind::IndependentHandoffReceipt,
            IssuerRole::ReceivingParty,
            "CONTAINER_HANDOFF_1",
            200,
            210,
        );
        let result = InterventionLedger::default()
            .apply(assertion(
                "A_DELIVERY",
                "CONTAINER_1",
                LifecycleStage::Delivered,
                record,
            ))
            .expect("counterparty receipt");
        assert!(matches!(
            result,
            ApplyResult::Applied {
                to: LifecycleStage::Delivered,
                ..
            }
        ));
    }

    #[test]
    fn higher_stage_requires_its_exact_evidence_class() {
        let record = evidence(
            "E_SCHEDULE",
            EvidenceKind::ProviderSchedule,
            IssuerRole::Provider,
            "BERTH_102",
            200,
            210,
        );
        let result = InterventionLedger::default().apply(assertion(
            "A_BAD_COMMITMENT",
            "BERTH_102",
            LifecycleStage::Committed,
            record,
        ));
        assert!(result.unwrap_err().contains("requires"));
    }

    #[test]
    fn lifecycle_cannot_regress() {
        let available = assertion(
            "A_AVAILABLE",
            "CRANE_1",
            LifecycleStage::Available,
            evidence(
                "E_AVAILABLE",
                EvidenceKind::CapabilityInspection,
                IssuerRole::IndependentObserver,
                "CRANE_1",
                100,
                110,
            ),
        );
        let announced = assertion(
            "A_OLD_ANNOUNCEMENT",
            "CRANE_1",
            LifecycleStage::Announced,
            evidence(
                "E_OLD",
                EvidenceKind::PublicStatement,
                IssuerRole::PublicSource,
                "CRANE_1",
                90,
                95,
            ),
        );
        let mut ledger = InterventionLedger::default();
        ledger.apply(available).expect("qualified availability");
        assert!(ledger.apply(announced).unwrap_err().contains("regression"));
    }

    #[test]
    fn one_intervention_cannot_silently_change_physical_resource_identity() {
        let first = assertion(
            "A_BERTH",
            "DIVERSION_PACKAGE",
            LifecycleStage::Announced,
            evidence(
                "E_BERTH",
                EvidenceKind::PublicStatement,
                IssuerRole::PublicSource,
                "BERTH_102",
                100,
                110,
            ),
        );
        let second = assertion(
            "A_OTHER",
            "DIVERSION_PACKAGE",
            LifecycleStage::OfferedScheduled,
            evidence(
                "E_OTHER",
                EvidenceKind::ProviderSchedule,
                IssuerRole::Provider,
                "NCT_BERTH_1",
                120,
                130,
            ),
        );
        let mut ledger = InterventionLedger::default();
        ledger.apply(first).expect("first resource claim");
        assert!(
            ledger
                .apply(second)
                .unwrap_err()
                .contains("resource identity changed")
        );
    }

    #[test]
    fn exact_assertion_replay_is_idempotent_but_conflicting_reuse_rejects() {
        let first = assertion(
            "A1",
            "RAIL_EVACUATION",
            LifecycleStage::Announced,
            evidence(
                "E1",
                EvidenceKind::PublicStatement,
                IssuerRole::PublicSource,
                "TFR_SERVICE",
                100,
                110,
            ),
        );
        let mut ledger = InterventionLedger::default();
        ledger.apply(first.clone()).expect("first assertion");
        assert_eq!(
            ledger.apply(first.clone()).expect("exact replay"),
            ApplyResult::Replayed
        );

        let conflict = assertion(
            "A1",
            "RAIL_EVACUATION",
            LifecycleStage::Announced,
            evidence(
                "E2",
                EvidenceKind::PublicStatement,
                IssuerRole::PublicSource,
                "TFR_SERVICE",
                100,
                110,
            ),
        );
        assert!(ledger.apply(conflict).unwrap_err().contains("reused"));
        assert_eq!(ledger.assertion_count(), 1);
    }

    #[test]
    fn post_cutoff_publication_is_rejected_even_when_event_time_is_earlier() {
        let record = evidence(
            "E_LATE",
            EvidenceKind::IndependentPhysicalObservation,
            IssuerRole::IndependentObserver,
            "CRANE_1",
            100,
            150,
        );
        let assertion = assertion("A_LATE", "CRANE_1", LifecycleStage::Realized, record);
        assert!(
            InterventionLedger::replay_as_of(&[assertion], 149)
                .unwrap_err()
                .contains("post-cutoff")
        );
    }

    #[test]
    fn future_scheduled_window_can_be_known_before_a_replay_cutoff() {
        let record = evidence(
            "E_SCHEDULE",
            EvidenceKind::ProviderSchedule,
            IssuerRole::Provider,
            "BERTH_102",
            500,
            200,
        );
        let assertion = assertion(
            "A_SCHEDULE",
            "BERTH_102_GEARED_SERVICE",
            LifecycleStage::OfferedScheduled,
            record,
        );
        let ledger = InterventionLedger::replay_as_of(&[assertion], 250)
            .expect("a future scheduled event may be known before its event time");
        assert_eq!(
            ledger.current_stage("BERTH_102_GEARED_SERVICE"),
            Some(LifecycleStage::OfferedScheduled)
        );
    }

    #[test]
    fn stage_jump_records_unobserved_predecessors_instead_of_inventing_them() {
        let record = evidence(
            "E_REALIZED",
            EvidenceKind::IndependentPhysicalObservation,
            IssuerRole::IndependentObserver,
            "STS_CRANE_1",
            100,
            110,
        );
        let result = InterventionLedger::default()
            .apply(assertion(
                "A_REALIZED",
                "CRANE_1",
                LifecycleStage::Realized,
                record,
            ))
            .expect("independent physical evidence");
        match result {
            ApplyResult::Applied {
                from,
                to,
                unobserved_predecessors,
            } => {
                assert_eq!(from, None);
                assert_eq!(to, LifecycleStage::Realized);
                assert_eq!(
                    unobserved_predecessors,
                    vec![
                        LifecycleStage::Announced,
                        LifecycleStage::OfferedScheduled,
                        LifecycleStage::Committed,
                        LifecycleStage::Available,
                        LifecycleStage::Activated,
                    ]
                );
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn physical_event_cannot_be_published_before_it_happened() {
        let record = evidence(
            "E_FUTURE",
            EvidenceKind::IndependentPhysicalObservation,
            IssuerRole::IndependentObserver,
            "STS_CRANE_1",
            200,
            190,
        );
        let result = InterventionLedger::default().apply(assertion(
            "A_FUTURE",
            "CRANE_1",
            LifecycleStage::Realized,
            record,
        ));
        assert!(result.unwrap_err().contains("published before"));
    }

    #[test]
    fn digest_and_receipt_clock_are_validated() {
        let mut record = evidence(
            "E_BAD",
            EvidenceKind::PublicStatement,
            IssuerRole::PublicSource,
            "BERTH_102",
            100,
            110,
        );
        record.artifact_sha256 = "not-a-digest".to_string();
        let bad_digest = assertion("A_BAD", "BERTH_102", LifecycleStage::Announced, record);
        assert!(
            InterventionLedger::default()
                .apply(bad_digest)
                .unwrap_err()
                .contains("SHA-256")
        );

        let mut record = evidence(
            "E_CLOCK",
            EvidenceKind::PublicStatement,
            IssuerRole::PublicSource,
            "BERTH_102",
            100,
            110,
        );
        record.received_time_utc = 109;
        let bad_clock = assertion("A_CLOCK", "BERTH_102", LifecycleStage::Announced, record);
        assert!(
            InterventionLedger::default()
                .apply(bad_clock)
                .unwrap_err()
                .contains("receipt time")
        );
    }
}
