use serde_json::Value;
use std::{env, fs, process::ExitCode};

const EXPECTED_FIXTURES: [&str; 5] = [
    "SCENE-LIGHT-01",
    "HUMAN-LOCO-01",
    "CITY-BLOCK-01",
    "PHYSICS-OBS-01",
    "ND-SLICE-01",
];

const REQUIRED_VIEWS: [&str; 4] = [
    "cinematic",
    "engineering_inspection",
    "accessibility",
    "low_power",
];

const REQUIRED_INVARIANTS: [&str; 8] = [
    "simulation_state_digest_unchanged",
    "simulation_event_order_unchanged",
    "replay_receipt_unchanged",
    "unknown_values_never_defaulted_to_zero",
    "synthetic_observations_never_relabelled_as_measurements",
    "unit_and_reference_frame_preserved",
    "time_and_uncertainty_provenance_preserved",
    "critical_engineering_geometry_not_silently_demoted",
];

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing or non-string field: {key}"))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("missing or non-array field: {key}"))
}

fn contains_all(values: &[Value], required: &[&str], field: &str) -> Result<(), String> {
    let strings: Vec<&str> = values.iter().filter_map(Value::as_str).collect();
    for item in required {
        if !strings.contains(&item) {
            return Err(format!("{field} is missing required item: {item}"));
        }
    }
    Ok(())
}

fn positive_number(value: &Value, key: &str) -> Result<f64, String> {
    let number = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("missing or non-numeric field: {key}"))?;
    if !number.is_finite() || number <= 0.0 {
        return Err(format!("{key} must be finite and positive"));
    }
    Ok(number)
}

fn validate_profile(profile: &Value, expected_id: &str) -> Result<(), String> {
    if string(profile, "id")? != expected_id {
        return Err(format!("expected profile {expected_id}"));
    }
    if string(profile, "status")? != "proposed_target_not_measured" {
        return Err(format!("{expected_id} must remain explicitly unmeasured"));
    }

    let resolution = array(profile, "resolution_px")?;
    if resolution.len() != 2
        || resolution[0].as_u64().unwrap_or(0) == 0
        || resolution[1].as_u64().unwrap_or(0) == 0
    {
        return Err(format!("{expected_id} has invalid resolution_px"));
    }

    positive_number(profile, "target_fps")?;
    let p95 = positive_number(profile, "p95_frame_time_ms_max")?;
    if let Some(value) = profile.get("p99_frame_time_ms_max") {
        let p99 = value
            .as_f64()
            .ok_or_else(|| format!("{expected_id} has invalid p99_frame_time_ms_max"))?;
        if !p99.is_finite() || p99 < p95 {
            return Err(format!("{expected_id} p99 target must be >= p95 target"));
        }
    }
    if let Some(value) = profile.get("max_gpu_allocation_mib") {
        let memory = value
            .as_u64()
            .ok_or_else(|| format!("{expected_id} has invalid GPU allocation limit"))?;
        if memory == 0 {
            return Err(format!("{expected_id} GPU allocation limit must be positive"));
        }
    }
    Ok(())
}

