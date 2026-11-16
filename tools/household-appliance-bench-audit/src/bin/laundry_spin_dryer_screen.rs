use serde_json::json;
use std::env;
use std::process::ExitCode;

#[derive(Debug, Clone, Copy)]
struct Scenario {
    dry_textile_mass_kg: f64,
    inlet_residual_moisture_dry_basis: f64,
    outlet_residual_moisture_dry_basis: f64,
    latent_heat_low_mj_per_kg: f64,
    latent_heat_high_mj_per_kg: f64,
    heat_pump_cop_low: f64,
    heat_pump_cop_high: f64,
}

#[derive(Debug, Clone, Copy)]
struct ScreeningResult {
    incremental_water_removed_kg: f64,
    latent_thermal_duty_low_kwh: f64,
    latent_thermal_duty_high_kwh: f64,
    idealized_electrical_equivalent_low_kwh: f64,
    idealized_electrical_equivalent_high_kwh: f64,
}

fn validate(s: Scenario) -> Result<(), String> {
    if !s.dry_textile_mass_kg.is_finite() || s.dry_textile_mass_kg <= 0.0 {
        return Err("dry textile mass must be finite and positive".into());
    }
    for (name, value) in [
        (
            "inlet residual moisture",
            s.inlet_residual_moisture_dry_basis,
        ),
        (
            "outlet residual moisture",
            s.outlet_residual_moisture_dry_basis,
        ),
    ] {
        if !value.is_finite() || !(0.0..=5.0).contains(&value) {
            return Err(format!(
                "{name} must be finite and between 0 and 5 kg/kg dry textile"
            ));
        }
    }
    if s.inlet_residual_moisture_dry_basis <= s.outlet_residual_moisture_dry_basis {
        return Err(
            "inlet moisture must exceed outlet moisture for a positive extraction delta".into(),
        );
    }
    if !s.latent_heat_low_mj_per_kg.is_finite()
        || !s.latent_heat_high_mj_per_kg.is_finite()
        || s.latent_heat_low_mj_per_kg <= 0.0
        || s.latent_heat_low_mj_per_kg > s.latent_heat_high_mj_per_kg
    {
        return Err("latent-heat interval must be finite, positive and ordered".into());
    }
    if !s.heat_pump_cop_low.is_finite()
        || !s.heat_pump_cop_high.is_finite()
        || s.heat_pump_cop_low <= 0.0
        || s.heat_pump_cop_low > s.heat_pump_cop_high
    {
        return Err("heat-pump COP interval must be finite, positive and ordered".into());
    }
    Ok(())
}

fn screen(s: Scenario) -> Result<ScreeningResult, String> {
    validate(s)?;
    let water_removed = s.dry_textile_mass_kg
        * (s.inlet_residual_moisture_dry_basis - s.outlet_residual_moisture_dry_basis);

    // 1 kWh = 3.6 MJ. This is latent thermal duty only, not total dryer input.
    let thermal_low = water_removed * s.latent_heat_low_mj_per_kg / 3.6;
    let thermal_high = water_removed * s.latent_heat_high_mj_per_kg / 3.6;

    // Conservative interval arithmetic: smallest electrical equivalent uses
    // low latent heat / high COP; largest uses high latent heat / low COP.
    let electric_low = thermal_low / s.heat_pump_cop_high;
    let electric_high = thermal_high / s.heat_pump_cop_low;

    Ok(ScreeningResult {
        incremental_water_removed_kg: water_removed,
        latent_thermal_duty_low_kwh: thermal_low,
        latent_thermal_duty_high_kwh: thermal_high,
        idealized_electrical_equivalent_low_kwh: electric_low,
        idealized_electrical_equivalent_high_kwh: electric_high,
    })
}

#[cfg(test)]
fn paired_net_reduction_kwh(
    incremental_washer_spin_electricity_kwh: f64,
    measured_dryer_electricity_reduction_kwh: f64,
) -> Result<f64, String> {
    if !incremental_washer_spin_electricity_kwh.is_finite()
        || !measured_dryer_electricity_reduction_kwh.is_finite()
        || incremental_washer_spin_electricity_kwh < 0.0
        || measured_dryer_electricity_reduction_kwh < 0.0
    {
        return Err("measured energy deltas must be finite and non-negative".into());
    }
    Ok(measured_dryer_electricity_reduction_kwh - incremental_washer_spin_electricity_kwh)
}

