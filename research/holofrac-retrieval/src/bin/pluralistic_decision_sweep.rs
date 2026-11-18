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

    let mut privacy_veto = [None; CRITERION_COUNT];
    privacy_veto[5] = Some(0.7);
    let mut access_veto = [None; CRITERION_COUNT];
    access_veto[4] = Some(0.7);
    let mut comfort_veto = [None; CRITERION_COUNT];
    comfort_veto[0] = Some(0.6);

    let events = vec![
        profile_event(
            1,
            "synthetic:resident-thermal",
            ConsentScope::AnonymousAggregate,
            ConsentStatus::Granted,
            [0.4, 0.1, 0.1, 0.1, 0.1, 0.1, 0.05, 0.05],
            comfort_veto,
        ),
        profile_event(
            2,
            "synthetic:resident-privacy",
            ConsentScope::AnonymousAggregate,
            ConsentStatus::Granted,
            [0.1, 0.1, 0.05, 0.05, 0.05, 0.4, 0.1, 0.15],
            privacy_veto,
        ),
        profile_event(
            3,
            "synthetic:facility-operator",
            ConsentScope::AnonymousAggregate,
            ConsentStatus::Granted,
            [0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.4, 0.3],
            [None; CRITERION_COUNT],
        ),
        profile_event(
            4,
            "synthetic:accessibility-panel",
            ConsentScope::AnonymousAggregate,
            ConsentStatus::Granted,
            [0.1, 0.1, 0.05, 0.05, 0.45, 0.05, 0.05, 0.15],
            access_veto,
        ),
        profile_event(
            5,
            "synthetic:community",
            ConsentScope::AnonymousAggregate,
            ConsentStatus::Granted,
            [0.05, 0.15, 0.05, 0.25, 0.1, 0.05, 0.2, 0.15],
            [None; CRITERION_COUNT],
        ),
        profile_event(
            6,
            "synthetic:owner",
            ConsentScope::NamedAggregate,
            ConsentStatus::Granted,
            [0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.4, 0.3],
            [None; CRITERION_COUNT],
        ),
        profile_event(
            7,
            "synthetic:local-only",
            ConsentScope::LocalOnly,
            ConsentStatus::Granted,
            [0.125; CRITERION_COUNT],
            [None; CRITERION_COUNT],
        ),
        profile_event(
            8,
            "synthetic:withdrawn",
            ConsentScope::NamedAggregate,
            ConsentStatus::Withdrawn,
            [0.0; CRITERION_COUNT],
            [None; CRITERION_COUNT],
        ),
    ];

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
        "summary,,anonymous_group_minimum,{},qualified",
        holofrac_spatial_retrieval::pluralistic::MIN_ANONYMOUS_GROUP_SIZE
    );

    for (method, ranking) in [
        ("mean", report.rank_by_mean()),
        ("maximin", report.rank_by_maximin()),
        ("leximin", report.rank_by_leximin()),
    ] {
        for (rank, alternative_id) in ranking.iter().enumerate() {
            println!(
                "ranking,{alternative_id},{method},{rank},",
                rank = rank + 1
            );
        }
    }

    for frontier_id in &report.pareto_frontier_ids {
        println!("pareto,{frontier_id},non_dominated,true,");
    }

    for assessment in &report.assessments {
        println!(
            "candidate,{al},mean_satisfaction,{value:.8},",
            al = assessment.alternative_id,
            value = assessment.mean_satisfaction
        );
        println!(
            "candidate,{al},minimum_satisfaction,{value:.8},",
            al = assessment.alternative_id,
            value = assessment.minimum_satisfaction
        );
        for stakeholder in &assessment.per_stakeholder_satisfaction {
            println!(
                "participant,{al},satisfaction,{satisfaction:.8},{id}",
                al = assessment.alternative_id,
                satisfaction = stakeholder.satisfaction,
                id = stakeholder.stakeholder_id
            );
        }
    }

    for rejected in &report.rejected_alternatives {
        println!(
            "rejected,{id},hard_constraints_satisfied,{value},",
            id = rejected.alternative_id,
            value = rejected.hard_constraints_satisfied
        );
        for violation in &rejected.veto_violations {
            println!(
                "veto,{id},{criterion},{observed:.8},{stakeholder}:minimum={minimum:.8}",
                id = rejected.alternative_id,
                criterion = violation.criterion.as_str(),
                observed = violation.observed_outcome,
                stakeholder = violation.stakeholder_id,
                minimum = violation.minimum_outcome
            );
        }
        for criterion in &rejected.anonymous_veto_criteria {
            println!(
                "anonymous-veto,{id},{criterion},blocked,threshold-redacted",
                id = rejected.alternative_id,
                criterion = criterion.as_str()
            );
        }
    }

    Ok(())
}
