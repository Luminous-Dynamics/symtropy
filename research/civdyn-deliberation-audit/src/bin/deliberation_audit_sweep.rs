// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic deliberation trace audit; no participant data is imported.

use civdyn_deliberation_audit::{
    DeliberationSession, EvidenceRecord, FacilitatorClaim, Participant, Proposal, TurnEvent,
    audit_session, compare_rhetoric_only,
};

const CSV_COLUMNS: usize = 12;

fn base_participants() -> Vec<Participant> {
    vec![
        Participant {
            id: "member_a".into(),
            preferences: vec![0.8, 0.1, 0.1],
            satisfaction_before: Some(0.6),
            satisfaction_after: Some(0.7),
            trust_before: Some(0.5),
            trust_after: Some(0.6),
        },
        Participant {
            id: "member_b".into(),
            preferences: vec![0.1, 0.8, 0.1],
            satisfaction_before: Some(0.6),
            satisfaction_after: Some(0.7),
            trust_before: Some(0.5),
            trust_after: Some(0.7),
        },
        Participant {
            id: "member_c".into(),
            preferences: vec![0.1, 0.1, 0.8],
            satisfaction_before: Some(0.6),
            satisfaction_after: Some(0.7),
            trust_before: Some(0.5),
            trust_after: Some(0.8),
        },
    ]
}

fn base_session() -> DeliberationSession {
    DeliberationSession {
        session_id: "synthetic_unfacilitated_reference".into(),
        option_labels: vec!["charity_a".into(), "charity_b".into(), "charity_c".into()],
        resource_budget: 100,
        participants: base_participants(),
        proposals: vec![
            Proposal {
                id: "proposal_a".into(),
                author_id: "member_a".into(),
                option_index: 0,
            },
            Proposal {
                id: "proposal_b".into(),
                author_id: "member_b".into(),
                option_index: 1,
            },
            Proposal {
                id: "proposal_c".into(),
                author_id: "member_c".into(),
                option_index: 2,
            },
        ],
        turns: vec![
            TurnEvent {
                speaker_id: "member_a".into(),
                duration_ms: 1000,
                proposal_ids: vec!["proposal_a".into()],
            },
            TurnEvent {
                speaker_id: "member_a".into(),
                duration_ms: 1000,
                proposal_ids: vec![],
            },
            TurnEvent {
                speaker_id: "member_a".into(),
                duration_ms: 1000,
                proposal_ids: vec![],
            },
            TurnEvent {
                speaker_id: "member_a".into(),
                duration_ms: 1000,
                proposal_ids: vec![],
            },
            TurnEvent {
                speaker_id: "member_a".into(),
                duration_ms: 1000,
                proposal_ids: vec![],
            },
            TurnEvent {
                speaker_id: "member_b".into(),
                duration_ms: 2000,
                proposal_ids: vec!["proposal_b".into()],
            },
            TurnEvent {
                speaker_id: "member_c".into(),
                duration_ms: 2000,
                proposal_ids: vec!["proposal_c".into()],
            },
        ],
        summary_proposal_ids: vec![
            "proposal_a".into(),
            "proposal_b".into(),
            "proposal_c".into(),
        ],
        evidence_catalogue: vec![EvidenceRecord {
            evidence_id: "verified_budget_record".into(),
            source_digest: "sha256:abc123".into(),
            verified: true,
        }],
        facilitator_claims: vec![FacilitatorClaim {
            claim_id: "claim_budget".into(),
            evidence_id: Some("verified_budget_record".into()),
        }],
        facilitator_declares_consensus: false,
        final_allocation: vec![34, 33, 33],
    }
}

fn steered_session() -> DeliberationSession {
    let mut session = base_session();
    session.session_id = "synthetic_facilitated_steering_control".into();
    session.turns = vec![
        TurnEvent {
            speaker_id: "member_a".into(),
            duration_ms: 3000,
            proposal_ids: vec!["proposal_a".into()],
        },
        TurnEvent {
            speaker_id: "member_b".into(),
            duration_ms: 500,
            proposal_ids: vec!["proposal_b".into()],
        },
        TurnEvent {
            speaker_id: "member_c".into(),
            duration_ms: 500,
            proposal_ids: vec!["proposal_c".into()],
        },
    ];
    session.summary_proposal_ids = vec!["proposal_a".into(), "proposal_b".into()];
    session.facilitator_claims.push(FacilitatorClaim {
        claim_id: "unsupported_consensus_claim".into(),
        evidence_id: Some("missing_consensus_source".into()),
    });
    session.facilitator_declares_consensus = true;
    session.final_allocation = vec![60, 35, 5];
    session
}

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS, "deliberation CSV schema must remain fixed");
    println!("{}", fields.join(","));
}

fn row(
    record_type: &str,
    session_id: &str,
    metric: &str,
    subject: &str,
    value: &str,
    unit: &str,
    allocation: &str,
    shift: &str,
    reallocated: &str,
    consensus: &str,
    false_consensus: &str,
) {
    emit(vec![
        record_type.into(),
        session_id.into(),
        metric.into(),
        subject.into(),
        value.into(),
        unit.into(),
        allocation.into(),
        shift.into(),
        reallocated.into(),
        consensus.into(),
        false_consensus.into(),
        "synthetic_fixture".into(),
    ]);
}

