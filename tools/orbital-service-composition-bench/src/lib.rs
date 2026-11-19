use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Component {
    EarthOrbitChain,
    OrbitalCapture,
    OrbitalFluid,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    LaunchAttemptSucceeded,
    ApproachObserved,
    PayloadDeliveryAccepted,
    CaptureHandoffAccepted,
    FluidReceiverAccepted,
    CustomerAcknowledged,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    AcceptedForSyntheticComposition,
    Rejected,
    Indeterminate,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub component: Component,
    pub receipt_schema: String,
    pub source_commit: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRef {
    pub component: Component,
    pub receipt_schema: String,
    pub source_commit: String,
    pub reference: String,
    /// SHA-256 claim for the referenced source artifact; bytes are not fetched here.
    pub artifact_sha256: String,
    pub producer_id: String,
    pub authority_scope: String,
    pub context_of_use: String,
    pub claim_ceiling: String,
    pub decision: String,
    pub evidence_class: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub event_id: String,
    pub sequence: u64,
    pub kind: EventKind,
    #[serde(default)]
    pub subject_id: Option<String>,
    #[serde(default)]
    pub target_orbit_id: Option<String>,
    #[serde(default)]
    pub destination_node_id: Option<String>,
    #[serde(default)]
    pub receiver_id: Option<String>,
    #[serde(default)]
    pub interface_id: Option<String>,
    #[serde(default)]
    pub interface_revision: Option<u64>,
    #[serde(default)]
    pub fluid_id: Option<String>,
    #[serde(default)]
    pub accepted_mass_g: Option<u64>,
    #[serde(default)]
    pub evidence: Option<EvidenceRef>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceProfile {
    pub subject_id: String,
    pub target_orbit_id: String,
    pub destination_node_id: String,
    pub interface_id: String,
    pub interface_revision: u64,
    pub fluid_id: String,
    pub minimum_accepted_mass_g: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub expected_status: GateStatus,
    pub profile: ServiceProfile,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub suite_id: String,
    pub suite_version: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub source_pins: Vec<SourcePin>,
    pub cases: Vec<Case>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub expected_status: GateStatus,
    pub actual_status: GateStatus,
    pub reason_codes: Vec<String>,
    pub required_stage_count: u64,
    pub observed_stage_count: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub case_count: u64,
    pub accepted_for_synthetic_composition: u64,
    pub rejected: u64,
    pub indeterminate: u64,
    pub expected_result_mismatches: u64,
    pub operational_authority_grants: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Report {
    pub suite_id: String,
    pub suite_version: String,
    pub evidence_class: String,
    pub result: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
    pub claim_ceiling: String,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_source_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct JoinedServiceV1 {
    pub cargo_lot_id: String,
    pub cargo_mass_grams: u64,
    pub target_orbit_id: String,
    pub destination_node_id: String,
    pub delivery_epoch: u64,
    pub client_id: String,
    pub capture_interface_id: String,
    pub capture_interface_revision: u64,
    pub capture_frame_id: String,
    pub capture_epoch_tick: u64,
    pub fluid_transfer_event_id: String,
    pub receiver_id: String,
    pub fluid_id: String,
    pub fluid_class: String,
    pub phase: String,
    pub accepted_receiver_mass_g: u64,
    pub final_temperature_mk: u64,
    pub final_pressure_pa: u64,
    pub measurement_uncertainty_g: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ReceiptBundleV1 {
    pub receipt_schema: &'static str,
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub source_commits: BTreeMap<String, String>,
    pub artifact_sha256: BTreeMap<String, String>,
    pub authority_granted: bool,
    pub joined_service: JoinedServiceV1,
}

fn required_str<'a>(value: &'a serde_json::Value, key: &str, label: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| format!("{label}: missing or empty {key}"))
}

fn required_u64(value: &serde_json::Value, key: &str, label: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("{label}: missing or invalid {key}"))
}

fn named_record<'a>(
    report: &'a serde_json::Value,
    collection: &str,
    id_field: &str,
    id_value: &str,
    label: &str,
) -> Result<&'a serde_json::Value, String> {
    report
        .get(collection)
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items.iter().find(|item| {
                item.get(id_field).and_then(serde_json::Value::as_str) == Some(id_value)
            })
        })
        .ok_or_else(|| format!("{label}: missing {collection} record {id_value}"))
}

