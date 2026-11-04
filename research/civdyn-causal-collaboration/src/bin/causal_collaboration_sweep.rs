// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic CCE, PAC and evidence-lineage demonstration.

use civdyn_causal_collaboration::{
    ActionKind, ActionRecord, AgentRecord, CausalEdge, CausalEdgeStatus, CausalEvidenceKind,
    CollaborationTrace,
};

const CSV_COLUMNS: usize = 36;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn fmt_opt(value: Option<f64>) -> String {
    value.map(fmt).unwrap_or_default()
}

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS, "each collaboration record has 36 columns");
    println!("{}", fields.join(","));
}

fn trace(success: bool) -> CollaborationTrace {
    CollaborationTrace {
        agents: vec![
            AgentRecord { agent_id: "miner".into(), role: "extract resources".into() },
            AgentRecord { agent_id: "courier".into(), role: "transport resources".into() },
            AgentRecord { agent_id: "smith".into(), role: "craft equipment".into() },
            AgentRecord { agent_id: "wizard".into(), role: "provide instructions".into() },
            AgentRecord { agent_id: "idle".into(), role: "standby".into() },
        ],
        actions: vec![
            ActionRecord { action_id: "ore".into(), agent_id: "miner".into(), sequence: 1, round: 1, kind: ActionKind::EnvironmentAction, fingerprint: Some("mine-iron".into()), resource_ref: Some("ore-1".into()) },
            ActionRecord { action_id: "message".into(), agent_id: "miner".into(), sequence: 2, round: 1, kind: ActionKind::Communication, fingerprint: None, resource_ref: None },
            ActionRecord { action_id: "wood".into(), agent_id: "courier".into(), sequence: 3, round: 2, kind: ActionKind::EnvironmentAction, fingerprint: Some("gather-wood".into()), resource_ref: Some("wood-1".into()) },
            ActionRecord { action_id: "transfer".into(), agent_id: "courier".into(), sequence: 4, round: 2, kind: ActionKind::ResourceTransfer, fingerprint: Some("transfer-ore-1".into()), resource_ref: Some("ore-1".into()) },
            ActionRecord { action_id: "craft".into(), agent_id: "smith".into(), sequence: 5, round: 3, kind: ActionKind::EnvironmentAction, fingerprint: Some("craft-vanguard".into()), resource_ref: None },
            ActionRecord { action_id: "hint".into(), agent_id: "wizard".into(), sequence: 6, round: 3, kind: ActionKind::Communication, fingerprint: None, resource_ref: None },
            ActionRecord { action_id: "duplicate-craft".into(), agent_id: "smith".into(), sequence: 7, round: 4, kind: ActionKind::EnvironmentAction, fingerprint: Some("craft-vanguard".into()), resource_ref: None },
            ActionRecord { action_id: "deliver".into(), agent_id: "smith".into(), sequence: 8, round: 5, kind: ActionKind::Checkpoint, fingerprint: Some("deliver-vanguard".into()), resource_ref: None },
        ],
        causal_edges: vec![
            CausalEdge { cause_action_id: "ore".into(), effect_action_id: "transfer".into(), evidence_ref: "ledger/ore-1".into(), evidence_kind: CausalEvidenceKind::ResourceLineageReceipt, status: CausalEdgeStatus::Verified },
            CausalEdge { cause_action_id: "message".into(), effect_action_id: "transfer".into(), evidence_ref: "chat/ack-44".into(), evidence_kind: CausalEvidenceKind::AcknowledgedCommunication, status: CausalEdgeStatus::Candidate },
            CausalEdge { cause_action_id: "wood".into(), effect_action_id: "craft".into(), evidence_ref: "ledger/wood-1".into(), evidence_kind: CausalEvidenceKind::ResourceLineageReceipt, status: CausalEdgeStatus::Verified },
            CausalEdge { cause_action_id: "transfer".into(), effect_action_id: "craft".into(), evidence_ref: "inventory/receipt-7".into(), evidence_kind: CausalEvidenceKind::EnvironmentStateTransition, status: CausalEdgeStatus::Verified },
            CausalEdge { cause_action_id: "craft".into(), effect_action_id: "deliver".into(), evidence_ref: "task/verifier-1".into(), evidence_kind: CausalEvidenceKind::DeterministicPrecondition, status: CausalEdgeStatus::Verified },
            CausalEdge { cause_action_id: "hint".into(), effect_action_id: "craft".into(), evidence_ref: "model-judge/proposal-2".into(), evidence_kind: CausalEvidenceKind::ModelProposed, status: CausalEdgeStatus::Candidate },
        ],
        success_action_ids: if success { vec!["deliver".into()] } else { Vec::new() },
        task_succeeded: success,
        partial_success_count: if success { 4 } else { 2 },
        partial_success_total: 4,
    }
}

