use std::process;

use civil_aero_benchmark::budget::LedgerOperation;
use civil_aero_benchmark::policy_benchmark::{
    CandidateAdmissionStatus, CandidateObservation, PolicyBenchmarkTask, evaluate_policy_benchmark,
    report_to_csv,
};

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn operation(
    replicate: &str,
    policy: &str,
    operation_id: &str,
    kind: &str,
    cost: f64,
    success: bool,
    output_byte: char,
) -> LedgerOperation {
    LedgerOperation {
        task_id: "airfoil-multipoint-smoke".to_owned(),
        replicate_id: replicate.to_owned(),
        policy_id: policy.to_owned(),
        operation_id: operation_id.to_owned(),
        operation_kind: kind.to_owned(),
        fidelity: if kind == "high_fidelity" {
            "high".to_owned()
        } else {
            "low".to_owned()
        },
        aerodynamic_condition_calls_attempted: if matches!(kind, "low_fidelity" | "high_fidelity") {
            6
        } else {
            0
        },
        aerodynamic_condition_calls_succeeded: if matches!(kind, "low_fidelity" | "high_fidelity")
            && success
        {
            6
        } else {
            0
        },
        cost_units: cost,
        budget_cap_units: 8.0,
        wall_seconds: cost * 10.0,
        cpu_seconds: cost * 8.0,
        success,
        output_sha256: if success {
            digest(output_byte)
        } else {
            "-".to_owned()
        },
        reason_code: if success {
            "-".to_owned()
        } else {
            "solver_nonconverged".to_owned()
        },
    }
}

struct CandidateFixture<'a> {
    replicate: &'a str,
    policy: &'a str,
    operation_id: &'a str,
    candidate_id: &'a str,
    output_byte: char,
    admission_status: CandidateAdmissionStatus,
    feasible: Option<bool>,
    objective_1: Option<f64>,
    objective_2: Option<f64>,
    receipt_byte: Option<char>,
}

fn candidate(fixture: CandidateFixture<'_>) -> CandidateObservation {
    let CandidateFixture {
        replicate,
        policy,
        operation_id,
        candidate_id,
        output_byte,
        admission_status,
        feasible,
        objective_1,
        objective_2,
        receipt_byte,
    } = fixture;
    CandidateObservation {
        task_id: "airfoil-multipoint-smoke".to_owned(),
        replicate_id: replicate.to_owned(),
        policy_id: policy.to_owned(),
        operation_id: operation_id.to_owned(),
        candidate_id: candidate_id.to_owned(),
        output_sha256: digest(output_byte),
        admission_status,
        admission_receipt_sha256: receipt_byte.map(digest).unwrap_or_else(|| "-".to_owned()),
        feasible,
        objective_1,
        objective_2,
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic policy benchmark rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let task = PolicyBenchmarkTask {
        task_id: "airfoil-multipoint-smoke".to_owned(),
        objective_1_name: "drag_coefficient".to_owned(),
        objective_2_name: "mission_energy_wh".to_owned(),
        ideal_point_1: 0.0,
        ideal_point_2: 0.0,
        reference_point_1: 0.05,
        reference_point_2: 1000.0,
    };
    let mut operations = Vec::new();
    let mut candidates = Vec::new();

    for replicate in ["seed-1", "seed-2"] {
        for (policy, costs, hashes, points) in [
            (
                "uniform",
                [0.5, 2.0, 2.5, 1.0, 0.75],
                ['a', 'b', 'c', 'd', 'e'],
                [(0.03, 600.0), (0.035, 500.0)],
            ),
            (
                "voi",
                [0.25, 1.75, 2.0, 0.5, 0.5],
                ['f', '0', '1', '2', '3'],
                [(0.028, 650.0), (0.032, 520.0)],
            ),
        ] {
            operations.push(operation(
                replicate,
                policy,
                "screen",
                "low_fidelity",
                costs[0],
                true,
                hashes[0],
            ));
            operations.push(operation(
                replicate,
                policy,
                "hf-1",
                "high_fidelity",
                costs[1],
                true,
                hashes[1],
            ));
            operations.push(operation(
                replicate,
                policy,
                "hf-2",
                "high_fidelity",
                costs[2],
                true,
                hashes[2],
            ));
            operations.push(operation(
                replicate,
                policy,
                "hf-rejected",
                "high_fidelity",
                costs[3],
                true,
                hashes[3],
            ));
            operations.push(operation(
                replicate,
                policy,
                "hf-failed",
                "high_fidelity",
                costs[4],
                false,
                hashes[4],
            ));

            candidates.push(candidate(CandidateFixture {
                replicate,
                policy,
                operation_id: "hf-1",
                candidate_id: "candidate-1",
                output_byte: hashes[1],
                admission_status: CandidateAdmissionStatus::Admitted,
                feasible: Some(true),
                objective_1: Some(points[0].0),
                objective_2: Some(points[0].1),
                receipt_byte: Some('6'),
            }));
            candidates.push(candidate(CandidateFixture {
                replicate,
                policy,
                operation_id: "hf-2",
                candidate_id: "candidate-2",
                output_byte: hashes[2],
                admission_status: CandidateAdmissionStatus::Admitted,
                feasible: Some(true),
                objective_1: Some(points[1].0),
                objective_2: Some(points[1].1),
                receipt_byte: Some('7'),
            }));
            candidates.push(candidate(CandidateFixture {
                replicate,
                policy,
                operation_id: "hf-rejected",
                candidate_id: "candidate-not-admitted",
                output_byte: hashes[3],
                admission_status: CandidateAdmissionStatus::NotAdmitted,
                feasible: None,
                objective_1: None,
                objective_2: None,
                receipt_byte: None,
            }));
        }
    }

    let report = evaluate_policy_benchmark(&[task], &operations, &candidates)?;
    eprintln!(
        "synthetic_policy_benchmark_only tasks=1 replicates=2 policies=2 rows={} high_fidelity_attempts_per_policy=4 high_fidelity_condition_calls_per_policy=24 high_fidelity_condition_calls_failed_per_policy=6 admitted_candidates_per_policy=2",
        report.rows.len()
    );
    print!("{}", report_to_csv(&report));
    Ok(())
}
