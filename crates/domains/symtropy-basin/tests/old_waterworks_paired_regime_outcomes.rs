//! Paired finite-horizon institutional/environmental scenario checks.
//!
//! These tests exercise the existing deterministic Old Waterworks model as
//! an applied fixture. They do not claim that this synthetic scenario is an
//! empirically calibrated water system or that a 20-step trajectory proves
//! asymptotic stability, resilience or an attractor.

use symtropy_basin::{BasinIntervention, BasinMetrics, OldWaterworksScenario};

const WIDTH: usize = 12;
const HEIGHT: usize = 8;
const HORIZON: usize = 20;

fn run_paired_arm(intervention: BasinIntervention) -> Vec<BasinMetrics> {
    let mut scenario = OldWaterworksScenario::new(WIDTH, HEIGHT);

    // Common initial conditions and common exogenous shock.
    scenario.apply(BasinIntervention::PipeLeak);

    // Only the intervention differs across paired arms.
    scenario.apply(intervention);
    scenario.run_steps(HORIZON);

    scenario
        .records()
        .iter()
        .map(|record| record.basin)
        .collect()
}

fn assert_finite_outcome(metrics: BasinMetrics) {
    let values = [
        metrics.water_trust,
        metrics.toxin_load,
        metrics.standing_water,
        metrics.infrastructure_integrity,
        metrics.recovery_momentum,
        metrics.extinction_debt,
        metrics.viability,
        metrics.signal_corruption,
    ];
    assert!(values.iter().all(|value| value.is_finite()));
}

#[test]
fn identical_world_shock_and_intervention_replay_exactly() {
    let first = run_paired_arm(BasinIntervention::EcologicalReroute);
    let replay = run_paired_arm(BasinIntervention::EcologicalReroute);

    assert_eq!(first.len(), HORIZON);
    assert_eq!(first, replay);
}

#[test]
fn alternative_interventions_are_compared_on_the_same_world_and_clock() {
    let fast_repair = run_paired_arm(BasinIntervention::FastMechanicalRepair);
    let ecological_reroute = run_paired_arm(BasinIntervention::EcologicalReroute);

    assert_eq!(fast_repair.len(), HORIZON);
    assert_eq!(ecological_reroute.len(), HORIZON);

    for (fast, ecological) in fast_repair.iter().zip(&ecological_reroute) {
        assert_eq!(fast.tick, ecological.tick);
        assert_finite_outcome(*fast);
        assert_finite_outcome(*ecological);
    }

    // The policies must not be compared through a single aggregate score.
    // At least one named outcome dimension must expose the policy difference.
    let fast_final = fast_repair.last().expect("HORIZON is non-zero");
    let ecological_final = ecological_reroute.last().expect("HORIZON is non-zero");
    let differs = fast_final.water_trust != ecological_final.water_trust
        || fast_final.toxin_load != ecological_final.toxin_load
        || fast_final.infrastructure_integrity != ecological_final.infrastructure_integrity
        || fast_final.recovery_momentum != ecological_final.recovery_momentum
        || fast_final.extinction_debt != ecological_final.extinction_debt
        || fast_final.viability != ecological_final.viability
        || fast_final.signal_corruption != ecological_final.signal_corruption;

    assert!(
        differs,
        "distinct intervention arms unexpectedly produced identical final outcome vectors"
    );
}

#[test]
fn finite_horizon_records_do_not_claim_asymptotic_regime_status() {
    let trajectory = run_paired_arm(BasinIntervention::EcologicalReroute);

    assert_eq!(trajectory.len(), HORIZON);
    assert!(
        trajectory
            .windows(2)
            .all(|pair| pair[0].tick < pair[1].tick)
    );

    // This fixture only establishes that the model emitted a finite trajectory.
    // It deliberately makes no convergence, basin or attractor assertion.
    assert!(trajectory.iter().all(|metrics| metrics.tick > 0));
}
