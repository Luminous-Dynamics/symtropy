// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent flat-report auditor for the canonical solar-thermal fixture.
//!
//! This program deliberately does not call the kernel's model or verifier
//! functions. It reads the emitted JSON from stdin and recomputes its energy,
//! temperature, transfer, and sensible-exergy ledgers from the declared inputs.
//! The parser is intentionally limited to the versioned, flat scalar report.

use std::io::{self, Read};

fn scalar<'a>(json: &'a str, key: &str) -> Result<&'a str, String> {
    let json = json.trim();
    if !json.starts_with('{')
        || !json.ends_with('}')
        || json.matches('{').count() != 1
        || json.matches('}').count() != 1
        || json.contains('[')
        || json.contains(']')
    {
        return Err("expected one flat JSON object".to_owned());
    }

    let needle = format!("\"{key}\":");
    let mut found = None;
    for (index, _) in json.match_indices(&needle) {
        if found.is_some() {
            return Err(format!("duplicate report field: {key}"));
        }
        found = Some(index);
    }
    let index = found.ok_or_else(|| format!("missing report field: {key}"))?;
    let start = index + needle.len();
    let tail = &json[start..];
    let end = tail.find([',', '}'])
        .ok_or_else(|| format!("unterminated report field: {key}"))?;
    let value = tail[..end].trim();
    if value.is_empty() {
        return Err(format!("empty report field: {key}"));
    }
    Ok(value)
}

fn text_field<'a>(json: &'a str, key: &str) -> Result<&'a str, String> {
    let raw = scalar(json, key)?;
    let value = raw.strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| format!("report field {key} must be a string"))?;
    if value.contains('"') || value.contains('\\') {
        return Err(format!("escaped strings are not supported for report field {key}"));
    }
    Ok(value)
}

fn number(json: &str, key: &str) -> Result<f64, String> {
    let value: f64 = scalar(json, key)?
        .parse()
        .map_err(|_| format!("report field {key} must be a number"))?;
    if !value.is_finite() {
        return Err(format!("report field {key} must be finite"));
    }
    Ok(value)
}

fn boolean(json: &str, key: &str) -> Result<bool, String> {
    match scalar(json, key)? {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("report field {key} must be a JSON boolean")),
    }
}

fn near(label: &str, actual: f64, expected: f64) -> Result<(), String> {
    let tolerance = 5.0e-8 * actual.abs().max(expected.abs()).max(1.0);
    if !actual.is_finite() || !expected.is_finite() || (actual - expected).abs() > tolerance {
        return Err(format!("{label}: actual={actual:.12}, expected={expected:.12}, tolerance={tolerance:.3e}"));
    }
    Ok(())
}

fn required_nonnegative(json: &str, key: &str) -> Result<f64, String> {
    let value = number(json, key)?;
    if value < 0.0 {
        return Err(format!("report field {key} cannot be negative"));
    }
    Ok(value)
}

// Independent enthalpy calculation for the stated two-point, linear cp(T) curve.
// The formula is integrated analytically and does not call the kernel.
fn specific_enthalpy(t: f64, low_t: f64, high_t: f64, low_cp: f64, high_cp: f64)
    -> Result<f64, String>
{
    if low_t >= high_t || low_cp <= 0.0 || high_cp <= 0.0 || t < low_t || t > high_t {
        return Err("temperature or cp(T) bounds are invalid for the declared curve".to_owned());
    }
    let delta = t - low_t;
    let slope = (high_cp - low_cp) / (high_t - low_t);
    Ok(low_cp * delta + 0.5 * slope * delta * delta)
}

fn total_store_energy(
    t: f64,
    mass: f64,
    low_t: f64,
    high_t: f64,
    low_cp: f64,
    high_cp: f64,
) -> Result<f64, String> {
    Ok(mass * specific_enthalpy(t, low_t, high_t, low_cp, high_cp)?)
}

// Independent sensible-exergy calculation for the same piecewise-linear cp(T).
fn total_store_exergy(
    t: f64,
    reference_t: f64,
    mass: f64,
    low_t: f64,
    high_t: f64,
    low_cp: f64,
    high_cp: f64,
) -> Result<f64, String> {
    if reference_t < low_t || reference_t > high_t || t < reference_t || t > high_t {
        return Err("exergy temperatures are outside the supported range".to_owned());
    }
    let h_t = specific_enthalpy(t, low_t, high_t, low_cp, high_cp)?;
    let h_ref = specific_enthalpy(reference_t, low_t, high_t, low_cp, high_cp)?;
    let slope = (high_cp - low_cp) / (high_t - low_t);
    let intercept = low_cp - slope * low_t;
    let integral_cp_over_t = intercept * (t / reference_t).ln() + slope * (t - reference_t);
    Ok(mass * ((h_t - h_ref) - reference_t * integral_cp_over_t).max(0.0))
}

