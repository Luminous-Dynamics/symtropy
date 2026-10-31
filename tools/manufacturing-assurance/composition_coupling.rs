//! Synthetic known-truth benchmark for composition-level co-simulation validity.
//!
//! Two thermally coupled bodies provide a closed-form oracle. The benchmark
//! separates physical conservation from numerical trajectory fidelity and
//! validates paired, time-qualified interface exchange receipts.
//!
//! Build and test:
//!   rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/composition_coupling.rs -o /tmp/mfg-coupling-tests
//!   /tmp/mfg-coupling-tests
//!
//! Build and run:
//!   rustc --edition=2021 -D warnings tools/manufacturing-assurance/composition_coupling.rs -o /tmp/mfg-coupling
//!   /tmp/mfg-coupling
//!
//! Claim ceiling: synthetic two-state thermal benchmark only. This is not an
//! FMI conformance suite, validated plant model, certified master algorithm,
//! real-time guarantee, or equipment-control authorization.

const CAPACITY_A_J_PER_K: f64 = 2.0;
const CAPACITY_B_J_PER_K: f64 = 5.0;
const CONDUCTANCE_W_PER_K: f64 = 0.8;
const INITIAL_A_K: f64 = 100.0;
const INITIAL_B_K: f64 = 20.0;
const DURATION_S: f64 = 2.0;
const ENERGY_TOLERANCE_J: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CouplingMode {
    SharedFluxJacobi,
    IndependentPortsOneStepStale,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ThermalState {
    body_a_k: f64,
    body_b_k: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SimulationMetrics {
    step_s: f64,
    steps: usize,
    max_energy_drift_j: f64,
    final_energy_drift_j: f64,
    rms_temperature_error_k: f64,
    max_temperature_error_k: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PortReceipt<'a> {
    exchange_id: &'a str,
    component_id: &'a str,
    peer_id: &'a str,
    timestamp_ms: i64,
    signed_energy_j: f64,
    unit: &'a str,
}

fn initial_energy_j() -> f64 {
    CAPACITY_A_J_PER_K * INITIAL_A_K + CAPACITY_B_J_PER_K * INITIAL_B_K
}

/// Exact solution of two lumped thermal capacities coupled by conductance.
/// There is no external heat input, so the capacity-weighted mean is constant.
fn analytic_state(time_s: f64) -> ThermalState {
    let total_capacity = CAPACITY_A_J_PER_K + CAPACITY_B_J_PER_K;
    let weighted_mean = initial_energy_j() / total_capacity;
    let decay_rate =
        CONDUCTANCE_W_PER_K * (1.0 / CAPACITY_A_J_PER_K + 1.0 / CAPACITY_B_J_PER_K);
    let temperature_difference =
        (INITIAL_A_K - INITIAL_B_K) * (-decay_rate * time_s).exp();

    ThermalState {
        body_a_k: weighted_mean
            + (CAPACITY_B_J_PER_K / total_capacity) * temperature_difference,
        body_b_k: weighted_mean
            - (CAPACITY_A_J_PER_K / total_capacity) * temperature_difference,
    }
}

fn total_energy_j(state: ThermalState) -> f64 {
    CAPACITY_A_J_PER_K * state.body_a_k + CAPACITY_B_J_PER_K * state.body_b_k
}

fn validate_step(step_s: f64) -> Result<usize, String> {
    if !step_s.is_finite() || step_s <= 0.0 {
        return Err("communication step must be finite and positive".to_string());
    }

    let steps_f = (DURATION_S / step_s).round();
    if !steps_f.is_finite() || steps_f < 1.0 || steps_f > 1_000_000.0 {
        return Err("communication step produces unsupported step count".to_string());
    }
    let steps = steps_f as usize;
    let covered_duration = steps as f64 * step_s;
    if (covered_duration - DURATION_S).abs() > 1.0e-10 {
        return Err("communication step must divide the benchmark duration".to_string());
    }
    Ok(steps)
}

fn simulate(mode: CouplingMode, step_s: f64) -> Result<SimulationMetrics, String> {
    let steps = validate_step(step_s)?;
    let initial_state = ThermalState {
        body_a_k: INITIAL_A_K,
        body_b_k: INITIAL_B_K,
    };
    let mut state = initial_state;
    let mut previously_published_a = INITIAL_A_K;
    let mut previously_published_b = INITIAL_B_K;
    let initial_energy = initial_energy_j();
    let mut max_energy_drift = 0.0_f64;
    let mut max_temperature_error = 0.0_f64;
    let mut squared_error_sum = 0.0_f64;
    let mut error_count = 0_usize;

    for step in 0..=steps {
        let time_s = step as f64 * step_s;
        let reference = analytic_state(time_s);
        let error_a = (state.body_a_k - reference.body_a_k).abs();
        let error_b = (state.body_b_k - reference.body_b_k).abs();
        max_temperature_error = max_temperature_error.max(error_a).max(error_b);
        squared_error_sum += error_a * error_a + error_b * error_b;
        error_count += 2;

        let energy_drift = (total_energy_j(state) - initial_energy).abs();
        max_energy_drift = max_energy_drift.max(energy_drift);

        if step == steps {
            break;
        }

        // The one-step-stale baseline models independently advanced ports that
        // use the peer's last published value rather than one shared transfer.
        // This is deliberately a negative control, not a recommended algorithm.
        let old_a = state.body_a_k;
        let old_b = state.body_b_k;
        match mode {
            CouplingMode::SharedFluxJacobi => {
                let transfer_j = CONDUCTANCE_W_PER_K
                    * (state.body_a_k - state.body_b_k)
                    * step_s;
                state.body_a_k -= transfer_j / CAPACITY_A_J_PER_K;
                state.body_b_k += transfer_j / CAPACITY_B_J_PER_K;
            }
            CouplingMode::IndependentPortsOneStepStale => {
                let transfer_from_a_j = CONDUCTANCE_W_PER_K
                    * (state.body_a_k - previously_published_b)
                    * step_s;
                let transfer_to_b_j = CONDUCTANCE_W_PER_K
                    * (previously_published_a - state.body_b_k)
                    * step_s;
                state.body_a_k -= transfer_from_a_j / CAPACITY_A_J_PER_K;
                state.body_b_k += transfer_to_b_j / CAPACITY_B_J_PER_K;
            }
        }
        previously_published_a = old_a;
        previously_published_b = old_b;

        if !state.body_a_k.is_finite() || !state.body_b_k.is_finite() {
            return Err("simulation produced a non-finite state".to_string());
        }
    }

    let final_energy_drift = (total_energy_j(state) - initial_energy).abs();
    let rms_temperature_error =
        (squared_error_sum / error_count as f64).sqrt();

    Ok(SimulationMetrics {
        step_s,
        steps,
        max_energy_drift_j: max_energy_drift,
        final_energy_drift_j: final_energy_drift,
        rms_temperature_error_k: rms_temperature_error,
        max_temperature_error_k: max_temperature_error,
    })
}

/// Validate two signed energy receipts for one physical exchange.
/// Positive/negative signs are each component's own energy change, so a
/// conservative two-party transfer must sum to zero within tolerance.
fn validate_exchange_pair(
    left: PortReceipt<'_>,
    right: PortReceipt<'_>,
    max_clock_skew_ms: u64,
    energy_tolerance_j: f64,
) -> Result<(), &'static str> {
    if !energy_tolerance_j.is_finite() || energy_tolerance_j < 0.0 {
        return Err("invalid_energy_tolerance");
    }
    if left.exchange_id.is_empty()
        || right.exchange_id.is_empty()
        || left.component_id.is_empty()
        || right.component_id.is_empty()
        || left.peer_id.is_empty()
        || right.peer_id.is_empty()
    {
        return Err("missing_exchange_identity");
    }
    if left.exchange_id != right.exchange_id {
        return Err("exchange_id_mismatch");
    }
    if left.component_id != right.peer_id || right.component_id != left.peer_id {
        return Err("component_pair_mismatch");
    }
    if left.unit != "J" || right.unit != "J" {
        return Err("unsupported_energy_unit");
    }
    if !left.signed_energy_j.is_finite() || !right.signed_energy_j.is_finite() {
        return Err("non_finite_energy");
    }
    if left.timestamp_ms.abs_diff(right.timestamp_ms) > max_clock_skew_ms {
        return Err("clock_skew_exceeded");
    }
    if (left.signed_energy_j + right.signed_energy_j).abs() > energy_tolerance_j {
        return Err("non_conservative_exchange");
    }
    Ok(())
}

fn baseline_receipts() -> (PortReceipt<'static>, PortReceipt<'static>) {
    (
        PortReceipt {
            exchange_id: "exchange-0001",
            component_id: "body-a",
            peer_id: "body-b",
            timestamp_ms: 1_000,
            signed_energy_j: -3.2,
            unit: "J",
        },
        PortReceipt {
            exchange_id: "exchange-0001",
            component_id: "body-b",
            peer_id: "body-a",
            timestamp_ms: 1_002,
            signed_energy_j: 3.2,
            unit: "J",
        },
    )
}

fn run_report() -> Result<String, String> {
    let shared = simulate(CouplingMode::SharedFluxJacobi, 0.05)?;
    let lagged = simulate(CouplingMode::IndependentPortsOneStepStale, 0.05)?;
    let coarse = simulate(CouplingMode::SharedFluxJacobi, 0.05)?;
    let fine = simulate(CouplingMode::SharedFluxJacobi, 0.025)?;
    if fine.rms_temperature_error_k <= 0.0 {
        return Err("fine-step reference error unexpectedly vanished".to_string());
    }
    let error_ratio = coarse.rms_temperature_error_k / fine.rms_temperature_error_k;
    let observed_order = error_ratio.ln() / 2.0_f64.ln();

    let (valid_left, valid_right) = baseline_receipts();
    let mut wrong_sign = valid_right;
    wrong_sign.signed_energy_j = -3.2;
    let mut stale_clock = valid_right;
    stale_clock.timestamp_ms = 1_050;
    let mut wrong_unit = valid_right;
    wrong_unit.unit = "kJ";

    let mut output = String::from(
        "MFG_COMPOSITION_COUPLING version=1 oracle=two_body_analytic duration_s=2.000000\n",
    );
    output.push_str(&format!(
        "SIMULATION method=shared_flux_jacobi step_s={:.6} steps={} max_energy_drift_j={:.9e} final_energy_drift_j={:.9e} rms_temperature_error_k={:.9} max_temperature_error_k={:.9}\n",
        shared.step_s,
        shared.steps,
        shared.max_energy_drift_j,
        shared.final_energy_drift_j,
        shared.rms_temperature_error_k,
        shared.max_temperature_error_k
    ));
    output.push_str(&format!(
        "SIMULATION method=independent_ports_one_step_stale step_s={:.6} steps={} max_energy_drift_j={:.9e} final_energy_drift_j={:.9e} rms_temperature_error_k={:.9} max_temperature_error_k={:.9}\n",
        lagged.step_s,
        lagged.steps,
        lagged.max_energy_drift_j,
        lagged.final_energy_drift_j,
        lagged.rms_temperature_error_k,
        lagged.max_temperature_error_k
    ));
    output.push_str(&format!(
        "CONVERGENCE method=shared_flux_jacobi coarse_step_s={:.6} fine_step_s={:.6} rms_error_ratio={:.6} observed_order={:.6} expected_order=1\n",
        coarse.step_s, fine.step_s, error_ratio, observed_order
    ));
    output.push_str(&format!(
        "CONTRACT valid_pair={} wrong_sign={} stale_clock={} wrong_unit={}\n",
        if validate_exchange_pair(valid_left, valid_right, 5, ENERGY_TOLERANCE_J).is_ok() {
            "ACCEPT"
        } else {
            "REJECT"
        },
        if validate_exchange_pair(valid_left, wrong_sign, 5, ENERGY_TOLERANCE_J).is_ok() {
            "ACCEPT"
        } else {
            "REJECT"
        },
        if validate_exchange_pair(valid_left, stale_clock, 5, ENERGY_TOLERANCE_J).is_ok() {
            "ACCEPT"
        } else {
            "REJECT"
        },
        if validate_exchange_pair(valid_left, wrong_unit, 5, ENERGY_TOLERANCE_J).is_ok() {
            "ACCEPT"
        } else {
            "REJECT"
        }
    ));
    output.push_str(
        "CLAIM_CEILING synthetic_known_truth=true real_factory_data=false fmi_conformance=false certified_master_algorithm=false machine_actuation=false\n",
    );
    Ok(output)
}

#[cfg(not(test))]
fn main() {
    match run_report() {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprintln!("mfg-composition-coupling: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytic_fixture_points_match_closed_form_solution() {
        let fixture = include_str!("fixtures/analytic-composition-coupling.tsv");
        let mut rows = 0_usize;
        for line in fixture.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 3, "fixture row: {line}");
            let time_s: f64 = fields[0].parse().unwrap();
            let expected_a: f64 = fields[1].parse().unwrap();
            let expected_b: f64 = fields[2].parse().unwrap();
            let actual = analytic_state(time_s);
            assert!((actual.body_a_k - expected_a).abs() < 1.0e-8);
            assert!((actual.body_b_k - expected_b).abs() < 1.0e-8);
            rows += 1;
        }
        assert_eq!(rows, 4);
    }

    #[test]
    fn shared_flux_conserves_energy_but_is_not_exact() {
        let metrics = simulate(CouplingMode::SharedFluxJacobi, 0.05).unwrap();
        assert!(metrics.max_energy_drift_j < ENERGY_TOLERANCE_J);
        assert!(metrics.rms_temperature_error_k > 0.01);
        assert!(metrics.max_temperature_error_k > 0.01);
    }

    #[test]
    fn asynchronous_independent_ports_leak_energy() {
        let metrics = simulate(CouplingMode::IndependentPortsOneStepStale, 0.05).unwrap();
        assert!(metrics.max_energy_drift_j > 0.5);
        assert!(metrics.final_energy_drift_j > 0.5);
    }

    #[test]
    fn halving_the_step_exhibits_first_order_convergence() {
        let coarse = simulate(CouplingMode::SharedFluxJacobi, 0.05).unwrap();
        let fine = simulate(CouplingMode::SharedFluxJacobi, 0.025).unwrap();
        let ratio = coarse.rms_temperature_error_k / fine.rms_temperature_error_k;
        let observed_order = ratio.ln() / 2.0_f64.ln();
        assert!(fine.rms_temperature_error_k < coarse.rms_temperature_error_k);
        assert!(ratio > 1.7 && ratio < 2.3, "observed error ratio: {ratio}");
        assert!(observed_order > 0.75 && observed_order < 1.25);
    }

    #[test]
    fn valid_exchange_pair_is_accepted() {
        let (left, right) = baseline_receipts();
        assert_eq!(
            validate_exchange_pair(left, right, 5, ENERGY_TOLERANCE_J),
            Ok(())
        );
    }

    #[test]
    fn mismatched_exchange_id_is_rejected() {
        let (left, mut right) = baseline_receipts();
        right.exchange_id = "exchange-0002";
        assert_eq!(
            validate_exchange_pair(left, right, 5, ENERGY_TOLERANCE_J),
            Err("exchange_id_mismatch")
        );
    }

    #[test]
    fn mismatched_component_pair_is_rejected() {
        let (left, mut right) = baseline_receipts();
        right.peer_id = "body-c";
        assert_eq!(
            validate_exchange_pair(left, right, 5, ENERGY_TOLERANCE_J),
            Err("component_pair_mismatch")
        );
    }

    #[test]
    fn wrong_sign_is_rejected() {
        let (left, mut right) = baseline_receipts();
        right.signed_energy_j = -3.2;
        assert_eq!(
            validate_exchange_pair(left, right, 5, ENERGY_TOLERANCE_J),
            Err("non_conservative_exchange")
        );
    }

    #[test]
    fn unequal_transfer_magnitude_is_rejected() {
        let (left, mut right) = baseline_receipts();
        right.signed_energy_j = 3.0;
        assert_eq!(
            validate_exchange_pair(left, right, 5, ENERGY_TOLERANCE_J),
            Err("non_conservative_exchange")
        );
    }

    #[test]
    fn stale_exchange_clock_is_rejected() {
        let (left, mut right) = baseline_receipts();
        right.timestamp_ms = 1_050;
        assert_eq!(
            validate_exchange_pair(left, right, 5, ENERGY_TOLERANCE_J),
            Err("clock_skew_exceeded")
        );
    }

    #[test]
    fn incompatible_units_are_rejected() {
        let (left, mut right) = baseline_receipts();
        right.unit = "kJ";
        assert_eq!(
            validate_exchange_pair(left, right, 5, ENERGY_TOLERANCE_J),
            Err("unsupported_energy_unit")
        );
    }

    #[test]
    fn non_finite_energy_is_rejected() {
        let (left, mut right) = baseline_receipts();
        right.signed_energy_j = f64::NAN;
        assert_eq!(
            validate_exchange_pair(left, right, 5, ENERGY_TOLERANCE_J),
            Err("non_finite_energy")
        );
    }

    #[test]
    fn invalid_tolerance_is_rejected() {
        let (left, right) = baseline_receipts();
        assert_eq!(
            validate_exchange_pair(left, right, 5, f64::NAN),
            Err("invalid_energy_tolerance")
        );
    }

    #[test]
    fn invalid_communication_steps_fail_closed() {
        assert!(simulate(CouplingMode::SharedFluxJacobi, 0.0).is_err());
        assert!(simulate(CouplingMode::SharedFluxJacobi, f64::NAN).is_err());
        assert!(simulate(CouplingMode::SharedFluxJacobi, 0.03).is_err());
    }

    #[test]
    fn deterministic_report_replays_identically() {
        assert_eq!(run_report().unwrap(), run_report().unwrap());
    }

    #[test]
    fn passing_exchange_contract_does_not_claim_trajectory_accuracy() {
        let (left, right) = baseline_receipts();
        assert!(validate_exchange_pair(left, right, 5, ENERGY_TOLERANCE_J).is_ok());
        let simulation = simulate(CouplingMode::SharedFluxJacobi, 0.05).unwrap();
        assert!(simulation.rms_temperature_error_k > 0.01);
    }
}
