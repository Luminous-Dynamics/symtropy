// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact small-council power indices and authority-topology synthetic sweep.

use civdyn_authority_audit::{
    ActorPermissions, Capability, GovernanceTopology, SeparationRule, VotingActor,
    WeightedCouncil,
};
use std::collections::BTreeSet;

const CSV_COLUMNS: usize = 23;

fn council(scenario: &str) -> WeightedCouncil {
    let (weights, quota): (Vec<(&str, u64)>, u64) = match scenario {
        "equal_majority" => (vec![("a", 1), ("b", 1), ("c", 1)], 2),
        "weighted_supermajority" => (vec![("large", 2), ("small_a", 1), ("small_b", 1)], 3),
        "concentrated_majority" => (vec![("majority", 51), ("minority", 49)], 51),
        _ => unreachable!("fixed scenario name"),
    };
    WeightedCouncil {
        actors: weights.into_iter().map(|(id, weight)| VotingActor { id: id.into(), weight }).collect(),
        quota,
    }
}

fn blank(kind: &str, scenario: &str) -> Vec<String> {
    let mut row = vec![String::new(); CSV_COLUMNS];
    row[0] = kind.into();
    row[1] = scenario.into();
    row
}

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(row: Vec<String>) {
    assert_eq!(row.len(), CSV_COLUMNS, "authority audit CSV schema must remain fixed");
    println!("{}", row.join(","));
}

fn emit_council(scenario: &str) {
    let council = council(scenario);
    let report = council.audit().expect("reference council must validate");
    for actor in &report.actors {
        let mut row = blank("actor_power", scenario);
        row[2] = actor.actor_id.clone();
        row[3] = actor.nominal_weight.to_string();
        row[4] = fmt(actor.nominal_weight_share);
        row[5] = actor.raw_banzhaf_swings.to_string();
        row[6] = fmt(actor.normalized_banzhaf_power);
        row[7] = fmt(actor.shapley_shubik_power);
        row[8] = actor.power_to_weight_ratio.map_or_else(String::new, fmt);
        row[9] = actor.veto_player.to_string();
        row[10] = actor.dictator.to_string();
        row[12] = fmt(report.nominal_weight_hhi);
        row[13] = fmt(report.normalized_banzhaf_hhi);
        row[14] = fmt(report.shapley_shubik_hhi);
        row[15] = fmt(report.nominal_weight_gini);
        row[16] = fmt(report.normalized_banzhaf_gini);
        emit(row);
    }

    for coalition in &report.minimal_winning_coalitions {
        let mut row = blank("minimal_winning_coalition", scenario);
        row[3] = coalition.total_weight.to_string();
        row[11] = coalition.actor_ids.join("|");
        emit(row);
    }

    let mut summary = blank("council_summary", scenario);
    summary[3] = report.total_weight.to_string();
    summary[4] = report.quota.to_string();
    summary[12] = fmt(report.nominal_weight_hhi);
    summary[13] = fmt(report.normalized_banzhaf_hhi);
    summary[14] = fmt(report.shapley_shubik_hhi);
    summary[15] = fmt(report.nominal_weight_gini);
    summary[16] = fmt(report.normalized_banzhaf_gini);
    emit(summary);
}

