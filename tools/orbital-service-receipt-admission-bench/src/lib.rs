use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

const FIXTURE_SCHEMA: &str = "luminous.orbital-service-receipt-admission.corpus.v0.1";
const CAPACITY_RECEIPT_SCHEMA: &str = "luminous.orbital-service-reconfiguration.receipt.v0.1";
const COMPOSITION_RECEIPT_SCHEMA: &str = "luminous.orbital-service-composition.cli-receipt.v0.1";
const COMPOSITION_BUNDLE_SCHEMA: &str = "luminous.orbital-service-composition.bundle-receipt.v0.1";
const CLAIM_CEILING: &str = "synthetic-receiver-accepted-service-receipt-admission-only";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionStatus {
    ServiceFloorMetSynthetic,
    PartialServiceSynthetic,
    ServiceShortfall,
    Indeterminate,
    Rejected,
    Disputed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceFixture {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub planned_capacity_source_commit: String,
    pub composition_source_commit: String,
    pub source_commits: SourceCommits,
    pub service_profile: ServiceProfile,
    pub service_window_start_tick: u64,
    pub service_window_end_tick: u64,
    pub finalization_tick: u64,
    pub providers: Vec<ProviderWindow>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCommits {
    pub earth_orbit_chain: String,
    pub orbital_capture: String,
    pub orbital_fluid: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProfile {
    pub client_id: String,
    pub interface_id: String,
    pub interface_revision: u64,
    pub fluid_id: String,
    pub minimum_receiver_accepted_mass_g: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderWindow {
    pub provider_id: String,
    pub profile_id: String,
    pub configuration_revision: String,
    pub lifecycle_state_id: String,
    pub effective_from_tick: u64,
    pub effective_until_tick: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub case_id: String,
    pub capacity_plan_case_id: String,
    pub capacity_plan_receipt_json: String,
    pub capacity_plan_receipt_sha256: String,
    pub required_operation_ticks: Vec<u64>,
    pub receipts: Vec<ServiceReceipt>,
    pub expected_status: AdmissionStatus,
    pub expected_reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceReceipt {
    pub event_id: String,
    pub idempotency_key: String,
    pub provider_id: String,
    pub provider_profile_id: String,
    pub provider_configuration_revision: String,
    pub provider_lifecycle_state_id: String,
    pub event_time_tick: u64,
    pub observed_at_tick: u64,
    #[serde(default)]
    pub causal_predecessor_event_id: Option<String>,
    #[serde(default)]
    pub supersedes_event_id: Option<String>,
    pub composition_receipt_json: String,
    pub composition_receipt_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceiptResult {
    pub event_id: String,
    pub event_time_tick: u64,
    pub observed_at_tick: u64,
    pub provider_id: String,
    pub accepted_receiver_mass_g: u64,
    pub counted: bool,
    pub duplicate_replay: bool,
    pub superseded: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResult {
    pub case_id: String,
    pub actual_status: AdmissionStatus,
    pub actual_reason: String,
    pub expected_status: AdmissionStatus,
    pub expected_reason: String,
    pub expected_result_matched: bool,
    pub accepted_receiver_mass_g: u64,
    pub required_receiver_accepted_mass_g: u64,
    pub counted_receipt_count: usize,
    pub duplicate_replays: usize,
    pub superseded_receipts: usize,
    pub missing_operation_ticks: Vec<u64>,
    pub authority_granted: bool,
    pub receipts: Vec<ReceiptResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub scenario_count: usize,
    pub service_floor_met_synthetic: usize,
    pub partial_service_synthetic: usize,
    pub service_shortfall: usize,
    pub indeterminate: usize,
    pub rejected: usize,
    pub disputed: usize,
    pub expected_result_mismatches: usize,
    pub false_service_success_claims: usize,
    pub duplicate_replays: usize,
    pub superseded_receipts: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub scenarios: Vec<ScenarioResult>,
}

#[derive(Debug, Clone, Copy)]
struct Decision {
    status: AdmissionStatus,
    reason: &'static str,
}

impl Decision {
    fn floor_met() -> Self {
        Self { status: AdmissionStatus::ServiceFloorMetSynthetic, reason: "receiver_accepted_service_floor_met" }
    }
    fn partial() -> Self {
        Self { status: AdmissionStatus::PartialServiceSynthetic, reason: "receiver_accepted_service_below_required_floor" }
    }
    fn shortfall() -> Self {
        Self { status: AdmissionStatus::ServiceShortfall, reason: "no_receiver_accepted_service" }
    }
    fn indeterminate(reason: &'static str) -> Self {
        Self { status: AdmissionStatus::Indeterminate, reason }
    }
    fn rejected(reason: &'static str) -> Self {
        Self { status: AdmissionStatus::Rejected, reason }
    }
    fn disputed(reason: &'static str) -> Self {
        Self { status: AdmissionStatus::Disputed, reason }
    }
}

#[derive(Debug, Clone)]
struct PlanTick {
    provider_id: String,
    provider_available: bool,
    qualification_status: String,
}

#[derive(Debug, Clone)]
struct AdmittedReceipt {
    event_time_tick: u64,
    accepted_receiver_mass_g: u64,
    causal_predecessor_event_id: Option<String>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String is infallible");
    }
    output
}

fn present(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn get_str<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

fn get_u64(value: &serde_json::Value, key: &str) -> Option<u64> {
    value.get(key).and_then(serde_json::Value::as_u64)
}

fn get_bool(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key).and_then(serde_json::Value::as_bool)
}

fn validate_source_hashes(
    bundle: &serde_json::Value,
    expected: &SourceCommits,
) -> Result<(), Decision> {
    let Some(source_commits) = bundle.get("source_commits") else {
        return Err(Decision::indeterminate("composition_source_commits_missing"));
    };
    for (key, value) in [
        ("earth_orbit_chain", expected.earth_orbit_chain.as_str()),
        ("orbital_capture", expected.orbital_capture.as_str()),
        ("orbital_fluid", expected.orbital_fluid.as_str()),
    ] {
        if get_str(source_commits, key) != Some(value) {
            return Err(Decision::rejected("composition_upstream_source_pin_mismatch"));
        }
    }

    let Some(artifacts) = bundle.get("artifact_sha256") else {
        return Err(Decision::indeterminate("composition_artifact_digests_missing"));
    };
    for key in ["composition_fixture", "delivery_receipt", "capture_receipt", "fluid_receipt"] {
        let Some(digest) = get_str(artifacts, key) else {
            return Err(Decision::indeterminate("composition_artifact_digest_missing"));
        };
        if !valid_hex(digest, 64) {
            return Err(Decision::rejected("composition_artifact_digest_malformed"));
        }
    }
    Ok(())
}

fn validate_plan_receipt(
    fixture: &ServiceFixture,
    scenario: &Scenario,
) -> Result<BTreeMap<u64, PlanTick>, Decision> {
    if !valid_hex(&scenario.capacity_plan_receipt_sha256, 64) {
        return Err(Decision::rejected("capacity_plan_receipt_digest_malformed"));
    }
    let bytes = scenario.capacity_plan_receipt_json.as_bytes();
    if sha256_hex(bytes) != scenario.capacity_plan_receipt_sha256 {
        return Err(Decision::rejected("capacity_plan_receipt_bytes_digest_mismatch"));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| Decision::rejected("capacity_plan_receipt_json_invalid"))?;
    if get_str(&value, "receipt_schema") != Some(CAPACITY_RECEIPT_SCHEMA) {
        return Err(Decision::rejected("capacity_plan_receipt_schema_mismatch"));
    }
    if get_str(&value, "source_commit") != Some(fixture.planned_capacity_source_commit.as_str()) {
        return Err(Decision::rejected("capacity_plan_source_commit_mismatch"));
    }
    if get_str(&value, "result") != Some("pass") {
        return Err(Decision::indeterminate("capacity_plan_receipt_not_pass"));
    }
    if get_bool(&value, "authority_granted") != Some(false) {
        return Err(Decision::rejected("capacity_plan_authority_flag_invalid"));
    }
    let Some(report) = value.get("report") else {
        return Err(Decision::indeterminate("capacity_plan_report_missing"));
    };
    if get_str(report, "evidence_class") != Some("synthetic") {
        return Err(Decision::indeterminate("capacity_plan_evidence_class_mismatch"));
    }
    if get_bool(report, "authority_granted") != Some(false) {
        return Err(Decision::rejected("capacity_plan_report_authority_invalid"));
    }
    let Some(scenarios) = report.get("scenarios").and_then(serde_json::Value::as_array) else {
        return Err(Decision::indeterminate("capacity_plan_scenarios_missing"));
    };
    let Some(plan) = scenarios.iter().find(|item| {
        get_str(item, "case_id") == Some(scenario.capacity_plan_case_id.as_str())
    }) else {
        return Err(Decision::indeterminate("capacity_plan_scenario_missing"));
    };
    if get_str(plan, "actual_status") != Some("planned_capacity_floor_met_synthetic") {
        return Err(Decision::rejected("capacity_plan_not_feasible"));
    }
    if get_bool(plan, "authority_granted") != Some(false) {
        return Err(Decision::rejected("capacity_plan_scenario_authority_invalid"));
    }
    let Some(ticks) = plan.get("ticks").and_then(serde_json::Value::as_array) else {
        return Err(Decision::indeterminate("capacity_plan_ticks_missing"));
    };
    let mut result = BTreeMap::new();
    for tick in ticks {
        let Some(tick_id) = get_u64(tick, "tick") else {
            return Err(Decision::rejected("capacity_plan_tick_id_invalid"));
        };
        let Some(provider_id) = get_str(tick, "provider_id") else {
            return Err(Decision::indeterminate("capacity_plan_provider_id_missing"));
        };
        let Some(provider_available) = get_bool(tick, "provider_available") else {
            return Err(Decision::indeterminate("capacity_plan_availability_missing"));
        };
        let Some(qualification_status) = get_str(tick, "qualification_status") else {
            return Err(Decision::indeterminate("capacity_plan_qualification_missing"));
        };
        if result.insert(tick_id, PlanTick {
            provider_id: provider_id.to_owned(),
            provider_available,
            qualification_status: qualification_status.to_owned(),
        }).is_some() {
            return Err(Decision::rejected("capacity_plan_duplicate_tick"));
        }
    }
    Ok(result)
}

fn validate_composition_receipt(
    fixture: &ServiceFixture,
    receipt: &ServiceReceipt,
) -> Result<u64, Decision> {
    if !valid_hex(&receipt.composition_receipt_sha256, 64) {
        return Err(Decision::rejected("composition_receipt_digest_malformed"));
    }
    let bytes = receipt.composition_receipt_json.as_bytes();
    let digest = sha256_hex(bytes);
    if digest != receipt.composition_receipt_sha256 {
        return Err(Decision::rejected("composition_receipt_bytes_digest_mismatch"));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| Decision::rejected("composition_receipt_json_invalid"))?;
    if get_str(&value, "receipt_schema") != Some(COMPOSITION_RECEIPT_SCHEMA) {
        return Err(Decision::rejected("composition_receipt_schema_mismatch"));
    }
    if get_str(&value, "source_commit") != Some(fixture.composition_source_commit.as_str()) {
        return Err(Decision::rejected("composition_validator_source_commit_mismatch"));
    }
    if get_str(&value, "result") != Some("pass") {
        return Err(Decision::rejected("composition_receipt_not_pass"));
    }
    let Some(bundle) = value.get("bundle") else {
        return Err(Decision::indeterminate("composition_bundle_missing"));
    };
    if get_str(bundle, "receipt_schema") != Some(COMPOSITION_BUNDLE_SCHEMA) {
        return Err(Decision::rejected("composition_bundle_schema_mismatch"));
    }
    if get_str(bundle, "result") != Some("pass") {
        return Err(Decision::rejected("composition_bundle_not_pass"));
    }
    if get_str(bundle, "evidence_class") != Some("synthetic") {
        return Err(Decision::indeterminate("composition_bundle_evidence_class_mismatch"));
    }
    if get_bool(bundle, "authority_granted") != Some(false) {
        return Err(Decision::rejected("composition_bundle_authority_invalid"));
    }
    validate_source_hashes(bundle, &fixture.source_commits)?;

    let Some(joined) = bundle.get("joined_service") else {
        return Err(Decision::indeterminate("joined_service_missing"));
    };
    for (key, expected, reason) in [
        ("client_id", fixture.service_profile.client_id.as_str(), "composition_client_id_mismatch"),
        ("receiver_id", fixture.service_profile.client_id.as_str(), "composition_receiver_id_mismatch"),
        ("capture_interface_id", fixture.service_profile.interface_id.as_str(), "composition_interface_id_mismatch"),
        ("fluid_id", fixture.service_profile.fluid_id.as_str(), "composition_fluid_id_mismatch"),
        ("phase", "liquid", "composition_fluid_phase_mismatch"),
        ("fluid_class", "storable_liquid", "composition_fluid_class_mismatch"),
    ] {
        match get_str(joined, key) {
            Some(value) if value == expected => {}
            Some(_) => return Err(Decision::rejected(reason)),
            None => return Err(Decision::indeterminate("joined_service_identity_missing")),
        }
    }
    if get_u64(joined, "capture_interface_revision") != Some(fixture.service_profile.interface_revision) {
        return Err(Decision::rejected("composition_interface_revision_mismatch"));
    }
    let Some(event_id) = get_str(joined, "fluid_transfer_event_id") else {
        return Err(Decision::indeterminate("fluid_transfer_event_id_missing"));
    };
    if event_id != receipt.event_id {
        return Err(Decision::rejected("fluid_transfer_event_id_mismatch"));
    }
    let Some(mass) = get_u64(joined, "accepted_receiver_mass_g") else {
        return Err(Decision::indeterminate("receiver_accepted_mass_missing"));
    };
    if mass == 0 {
        return Err(Decision::rejected("receiver_accepted_mass_zero"));
    }
    let Some(uncertainty) = get_u64(joined, "measurement_uncertainty_g") else {
        return Err(Decision::indeterminate("receiver_mass_uncertainty_missing"));
    };
    if uncertainty == 0 || uncertainty > mass {
        return Err(Decision::rejected("receiver_mass_uncertainty_invalid"));
    }
    if get_str(bundle, "claim_ceiling").is_none() {
        return Err(Decision::indeterminate("composition_claim_ceiling_missing"));
    }
    Ok(mass)
}

fn evaluate_scenario(fixture: &ServiceFixture, scenario: &Scenario) -> ScenarioResult {
    let mut result = ScenarioResult {
        case_id: scenario.case_id.clone(),
        actual_status: AdmissionStatus::Indeterminate,
        actual_reason: "not_evaluated".to_owned(),
        expected_status: scenario.expected_status,
        expected_reason: scenario.expected_reason.clone(),
        expected_result_matched: false,
        accepted_receiver_mass_g: 0,
        required_receiver_accepted_mass_g: fixture.service_profile.minimum_receiver_accepted_mass_g,
        counted_receipt_count: 0,
        duplicate_replays: 0,
        superseded_receipts: 0,
        missing_operation_ticks: Vec::new(),
        authority_granted: false,
        receipts: Vec::new(),
    };

    let plan = match validate_plan_receipt(fixture, scenario) {
        Ok(value) => value,
        Err(decision) => {
            result.actual_status = decision.status;
            result.actual_reason = decision.reason.to_owned();
            result.expected_result_matched = result.actual_status == result.expected_status
                && result.actual_reason == result.expected_reason;
            return result;
        }
    };
    if scenario.required_operation_ticks.is_empty() {
        result.actual_status = AdmissionStatus::Indeterminate;
        result.actual_reason = "required_operation_ticks_missing".to_owned();
        result.expected_result_matched = result.actual_status == result.expected_status
            && result.actual_reason == result.expected_reason;
        return result;
    }
    let required_ticks: HashSet<u64> = scenario.required_operation_ticks.iter().copied().collect();
    if required_ticks.len() != scenario.required_operation_ticks.len() {
        result.actual_status = AdmissionStatus::Rejected;
        result.actual_reason = "duplicate_required_operation_tick".to_owned();
        result.expected_result_matched = result.actual_status == result.expected_status
            && result.actual_reason == result.expected_reason;
        return result;
    }

    let providers: HashMap<&str, &ProviderWindow> = fixture.providers.iter()
        .map(|provider| (provider.provider_id.as_str(), provider))
        .collect();
    let mut seen_event: HashMap<String, (String, String)> = HashMap::new();
    let mut seen_key: HashMap<String, (String, String)> = HashMap::new();
    let mut admitted: BTreeMap<String, AdmittedReceipt> = BTreeMap::new();
    let mut superseded_ids = HashSet::new();
    let mut hard_rejection = None;
    let mut disputed = None;
    let mut unknown_reason = None;

    // Input order is observation/ingestion order. Event time is kept separately.
    for receipt in &scenario.receipts {
        if !present(&receipt.event_id) || !present(&receipt.idempotency_key) {
            hard_rejection = Some("event_or_idempotency_identity_missing");
            break;
        }
        if receipt.event_time_tick < fixture.service_window_start_tick
            || receipt.event_time_tick > fixture.service_window_end_tick
        {
            hard_rejection = Some("service_event_outside_window");
            break;
        }
        if receipt.observed_at_tick < receipt.event_time_tick {
            hard_rejection = Some("receipt_observed_before_event");
            break;
        }
        if receipt.observed_at_tick > fixture.finalization_tick {
            unknown_reason = Some("receipt_arrived_after_finalization");
            continue;
        }
        if !required_ticks.contains(&receipt.event_time_tick) {
            hard_rejection = Some("receipt_event_not_in_required_operation_window");
            break;
        }
        let Some(plan_tick) = plan.get(&receipt.event_time_tick) else {
            unknown_reason = Some("capacity_plan_tick_missing");
            continue;
        };
        if !plan_tick.provider_available
            || plan_tick.qualification_status != "robustly_compatible_synthetic"
        {
            hard_rejection = Some("receipt_for_ineligible_capacity_plan_tick");
            break;
        }
        if receipt.provider_id != plan_tick.provider_id {
            hard_rejection = Some("receipt_provider_does_not_match_plan_tick");
            break;
        }
        let Some(provider) = providers.get(receipt.provider_id.as_str()) else {
            hard_rejection = Some("receipt_provider_unknown");
            break;
        };
        if receipt.provider_profile_id != provider.profile_id
            || receipt.provider_configuration_revision != provider.configuration_revision
            || receipt.provider_lifecycle_state_id != provider.lifecycle_state_id
        {
            hard_rejection = Some("receipt_provider_configuration_mismatch");
            break;
        }
        if receipt.event_time_tick < provider.effective_from_tick
            || receipt.event_time_tick > provider.effective_until_tick
        {
            hard_rejection = Some("receipt_outside_provider_effectivity");
            break;
        }

        let mass = match validate_composition_receipt(fixture, receipt) {
            Ok(value) => value,
            Err(decision) => {
                match decision.status {
                    AdmissionStatus::Disputed => disputed = Some(decision.reason),
                    AdmissionStatus::Indeterminate => unknown_reason = Some(decision.reason),
                    _ => hard_rejection = Some(decision.reason),
                }
                if hard_rejection.is_some() || disputed.is_some() {
                    break;
                }
                continue;
            }
        };
        let digest = receipt.composition_receipt_sha256.clone();

        if superseded_ids.contains(&receipt.event_id) {
            if let Some((old_key, old_digest)) = seen_event.get(&receipt.event_id) {
                if old_key == &receipt.idempotency_key && old_digest == &digest {
                    result.duplicate_replays += 1;
                    result.receipts.push(ReceiptResult {
                        event_id: receipt.event_id.clone(),
                        event_time_tick: receipt.event_time_tick,
                        observed_at_tick: receipt.observed_at_tick,
                        provider_id: receipt.provider_id.clone(),
                        accepted_receiver_mass_g: mass,
                        counted: false,
                        duplicate_replay: true,
                        superseded: true,
                    });
                    continue;
                }
            }
            disputed = Some("superseded_event_id_reused_with_conflicting_receipt");
            break;
        }

        let mut supersession_verified = false;
        if let Some(previous_id) = receipt.supersedes_event_id.as_deref() {
            let Some((previous_key, _)) = seen_event.get(previous_id) else {
                unknown_reason = Some("superseded_receipt_predecessor_missing");
                continue;
            };
            if previous_key != &receipt.idempotency_key {
                disputed = Some("supersession_idempotency_key_mismatch");
                break;
            }
            if admitted.remove(previous_id).is_none() {
                unknown_reason = Some("superseded_receipt_predecessor_missing");
                continue;
            }
            superseded_ids.insert(previous_id.to_owned());
            result.superseded_receipts += 1;
            if let Some(previous_row) = result.receipts.iter_mut()
                .find(|row| row.event_id == previous_id)
            {
                previous_row.counted = false;
                previous_row.superseded = true;
            }
            supersession_verified = true;
        }

        if let Some((existing_event, existing_digest)) = seen_key.get(&receipt.idempotency_key) {
            if existing_event == &receipt.event_id && existing_digest == &digest {
                result.duplicate_replays += 1;
                result.receipts.push(ReceiptResult {
                    event_id: receipt.event_id.clone(),
                    event_time_tick: receipt.event_time_tick,
                    observed_at_tick: receipt.observed_at_tick,
                    provider_id: receipt.provider_id.clone(),
                    accepted_receiver_mass_g: mass,
                    counted: false,
                    duplicate_replay: true,
                    superseded: false,
                });
                continue;
            }
            if !(supersession_verified
                && receipt.supersedes_event_id.as_deref() == Some(existing_event.as_str()))
            {
                disputed = Some("idempotency_key_reused_with_conflicting_receipt");
                break;
            }
        }
        if let Some((existing_key, existing_digest)) = seen_event.get(&receipt.event_id) {
            if existing_key != &receipt.idempotency_key || existing_digest != &digest {
                disputed = Some("event_id_reused_with_conflicting_receipt");
                break;
            }
        }
        seen_key.insert(
            receipt.idempotency_key.clone(),
            (receipt.event_id.clone(), digest.clone()),
        );
        seen_event.insert(receipt.event_id.clone(), (receipt.idempotency_key.clone(), digest.clone()));
        let admitted_receipt = AdmittedReceipt {
            event_time_tick: receipt.event_time_tick,
            accepted_receiver_mass_g: mass,
            causal_predecessor_event_id: receipt.causal_predecessor_event_id.clone(),
        };
        admitted.insert(receipt.event_id.clone(), admitted_receipt);
        result.receipts.push(ReceiptResult {
            event_id: receipt.event_id.clone(),
            event_time_tick: receipt.event_time_tick,
            observed_at_tick: receipt.observed_at_tick,
            provider_id: receipt.provider_id.clone(),
            accepted_receiver_mass_g: mass,
            counted: true,
            duplicate_replay: false,
            superseded: false,
        });
    }

    if let Some(reason) = disputed {
        result.actual_status = AdmissionStatus::Disputed;
        result.actual_reason = reason.to_owned();
    } else if let Some(reason) = hard_rejection {
        result.actual_status = AdmissionStatus::Rejected;
        result.actual_reason = reason.to_owned();
    } else {
        // A late receipt is admitted by event time, not arrival time, provided it
        // reached the finalization boundary and is otherwise valid.
        let mut causal_unknown = false;
        for item in admitted.values() {
            if let Some(predecessor) = item.causal_predecessor_event_id.as_deref() {
                if let Some(prior) = admitted.get(predecessor) {
                    if prior.event_time_tick >= item.event_time_tick {
                        result.actual_status = AdmissionStatus::Rejected;
                        result.actual_reason = "causal_predecessor_not_earlier".to_owned();
                        break;
                    }
                } else {
                    causal_unknown = true;
                }
            }
        }
        if result.actual_reason != "causal_predecessor_not_earlier" {
            let covered: HashSet<u64> = admitted.values().map(|item| item.event_time_tick).collect();
            result.missing_operation_ticks = scenario.required_operation_ticks.iter()
                .filter(|tick| !covered.contains(tick))
                .copied()
                .collect();
            for item in admitted.values() {
                result.accepted_receiver_mass_g = result.accepted_receiver_mass_g
                    .checked_add(item.accepted_receiver_mass_g)
                    .unwrap_or(u64::MAX);
            }
            result.counted_receipt_count = admitted.len();
            if causal_unknown || !result.missing_operation_ticks.is_empty() {
                result.actual_status = AdmissionStatus::Indeterminate;
                result.actual_reason = unknown_reason.unwrap_or("required_receipt_or_causal_predecessor_missing").to_owned();
            } else if let Some(reason) = unknown_reason {
                result.actual_status = AdmissionStatus::Indeterminate;
                result.actual_reason = reason.to_owned();
            } else if result.accepted_receiver_mass_g >= fixture.service_profile.minimum_receiver_accepted_mass_g {
                result.actual_status = AdmissionStatus::ServiceFloorMetSynthetic;
                result.actual_reason = "receiver_accepted_service_floor_met".to_owned();
            } else if result.accepted_receiver_mass_g > 0 {
                result.actual_status = AdmissionStatus::PartialServiceSynthetic;
                result.actual_reason = "receiver_accepted_service_below_required_floor".to_owned();
            } else {
                result.actual_status = AdmissionStatus::ServiceShortfall;
                result.actual_reason = "no_receiver_accepted_service".to_owned();
            }
        }
    }
    result.expected_result_matched = result.actual_status == result.expected_status
        && result.actual_reason == result.expected_reason;
    result
}

pub fn evaluate(fixture: &ServiceFixture) -> Result<Report, String> {
    if fixture.schema != FIXTURE_SCHEMA
        || fixture.evidence_class != "synthetic"
        || fixture.claim_ceiling != CLAIM_CEILING
    {
        return Err("unsupported service receipt fixture identity or claim ceiling".to_owned());
    }
    if !valid_hex(&fixture.planned_capacity_source_commit, 40)
        || !valid_hex(&fixture.composition_source_commit, 40)
        || !valid_hex(&fixture.source_commits.earth_orbit_chain, 40)
        || !valid_hex(&fixture.source_commits.orbital_capture, 40)
        || !valid_hex(&fixture.source_commits.orbital_fluid, 40)
    {
        return Err("source commits must be full 40-character hexadecimal values".to_owned());
    }
    if !present(&fixture.service_profile.client_id)
        || !present(&fixture.service_profile.interface_id)
        || !present(&fixture.service_profile.fluid_id)
        || fixture.service_profile.interface_revision == 0
        || fixture.service_profile.minimum_receiver_accepted_mass_g == 0
        || fixture.service_window_start_tick > fixture.service_window_end_tick
        || fixture.finalization_tick < fixture.service_window_end_tick
    {
        return Err("invalid service receipt profile or time window".to_owned());
    }
    if fixture.providers.is_empty() || fixture.scenarios.is_empty() {
        return Err("provider windows and scenarios must be non-empty".to_owned());
    }
    let mut provider_ids = HashSet::new();
    for provider in &fixture.providers {
        if !present(&provider.provider_id)
            || !present(&provider.profile_id)
            || !present(&provider.configuration_revision)
            || !present(&provider.lifecycle_state_id)
            || provider.effective_from_tick > provider.effective_until_tick
            || !provider_ids.insert(provider.provider_id.as_str())
        {
            return Err("provider windows must have unique complete identities and valid effectivity".to_owned());
        }
    }
    let mut scenario_ids = HashSet::new();
    let mut rows = Vec::with_capacity(fixture.scenarios.len());
    let mut summary = Summary {
        scenario_count: fixture.scenarios.len(),
        service_floor_met_synthetic: 0,
        partial_service_synthetic: 0,
        service_shortfall: 0,
        indeterminate: 0,
        rejected: 0,
        disputed: 0,
        expected_result_mismatches: 0,
        false_service_success_claims: 0,
        duplicate_replays: 0,
        superseded_receipts: 0,
        authority_grants: 0,
    };
    for scenario in &fixture.scenarios {
        if !scenario_ids.insert(scenario.case_id.as_str()) {
            return Err(format!("duplicate scenario ID: {}", scenario.case_id));
        }
        let row = evaluate_scenario(fixture, scenario);
        match row.actual_status {
            AdmissionStatus::ServiceFloorMetSynthetic => summary.service_floor_met_synthetic += 1,
            AdmissionStatus::PartialServiceSynthetic => summary.partial_service_synthetic += 1,
            AdmissionStatus::ServiceShortfall => summary.service_shortfall += 1,
            AdmissionStatus::Indeterminate => summary.indeterminate += 1,
            AdmissionStatus::Rejected => summary.rejected += 1,
            AdmissionStatus::Disputed => summary.disputed += 1,
        }
        if row.actual_status == AdmissionStatus::ServiceFloorMetSynthetic
            && scenario.expected_status != AdmissionStatus::ServiceFloorMetSynthetic
        {
            summary.false_service_success_claims += 1;
        }
        if row.actual_status != scenario.expected_status || row.actual_reason != scenario.expected_reason {
            summary.expected_result_mismatches += 1;
        }
        summary.duplicate_replays += row.duplicate_replays;
        summary.superseded_receipts += row.superseded_receipts;
        rows.push(row);
    }
    Ok(Report {
        result: if summary.expected_result_mismatches == 0 { "pass" } else { "fail" }.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        scenarios: rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ServiceFixture {
        serde_json::from_str(include_str!("../fixtures/v0.1/cases.json"))
            .expect("frozen receipt-admission corpus parses")
    }

    #[test]
    fn frozen_receipt_corpus_matches_independent_expected_outcomes() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        assert_eq!(report.summary.scenario_count, 32);
        assert_eq!(report.summary.expected_result_mismatches, 0);
        assert_eq!(report.summary.false_service_success_claims, 0);
        assert_eq!(report.summary.authority_grants, 0);
        assert!(!report.authority_granted);
    }

    #[test]
    fn capacity_never_substitutes_for_receiver_accepted_mass() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "MISSING_RECEIVER_RECEIPT")
            .expect("missing receipt case exists");
        assert_eq!(row.actual_status, AdmissionStatus::Indeterminate);
        assert_eq!(row.accepted_receiver_mass_g, 0);
    }

    #[test]
    fn identical_idempotent_replay_does_not_double_count_mass() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "DUPLICATE_IDENTICAL_REPLAY")
            .expect("idempotent replay case exists");
        assert_eq!(row.actual_status, AdmissionStatus::ServiceFloorMetSynthetic);
        assert_eq!(row.duplicate_replays, 1);
        assert_eq!(row.accepted_receiver_mass_g, 1_140_000);
    }

    #[test]
    fn conflicting_idempotency_key_is_disputed() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "CONFLICTING_IDEMPOTENCY_KEY")
            .expect("conflicting key case exists");
        assert_eq!(row.actual_status, AdmissionStatus::Disputed);
    }

    #[test]
    fn late_receipt_uses_event_time_not_arrival_time() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "LATE_RECEIPT_WITHIN_FINALIZATION")
            .expect("late receipt case exists");
        assert_eq!(row.actual_status, AdmissionStatus::ServiceFloorMetSynthetic);
    }

    #[test]
    fn missing_causal_predecessor_remains_indeterminate() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "MISSING_CAUSAL_PREDECESSOR")
            .expect("causal predecessor case exists");
        assert_eq!(row.actual_status, AdmissionStatus::Indeterminate);
    }

    #[test]
    fn superseded_receipt_is_not_double_counted() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "SUPERSEDED_RECEIPT_CORRECTED")
            .expect("supersession case exists");
        assert_eq!(row.actual_status, AdmissionStatus::ServiceFloorMetSynthetic);
        assert_eq!(row.superseded_receipts, 1);
        assert_eq!(row.accepted_receiver_mass_g, 1_140_000);
    }

    #[test]
    fn provider_configuration_is_bound_at_event_time() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "PROVIDER_CONFIGURATION_NOT_EFFECTIVE")
            .expect("configuration-window case exists");
        assert_eq!(row.actual_status, AdmissionStatus::Rejected);
    }

    #[test]
    fn exact_receipt_bytes_are_hashed_before_json_parse() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        let row = report.scenarios.iter()
            .find(|row| row.case_id == "RECEIPT_BYTES_CHANGED_AFTER_HASH")
            .expect("tampered receipt case exists");
        assert_eq!(row.actual_status, AdmissionStatus::Rejected);
        assert_eq!(row.actual_reason, "composition_receipt_bytes_digest_mismatch");
    }

    #[test]
    fn no_receipt_admission_grants_operational_authority() {
        let report = evaluate(&fixture()).expect("receipt fixture evaluates");
        assert!(!report.authority_granted);
        assert!(report.scenarios.iter().all(|row| !row.authority_granted));
    }
}
