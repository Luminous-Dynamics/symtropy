use adaptive_evidence_acquisition::{
    AcquisitionBudget, AcquisitionCandidate, CandidateQuality, EvidenceTarget,
    QualifiedExistingPass, plan_acquisition,
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

    match plan_acquisition(&targets, &existing, &candidates, &budget) {
        Ok(plan) => {
            println!("suite=ADAPTIVE-EVIDENCE-ACQUISITION-001");
            println!("disposition={:?}", plan.disposition);
            println!("selected_steps={}", plan.steps.len());
            for step in &plan.steps {
                println!(
                    "step={} candidate={} pass={} new_targets={} elapsed_s={:.3} energy_wh={:.3} score_per_s={:.6}",
                    step.ordinal,
                    step.candidate_id,
                    step.pass_id,
                    step.marginal_target_ids.join(","),
                    step.cumulative_elapsed_time_s,
                    step.cumulative_energy_wh,
                    step.utility_score_per_s
                );
            }
            println!("completed_targets={}/{}", plan.completed_targets, plan.total_targets);
            println!("unknown_candidates={}", plan.blockers.unknown);
            println!("invalid_candidates={}", plan.blockers.invalid_evidence);
            println!("note=synthetic candidate plan; not a robot motion or safety authorization");
        }
        Err(error) => {
            eprintln!("planning_error={error}");
            std::process::exit(2);
        }
    }
}
