use adaptive_evidence_acquisition::{
    AcquisitionBudget, AcquisitionCandidate, CandidateQuality, EvidenceTarget,
    QualifiedExistingPass, SelectionPolicy, plan_acquisition_with_policy,
};

fn main() {
    let targets = vec![
        EvidenceTarget {
            target_id: "module-01".to_owned(),
            required_independent_passes: 2,
            minimum_view_angle_separation_rad: 0.35,
            weight: 2.0,
        },
        EvidenceTarget {
            target_id: "module-02".to_owned(),
            required_independent_passes: 2,
            minimum_view_angle_separation_rad: 0.35,
            weight: 1.0,
        },
        EvidenceTarget {
            target_id: "module-03".to_owned(),
            required_independent_passes: 1,
            minimum_view_angle_separation_rad: 0.0,
            weight: 1.5,
        },
    ];
    let existing = vec![QualifiedExistingPass {
        target_id: "module-01".to_owned(),
        pass_id: "pass-a".to_owned(),
        view_angle_rad: 0.0,
    }];
    let candidates = vec![
        AcquisitionCandidate {
            candidate_id: "view-north".to_owned(),
            pass_id: "pass-b".to_owned(),
            u_m: 10.0,
            v_m: 2.0,
            view_angle_rad: 0.6,
            covered_target_ids: vec!["module-01".to_owned(), "module-02".to_owned()],
            quality: CandidateQuality::Qualified,
            acquisition_time_s: 3.0,
            energy_wh: 4.0,
            risk_penalty_s: 0.5,
        },
        AcquisitionCandidate {
            candidate_id: "view-east".to_owned(),
            pass_id: "pass-c".to_owned(),
            u_m: 5.0,
            v_m: 8.0,
            view_angle_rad: 1.1,
            covered_target_ids: vec!["module-01".to_owned(), "module-03".to_owned()],
            quality: CandidateQuality::Qualified,
            acquisition_time_s: 2.0,
            energy_wh: 2.0,
            risk_penalty_s: 0.0,
        },
        AcquisitionCandidate {
            candidate_id: "uncertain-view".to_owned(),
            pass_id: "pass-d".to_owned(),
            u_m: 3.0,
            v_m: 3.0,
            view_angle_rad: 0.9,
            covered_target_ids: vec!["module-02".to_owned()],
            quality: CandidateQuality::Unknown,
            acquisition_time_s: 1.0,
            energy_wh: 1.0,
            risk_penalty_s: 0.0,
        },
    ];
    let budget = AcquisitionBudget {
        start_u_m: 0.0,
        start_v_m: 0.0,
        travel_speed_m_per_s: 2.0,
        maximum_elapsed_time_s: 60.0,
        maximum_energy_wh: 20.0,
        energy_penalty_s_per_wh: 0.5,
    };

    let policies = [
        (
            "evidence_gain_per_cost",
            SelectionPolicy::EvidenceGainPerCost,
        ),
        ("nearest_feasible", SelectionPolicy::NearestFeasible),
        (
            "maximum_marginal_evidence",
            SelectionPolicy::MaximumMarginalEvidence,
        ),
        ("input_order_fixed_route", SelectionPolicy::InputOrder),
        (
            "seeded_hash_order",
            SelectionPolicy::SeededHashOrder { seed: 7 },
        ),
    ];

    println!("suite=ADAPTIVE-EVIDENCE-ACQUISITION-001");
    println!("candidate_pool_size={}", candidates.len());
    for (name, policy) in policies {
        match plan_acquisition_with_policy(&targets, &existing, &candidates, &budget, policy) {
            Ok(plan) => {
                let selected = plan
                    .steps
                    .iter()
                    .map(|step| step.candidate_id.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                println!(
                    "policy={} disposition={:?} selected={} completed_targets={}/{} elapsed_s={:.3} energy_wh={:.3} unknown_candidates={} invalid_candidates={}",
                    name,
                    plan.disposition,
                    selected,
                    plan.completed_targets,
                    plan.total_targets,
                    plan.cumulative_elapsed_time_s,
                    plan.cumulative_energy_wh,
                    plan.blockers.unknown,
                    plan.blockers.invalid_evidence,
                );
            }
            Err(error) => {
                eprintln!("policy={} planning_error={error}", name);
                std::process::exit(2);
            }
        }
    }
    println!(
        "note=matched synthetic candidates and budgets; selection policies do not alter qualification gates"
    );
}
