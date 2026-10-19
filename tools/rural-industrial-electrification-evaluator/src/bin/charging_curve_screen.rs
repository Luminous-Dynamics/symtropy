use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process};

const EPSILON: f64 = 1.0e-9;
const CLAIM_CEILING: &str = "Analytic piecewise-SOC charging screen with synthetic/reference inputs and static derating factors; not a measured battery charge curve, electrochemical/thermal model, charger qualification, or deployment recommendation.";

#[derive(Debug, Deserialize)]
struct Corpus {
    schema_version: String,
    suite_id: String,
    claim_ceiling: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
struct Scenario {
    scenario_id: String,
    label: String,
    evidence_class: String,
    evidence_reference: String,
    evidence_note: String,
    nominal_pack_capacity_kwh: f64,
    state_of_health_fraction: f64,
    temperature_capacity_fraction: f64,
    temperature_acceptance_fraction: f64,
    initial_soc_fraction: f64,
    target_soc_fraction: f64,
    charging_efficiency_fraction: f64,
    port_input_power_kw: f64,
    site_input_power_budget_kw: f64,
    acceptance_curve: Vec<CurveBand>,
}

#[derive(Debug, Clone, Deserialize)]
struct CurveBand {
    soc_lower_fraction: f64,
    soc_upper_fraction: f64,
    max_battery_acceptance_kw: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct BandResult {
    curve_band_index: usize,
    soc_start_fraction: f64,
    soc_end_fraction: f64,
    battery_energy_kwh: f64,
    site_input_energy_kwh: f64,
    conversion_loss_kwh: f64,
    reference_battery_acceptance_kw: f64,
    temperature_derated_battery_acceptance_kw: f64,
    external_battery_side_power_limit_kw: f64,
    effective_battery_charge_power_kw: f64,
    effective_site_input_power_kw: f64,
    duration_minutes: f64,
    bottleneck: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioResult {
    scenario_id: String,
    label: String,
    evidence_class: String,
    result_status: String,
    evidence_reference: String,
    evidence_note: String,
    nominal_pack_capacity_kwh: f64,
    effective_capacity_kwh: f64,
    state_of_health_fraction: f64,
    temperature_capacity_fraction: f64,
    temperature_acceptance_fraction: f64,
    initial_soc_fraction: f64,
    target_soc_fraction: f64,
    site_input_power_limit_kw: f64,
    effective_battery_energy_required_kwh: f64,
    total_battery_energy_delivered_kwh: f64,
    total_site_input_energy_kwh: f64,
    total_conversion_loss_kwh: f64,
    energy_balance_residual_kwh: f64,
    total_charge_duration_minutes: f64,
    average_battery_charge_power_kw: f64,
    minimum_effective_battery_charge_power_kw: f64,
    maximum_effective_battery_charge_power_kw: f64,
    battery_curve_bottleneck_band_count: usize,
    site_or_port_bottleneck_band_count: usize,
    results: Vec<BandResult>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    suite_id: String,
    method: &'static str,
    results: Vec<ScenarioResult>,
    claim_ceiling: &'static str,
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/rural-industrial-electrification-evaluator/fixtures/charging_curve_v0_1.json"
            .to_string()
    });
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let corpus: Corpus = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid charging-curve JSON: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_corpus(&corpus) {
        eprintln!("invalid charging-curve corpus: {error}");
        process::exit(2);
    }

    let mut results: Vec<ScenarioResult> = corpus.scenarios.iter().map(evaluate_scenario).collect();
    results.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    let report = Report {
        schema_version: corpus.schema_version,
        suite_id: corpus.suite_id,
        method: "piecewise-constant battery-side acceptance by SOC band, static temperature derating, constant site-to-battery efficiency",
        results,
        claim_ceiling: CLAIM_CEILING,
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize charge-curve report: {error}");
            process::exit(2);
        }
    }
}