fn emit_audit(session: &DeliberationSession) {
    let audit = audit_session(session).expect("synthetic deliberation trace must validate");
    let allocation = audit
        .final_allocation
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join("|");
    let shift = audit.allocation_l1_shift_units.to_string();
    let reallocated = fmt(audit.reallocated_units);
    let consensus = audit.facilitator_declares_consensus.to_string();
    let false_consensus = audit.false_consensus_declaration.to_string();

    for (metric, value, unit) in [
        ("speaking_time_gini", fmt(audit.speaking_time_gini), "index"),
        ("turn_count_gini", fmt(audit.turn_count_gini), "index"),
        ("raised_proposal_count", audit.raised_proposal_count.to_string(), "count"),
        ("summary_proposal_count", audit.summary_proposal_count.to_string(), "count"),
        ("suppressed_proposal_count", audit.suppressed_proposal_count.to_string(), "count"),
        (
            "proposal_summary_coverage",
            audit.proposal_summary_coverage.map_or_else(String::new, fmt),
            "fraction",
        ),
        (
            "summary_proposal_option_funded_share",
            audit.summary_proposal_option_funded_share.map_or_else(String::new, fmt),
            "fraction",
        ),
        (
            "verified_facilitator_claim_count",
            audit.verified_facilitator_claim_count.to_string(),
            "count",
        ),
        (
            "unsupported_facilitator_claim_count",
            audit.unsupported_facilitator_claim_count.to_string(),
            "count",
        ),
        (
            "preference_consensus_share",
            fmt(audit.preference_consensus_share),
            "fraction",
        ),
        (
            "baseline_preference_alignment",
            fmt(audit.baseline_preference_alignment),
            "fraction",
        ),
        (
            "final_preference_alignment",
            fmt(audit.final_preference_alignment),
            "fraction",
        ),
        (
            "allocation_l1_shift_units",
            audit.allocation_l1_shift_units.to_string(),
            "units",
        ),
        ("reallocated_units", reallocated.clone(), "units"),
        (
            "mean_satisfaction_before",
            audit.mean_satisfaction_before.map_or_else(String::new, fmt),
            "self_report",
        ),
        (
            "mean_satisfaction_after",
            audit.mean_satisfaction_after.map_or_else(String::new, fmt),
            "self_report",
        ),
        (
            "mean_trust_before",
            audit.mean_trust_before.map_or_else(String::new, fmt),
            "self_report",
        ),
        (
            "mean_trust_after",
            audit.mean_trust_after.map_or_else(String::new, fmt),
            "self_report",
        ),
    ] {
        row(
            "session_metric",
            &audit.session_id,
            metric,
            "all_members",
            &value,
            unit,
            &allocation,
            &shift,
            &reallocated,
            &consensus,
            &false_consensus,
        );
    }

    for member in &audit.members {
        for (metric, value, unit) in [
            ("turn_count", member.turns.to_string(), "count"),
            ("speaking_time_ms", member.speaking_time_ms.to_string(), "ms"),
            ("speaking_time_share", fmt(member.speaking_time_share), "fraction"),
            ("proposals_authored", member.proposals_authored.to_string(), "count"),
            (
                "proposals_raised_in_turns",
                member.proposals_raised_in_turns.to_string(),
                "count",
            ),
            (
                "proposals_in_summary",
                member.proposals_in_summary.to_string(),
                "count",
            ),
            (
                "final_preference_alignment",
                fmt(member.final_preference_alignment),
                "fraction",
            ),
        ] {
            row(
                "member_metric",
                &audit.session_id,
                metric,
                &member.participant_id,
                &value,
                unit,
                &allocation,
                &shift,
                &reallocated,
                &consensus,
                &false_consensus,
            );
        }
    }
}

fn main() {
    println!(
        "record_type,session_id,metric,subject,value,unit,final_allocation,allocation_l1_shift_units,reallocated_units,facilitator_declares_consensus,false_consensus_declaration,source_type"
    );

    let baseline = base_session();
    let steered = steered_session();
    emit_audit(&baseline);
    emit_audit(&steered);

    let difference = compare_rhetoric_only(&baseline, &steered)
        .expect("paired sessions must preserve declared preferences and evidence");
    row(
        "rhetoric_only_pair",
        &baseline.session_id,
        "allocation_sensitivity",
        &steered.session_id,
        &fmt(difference.reallocated_units),
        "units",
        "",
        &difference.allocation_l1_shift_units.to_string(),
        &fmt(difference.reallocated_units),
        "",
        "",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_baseline_and_steering_pair_validate() {
        let baseline = audit_session(&base_session()).unwrap();
        let steered = audit_session(&steered_session()).unwrap();
        assert_eq!(baseline.preference_proportional_baseline, vec![34, 33, 33]);
        assert_eq!(steered.suppressed_proposal_count, 1);
        assert!(steered.false_consensus_declaration);
    }

    #[test]
    fn rhetoric_only_pair_reports_fifty_six_l1_units() {
        let difference = compare_rhetoric_only(&base_session(), &steered_session()).unwrap();
        assert_eq!(difference.allocation_l1_shift_units, 56);
        assert_eq!(difference.reallocated_units, 28.0);
    }
}