fn parse_receipt<'a>(
    bytes: &'a [u8],
    label: &str,
    expected_schema: &str,
    pin: &SourcePin,
    fixture_class: &str,
) -> Result<serde_json::Value, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("{label}: invalid receipt JSON: {error}"))?;
    if required_str(&value, "receipt_schema", label)? != expected_schema {
        return Err(format!("{label}: receipt schema mismatch"));
    }
    if required_str(&value, "source_commit", label)? != pin.source_commit {
        return Err(format!("{label}: source commit does not match frozen pin"));
    }
    if required_str(&value, "result", label)? != "pass" {
        return Err(format!("{label}: source receipt is not a pass"));
    }
    let report = value
        .get("report")
        .ok_or_else(|| format!("{label}: report missing"))?;
    if required_str(report, "evidence_class", label)? != fixture_class {
        return Err(format!("{label}: evidence class mismatch"));
    }
    required_str(report, "claim_ceiling", label)?;
    let digest_field = if label == "delivery" { "campaign_sha256" } else { "fixture_sha256" };
    if !is_sha256(required_str(&value, digest_field, label)?) {
        return Err(format!("{label}: {digest_field} is not a SHA-256 digest"));
    }
    Ok(value)
}

/// Joins three independently generated source receipts. The exact input bytes are
/// hashed locally; source commits and report identities are compared to frozen pins.
/// No signature verification or physical validation is performed here.
pub fn compose_receipt_bundle(
    composition_fixture_bytes: &[u8],
    delivery_receipt_bytes: &[u8],
    capture_receipt_bytes: &[u8],
    fluid_receipt_bytes: &[u8],
) -> Result<ReceiptBundleV1, String> {
    let fixture: Fixture = serde_json::from_slice(composition_fixture_bytes)
        .map_err(|error| format!("invalid composition fixture: {error}"))?;
    if fixture.suite_id != "orbital-service-composition"
        || fixture.suite_version != "0.1.0"
        || fixture.evidence_class != "synthetic"
        || !non_empty(&fixture.claim_ceiling)
    {
        return Err("unsupported composition fixture identity or claim ceiling".to_owned());
    }
    let pins: BTreeMap<Component, &SourcePin> = fixture
        .source_pins
        .iter()
        .map(|pin| (pin.component, pin))
        .collect();
    if pins.len() != 3 {
        return Err("composition fixture must pin all three source components".to_owned());
    }
    let service_case = fixture
        .cases
        .iter()
        .find(|case| case.case_id == "NOMINAL_SYNTHETIC_COMPOSITION")
        .ok_or_else(|| "nominal service profile missing from composition fixture".to_owned())?;
    if service_case.expected_status != GateStatus::AcceptedForSyntheticComposition {
        return Err("nominal composition profile is not an accepted positive control".to_owned());
    }
    let profile = &service_case.profile;

    let delivery_pin = pins.get(&Component::EarthOrbitChain).expect("validated source pin");
    let capture_pin = pins.get(&Component::OrbitalCapture).expect("validated source pin");
    let fluid_pin = pins.get(&Component::OrbitalFluid).expect("validated source pin");

    let delivery = parse_receipt(
        delivery_receipt_bytes,
        "delivery",
        "luminous.earth-orbit-chain.receipt.v1",
        delivery_pin,
        &fixture.evidence_class,
    )?;
    let capture = parse_receipt(
        capture_receipt_bytes,
        "capture",
        "luminous.orbital-capture-handoff.receipt.v0.1",
        capture_pin,
        &fixture.evidence_class,
    )?;
    let fluid = parse_receipt(
        fluid_receipt_bytes,
        "fluid",
        "luminous.orbital-fluid-transfer.receipt.v0.1",
        fluid_pin,
        &fixture.evidence_class,
    )?;

    let delivery_report = delivery.get("report").expect("delivery report checked above");
    if required_str(delivery_report, "disposition", "delivery")? != "ready" {
        return Err("delivery report is not Ready".to_owned());
    }
    let cargo = named_record(
        delivery_report,
        "cargo_deliveries",
        "cargo_lot_id",
        &profile.subject_id,
        "delivery",
    )?;
    if required_str(cargo, "status", "delivery cargo")? != "accepted" {
        return Err("payload delivery was not accepted".to_owned());
    }
    if required_str(cargo, "target_orbit_id", "delivery cargo")? != profile.target_orbit_id {
        return Err("target orbit does not match service profile".to_owned());
    }
    let destination_node_id = required_str(cargo, "destination_node_id", "delivery cargo")?;
    if required_str(cargo, "terminal_node_id", "delivery cargo")? != destination_node_id {
        return Err("cargo terminal node does not match declared destination".to_owned());
    }
    if required_u64(cargo, "cargo_mass_grams", "delivery cargo")? == 0 {
        return Err("accepted cargo mass must be positive".to_owned());
    }
    let delivery_epoch = required_u64(cargo, "arrival_epoch_seconds", "delivery cargo")?;

    let capture_report = capture.get("report").expect("capture report checked above");
    if required_str(capture_report, "result", "capture")? != "pass" {
        return Err("capture report is not a pass".to_owned());
    }
    let captured = named_record(
        capture_report,
        "cases",
        "case_id",
        "NOMINAL_CAPTURE",
        "capture",
    )?;
    if required_str(captured, "status", "capture case")? != "accepted_for_handoff" {
        return Err("capture handoff was not accepted".to_owned());
    }
    if captured.get("authority_granted").and_then(serde_json::Value::as_bool) != Some(false) {
        return Err("capture case authority flag missing or unexpectedly granted".to_owned());
    }
    let client_id = required_str(captured, "client_id", "capture case")?;
    if client_id != profile.subject_id {
        return Err("capture client identity does not match delivered payload".to_owned());
    }
    let capture_interface_id = required_str(captured, "interface_id", "capture case")?;
    let capture_interface_revision = required_u64(captured, "interface_revision", "capture case")?;
    if capture_interface_id != profile.interface_id
        || capture_interface_revision != profile.interface_revision
    {
        return Err("capture interface identity/revision does not match service profile".to_owned());
    }
    let capture_frame_id = required_str(captured, "frame_id", "capture case")?.to_owned();
    let capture_epoch_tick = required_u64(captured, "epoch_tick", "capture case")?;

    let fluid_report = fluid.get("report").expect("fluid report checked above");
    if required_str(fluid_report, "result", "fluid")? != "pass" {
        return Err("fluid report is not a pass".to_owned());
    }
    let transfer = named_record(
        fluid_report,
        "scenarios",
        "scenario_id",
        "STORABLE_LIQUID_NOMINAL",
        "fluid",
    )?;
    if required_str(transfer, "status", "fluid scenario")? != "transfer_accepted" {
        return Err("fluid receiver acceptance was not accepted".to_owned());
    }
    let receiver_id = required_str(transfer, "receiver_id", "fluid scenario")?;
    if receiver_id != client_id {
        return Err("fluid receiver identity does not match captured client".to_owned());
    }
    let interface_id = required_str(transfer, "interface_id", "fluid scenario")?;
    let interface_revision = required_u64(transfer, "interface_revision", "fluid scenario")?;
    if interface_id != capture_interface_id || interface_revision != capture_interface_revision {
        return Err("fluid interface identity/revision does not match capture handoff".to_owned());
    }
    let fluid_id = required_str(transfer, "fluid_id", "fluid scenario")?;
    if fluid_id != profile.fluid_id {
        return Err("fluid identity does not match service profile".to_owned());
    }
    let fluid_class = required_str(transfer, "fluid_class", "fluid scenario")?.to_owned();
    let phase = required_str(transfer, "phase", "fluid scenario")?.to_owned();
    if phase != "liquid" {
        return Err("nominal storable-fluid profile requires liquid phase".to_owned());
    }
    let accepted_receiver_mass_g = required_u64(transfer, "receiver_accepted_g", "fluid scenario")?;
    if accepted_receiver_mass_g < profile.minimum_accepted_mass_g {
        return Err("receiver-accepted mass is below service requirement".to_owned());
    }

    let source_commits = BTreeMap::from([
        ("earth_orbit_chain".to_owned(), delivery_pin.source_commit.clone()),
        ("orbital_capture".to_owned(), capture_pin.source_commit.clone()),
        ("orbital_fluid".to_owned(), fluid_pin.source_commit.clone()),
    ]);
    let artifact_sha256 = BTreeMap::from([
        ("composition_fixture".to_owned(), sha256_hex(composition_fixture_bytes)),
        ("delivery_receipt".to_owned(), sha256_hex(delivery_receipt_bytes)),
        ("capture_receipt".to_owned(), sha256_hex(capture_receipt_bytes)),
        ("fluid_receipt".to_owned(), sha256_hex(fluid_receipt_bytes)),
    ]);
    Ok(ReceiptBundleV1 {
        receipt_schema: "luminous.orbital-service-composition.bundle-receipt.v0.1",
        result: "pass".to_owned(),
        evidence_class: fixture.evidence_class,
        claim_ceiling: "Synthetic composition of independently generated source software receipts only; no physical launch, capture, seal, fluid transfer, mission service or flight qualification claim.".to_owned(),
        source_commits,
        artifact_sha256,
        authority_granted: false,
        joined_service: JoinedServiceV1 {
            cargo_lot_id: profile.subject_id.clone(),
            cargo_mass_grams: required_u64(cargo, "cargo_mass_grams", "delivery cargo")?,
            target_orbit_id: profile.target_orbit_id.clone(),
            destination_node_id: destination_node_id.to_owned(),
            delivery_epoch,
            client_id: client_id.to_owned(),
            capture_interface_id: capture_interface_id.to_owned(),
            capture_interface_revision,
            capture_frame_id,
            capture_epoch_tick,
            fluid_transfer_event_id: required_str(transfer, "transfer_event_id", "fluid scenario")?.to_owned(),
            receiver_id: receiver_id.to_owned(),
            fluid_id: fluid_id.to_owned(),
            fluid_class,
            phase,
            accepted_receiver_mass_g,
            final_temperature_mk: required_u64(transfer, "final_temperature_mk", "fluid scenario")?,
            final_pressure_pa: required_u64(transfer, "final_pressure_pa", "fluid scenario")?,
            measurement_uncertainty_g: required_u64(transfer, "measurement_uncertainty_g", "fluid scenario")?,
        },
    })
}

