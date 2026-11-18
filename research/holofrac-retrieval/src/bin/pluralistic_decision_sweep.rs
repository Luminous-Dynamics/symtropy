use std::env;

use holofrac_spatial_retrieval::pluralistic::{
    evaluate_group, ConsentScope, ConsentStatus, DecisionAlternative, PreferenceEvent,
    PreferenceLedger, CRITERION_COUNT,
};

fn profile_event(
    event_id: u64,
    stakeholder_id: &str,
    scope: ConsentScope,
    status: ConsentStatus,
    weights: [f64; CRITERION_COUNT],
    veto_thresholds: [Option<f64>; CRITERION_COUNT],
) -> PreferenceEvent {
    PreferenceEvent {
        event_id,
        stakeholder_id: stakeholder_id.to_owned(),
        revision: 1,
        previous_event_id: None,
        effective_at_micros: 100,
        recorded_at_micros: 100,
        consent_scope: scope,
        consent_status: status,
        weights,
        veto_thresholds,
    }
}

fn alternative(
    alternative_id: &str,
    outcomes: [f64; CRITERION_COUNT],
    hard_constraints_satisfied: bool,
) -> DesignAlternative {
    DesignAlternative {
        alternative_id: alternative_id.to_owned(),
        outcomes,
        hard_constraints_satisfied,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac consent-first pluralistic decision benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);
    println!("row_type,alternative,metric,value,detail");

    let snapshot = PreferenceLedger { events }.snapshot_as_of(100)?;
    let alternatives = vec![
        alternative(
            "balanced",
            [0.75, 0.8, 0.7, 0.7, 0.85, 0.85, 0.65, 0.8],
            true,
        ),
        alternative(
            "energy-maximizing",
            [0.55, 0.65, 0.7, 0.65, 0.75, 0.5, 0.98, 0.55],
            true,
        ),
        alternative(
            "privacy-and-daylight",
            [0.7, 0.75, 0.95, 0.7, 0.8, 0.98, 0.55, 0.7],
            true,
        ),
        alternative(
            "adaptable-daylight",
            [0.65, 0.7, 0.98, 0.6, 0.8, 0.8, 0.6, 0.98],
            true,
        ),
        alternative(
            "hard-constraint-failure",
            [1.0; CRITERION_COUNT],
            false,
        ),
    ];

    let report = evaluate_group(&alternatives, &snapshot)?;
    println!(
        "summary,,decision_cutoff_micros,{},",
        report.decision_cutoff_micros
    );
    println!(
        "summary,,visible_preference_events,{},",
        snapshot.visible_event_count
    );
    println!(
        "summary,,active_stakeholders,{},",
        report.active_stakeholder_count
    );
    println!(
        "summary,,excluded_profiles,{},",
        report.excluded_profile_count
    );

    for (method, ranking) in [
        ("mean", report.rank_by_mean()),
        ("maximin", report.rank_by_maximin()),
        ("leximin", report.rank_by_leximin()),
    ] {
        for (rank, alternative_id) in ranking.iter().enumerate() {
            println!(
                "ranking,{alternative_id},{method},{},",
                rank + 1
            );
        }
    }

    for frontier_id in &report.pareto_frontier_ids {
        println!("pareto,{frontier_id},non_dominated,true,");
    }

    for assessment in &report.assessments {
        println!(
            "candidate,{},{:.8},mean_satisfaction,",
            assessment.alternative_id,
            assessment.mean_satisfaction
        );
        println!(
            "candidate,{},{:.8},minimum_satisfaction,",
            assessment.alternative_id,
            assessment.minimum_satisfaction
        );
        for stakeholder in &assessment.per_stakeholder_satisfaction {
            println!(
                "participant,{},{:.8},satisfaction,{}",
                assessment.alternative_id,
                stakeholder.satisfaction,
                stakeholder.stakeholder_id.as_deref().unwrap_or("anonymous")
            );
        }
    }

    for rejected in &report.rejected_alternatives {
        let reasons = rejected
            .veto_violations
            .iter()
            .map(|violation| {
                format!(
                    "{}:{}<{:.3}",
                    violation
                        .stakeholder_id
                        .as_deref()
                        .unwrap_or("anonymous"),
                    violation.criterion.as_str(),
                    violation.minimum_outcome
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        println!(
            "rejected,{},{},{},{}",
            rejected.alternative_id,
            "hard_constraints_satisfied",
            rejected.hard_constraints_satisfied,
            reasons
        );
    }
    Ok(())
}
