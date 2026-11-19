use ocean_thermal_balance_verifier::{
    verify_cooling_balance, CoolingCase, CoolingMode, SiteThermalLimits,
};

fn case(mode: CoolingMode, direct: f64, evaporator: f64, chiller: f64) -> CoolingCase {
    let pump = 5_000.0;
    let pump_fraction = 1.0;
    let expected_rejection = direct + evaporator + chiller + pump * pump_fraction;
    let flow = 5.0;
    let cp = 4180.0;
    let intake = 20.0;
    let discharge = intake + expected_rejection / (flow * cp);
    CoolingCase {
        case_id: format!("synthetic-{mode:?}"),
        configuration_id: "synthetic-three-mode-v0".to_owned(),
        meter_boundary_id: "synthetic-cooling-boundary".to_owned(),
        intake_observation_id: "synthetic-intake".to_owned(),
        discharge_observation_id: "synthetic-discharge".to_owned(),
        mode,
        it_heat_load_w: 100_000.0,
        direct_heat_to_seawater_w: direct,
        chiller_evaporator_heat_w: evaporator,
        chiller_electric_input_w: chiller,
        water_pump_electric_input_w: pump,
        other_cooling_auxiliary_electric_input_w: 0.0,
        recovered_heat_source_w: 0.0,
        useful_heat_delivered_w: 0.0,
        recovery_loss_to_seawater_w: 0.0,
        recovery_loss_to_ambient_w: 0.0,
        heat_sink_id: None,
        heat_delivery_observation_id: None,
        pump_heat_fraction_to_seawater: pump_fraction,
        other_heat_to_seawater_w: 0.0,
        seawater_mass_flow_kg_per_s: flow,
        seawater_specific_heat_j_per_kg_k: cp,
        seawater_intake_temperature_c: intake,
        seawater_discharge_temperature_c: discharge,
        interval_duration_seconds: 900.0,
        heat_balance_tolerance_w: 1e-6,
        site_limits: SiteThermalLimits {
            maximum_discharge_temperature_c: None,
            maximum_temperature_rise_c: None,
            temperature_tolerance_c: 1e-9,
        },
    }
}

fn heat_recovery_case() -> CoolingCase {
    let mut recovery = case(CoolingMode::DirectSeawater, 2_162_500.0, 0.0, 0.0);
    recovery.case_id = "synthetic-compute-ro-feed-preheat".to_owned();
    recovery.configuration_id = "synthetic-heat-recovery-v0".to_owned();
    recovery.it_heat_load_w = 10_000_000.0;
    recovery.water_pump_electric_input_w = 0.0;
    recovery.pump_heat_fraction_to_seawater = 0.0;
    recovery.recovered_heat_source_w = 7_837_500.0;
    recovery.useful_heat_delivered_w = 6_270_000.0;
    recovery.recovery_loss_to_seawater_w = 1_567_500.0;
    recovery.recovery_loss_to_ambient_w = 0.0;
    recovery.heat_sink_id = Some("synthetic-ro-feed-sink".to_owned());
    recovery.heat_delivery_observation_id =
        Some("synthetic-ro-feed-heat-meter".to_owned());
    recovery.seawater_mass_flow_kg_per_s = 500.0;
    recovery.seawater_specific_heat_j_per_kg_k = 4180.0;
    recovery.seawater_intake_temperature_c = 20.0;
    recovery.seawater_discharge_temperature_c =
        20.0 + 3_730_000.0 / (500.0 * 4180.0);
    recovery
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = vec![
        case(CoolingMode::DirectSeawater, 100_000.0, 0.0, 0.0),
        case(CoolingMode::PartialEconomiserPlusChiller, 60_000.0, 40_000.0, 10_000.0),
        case(CoolingMode::ChillerOnly, 0.0, 100_000.0, 25_000.0),
    ];
    cases.push(heat_recovery_case());
    println!("Ocean thermal balance verifier v0.1.0");
    println!("0D steady-state accounting only; no CFD or environmental approval.");
    for case in cases {
        let report = verify_cooling_balance(&case)?;
        println!(
            "{:?}: {}; IT heat {:.2} kWh; useful recovered heat {:.2} kWh; cooling electricity {:.2} kWh; seawater rejection {:.1} kW; delta-T {:.3} C; COP {:?}; violations {:?}",
            report.mode,
            if report.passed { "PASS" } else { "FAIL" },
            report.it_heat_energy_kwh,
            report.useful_heat_delivered_energy_kwh,
            report.cooling_electric_energy_kwh,
            report.seawater_heat_pickup_w / 1000.0,
            report.seawater_temperature_rise_c,
            report.chiller_cop,
            report.violations.iter().map(|v| v.code.to_string()).collect::<Vec<_>>()
        );
    }
    Ok(())
}