fn validate_corpus(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_version != "0.1.0" || corpus.suite_id != "EV-CHARGE-CURVE-001" {
        return Err("unsupported_schema_or_suite_id".to_string());
    }
    if corpus.claim_ceiling.trim().is_empty() || corpus.scenarios.is_empty() {
        return Err("claim_ceiling_and_nonempty_scenarios_are_required".to_string());
    }
    let mut scenario_ids = BTreeSet::new();
    for scenario in &corpus.scenarios {
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err(format!("duplicate_scenario_id: {}", scenario.scenario_id));
        }
        validate_scenario(scenario)
            .map_err(|error| format!("{}: {error}", scenario.scenario_id))?;
    }
    Ok(())
}

fn validate_scenario(scenario: &Scenario) -> Result<(), String> {
    for (name, value) in [
        ("scenario_id", scenario.scenario_id.as_str()),
        ("label", scenario.label.as_str()),
        ("evidence_note", scenario.evidence_note.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name}_must_be_nonempty"));
        }
    }
    if ![
        "synthetic",
        "measured",
        "manufacturer",
        "literature",
        "estimate",
        "unknown",
    ]
    .contains(&scenario.evidence_class.as_str())
    {
        return Err("unsupported_evidence_class".to_string());
    }
    if scenario.evidence_class != "synthetic"
        && scenario.evidence_class != "unknown"
        && scenario.evidence_reference.trim().is_empty()
    {
        return Err("non_synthetic_evidence_requires_reference".to_string());
    }

    for (name, value) in [
        (
            "nominal_pack_capacity_kwh",
            scenario.nominal_pack_capacity_kwh,
        ),
        ("port_input_power_kw", scenario.port_input_power_kw),
        (
            "site_input_power_budget_kw",
            scenario.site_input_power_budget_kw,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name}_must_be_finite_and_positive"));
        }
    }
    for (name, value) in [
        (
            "state_of_health_fraction",
            scenario.state_of_health_fraction,
        ),
        (
            "temperature_capacity_fraction",
            scenario.temperature_capacity_fraction,
        ),
        (
            "temperature_acceptance_fraction",
            scenario.temperature_acceptance_fraction,
        ),
        (
            "charging_efficiency_fraction",
            scenario.charging_efficiency_fraction,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 || value > 1.0 {
            return Err(format!("{name}_must_be_in_(0,1]"));
        }
    }
    if !scenario.initial_soc_fraction.is_finite()
        || !scenario.target_soc_fraction.is_finite()
        || scenario.initial_soc_fraction < 0.0
        || scenario.target_soc_fraction > 1.0
        || scenario.target_soc_fraction <= scenario.initial_soc_fraction
    {
        return Err("soc_range_must_be_finite_with_0_le_initial_lt_target_le_1".to_string());
    }
    if scenario.acceptance_curve.is_empty() {
        return Err("acceptance_curve_must_cover_full_soc_range".to_string());
    }

    let mut expected_lower = 0.0;
    for (index, band) in scenario.acceptance_curve.iter().enumerate() {
        for (name, value) in [
            ("soc_lower_fraction", band.soc_lower_fraction),
            ("soc_upper_fraction", band.soc_upper_fraction),
            ("max_battery_acceptance_kw", band.max_battery_acceptance_kw),
        ] {
            if !value.is_finite() {
                return Err(format!("curve_band_{index}_{name}_must_be_finite"));
            }
        }
        if band.soc_lower_fraction < 0.0
            || band.soc_upper_fraction > 1.0
            || band.soc_upper_fraction <= band.soc_lower_fraction
        {
            return Err(format!("curve_band_{index}_soc_bounds_out_of_domain"));
        }
        if (band.soc_lower_fraction - expected_lower).abs() > 1.0e-8 {
            return Err(format!("curve_band_{index}_gap_overlap_or_unsorted_curve"));
        }
        if band.max_battery_acceptance_kw <= 0.0 {
            return Err(format!(
                "curve_band_{index}_acceptance_power_must_be_positive"
            ));
        }
        expected_lower = band.soc_upper_fraction;
    }
    if (expected_lower - 1.0).abs() > 1.0e-8 {
        return Err("acceptance_curve_must_end_at_soc_1".to_string());
    }
    let effective_capacity = scenario.nominal_pack_capacity_kwh
        * scenario.state_of_health_fraction
        * scenario.temperature_capacity_fraction;
    if !effective_capacity.is_finite() || effective_capacity <= 0.0 {
        return Err("derived_effective_capacity_must_be_finite_and_positive".to_string());
    }
    for band in &scenario.acceptance_curve {
        let _reference = band.max_battery_acceptance_kw * scenario.temperature_acceptance_fraction;
        if !_reference.is_finite() || _reference <= 0.0 {
            return Err(
                "derived_temperature_derated_acceptance_must_be_finite_and_positive".to_string(),
            );
        }
    }
    Ok(())
}