fn summary_row(fixture_id: &str, metrics: &civdyn_causal_collaboration::CollaborationMetrics) -> Vec<String> {
    vec![
        "summary".into(),
        fixture_id.into(),
        metrics.task_succeeded.to_string(),
        metrics.action_count.to_string(),
        metrics.agent_count.to_string(),
        metrics.round_count.to_string(),
        fmt_opt(metrics.partial_success_ratio),
        metrics.verified_contributing_actions.to_string(),
        metrics.candidate_inclusive_contributing_actions.to_string(),
        fmt(metrics.cce_verified_lower),
        fmt(metrics.cce_candidate_upper),
        metrics.verified_cross_agent_causal_edges.to_string(),
        metrics.candidate_inclusive_cross_agent_causal_edges.to_string(),
        metrics.verified_cross_agent_contributing_actions.to_string(),
        metrics.candidate_inclusive_cross_agent_contributing_actions.to_string(),
        metrics.message_action_count.to_string(),
        metrics.verified_contributing_messages.to_string(),
        metrics.candidate_inclusive_contributing_messages.to_string(),
        fmt_opt(metrics.message_effectiveness_verified_lower),
        fmt_opt(metrics.message_effectiveness_candidate_upper),
        metrics.resource_transfer_action_count.to_string(),
        metrics.duplicate_action_count.to_string(),
        metrics.duplicate_fingerprint_groups.to_string(),
        String::new(), String::new(), String::new(), String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(), String::new(), String::new(),
    ]
}

fn evidence_kind_name(kind: CausalEvidenceKind) -> &'static str {
    match kind {
        CausalEvidenceKind::EnvironmentStateTransition => "environment_state_transition",
        CausalEvidenceKind::ResourceLineageReceipt => "resource_lineage_receipt",
        CausalEvidenceKind::AcknowledgedCommunication => "acknowledged_communication",
        CausalEvidenceKind::DeterministicPrecondition => "deterministic_precondition",
        CausalEvidenceKind::HumanAdjudication => "human_adjudication",
        CausalEvidenceKind::ModelProposed => "model_proposed",
        CausalEvidenceKind::Other => "other",
    }
}

fn main() {
    println!(
        "record_type,fixture_id,task_succeeded,action_count,agent_count,round_count,partial_success_ratio,verified_contributing_actions,candidate_inclusive_contributing_actions,cce_verified_lower,cce_candidate_upper,verified_cross_agent_causal_edges,candidate_inclusive_cross_agent_causal_edges,verified_cross_agent_contributing_actions,candidate_inclusive_cross_agent_contributing_actions,message_action_count,verified_contributing_messages,candidate_inclusive_contributing_messages,message_effectiveness_verified_lower,message_effectiveness_candidate_upper,resource_transfer_action_count,duplicate_action_count,duplicate_fingerprint_groups,agent_id,agent_role,agent_action_count,pac_verified_lower,pac_candidate_upper,useful_effort_share_lower,useful_effort_share_upper,cause_action_id,effect_action_id,evidence_ref,evidence_kind,evidence_status,cross_agent"
    );

    for (fixture_id, succeeded) in [("successful_trace", true), ("failed_trace", false)] {
        let trace = trace(succeeded);
        let metrics = trace.score().expect("synthetic trace must validate");
        emit(summary_row(fixture_id, &metrics));

        for agent in &metrics.per_agent {
            let mut row = vec![String::new(); CSV_COLUMNS];
            row[0] = "agent".into();
            row[1] = fixture_id.into();
            row[2] = metrics.task_succeeded.to_string();
            row[23] = agent.agent_id.clone();
            row[24] = agent.role.clone();
            row[25] = agent.action_count.to_string();
            row[26] = fmt_opt(agent.pac_verified_lower);
            row[27] = fmt_opt(agent.pac_candidate_upper);
            row[28] = fmt_opt(agent.useful_effort_share_lower);
            row[29] = fmt_opt(agent.useful_effort_share_upper);
            emit(row);
        }

        for edge in &trace.causal_edges {
            let cause_agent = trace.actions.iter()
                .find(|action| action.action_id == edge.cause_action_id)
                .expect("causal source exists")
                .agent_id.as_str();
            let effect_agent = trace.actions.iter()
                .find(|action| action.action_id == edge.effect_action_id)
                .expect("causal target exists")
                .agent_id.as_str();
            let mut row = vec![String::new(); CSV_COLUMNS];
            row[0] = "causal_edge".into();
            row[1] = fixture_id.into();
            row[2] = metrics.task_succeeded.to_string();
            row[30] = edge.cause_action_id.clone();
            row[31] = edge.effect_action_id.clone();
            row[32] = edge.evidence_ref.clone();
            row[33] = evidence_kind_name(edge.evidence_kind).into();
            row[34] = match edge.status {
                CausalEdgeStatus::Verified => "verified",
                CausalEdgeStatus::Candidate => "candidate",
            }.into();
            row[35] = (cause_agent != effect_agent).to_string();
            emit(row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_contains_asymmetric_roles_and_blackbox_evidence_types() {
        let successful = trace(true);
        assert_eq!(successful.agents.len(), 5);
        assert_eq!(successful.actions.len(), 8);
        assert!(successful.causal_edges.iter().any(|edge| edge.evidence_kind == CausalEvidenceKind::ModelProposed));
        assert!(successful.causal_edges.iter().any(|edge| edge.evidence_kind == CausalEvidenceKind::ResourceLineageReceipt));
    }

    #[test]
    fn success_and_failure_fixtures_have_distinct_claim_semantics() {
        assert!(trace(true).task_succeeded);
        assert!(!trace(false).task_succeeded);
        assert_eq!(trace(true).success_action_ids, vec!["deliver"]);
        assert!(trace(false).success_action_ids.is_empty());
    }
}