fn verify_report(json: &str) -> Result<(), String> {
    if text_field(json, "schema")? != "solar-thermal-kernel-report-v0.1" {
        return Err("unsupported report schema".to_owned());
    }
    if text_field(json, "fixture_id")? != "analytic-sensible-store-001" {
        return Err("unexpected fixture_id".to_owned());
    }
    if text_field(json, "claim_level")? != "synthetic-mechanics-only" {
        return Err("report overstates its claim level".to_owned());
    }
    if text_field(json, "source_binding")? != "not-source-hash-bound" {
        return Err("report's source-binding declaration changed".to_owned());
    }
    if !boolean(json, "all_verifiers_passed")? {
        return Err("model-side verifier flag is false".to_owned());
    }
    if text_field(json, "bni_fixture_id")? != "interval-bni-to-mean-dni-001" {
        return Err("unexpected interval-BNI fixture id".to_owned());
    }
    if text_field(json, "risk_fixture_id")? != "scenario-service-risk-001" {
        return Err("unexpected scenario-risk fixture id".to_owned());
    }
    if text_field(json, "dni_fixture_id")? != "direct-beam-interval-001" {
        return Err("unexpected direct-beam fixture id".to_owned());
    }
    if boolean(json, "hardware_validated")? {
        return Err("synthetic fixture must not claim hardware validation".to_owned());
    }

    // Pin all inputs so that a coherently rewritten report cannot silently
    // redefine this canonical fixture and still pass its own equations.
    for (key, expected) in [
        ("incident_solar_j", 1_000_000.0),
        ("dni_direct_normal_irradiance_w_per_m2", 800.0),
        ("dni_aperture_area_m2", 10.0),
        ("dni_incidence_cosine", 0.5),
        ("dni_unshaded_fraction", 0.8),
        ("dni_duration_s", 3_600.0),
        ("bni_interval_irradiation_wh_per_m2", 100.0),
        ("bni_interval_duration_s", 900.0),
        ("risk_service_floor_fraction", 0.8),
        ("risk_scenario_count", 3.0),
        ("risk_scenario_0_id", 10.0),
        ("risk_scenario_0_probability", 0.80),
        ("risk_scenario_0_requested_heat_j", 100.0),
        ("risk_scenario_0_delivered_heat_j", 100.0),
        ("risk_scenario_1_id", 20.0),
        ("risk_scenario_1_probability", 0.17),
        ("risk_scenario_1_requested_heat_j", 100.0),
        ("risk_scenario_1_delivered_heat_j", 80.0),
        ("risk_scenario_2_id", 30.0),
        ("risk_scenario_2_probability", 0.03),
        ("risk_scenario_2_requested_heat_j", 100.0),
        ("risk_scenario_2_delivered_heat_j", 20.0),
        ("optical_efficiency", 0.8),
        ("receiver_loss_j", 100_000.0),
        ("store_mass_kg", 1.0),
        ("store_min_temperature_k", 300.0),
        ("store_max_temperature_k", 500.0),
        ("store_initial_temperature_k", 300.0),
        ("cp_curve_low_temperature_k", 300.0),
        ("cp_curve_low_j_per_kg_k", 1_000.0),
        ("cp_curve_high_temperature_k", 500.0),
        ("cp_curve_high_j_per_kg_k", 2_000.0),
        ("maximum_charge_power_w", 10_000.0),
        ("store_charge_effectiveness", 1.0),
        ("charge_duration_s", 100.0),
        ("maximum_discharge_power_w", 10_000.0),
        ("discharge_duration_s", 100.0),
        ("standing_loss_conductance_w_per_k", 10.0),
        ("standing_loss_ambient_temperature_k", 300.0),
        ("standing_loss_duration_s", 10.0),
        ("exergy_reference_temperature_k", 300.0),
        ("load_supply_temperature_k", 450.0),
        ("minimum_heat_exchanger_approach_k", 10.0),
        ("heat_exchanger_effectiveness", 0.9),
        ("requested_useful_heat_j", 10_000.0),
    ] {
        near(&format!("canonical input {key}"), number(json, key)?, expected)?;
    }

    // Independent direct-beam arithmetic. This fixture is intentionally
    // separate from the downstream collector fixture, whose incident input
    // is a different pinned synthetic value.
    let dni = required_nonnegative(json, "dni_direct_normal_irradiance_w_per_m2")?;
    let aperture = required_nonnegative(json, "dni_aperture_area_m2")?;
    let incidence = number(json, "dni_incidence_cosine")?;
    let unshaded = number(json, "dni_unshaded_fraction")?;
    let dni_duration = required_nonnegative(json, "dni_duration_s")?;
    if !(0.0..=1.0).contains(&incidence) || !(0.0..=1.0).contains(&unshaded) {
        return Err("direct-beam geometry factors must be in [0, 1]".to_owned());
    }
    if aperture <= 0.0 || dni_duration <= 0.0 {
        return Err("direct-beam aperture area and duration must be positive".to_owned());
    }
    let dni_expected_power = dni * aperture * incidence * unshaded;
    let dni_expected_energy = dni_expected_power * dni_duration;
    near("direct-beam incident power", number(json, "dni_incident_power_w")?, dni_expected_power)?;
    near("direct-beam interval energy",
        required_nonnegative(json, "dni_incident_energy_j")?, dni_expected_energy)?;

    // Independently convert interval-integrated BNI to mean DNI.
    let bni_wh_per_m2 = required_nonnegative(json, "bni_interval_irradiation_wh_per_m2")?;
    let bni_duration_s = required_nonnegative(json, "bni_interval_duration_s")?;
    if bni_duration_s <= 0.0 {
        return Err("interval-BNI duration must be positive".to_owned());
    }
    let expected_mean_dni = bni_wh_per_m2 * 3_600.0 / bni_duration_s;
    near(
        "interval BNI to mean DNI",
        required_nonnegative(json, "bni_interval_mean_dni_w_per_m2")?,
        expected_mean_dni,
    )?;

    // Independently recompute the canonical scenario-risk distribution.
    let service_floor = number(json, "risk_service_floor_fraction")?;
    let scenario_count = number(json, "risk_scenario_count")?;
    if scenario_count != 3.0 || !(0.0..=1.0).contains(&service_floor) {
        return Err("unsupported scenario-risk fixture shape".to_owned());
    }
    let mut scenario_rows = Vec::with_capacity(3);
    let mut probability_sum = 0.0;
    let mut expected_unmet = 0.0;
    let mut expected_shortfall = 0.0;
    let mut worst_unmet: f64 = 0.0;
    let mut worst_shortfall: f64 = 0.0;
    let mut floor_violation_probability = 0.0;
    let mut worst_scenario_id = u64::MAX;
    for index in 0..3 {
        let id_raw = number(json, &format!("risk_scenario_{index}_id"))?;
        if id_raw < 0.0 || id_raw.fract() != 0.0 || id_raw > u64::MAX as f64 {
            return Err("scenario id must be a non-negative integer".to_owned());
        }
        let id = id_raw as u64;
        let probability = required_nonnegative(json, &format!("risk_scenario_{index}_probability"))?;
        let requested = required_nonnegative(json, &format!("risk_scenario_{index}_requested_heat_j"))?;
        let delivered = required_nonnegative(json, &format!("risk_scenario_{index}_delivered_heat_j"))?;
        if probability <= 0.0 || probability > 1.0 || delivered > requested {
            return Err("invalid scenario probability or service outcome".to_owned());
        }
        if scenario_rows.iter().any(|row: &(u64, f64, f64, f64, f64)| row.0 == id) {
            return Err("duplicate scenario ID in risk fixture".to_owned());
        }
        probability_sum += probability;
        let unmet = requested - delivered;
        let fraction = if requested == 0.0 { 0.0 } else { unmet / requested };
        expected_unmet += probability * unmet;
        expected_shortfall += probability * fraction;
        if unmet > worst_unmet {
            worst_unmet = unmet;
        }
        if fraction > worst_shortfall || (fraction == worst_shortfall && id < worst_scenario_id) {
            worst_shortfall = fraction;
            worst_scenario_id = id;
        }
        let service = if requested == 0.0 { 1.0 } else { delivered / requested };
        if service < service_floor {
            floor_violation_probability += probability;
        }
        scenario_rows.push((id, probability, requested, delivered, fraction));
    }
    near("risk probability mass", probability_sum, 1.0)?;

    // Weighted empirical P95: smallest shortfall whose cumulative mass reaches 0.95.
    let mut ascending = scenario_rows.clone();
    ascending.sort_by(|a, b| a.4.total_cmp(&b.4).then_with(|| a.0.cmp(&b.0)));
    let mut cumulative = 0.0;
    let mut p95_shortfall = 0.0;
    for row in &ascending {
        cumulative += row.1;
        p95_shortfall = row.4;
        if cumulative >= 0.95 - 1.0e-12 {
            break;
        }
    }

    // CVaR95 is the expected shortfall over exactly the worst 5% probability mass.
    let mut descending = scenario_rows.clone();
    descending.sort_by(|a, b| b.4.total_cmp(&a.4).then_with(|| a.0.cmp(&b.0)));
    let mut remaining_tail = 0.05;
    let mut tail_integral = 0.0;
    for row in &descending {
        if remaining_tail <= 1.0e-12 {
            break;
        }
        let mass = row.1.min(remaining_tail);
        tail_integral += mass * row.4;
        remaining_tail -= mass;
    }
    near("risk CVaR tail mass", remaining_tail, 0.0)?;
    let cvar95_shortfall = tail_integral / 0.05;
    near("risk expected unmet heat",
        required_nonnegative(json, "risk_expected_unmet_useful_heat_j")?, expected_unmet)?;
    near("risk worst unmet heat",
        required_nonnegative(json, "risk_worst_unmet_useful_heat_j")?, worst_unmet)?;
    near("risk mean normalized shortfall", number(json, "risk_expected_shortfall_fraction")?, expected_shortfall)?;
    near("risk worst normalized shortfall", number(json, "risk_worst_shortfall_fraction")?, worst_shortfall)?;
    near("risk weighted P95", number(json, "risk_p95_shortfall_fraction")?, p95_shortfall)?;
    near("risk CVaR95", number(json, "risk_cvar95_shortfall_fraction")?, cvar95_shortfall)?;
    near("risk service-floor violation probability",
        number(json, "risk_probability_below_service_floor")?, floor_violation_probability)?;
    near("risk worst scenario ID", number(json, "risk_worst_case_scenario_id")?, worst_scenario_id as f64)?;

    let incident = required_nonnegative(json, "incident_solar_j")?;
    let optical_efficiency = number(json, "optical_efficiency")?;
    let optical_loss = required_nonnegative(json, "optical_loss_j")?;
    let absorbed = required_nonnegative(json, "absorbed_solar_j")?;
    let receiver_loss = required_nonnegative(json, "receiver_loss_j")?;
    let captured = required_nonnegative(json, "useful_captured_heat_j")?;
    near("absorbed solar formula", absorbed, incident * optical_efficiency)?;
    near("optical loss formula", optical_loss, incident - absorbed)?;
    near("absorbed = receiver loss + captured heat", absorbed, receiver_loss + captured)?;
    near("incident solar ledger closure", incident, optical_loss + receiver_loss + captured)?;

    let mass = number(json, "store_mass_kg")?;
    let min_t = number(json, "store_min_temperature_k")?;
    let max_t = number(json, "store_max_temperature_k")?;
    let initial_t = number(json, "store_initial_temperature_k")?;
    let low_t = number(json, "cp_curve_low_temperature_k")?;
    let high_t = number(json, "cp_curve_high_temperature_k")?;
    let low_cp = number(json, "cp_curve_low_j_per_kg_k")?;
    let high_cp = number(json, "cp_curve_high_j_per_kg_k")?;
    let max_charge_power = number(json, "maximum_charge_power_w")?;
    let charge_effectiveness = number(json, "store_charge_effectiveness")?;
    let charge_duration = number(json, "charge_duration_s")?;
    let max_discharge_power = number(json, "maximum_discharge_power_w")?;
    let discharge_duration = number(json, "discharge_duration_s")?;
    let conductance = number(json, "standing_loss_conductance_w_per_k")?;
    let ambient_t = number(json, "standing_loss_ambient_temperature_k")?;
    let loss_duration = number(json, "standing_loss_duration_s")?;
    let reference_t = number(json, "exergy_reference_temperature_k")?;
    if mass <= 0.0 || min_t >= max_t || min_t != low_t || max_t != high_t
        || initial_t != min_t || !(0.0..=1.0).contains(&optical_efficiency)
        || !(0.0..=1.0).contains(&charge_effectiveness) || charge_effectiveness == 0.0
        || max_charge_power < 0.0 || max_discharge_power < 0.0
        || charge_duration <= 0.0 || discharge_duration <= 0.0 || loss_duration <= 0.0
        || conductance < 0.0 || ambient_t <= 0.0
    {
        return Err("invalid store or operating assumptions".to_owned());
    }

    let initial_energy = total_store_energy(initial_t, mass, low_t, high_t, low_cp, high_cp)?;
    let capacity = total_store_energy(max_t, mass, low_t, high_t, low_cp, high_cp)?;
    let accepted = required_nonnegative(json, "store_energy_accepted_j")?;
    let rejected = required_nonnegative(json, "store_energy_rejected_j")?;
    let t_charge = number(json, "store_temperature_after_charge_k")?;
    let expected_accepted = captured
        .min(max_charge_power * charge_duration)
        .min((capacity - initial_energy).max(0.0));
    near("accepted charge input", accepted, expected_accepted)?;
    near("rejected charge input", rejected, captured - expected_accepted)?;
    near(
        "stored energy after charge",
        total_store_energy(t_charge, mass, low_t, high_t, low_cp, high_cp)? - initial_energy,
        accepted * charge_effectiveness,
    )?;

    let loss = required_nonnegative(json, "standing_loss_j")?;
    let energy_at_charge = total_store_energy(t_charge, mass, low_t, high_t, low_cp, high_cp)?;
    let expected_loss = (conductance * (t_charge - ambient_t).max(0.0) * loss_duration)
        .min(energy_at_charge);
    near("standing loss model", loss, expected_loss)?;
    let t_after_loss = number(json, "store_temperature_after_loss_k")?;
    let energy_after_loss = total_store_energy(t_after_loss, mass, low_t, high_t, low_cp, high_cp)?;
    near("stored energy after standing loss", energy_after_loss, energy_at_charge - loss)?;

    let before_discharge_exergy = required_nonnegative(json, "stored_exergy_before_discharge_j")?;
    near(
        "pre-discharge sensible exergy",
        before_discharge_exergy,
        total_store_exergy(t_after_loss, reference_t, mass, low_t, high_t, low_cp, high_cp)?,
    )?;

    let supply_t = number(json, "load_supply_temperature_k")?;
    let approach = number(json, "minimum_heat_exchanger_approach_k")?;
    let effectiveness = number(json, "heat_exchanger_effectiveness")?;
    let requested = required_nonnegative(json, "requested_useful_heat_j")?;
    let withdrawn = required_nonnegative(json, "source_heat_withdrawn_j")?;
    let delivered = required_nonnegative(json, "useful_heat_delivered_j")?;
    let exchanger_loss = required_nonnegative(json, "heat_exchanger_loss_j")?;
    let unmet = required_nonnegative(json, "unmet_useful_heat_j")?;
    if effectiveness <= 0.0 || effectiveness > 1.0 || approach < 0.0 {
        return Err("invalid heat-exchanger assumptions".to_owned());
    }

    let cutoff = supply_t + approach;
    let energy_above_cutoff = if cutoff <= max_t {
        (energy_after_loss
            - total_store_energy(cutoff.max(min_t), mass, low_t, high_t, low_cp, high_cp)?).max(0.0)
    } else {
        0.0
    };
    let expected_withdrawn = if requested > 0.0 && t_after_loss > cutoff {
        (requested / effectiveness)
            .min(max_discharge_power * discharge_duration)
            .min(energy_above_cutoff)
    } else {
        0.0
    };
    near("source heat withdrawn", withdrawn, expected_withdrawn)?;
    near("source heat split", withdrawn, delivered + exchanger_loss)?;
    near("heat-exchanger effectiveness", delivered, (withdrawn * effectiveness).min(requested))?;
    near("unmet load", unmet, requested - delivered)?;
    if withdrawn > 0.0 && t_after_loss < cutoff {
        return Err("discharge occurred below the required temperature approach".to_owned());
    }

    let final_t = number(json, "final_store_temperature_k")?;
    near(
        "final stored energy",
        total_store_energy(final_t, mass, low_t, high_t, low_cp, high_cp)?,
        energy_after_loss - withdrawn,
    )?;
    if withdrawn > 0.0 && final_t < cutoff {
        return Err("final store temperature fell below the required temperature approach".to_owned());
    }
    let final_exergy = required_nonnegative(json, "final_store_exergy_j")?;
    near(
        "final sensible exergy",
        final_exergy,
        total_store_exergy(final_t, reference_t, mass, low_t, high_t, low_cp, high_cp)?,
    )?;

    // Independent two-store cascade audit. Both stores use constant cp(T), so
    // the expected temperatures and energy limits are closed-form, hand-checkable.
    if text_field(json, "cascade_fixture_id")? != "two-temperature-sensible-001" {
        return Err("unexpected cascade fixture id".to_owned());
    }
    for (key, expected) in [
        ("cascade_high_store_mass_kg", 1.0),
        ("cascade_high_store_min_temperature_k", 300.0),
        ("cascade_high_store_max_temperature_k", 500.0),
        ("cascade_high_store_initial_temperature_k", 450.0),
        ("cascade_high_cp_low_temperature_k", 300.0),
        ("cascade_high_cp_low_j_per_kg_k", 1_000.0),
        ("cascade_high_cp_high_temperature_k", 500.0),
        ("cascade_high_cp_high_j_per_kg_k", 1_000.0),
        ("cascade_high_max_discharge_power_w", 10_000.0),
        ("cascade_low_store_mass_kg", 1.0),
        ("cascade_low_store_min_temperature_k", 300.0),
        ("cascade_low_store_max_temperature_k", 360.0),
        ("cascade_low_store_initial_temperature_k", 300.0),
        ("cascade_low_cp_low_temperature_k", 300.0),
        ("cascade_low_cp_low_j_per_kg_k", 1_000.0),
        ("cascade_low_cp_high_temperature_k", 360.0),
        ("cascade_low_cp_high_j_per_kg_k", 1_000.0),
        ("cascade_requested_low_store_heat_j", 40_000.0),
        ("cascade_minimum_approach_k", 10.0),
        ("cascade_heat_exchanger_effectiveness", 0.8),
        ("cascade_duration_s", 10.0),
    ] {
        near(&format!("canonical cascade input {key}"), number(json, key)?, expected)?;
    }

    let high_mass = number(json, "cascade_high_store_mass_kg")?;
    let high_min_t = number(json, "cascade_high_store_min_temperature_k")?;
    let high_max_t = number(json, "cascade_high_store_max_temperature_k")?;
    let high_initial_t = number(json, "cascade_high_store_initial_temperature_k")?;
    let high_cp_low = number(json, "cascade_high_cp_low_j_per_kg_k")?;
    let high_cp_high = number(json, "cascade_high_cp_high_j_per_kg_k")?;
    let high_power = number(json, "cascade_high_max_discharge_power_w")?;
    let low_mass = number(json, "cascade_low_store_mass_kg")?;
    let low_min_t = number(json, "cascade_low_store_min_temperature_k")?;
    let low_max_t = number(json, "cascade_low_store_max_temperature_k")?;
    let low_initial_t = number(json, "cascade_low_store_initial_temperature_k")?;
    let low_cp_low = number(json, "cascade_low_cp_low_j_per_kg_k")?;
    let low_cp_high = number(json, "cascade_low_cp_high_j_per_kg_k")?;
    let cascade_requested = required_nonnegative(json, "cascade_requested_low_store_heat_j")?;
    let cascade_approach = number(json, "cascade_minimum_approach_k")?;
    let cascade_effectiveness = number(json, "cascade_heat_exchanger_effectiveness")?;
    let cascade_duration = number(json, "cascade_duration_s")?;
    if high_mass <= 0.0 || low_mass <= 0.0
        || high_cp_low <= 0.0 || high_cp_high <= 0.0
        || low_cp_low <= 0.0 || low_cp_high <= 0.0
        || (high_cp_low - high_cp_high).abs() > 1.0e-10
        || (low_cp_low - low_cp_high).abs() > 1.0e-10
        || high_min_t >= high_max_t || low_min_t >= low_max_t
        || cascade_approach < 0.0
        || cascade_effectiveness <= 0.0 || cascade_effectiveness > 1.0
        || cascade_duration <= 0.0 || high_power < 0.0
    {
        return Err("invalid cascade fixture assumptions".to_owned());
    }

    let cutoff = low_max_t + cascade_approach;
    near("cascade minimum source temperature", number(json, "cascade_minimum_source_temperature_k")?, cutoff)?;
    if cutoff > high_max_t || high_initial_t <= cutoff {
        return Err("canonical cascade source lacks required temperature grade".to_owned());
    }
    if boolean(json, "cascade_blocked_by_temperature")? {
        return Err("canonical cascade fixture unexpectedly blocked".to_owned());
    }

    let high_energy_above_cutoff = high_mass * high_cp_low * (high_initial_t - cutoff);
    let source_power_budget = high_power * cascade_duration;
    let low_store_room = low_mass * low_cp_low * (low_max_t - low_initial_t);
    let expected_withdrawn = high_energy_above_cutoff
        .min(source_power_budget)
        .min(low_store_room / cascade_effectiveness)
        .min(cascade_requested / cascade_effectiveness);
    let expected_accepted = expected_withdrawn * cascade_effectiveness;
    let expected_loss = expected_withdrawn - expected_accepted;
    let expected_unmet = cascade_requested - expected_accepted;
    let expected_high_final_t = high_initial_t
        - expected_withdrawn / (high_mass * high_cp_low);
    let expected_low_final_t = low_initial_t
        + expected_accepted / (low_mass * low_cp_low);

    near("cascade source heat withdrawn",
        required_nonnegative(json, "cascade_source_heat_withdrawn_j")?,
        expected_withdrawn)?;
    near("cascade low-store heat accepted",
        required_nonnegative(json, "cascade_low_store_heat_accepted_j")?,
        expected_accepted)?;
    near("cascade exchanger loss",
        required_nonnegative(json, "cascade_heat_exchanger_loss_j")?,
        expected_loss)?;
    near("cascade unmet request",
        required_nonnegative(json, "cascade_unmet_low_store_heat_j")?,
        expected_unmet)?;
    near("cascade high-store final temperature",
        number(json, "cascade_high_store_final_temperature_k")?,
        expected_high_final_t)?;
    near("cascade low-store final temperature",
        number(json, "cascade_low_store_final_temperature_k")?,
        expected_low_final_t)?;
    near("cascade total energy closure",
        high_mass * high_cp_low * (high_initial_t - high_min_t)
            + low_mass * low_cp_low * (low_initial_t - low_min_t),
        high_mass * high_cp_low * (expected_high_final_t - high_min_t)
            + low_mass * low_cp_low * (expected_low_final_t - low_min_t)
            + expected_loss)?;
    near("cascade request accounting",
        cascade_requested,
        expected_accepted + expected_unmet)?;

    // Independently recompute the canonical single-hot/single-cold pinch case.
    if text_field(json, "pinch_fixture_id")? != "static-sensible-pinch-001" {
        return Err("unexpected pinch fixture id".to_owned());
    }
    for (key, expected) in [
        ("pinch_hot_supply_temperature_k", 423.15),
        ("pinch_hot_target_temperature_k", 323.15),
        ("pinch_hot_available_heat_j", 100_000.0),
        ("pinch_cold_supply_temperature_k", 393.15),
        ("pinch_cold_target_temperature_k", 493.15),
        ("pinch_cold_required_heat_j", 120_000.0),
        ("pinch_minimum_approach_k", 20.0),
    ] {
        near(&format!("canonical pinch input {key}"), number(json, key)?, expected)?;
    }
    let hot_supply = number(json, "pinch_hot_supply_temperature_k")?;
    let hot_target = number(json, "pinch_hot_target_temperature_k")?;
    let hot_duty = required_nonnegative(json, "pinch_hot_available_heat_j")?;
    let cold_supply = number(json, "pinch_cold_supply_temperature_k")?;
    let cold_target = number(json, "pinch_cold_target_temperature_k")?;
    let cold_duty = required_nonnegative(json, "pinch_cold_required_heat_j")?;
    let dt_min = number(json, "pinch_minimum_approach_k")?;
    if hot_supply <= hot_target || cold_target <= cold_supply || dt_min < 0.0 {
        return Err("invalid pinch stream direction or minimum approach".to_owned());
    }

    let half_dt = dt_min / 2.0;
    let hot_high = hot_supply - half_dt;
    let hot_low = hot_target - half_dt;
    let cold_high = cold_target + half_dt;
    let cold_low = cold_supply + half_dt;
    let hot_integrated_cp = hot_duty / (hot_supply - hot_target);
    let cold_integrated_cp = cold_duty / (cold_target - cold_supply);

    // This fixture has exactly one hot and one cold stream. Construct shifted
    // endpoints from the source temperatures, then independently sweep intervals.
    let mut endpoints = vec![hot_high, hot_low, cold_high, cold_low];
    endpoints.sort_by(|a, b| b.total_cmp(a));
    endpoints.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-10);
    let mut cumulative = 0.0_f64;
    let mut minimum_cumulative = 0.0_f64;
    let mut interval_count = 0.0_f64;
    let mut pinch_shifted = Vec::new();
    for bounds in endpoints.windows(2) {
        let top = bounds[0];
        let bottom = bounds[1];
        let span = top - bottom;
        if span <= 1.0e-10 {
            continue;
        }
        let hot_active = hot_high >= top && hot_low <= bottom;
        let cold_active = cold_high >= top && cold_low <= bottom;
        let hot_cp = if hot_active { hot_integrated_cp } else { 0.0 };
        let cold_cp = if cold_active { cold_integrated_cp } else { 0.0 };
        cumulative += (hot_cp - cold_cp) * span;
        minimum_cumulative = minimum_cumulative.min(cumulative);
        interval_count += 1.0;
    }
    let expected_hot_utility = (-minimum_cumulative).max(0.0);
    let expected_cold_utility = (expected_hot_utility + cumulative).max(0.0);
    let expected_recovery = (hot_duty - expected_cold_utility)
        .min(cold_duty - expected_hot_utility)
        .max(0.0);

    // Re-run the interval sweep to identify all boundaries where the adjusted
    // cascade reaches zero, rather than trusting the reported pinch coordinate.
    cumulative = 0.0;
    for bounds in endpoints.windows(2) {
        let top = bounds[0];
        let bottom = bounds[1];
        let span = top - bottom;
        if span <= 1.0e-10 {
            continue;
        }
        let hot_cp = if hot_high >= top && hot_low <= bottom { hot_integrated_cp } else { 0.0 };
        let cold_cp = if cold_high >= top && cold_low <= bottom { cold_integrated_cp } else { 0.0 };
        cumulative += (hot_cp - cold_cp) * span;
        if expected_hot_utility > 0.0
            && (cumulative + expected_hot_utility).abs()
                <= 5.0e-8 * cumulative.abs().max(expected_hot_utility.abs()).max(1.0)
        {
            pinch_shifted.push(bottom);
        }
    }

    near("pinch minimum hot utility",
        required_nonnegative(json, "pinch_minimum_hot_utility_j")?,
        expected_hot_utility)?;
    near("pinch minimum cold utility",
        required_nonnegative(json, "pinch_minimum_cold_utility_j")?,
        expected_cold_utility)?;
    near("pinch maximum heat recovery",
        required_nonnegative(json, "pinch_maximum_process_heat_recovery_j")?,
        expected_recovery)?;
    near("pinch interval count", number(json, "pinch_interval_count")?, interval_count)?;
    near("pinch point count", number(json, "pinch_point_count")?, pinch_shifted.len() as f64)?;
    near("pinch global first-law balance",
        hot_duty + expected_hot_utility,
        cold_duty + expected_cold_utility)?;
    let expected_shifted = pinch_shifted.first().copied().unwrap_or(0.0);
    near("pinch shifted temperature", number(json, "pinch_shifted_temperature_k")?, expected_shifted)?;
    near("pinch hot-side temperature",
        number(json, "pinch_hot_side_temperature_k")?,
        if expected_shifted > 0.0 { expected_shifted + half_dt } else { 0.0 })?;
    near("pinch cold-side temperature",
        number(json, "pinch_cold_side_temperature_k")?,
        if expected_shifted > 0.0 { expected_shifted - half_dt } else { 0.0 })?;

    // Independently recompute the fixed two-period day/night storage fixture
    // from scalar inputs. No temporal kernel functions are called here.
    if text_field(json, "temporal_fixture_id")? != "analytic-day-night-storage-001" {
        return Err("unexpected temporal fixture id".to_owned());
    }
    for (key, expected) in [
        ("temporal_store_mass_kg", 10.0),
        ("temporal_store_min_temperature_k", 300.0),
        ("temporal_store_max_temperature_k", 400.0),
        ("temporal_store_initial_temperature_k", 300.0),
        ("temporal_cp_j_per_kg_k", 1_000.0),
        ("temporal_max_charge_power_w", 5_000.0),
        ("temporal_max_discharge_power_w", 1_000.0),
        ("temporal_standing_loss_conductance_w_per_k", 0.1),
        ("temporal_period_duration_s", 3_600.0),
        ("temporal_ambient_temperature_k", 300.0),
        ("temporal_charge_available_heat_j", 900_000.0),
        ("temporal_load_supply_temperature_k", 340.0),
        ("temporal_minimum_approach_k", 10.0),
        ("temporal_heat_exchanger_effectiveness", 0.9),
        ("temporal_requested_useful_heat_j", 300_000.0),
    ] {
        near(&format!("canonical temporal input {key}"), number(json, key)?, expected)?;
    }
    let store_mass = number(json, "temporal_store_mass_kg")?;
    let store_min = number(json, "temporal_store_min_temperature_k")?;
    let store_max = number(json, "temporal_store_max_temperature_k")?;
    let initial_t = number(json, "temporal_store_initial_temperature_k")?;
    let cp = number(json, "temporal_cp_j_per_kg_k")?;
    let charge_power = number(json, "temporal_max_charge_power_w")?;
    let discharge_power = number(json, "temporal_max_discharge_power_w")?;
    let conductance = number(json, "temporal_standing_loss_conductance_w_per_k")?;
    let duration = number(json, "temporal_period_duration_s")?;
    let ambient = number(json, "temporal_ambient_temperature_k")?;
    let offered_charge = required_nonnegative(json, "temporal_charge_available_heat_j")?;
    let supply = number(json, "temporal_load_supply_temperature_k")?;
    let approach = number(json, "temporal_minimum_approach_k")?;
    let effectiveness = number(json, "temporal_heat_exchanger_effectiveness")?;
    let requested = required_nonnegative(json, "temporal_requested_useful_heat_j")?;
    if store_mass <= 0.0 || cp <= 0.0 || store_min >= store_max
        || initial_t < store_min || initial_t > store_max
        || charge_power < 0.0 || discharge_power < 0.0 || conductance < 0.0
        || duration <= 0.0 || ambient <= 0.0 || approach < 0.0
        || effectiveness <= 0.0 || effectiveness > 1.0
    {
        return Err("invalid temporal store or dispatch assumptions".to_owned());
    }
    let heat_capacity_j_per_k = store_mass * cp;
    let capacity_j = heat_capacity_j_per_k * (store_max - store_min);
    let initial_energy = heat_capacity_j_per_k * (initial_t - store_min);
    let period_0_loss = (conductance * (initial_t - ambient).max(0.0) * duration)
        .min(initial_energy);
    let energy_before_charge = (initial_energy - period_0_loss).max(0.0);
    let expected_charge = offered_charge
        .min(charge_power * duration)
        .min((capacity_j - energy_before_charge).max(0.0));
    let expected_rejected = offered_charge - expected_charge;
    let energy_after_charge = energy_before_charge + expected_charge;
    let t_after_charge = store_min + energy_after_charge / heat_capacity_j_per_k;
    near("temporal period 0 input temperature",
        number(json, "temporal_period_0_temperature_before_k")?, initial_t)?;
    near("temporal period 0 loss-adjusted temperature",
        number(json, "temporal_period_0_temperature_after_loss_k")?,
        store_min + energy_before_charge / heat_capacity_j_per_k)?;
    near("temporal period 0 post-charge temperature",
        number(json, "temporal_period_0_temperature_after_action_k")?, t_after_charge)?;
    near("temporal period 0 standing loss",
        required_nonnegative(json, "temporal_period_0_standing_loss_j")?, period_0_loss)?;
    near("temporal period 0 requested charge",
        required_nonnegative(json, "temporal_period_0_charge_requested_j")?, offered_charge)?;
    near("temporal period 0 accepted charge",
        required_nonnegative(json, "temporal_period_0_charge_accepted_j")?, expected_charge)?;
    near("temporal period 0 rejected charge",
        required_nonnegative(json, "temporal_period_0_charge_rejected_j")?, expected_rejected)?;
    near("temporal period 0 charge balance",
        energy_before_charge + expected_charge, heat_capacity_j_per_k * (t_after_charge - store_min))?;

    let energy_after_loss = (energy_after_charge
        - (conductance * (t_after_charge - ambient).max(0.0) * duration)
            .min(energy_after_charge)).max(0.0);
    let period_1_loss = energy_after_charge - energy_after_loss;
    let t_after_loss = store_min + energy_after_loss / heat_capacity_j_per_k;
    let cutoff = supply + approach;
    let energy_above_cutoff = if cutoff <= store_max && t_after_loss > cutoff {
        (energy_after_loss - heat_capacity_j_per_k * (cutoff - store_min)).max(0.0)
    } else {
        0.0
    };
    let expected_withdrawn = if t_after_loss > cutoff {
        (requested / effectiveness)
            .min(discharge_power * duration)
            .min(energy_above_cutoff)
    } else {
        0.0
    };
    let expected_delivered = (expected_withdrawn * effectiveness).min(requested);
    let expected_exchange_loss = (expected_withdrawn - expected_delivered).max(0.0);
    let expected_unmet = (requested - expected_delivered).max(0.0);
    let energy_final = (energy_after_loss - expected_withdrawn).max(0.0);
    let t_final = store_min + energy_final / heat_capacity_j_per_k;

    near("temporal period 1 input temperature",
        number(json, "temporal_period_1_temperature_before_k")?, t_after_charge)?;
    near("temporal period 1 loss-adjusted temperature",
        number(json, "temporal_period_1_temperature_after_loss_k")?, t_after_loss)?;
    near("temporal period 1 post-discharge temperature",
        number(json, "temporal_period_1_temperature_after_action_k")?, t_final)?;
    near("temporal period 1 standing loss",
        required_nonnegative(json, "temporal_period_1_standing_loss_j")?, period_1_loss)?;
    near("temporal period 1 requested useful heat",
        required_nonnegative(json, "temporal_period_1_useful_heat_requested_j")?, requested)?;
    near("temporal period 1 source heat withdrawn",
        required_nonnegative(json, "temporal_period_1_source_heat_withdrawn_j")?, expected_withdrawn)?;
    near("temporal period 1 useful heat delivered",
        required_nonnegative(json, "temporal_period_1_useful_heat_delivered_j")?, expected_delivered)?;
    near("temporal period 1 exchanger loss",
        required_nonnegative(json, "temporal_period_1_heat_exchanger_loss_j")?, expected_exchange_loss)?;
    near("temporal period 1 unmet useful heat",
        required_nonnegative(json, "temporal_period_1_unmet_useful_heat_j")?, expected_unmet)?;
    if boolean(json, "temporal_period_1_blocked_by_temperature")? != (t_after_loss <= cutoff) {
        return Err("temporal discharge temperature-gate flag mismatch".to_owned());
    }

    near("temporal aggregate initial energy",
        required_nonnegative(json, "temporal_initial_stored_energy_j")?, initial_energy)?;
    near("temporal aggregate final energy",
        required_nonnegative(json, "temporal_final_stored_energy_j")?, energy_final)?;
    near("temporal aggregate accepted charge",
        required_nonnegative(json, "temporal_accepted_charge_heat_j")?, expected_charge)?;
    near("temporal aggregate rejected charge",
        required_nonnegative(json, "temporal_rejected_charge_heat_j")?, expected_rejected)?;
    near("temporal aggregate source withdrawal",
        required_nonnegative(json, "temporal_source_heat_withdrawn_j")?, expected_withdrawn)?;
    near("temporal aggregate useful delivery",
        required_nonnegative(json, "temporal_useful_heat_delivered_j")?, expected_delivered)?;
    near("temporal aggregate exchanger loss",
        required_nonnegative(json, "temporal_heat_exchanger_loss_j")?, expected_exchange_loss)?;
    near("temporal aggregate unmet service",
        required_nonnegative(json, "temporal_unmet_useful_heat_j")?, expected_unmet)?;
    near("temporal aggregate standing loss",
        required_nonnegative(json, "temporal_standing_heat_loss_j")?, period_0_loss + period_1_loss)?;
    let residual = initial_energy + expected_charge - period_0_loss - period_1_loss
        - expected_withdrawn - energy_final;
    near("temporal global first-law residual",
        number(json, "temporal_energy_balance_residual_j")?, residual)?;
    near("temporal global first-law closure",
        initial_energy + expected_charge,
        energy_final + period_0_loss + period_1_loss + expected_withdrawn)?;
    near("temporal service balance", requested, expected_delivered + expected_unmet)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut report = String::new();
    io::stdin().read_to_string(&mut report)?;
    verify_report(&report).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    println!("independent thermal report audit: PASS");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_fixture_passes_independent_audit() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        verify_report(&report).unwrap();
    }

    #[test]
    fn catches_tampered_useful_heat_in_report() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"useful_captured_heat_j\":700000.000000000",
            "\"useful_captured_heat_j\":701000.000000000",
        );
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn catches_tampered_cascade_transfer() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"cascade_low_store_heat_accepted_j\":40000.000000000",
            "\"cascade_low_store_heat_accepted_j\":41000.000000000",
        );
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn catches_tampered_pinch_utility_target() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"pinch_minimum_hot_utility_j\":110000.000000000",
            "\"pinch_minimum_hot_utility_j\":109000.000000000",
        );
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn catches_tampered_temporal_dispatch_delivery() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"temporal_period_1_useful_heat_delivered_j\":300000.000000000",
            "\"temporal_period_1_useful_heat_delivered_j\":301000.000000000",
        );
        assert_ne!(tampered, report);
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn catches_tampered_direct_beam_energy() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"dni_incident_energy_j\":11520000.000000000",
            "\"dni_incident_energy_j\":11520001.000000000",
        );
        assert_ne!(tampered, report);
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn catches_tampered_direct_beam_input() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"dni_direct_normal_irradiance_w_per_m2\":800.000000000",
            "\"dni_direct_normal_irradiance_w_per_m2\":801.000000000",
        );
        assert_ne!(tampered, report);
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn catches_tampered_interval_bni_conversion_output() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"bni_interval_mean_dni_w_per_m2\":400.000000000",
            "\"bni_interval_mean_dni_w_per_m2\":401.000000000",
        );
        assert_ne!(tampered, report);
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn catches_tampered_interval_bni_input() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"bni_interval_irradiation_wh_per_m2\":100.000000000",
            "\"bni_interval_irradiation_wh_per_m2\":101.000000000",
        );
        assert_ne!(tampered, report);
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn catches_tampered_scenario_risk_metric() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"risk_cvar95_shortfall_fraction\":0.560000000",
            "\"risk_cvar95_shortfall_fraction\":0.500000000",
        );
        assert_ne!(tampered, report);
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn catches_tampered_scenario_outcome() {
        let report = solar_thermal_kernel::canonical_demo_receipt().unwrap();
        let tampered = report.replace(
            "\"risk_scenario_2_delivered_heat_j\":20.000000000",
            "\"risk_scenario_2_delivered_heat_j\":21.000000000",
        );
        assert_ne!(tampered, report);
        assert!(verify_report(&tampered).is_err());
    }

    #[test]
    fn rejects_duplicate_fields_in_report() {
        let json = "{\"key\":1,\"key\":2}";
        assert!(scalar(json, "key").is_err());
    }

    #[test]
    fn rejects_missing_ambient_temperature_assumption() {
        let json = "{\"key\":1}";
        assert!(number(json, "standing_loss_ambient_temperature_k").is_err());
    }
}
