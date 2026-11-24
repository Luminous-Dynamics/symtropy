// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic CSV tournament for temporal normal-wrench transfer policies.

use temporal_wrench_transfer::{
    NewContactCandidate, PreviousNormalImpulse, transfer_equal_distribution,
    transfer_normal_impulse,
};

fn old(id: &str, x: f64, y: f64, impulse: f64) -> PreviousNormalImpulse {
    PreviousNormalImpulse {
        contact_id: id.into(),
        position: [x, y],
        normal_impulse: impulse,
    }
}

fn new(id: &str, x: f64, y: f64) -> NewContactCandidate {
    NewContactCandidate {
        contact_id: id.into(),
        position: [x, y],
    }
}

fn emit(scenario: &str, policy: &str, report: temporal_wrench_transfer::TemporalTransferReport) {
    let ids = report
        .retained_contact_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("|");
    println!(
        "planar,{scenario},{policy},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{}",
        report.total_normal_impulse,
        report.transferred_normal_impulse,
        report.force_error,
        report.center_of_pressure_error_norm,
        report.moment_error_norm,
        report.impulse_sum_error,
        ids
    );
}

fn run_scenario(
    name: &str,
    previous: &[PreviousNormalImpulse],
    next: &[NewContactCandidate],
) -> Result<(), Box<dyn std::error::Error>> {
    let equal = transfer_equal_distribution(previous, next, 0.8)?;
    emit(name, "scalar-cache-equal-0.8", equal);

    let transport = transfer_normal_impulse(previous, next)?;
    emit(name, "nonnegative-CoP-transport", transport);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "model,scenario,policy,source_normal_impulse,transferred_normal_impulse,force_error,cop_error,moment_error,impulse_sum_error,retained_contact_ids"
    );

    let old_balanced = vec![
        old("old-left", -1.0, 0.0, 1.0),
        old("old-right", 1.0, 0.0, 1.0),
    ];
    let new_shifted = vec![new("new-left", -1.0, 0.0), new("new-right", 3.0, 0.0)];
    run_scenario("shifted-new-manifold", &old_balanced, &new_shifted)?;

    let old_outside = vec![old("old-outside", 2.0, 0.0, 2.0)];
    let new_bounded = vec![new("new-left", -1.0, 0.0), new("new-right", 1.0, 0.0)];
    run_scenario("old-CoP-outside-new-hull", &old_outside, &new_bounded)?;
    Ok(())
}
