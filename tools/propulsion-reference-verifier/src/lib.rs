use serde_json::Value;
use std::collections::BTreeSet;

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing required field: {key}"))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
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

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

pub fn validate_case(case: &Value) -> Result<(), String> {
    if text(case, "schema_version")? != "0.1.0" {
        return Err("unsupported schema_version".into());
    }
    if text(case, "case_id")? != "ideal-rocket-equation-and-nozzle-thrust" {
        return Err("unsupported case_id".into());
    }
    if text(case, "scope")? != "analytical_invariant_check_only" {
        return Err("unsupported scope".into());
    }
    if text(case, "claim_ceiling")? != "analytical_equation_checks_only_not_engine_performance_or_qualification" {
        return Err("claim ceiling must exclude hardware and flight qualification".into());
    }

    let constants = required(case, "constants")?;
    let g0 = number(constants, "standard_gravity_m_s2")?;
    if !close(g0, 9.80665, 1e-12) {
        return Err("standard gravity constant differs from the defined SI convention".into());
    }

    let rocket = required(case, "rocket_equation")?;
    let m0 = number(rocket, "mass_initial_kg")?;
    let mf = number(rocket, "mass_final_kg")?;
    let isp = number(rocket, "specific_impulse_s")?;
    let expected_delta_v = number(rocket, "expected_delta_v_m_s")?;
    let delta_v_tolerance = number(rocket, "tolerance_m_s")?;
    if m0 <= mf || mf <= 0.0 || isp <= 0.0 || delta_v_tolerance <= 0.0 {
        return Err("rocket equation requires m0 > mf > 0, positive Isp and positive tolerance".into());
    }
    let delta_v = isp * g0 * (m0 / mf).ln();
    if !close(delta_v, expected_delta_v, delta_v_tolerance) {
        return Err(format!("ideal rocket equation mismatch: computed {delta_v}, expected {expected_delta_v}"));
    }

    let nozzle = required(case, "nozzle_thrust")?;
    let mdot = number(nozzle, "mass_flow_kg_s")?;
    let ve = number(nozzle, "exit_velocity_m_s")?;
    let pe = number(nozzle, "exit_pressure_pa")?;
    let pa = number(nozzle, "ambient_pressure_pa")?;
    let area = number(nozzle, "exit_area_m2")?;
    let expected_thrust = number(nozzle, "expected_thrust_n")?;
    let expected_isp = number(nozzle, "expected_specific_impulse_s")?;
    let thrust_tolerance = number(nozzle, "tolerance_thrust_n")?;
    let isp_tolerance = number(nozzle, "tolerance_specific_impulse_s")?;
    if mdot <= 0.0 || ve < 0.0 || pe < 0.0 || pa < 0.0 || area < 0.0 ||
       thrust_tolerance <= 0.0 || isp_tolerance <= 0.0 {
        return Err("nozzle inputs must be finite and physically admissible".into());
    }
    let thrust = mdot * ve + area * (pe - pa);
    if thrust <= 0.0 || !close(thrust, expected_thrust, thrust_tolerance) {
        return Err(format!("rocket thrust mismatch: computed {thrust}, expected {expected_thrust}"));
    }
    let computed_isp = thrust / (mdot * g0);
    if !close(computed_isp, expected_isp, isp_tolerance) {
        return Err(format!("specific impulse mismatch: computed {computed_isp}, expected {expected_isp}"));
    }

    let sources = required(case, "sources")?
        .as_array()
        .ok_or_else(|| "sources must be an array".to_string())?;
    let mut source_ids = BTreeSet::new();
    for source in sources {
        let id = text(source, "source_id")?;
        let url = text(source, "url")?;
        if !url.starts_with("https://") {
            return Err(format!("source {id} must use HTTPS"));
        }
        text(source, "title")?;
        if !source_ids.insert(id.to_owned()) {
            return Err(format!("duplicate source_id: {id}"));
        }
    }
    if source_ids.len() < 2 {
        return Err("at least two separately identified source records are required".into());
    }

    let not_established = required(case, "not_established")?
        .as_array().ok_or_else(|| "not_established must be an array".to_string())?;
    if not_established.len() < 3 {
        return Err("at least three explicit non-claims are required".into());
    }
    for item in not_established {
        if item.as_str().map(str::trim).unwrap_or("").len() < 8 {
            return Err("not_established entries must be explicit non-empty strings".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../../docs/space-engineering/reference-cases/ideal-rocket-thrust-delta-v-v0.1.json"))
            .expect("frozen analytic reference fixture must parse")
    }

    #[test]
    fn accepts_frozen_analytic_reference_case() {
        validate_case(&fixture()).expect("valid analytic reference case");
    }

    #[test]
    fn rejects_final_mass_not_below_initial_mass() {
        let mut case = fixture();
        case["rocket_equation"]["mass_final_kg"] = json!(10000.0);
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_zero_mass_flow() {
        let mut case = fixture();
        case["nozzle_thrust"]["mass_flow_kg_s"] = json!(0.0);
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_changed_thrust_oracle() {
        let mut case = fixture();
        case["nozzle_thrust"]["expected_thrust_n"] = json!(290000.0);
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_negative_exit_area() {
        let mut case = fixture();
        case["nozzle_thrust"]["exit_area_m2"] = json!(-0.5);
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_changed_standard_gravity() {
        let mut case = fixture();
        case["constants"]["standard_gravity_m_s2"] = json!(9.8);
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn rejects_changed_claim_ceiling() {
        let mut case = fixture();
        case["claim_ceiling"] = json!("flight_qualified");
        assert!(validate_case(&case).is_err());
    }
}
