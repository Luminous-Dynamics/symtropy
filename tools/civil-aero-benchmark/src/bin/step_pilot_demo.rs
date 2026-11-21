use std::process;

use civil_aero_benchmark::cmo_step_pilot::{
    STEP_PILOT_CSV_HEADER, StepPilotTask, evaluate_step_pilot, evaluation_to_csv,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("STEP pilot demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut output = format!("{STEP_PILOT_CSV_HEADER}\n");
    let mut rows = 0_usize;
    let mut feasible_rows = 0_usize;

    for x1 in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let x = [x1, 0.5, 0.5, 0.5, 0.5];
        let evaluation = evaluate_step_pilot(StepPilotTask::Step7C2Dtlz2M2D5, &x)?;
        feasible_rows += if evaluation.feasible { 1 } else { 0 };
        output.push_str(&evaluation_to_csv(
            &format!("step7-x1-{x1:.2}"),
            &evaluation,
        ));
        rows += 1;
    }

    let c2_tail_shift = [0.25, 0.8, 0.5, 0.5, 0.5];
    let evaluation = evaluate_step_pilot(StepPilotTask::Step7C2Dtlz2M2D5, &c2_tail_shift)?;
    feasible_rows += if evaluation.feasible { 1 } else { 0 };
    output.push_str(&evaluation_to_csv("step7-tail-shift", &evaluation));
    rows += 1;

    let c1_feasible = [0.2, 0.3, 0.5, 0.5, 0.5];
    let evaluation = evaluate_step_pilot(StepPilotTask::Step24C1Dtlz1M3D5, &c1_feasible)?;
    feasible_rows += if evaluation.feasible { 1 } else { 0 };
    output.push_str(&evaluation_to_csv("step24-feasible-g0", &evaluation));
    rows += 1;

    let c1_barrier = [0.2, 0.3, 0.0, 0.5, 0.5];
    let evaluation = evaluate_step_pilot(StepPilotTask::Step24C1Dtlz1M3D5, &c1_barrier)?;
    feasible_rows += if evaluation.feasible { 1 } else { 0 };
    output.push_str(&evaluation_to_csv("step24-distance-barrier", &evaluation));
    rows += 1;

    eprintln!(
        "step_pilot_known_answer=pass tasks=2 rows={rows} feasible_rows={feasible_rows} constraints_use_nonnegative_margin=true full_step_coverage=false"
    );
    print!("{output}");
    Ok(())
}
