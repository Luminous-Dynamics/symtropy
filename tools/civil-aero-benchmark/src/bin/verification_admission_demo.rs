use std::process;

use civil_aero_benchmark::cfd_admission::{
    AdmissionCriteria, CfdEvidenceStudy, CfdGridRun, evaluate_cfd_evidence_admission, report_to_csv,
};
use civil_aero_benchmark::grid_convergence::GridFamilyKind;

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic CFD admission demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let spacings = [4.0_f64, 2.0, 1.0, 0.5, 0.25, 0.125];
    let runs = spacings
        .iter()
        .enumerate()
        .map(|(index, spacing)| {
            let value = 2.0 + 0.8 * spacing.powi(2);
            let run_number =
                f64::from(u32::try_from(index + 1).expect("six fixture grids fit u32"));
            CfdGridRun {
                run_id: format!("run-{}", index + 1),
                grid_id: format!("grid-{}", index + 1),
                grid_level: u32::try_from(index + 1).expect("six fixture grids fit u32"),
                characteristic_spacing: *spacing,
                mesh_sha256: digest(
                    char::from_digit(
                        u32::try_from(index + 1).expect("six fixture grids fit u32"),
                        10,
                    )
                    .expect("grid IDs use base-10 digits"),
                ),
                solver_build_sha256: digest('a'),
                physics_config_sha256: digest('b'),
                geometry_sha256: digest('c'),
                solver_exit_code: 0,
                iterations_completed: 5000 + u64::try_from(index).expect("index fits u64"),
                mesh_node_count: 10_000 * u64::try_from(index + 1).expect("index fits u64"),
                cpu_core_hours: 0.25 * run_number,
                gpu_hours: if index % 2 == 0 { Some(0.5) } else { None },
                wall_clock_seconds: 120.0 * run_number,
                peak_memory_bytes: 1024 * u64::try_from(index + 1).expect("index fits u64"),
                energy_consumed_joules: if index < 4 {
                    Some(1000.0 * run_number)
                } else {
                    None
                },
                residual_initial: 1.0e-1,
                residual_final: 1.0e-10,
                quantity_history: vec![value; 6],
                mass_imbalance_fraction: 1.0e-8,
                energy_imbalance_fraction: 2.0e-8,
                quantity_value: value,
            }
        })
        .collect();

    let study = CfdEvidenceStudy {
        case_id: "synthetic-v1a".to_owned(),
        quantity: "CD".to_owned(),
        source_reference: "synthetic-known-answer-not-cfd".to_owned(),
        spacing_definition: "explicit synthetic characteristic spacing".to_owned(),
        grid_family_kind: GridFamilyKind::Structured,
        safety_factor: 1.25,
        criteria: AdmissionCriteria {
            residual_final_max: 1.0e-8,
            residual_reduction_ratio_max: 1.0e-7,
            quantity_history_drift_absolute_max: 1.0e-9,
            mass_imbalance_fraction_max: 1.0e-6,
            energy_imbalance_fraction_max: 1.0e-6,
            observed_order_min: 1.0,
            observed_order_max: 3.0,
            asymptotic_ratio_min: 0.9,
            asymptotic_ratio_max: 1.1,
        },
        runs,
    };

    let report = evaluate_cfd_evidence_admission(&study)?;
    eprintln!(
        "synthetic_cfd_admission_known_answer_only status={} grids={} triplets={} cpu_core_hours={:.6} gpu_hours_measured={} wall_seconds={:.3}",
        report.status,
        report.grid_run_audits.len(),
        report.gci_report.triplets.len(),
        report.total_cpu_core_hours,
        report.gpu_measured_run_count,
        report.total_wall_clock_seconds
    );
    print!("{}", report_to_csv(&report)?);
    Ok(())
}
