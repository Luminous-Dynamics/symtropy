use maritime_dgt_intervention_effect::{
    ANNOUNCEMENT_SOURCE_ID, EvidenceKind, EvidenceRecord, InterventionLedger, IssuerRole,
    LifecycleStage, StageAssertion, parse_announcement_fixture,
};

fn synthetic_evidence(
    evidence_id: &str,
    kind: EvidenceKind,
    issuer_role: IssuerRole,
    event_time_utc: i64,
    publication_time_utc: i64,
) -> EvidenceRecord {
    EvidenceRecord {
        evidence_id: evidence_id.to_string(),
        source_id: format!("SYNTHETIC_{evidence_id}"),
        source_uri: format!("https://example.invalid/synthetic/{evidence_id}"),
        artifact_sha256: "b".repeat(64),
        issuer_id: format!("SYNTHETIC_ISSUER_{evidence_id}"),
        issuer_role,
        authority_ref: format!("SYNTHETIC_AUTHORITY_{evidence_id}"),
        resource_id: "SYNTHETIC_CRANE_01".to_string(),
        event_time_utc,
        publication_time_utc,
        received_time_utc: publication_time_utc + 5,
        kind,
    }
}

fn main() {
    let rows = parse_announcement_fixture(include_str!("../data/dgt-package-announcements.csv"))
        .unwrap_or_else(|error| {
            eprintln!("DGT public-announcement fixture is invalid: {error}");
            std::process::exit(2);
        });

    println!("DGT intervention evidence lifecycle benchmark v0.1");
    println!("source_id={ANNOUNCEMENT_SOURCE_ID}");
    println!(
        "public statement rows={} claim_ceiling=ANNOUNCED",
        rows.len()
    );
    for row in &rows {
        println!(
            "  {} resource={} stage={}",
            row.fixture_id, row.resource_id, row.stage
        );
    }

    // This sequence is deliberately synthetic. It demonstrates evidence-stage
    // validation and does not assert that any Transnet intervention progressed.
    let sequence = [
        (
            LifecycleStage::Announced,
            EvidenceKind::PublicStatement,
            IssuerRole::PublicSource,
            100,
            110,
        ),
        (
            LifecycleStage::OfferedScheduled,
            EvidenceKind::ProviderSchedule,
            IssuerRole::Provider,
            120,
            125,
        ),
        (
            LifecycleStage::Committed,
            EvidenceKind::AuthorizedCommitment,
            IssuerRole::AuthorizedOperator,
            130,
            135,
        ),
        (
            LifecycleStage::Available,
            EvidenceKind::CapabilityInspection,
            IssuerRole::IndependentObserver,
            140,
            145,
        ),
        (
            LifecycleStage::Activated,
            EvidenceKind::OperatorActivation,
            IssuerRole::AuthorizedOperator,
            150,
            155,
        ),
        (
            LifecycleStage::Realized,
            EvidenceKind::IndependentPhysicalObservation,
            IssuerRole::IndependentObserver,
            160,
            165,
        ),
        (
            LifecycleStage::Delivered,
            EvidenceKind::IndependentHandoffReceipt,
            IssuerRole::ReceivingParty,
            170,
            175,
        ),
    ];

    let mut ledger = InterventionLedger::default();
    for (index, (stage, kind, role, event_time, publication_time)) in
        sequence.into_iter().enumerate()
    {
        let assertion = StageAssertion {
            assertion_id: format!("SYNTHETIC_ASSERTION_{index}"),
            intervention_id: "SYNTHETIC_CRANE_AND_HANDOFF".to_string(),
            stage,
            evidence: synthetic_evidence(
                &format!("SYNTHETIC_EVIDENCE_{index}"),
                kind,
                role,
                event_time,
                publication_time,
            ),
        };
        if let Err(error) = ledger.apply(assertion) {
            eprintln!("synthetic lifecycle example failed: {error}");
            std::process::exit(2);
        }
    }

    println!(
        "synthetic lifecycle final stage={}",
        ledger
            .current_stage("SYNTHETIC_CRANE_AND_HANDOFF")
            .expect("synthetic stage exists")
    );
    println!("No DGT capacity activation, realized moves or cargo deliveries are inferred.");
}
