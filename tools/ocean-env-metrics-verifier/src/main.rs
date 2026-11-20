use serde_json::Value;
use std::collections::HashSet;
use std::{env, fs, path::Path};

const DEFAULT_PATH: &str =
    "docs/research/port-energy-compute/OCEAN_DC_ENVIRONMENTAL_METROLOGY_V0_1.json";
const REQUIRED_ITU: [&str; 14] = [
    "ITU-PUE", "ITU-ERF", "ITU-SUE", "ITU-WUE", "ITU-P-RECLAIMED",
    "ITU-WEEE-RR", "ITU-REF", "ITU-CUE", "ITU-GHGEF", "ITU-AIR-POLLUTANTS",
    "ITU-WATER-POLLUTANTS", "ITU-LAND-POLLUTANTS", "ITU-OUTDOOR-NOISE", "ITU-MSA",
];
const REQUIRED_GATES: [&str; 6] = [
    "ENV-GATE-01", "ENV-GATE-02", "ENV-GATE-03",
    "ENV-GATE-04", "ENV-GATE-05", "ENV-GATE-06",
];
const REQUIRED_SENSORS: [&str; 7] = [
    "facility_energy_meters", "thermal_hydraulic_loop", "ocean_profile_and_hydrodynamics",
    "water_quality", "ecology_and_habitat", "underwater_acoustics", "meteorology_and_sea_state",
];