fn expected_component(kind: EventKind) -> Option<(Component, &'static str, &'static str)> {
    match kind {
        EventKind::PayloadDeliveryAccepted => Some((
            Component::EarthOrbitChain,
            "luminous.earth-orbit-chain.receipt.v1",
            "ready",
        )),
        EventKind::CaptureHandoffAccepted => Some((
            Component::OrbitalCapture,
            "luminous.orbital-capture-handoff.receipt.v0.1",
            "accepted_for_handoff",
        )),
        EventKind::FluidReceiverAccepted => Some((
            Component::OrbitalFluid,
            "luminous.orbital-fluid-transfer.receipt.v0.1",
            "transfer_accepted",
        )),
        EventKind::LaunchAttemptSucceeded
        | EventKind::ApproachObserved
        | EventKind::CustomerAcknowledged => None,
    }
}

fn push_missing(reasons: &mut Vec<String>, code: &str) {
    reasons.push(format!("missing_{code}"));
}

fn push_violation(reasons: &mut Vec<String>, code: &str) {
    reasons.push(format!("limit_{code}"));
}

fn validate_evidence(
    event: &Event,
    kind: EventKind,
    pins: &BTreeMap<Component, &SourcePin>,
    reasons: &mut Vec<String>,
) {
    let Some((component, schema, decision)) = expected_component(kind) else {
        return;
    };
    let Some(evidence) = event.evidence.as_ref() else {
        push_missing(reasons, "component_receipt");
        return;
    };
    if evidence.component != component {
        push_violation(reasons, "component_identity_mismatch");
    }
    if evidence.receipt_schema != schema {
        push_violation(reasons, "receipt_schema_mismatch");
    }
    if !non_empty(&evidence.reference) {
        push_missing(reasons, "receipt_reference");
    }
    if !non_empty(&evidence.artifact_sha256) {
        push_missing(reasons, "artifact_digest");
    } else if !is_sha256(&evidence.artifact_sha256) {
        push_violation(reasons, "artifact_digest_malformed");
    }
    if !non_empty(&evidence.producer_id) {
        push_missing(reasons, "producer_identity");
    }
    if !non_empty(&evidence.authority_scope) {
        push_missing(reasons, "authority_scope");
    }
    if !non_empty(&evidence.context_of_use) {
        push_missing(reasons, "context_of_use");
    }
    if !non_empty(&evidence.claim_ceiling) {
        push_missing(reasons, "claim_ceiling");
    }
    if evidence.evidence_class != "synthetic" {
        push_violation(reasons, "non_synthetic_evidence_in_v0_1");
    }
    match pins.get(&component) {
        Some(pin) => {
            if evidence.receipt_schema != pin.receipt_schema
                || evidence.source_commit != pin.source_commit
            {
                push_violation(reasons, "source_pin_mismatch");
            }
        }
        None => push_missing(reasons, "source_pin"),
    }
    if !is_source_commit(&evidence.source_commit) {
        push_violation(reasons, "source_commit_malformed");
    }
    if evidence.decision == "storage_only_not_transfer_evidence"
        && kind == EventKind::FluidReceiverAccepted
    {
        push_missing(reasons, "fluid_transfer_demonstration");
    } else if evidence.decision != decision {
        push_violation(reasons, "component_decision_mismatch");
    }
}