fn topology() -> GovernanceTopology {
    let actors = vec![
        ActorPermissions {
            actor_id: "central_actor".into(),
            capabilities: BTreeSet::from([
                Capability::Propose,
                Capability::ControlAgenda,
                Capability::EditPublicRecord,
                Capability::IssueEvidence,
                Capability::VerifyEvidence,
                Capability::Approve,
                Capability::Disburse,
                Capability::AdjudicateInitial,
            ]),
        },
        ActorPermissions {
            actor_id: "appeal_member".into(),
            capabilities: BTreeSet::from([Capability::AdjudicateAppeal, Capability::Audit]),
        },
        ActorPermissions {
            actor_id: "community_member".into(),
            capabilities: BTreeSet::from([Capability::Propose, Capability::Summarize]),
        },
    ];
    let separation_rules = vec![
        SeparationRule { rule_id: "proposal_approval".into(), first: Capability::Propose, second: Capability::Approve },
        SeparationRule { rule_id: "agenda_record".into(), first: Capability::ControlAgenda, second: Capability::EditPublicRecord },
        SeparationRule { rule_id: "evidence_verification".into(), first: Capability::IssueEvidence, second: Capability::VerifyEvidence },
        SeparationRule { rule_id: "approval_disbursement".into(), first: Capability::Approve, second: Capability::Disburse },
    ];
    GovernanceTopology {
        actors,
        separation_rules,
        initial_adjudicator_ids: BTreeSet::from(["central_actor".into()]),
        final_appeal_adjudicator_id: Some("appeal_member".into()),
    }
}

fn emit_topology() {
    let report = topology().audit().expect("reference topology must validate");
    for conflict in &report.role_conflicts {
        let mut row = blank("role_conflict", "captured_topology");
        row[2] = conflict.actor_id.clone();
        row[17] = format!("{}:{}", conflict.first.name(), conflict.second.name());
        row[18] = conflict.rule_id.clone().len().to_string();
        row[20] = report.appeal_channel_available.to_string();
        row[21] = report.appeal_adjudicator_independent.to_string();
        row[22] = report.declared_policy_satisfied.to_string();
        emit(row);
    }
    for item in &report.capability_concentration {
        let mut row = blank("capability_concentration", "captured_topology");
        row[17] = item.capability.name().into();
        row[18] = item.holder_count.to_string();
        row[19] = item.single_point_of_control.to_string();
        row[20] = report.appeal_channel_available.to_string();
        row[21] = report.appeal_adjudicator_independent.to_string();
        row[22] = report.declared_policy_satisfied.to_string();
        emit(row);
    }
    let mut summary = blank("topology_summary", "captured_topology");
    summary[5] = report.role_conflicts.len().to_string();
    summary[20] = report.appeal_channel_available.to_string();
    summary[21] = report.appeal_adjudicator_independent.to_string();
    summary[22] = report.declared_policy_satisfied.to_string();
    emit(summary);
}

fn main() {
    println!(
        "record_type,scenario,actor_id,weight_or_total,nominal_share_or_quota,raw_banzhaf_swings,normalized_banzhaf_power,shapley_shubik_power,power_to_weight_ratio,veto_player,dictator,coalition_members,nominal_weight_hhi,banzhaf_hhi,shapley_shubik_hhi,nominal_weight_gini,banzhaf_gini,capability_or_conflict,holder_count_or_rule_len,single_point_of_control,appeal_channel_available,appeal_adjudicator_independent,declared_policy_satisfied"
    );
    for scenario in ["equal_majority", "weighted_supermajority", "concentrated_majority"] {
        emit_council(scenario);
    }
    emit_topology();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_reference_councils_preserve_hand_calculated_power() {
        let equal = council("equal_majority").audit().unwrap();
        assert!(equal.actors.iter().all(|actor| (actor.normalized_banzhaf_power - 1.0/3.0).abs() < 1e-12));

        let weighted = council("weighted_supermajority").audit().unwrap();
        assert!((weighted.actors[0].normalized_banzhaf_power - 0.6).abs() < 1e-12);
        assert!((weighted.actors[0].shapley_shubik_power - 2.0/3.0).abs() < 1e-12);

        let concentrated = council("concentrated_majority").audit().unwrap();
        assert!(concentrated.actors[0].dictator);
    }

    #[test]
    fn role_conflicts_do_not_hide_independent_appeal() {
        let report = topology().audit().unwrap();
        assert_eq!(report.role_conflicts.len(), 4);
        assert!(report.appeal_channel_available);
        assert!(report.appeal_adjudicator_independent);
        assert!(!report.declared_policy_satisfied);
    }
}