fn check_keys(value: &Value, allowed: &[&str], context: &str) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| format!("{context} must be an object"))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("{context} contains unknown field {key}"));
        }
    }
    Ok(())
}
fn required_string<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{context}.{key} must be a nonempty string"))
}
fn required_array<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Vec<Value>, String> {
    value.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{context}.{key} must be an array"))
}
fn unique_ids(items: &[Value], context: &str) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for item in items {
        let key = match context {
            "sensor_package" => "sensor_class",
            "non_compensatory_gates" => "gate_id",
            _ => "metric_id",
        };
        let id = required_string(item, key, context)?.to_string();
        if !seen.insert(id.clone()) {
            return Err(format!("{context} contains duplicate identifier {id}"));
        }
        ids.push(id);
    }
    Ok(ids)
}
fn validate_metric(metric: &Value, index: usize, official: bool) -> Result<(), String> {
    let ctx = format!("official_metrics[{index}]");
    for key in ["metric_id", "name", "dimension", "unit", "direction", "definition", "boundary", "standard_reference", "comparison_rule"] {
        required_string(metric, key, &ctx)?;
    }
    let stages = required_array(metric, "lifecycle_stages", &ctx)?;
    if stages.is_empty() || stages.iter().any(|v| v.as_str().unwrap_or("").trim().is_empty()) {
        return Err(format!("{ctx}.lifecycle_stages must contain nonempty strings"));
    }
    if official && !required_string(metric, "metric_id", &ctx)?.starts_with("ITU-") {
        return Err(format!("{ctx} is not labelled as an ITU metric"));
    }
    Ok(())
}
fn validate_extension(metric: &Value, index: usize) -> Result<(), String> {
    let ctx = format!("marine_extension_metrics[{index}]");
    for key in ["metric_id", "name", "unit", "direction", "purpose"] {
        required_string(metric, key, &ctx)?;
    }
    if !required_string(metric, "metric_id", &ctx)?.starts_with("MARINE-") {
        return Err(format!("{ctx} must use MARINE- namespace, not an ITU namespace"));
    }
    let context = required_array(metric, "required_context", &ctx)?;
    if context.is_empty() || context.iter().any(|v| v.as_str().unwrap_or("").trim().is_empty()) {
        return Err(format!("{ctx}.required_context must be nonempty"));
    }
    Ok(())
}
fn validate(catalog: &Value) -> Result<String, String> {
    if !catalog.is_object() {
        return Err("catalogue root must be an object".into());
    }
    let allowed_root: HashSet<&str> = [
        "schema_version", "fixture_id", "as_of", "synthetic_catalog_only",
        "claim_ceiling", "primary_standard", "supporting_standards", "official_metrics",
        "marine_extension_metrics", "sensor_package", "site_evidence_record_contract",
        "spatial_sampling_design", "non_compensatory_gates", "aggregation_policy",
    ].into_iter().collect();
    for key in catalog.as_object().unwrap().keys() {
        if !allowed_root.contains(key.as_str()) {
            return Err(format!("unknown top-level field {key}"));
        }
    }
    for key in ["schema_version", "fixture_id", "as_of", "claim_ceiling"] {
        required_string(catalog, key, "catalogue")?;
    }
    if catalog.get("synthetic_catalog_only").and_then(Value::as_bool) != Some(true) {
        return Err("synthetic_catalog_only must remain true".into());
    }

    let standard = catalog.get("primary_standard").ok_or("primary_standard missing")?;
    check_keys(standard, &["id", "title", "approved", "status", "url", "metric_count"], "primary_standard")?;
    if required_string(standard, "id", "primary_standard")? != "ITU-T L.1312 (07/2026)"
        || required_string(standard, "status", "primary_standard")? != "in_force"
        || required_string(standard, "url", "primary_standard")?.is_empty()
        || standard.get("metric_count").and_then(Value::as_u64) != Some(14) {
        return Err("primary standard must be ITU-T L.1312 (07/2026), in force, with 14 metrics".into());
    }

    let supporting = required_array(catalog, "supporting_standards", "catalogue")?;
    for (i, standard) in supporting.iter().enumerate() {
        let ctx = format!("supporting_standards[{i}]");
        check_keys(standard, &["id", "topic", "url"], &ctx)?;
        for key in ["id", "topic", "url"] { required_string(standard, key, &ctx)?; }
    }

    let official = required_array(catalog, "official_metrics", "catalogue")?;
    if official.len() != REQUIRED_ITU.len() {
        return Err(format!("expected exactly 14 official ITU metrics; got {}", official.len()));
    }
    let official_ids = unique_ids(official, "official_metrics")?;
    for (i, metric) in official.iter().enumerate() {
        check_keys(metric, &["metric_id", "name", "dimension", "unit", "direction", "definition", "lifecycle_stages", "boundary", "standard_reference", "comparison_rule"], &format!("official_metrics[{i}]"))?;
        validate_metric(metric, i, true)?;
    }
    let actual: HashSet<&str> = official_ids.iter().map(String::as_str).collect();
    for id in REQUIRED_ITU {
        if !actual.contains(id) {
            return Err(format!("required ITU metric {id} missing"));
        }
    }

    let wue = official.iter().find(|m| m["metric_id"] == "ITU-WUE").unwrap();
    let wue_rule = required_string(wue, "comparison_rule", "ITU-WUE")?;
    if !wue_rule.contains("category") || !wue_rule.contains("same categories") {
        return Err("WUE comparison must require declared and comparable ISO/IEC 30134-9 categories".into());
    }
    let reclaimed = official.iter().find(|m| m["metric_id"] == "ITU-P-RECLAIMED").unwrap();
    let reclaimed_rule = required_string(reclaimed, "comparison_rule", "ITU-P-RECLAIMED")?;
    if !reclaimed_rule.contains("excludes seawater")
        || !reclaimed_rule.contains("report seawater intake and discharge separately") {
        return Err("reclaimed-water metric must preserve seawater exclusion and separate seawater reporting".into());
    }

    let marine = required_array(catalog, "marine_extension_metrics", "catalogue")?;
    if marine.len() != 14 {
        return Err(format!("expected exactly 14 ocean-specific marine extension metrics; got {}", marine.len()));
    }
    let marine_ids = unique_ids(marine, "marine_extension_metrics")?;
    for (i, metric) in marine.iter().enumerate() {
        check_keys(metric, &["metric_id", "name", "unit", "direction", "purpose", "required_context"], &format!("marine_extension_metrics[{i}]"))?;
        validate_extension(metric, i)?;
    }
    if marine_ids.iter().any(|id| actual.contains(id.as_str())) {
        return Err("marine extensions must not be mislabelled as ITU metrics".into());
    }

    let sensors = required_array(catalog, "sensor_package", "catalogue")?;
    let sensor_ids = unique_ids(sensors, "sensor_package")?;
    for required in REQUIRED_SENSORS {
        if !sensor_ids.iter().any(|id| id == required) {
            return Err(format!("required sensor class {required} missing"));
        }
    }
    for (i, sensor) in sensors.iter().enumerate() {
        let ctx = format!("sensor_package[{i}]");
        check_keys(sensor, &["sensor_class", "measurements", "placement", "cadence", "minimum_evidence"], &ctx)?;
        required_string(sensor, "placement", &ctx)?;
        required_string(sensor, "cadence", &ctx)?;
        let measurements = required_array(sensor, "measurements", &ctx)?;
        let evidence = required_array(sensor, "minimum_evidence", &ctx)?;
        if measurements.is_empty() || evidence.is_empty() {
            return Err(format!("{ctx} requires measurements and evidence fields"));
        }
    }

    let record = catalog.get("site_evidence_record_contract").ok_or("site_evidence_record_contract missing")?;
    check_keys(record, &["required_fields", "quality_states", "evidence_states", "rule", "time_rule", "uncertainty_rule"], "site_evidence_record_contract")?;
    let record_fields = required_array(record, "required_fields", "site_evidence_record_contract")?;
    let required_record: HashSet<&str> = [
        "record_id", "metric_id", "value", "unit", "interval_start_utc", "interval_end_utc",
        "location_coordinates_and_datum", "depth_or_elevation", "sensor_or_method_id",
        "calibration_reference", "uncertainty", "quality_state", "source_rights",
        "raw_data_digest", "processing_version", "custody_receipt",
    ].into_iter().collect();
    let got_record: HashSet<&str> = record_fields.iter().filter_map(Value::as_str).collect();
    if !required_record.is_subset(&got_record) {
        return Err("site evidence record is missing provenance/calibration/time/uncertainty/rights fields".into());
    }
    let qstates: HashSet<&str> = required_array(record, "quality_states", "site_evidence_record_contract")?
        .iter().filter_map(Value::as_str).collect();
    for q in ["valid", "provisional", "unknown", "invalid", "quarantined"] {
        if !qstates.contains(q) { return Err(format!("quality state {q} missing")); }
    }
    let estates: HashSet<&str> = required_array(record, "evidence_states", "site_evidence_record_contract")?
        .iter().filter_map(Value::as_str).collect();
    for e in ["measured", "modelled", "synthetic", "unavailable"] {
        if !estates.contains(e) { return Err(format!("evidence state {e} missing")); }
    }
    if !required_string(record, "rule", "site_evidence_record_contract")?.contains("cannot be presented as measured zero") {
        return Err("unknown/synthetic evidence must not be represented as measured zero".into());
    }

    let spatial = catalog.get("spatial_sampling_design").ok_or("spatial_sampling_design missing")?;
    check_keys(spatial, &["required_station_roles", "rule", "lifecycle_phases", "seasonal_rule"], "spatial_sampling_design")?;
    let stations: HashSet<&str> = required_array(spatial, "required_station_roles", "spatial_sampling_design")?
        .iter().filter_map(Value::as_str).collect();
    for station in ["intake", "outfall", "near_field_plume", "far_field_impact", "independent_reference_control"] {
        if !stations.contains(station) { return Err(format!("required station role {station} missing")); }
    }
    let phases: HashSet<&str> = required_array(spatial, "lifecycle_phases", "spatial_sampling_design")?
        .iter().filter_map(Value::as_str).collect();
    for phase in ["pre_construction_baseline", "construction", "commissioning", "normal_operation", "maintenance_and_cleaning", "incident_or_abnormal_discharge", "decommission", "post_decommission_recovery"] {
        if !phases.contains(phase) { return Err(format!("lifecycle phase {phase} missing")); }
    }

    let gates = required_array(catalog, "non_compensatory_gates", "catalogue")?;
    if gates.len() != REQUIRED_GATES.len() {
        return Err("exactly six environmental hard gates are required".into());
    }
    let gate_ids = unique_ids(gates, "non_compensatory_gates")?;
    for id in REQUIRED_GATES {
        if !gate_ids.iter().any(|x| x == id) {
            return Err(format!("environmental gate {id} missing"));
        }
    }
    let expected_gate_dispositions = [
        ("ENV-GATE-01", "reject_industrial_site_candidate"),
        ("ENV-GATE-02", "site_clearance_unknown_and_blocked"),
        ("ENV-GATE-03", "block_discharge_and_site_clearance"),
        ("ENV-GATE-04", "quarantine_result_and_investigate"),
        ("ENV-GATE-05", "reduce_or_stop_affected_operation_per_approved_response_plan"),
        ("ENV-GATE-06", "evidence_unknown_no_clearance"),
    ];
    for (id, expected) in expected_gate_dispositions {
        let gate = gates.iter().find(|g| g["gate_id"] == id).unwrap();
        if required_string(gate, "disposition", id)? != expected {
            return Err(format!("environmental gate {id} disposition was weakened or changed"));
        }
    }
    for (i, gate) in gates.iter().enumerate() {
        let ctx = format!("non_compensatory_gates[{i}]");
        check_keys(gate, &["gate_id", "condition", "disposition"], &ctx)?;
        required_string(gate, "condition", &ctx)?;
        let disposition = required_string(gate, "disposition", &ctx)?;
        let normalized = disposition.to_ascii_lowercase();
        if normalized.starts_with("allow") || normalized.starts_with("approve") || normalized == "pass" {
            return Err(format!("{ctx} weakens a non-compensatory gate"));
        }
    }
    let aggregate = catalog.get("aggregation_policy").ok_or("aggregation_policy missing")?;
    check_keys(aggregate, &["optional_itu_comprehensive_metric", "project_rule", "environmental_claim_ceiling"], "aggregation_policy")?;
    if !required_string(aggregate, "optional_itu_comprehensive_metric", "aggregation_policy")?.contains("supplementary")
        || !required_string(aggregate, "project_rule", "aggregation_policy")?.contains("Never let a composite score offset") {
        return Err("composite environmental metric must remain supplementary to hard gates".into());
    }

    Ok(format!(
        "CATALOG_VALID_NOT_SITE_EVIDENCE: {} ITU metrics, {} marine extensions, {} sensor classes, {} hard gates; no site measurements or clearance assessed",
        official.len(), marine.len(), sensors.len(), gates.len()
    ))
}
fn run(path: &Path) -> Result<String, String> {
    let input = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let value: Value = serde_json::from_str(&input).map_err(|e| format!("invalid JSON: {e}"))?;
    validate(&value)
}
fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| DEFAULT_PATH.into());
    match run(Path::new(&path)) {
        Ok(message) => println!("{message}"),
        Err(error) => { eprintln!("OCEAN_ENVIRONMENT_CATALOG_REJECTED: {error}"); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../../docs/research/port-energy-compute/OCEAN_DC_ENVIRONMENTAL_METROLOGY_V0_1.json"))
            .expect("environmental metric catalogue parses")
    }
    #[test]
    fn complete_catalogue_is_accepted_as_catalogue_only() {
        let message = validate(&fixture()).expect("complete catalogue accepted");
        assert!(message.starts_with("CATALOG_VALID_NOT_SITE_EVIDENCE"));
    }
    #[test]
    fn missing_official_metric_is_rejected() {
        let mut v = fixture();
        v["official_metrics"].as_array_mut().unwrap().retain(|m| m["metric_id"] != "ITU-MSA");
        assert!(validate(&v).unwrap_err().contains("expected exactly 14"));
    }
    #[test]
    fn seawater_cannot_be_hidden_in_reclaimed_water_metric() {
        let mut v = fixture();
        let metric = v["official_metrics"].as_array_mut().unwrap()
            .iter_mut().find(|m| m["metric_id"] == "ITU-P-RECLAIMED").unwrap();
        metric["comparison_rule"] = Value::String("No separate seawater account required".into());
        assert!(validate(&v).unwrap_err().contains("seawater exclusion"));
    }
    #[test]
    fn weakening_any_environmental_gate_is_rejected() {
        let mut v = fixture();
        v["non_compensatory_gates"][0]["disposition"] = Value::String("allow_if_pue_is_good".into());
        assert!(validate(&v).unwrap_err().contains("disposition was weakened or changed"));
    }
    #[test]
    fn missing_calibration_and_custody_fields_are_rejected() {
        let mut v = fixture();
        v["site_evidence_record_contract"]["required_fields"].as_array_mut().unwrap()
            .retain(|x| x != "calibration_reference" && x != "custody_receipt");
        assert!(validate(&v).unwrap_err().contains("missing provenance/calibration"));
    }
    #[test]
    fn unknown_nested_field_is_rejected() {
        let mut v = fixture();
        v["official_metrics"][0]["unreviewed_shortcut"] = Value::Bool(true);
        assert!(validate(&v).unwrap_err().contains("unknown field unreviewed_shortcut"));
    }
    #[test]
    fn missing_marine_extension_is_rejected() {
        let mut v = fixture();
        v["marine_extension_metrics"].as_array_mut().unwrap().pop();
        assert!(validate(&v).unwrap_err().contains("expected exactly 14 ocean-specific"));
    }
}
