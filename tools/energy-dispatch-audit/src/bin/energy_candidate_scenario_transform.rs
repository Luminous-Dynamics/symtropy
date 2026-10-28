//! Apply a named, source-versioned scenario transform to one energy candidate.
//!
//! The binary is deterministic and fail-closed. It writes the transformed candidate
//! and a receipt binding parent bytes, child bytes, the scenario-profile digest,
//! applicability, and every declared before/after mutation.

use std::{env, error::Error, fs, io};

use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}

#[derive(Debug, Serialize)]
struct Mutation {
    json_pointer: String,
    before: Value,
    after: Value,
}

#[derive(Debug, Serialize)]
struct TransformReceipt {
    schema_version: &'static str,
    panel_id: String,
    scenario_id: String,
    candidate_id: String,
    applicability: String,
    parent_candidate_sha256: String,
    transformed_candidate_sha256: String,
    scenario_profile_sha256: String,
    mutations: Vec<Mutation>,
    claim_ceiling: String,
}

fn set_pointer(
    candidate: &mut Value,
    pointer: &str,
    replacement: Value,
    mutations: &mut Vec<Mutation>,
) -> Result<(), Box<dyn Error>> {
    let target = candidate.pointer_mut(pointer).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("scenario mutation path does not exist: {pointer}"),
        )
    })?;
    let before = target.clone();
    *target = replacement.clone();
    mutations.push(Mutation {
        json_pointer: pointer.to_owned(),
        before,
        after: replacement,
    });
    Ok(())
}

fn numeric_claim(
    candidate: &Value,
    component_id: &str,
    parameter_id: &str,
) -> Result<Option<f64>, Box<dyn Error>> {
    let components = candidate["components"].as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "candidate components array missing")
    })?;
    let Some(component) = components
        .iter()
        .find(|item| item["component_id"].as_str() == Some(component_id))
    else {
        return Ok(None);
    };
    let claims = component["parameter_claims"].as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("parameter claims missing for component {component_id}"),
        )
    })?;
    let claim = claims
        .iter()
        .find(|item| item["parameter_id"].as_str() == Some(parameter_id))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("required candidate claim missing: {component_id}.{parameter_id}"),
            )
        })?;
    let value = claim["value"].as_f64().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("candidate claim is not numeric: {component_id}.{parameter_id}"),
        )
    })?;
    if !value.is_finite() || value < 0.0 {
        return Err(invalid(format!(
            "candidate claim must be finite and non-negative: {component_id}.{parameter_id}"
        )));
    }
    Ok(Some(value))
}

fn required_resource_index(
    candidate: &Value,
    interval_index: usize,
    component_id: &str,
) -> Result<Option<usize>, Box<dyn Error>> {
    let resources = candidate["scenarios"][0]["intervals"][interval_index]["resource_available_kwh"]
        .as_array()
        .ok_or_else(|| invalid("candidate interval resource list missing"))?;
    Ok(resources
        .iter()
        .position(|item| item["component_id"].as_str() == Some(component_id)))
}

fn set_scenario_identity(
    candidate: &mut Value,
    scenario_id: &str,
    mutations: &mut Vec<Mutation>,
) -> Result<(), Box<dyn Error>> {
    set_pointer(
        candidate,
        "/scenarios/0/scenario_id",
        json!(format!("synthetic-energy-{scenario_id}")),
        mutations,
    )?;
    set_pointer(
        candidate,
        "/scenarios/0/description",
        json!(format!(
            "Synthetic candidate-level robustness transform '{scenario_id}'. The original candidate and its parameters are preserved except for the explicitly receipted scenario edits."
        )),
        mutations,
    )?;
    Ok(())
}