pub fn evaluate_case(case: &Case, pins: &BTreeMap<Component, &SourcePin>) -> CaseResult {
    let mut reasons = Vec::new();
    let profile = &case.profile;
    if !non_empty(&case.case_id)
        || !non_empty(&profile.subject_id)
        || !non_empty(&profile.target_orbit_id)
        || !non_empty(&profile.destination_node_id)
        || !non_empty(&profile.interface_id)
        || !non_empty(&profile.fluid_id)
        || profile.interface_revision == 0
        || profile.minimum_accepted_mass_g == 0
    {
        push_violation(&mut reasons, "profile_invalid_or_incomplete");
    }

    let mut event_ids = BTreeSet::new();
    let mut previous_sequence = None;
    let mut previous_stage = 0_u8;
    let mut stages = BTreeMap::<u8, &Event>::new();
    let mut observed_stage_count = 0_u64;

    for event in &case.events {
        if !non_empty(&event.event_id) || !event_ids.insert(event.event_id.as_str()) {
            push_violation(&mut reasons, "event_id_empty_or_duplicate");
        }
        if previous_sequence.is_some_and(|previous| event.sequence <= previous) {
            push_violation(&mut reasons, "event_sequence_not_strictly_increasing");
        }
        previous_sequence = Some(event.sequence);

        let stage = match event.kind {
            EventKind::PayloadDeliveryAccepted => Some(1_u8),
            EventKind::CaptureHandoffAccepted => Some(2_u8),
            EventKind::FluidReceiverAccepted => Some(3_u8),
            EventKind::LaunchAttemptSucceeded
            | EventKind::ApproachObserved
            | EventKind::CustomerAcknowledged => None,
        };
        if let Some(stage) = stage {
            observed_stage_count += 1;
            if stage <= previous_stage {
                push_violation(&mut reasons, "service_stage_order_or_duplicate");
            }
            previous_stage = stage;
            if stages.insert(stage, event).is_some() {
                push_violation(&mut reasons, "duplicate_service_stage");
            }
            validate_evidence(event, event.kind, pins, &mut reasons);
        }
    }

    let required_stage_count = 3_u64;
    for stage in 1..=3 {
        if !stages.contains_key(&stage) {
            push_missing(&mut reasons, match stage {
                1 => "payload_delivery",
                2 => "capture_handoff",
                _ => "fluid_receiver_acceptance",
            });
        }
    }

    if let Some(event) = stages.get(&1) {
        match event.subject_id.as_deref() {
            Some(value) if value == profile.subject_id => {}
            Some(_) => push_violation(&mut reasons, "payload_subject_mismatch"),
            None => push_missing(&mut reasons, "payload_subject_identity"),
        }
        match event.target_orbit_id.as_deref() {
            Some(value) if value == profile.target_orbit_id => {}
            Some(_) => push_violation(&mut reasons, "target_orbit_mismatch"),
            None => push_missing(&mut reasons, "target_orbit_acceptance"),
        }
        match event.destination_node_id.as_deref() {
            Some(value) if value == profile.destination_node_id => {}
            Some(_) => push_violation(&mut reasons, "destination_node_mismatch"),
            None => push_missing(&mut reasons, "destination_node_acceptance"),
        }
    }

    if let Some(event) = stages.get(&2) {
        match event.subject_id.as_deref() {
            Some(value) if value == profile.subject_id => {}
            Some(_) => push_violation(&mut reasons, "capture_subject_mismatch"),
            None => push_missing(&mut reasons, "capture_subject_identity"),
        }
        match event.interface_id.as_deref() {
            Some(value) if value == profile.interface_id => {}
            Some(_) => push_violation(&mut reasons, "capture_interface_mismatch"),
            None => push_missing(&mut reasons, "capture_interface_identity"),
        }
        match event.interface_revision {
            Some(value) if value == profile.interface_revision => {}
            Some(_) => push_violation(&mut reasons, "capture_interface_revision_mismatch"),
            None => push_missing(&mut reasons, "capture_interface_revision"),
        }
    }

    if let Some(event) = stages.get(&3) {
        match event.subject_id.as_deref() {
            Some(value) if value == profile.subject_id => {}
            Some(_) => push_violation(&mut reasons, "fluid_receiver_subject_mismatch"),
            None => push_missing(&mut reasons, "fluid_receiver_identity"),
        }
        match event.receiver_id.as_deref() {
            Some(value) if value == profile.subject_id => {}
            Some(_) => push_violation(&mut reasons, "receiver_id_mismatch"),
            None => push_missing(&mut reasons, "receiver_id"),
        }
        match event.interface_id.as_deref() {
            Some(value) if value == profile.interface_id => {}
            Some(_) => push_violation(&mut reasons, "fluid_interface_mismatch"),
            None => push_missing(&mut reasons, "fluid_interface_identity"),
        }
        match event.interface_revision {
            Some(value) if value == profile.interface_revision => {}
            Some(_) => push_violation(&mut reasons, "fluid_interface_revision_mismatch"),
            None => push_missing(&mut reasons, "fluid_interface_revision"),
        }
        match event.fluid_id.as_deref() {
            Some(value) if value == profile.fluid_id => {}
            Some(_) => push_violation(&mut reasons, "fluid_identity_mismatch"),
            None => push_missing(&mut reasons, "fluid_identity"),
        }
        let storage_only = event
            .evidence
            .as_ref()
            .is_some_and(|evidence| evidence.decision == "storage_only_not_transfer_evidence");
        if !storage_only {
            match event.accepted_mass_g {
                Some(value) if value >= profile.minimum_accepted_mass_g => {}
                Some(_) => push_violation(&mut reasons, "accepted_mass_below_requirement"),
                None => push_missing(&mut reasons, "receiver_accepted_mass"),
            }
        }
    }

    reasons.sort();
    reasons.dedup();
    let has_violation = reasons.iter().any(|reason| reason.starts_with("limit_"));
    let has_missing = reasons.iter().any(|reason| reason.starts_with("missing_"));
    let actual_status = if has_violation {
        GateStatus::Rejected
    } else if has_missing {
        GateStatus::Indeterminate
    } else {
        GateStatus::AcceptedForSyntheticComposition
    };

    CaseResult {
        case_id: case.case_id.clone(),
        expected_status: case.expected_status,
        actual_status,
        reason_codes: reasons,
        required_stage_count,
        observed_stage_count,
    }
}

