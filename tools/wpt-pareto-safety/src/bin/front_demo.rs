use std::error::Error;

use wpt_pareto_safety::{
    Candidate, ConstraintSense, Disposition, EvaluationPlan, HardConstraint, Interval,
    Objective, ObjectiveDirection, evaluate_portfolio,
};

fn main() -> Result<(), Box<dyn Error>> {
    let plan = EvaluationPlan::new(
        vec![
            Objective::new("efficiency_percent", ObjectiveDirection::Maximize)?,
            Objective::new("load_power_w", ObjectiveDirection::Maximize)?,
            Objective::new("shield_loss_w", ObjectiveDirection::Minimize)?,
        ],
        vec![
            HardConstraint::new(
                "illustrative_exposure_screen",
                "magnetic_field_ua_per_m",
                ConstraintSense::AtMost,
                21.0,
                "A/m",
            )?,
            HardConstraint::new(
                "illustrative_temperature_screen",
                "temperature_rise_c",
                ConstraintSense::AtMost,
                40.0,
                "degC",
            )?,
        ],
    )?;

    let candidates = vec![
        Candidate::new("high-efficiency-unsafe", [
            ("efficiency_percent", Interval::new(98.5, 99.0)?),
            ("load_power_w", Interval::new(90_000.0, 95_000.0)?),
            ("shield_loss_w", Interval::new(1.0, 1.5)?),
            ("magnetic_field_ua_per_m", Interval::new(22.0, 23.0)?),
            ("temperature_rise_c", Interval::new(30.0, 35.0)?),
        ])?,
        Candidate::new("uncertain-exposure", [
            ("efficiency_percent", Interval::new(96.0, 97.0)?),
            ("load_power_w", Interval::new(75_000.0, 80_000.0)?),
            ("shield_loss_w", Interval::new(2.0, 3.0)?),
            ("magnetic_field_ua_per_m", Interval::new(20.0, 22.0)?),
            ("temperature_rise_c", Interval::new(30.0, 35.0)?),
        ])?,
        Candidate::new("feasible-a", [
            ("efficiency_percent", Interval::new(94.0, 95.0)?),
            ("load_power_w", Interval::new(65_000.0, 70_000.0)?),
            ("shield_loss_w", Interval::new(4.0, 5.0)?),
            ("magnetic_field_ua_per_m", Interval::new(15.0, 16.0)?),
            ("temperature_rise_c", Interval::new(30.0, 35.0)?),
        ])?,
        Candidate::new("feasible-b", [
            ("efficiency_percent", Interval::new(95.5, 96.0)?),
            ("load_power_w", Interval::new(70_000.0, 72_000.0)?),
            ("shield_loss_w", Interval::new(3.0, 3.5)?),
            ("magnetic_field_ua_per_m", Interval::new(17.0, 18.0)?),
            ("temperature_rise_c", Interval::new(32.0, 36.0)?),
        ])?,
        Candidate::new("missing-field", [
            ("efficiency_percent", Interval::new(95.0, 96.0)?),
            ("load_power_w", Interval::new(68_000.0, 71_000.0)?),
            ("shield_loss_w", Interval::new(4.0, 4.5)?),
            ("temperature_rise_c", Interval::new(32.0, 36.0)?),
        ])?,
    ];

    let report = evaluate_portfolio(&candidates, &plan)?;
    println!("candidate_id,disposition");
    for evaluation in &report.evaluations {
        let label = match evaluation.disposition {
            Disposition::Feasible => "Feasible",
            Disposition::Infeasible => "Infeasible",
            Disposition::Uncertain => "Uncertain",
            Disposition::NotEvaluable => "NotEvaluable",
        };
        println!("{},{}", evaluation.candidate_id, label);
    }
    println!("feasible_pareto_front");
    for id in report.feasible_pareto_front {
        println!("{id}");
    }
    Ok(())
}
