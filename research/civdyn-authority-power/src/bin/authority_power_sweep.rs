// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic weighted-voting and procedural authority audit sweep.

use civdyn_authority_power::{
    AuthorityAssignment, AuthorityPolicy, AuthorityRole, ProposalDecisionTrace, WeightedCouncil,
    analyze_voting_power, audit_authority_topology, audit_decision_trace,
};

const CSV_COLUMNS: usize = 12;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "authority-power CSV schema must remain fixed"
    );
    println!("{}", fields.join(","));
}

fn empty_row(record_type: &str, case_id: &str, subject: &str) -> Vec<String> {
    vec![
        record_type.into(),
        case_id.into(),
        subject.into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]
}

fn emit_council(case_id: &str, weights: &[u64], quota: u64) {
    let council = WeightedCouncil {
        actor_ids: (0..weights.len())
            .map(|index| format!("{case_id}_actor_{index}"))
            .collect(),
        weights: weights.to_vec(),
        quota,
    };
    let report = analyze_voting_power(&council).expect("fixed weighted council must validate");

    for (metric, value, unit) in [
        (
            "nominal_weight_hhi",
            fmt(report.nominal_weight_hhi),
            "index",
        ),
        (
            "normalized_banzhaf_hhi",
            fmt(report.normalized_banzhaf_hhi),
            "index",
        ),
        (
            "nominal_weight_gini",
            fmt(report.nominal_weight_gini),
            "index",
        ),
        (
            "normalized_banzhaf_gini",
            fmt(report.normalized_banzhaf_gini),
            "index",
        ),
        (
            "winning_coalition_count",
            report.winning_coalition_count.to_string(),
            "count",
        ),
        (
            "minimal_winning_coalition_count",
            report.minimal_winning_coalition_count.to_string(),
            "count",
        ),
        (
            "enumerated_coalition_count",
            report.enumerated_coalition_count.to_string(),
            "count",
        ),
    ] {
        let mut row = empty_row("council_metric", case_id, "council");
        row[3] = metric.into();
        row[4] = value;
        row[5] = unit.into();
        row[6] = report.total_weight.to_string();
        row[7] = report.quota.to_string();
        emit(row);
    }

    for actor in &report.actors {
        let mut row = empty_row("voting_actor", case_id, &actor.actor_id);
        row[3] = "effective_power".into();
        row[4] = fmt(actor.normalized_banzhaf);
        row[5] = "normalized_banzhaf".into();
        row[6] = actor.nominal_weight.to_string();
        row[7] = fmt(actor.nominal_weight_share);
        row[8] = fmt(actor.normalized_banzhaf);
        row[9] = fmt(actor.shapley_shubik);
        row[10] = format!(
            "veto={};dictator={};removal_winnable={}",
            actor.veto_player, actor.dictator, actor.council_remains_winnable_after_removal
        );
        emit(row);
    }

    for (index, coalition) in report.minimal_winning_coalitions.iter().enumerate() {
        let mut row = empty_row(
            "minimal_winning_coalition",
            case_id,
            &format!("coalition_{index}"),
        );
        row[3] = "members".into();
        row[4] = coalition.total_weight.to_string();
        row[5] = "total_weight".into();
        row[11] = coalition.actor_ids.join("|");
        emit(row);
    }
}