pub fn evaluate(fixture: &Fixture) -> Result<Report, String> {
    if fixture.suite_id != "orbital-service-composition"
        || fixture.suite_version != "0.1.0"
        || fixture.evidence_class != "synthetic"
        || !non_empty(&fixture.claim_ceiling)
    {
        return Err("unsupported suite identity, version, evidence class or claim ceiling".to_owned());
    }
    if fixture.cases.is_empty() {
        return Err("fixture must contain cases".to_owned());
    }
    let mut pins = BTreeMap::new();
    for pin in &fixture.source_pins {
        if !is_source_commit(&pin.source_commit)
            || !non_empty(&pin.receipt_schema)
            || pins.insert(pin.component, pin).is_some()
        {
            return Err("source pins must be unique, complete and use full commit SHAs".to_owned());
        }
    }
    if pins.len() != 3 {
        return Err("v0.1 requires launch, capture and fluid source pins".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(fixture.cases.len());
    let mut mismatches = 0_u64;
    for case in &fixture.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("case IDs must be unique".to_owned());
        }
        let result = evaluate_case(case, &pins);
        if result.expected_status != result.actual_status {
            mismatches += 1;
        }
        results.push(result);
    }

    let accepted = results.iter().filter(|item| {
        item.actual_status == GateStatus::AcceptedForSyntheticComposition
    }).count() as u64;
    let rejected = results.iter().filter(|item| item.actual_status == GateStatus::Rejected).count() as u64;
    let indeterminate = results.iter().filter(|item| item.actual_status == GateStatus::Indeterminate).count() as u64;
    let result = if mismatches == 0 { "pass" } else { "fail" };

    Ok(Report {
        suite_id: fixture.suite_id.clone(),
        suite_version: fixture.suite_version.clone(),
        evidence_class: fixture.evidence_class.clone(),
        result: result.to_owned(),
        summary: Summary {
            case_count: results.len() as u64,
            accepted_for_synthetic_composition: accepted,
            rejected,
            indeterminate,
            expected_result_mismatches: mismatches,
            operational_authority_grants: 0,
        },
        cases: results,
        claim_ceiling: fixture.claim_ceiling.clone(),
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String is infallible");
    }
    output
}

pub fn is_valid_source_commit(value: &str) -> bool {
    is_source_commit(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../fixtures/v0.1/cases.json"))
            .expect("frozen fixture must parse")
    }

    fn pins(data: &Fixture) -> BTreeMap<Component, &SourcePin> {
        data.source_pins.iter().map(|pin| (pin.component, pin)).collect()
    }

    #[test]
    fn frozen_corpus_matches_every_expected_disposition() {
        let data = fixture();
        let report = evaluate(&data).expect("fixture should evaluate");
        assert_eq!(report.result, "pass");
        assert_eq!(report.summary.case_count, 20);
        assert_eq!(report.summary.accepted_for_synthetic_composition, 1);
        assert_eq!(report.summary.rejected, 10);
        assert_eq!(report.summary.indeterminate, 9);
        assert_eq!(report.summary.expected_result_mismatches, 0);
        assert_eq!(report.summary.operational_authority_grants, 0);
    }

    fn receipt_bundle_inputs() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
        let data = fixture();
        let source_commit = |component: Component| {
            data.source_pins
                .iter()
                .find(|pin| pin.component == component)
                .expect("source pin must exist")
                .source_commit
                .clone()
        };
        let delivery = serde_json::json!({
            "receipt_schema": "luminous.earth-orbit-chain.receipt.v1",
            "validator_version": "0.1.0",
            "source_commit": source_commit(Component::EarthOrbitChain),
            "campaign_sha256": "a".repeat(64),
            "result": "pass",
            "report": {
                "evidence_class": "synthetic",
                "claim_ceiling": "Synthetic Earth-orbit campaign only.",
                "disposition": "ready",
                "cargo_deliveries": [{
                    "cargo_lot_id": "serviced-client-001",
                    "cargo_mass_grams": 1_000_000,
                    "target_orbit_id": "leo-550-km",
                    "destination_node_id": "orbit",
                    "terminal_node_id": "orbit",
                    "arrival_epoch_seconds": 50,
                    "status": "accepted"
                }]
            }
        });
        let capture = serde_json::json!({
            "receipt_schema": "luminous.orbital-capture-handoff.receipt.v0.1",
            "validator_version": "0.1.0",
            "source_commit": source_commit(Component::OrbitalCapture),
            "fixture_sha256": "b".repeat(64),
            "result": "pass",
            "report": {
                "evidence_class": "synthetic",
                "claim_ceiling": "Synthetic capture contract only.",
                "result": "pass",
                "cases": [{
                    "case_id": "NOMINAL_CAPTURE",
                    "status": "accepted_for_handoff",
                    "authority_granted": false,
                    "client_id": "serviced-client-001",
                    "interface_id": "COOPERATIVE-CAPTURE-PORT-A",
                    "interface_revision": 3,
                    "frame_id": "TARGET-LVLH-001",
                    "epoch_tick": 1000
                }]
            }
        });
        let fluid = serde_json::json!({
            "receipt_schema": "luminous.orbital-fluid-transfer.receipt.v0.1",
            "validator_version": "0.1.0",
            "source_commit": source_commit(Component::OrbitalFluid),
            "fixture_sha256": "c".repeat(64),
            "result": "pass",
            "report": {
                "evidence_class": "synthetic",
                "claim_ceiling": "Synthetic fluid accounting only.",
                "result": "pass",
                "scenarios": [{
                    "scenario_id": "STORABLE_LIQUID_NOMINAL",
                    "transfer_event_id": "transfer-storable-001",
                    "status": "transfer_accepted",
                    "fluid_class": "storable_liquid",
                    "fluid_id": "synthetic-hydrazine-analogue",
                    "receiver_id": "serviced-client-001",
                    "interface_id": "COOPERATIVE-CAPTURE-PORT-A",
                    "interface_revision": 3,
                    "phase": "liquid",
                    "receiver_accepted_g": 390_000,
                    "final_temperature_mk": 293_150,
                    "final_pressure_pa": 250_000,
                    "measurement_uncertainty_g": 1_000
                }]
            }
        });
        (
            include_str!("../fixtures/v0.1/cases.json").as_bytes().to_vec(),
            serde_json::to_vec(&delivery).expect("delivery receipt encodes"),
            serde_json::to_vec(&capture).expect("capture receipt encodes"),
            serde_json::to_vec(&fluid).expect("fluid receipt encodes"),
        )
    }

    #[test]
    fn generated_source_receipts_join_on_payload_client_receiver_and_interface() {
        let (fixture, delivery, capture, fluid) = receipt_bundle_inputs();
        let receipt = compose_receipt_bundle(&fixture, &delivery, &capture, &fluid)
            .expect("source receipts should compose");
        assert_eq!(receipt.result, "pass");
        assert!(!receipt.authority_granted);
        assert_eq!(receipt.joined_service.cargo_lot_id, "serviced-client-001");
        assert_eq!(receipt.joined_service.cargo_mass_grams, 1_000_000);
        assert_eq!(receipt.joined_service.target_orbit_id, "leo-550-km");
        assert_eq!(receipt.joined_service.destination_node_id, "orbit");
        assert_eq!(receipt.joined_service.client_id, "serviced-client-001");
        assert_eq!(receipt.joined_service.receiver_id, "serviced-client-001");
        assert_eq!(receipt.joined_service.capture_interface_revision, 3);
        assert_eq!(receipt.joined_service.accepted_receiver_mass_g, 390_000);
        assert_eq!(receipt.artifact_sha256.len(), 4);
        assert!(receipt.artifact_sha256.values().all(|digest| is_sha256(digest)));
    }

    #[test]
    fn mismatched_receiver_in_actual_source_receipt_is_rejected() {
        let (fixture, delivery, capture, fluid) = receipt_bundle_inputs();
        let mut value: serde_json::Value = serde_json::from_slice(&fluid).unwrap();
        value["report"]["scenarios"][0]["receiver_id"] = serde_json::json!("unrelated-receiver");
        let altered_fluid = serde_json::to_vec(&value).unwrap();
        assert!(
            compose_receipt_bundle(&fixture, &delivery, &capture, &altered_fluid)
                .unwrap_err()
                .contains("fluid receiver identity")
        );
    }

    #[test]
    fn source_receipt_commit_must_match_exact_frozen_pin() {
        let (fixture, delivery, mut capture, fluid) = receipt_bundle_inputs();
        let mut value: serde_json::Value = serde_json::from_slice(&capture).unwrap();
        value["source_commit"] = serde_json::json!("0000000000000000000000000000000000000000");
        capture = serde_json::to_vec(&value).unwrap();
        assert!(
            compose_receipt_bundle(&fixture, &delivery, &capture, &fluid)
                .unwrap_err()
                .contains("source commit does not match")
        );
    }

    #[test]
    fn missing_source_receipt_digest_is_not_accepted() {
        let (fixture, mut delivery, capture, fluid) = receipt_bundle_inputs();
        let mut value: serde_json::Value = serde_json::from_slice(&delivery).unwrap();
        value["campaign_sha256"] = serde_json::json!("not-a-sha256");
        delivery = serde_json::to_vec(&value).unwrap();
        assert!(
            compose_receipt_bundle(&fixture, &delivery, &capture, &fluid)
                .unwrap_err()
                .contains("campaign_sha256 is not a SHA-256 digest")
        );
    }

    #[test]
    fn nominal_chain_requires_delivery_capture_and_receiver_acceptance() {
        let data = fixture();
        let result = evaluate_case(&data.cases[0], &pins(&data));
        assert_eq!(result.actual_status, GateStatus::AcceptedForSyntheticComposition);
        assert_eq!(result.observed_stage_count, 3);
    }

    #[test]
    fn a_launch_attempt_is_not_payload_delivery() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "LAUNCH_ATTEMPT_ONLY").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn approach_observation_is_not_mechanical_capture() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "APPROACH_ONLY").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn wrong_payload_identity_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "CAPTURE_WRONG_SUBJECT").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn interface_revision_mismatch_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "INTERFACE_REVISION_MISMATCH").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn storage_only_is_not_receiver_accepted_transfer() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "STORAGE_ONLY_NOT_TRANSFER_EVIDENCE").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn receiver_mass_shortfall_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "RECEIVER_MASS_SHORTFALL").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn stale_component_commit_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "STALE_SOURCE_COMMIT").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn duplicate_event_identity_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "DUPLICATE_EVENT_ID").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn customer_acknowledgement_alone_is_not_technical_acceptance() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "CUSTOMER_ACK_ONLY").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn out_of_order_service_stages_are_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "OUT_OF_ORDER").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn missing_fluid_evidence_is_indeterminate() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "MISSING_FLUID_EVIDENCE").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn missing_receiver_identity_is_indeterminate() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[2].receiver_id = None;
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn wrong_receiver_identity_is_rejected() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[2].receiver_id = Some("unrelated-receiver".to_owned());
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn wrong_destination_node_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "WRONG_DESTINATION_NODE").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn wrong_target_orbit_is_rejected() {
        let data = fixture();
        let case = data.cases.iter().find(|item| item.case_id == "WRONG_TARGET_ORBIT").unwrap();
        assert_eq!(evaluate_case(case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn source_commit_digest_shape_is_checked() {
        assert!(is_valid_source_commit("0123456789abcdef0123456789abcdef01234567"));
        assert!(!is_valid_source_commit("main"));
    }

    #[test]
    fn missing_artifact_digest_is_indeterminate() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[0].evidence.as_mut().unwrap().artifact_sha256.clear();
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn missing_producer_or_authority_scope_is_indeterminate() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        let evidence = case.events[1].evidence.as_mut().unwrap();
        evidence.producer_id.clear();
        evidence.authority_scope.clear();
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Indeterminate);
    }

    #[test]
    fn malformed_artifact_digest_is_rejected() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[2].evidence.as_mut().unwrap().artifact_sha256 = "not-a-sha256".to_owned();
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Rejected);
    }

    #[test]
    fn evidence_component_mismatch_is_rejected() {
        let data = fixture();
        let mut case = data.cases[0].clone();
        case.events[1].evidence.as_mut().unwrap().component = Component::EarthOrbitChain;
        assert_eq!(evaluate_case(&case, &pins(&data)).actual_status, GateStatus::Rejected);
    }
}