fn validate(value: &Value) -> Result<(), String> {
    if string(value, "schema_version")? != "1.0.0"
        || string(value, "benchmark_id")? != "WORLD-STATE-TO-PIXEL-001"
        || string(value, "version")? != "0.1.0"
    {
        return Err("unexpected schema or benchmark identity".to_owned());
    }
    if string(value, "status")? != "proposed_not_measured" {
        return Err("manifest must not imply measured or passing results".to_owned());
    }
    if value.get("owner_issue").and_then(Value::as_u64) != Some(4468)
        || value
            .get("canonical_physics_corpus_issue")
            .and_then(Value::as_u64)
            != Some(460)
    {
        return Err("issue ownership or canonical physics corpus reference changed".to_owned());
    }

    let fixtures = array(value, "fixtures")?;
    if fixtures.len() != EXPECTED_FIXTURES.len() {
        return Err("fixture count changed; review the benchmark contract".to_owned());
    }
    let mut observed_fixtures = Vec::with_capacity(fixtures.len());
    for fixture in fixtures {
        observed_fixtures.push(string(fixture, "id")?);
        let _ = string(fixture, "purpose")?;
    }
    for expected in EXPECTED_FIXTURES {
        if observed_fixtures.iter().filter(|id| **id == expected).count() != 1 {
            return Err(format!("fixture {expected} must occur exactly once"));
        }
    }

    contains_all(array(value, "view_variants")?, &REQUIRED_VIEWS, "view_variants")?;
    contains_all(
        array(value, "required_invariants")?,
        &REQUIRED_INVARIANTS,
        "required_invariants",
    )?;

    let profiles = array(value, "proposed_profiles")?;
    if profiles.len() != 2 {
        return Err("expected exactly two proposed render profiles".to_owned());
    }
    validate_profile(&profiles[0], "balanced-1080p-reference")?;
    validate_profile(&profiles[1], "low-power-720p")?;

    let overlay = value
        .get("proposed_overlay_budget")
        .ok_or_else(|| "missing proposed_overlay_budget".to_owned())?;
    if overlay.get("entity_count").and_then(Value::as_u64) != Some(1000) {
        return Err("overlay budget must identify the 1k-entity workload".to_owned());
    }
    let overhead = positive_number(overlay, "p95_cpu_overhead_ms_max")?;
    let fraction = positive_number(overlay, "fraction_of_frame_budget_max")?;
    if overhead > 1.0 || fraction > 0.05 {
        return Err("overlay budget exceeds the initial proposed limits".to_owned());
    }
    if string(overlay, "status")? != "proposed_target_not_measured" {
        return Err("overlay target must remain explicitly unmeasured".to_owned());
    }

    for key in ["required_visual_metrics", "required_physics_metrics"] {
        if array(value, key)?.is_empty() {
            return Err(format!("{key} cannot be empty"));
        }
    }
    let policy = value
        .get("benchmark_policy")
        .ok_or_else(|| "missing benchmark_policy".to_owned())?;
    for key in [
        "no_single_realism_score",
        "unresolved_solver_disagreement_remains_unresolved",
        "independent_solver_is_not_physical_truth",
        "render_quality_is_not_physics_validation",
        "cross_gpu_byte_identity_not_required_by_default",
    ] {
        if policy.get(key).and_then(Value::as_bool) != Some(true) {
            return Err(format!("benchmark policy must preserve {key}=true"));
        }
    }
    if string(value, "claim_ceiling")?.trim().is_empty() {
        return Err("claim_ceiling must be non-empty".to_owned());
    }
    Ok(())
}

fn main() -> ExitCode {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "benchmarks/world-state-to-pixel/v0.1/manifest.json".to_owned());
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("world-state-to-pixel-validator: cannot read {path}: {error}");
            return ExitCode::from(2);
        }
    };
    let value: Value = match serde_json::from_str(&source) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("world-state-to-pixel-validator: invalid JSON: {error}");
            return ExitCode::from(2);
        }
    };
    match validate(&value) {
        Ok(()) => {
            println!("CONFORMS: {path}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("NONCONFORMING: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../benchmarks/world-state-to-pixel/v0.1/manifest.json"
        ))
        .expect("checked-in benchmark manifest must be valid JSON")
    }

    #[test]
    fn proposed_manifest_conforms() {
        validate(&fixture()).expect("baseline contract should conform");
    }

    #[test]
    fn measured_status_cannot_be_asserted_by_a_manifest_only() {
        let mut value = fixture();
        value["status"] = Value::String("measured_pass".to_owned());
        assert!(validate(&value).is_err());
    }

    #[test]
    fn duplicate_fixture_identity_is_rejected() {
        let mut value = fixture();
        value["fixtures"][1]["id"] = Value::String("SCENE-LIGHT-01".to_owned());
        assert!(validate(&value).is_err());
    }

    #[test]
    fn removal_of_simulation_invariance_is_rejected() {
        let mut value = fixture();
        let invariants = value["required_invariants"]
            .as_array_mut()
            .expect("invariants must be an array");
        invariants.retain(|item| item.as_str() != Some("simulation_state_digest_unchanged"));
        assert!(validate(&value).is_err());
    }

    #[test]
    fn missing_unit_and_frame_preservation_is_rejected() {
        let mut value = fixture();
        let invariants = value["required_invariants"]
            .as_array_mut()
            .expect("invariants must be an array");
        invariants.retain(|item| item.as_str() != Some("unit_and_reference_frame_preserved"));
        assert!(validate(&value).is_err());
    }
}
