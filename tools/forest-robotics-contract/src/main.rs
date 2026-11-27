use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
type Check<T> = Result<T, String>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    evaluation_time_epoch_seconds: u64,
    mission: MissionRequest,
    robot: RobotProfile,
    safety_profile: SafetyProfile,
    telemetry: TelemetrySnapshot,
    expected_decision: MissionDecision,
    expected_reason_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MissionRequest {
    request_id: String,
    task: MissionTask,
    autonomy_mode: AutonomyMode,
    requested_at_epoch_seconds: u64,
    planned_payload_mass_grams: u64,
    site_permission: SitePermission,
    site_permission_ref: Option<String>,
    operator_approved: bool,
    operator_approval_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MissionTask {
    Survey,
    MapTerrain,
    MeasureTrees,
    InspectLogDeck,
    SoilSample,
    PlantSeedling,
    CarrySmallLoad,
    CutOrFell,
    ChemicalTreatment,
    Unknown,
}

impl MissionTask {
    fn allowed_for_simulation_v0_1(self) -> bool {
        matches!(self, Self::Survey | Self::MapTerrain | Self::MeasureTrees | Self::InspectLogDeck)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AutonomyMode { Teleoperated, Supervised, Autonomous }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SitePermission { Granted, Pending, Denied, Unknown }

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RobotProfile {
    robot_id: String,
    platform_class: String,
    capabilities: Vec<MissionTask>,
    max_slope_permille: u32,
    max_speed_mm_per_second: u32,
    max_localization_error_mm: u32,
    max_payload_mass_grams: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SafetyProfile {
    profile_id: String,
    profile_version: String,
    approved_by_reference: String,
    risk_assessment_reference: String,
    valid_until_epoch_seconds: u64,
    max_telemetry_age_seconds: u64,
    max_slope_permille: u32,
    max_speed_mm_per_second: u32,
    max_localization_error_mm: u32,
    minimum_obstacle_clearance_mm: u32,
    minimum_battery_basis_points: u16,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TelemetrySnapshot {
    captured_at_epoch_seconds: u64,
    localization_health: SensorHealth,
    localization_error_mm: u32,
    perception_health: SensorHealth,
    obstacle_sensor_health: SensorHealth,
    obstacle_status: ObstacleStatus,
    obstacle_clearance_mm: u32,
    mobility_sensor_health: SensorHealth,
    safety_controller_status: SafetyControllerStatus,
    emergency_stop_status: EmergencyStopStatus,
    communications_status: CommunicationsStatus,
    slope_permille: u32,
    commanded_speed_mm_per_second: u32,
    battery_basis_points: u16,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SensorHealth { Healthy, Degraded, Unavailable, Unknown }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ObstacleStatus { Clear, Detected, Unknown }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SafetyControllerStatus { Healthy, Fault, Unknown }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EmergencyStopStatus { Ready, Engaged, Unknown }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CommunicationsStatus { Healthy, Degraded, Lost, Unknown }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MissionDecision {
    EligibleForSimulationOnly,
    BlockedFailClosed,
    RejectedOutOfScope,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ScenarioResult {
    scenario_id: String,
    decision: MissionDecision,
    reason_codes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Receipt {
    result: &'static str,
    contract_version: &'static str,
    suite_id: String,
    suite_version: String,
    claim_ceiling: String,
    scenario_count: usize,
    eligible_for_simulation_only_count: usize,
    blocked_fail_closed_count: usize,
    rejected_out_of_scope_count: usize,
    corpus_sha256: String,
    source_commit: String,
    scenarios: Vec<ScenarioResult>,
}

fn nonempty(value: &str, field: &str, context: &str) -> Check<()> {
    if value.trim().is_empty() { return Err(format!("{context}: {field} must not be empty")); }
    Ok(())
}

fn validate_scenario(s: &Scenario) -> Check<()> {
    let ctx = s.scenario_id.as_str();
    nonempty(ctx, "scenario_id", "scenario")?;
    nonempty(&s.mission.request_id, "mission.request_id", ctx)?;
    nonempty(&s.robot.robot_id, "robot.robot_id", ctx)?;
    nonempty(&s.robot.platform_class, "robot.platform_class", ctx)?;
    nonempty(&s.safety_profile.profile_id, "safety_profile.profile_id", ctx)?;
    nonempty(&s.safety_profile.profile_version, "safety_profile.profile_version", ctx)?;
    nonempty(&s.safety_profile.approved_by_reference, "safety_profile.approved_by_reference", ctx)?;
    nonempty(&s.safety_profile.risk_assessment_reference, "safety_profile.risk_assessment_reference", ctx)?;

    if s.mission.site_permission_ref.as_ref().is_some_and(|v| v.trim().is_empty()) {
        return Err(format!("{ctx}: site_permission_ref must be omitted rather than blank"));
    }
    if s.mission.operator_approval_ref.as_ref().is_some_and(|v| v.trim().is_empty()) {
        return Err(format!("{ctx}: operator_approval_ref must be omitted rather than blank"));
    }
    if s.mission.site_permission == SitePermission::Granted && s.mission.site_permission_ref.is_none() {
        return Err(format!("{ctx}: granted site permission requires a reference"));
    }
    if s.mission.operator_approved && s.mission.operator_approval_ref.is_none() {
        return Err(format!("{ctx}: operator approval requires a reference"));
    }
    if s.robot.capabilities.is_empty() {
        return Err(format!("{ctx}: robot capabilities must not be empty"));
    }
    let mut capabilities = BTreeSet::new();
    for capability in &s.robot.capabilities {
        if !capabilities.insert(*capability as u8) {
            return Err(format!("{ctx}: duplicate robot capability"));
        }
    }
    if s.robot.max_slope_permille == 0 || s.robot.max_speed_mm_per_second == 0
        || s.robot.max_localization_error_mm == 0 {
        return Err(format!("{ctx}: robot operational limits must be positive"));
    }
    if s.safety_profile.valid_until_epoch_seconds == 0 || s.safety_profile.max_telemetry_age_seconds == 0
        || s.safety_profile.max_slope_permille == 0 || s.safety_profile.max_speed_mm_per_second == 0
        || s.safety_profile.max_localization_error_mm == 0
        || s.safety_profile.minimum_obstacle_clearance_mm == 0
        || s.safety_profile.minimum_battery_basis_points > 10_000 {
        return Err(format!("{ctx}: safety profile limits are invalid"));
    }
    if s.telemetry.battery_basis_points > 10_000 {
        return Err(format!("{ctx}: battery_basis_points must be in [0, 10000]"));
    }
    if s.mission.requested_at_epoch_seconds > s.evaluation_time_epoch_seconds {
        return Err(format!("{ctx}: mission request timestamp is in the future"));
    }
    if s.telemetry.captured_at_epoch_seconds == 0 {
        return Err(format!("{ctx}: telemetry timestamp must be positive"));
    }
    Ok(())
}

fn push_if(reasons: &mut Vec<String>, condition: bool, code: &'static str) {
    if condition { reasons.push(code.to_owned()); }
}

fn evaluate(s: &Scenario) -> ScenarioResult {
    let id = s.scenario_id.clone();
    let task = s.mission.task;

    // v0.1 is a software-only simulation gate. Tool-bearing/high-hazard tasks
    // are rejected before considering other favourable fields.
    if !task.allowed_for_simulation_v0_1() {
        return ScenarioResult {
            scenario_id: id,
            decision: MissionDecision::RejectedOutOfScope,
            reason_codes: vec!["TASK_OUT_OF_SCOPE_V0_1".to_owned()],
        };
    }

    let mut reasons = Vec::new();
    push_if(&mut reasons, s.mission.site_permission != SitePermission::Granted, "SITE_PERMISSION_NOT_GRANTED");
    push_if(&mut reasons, !s.mission.operator_approved || s.mission.operator_approval_ref.is_none(), "OPERATOR_APPROVAL_MISSING");
    push_if(&mut reasons, s.mission.autonomy_mode == AutonomyMode::Autonomous, "AUTONOMY_MODE_REQUIRES_SEPARATE_REVIEW");
    push_if(&mut reasons, s.safety_profile.valid_until_epoch_seconds < s.evaluation_time_epoch_seconds, "SAFETY_PROFILE_EXPIRED");
    push_if(&mut reasons, s.telemetry.captured_at_epoch_seconds > s.evaluation_time_epoch_seconds, "TELEMETRY_FROM_FUTURE");
    push_if(&mut reasons,
        s.evaluation_time_epoch_seconds.saturating_sub(s.telemetry.captured_at_epoch_seconds)
            > s.safety_profile.max_telemetry_age_seconds,
        "TELEMETRY_STALE");
    push_if(&mut reasons, !s.robot.capabilities.contains(&task), "ROBOT_LACKS_TASK_CAPABILITY");
    push_if(&mut reasons, s.safety_profile.max_slope_permille > s.robot.max_slope_permille
        || s.safety_profile.max_speed_mm_per_second > s.robot.max_speed_mm_per_second
        || s.safety_profile.max_localization_error_mm > s.robot.max_localization_error_mm,
        "PROFILE_EXCEEDS_ROBOT_CAPABILITY");
    push_if(&mut reasons, s.telemetry.localization_health != SensorHealth::Healthy, "LOCALIZATION_NOT_HEALTHY");
    push_if(&mut reasons,
        s.telemetry.localization_error_mm > s.robot.max_localization_error_mm
            || s.telemetry.localization_error_mm > s.safety_profile.max_localization_error_mm,
        "LOCALIZATION_ERROR_OVER_LIMIT");
    push_if(&mut reasons, s.telemetry.perception_health != SensorHealth::Healthy, "PERCEPTION_NOT_HEALTHY");
    push_if(&mut reasons, s.telemetry.obstacle_sensor_health != SensorHealth::Healthy, "OBSTACLE_SENSOR_NOT_HEALTHY");
    push_if(&mut reasons, s.telemetry.obstacle_status != ObstacleStatus::Clear, "OBSTACLE_NOT_CLEARED");
    push_if(&mut reasons, s.telemetry.obstacle_clearance_mm < s.safety_profile.minimum_obstacle_clearance_mm, "OBSTACLE_CLEARANCE_BELOW_PROFILE");
    push_if(&mut reasons, s.telemetry.mobility_sensor_health != SensorHealth::Healthy, "MOBILITY_SENSOR_NOT_HEALTHY");
    push_if(&mut reasons, s.telemetry.slope_permille > s.robot.max_slope_permille, "SLOPE_OVER_ROBOT_LIMIT");
    push_if(&mut reasons, s.telemetry.slope_permille > s.safety_profile.max_slope_permille, "SLOPE_OVER_PROFILE_LIMIT");
    push_if(&mut reasons,
        s.telemetry.commanded_speed_mm_per_second > s.robot.max_speed_mm_per_second
            || s.telemetry.commanded_speed_mm_per_second > s.safety_profile.max_speed_mm_per_second,
        "SPEED_OVER_LIMIT");
    push_if(&mut reasons, s.mission.planned_payload_mass_grams > s.robot.max_payload_mass_grams, "PAYLOAD_OVER_ROBOT_LIMIT");
    push_if(&mut reasons, s.telemetry.battery_basis_points < s.safety_profile.minimum_battery_basis_points, "BATTERY_BELOW_RESERVE");
    push_if(&mut reasons, s.telemetry.safety_controller_status != SafetyControllerStatus::Healthy, "SAFETY_CONTROLLER_NOT_HEALTHY");
    push_if(&mut reasons, s.telemetry.emergency_stop_status != EmergencyStopStatus::Ready, "EMERGENCY_STOP_NOT_READY");
    push_if(&mut reasons, s.telemetry.communications_status != CommunicationsStatus::Healthy, "COMMUNICATIONS_NOT_HEALTHY");

    if reasons.is_empty() {
        ScenarioResult { scenario_id: id, decision: MissionDecision::EligibleForSimulationOnly, reason_codes: reasons }
    } else {
        ScenarioResult { scenario_id: id, decision: MissionDecision::BlockedFailClosed, reason_codes: reasons }
    }
}

fn validate_corpus(corpus: &Corpus) -> Check<Vec<ScenarioResult>> {
    nonempty(&corpus.suite_id, "suite_id", "corpus")?;
    nonempty(&corpus.suite_version, "suite_version", "corpus")?;
    nonempty(&corpus.claim_ceiling, "claim_ceiling", "corpus")?;
    if corpus.evidence_class != "synthetic" {
        return Err("v0.1 accepts only synthetic corpus data".to_owned());
    }
    if corpus.scenarios.is_empty() || corpus.scenarios.len() > 100 {
        return Err("scenario count must be within 1..=100".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut results = Vec::with_capacity(corpus.scenarios.len());
    for scenario in &corpus.scenarios {
        validate_scenario(scenario)?;
        if !ids.insert(scenario.scenario_id.as_str()) {
            return Err(format!("duplicate scenario_id {}", scenario.scenario_id));
        }
        let result = evaluate(scenario);
        if result.decision != scenario.expected_decision {
            return Err(format!("{}: expected {:?}, evaluator returned {:?}",
                scenario.scenario_id, scenario.expected_decision, result.decision));
        }
        if result.reason_codes != scenario.expected_reason_codes {
            return Err(format!("{}: expected reasons {:?}, evaluator returned {:?}",
                scenario.scenario_id, scenario.expected_reason_codes, result.reason_codes));
        }
        results.push(result);
    }
    Ok(results)
}

fn build_receipt(corpus: &Corpus, corpus_sha256: String, source_commit: String,
    scenarios: Vec<ScenarioResult>) -> Receipt {
    let eligible = scenarios.iter().filter(|s| s.decision == MissionDecision::EligibleForSimulationOnly).count();
    let blocked = scenarios.iter().filter(|s| s.decision == MissionDecision::BlockedFailClosed).count();
    let rejected = scenarios.iter().filter(|s| s.decision == MissionDecision::RejectedOutOfScope).count();
    Receipt {
        result: "validated",
        contract_version: VERSION,
        suite_id: corpus.suite_id.clone(),
        suite_version: corpus.suite_version.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        scenario_count: scenarios.len(),
        eligible_for_simulation_only_count: eligible,
        blocked_fail_closed_count: blocked,
        rejected_out_of_scope_count: rejected,
        corpus_sha256,
        source_commit,
        scenarios,
    }
}

fn execute() -> Check<()> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(|| "usage: forest-robotics-contract CORPUS.json --source-commit <40-hex-sha>".to_owned())?;
    if args.next().as_deref() != Some("--source-commit") { return Err("expected --source-commit".to_owned()); }
    let commit = args.next().ok_or_else(|| "missing source commit SHA".to_owned())?;
    if args.next().is_some() { return Err("unexpected trailing arguments".to_owned()); }
    if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("source commit must be exactly 40 hexadecimal characters".to_owned());
    }
    let bytes = fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let corpus: Corpus = serde_json::from_slice(&bytes).map_err(|e| format!("invalid corpus JSON: {e}"))?;
    let results = validate_corpus(&corpus)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let receipt = build_receipt(&corpus, digest, commit, results);
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => { eprintln!("forest-robotics-contract: {error}"); ExitCode::from(2) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../docs/tech/forest-robotics-corpus-v0.1.json");
    fn corpus() -> Corpus { serde_json::from_str(FIXTURE).expect("frozen synthetic corpus parses") }

    #[test]
    fn frozen_corpus_matches_expected_decisions_and_reason_codes() {
        let c = corpus();
        let results = validate_corpus(&c).expect("frozen cases match contract");
        assert_eq!(results.len(), 11);
        assert_eq!(results.iter().filter(|r| r.decision == MissionDecision::EligibleForSimulationOnly).count(), 1);
        assert_eq!(results.iter().filter(|r| r.decision == MissionDecision::BlockedFailClosed).count(), 9);
        assert_eq!(results.iter().filter(|r| r.decision == MissionDecision::RejectedOutOfScope).count(), 1);
    }

    #[test]
    fn cutting_or_felling_is_rejected_even_when_other_fields_are_favorable() {
        let mut c = corpus();
        let scenario = c.scenarios.iter_mut().find(|s| s.scenario_id == "CUT_OR_FELL_OUT_OF_SCOPE").unwrap();
        scenario.mission.operator_approved = true;
        scenario.mission.operator_approval_ref = Some("approval-001".to_owned());
        assert_eq!(evaluate(scenario).decision, MissionDecision::RejectedOutOfScope);
        assert_eq!(evaluate(scenario).reason_codes, vec!["TASK_OUT_OF_SCOPE_V0_1"]);
    }

    #[test]
    fn autonomous_mode_requires_separate_review_in_v0_1() {
        let mut c = corpus();
        let scenario = c.scenarios.iter_mut()
            .find(|s| s.scenario_id == "AUTONOMOUS_MODE_REQUIRES_REVIEW").unwrap();
        let result = evaluate(scenario);
        assert_eq!(result.decision, MissionDecision::BlockedFailClosed);
        assert!(result.reason_codes.contains(&"AUTONOMY_MODE_REQUIRES_SEPARATE_REVIEW".to_owned()));
    }

    #[test]
    fn stale_telemetry_fails_closed() {
        let mut c = corpus();
        let scenario = c.scenarios.iter_mut().find(|s| s.scenario_id == "SURVEY_STALE_TELEMETRY").unwrap();
        assert!(evaluate(scenario).reason_codes.contains(&"TELEMETRY_STALE".to_owned()));
    }

    #[test]
    fn unknown_obstacle_sensor_status_fails_closed() {
        let mut c = corpus();
        let scenario = c.scenarios.iter_mut().find(|s| s.scenario_id == "OBSTACLE_SENSOR_UNKNOWN").unwrap();
        assert!(evaluate(scenario).reason_codes.contains(&"OBSTACLE_SENSOR_NOT_HEALTHY".to_owned()));
        assert_eq!(evaluate(scenario).decision, MissionDecision::BlockedFailClosed);
    }

    #[test]
    fn expired_safety_profile_fails_closed() {
        let mut c = corpus();
        let scenario = c.scenarios.iter_mut().find(|s| s.scenario_id == "SAFETY_PROFILE_EXPIRED").unwrap();
        assert!(evaluate(scenario).reason_codes.contains(&"SAFETY_PROFILE_EXPIRED".to_owned()));
    }

    #[test]
    fn unknown_site_permission_cannot_be_offset_by_good_sensors() {
        let mut c = corpus();
        let scenario = c.scenarios.iter_mut().find(|s| s.scenario_id == "SITE_PERMISSION_UNKNOWN").unwrap();
        scenario.telemetry.localization_health = SensorHealth::Healthy;
        assert!(evaluate(scenario).reason_codes.contains(&"SITE_PERMISSION_NOT_GRANTED".to_owned()));
    }

    #[test]
    fn slope_and_speed_limits_are_independently_enforced() {
        let mut c = corpus();
        let scenario = c.scenarios.iter_mut().find(|s| s.scenario_id == "SLOPE_AND_SPEED_OVER_LIMIT").unwrap();
        let result = evaluate(scenario);
        assert!(result.reason_codes.contains(&"SLOPE_OVER_ROBOT_LIMIT".to_owned()));
        assert!(result.reason_codes.contains(&"SLOPE_OVER_PROFILE_LIMIT".to_owned()));
        assert!(result.reason_codes.contains(&"SPEED_OVER_LIMIT".to_owned()));
    }

    #[test]
    fn engaged_emergency_stop_blocks_a_mission() {
        let mut c = corpus();
        let scenario = c.scenarios.iter_mut().find(|s| s.scenario_id == "EMERGENCY_STOP_ENGAGED").unwrap();
        assert!(evaluate(scenario).reason_codes.contains(&"EMERGENCY_STOP_NOT_READY".to_owned()));
    }

    #[test]
    fn excessive_localization_error_fails_closed() {
        let mut c = corpus();
        let scenario = c.scenarios.iter_mut().find(|s| s.scenario_id == "LOCALIZATION_ERROR_OVER_LIMIT").unwrap();
        assert!(evaluate(scenario).reason_codes.contains(&"LOCALIZATION_ERROR_OVER_LIMIT".to_owned()));
    }

    #[test]
    fn unknown_json_fields_are_rejected() {
        let mut value: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        value["unexpected_dispatch_command"] = serde_json::Value::String("move_arm".to_owned());
        assert!(serde_json::from_value::<Corpus>(value).unwrap_err().to_string().contains("unexpected_dispatch_command"));
    }

    #[test]
    fn receipt_is_deterministic_and_explicitly_simulation_only() {
        let c = corpus();
        let results = validate_corpus(&c).unwrap();
        let digest = format!("{:x}", Sha256::digest(FIXTURE.as_bytes()));
        let build = || serde_json::to_string(&build_receipt(&c, digest.clone(), "a".repeat(40), validate_corpus(&c).unwrap())).unwrap();
        assert_eq!(build(), build());
        let receipt = build_receipt(&c, digest, "a".repeat(40), results);
        assert_eq!(receipt.result, "validated");
        assert_eq!(receipt.eligible_for_simulation_only_count, 1);
        assert_eq!(receipt.rejected_out_of_scope_count, 1);
    }
}
