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
    let end = tail.find(|ch| ch == ',' || ch == '}')
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
    if !(low_t < high_t) || low_cp <= 0.0 || high_cp <= 0.0 || t < low_t || t > high_t {
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
    if boolean(json, "hardware_validated")? {
        return Err("synthetic fixture must not claim hardware validation".to_owned());
    }

    // Pin all inputs so that a coherently rewritten report cannot silently
    // redefine this canonical fixture and still pass its own equations.
    for (key, expected) in [
        ("incident_solar_j", 1_000_000.0),
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
        || initial_t != min_t || optical_efficiency < 0.0 || optical_efficiency > 1.0
        || charge_effectiveness <= 0.0 || charge_effectiveness > 1.0
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
