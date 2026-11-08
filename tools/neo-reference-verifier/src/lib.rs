use serde_json::Value;
use std::collections::BTreeSet;

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing required field: {key}"))
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    required(value, key)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| format!("{key} must be a non-empty string"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    let parsed = required(value, key)?
        .as_f64()
        .ok_or_else(|| format!("{key} must be a number"))?;
    if !parsed.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(parsed)
}

fn approx(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

pub fn validate_case(case: &Value) -> Result<(), String> {
    if string(case, "schema_version")? != "0.1.0" {
        return Err("unsupported schema_version".into());
    }
    if string(case, "case_type")? != "historical_mission_observation_replay" {
        return Err("unsupported case_type".into());
    }
    let case_id = string(case, "case_id")?;
    if !case_id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || ".-_".contains(c)) {
        return Err("case_id contains unsupported characters".into());
    }
    if string(case, "claim_ceiling")? != "historical_reference_only_not_operational_validation" {
        return Err("claim_ceiling must preserve the historical-reference-only boundary".into());
    }

    let target = required(case, "target")?;
    string(target, "designation")?;
    if string(target, "name")? != "Dimorphos" ||
       string(target, "primary")? != "65803 Didymos" {
        return Err("unexpected target identity".into());
    }

    let event = required(case, "event")?;
    string(event, "epoch_utc")?;
    if string(event, "event_type")? != "kinetic_impact" ||
       string(event, "mission")? != "NASA Double Asteroid Redirection Test (DART)" {
        return Err("unexpected event identity".into());
    }

    let sources = required(case, "sources")?
        .as_array()
        .ok_or_else(|| "sources must be an array".to_string())?;
    let mut source_ids = BTreeSet::new();
    for source in sources {
        let id = string(source, "source_id")?;
        let url = string(source, "url")?;
        if !url.starts_with("https://") {
            return Err(format!("source {id} must use HTTPS"));
        }
        string(source, "title")?;
        if !source_ids.insert(id.to_owned()) {
            return Err(format!("duplicate source_id: {id}"));
        }
    }
    if source_ids.len() < 2 {
        return Err("at least two separately identified source records are required".into());
    }

    let observables = required(case, "reference_observables")?
        .as_array()
        .ok_or_else(|| "reference_observables must be an array".to_string())?;
    let mut seen = BTreeSet::new();
    for item in observables {
        let id = string(item, "observable_id")?;
        let unit = string(item, "unit")?;
        let value = number(item, "value")?;
        let uncertainty = required(item, "uncertainty")?;
        let uncertainty_value = number(uncertainty, "value")?;
        let uncertainty_unit = string(uncertainty, "unit")?;
        let sigma = number(uncertainty, "sigma_multiplier")?;
        if uncertainty_value <= 0.0 || sigma <= 0.0 {
            return Err(format!("{id}: uncertainty and sigma_multiplier must be positive"));
        }
        if unit != uncertainty_unit {
            return Err(format!("{id}: uncertainty unit does not match observable unit"));
        }
        let source_id = string(item, "source_id")?;
        if !source_ids.contains(source_id) {
            return Err(format!("{id}: source_id is not present in sources"));
        }
        if !seen.insert(id.to_owned()) {
            return Err(format!("duplicate observable_id: {id}"));
        }

        match id {
            "dimorphos_binary_period_change" => {
                if unit != "min" || !approx(value, -33.0, 1e-9) ||
                   !approx(uncertainty_value, 1.0, 1e-9) || !approx(sigma, 3.0, 1e-9) {
                    return Err("period-change reference anchor or uncertainty convention changed".into());
                }
                if string(item, "evidence_status")? != "directly_observed" {
                    return Err("period change must remain labeled directly observed".into());
                }
            }
            "dimorphos_along_track_velocity_change" => {
                if unit != "mm/s" || !approx(value, -2.70, 1e-9) ||
                   !approx(uncertainty_value, 0.10, 1e-9) || !approx(sigma, 1.0, 1e-9) {
                    return Err("along-track velocity reference anchor or uncertainty convention changed".into());
                }
                if string(item, "evidence_status")? != "derived_from_observations_and_geometry" {
                    return Err("along-track velocity change must remain labeled derived".into());
                }
            }
            _ => return Err(format!("unsupported observable_id: {id}")),
        }
        string(item, "method")?;
    }
    if seen.len() != 2 {
        return Err("both reference observables must be present exactly once".into());
    }

    let inferences = required(case, "derived_inferences")?
        .as_array()
        .ok_or_else(|| "derived_inferences must be an array".to_string())?;
    if inferences.len() != 1 {
        return Err("exactly one conditional beta inference is expected in v0.1".into());
    }
    let beta = &inferences[0];
    if string(beta, "inference_id")? != "momentum_enhancement_factor_beta" ||
       string(beta, "unit")? != "dimensionless" ||
       !approx(number(beta, "value")?, 3.61, 1e-9) {
        return Err("conditional beta reference anchor changed".into());
    }
    let beta_source_id = string(beta, "source_id")?;
    if !source_ids.contains(beta_source_id) {
        return Err("beta inference source_id is not present in sources".into());
    }
    let asym = required(beta, "asymmetric_uncertainty")?;
    if !approx(number(asym, "lower")?, 0.25, 1e-9) ||
       !approx(number(asym, "upper")?, 0.19, 1e-9) ||
       !approx(number(asym, "sigma_multiplier")?, 1.0, 1e-9) {
        return Err("beta asymmetric uncertainty convention changed".into());
    }
    let conditioning = required(beta, "conditioning")?;
    if !approx(number(conditioning, "dimorphos_bulk_density_kg_m3")?, 2400.0, 1e-9) {
        return Err("beta must remain conditional on the published equal-density assumption".into());
    }
    string(conditioning, "interpretation")?;

    let sensitivity = required(beta, "sensitivity")?;
    let density = required(sensitivity, "density_range_kg_m3")?
        .as_array().ok_or_else(|| "density_range_kg_m3 must be an array".to_string())?;
    let beta_range = required(sensitivity, "beta_range")?
        .as_array().ok_or_else(|| "beta_range must be an array".to_string())?;
    if density.len() != 2 || beta_range.len() != 2 ||
       !approx(density[0].as_f64().unwrap_or(f64::NAN), 1500.0, 1e-9) ||
       !approx(density[1].as_f64().unwrap_or(f64::NAN), 3300.0, 1e-9) ||
       !approx(beta_range[0].as_f64().unwrap_or(f64::NAN), 2.2, 1e-9) ||
       !approx(beta_range[1].as_f64().unwrap_or(f64::NAN), 4.9, 1e-9) {
        return Err("beta sensitivity range changed; review source and uncertainty semantics".into());
    }
    string(sensitivity, "interpretation")?;

    let not_established = required(case, "not_established")?
        .as_array().ok_or_else(|| "not_established must be an array".to_string())?;
    if not_established.len() < 3 {
        return Err("at least three explicit non-claims are required".into());
    }
    for item in not_established {
        if item.as_str().map(str::trim).unwrap_or("").len() < 8 {
            return Err("not_established entries must be explicit non-empty claims".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../../docs/space-engineering/reference-cases/dart-dimorphos-v0.1.json"))
            .expect("frozen DART fixture must parse")
    }

    #[test]
    fn accepts_frozen_dart_reference_case() {
        validate_case(&fixture()).expect("valid reference case");
    }

    #[test]
    fn rejects_period_unit_mismatch() {
        let mut case = fixture();
        case["reference_observables"][0]["unit"] = json!("mm/s");
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_non_positive_uncertainty() {
        let mut case = fixture();
        case["reference_observables"][0]["uncertainty"]["value"] = json!(0.0);
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_beta_without_equal_density_condition() {
        let mut case = fixture();
        case["derived_inferences"][0]["conditioning"]["dimorphos_bulk_density_kg_m3"] = json!(1800.0);
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_missing_source_record() {
        let mut case = fixture();
        case["reference_observables"][0]["source_id"] = json!("not-in-source-table");
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_beta_without_source_record() {
        let mut case = fixture();
        case["derived_inferences"][0]["source_id"] = json!("missing-beta-source");
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_changed_claim_ceiling() {
        let mut case = fixture();
        case["claim_ceiling"] = json!("operationally_qualified");
        assert!(validate_case(&case).is_err());
    }
}
