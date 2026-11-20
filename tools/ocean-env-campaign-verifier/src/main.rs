use serde_json::Value;
use std::collections::HashSet;
use std::{env, fs, path::Path};

const DEFAULT_CAMPAIGN: &str =
    "docs/research/port-energy-compute/OCEAN_ENVIRONMENT_SAMPLING_CAMPAIGN_V0_1.json";
const DEFAULT_CATALOG: &str =
    "docs/research/port-energy-compute/OCEAN_DC_ENVIRONMENTAL_METROLOGY_V0_1.json";
const REQUIRED_STATIONS: [&str; 5] = [
    "intake", "outfall", "near_field_plume", "far_field_impact",
    "independent_reference_control",
];
const REQUIRED_STRATA: [&str; 6] = [
    "seasonal", "hydrodynamic", "compute_and_thermal_load", "water_operations",
    "abnormal_events", "lifecycle",
];
const REQUIRED_CHANNELS: [&str; 6] = [
    "continuous_thermal_hydraulic", "continuous_metocean", "discrete_water_chemistry",
    "ecology_and_habitat", "underwater_acoustics", "construction_and_maintenance_events",
];
const REQUIRED_TRIGGERS: [&str; 5] = [
    "TRG-LOAD-STEP", "TRG-OUTFALL-ANOMALY", "TRG-CLEANING", "TRG-METOCEAN", "TRG-ECOLOGY",
];
const REQUIRED_STOPS: [&str; 6] = [
    "STOP-BASELINE", "STOP-CONTROL", "STOP-POWER", "STOP-CUSTODY",
    "STOP-EVENT-GAP", "STOP-ECOLOGY",
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
fn string<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{context}.{key} must be a nonempty string"))
}
fn array<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Vec<Value>, String> {
    value.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{context}.{key} must be an array"))
}
fn id_set(items: &[Value], key: &str, context: &str) -> Result<HashSet<String>, String> {
    let mut output = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let ctx = format!("{context}[{index}]");
        let id = string(item, key, &ctx)?.to_string();
        if !output.insert(id.clone()) {
            return Err(format!("{context} contains duplicate {key} {id}"));
        }
    }
    Ok(output)
}
fn validate(campaign: &Value, catalog: &Value) -> Result<String, String> {
    if !campaign.is_object() {
        return Err("campaign root must be an object".into());
    }
    let root_keys = [
        "schema_version", "campaign_id", "as_of", "design_only", "claim_ceiling",
        "standards_context", "decision_questions", "spatial_design", "temporal_strata",
        "sampling_channels", "event_triggers", "statistical_design",
        "adaptive_sampling_policy", "quality_and_rights", "stop_rules",
        "proposed_benchmarks", "comparison_policy", "owner_native_research",
        "imperfect_detection_model",
    ];
    check_keys(campaign, &root_keys, "campaign")?;
    for key in ["schema_version", "campaign_id", "as_of", "claim_ceiling", "comparison_policy"] {
        string(campaign, key, "campaign")?;
    }
    if campaign.get("design_only").and_then(Value::as_bool) != Some(true) {
        return Err("campaign design_only must remain true".into());
    }
    if !string(campaign, "claim_ceiling", "campaign")?.contains("No site-specific sampling plan") {
        return Err("campaign claim ceiling must disclaim site-specific/permit approval".into());
    }

    let official: HashSet<&str> = array(catalog, "official_metrics", "catalog")?
        .iter().filter_map(|m| m.get("metric_id").and_then(Value::as_str)).collect();
    let marine: HashSet<&str> = array(catalog, "marine_extension_metrics", "catalog")?
        .iter().filter_map(|m| m.get("metric_id").and_then(Value::as_str)).collect();
    let all_metrics: HashSet<&str> = official.union(&marine).copied().collect();

    let questions = array(campaign, "decision_questions", "campaign")?;
    if questions.len() < 4 {
        return Err("at least four decision questions are required".into());
    }
    let qids = id_set(questions, "id", "decision_questions")?;
    if qids.len() != questions.len() {
        return Err("decision question IDs are not unique".into());
    }
    let mut question_observables = HashSet::new();
    for (i, question) in questions.iter().enumerate() {
        let ctx = format!("decision_questions[{i}]");
        check_keys(question, &["id", "question", "primary_observables", "authority"], &ctx)?;
        string(question, "question", &ctx)?;
        string(question, "authority", &ctx)?;
        let observables = array(question, "primary_observables", &ctx)?;
        if observables.is_empty() {
            return Err(format!("{ctx} has no primary observables"));
        }
        for metric in observables.iter().filter_map(Value::as_str) {
            if !all_metrics.contains(metric) {
                return Err(format!("{ctx} refers to unregistered metric {metric}"));
            }
            question_observables.insert(metric.to_string());
        }
    }
    if question_observables.len() < 8 {
        return Err("decision questions do not cover enough distinct registered observables".into());
    }

    let spatial = campaign.get("spatial_design").ok_or("spatial_design missing")?;
    check_keys(spatial, &["mandatory_station_roles", "location_rule", "control_rule", "impact_area_rule"], "spatial_design")?;
    let station_list: HashSet<&str> = array(spatial, "mandatory_station_roles", "spatial_design")?
        .iter().filter_map(Value::as_str).collect();
    for station in REQUIRED_STATIONS {
        if !station_list.contains(station) {
            return Err(format!("mandatory station role {station} missing"));
        }
    }
    let control = string(spatial, "control_rule", "spatial_design")?;
    if !control.contains("outside the modeled/observed influence")
        || !control.contains("independently justified") {
        return Err("reference/control design is not demonstrably independent".into());
    }
    if !string(spatial, "location_rule", "spatial_design")?.contains("No generic universal plume radius") {
        return Err("campaign must not invent a universal plume radius".into());
    }

    let strata = array(campaign, "temporal_strata", "campaign")?;
    let strata_ids = id_set(strata, "stratum_id", "temporal_strata")?;
    for required in REQUIRED_STRATA {
        if !strata_ids.contains(required) {
            return Err(format!("required temporal stratum {required} missing"));
        }
    }
    for (i, stratum) in strata.iter().enumerate() {
        let ctx = format!("temporal_strata[{i}]");
        check_keys(stratum, &["stratum_id", "states", "evidence_basis", "coverage"], &ctx)?;
        if array(stratum, "states", &ctx)?.is_empty() {
            return Err(format!("{ctx}.states cannot be empty"));
        }
        string(stratum, "evidence_basis", &ctx)?;
        string(stratum, "coverage", &ctx)?;
    }

    let channels = array(campaign, "sampling_channels", "campaign")?;
    let channel_ids = id_set(channels, "channel", "sampling_channels")?;
    for required in REQUIRED_CHANNELS {
        if !channel_ids.contains(required) {
            return Err(format!("required sampling channel {required} missing"));
        }
    }
    for (i, channel) in channels.iter().enumerate() {
        let ctx = format!("sampling_channels[{i}]");
        check_keys(channel, &["channel", "methods", "sampling_rule", "alignment"], &ctx)?;
        if array(channel, "methods", &ctx)?.is_empty() {
            return Err(format!("{ctx}.methods cannot be empty"));
        }
        string(channel, "sampling_rule", &ctx)?;
        string(channel, "alignment", &ctx)?;
    }

    let triggers = array(campaign, "event_triggers", "campaign")?;
    let trigger_ids = id_set(triggers, "trigger_id", "event_triggers")?;
    for required in REQUIRED_TRIGGERS {
        if !trigger_ids.contains(required) {
            return Err(format!("event trigger {required} missing"));
        }
    }
    for (i, trigger) in triggers.iter().enumerate() {
        let ctx = format!("event_triggers[{i}]");
        check_keys(trigger, &["trigger_id", "when", "action"], &ctx)?;
        string(trigger, "when", &ctx)?;
        string(trigger, "action", &ctx)?;
    }

    let stats = campaign.get("statistical_design").ok_or("statistical_design missing")?;
    check_keys(stats, &[
        "primary_contrast_required", "example_contrasts", "minimum_detectable_effect",
        "target_statistical_power", "type_i_error_rate", "power_analysis",
        "independent_unit_rule", "multiplicity_rule",
    ], "statistical_design")?;
    if !string(stats, "minimum_detectable_effect", "statistical_design")?
        .contains("before collecting confirmatory data") {
        return Err("minimum detectable effect must be pre-registered before confirmatory data".into());
    }
    if stats.get("target_statistical_power").and_then(Value::as_f64) != Some(0.8)
        || stats.get("type_i_error_rate").and_then(Value::as_f64) != Some(0.05) {
        return Err("campaign statistical design must pin its proposed 0.80 power / 0.05 type-I error targets".into());
    }
    for key in ["primary_contrast_required", "power_analysis", "independent_unit_rule", "multiplicity_rule"] {
        string(stats, key, "statistical_design")?;
    }
    if !string(stats, "independent_unit_rule", "statistical_design")?.contains("pseudoreplication") {
        return Err("independent-unit plan must address pseudoreplication".into());
    }

    let adaptive = campaign.get("adaptive_sampling_policy").ok_or("adaptive_sampling_policy missing")?;
    check_keys(adaptive, &["optimization_goal", "decision_inputs", "must_preserve", "rule", "linked_work"], "adaptive_sampling_policy")?;
    for required in ["expected_information_gain", "parameter_identifiability", "decision_sensitivity", "environmental_disturbance_cost", "data_rights"] {
        if !array(adaptive, "decision_inputs", "adaptive_sampling_policy")?
            .iter().any(|v| v.as_str() == Some(required)) {
            return Err(format!("adaptive sampling omits decision input {required}"));
        }
    }
    for required in ["pre_registered_primary_contrasts", "independent_reference_control", "seasonal_coverage", "non_compensatory_environmental_gates"] {
        if !array(adaptive, "must_preserve", "adaptive_sampling_policy")?
            .iter().any(|v| v.as_str() == Some(required)) {
            return Err(format!("adaptive sampling can weaken protected condition {required}"));
        }
    }
    if !string(adaptive, "rule", "adaptive_sampling_policy")?.contains("may not quietly remove") {
        return Err("adaptive sampling lacks anti-cherry-picking rule".into());
    }

    let detection = campaign.get("imperfect_detection_model").ok_or("imperfect_detection_model missing")?;
    check_keys(detection, &["rule", "required_components", "acceptance", "integration"], "imperfect_detection_model")?;
    if !string(detection, "rule", "imperfect_detection_model")?.contains("A non-detection is not absence") {
        return Err("imperfect-detection rule must not equate non-detection with absence".into());
    }
    let components: HashSet<&str> = array(detection, "required_components", "imperfect_detection_model")?
        .iter().filter_map(Value::as_str).collect();
    for required in ["latent_occupancy_or_abundance_state", "method_specific_detection_probability",
        "sampling_effort_and_duration", "repeat_visit_or_temporal_replication", "uncertainty_interval"] {
        if !components.contains(required) {
            return Err(format!("imperfect detection model missing {required}"));
        }
    }
    string(detection, "acceptance", "imperfect_detection_model")?;
    string(detection, "integration", "imperfect_detection_model")?;

    let quality = campaign.get("quality_and_rights").ok_or("quality_and_rights missing")?;
    check_keys(quality, &["required_record_fields", "qc_controls", "evidence_states", "rule"], "quality_and_rights")?;
    let fields: HashSet<&str> = array(quality, "required_record_fields", "quality_and_rights")?
        .iter().filter_map(Value::as_str).collect();
    for required in ["station_id", "sample_window_start_utc", "sample_window_end_utc",
        "signed_sensor_clock_offset_ms", "calibration_validity_window", "measurement_uncertainty",
        "source_rights", "raw_data_digest"] {
        if !fields.contains(required) {
            return Err(format!("campaign evidence record missing {required}"));
        }
    }
    if !string(quality, "rule", "quality_and_rights")?.contains("never converted to zero") {
        return Err("campaign evidence rule must preserve unknowns rather than zero-fill".into());
    }
    let states: HashSet<&str> = array(quality, "evidence_states", "quality_and_rights")?
        .iter().filter_map(Value::as_str).collect();
    for required in ["measured", "modelled", "synthetic", "unavailable"] {
        if !states.contains(required) {
            return Err(format!("campaign evidence state {required} missing"));
        }
    }

    let stops = array(campaign, "stop_rules", "campaign")?;
    let stop_ids = id_set(stops, "id", "stop_rules")?;
    for required in REQUIRED_STOPS {
        if !stop_ids.contains(required) {
            return Err(format!("campaign stop rule {required} missing"));
        }
    }
    for (i, stop) in stops.iter().enumerate() {
        let ctx = format!("stop_rules[{i}]");
        check_keys(stop, &["id", "condition", "disposition"], &ctx)?;
        string(stop, "condition", &ctx)?;
        string(stop, "disposition", &ctx)?;
    }

    let owners = array(campaign, "owner_native_research", "campaign")?;
    if owners.len() < 3 {
        return Err("campaign must link to the three owner-native survey/VOI workstreams".into());
    }
    for (i, owner) in owners.iter().enumerate() {
        let ctx = format!("owner_native_research[{i}]");
        check_keys(owner, &["issue", "scope"], &ctx)?;
        if !string(owner, "issue", &ctx)?.starts_with("https://github.com/Luminous-Dynamics/") {
            return Err(format!("{ctx}.issue must be a public Luminous-Dynamics issue URL"));
        }
        string(owner, "scope", &ctx)?;
    }

    let benchmarks = campaign.get("proposed_benchmarks").ok_or("proposed_benchmarks missing")?;
    if benchmarks.as_object().map(|m| m.len()).unwrap_or(0) < 7 {
        return Err("at least seven proposed sampling benchmarks are required".into());
    }
    if !string(campaign, "comparison_policy", "campaign")?.contains("No site or architecture wins by having fewer samples") {
        return Err("campaign comparison policy permits sample-volume cherry-picking".into());
    }
    Ok(format!(
        "CAMPAIGN_DESIGN_VALID_NOT_SITE_PLAN: {} decision questions, {} temporal strata, {} sampling channels, {} event triggers, {} stop rules, {} registered metrics",
        questions.len(), strata.len(), channels.len(), triggers.len(), stops.len(), question_observables.len()
    ))
}
fn load(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("invalid JSON {}: {e}", path.display()))
}
fn main() {
    let campaign_path = env::args().nth(1).unwrap_or_else(|| DEFAULT_CAMPAIGN.into());
    let catalog_path = env::args().nth(2).unwrap_or_else(|| DEFAULT_CATALOG.into());
    let result = (|| {
        let campaign = load(Path::new(&campaign_path))?;
        let catalog = load(Path::new(&catalog_path))?;
        validate(&campaign, &catalog)
    })();
    match result {
        Ok(summary) => println!("{summary}"),
        Err(error) => { eprintln!("OCEAN_ENV_CAMPAIGN_REJECTED: {error}"); std::process::exit(1); }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn campaign() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/research/port-energy-compute/OCEAN_ENVIRONMENT_SAMPLING_CAMPAIGN_V0_1.json"
        )).expect("campaign fixture parses")
    }
    fn catalog() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/research/port-energy-compute/OCEAN_DC_ENVIRONMENTAL_METROLOGY_V0_1.json"
        )).expect("environmental catalogue parses")
    }
    #[test]
    fn complete_campaign_contract_is_accepted_as_design_only() {
        let result = validate(&campaign(), &catalog()).expect("campaign design accepted");
        assert!(result.starts_with("CAMPAIGN_DESIGN_VALID_NOT_SITE_PLAN"));
    }
    #[test]
    fn missing_independent_reference_station_is_rejected() {
        let mut c = campaign();
        c["spatial_design"]["mandatory_station_roles"].as_array_mut().unwrap()
            .retain(|x| x != "independent_reference_control");
        assert!(validate(&c, &catalog()).unwrap_err().contains("mandatory station role independent_reference_control"));
    }
    #[test]
    fn minimum_detectable_effect_must_be_preregistered() {
        let mut c = campaign();
        c["statistical_design"]["minimum_detectable_effect"] =
            Value::String("to be selected after seeing the data".into());
        assert!(validate(&c, &catalog()).unwrap_err().contains("pre-registered"));
    }
    #[test]
    fn unregistered_observable_is_rejected() {
        let mut c = campaign();
        c["decision_questions"][0]["primary_observables"][0] =
            Value::String("UNREGISTERED-ECOLOGY-METRIC".into());
        assert!(validate(&c, &catalog()).unwrap_err().contains("unregistered metric"));
    }
    #[test]
    fn zero_detection_cannot_be_interpreted_as_absence() {
        let mut c = campaign();
        c["imperfect_detection_model"]["rule"] =
            Value::String("If no detections occur, conclude absence".into());
        assert!(validate(&c, &catalog()).unwrap_err().contains("must not equate non-detection with absence"));
    }
    #[test]
    fn adaptive_sampling_must_preserve_primary_controls_and_coverage() {
        let mut c = campaign();
        c["adaptive_sampling_policy"]["must_preserve"].as_array_mut().unwrap()
            .retain(|x| x != "independent_reference_control");
        assert!(validate(&c, &catalog()).unwrap_err().contains("can weaken protected condition"));
    }
    #[test]
    fn incomplete_owner_native_alignment_is_rejected() {
        let mut c = campaign();
        c["owner_native_research"].as_array_mut().unwrap().pop();
        assert!(validate(&c, &catalog()).unwrap_err().contains("three owner-native"));
    }
}
