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
        expected_illuminance_points: 104,
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

    let mut compliant_grid = vec![520.0; 104];
    compliant_grid[0] = 315.0;
    let underlit_grid = vec![240.0; 104];

    let candidates = [
        CandidateObservation {
            candidate_id: "synthetic-compliant".to_owned(),
            energy_wh: 8.0,
            observed_hours: 1.0,
            compliant_task_hours: 1.0,
            peak_wall_power_w: 10.0,
            illuminance_samples_lx: compliant_grid,
            gates: all_pass.clone(),
        },
        CandidateObservation {
            candidate_id: "synthetic-underlit".to_owned(),
            energy_wh: 4.0,
            observed_hours: 1.0,
            compliant_task_hours: 0.0,
            peak_wall_power_w: 5.0,
            illuminance_samples_lx: underlit_grid,
            gates: all_pass,
        },
    ];

    println!(
        "candidate_id,status,average_wall_power_w,average_lx,minimum_lx,uniformity,energy_wh_per_compliant_task_hour,reason_codes"
    );
    for candidate in &candidates {
        let result = evaluate(candidate, &limits);
        let average_power = result
            .average_wall_power_w
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "null".to_owned());
        let (average_lx, minimum_lx, uniformity) = match result.illuminance {
            Some(grid) => (
                format!("{:.3}", grid.average_lx),
                format!("{:.3}", grid.minimum_lx),
                format!("{:.5}", grid.minimum_to_average_uniformity),
            ),
            None => ("null".to_owned(), "null".to_owned(), "null".to_owned()),
        };
        let energy_metric = result
            .energy_wh_per_compliant_task_hour
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "null".to_owned());
        let reasons = result.reason_codes.join(";");
        println!(
            "{},{},{},{},{},{},{},{}",
            result.candidate_id,
            result.status.as_str(),
            average_power,
            average_lx,
            minimum_lx,
            uniformity,
            energy_metric,
            reasons
        );
    }
}
