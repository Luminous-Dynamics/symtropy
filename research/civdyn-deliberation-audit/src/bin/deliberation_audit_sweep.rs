// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic deliberation trace audit; no participant data is imported.

use civdyn_deliberation_audit::{
    ConsensusRule, DeliberationSession, EvidenceRecord, FacilitatorClaim, Participant, Proposal,
    TurnEvent, audit_session, compare_facilitation_variants,
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
            consensus_endorsement: Some(true),
        },
        Participant {
            id: "member_b".into(),
            preferences: vec![0.1, 0.8, 0.1],
            satisfaction_before: Some(0.6),
            satisfaction_after: Some(0.7),
            trust_before: Some(0.5),
            trust_after: Some(0.7),
            consensus_endorsement: Some(true),
        },
        Participant {
            id: "member_c".into(),
            preferences: vec![0.1, 0.1, 0.8],
            satisfaction_before: Some(0.6),
            satisfaction_after: Some(0.7),
            trust_before: Some(0.5),
            trust_after: Some(0.8),
            consensus_endorsement: Some(true),
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
        consensus_rule: Some(ConsensusRule::Unanimous),
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
    session.participants[2].consensus_endorsement = Some(false);
    session.final_allocation = vec![60, 35, 5];
    session
}

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "deliberation CSV schema must remain fixed"
    );
    println!("{}", fields.join(","));
}

struct AuditRow<'a> {
    record_type: &'a str,
    session_id: &'a str,
    metric: &'a str,
    subject: &'a str,
    value: &'a str,
    unit: &'a str,
    allocation: &'a str,
    shift: &'a str,
    reallocated: &'a str,
    consensus: &'a str,
    consensus_conflict: &'a str,
}