fn apply_transform(
    mut candidate: Value,
    scenario_id: &str,
    profile: &Value,
) -> Result<(Value, String, Vec<Mutation>), Box<dyn Error>> {
    let mut mutations = Vec::new();
    let intervals = candidate["scenarios"][0]["intervals"]
        .as_array()
        .ok_or_else(|| invalid("candidate scenario interval list missing"))?;
    if intervals.len() != 2
        || intervals[0]["interval_id"].as_str() != Some("pv-surplus")
        || intervals[1]["interval_id"].as_str() != Some("grid-outage")
    {
        return Err(invalid(
            "candidate scenario transform contract requires the frozen two-interval pv-surplus/grid-outage baseline",
        ));
    }
    if candidate["scenarios"].as_array().map(Vec::len) != Some(1) {
        return Err(invalid("candidate-level scenario panel requires exactly one input scenario"));
    }

    if scenario_id == "nominal" {
        return Ok((candidate, "applicable_baseline".to_owned(), mutations));
    }

    set_scenario_identity(&mut candidate, scenario_id, &mut mutations)?;
    let mut applicability = "applicable".to_owned();

    match scenario_id {
        "pv_total_loss" => {
            let mut pv_resources = 0_usize;
            for interval_index in 0..2 {
                if let Some(resource_index) =
                    required_resource_index(&candidate, interval_index, "pv")?
                {
                    let path = format!(
                        "/scenarios/0/intervals/{interval_index}/resource_available_kwh/{resource_index}/available_kwh"
                    );
                    set_pointer(&mut candidate, &path, json!(0), &mut mutations)?;
                    pv_resources += 1;
                }
            }
            if pv_resources == 0 {
                applicability = "not_applicable_no_pv_component".to_owned();
            }
        }
        "outage_early" => {
            let grid_cap = numeric_claim(
                &candidate,
                "grid",
                "interval_import_energy_cap_kwh",
            )?
            .ok_or_else(|| invalid("outage-timing scenario requires an explicit grid component"))?;
            if grid_cap.fract() != 0.0 || grid_cap > u16::MAX as f64 {
                return Err(invalid("grid energy cap cannot be represented in integer dispatch quanta"));
            }
            set_pointer(
                &mut candidate,
                "/scenarios/0/intervals/0/grid_outage",
                json!(true),
                &mut mutations,
            )?;
            set_pointer(
                &mut candidate,
                "/scenarios/0/intervals/1/grid_outage",
                json!(false),
                &mut mutations,
            )?;
            let first_grid_index = required_resource_index(&candidate, 0, "grid")?
                .ok_or_else(|| invalid("grid resource missing in first interval"))?;
            let second_grid_index = required_resource_index(&candidate, 1, "grid")?
                .ok_or_else(|| invalid("grid resource missing in second interval"))?;
            set_pointer(
                &mut candidate,
                &format!("/scenarios/0/intervals/0/resource_available_kwh/{first_grid_index}/available_kwh"),
                json!(0),
                &mut mutations,
            )?;
            set_pointer(
                &mut candidate,
                &format!("/scenarios/0/intervals/1/resource_available_kwh/{second_grid_index}/available_kwh"),
                json!(grid_cap as u64),
                &mut mutations,
            )?;
        }
        "essential_demand_2x" => {
            let essential_ids = candidate["services"].as_array()
                .ok_or_else(|| invalid("candidate service catalog missing"))?
                .iter()
                .filter(|service| service["essential"].as_bool() == Some(true))
                .map(|service| service["service_id"].as_str().map(str::to_owned)
                    .ok_or_else(|| invalid("essential service ID missing")))
                .collect::<Result<Vec<_>, _>>()?;
            if essential_ids.is_empty() {
                return Err(invalid("demand surge scenario requires at least one explicitly essential service"));
            }
            let mut found = 0_usize;
            for interval_index in 0..2 {
                // Clone the immutable list before set_pointer mutably borrows the candidate tree.
                // Service indices are preserved, so the exact JSON-pointer mutations stay stable.
                let services = candidate["scenarios"][0]["intervals"][interval_index]["service_requested"]
                    .as_array()
                    .ok_or_else(|| invalid("candidate interval service_requested list missing"))?
                    .clone();
                for (service_index, service) in services.iter().enumerate() {
                    let service_id = service["service_id"].as_str()
                        .ok_or_else(|| invalid("requested service ID missing"))?;
                    if !essential_ids.iter().any(|id| id.as_str() == service_id) {
                        continue;
                    }
                    let amount_f64 = service["requested_quantity"].as_f64().ok_or_else(|| {
                        invalid("requested essential service quantity must be numeric")
                    })?;
                    if !amount_f64.is_finite()
                        || amount_f64 < 0.0
                        || amount_f64.fract() != 0.0
                        || amount_f64 > (u64::MAX / 2) as f64
                    {
                        return Err(invalid("requested essential service quantity must be an integral non-negative value in range"));
                    }
                    let amount = amount_f64 as u64;
                    let doubled = amount.checked_mul(2).ok_or_else(|| {
                        invalid("doubled service request overflows integer range")
                    })?;
                    let path = format!(
                        "/scenarios/0/intervals/{interval_index}/service_requested/{service_index}/requested_quantity"
                    );
                    set_pointer(&mut candidate, &path, json!(doubled), &mut mutations)?;
                    found += 1;
                }
            }
            if found == 0 {
                return Err(invalid("demand surge scenario found no requested essential service quantity"));
            }
        }
        "terminal_reserve_1" => {
            let Some(capacity) = numeric_claim(&candidate, "battery", "capacity_kwh")? else {
                applicability = "not_applicable_no_battery_component".to_owned();
                return Ok((candidate, applicability, mutations));
            };
            if capacity.fract() != 0.0 || capacity > u16::MAX as f64 {
                return Err(invalid("battery capacity cannot be represented in integer dispatch quanta"));
            }
            let components = candidate["components"].as_array().ok_or_else(|| invalid("component list missing"))?;
            let battery_index = components
                .iter()
                .position(|item| item["component_id"].as_str() == Some("battery"))
                .ok_or_else(|| invalid("battery claim and component identity disagree"))?;
            let claim_index = components[battery_index]["parameter_claims"]
                .as_array()
                .and_then(|claims| claims.iter().position(|claim| claim["parameter_id"].as_str() == Some("terminal_reserve_kwh")))
                .ok_or_else(|| invalid("battery terminal_reserve_kwh claim missing"))?;
            set_pointer(
                &mut candidate,
                &format!("/components/{battery_index}/parameter_claims/{claim_index}/value"),
                json!(1),
                &mut mutations,
            )?;
        }
        _ => {
            return Err(invalid(format!(
                "scenario ID '{}' is not listed in the pinned scenario profile",
                scenario_id
            )));
        }
    }

    let declared = profile["required_scenarios"]
        .as_array()
        .is_some_and(|rows| rows.iter().any(|row| row["scenario_id"].as_str() == Some(scenario_id)));
    if !declared {
        return Err(invalid("scenario ID is not present in the pinned source profile"));
    }
    Ok((candidate, applicability, mutations))
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_candidate_scenario_transform <candidate.json> <scenario-id> <output-candidate.json> <receipt.json>",
        ).into());
    }
    let input_bytes = fs::read(&args[1])?;
    let mut candidate: Value = serde_json::from_slice(&input_bytes)?;
    if candidate["schema_version"].as_str() != Some("energy-system-candidate-v0.1") {
        return Err(invalid("unsupported candidate schema"));
    }
    let candidate_id = candidate["candidate_id"]
        .as_str()
        .ok_or_else(|| invalid("candidate ID missing"))?
        .to_owned();
    let scenario_id = args[2].as_str();
    let profile_bytes = include_bytes!("../../fixtures/energy-candidate-scenario-panel-v0.1.json");
    let profile: Value = serde_json::from_slice(profile_bytes)?;
    if profile["schema_version"].as_str() != Some("energy-candidate-scenario-panel-v0.1") {
        return Err(invalid("embedded candidate scenario profile schema mismatch"));
    }
    if !profile["required_scenarios"]
        .as_array()
        .is_some_and(|rows| rows.iter().any(|row| row["scenario_id"].as_str() == Some(scenario_id)))
    {
        return Err(invalid("scenario ID missing from pinned candidate scenario profile"));
    }

    // A nominal row retains the exact input bytes, avoiding a reserialization-only mutation.
    let (output_bytes, applicability, mutations) = if scenario_id == "nominal" {
        (input_bytes.clone(), "applicable_baseline".to_owned(), Vec::new())
    } else {
        let (transformed, applicability, mutations) = apply_transform(candidate.clone(), scenario_id, &profile)?;
        (serde_json::to_vec_pretty(&transformed)?, applicability, mutations)
    };
    if scenario_id != "nominal" && !profile["required_scenarios"]
        .as_array()
        .is_some_and(|rows| rows.iter().any(|row| row["scenario_id"].as_str() == Some(scenario_id)))
    {
        return Err(invalid("scenario ID missing from profile"));
    }

    let receipt = TransformReceipt {
        schema_version: "energy-candidate-scenario-transform-receipt-v0.1",
        panel_id: profile["panel_id"].as_str().unwrap_or("unknown").to_owned(),
        scenario_id: scenario_id.to_owned(),
        candidate_id,
        applicability,
        parent_candidate_sha256: sha256_hex(&input_bytes),
        transformed_candidate_sha256: sha256_hex(&output_bytes),
        scenario_profile_sha256: sha256_hex(profile_bytes),
        mutations,
        claim_ceiling: profile["claim_ceiling"].as_str().unwrap_or("Synthetic-only candidate transform.").to_owned(),
    };
    fs::write(&args[3], &output_bytes)?;
    fs::write(&args[4], serde_json::to_vec_pretty(&receipt)?)?;
    println!("candidate_sha256={}", receipt.parent_candidate_sha256);
    println!("transformed_candidate_sha256={}", receipt.transformed_candidate_sha256);
    println!("scenario_profile_sha256={}", receipt.scenario_profile_sha256);
    println!("scenario_id={}", receipt.scenario_id);
    println!("applicability={}", receipt.applicability);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("candidate scenario transform failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str =
        include_str!("../../fixtures/energy-candidate-scenario-panel-v0.1.json");
    const CANDIDATE: &str =
        include_str!("../../fixtures/energy-system-candidate.synthetic.json");

    #[test]
    fn pv_loss_is_explicit_and_preserves_non_pv_claims() {
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        let source: Value = serde_json::from_str(CANDIDATE).unwrap();
        let (child, applicability, mutations) =
            apply_transform(source.clone(), "pv_total_loss", &profile).unwrap();
        assert_eq!(applicability, "applicable");
        assert!(!mutations.is_empty());
        assert_eq!(
            child["components"]
                .as_array().unwrap()
                .iter()
                .find(|item| item["component_id"] == "battery").unwrap(),
            source["components"]
                .as_array().unwrap()
                .iter()
                .find(|item| item["component_id"] == "battery").unwrap()
        );
        for interval in child["scenarios"][0]["intervals"].as_array().unwrap() {
            assert!(interval["resource_available_kwh"].as_array().unwrap().iter().all(|resource| {
                resource["component_id"] != "pv" || resource["available_kwh"] == 0
            }));
        }
    }

    #[test]
    fn outage_early_preserves_candidate_grid_cap_and_moves_the_outage() {
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        let source: Value = serde_json::from_str(CANDIDATE).unwrap();
        let cap = numeric_claim(&source, "grid", "interval_import_energy_cap_kwh")
            .unwrap().unwrap();
        let (child, applicability, mutations) =
            apply_transform(source, "outage_early", &profile).unwrap();
        assert_eq!(applicability, "applicable");
        assert!(mutations.len() >= 4);
        assert_eq!(child["scenarios"][0]["intervals"][0]["grid_outage"], true);
        assert_eq!(child["scenarios"][0]["intervals"][1]["grid_outage"], false);
        assert_eq!(resource_amount(&child, 0, "grid"), Some(0.0));
        assert_eq!(resource_amount(&child, 1, "grid"), Some(cap));
    }

    fn resource_amount(candidate: &Value, interval_index: usize, component_id: &str) -> Option<f64> {
        candidate["scenarios"][0]["intervals"][interval_index]["resource_available_kwh"]
            .as_array()?.iter()
            .find(|item| item["component_id"].as_str() == Some(component_id))?
            ["available_kwh"].as_f64()
    }

    #[test]
    fn demand_surge_doubles_requested_service_without_changing_generation_claims() {
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        let source: Value = serde_json::from_str(CANDIDATE).unwrap();
        let source_battery = source["components"].as_array().unwrap().iter()
            .find(|item| item["component_id"] == "battery").unwrap().clone();
        let (child, applicability, _) =
            apply_transform(source, "essential_demand_2x", &profile).unwrap();
        assert_eq!(applicability, "applicable");
        assert_eq!(
            child["components"].as_array().unwrap().iter()
                .find(|item| item["component_id"] == "battery").unwrap(),
            &source_battery
        );
        let old = child["scenarios"][0]["intervals"][1]["service_requested"][0]["requested_quantity"]
            .as_f64().unwrap();
        assert_eq!(old, 4.0);
    }

    #[test]
    fn demand_surge_leaves_nonessential_service_unchanged() {
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        let mut source: Value = serde_json::from_str(CANDIDATE).unwrap();
        source["services"].as_array_mut().unwrap().push(json!({
            "service_id": "flexible-electricity",
            "service_kind": "electricity",
            "component_id": "essential-load",
            "essential": false,
            "unit": "kWh",
            "minimum_delivery_temperature_c": null,
            "description": "Synthetic flexible service for mutation-control testing."
        }));
        source["scenarios"][0]["intervals"][0]["service_requested"].as_array_mut().unwrap().push(json!({
            "service_id": "flexible-electricity", "requested_quantity": 3
        }));
        source["scenarios"][0]["intervals"][1]["service_requested"].as_array_mut().unwrap().push(json!({
            "service_id": "flexible-electricity", "requested_quantity": 4
        }));

        let (child, _, _) = apply_transform(source, "essential_demand_2x", &profile).unwrap();
        for interval in child["scenarios"][0]["intervals"].as_array().unwrap() {
            let flexible = interval["service_requested"].as_array().unwrap()
                .iter().find(|service| service["service_id"] == "flexible-electricity").unwrap();
            let expected = if interval["interval_id"] == "pv-surplus" { 3.0 } else { 4.0 };
            assert_eq!(flexible["requested_quantity"].as_f64(), Some(expected));
        }
    }

    #[test]
    fn battery_reserve_changes_only_the_claim_and_scenario_metadata() {
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        let source: Value = serde_json::from_str(CANDIDATE).unwrap();
        let (child, applicability, mutations) =
            apply_transform(source, "terminal_reserve_1", &profile).unwrap();
        assert_eq!(applicability, "applicable");
        assert_eq!(
            numeric_claim(&child, "battery", "terminal_reserve_kwh").unwrap(),
            Some(1.0)
        );
        let battery_index = child["components"].as_array().unwrap().iter()
            .position(|component| component["component_id"] == "battery").unwrap();
        let claim_index = child["components"][battery_index]["parameter_claims"].as_array().unwrap()
            .iter().position(|claim| claim["parameter_id"] == "terminal_reserve_kwh").unwrap();
        let expected_pointer = format!("/components/{battery_index}/parameter_claims/{claim_index}/value");
        let mut expected_pointers = vec![
            "/scenarios/0/scenario_id".to_owned(),
            "/scenarios/0/description".to_owned(),
            expected_pointer,
        ];
        expected_pointers.sort();
        let mut actual_pointers = mutations.iter()
            .map(|mutation| mutation.json_pointer.clone())
            .collect::<Vec<_>>();
        actual_pointers.sort();
        assert_eq!(actual_pointers, expected_pointers);
    }

    #[test]
    fn no_battery_terminal_reserve_is_explicitly_not_applicable() {
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        let mut source: Value = serde_json::from_str(CANDIDATE).unwrap();
        let without_battery = source["components"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["component_id"] != "battery")
            .cloned()
            .collect::<Vec<_>>();
        source["components"] = Value::Array(without_battery);
        let (child, applicability, mutations) =
            apply_transform(source.clone(), "terminal_reserve_1", &profile).unwrap();
        assert_eq!(applicability, "not_applicable_no_battery_component");
        assert_eq!(mutations.len(), 2, "only scenario identity/description are changed");
        assert_ne!(child, source);
        assert_eq!(
            child["components"]
                .as_array().unwrap()
                .iter()
                .any(|item| item["component_id"] == "battery"),
            false
        );
    }

    #[test]
    fn undeclared_scenario_is_rejected() {
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        let source: Value = serde_json::from_str(CANDIDATE).unwrap();
        assert!(apply_transform(source, "unlisted-scenario", &profile).is_err());
    }
}
