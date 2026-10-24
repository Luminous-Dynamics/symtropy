use lighting_benchmark::{
    evaluate, CandidateObservation, CaseLimits, GateResult, GateStatus,
};

fn main() {
    eprintln!("SYNTHETIC DEMONSTRATION ONLY — not a product or SOTA result");

    let limits = CaseLimits {
        average_illuminance_min_lx: 500.0,
        minimum_illuminance_min_lx: 300.0,
        minimum_to_average_uniformity_min: 0.6,
        maximum_peak_power_w: 15.0,
        maximum_average_power_w: 15.0,
        required_gate_names: vec![
            "photometry".to_owned(),
            "color_quality".to_owned(),
            "flicker".to_owned(),
            "thermal".to_owned(),
            "electrical_safety".to_owned(),
        ],
    };

    let all_pass = limits
        .required_gate_names
        .iter()
        .map(|name| GateResult {
            name: name.clone(),
            status: GateStatus::Pass,
        })
        .collect::<Vec<_>>();

    let candidates = [
        CandidateObservation {
            candidate_id: "synthetic-compliant".to_owned(),
            energy_wh: 8.0,
            observed_hours: 1.0,
            compliant_task_hours: 1.0,
            peak_wall_power_w: 10.0,
            average_illuminance_lx: 520.0,
            minimum_illuminance_lx: 310.0,
            minimum_to_average_uniformity: 0.62,
            gates: all_pass.clone(),
        },
        CandidateObservation {
            candidate_id: "synthetic-underlit".to_owned(),
            energy_wh: 4.0,
            observed_hours: 1.0,
            compliant_task_hours: 0.0,
            peak_wall_power_w: 5.0,
            average_illuminance_lx: 240.0,
            minimum_illuminance_lx: 90.0,
            minimum_to_average_uniformity: 0.35,
            gates: all_pass,
        },
    ];

    println!(
        "candidate_id,status,average_wall_power_w,energy_wh_per_compliant_task_hour,reason_codes"
    );
    for candidate in &candidates {
        let result = evaluate(candidate, &limits);
        let average_power = result
            .average_wall_power_w
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "null".to_owned());
        let energy_metric = result
            .energy_wh_per_compliant_task_hour
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "null".to_owned());
        let reasons = result.reason_codes.join(";");
        println!(
            "{},{},{},{},{}",
            result.candidate_id,
            result.status.as_str(),
            average_power,
            energy_metric,
            reasons
        );
    }
}