fn parse_scenario(args: &[String]) -> Result<Scenario, String> {
    const DEFAULTS: [f64; 7] = [8.0, 0.539, 0.45, 2.25, 2.45, 2.0, 3.0];
    if args.len() != 1 && args.len() != 8 {
        return Err(
            "usage: laundry_spin_dryer_screen [DRY_KG INLET_RMC OUTLET_RMC LATENT_LOW_MJ_KG LATENT_HIGH_MJ_KG COP_LOW COP_HIGH]"
                .into(),
        );
    }
    let values = if args.len() == 1 {
        DEFAULTS
    } else {
        let mut parsed = [0.0; 7];
        for (index, raw) in args[1..].iter().enumerate() {
            parsed[index] = raw
                .parse::<f64>()
                .map_err(|_| format!("argument {} is not a valid number: {raw}", index + 1))?;
        }
        parsed
    };
    Ok(Scenario {
        dry_textile_mass_kg: values[0],
        inlet_residual_moisture_dry_basis: values[1],
        outlet_residual_moisture_dry_basis: values[2],
        latent_heat_low_mj_per_kg: values[3],
        latent_heat_high_mj_per_kg: values[4],
        heat_pump_cop_low: values[5],
        heat_pump_cop_high: values[6],
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let scenario = match parse_scenario(&args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    match screen(scenario) {
        Ok(result) => {
            println!(
                "{}",
                json!({
                    "model": "washer_spin_to_dryer_latent_heat_screen_v0_1",
                    "status": "analytical_screening_not_validated",
                    "evidence_class": "assumption_driven_calculation",
                    "scenario": {
                        "dry_textile_mass_kg": scenario.dry_textile_mass_kg,
                        "inlet_residual_moisture_dry_basis": scenario.inlet_residual_moisture_dry_basis,
                        "outlet_residual_moisture_dry_basis": scenario.outlet_residual_moisture_dry_basis,
                        "latent_heat_interval_mj_per_kg": [
                            scenario.latent_heat_low_mj_per_kg,
                            scenario.latent_heat_high_mj_per_kg
                        ],
                        "heat_pump_cop_interval": [
                            scenario.heat_pump_cop_low,
                            scenario.heat_pump_cop_high
                        ]
                    },
                    "result": {
                        "incremental_water_removed_kg": result.incremental_water_removed_kg,
                        "latent_thermal_duty_interval_kwh": [
                            result.latent_thermal_duty_low_kwh,
                            result.latent_thermal_duty_high_kwh
                        ],
                        "idealized_electrical_equivalent_interval_kwh": [
                            result.idealized_electrical_equivalent_low_kwh,
                            result.idealized_electrical_equivalent_high_kwh
                        ]
                    },
                    "exclusions": [
                        "washer incremental spin electricity",
                        "sensible heating of textiles, water and machine",
                        "air leakage and heat-exchanger losses",
                        "fans, pumps, compressor maps, defrost and controls",
                        "load-dependent mass transfer and drying kinetics",
                        "fabric damage, vibration, noise and durability",
                        "no physical observations or product performance claim"
                    ],
                    "interpretation": "latent-heat-only screening interval; not a dryer-cycle energy prediction or savings claim"
                })
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("screening model rejected scenario: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn illustrative() -> Scenario {
        Scenario {
            dry_textile_mass_kg: 8.0,
            inlet_residual_moisture_dry_basis: 0.539,
            outlet_residual_moisture_dry_basis: 0.45,
            latent_heat_low_mj_per_kg: 2.25,
            latent_heat_high_mj_per_kg: 2.45,
            heat_pump_cop_low: 2.0,
            heat_pump_cop_high: 3.0,
        }
    }

    #[test]
    fn reproduces_illustrative_incremental_water_mass() {
        let result = screen(illustrative()).expect("valid scenario");
        assert!((result.incremental_water_removed_kg - 0.712).abs() < 1e-12);
    }

    #[test]
    fn emits_bounded_latent_heat_screen_not_cycle_prediction() {
        let result = screen(illustrative()).expect("valid scenario");
        assert!((result.latent_thermal_duty_low_kwh - 0.445).abs() < 0.002);
        assert!((result.latent_thermal_duty_high_kwh - 0.484).abs() < 0.002);
        assert!((result.idealized_electrical_equivalent_low_kwh - 0.148).abs() < 0.002);
        assert!((result.idealized_electrical_equivalent_high_kwh - 0.242).abs() < 0.002);
    }

    #[test]
    fn more_extraction_removes_more_water_for_same_dry_mass() {
        let base = screen(illustrative()).expect("valid scenario");
        let mut more_extraction = illustrative();
        more_extraction.outlet_residual_moisture_dry_basis = 0.40;
        let improved = screen(more_extraction).expect("valid scenario");
        assert!(improved.incremental_water_removed_kg > base.incremental_water_removed_kg);
    }

    #[test]
    fn rejects_unknown_or_non_positive_dry_mass() {
        let mut scenario = illustrative();
        scenario.dry_textile_mass_kg = 0.0;
        assert!(screen(scenario).is_err());
        scenario.dry_textile_mass_kg = f64::NAN;
        assert!(screen(scenario).is_err());
    }

    #[test]
    fn rejects_reversed_moisture_gradient() {
        let mut scenario = illustrative();
        scenario.inlet_residual_moisture_dry_basis = 0.40;
        assert!(screen(scenario).is_err());
    }

    #[test]
    fn rejects_invalid_or_unordered_assumption_intervals() {
        let mut scenario = illustrative();
        scenario.heat_pump_cop_low = 0.0;
        assert!(screen(scenario).is_err());
        scenario = illustrative();
        scenario.latent_heat_low_mj_per_kg = 2.6;
        assert!(screen(scenario).is_err());
    }

    #[test]
    fn washer_spin_cost_must_be_subtracted_from_dryer_saving() {
        let net = paired_net_reduction_kwh(0.04, 0.18).expect("valid deltas");
        assert!((net - 0.14).abs() < 1e-12);
        let negative = paired_net_reduction_kwh(0.20, 0.18).expect("valid deltas");
        assert!((negative + 0.02).abs() < 1e-12);
    }

    #[test]
    fn rejects_negative_or_non_finite_measured_energy_deltas() {
        assert!(paired_net_reduction_kwh(-0.01, 0.2).is_err());
        assert!(paired_net_reduction_kwh(0.1, f64::NAN).is_err());
    }
}
