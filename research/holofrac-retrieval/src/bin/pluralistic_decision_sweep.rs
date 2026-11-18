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
    println!("metric,value");

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
    println!("decision_cutoff_micros,{}", report.decision_cutoff_micros);
    println!("active_stakeholders,{}", report.active_stakeholder_count);
    println!("excluded_profiles,{}", report.excluded_profile_count);
    println!("pareto_frontier,{}", report.pareto_frontier_ids.join(";"));
    println!("rank_mean,{}", report.rank_by_mean().join(";"));
    println!("rank_maximin,{}", report.rank_by_maximin().join(";"));
    println!("rank_leximin,{}", report.rank_by_leximin().join(";"));
    println!("# alternative,mean_satisfaction,minimum_satisfaction,individual_satisfaction");
    for assessment in &report.assessments {
        let per_stakeholder = assessment
            .per_stakeholder_satisfaction
            .iter()
            .map(|stakeholder| {
                format!(
                    "{}:{:.6}",
                    stakeholder.stakeholder_id.as_deref().unwrap_or("anonymous"),
                    stakeholder.satisfaction
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        println!(
            "{},{:.8},{:.8},{}",
            assessment.alternative_id,
            assessment.mean_satisfaction,
            assessment.minimum_satisfaction,
            per_stakeholder
        );
    }

    println!("# rejected alternatives");
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
            "{},hard_constraints_satisfied={},vetoes={}",
            rejected.alternative_id, rejected.hard_constraints_satisfied, reasons
        );
    }

    Ok(())
}