fn evaluate_scenario(scenario: &Scenario) -> ScenarioResult {
    let effective_capacity_kwh = scenario.nominal_pack_capacity_kwh
        * scenario.state_of_health_fraction
        * scenario.temperature_capacity_fraction;
    let requested_battery_kwh =
        effective_capacity_kwh * (scenario.target_soc_fraction - scenario.initial_soc_fraction);
    let site_input_power_limit_kw = scenario
        .port_input_power_kw
        .min(scenario.site_input_power_budget_kw);
    let external_battery_side_power_limit_kw =
        site_input_power_limit_kw * scenario.charging_efficiency_fraction;

    let mut results = Vec::new();
    for (index, band) in scenario.acceptance_curve.iter().enumerate() {
        let start = scenario.initial_soc_fraction.max(band.soc_lower_fraction);
        let end = scenario.target_soc_fraction.min(band.soc_upper_fraction);
        if end <= start + EPSILON {
            continue;
        }
        let battery_energy_kwh = effective_capacity_kwh * (end - start);
        let temperature_derated_battery_acceptance_kw =
            band.max_battery_acceptance_kw * scenario.temperature_acceptance_fraction;
        let effective_battery_charge_power_kw =
            temperature_derated_battery_acceptance_kw.min(external_battery_side_power_limit_kw);
        let effective_site_input_power_kw =
            effective_battery_charge_power_kw / scenario.charging_efficiency_fraction;
        let site_input_energy_kwh = battery_energy_kwh / scenario.charging_efficiency_fraction;
        let conversion_loss_kwh = site_input_energy_kwh - battery_energy_kwh;
        let duration_minutes = 60.0 * battery_energy_kwh / effective_battery_charge_power_kw;
        let bottleneck = if temperature_derated_battery_acceptance_kw
            <= external_battery_side_power_limit_kw + EPSILON
        {
            "battery_curve"
        } else {
            "site_or_port_cap"
        };
        results.push(BandResult {
            curve_band_index: index,
            soc_start_fraction: start,
            soc_end_fraction: end,
            battery_energy_kwh,
            site_input_energy_kwh,
            conversion_loss_kwh,
            reference_battery_acceptance_kw: band.max_battery_acceptance_kw,
            temperature_derated_battery_acceptance_kw,
            external_battery_side_power_limit_kw,
            effective_battery_charge_power_kw,
            effective_site_input_power_kw,
            duration_minutes,
            bottleneck: bottleneck.to_string(),
        });
    }

    let total_battery_energy_delivered_kwh: f64 =
        results.iter().map(|band| band.battery_energy_kwh).sum();
    let total_site_input_energy_kwh: f64 =
        results.iter().map(|band| band.site_input_energy_kwh).sum();
    let total_conversion_loss_kwh: f64 = results.iter().map(|band| band.conversion_loss_kwh).sum();
    let total_charge_duration_minutes: f64 = results.iter().map(|band| band.duration_minutes).sum();
    let min_power = results
        .iter()
        .map(|band| band.effective_battery_charge_power_kw)
        .fold(f64::INFINITY, f64::min);
    let max_power = results
        .iter()
        .map(|band| band.effective_battery_charge_power_kw)
        .fold(0.0, f64::max);
    let average_power = if total_charge_duration_minutes > 0.0 {
        total_battery_energy_delivered_kwh / (total_charge_duration_minutes / 60.0)
    } else {
        0.0
    };

    ScenarioResult {
        scenario_id: scenario.scenario_id.clone(),
        label: scenario.label.clone(),
        evidence_class: scenario.evidence_class.clone(),
        result_status: match scenario.evidence_class.as_str() {
            "synthetic" => "passes_synthetic_screen".to_string(),
            "unknown" => "indeterminate".to_string(),
            _ => "passes_evidence_backed_screen".to_string(),
        },
        evidence_reference: scenario.evidence_reference.clone(),
        evidence_note: scenario.evidence_note.clone(),
        nominal_pack_capacity_kwh: scenario.nominal_pack_capacity_kwh,
        effective_capacity_kwh,
        state_of_health_fraction: scenario.state_of_health_fraction,
        temperature_capacity_fraction: scenario.temperature_capacity_fraction,
        temperature_acceptance_fraction: scenario.temperature_acceptance_fraction,
        initial_soc_fraction: scenario.initial_soc_fraction,
        target_soc_fraction: scenario.target_soc_fraction,
        site_input_power_limit_kw,
        effective_battery_energy_required_kwh: requested_battery_kwh,
        total_battery_energy_delivered_kwh,
        total_site_input_energy_kwh,
        total_conversion_loss_kwh,
        energy_balance_residual_kwh: total_site_input_energy_kwh
            - total_battery_energy_delivered_kwh
            - total_conversion_loss_kwh,
        total_charge_duration_minutes,
        average_battery_charge_power_kw: average_power,
        minimum_effective_battery_charge_power_kw: if min_power.is_finite() {
            min_power
        } else {
            0.0
        },
        maximum_effective_battery_charge_power_kw: max_power,
        battery_curve_bottleneck_band_count: results
            .iter()
            .filter(|band| band.bottleneck == "battery_curve")
            .count(),
        site_or_port_bottleneck_band_count: results
            .iter()
            .filter(|band| band.bottleneck == "site_or_port_cap")
            .count(),
        results,
        claim_ceiling: CLAIM_CEILING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_scenario() -> Scenario {
        Scenario {
            scenario_id: "sample".to_string(),
            label: "synthetic sample".to_string(),
            evidence_class: "synthetic".to_string(),
            evidence_reference: "synthetic".to_string(),
            evidence_note: "unit test only".to_string(),
            nominal_pack_capacity_kwh: 50.0,
            state_of_health_fraction: 1.0,
            temperature_capacity_fraction: 1.0,
            temperature_acceptance_fraction: 1.0,
            initial_soc_fraction: 0.2,
            target_soc_fraction: 0.8,
            charging_efficiency_fraction: 0.9,
            port_input_power_kw: 100.0,
            site_input_power_budget_kw: 100.0,
            acceptance_curve: vec![
                CurveBand {
                    soc_lower_fraction: 0.0,
                    soc_upper_fraction: 0.5,
                    max_battery_acceptance_kw: 100.0,
                },
                CurveBand {
                    soc_lower_fraction: 0.5,
                    soc_upper_fraction: 0.8,
                    max_battery_acceptance_kw: 50.0,
                },
                CurveBand {
                    soc_lower_fraction: 0.8,
                    soc_upper_fraction: 1.0,
                    max_battery_acceptance_kw: 20.0,
                },
            ],
        }
    }

    #[test]
    fn taper_increases_session_duration_vs_flat_acceptance() {
        let mut tapered = sample_scenario();
        let tapered_result = evaluate_scenario(&tapered);
        tapered.acceptance_curve = vec![CurveBand {
            soc_lower_fraction: 0.0,
            soc_upper_fraction: 1.0,
            max_battery_acceptance_kw: 100.0,
        }];
        let flat_result = evaluate_scenario(&tapered);
        assert!(
            tapered_result.total_charge_duration_minutes
                > flat_result.total_charge_duration_minutes
        );
    }

    #[test]
    fn site_input_cap_limits_battery_side_power() {
        let mut scenario = sample_scenario();
        scenario.site_input_power_budget_kw = 20.0;
        let result = evaluate_scenario(&scenario);
        assert!((result.results[0].effective_battery_charge_power_kw - 18.0).abs() < 1e-9);
        assert_eq!(result.results[0].bottleneck, "site_or_port_cap");
    }

    #[test]
    fn state_of_health_reduces_energy_for_the_same_soc_window() {
        let mut scenario = sample_scenario();
        let healthy = evaluate_scenario(&scenario);
        scenario.state_of_health_fraction = 0.8;
        let aged = evaluate_scenario(&scenario);
        assert!(
            aged.effective_battery_energy_required_kwh
                < healthy.effective_battery_energy_required_kwh
        );
    }

    #[test]
    fn temperature_acceptance_derating_cannot_shorten_session() {
        let mut scenario = sample_scenario();
        let nominal = evaluate_scenario(&scenario);
        scenario.temperature_acceptance_fraction = 0.5;
        let derated = evaluate_scenario(&scenario);
        assert!(derated.total_charge_duration_minutes >= nominal.total_charge_duration_minutes);
    }

    #[test]
    fn site_energy_reconciles_with_battery_energy_and_losses() {
        let result = evaluate_scenario(&sample_scenario());
        assert!((result.energy_balance_residual_kwh).abs() < 1e-9);
        assert!(
            (result.total_battery_energy_delivered_kwh
                - result.effective_battery_energy_required_kwh)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn gapped_curve_is_rejected() {
        let mut scenario = sample_scenario();
        scenario.acceptance_curve[1].soc_lower_fraction = 0.55;
        assert!(
            validate_scenario(&scenario)
                .unwrap_err()
                .contains("gap_overlap")
        );
    }

    #[test]
    fn unsorted_overlapping_curve_is_rejected() {
        let mut scenario = sample_scenario();
        scenario.acceptance_curve.swap(0, 1);
        assert!(validate_scenario(&scenario).is_err());
    }

    #[test]
    fn target_soc_must_exceed_initial_and_not_exceed_one() {
        let mut scenario = sample_scenario();
        scenario.target_soc_fraction = scenario.initial_soc_fraction;
        assert!(validate_scenario(&scenario).is_err());
        scenario.target_soc_fraction = 1.1;
        assert!(validate_scenario(&scenario).is_err());
    }

    #[test]
    fn curve_bands_cover_partial_session_without_gaps() {
        let mut scenario = sample_scenario();
        scenario.initial_soc_fraction = 0.52;
        scenario.target_soc_fraction = 0.78;
        let result = evaluate_scenario(&scenario);
        assert_eq!(result.results.len(), 1);
        assert!((result.results[0].soc_start_fraction - 0.52).abs() < 1e-9);
        assert!((result.results[0].soc_end_fraction - 0.78).abs() < 1e-9);
    }

    #[test]
    fn unknown_evidence_remains_indeterminate() {
        let mut scenario = sample_scenario();
        scenario.evidence_class = "unknown".to_string();
        scenario.evidence_reference.clear();
        let result = evaluate_scenario(&scenario);
        assert_eq!(result.result_status, "indeterminate");
    }

    #[test]
    fn report_order_is_deterministic_after_scenario_sort() {
        let mut a = sample_scenario();
        a.scenario_id = "z-last".to_string();
        let mut b = sample_scenario();
        b.scenario_id = "a-first".to_string();
        let mut left = vec![evaluate_scenario(&a), evaluate_scenario(&b)];
        let mut right = vec![evaluate_scenario(&b), evaluate_scenario(&a)];
        left.sort_by(|x, y| x.scenario_id.cmp(&y.scenario_id));
        right.sort_by(|x, y| x.scenario_id.cmp(&y.scenario_id));
        assert_eq!(left, right);
        assert_eq!(left[0].scenario_id, "a-first");
    }
}