fn emit_topology(case_id: &str, assignments: &[AuthorityAssignment]) {
    let actors = ["alice", "bob", "carol", "dave", "erin"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let report = audit_authority_topology(
        &actors,
        assignments,
        &AuthorityPolicy::public_governance_reference(),
    )
    .expect("fixed authority topology must validate");

    for coverage in &report.role_coverage {
        let mut row = empty_row("role_coverage", case_id, coverage.role.name());
        row[3] = "holder_count".into();
        row[4] = coverage.holder_ids.len().to_string();
        row[5] = "count".into();
        row[10] = if coverage.holder_ids.len() == 1 {
            "single_holder".into()
        } else {
            String::new()
        };
        row[11] = coverage.holder_ids.join("|");
        emit(row);
    }
    for conflict in &report.conflicts {
        let mut row = empty_row("role_conflict", case_id, &conflict.actor_id);
        row[3] = format!("{}+{}", conflict.first.name(), conflict.second.name());
        row[4] = conflict.severity.name().into();
        row[5] = "severity".into();
        row[10] = conflict.rationale.clone();
        emit(row);
    }

    for (metric, value) in [
        (
            "forbidden_conflict_count",
            report.forbidden_conflict_count.to_string(),
        ),
        (
            "advisory_conflict_count",
            report.advisory_conflict_count.to_string(),
        ),
        (
            "missing_required_role_count",
            report.missing_required_roles.len().to_string(),
        ),
        (
            "appeal_independence",
            report
                .appeal_independence
                .map_or_else(|| "unassessed".into(), |value| value.to_string()),
        ),
        (
            "compliant_with_declared_policy",
            report.compliant_with_declared_policy.to_string(),
        ),
    ] {
        let mut row = empty_row("topology_metric", case_id, "policy");
        row[3] = metric.into();
        row[4] = value;
        emit(row);
    }
}

fn safe_assignments() -> Vec<AuthorityAssignment> {
    vec![
        AuthorityAssignment {
            actor_id: "alice".into(),
            roles: vec![AuthorityRole::AgendaController],
        },
        AuthorityAssignment {
            actor_id: "bob".into(),
            roles: vec![AuthorityRole::Summarizer],
        },
        AuthorityAssignment {
            actor_id: "carol".into(),
            roles: vec![AuthorityRole::PublicRecordEditor],
        },
        AuthorityAssignment {
            actor_id: "dave".into(),
            roles: vec![
                AuthorityRole::Approver,
                AuthorityRole::EvidenceIssuer,
                AuthorityRole::OriginalAdjudicator,
            ],
        },
        AuthorityAssignment {
            actor_id: "erin".into(),
            roles: vec![
                AuthorityRole::EvidenceVerifier,
                AuthorityRole::Disburser,
                AuthorityRole::Auditor,
                AuthorityRole::AppealAdjudicator,
            ],
        },
    ]
}

fn capture_assignments() -> Vec<AuthorityAssignment> {
    vec![
        AuthorityAssignment {
            actor_id: "alice".into(),
            roles: vec![
                AuthorityRole::AgendaController,
                AuthorityRole::Summarizer,
                AuthorityRole::PublicRecordEditor,
                AuthorityRole::EvidenceIssuer,
                AuthorityRole::EvidenceVerifier,
                AuthorityRole::Approver,
                AuthorityRole::Disburser,
                AuthorityRole::Auditor,
                AuthorityRole::OriginalAdjudicator,
                AuthorityRole::AppealAdjudicator,
                AuthorityRole::RuleAmender,
            ],
        },
        AuthorityAssignment {
            actor_id: "bob".into(),
            roles: vec![AuthorityRole::Proposer],
        },
        AuthorityAssignment {
            actor_id: "carol".into(),
            roles: vec![],
        },
        AuthorityAssignment {
            actor_id: "dave".into(),
            roles: vec![],
        },
        AuthorityAssignment {
            actor_id: "erin".into(),
            roles: vec![],
        },
    ]
}

fn emit_decision_trace(case_id: &str, trace: ProposalDecisionTrace) {
    let actors = ["alice", "bob", "carol", "dave", "erin"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let audit =
        audit_decision_trace(&trace, &actors).expect("fixed proposal decision trace must validate");
    for conflict in &audit.conflicts {
        let mut row = empty_row("decision_conflict", case_id, &trace.proposal_id);
        row[3] = conflict.name().into();
        row[4] = "detected".into();
        row[5] = "status".into();
        row[10] = "reference separation-of-duties check".into();
        emit(row);
    }
    let mut row = empty_row("decision_trace_metric", case_id, &trace.proposal_id);
    row[3] = "compliant_with_reference_separation".into();
    row[4] = audit.compliant_with_reference_separation.to_string();
    emit(row);
}

fn main() {
    println!(
        "record_type,case_id,subject,metric,value,unit,total_weight_or_weight,quota_or_weight_share,normalized_banzhaf,shapley_shubik,flag_or_rationale,details"
    );

    emit_council("balanced_five", &[2, 2, 2, 2, 2], 6);
    emit_council("asymmetric_five", &[40, 25, 20, 10, 5], 51);
    emit_council("dictatorial_five", &[51, 13, 13, 13, 10], 51);

    emit_topology("separated_roles", &safe_assignments());
    emit_topology("concentrated_capture", &capture_assignments());

    emit_decision_trace(
        "proposal_separated",
        ProposalDecisionTrace {
            proposal_id: "proposal_safe".into(),
            proposer_id: "bob".into(),
            approver_id: Some("dave".into()),
            disburser_id: Some("erin".into()),
            evidence_issuer_ids: vec!["dave".into()],
            evidence_verifier_ids: vec!["erin".into()],
            original_adjudicator_id: Some("dave".into()),
            appeal_adjudicator_id: Some("erin".into()),
        },
    );
    emit_decision_trace(
        "proposal_capture",
        ProposalDecisionTrace {
            proposal_id: "proposal_captured".into(),
            proposer_id: "alice".into(),
            approver_id: Some("alice".into()),
            disburser_id: Some("alice".into()),
            evidence_issuer_ids: vec!["alice".into()],
            evidence_verifier_ids: vec!["alice".into()],
            original_adjudicator_id: Some("alice".into()),
            appeal_adjudicator_id: Some("alice".into()),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_council_fixtures_include_balanced_asymmetric_and_dictatorial_games() {
        assert_eq!(
            analyze_voting_power(&WeightedCouncil {
                actor_ids: (0..5).map(|i| format!("a{i}")).collect(),
                weights: vec![2, 2, 2, 2, 2],
                quota: 6,
            })
            .unwrap()
            .actors
            .len(),
            5
        );
    }

    #[test]
    fn safe_role_fixture_has_independent_appeal_and_no_forbidden_conflicts() {
        let actors = ["alice", "bob", "carol", "dave", "erin"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let report = audit_authority_topology(
            &actors,
            &safe_assignments(),
            &AuthorityPolicy::public_governance_reference(),
        )
        .unwrap();
        assert!(report.compliant_with_declared_policy);
        assert_eq!(report.appeal_independence, Some(true));
    }

    #[test]
    fn captured_fixture_is_not_compliant() {
        let actors = ["alice", "bob", "carol", "dave", "erin"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let report = audit_authority_topology(
            &actors,
            &capture_assignments(),
            &AuthorityPolicy::public_governance_reference(),
        ).unwrap();
        assert!(!report.compliant_with_declared_policy);
        assert!(report.forbidden_conflict_count >= 3);
        assert_eq!(report.appeal_independence, Some(false));
    }
}