fn row(record: AuditRow<'_>) {
    emit(vec![
        record.record_type.into(),
        record.session_id.into(),
        record.metric.into(),
        record.subject.into(),
        record.value.into(),
        record.unit.into(),
        record.allocation.into(),
        record.shift.into(),
        record.reallocated.into(),
        record.consensus.into(),
        record.consensus_conflict.into(),
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
    let consensus_conflict = audit
        .consensus_claim_contradicted_by_recorded_non_endorsement
        .to_string();

    for (metric, value, unit) in [
        ("speaking_time_gini", fmt(audit.speaking_time_gini), "index"),
        ("turn_count_gini", fmt(audit.turn_count_gini), "index"),
        (
            "raised_proposal_count",
            audit.raised_proposal_count.to_string(),
            "count",
        ),
        (
            "summary_proposal_count",
            audit.summary_proposal_count.to_string(),
            "count",
        ),
        (
            "suppressed_proposal_count",
            audit.suppressed_proposal_count.to_string(),
            "count",
        ),
        (
            "proposal_summary_coverage",
            audit
                .proposal_summary_coverage
                .map_or_else(String::new, fmt),
            "fraction",
        ),
        (
            "summary_proposal_option_funded_share",
            audit
                .summary_proposal_option_funded_share
                .map_or_else(String::new, fmt),
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
            "consensus_claim_contradicted_by_recorded_non_endorsement",
            audit
                .consensus_claim_contradicted_by_recorded_non_endorsement
                .to_string(),
            "boolean",
        ),
        (
            "modal_top_preference_share",
            fmt(audit.modal_top_preference_share),
            "fraction",
        ),
        (
            "consensus_endorsement_count",
            audit.consensus_endorsement_count.to_string(),
            "count",
        ),
        (
            "consensus_non_endorsement_count",
            audit.consensus_non_endorsement_count.to_string(),
            "count",
        ),
        (
            "consensus_unrecorded_count",
            audit.consensus_unrecorded_count.to_string(),
            "count",
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
        row(AuditRow {
            record_type: "session_metric",
            session_id: &audit.session_id,
            metric,
            subject: "all_members",
            value: &value,
            unit,
            allocation: &allocation,
            shift: &shift,
            reallocated: &reallocated,
            consensus: &consensus,
            consensus_conflict: &consensus_conflict,
        });
    }

    for member in &audit.members {
        for (metric, value, unit) in [
            ("turn_count", member.turns.to_string(), "count"),
            (
                "speaking_time_ms",
                member.speaking_time_ms.to_string(),
                "ms",
            ),
            (
                "speaking_time_share",
                fmt(member.speaking_time_share),
                "fraction",
            ),
            (
                "proposals_authored",
                member.proposals_authored.to_string(),
                "count",
            ),
            (
                "proposals_referenced_in_turns",
                member.proposals_referenced_in_turns.to_string(),
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
            (
                "consensus_endorsement",
                member
                    .consensus_endorsement
                    .map_or_else(|| "unrecorded".into(), |value| value.to_string()),
                "categorical",
            ),
        ] {
            row(AuditRow {
                record_type: "member_metric",
                session_id: &audit.session_id,
                metric,
                subject: &member.participant_id,
                value: &value,
                unit,
                allocation: &allocation,
                shift: &shift,
                reallocated: &reallocated,
                consensus: &consensus,
                consensus_conflict: &consensus_conflict,
            });
        }
    }
}

fn main() {
    println!(
        "record_type,session_id,metric,subject,value,unit,final_allocation,allocation_l1_shift_units,reallocated_units,facilitator_declares_consensus,consensus_claim_contradicted_by_recorded_non_endorsement,source_type"
    );

    let baseline = base_session();
    let steered = steered_session();
    emit_audit(&baseline);
    emit_audit(&steered);

    let difference = compare_facilitation_variants(&baseline, &steered)
        .expect("paired sessions must preserve declared preferences and evidence");
    let pair_shift = difference.allocation_l1_shift_units.to_string();
    let pair_reallocated = fmt(difference.reallocated_units);
    row(AuditRow {
        record_type: "facilitation_variant_pair",
        session_id: &baseline.session_id,
        metric: "allocation_sensitivity",
        subject: &steered.session_id,
        value: &pair_reallocated,
        unit: "units",
        allocation: "",
        shift: &pair_shift,
        reallocated: &pair_reallocated,
        consensus: "",
        consensus_conflict: "",
    });

    let mut reordered = baseline.clone();
    reordered.session_id = "synthetic_option_order_position_bias".into();
    reordered.option_labels = vec!["charity_c".into(), "charity_b".into(), "charity_a".into()];
    for member in &mut reordered.participants {
        member.preferences.reverse();
    }
    for proposal in &mut reordered.proposals {
        proposal.option_index = 2 - proposal.option_index;
    }
    reordered.final_allocation = vec![5, 35, 60];
    let order_difference = compare_option_order_variants(&baseline, &reordered)
        .expect("option order pair must preserve semantic inputs");
    let order_shift = order_difference.allocation_l1_shift_units.to_string();
    let order_reallocated = fmt(order_difference.reallocated_units);
    row(AuditRow {
        record_type: "option_order_pair",
        session_id: &baseline.session_id,
        metric: "semantic_option_order_sensitivity",
        subject: &reordered.session_id,
        value: &order_reallocated,
        unit: "units",
        allocation: "",
        shift: &order_shift,
        reallocated: &order_reallocated,
        consensus: "",
        consensus_conflict: "",
    });
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
        assert_eq!(steered.consensus_endorsement_count, 2);
        assert_eq!(steered.consensus_non_endorsement_count, 1);
        assert_eq!(steered.consensus_unrecorded_count, 0);
        assert!(steered.consensus_claim_contradicted_by_recorded_non_endorsement);
    }

    #[test]
    fn facilitation_variant_pair_reports_fifty_six_l1_units() {
        let difference =
            compare_facilitation_variants(&base_session(), &steered_session()).unwrap();
        assert_eq!(difference.allocation_l1_shift_units, 56);
        assert_eq!(difference.reallocated_units, 28.0);
    }
}
