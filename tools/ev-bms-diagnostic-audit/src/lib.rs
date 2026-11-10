use jsonschema::{Validator, draft202012};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA_ID: &str =
    "https://luminousdynamics.io/schemas/transport/ev-bms-diagnostic-event/0.1";
const CORPUS_VERSION: &str = "ev-bms-diagnostic-corpus/0.1";
const EVENT_VERSION: &str = "ev-bms-diagnostic-event/0.1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    corpus_version: String,
    schema_id: String,
    cases: Vec<CorpusCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusCase {
    case_id: String,
    expectation: String,
    event: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    schema_version: String,
    event_id: String,
    evidence_class: String,
    asset: Asset,
    bms: Bms,
    observation_context: ObservationContext,
    observations: Vec<Observation>,
    diagnosis: Diagnosis,
    reaction: Reaction,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Asset {
    vehicle_ref: String,
    vehicle_configuration_digest: String,
    pack_ref: String,
    pack_configuration_digest: String,
    module_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bms {
    hardware_ref: String,
    software_digest: String,
    firmware_digest: String,
    calibration_digest: String,
    diagnostic_rule_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationContext {
    window_start_ms: u64,
    window_end_ms: u64,
    temperature_min_c: f64,
    temperature_max_c: f64,
    soc_min_fraction: f64,
    soc_max_fraction: f64,
    context_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    channel_id: String,
    value: f64,
    unit: String,
    sample_time_ms: u64,
    validity: String,
    sensor_calibration_digest: String,
    source_digest: String,
    evidence_class: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Diagnosis {
    state: String,
    classification_rule_digest: String,
    fault_candidates: Vec<FaultCandidate>,
    diagnosis_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FaultCandidate {
    fault_id: String,
    classification: String,
    severity: String,
    confidence: f64,
    evidence_digests: Vec<String>,
    model_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reaction {
    proposal: String,
    reason: String,
    authority_status: String,
    authority_ref: Option<String>,
    authority_digest: Option<String>,
    safety_policy_digest: String,
    operating_envelope_digest: String,
    execution_status: String,
    execution_receipt_digest: Option<String>,
    reconciliation_required: bool,
    mutation_blocked: bool,
    maintenance_event_digest: Option<String>,
    post_action_test_digest: Option<String>,
    verification_status: String,
    verifier_ref: Option<String>,
    verifier_digest: Option<String>,
}

#[derive(Debug, Serialize)]
struct CaseAudit {
    case_id: String,
    expectation: String,
    accepted: bool,
    errors: Vec<String>,
    event_id: Option<String>,
    diagnosis_state: Option<String>,
    reaction_proposal: Option<String>,
    authority_status: Option<String>,
    execution_status: Option<String>,
    verification_status: Option<String>,
    claim_ceiling: String,
}

#[derive(Debug, Serialize)]
struct ReportPayload {
    corpus_version: String,
    schema_id: String,
    schema_sha256: String,
    canonical_corpus_sha256: String,
    case_count: usize,
    accepted_count: usize,
    rejected_count: usize,
    cases: Vec<CaseAudit>,
}

#[derive(Debug, Serialize)]
pub struct CorpusReport {
    #[serde(flatten)]
    payload: ReportPayload,
    report_sha256: String,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn allowed_evidence_class(value: &str) -> bool {
    matches!(
        value,
        "physical_observation"
            | "derived_from_physical"
            | "simulation_output"
            | "synthetic_fixture"
            | "vendor_claim"
    )
}

fn schema_validator(bytes: &[u8]) -> Result<Validator, String> {
    let schema: Value =
        serde_json::from_slice(bytes).map_err(|error| format!("schema JSON: {error}"))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("$id").and_then(Value::as_str) != Some(SCHEMA_ID) {
        return Err("schema identity does not match EV BMS diagnostic v0.1".to_owned());
    }
    if !draft202012::meta::is_valid(&schema) {
        return Err("schema fails Draft 2020-12 meta-schema validation".to_owned());
    }
    draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("schema compilation: {error}"))
}

fn validate_event(event: &Event) -> Vec<String> {
    let mut errors = Vec::new();

    if event.schema_version != EVENT_VERSION || !nonempty(&event.event_id) {
        errors.push("event version or identity is invalid".to_owned());
    }
    if !allowed_evidence_class(&event.evidence_class) {
        errors.push("event evidence class is unknown".to_owned());
    }

    let asset = &event.asset;
    if !nonempty(&asset.vehicle_ref)
        || !nonempty(&asset.pack_ref)
        || !is_digest(&asset.vehicle_configuration_digest)
        || !is_digest(&asset.pack_configuration_digest)
        || asset.module_ref.as_deref().is_some_and(|value| !nonempty(value))
    {
        errors.push("physical asset identity/configuration evidence is incomplete".to_owned());
    }

    let bms = &event.bms;
    if !nonempty(&bms.hardware_ref)
        || !is_digest(&bms.software_digest)
        || !is_digest(&bms.firmware_digest)
        || !is_digest(&bms.calibration_digest)
        || !is_digest(&bms.diagnostic_rule_digest)
    {
        errors.push("BMS hardware, software, firmware or calibration identity is invalid".to_owned());
    }

    let context = &event.observation_context;
    if context.window_start_ms > context.window_end_ms
        || !context.temperature_min_c.is_finite()
        || !context.temperature_max_c.is_finite()
        || context.temperature_min_c > context.temperature_max_c
        || !context.soc_min_fraction.is_finite()
        || !context.soc_max_fraction.is_finite()
        || !(0.0..=1.0).contains(&context.soc_min_fraction)
        || !(0.0..=1.0).contains(&context.soc_max_fraction)
        || context.soc_min_fraction > context.soc_max_fraction
        || !is_digest(&context.context_digest)
    {
        errors.push("observation operating context or time window is invalid".to_owned());
    }

    let mut observation_digests = BTreeSet::new();
    let mut channel_samples = BTreeSet::new();
    let mut valid_evidence_digests = BTreeSet::new();
    for observation in &event.observations {
        if !nonempty(&observation.channel_id)
            || !nonempty(&observation.unit)
            || !observation.value.is_finite()
            || !is_digest(&observation.sensor_calibration_digest)
            || !is_digest(&observation.source_digest)
            || !allowed_evidence_class(&observation.evidence_class)
            || !matches!(observation.validity.as_str(), "valid" | "suspect" | "invalid")
        {
            errors.push(format!(
                "observation {} has invalid value, unit, validity or provenance",
                observation.channel_id
            ));
        }
        if observation.sample_time_ms < context.window_start_ms
            || observation.sample_time_ms > context.window_end_ms
        {
            errors.push(format!(
                "observation {} falls outside the declared time window",
                observation.channel_id
            ));
        }
        if !channel_samples.insert((observation.channel_id.clone(), observation.sample_time_ms)) {
            errors.push(format!(
                "observation {} has a duplicate channel/timestamp pair",
                observation.channel_id
            ));
        }
        if !observation_digests.insert(observation.source_digest.clone()) {
            errors.push(format!(
                "observation {} reuses a source digest",
                observation.channel_id
            ));
        }
        if observation.validity == "valid" && is_digest(&observation.source_digest) {
            valid_evidence_digests.insert(observation.source_digest.clone());
        }
    }

    let diagnosis = &event.diagnosis;
    if !is_digest(&diagnosis.classification_rule_digest)
        || !is_digest(&diagnosis.diagnosis_digest)
        || !matches!(
            diagnosis.state.as_str(),
            "not_identified" | "identified" | "classified" | "unknown" | "abstained"
        )
    {
        errors.push("diagnosis state or rule/provenance digest is invalid".to_owned());
    }
    if matches!(diagnosis.state.as_str(), "identified" | "classified")
        && diagnosis.fault_candidates.is_empty()
    {
        errors.push("identified/classified diagnosis requires at least one fault candidate".to_owned());
    }
    if matches!(diagnosis.state.as_str(), "not_identified" | "abstained")
        && !diagnosis.fault_candidates.is_empty()
    {
        errors.push("not_identified/abstained diagnosis cannot publish classified fault candidates".to_owned());
    }

    let mut fault_ids = BTreeSet::new();
    for candidate in &diagnosis.fault_candidates {
        if !nonempty(&candidate.fault_id)
            || !fault_ids.insert(candidate.fault_id.clone())
            || !is_digest(&candidate.model_digest)
            || !candidate.confidence.is_finite()
            || !(0.0..=1.0).contains(&candidate.confidence)
            || !matches!(
                candidate.classification.as_str(),
                "cell_undervoltage"
                    | "cell_overvoltage"
                    | "thermal_anomaly"
                    | "isolation_fault"
                    | "contactor_fault"
                    | "sensor_fault"
                    | "cell_imbalance"
                    | "capacity_loss"
                    | "resistance_growth"
                    | "other"
                    | "unknown"
            )
            || !matches!(
                candidate.severity.as_str(),
                "informational" | "minor" | "major" | "critical" | "unknown"
            )
            || candidate.evidence_digests.is_empty()
        {
            errors.push(format!(
                "fault candidate {} has invalid identity, confidence, class or evidence",
                candidate.fault_id
            ));
        }
        for digest in &candidate.evidence_digests {
            if !is_digest(digest) || !valid_evidence_digests.contains(digest) {
                errors.push(format!(
                    "fault candidate {} cites evidence not present in valid observations",
                    candidate.fault_id
                ));
            }
        }
    }
    if diagnosis.state == "classified"
        && diagnosis
            .fault_candidates
            .iter()
            .any(|candidate| candidate.classification == "unknown")
    {
        errors.push("classified diagnosis cannot contain an unknown fault class".to_owned());
    }

    let reaction = &event.reaction;
    if !matches!(
        reaction.proposal.as_str(),
        "no_action" | "monitor" | "warn" | "derate" | "isolate" | "service" | "replace"
    ) || !nonempty(&reaction.reason)
        || !is_digest(&reaction.safety_policy_digest)
        || !is_digest(&reaction.operating_envelope_digest)
    {
        errors.push("reaction proposal, reason or safety envelope identity is invalid".to_owned());
    }
    if !matches!(
        reaction.authority_status.as_str(),
        "not_requested" | "approved" | "denied"
    ) {
        errors.push("reaction authority status is unknown".to_owned());
    }
    match reaction.authority_status.as_str() {
        "not_requested" => {
            if reaction.authority_ref.is_some() || reaction.authority_digest.is_some() {
                errors.push("not-requested authority must not fabricate an approval reference".to_owned());
            }
        }
        "approved" | "denied" => {
            if reaction.authority_ref.as_deref().is_none_or(|value| !nonempty(value))
                || reaction.authority_digest.as_deref().is_none_or(|value| !is_digest(value))
            {
                errors.push("authority decision requires an authority reference and digest".to_owned());
            }
        }
        _ => {}
    }

    if !matches!(
        reaction.execution_status.as_str(),
        "not_executed" | "acknowledged" | "executed" | "failed" | "unknown"
    ) {
        errors.push("reaction execution status is unknown".to_owned());
    }
    match reaction.execution_status.as_str() {
        "not_executed" => {
            if reaction.execution_receipt_digest.is_some() {
                errors.push("not-executed reaction cannot carry an execution receipt".to_owned());
            }
        }
        "acknowledged" | "executed" | "failed" | "unknown" => {
            if reaction.authority_status != "approved"
                || reaction.authority_ref.is_none()
                || reaction.authority_digest.is_none()
            {
                errors.push("acknowledged/executed reaction requires explicit authority approval".to_owned());
            }
            if reaction
                .execution_receipt_digest
                .as_deref()
                .is_none_or(|digest| !is_digest(digest))
            {
                errors.push("acknowledged/executed reaction requires an execution receipt digest".to_owned());
            }
        }
        _ => {}
    }

    let requires_reconciliation = reaction.execution_status == "unknown"
        || reaction.execution_status == "failed";
    if requires_reconciliation
        && (!reaction.reconciliation_required || !reaction.mutation_blocked)
    {
        errors.push("unknown/failed execution requires reconciliation and mutation blocking".to_owned());
    }
    if reaction.reconciliation_required && !reaction.mutation_blocked {
        errors.push("reconciliation-required event must block subsequent mutations".to_owned());
    }
    if reaction.mutation_blocked && !reaction.reconciliation_required {
        errors.push("mutation block requires an explicit reconciliation state".to_owned());
    }
    if reaction.execution_status == "not_executed"
        && (reaction.maintenance_event_digest.is_some()
            || reaction.post_action_test_digest.is_some()
            || reaction.verification_status != "not_applicable")
    {
        errors.push("not-executed reaction cannot claim service, post-action test or verification".to_owned());
    }

    let service_or_replacement = matches!(reaction.proposal.as_str(), "service" | "replace");
    if service_or_replacement && reaction.execution_status == "executed"
        && reaction
            .maintenance_event_digest
            .as_deref()
            .is_none_or(|digest| !is_digest(digest))
    {
        errors.push("executed service/replacement requires a maintenance event digest".to_owned());
    }
    if !service_or_replacement && reaction.maintenance_event_digest.is_some() {
        errors.push("non-service reaction cannot claim a maintenance event".to_owned());
    }
    if reaction.maintenance_event_digest.as_deref().is_some_and(|digest| !is_digest(digest))
        || reaction.post_action_test_digest.as_deref().is_some_and(|digest| !is_digest(digest))
    {
        errors.push("maintenance or post-action evidence digest is malformed".to_owned());
    }

    if !matches!(
        reaction.verification_status.as_str(),
        "not_applicable" | "pending" | "verified" | "rejected"
    ) {
        errors.push("verification status is unknown".to_owned());
    }
    match reaction.verification_status.as_str() {
        "not_applicable" => {
            if reaction.verifier_ref.is_some() || reaction.verifier_digest.is_some() {
                errors.push("not-applicable verification cannot carry a verifier receipt".to_owned());
            }
        }
        "pending" => {
            if reaction.execution_status != "executed" {
                errors.push("pending post-action verification requires an executed reaction".to_owned());
            }
            if reaction.verifier_ref.is_some() || reaction.verifier_digest.is_some() {
                errors.push("pending verification cannot claim an independent verifier receipt".to_owned());
            }
        }
        "verified" | "rejected" => {
            if reaction.execution_status != "executed"
                || reaction
                    .post_action_test_digest
                    .as_deref()
                    .is_none_or(|digest| !is_digest(digest))
                || reaction.verifier_ref.as_deref().is_none_or(|value| !nonempty(value))
                || reaction.verifier_digest.as_deref().is_none_or(|digest| !is_digest(digest))
            {
                errors.push("final verification requires execution, post-action test and independent verifier evidence".to_owned());
            }
            if reaction.verifier_ref == reaction.authority_ref {
                errors.push("independent verifier must be distinct from the reaction authority".to_owned());
            }
        }
        _ => {}
    }

    errors
}

fn canonicalize_corpus(value: &mut Value) {
    let Some(cases) = value.get_mut("cases").and_then(Value::as_array_mut) else {
        return;
    };
    cases.sort_by(|left, right| {
        left.get("case_id")
            .and_then(Value::as_str)
            .cmp(&right.get("case_id").and_then(Value::as_str))
    });
    for case in cases {
        if let Some(event) = case.get_mut("event") {
            if let Some(observations) = event.get_mut("observations").and_then(Value::as_array_mut) {
                observations.sort_by(|left, right| {
                    (
                        left.get("channel_id").and_then(Value::as_str),
                        left.get("sample_time_ms").and_then(Value::as_u64),
                    )
                        .cmp(&(
                            right.get("channel_id").and_then(Value::as_str),
                            right.get("sample_time_ms").and_then(Value::as_u64),
                        ))
                });
            }
            if let Some(candidates) = event
                .get_mut("diagnosis")
                .and_then(|diagnosis| diagnosis.get_mut("fault_candidates"))
                .and_then(Value::as_array_mut)
            {
                candidates.sort_by(|left, right| {
                    left.get("fault_id")
                        .and_then(Value::as_str)
                        .cmp(&right.get("fault_id").and_then(Value::as_str))
                });
                for candidate in candidates {
                    if let Some(digests) = candidate.get_mut("evidence_digests").and_then(Value::as_array_mut) {
                        digests.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
                    }
                }
            }
        }
    }
}

pub fn audit_corpus(schema_bytes: &[u8], corpus_bytes: &[u8]) -> Result<CorpusReport, String> {
    let validator = schema_validator(schema_bytes)?;
    let mut value: Value =
        serde_json::from_slice(corpus_bytes).map_err(|error| format!("corpus JSON: {error}"))?;
    canonicalize_corpus(&mut value);
    let canonical = serde_json::to_vec(&value)
        .map_err(|error| format!("canonical corpus: {error}"))?;
    let corpus: Corpus =
        serde_json::from_slice(&canonical).map_err(|error| format!("typed corpus: {error}"))?;
    if corpus.corpus_version != CORPUS_VERSION || corpus.schema_id != SCHEMA_ID {
        return Err("corpus version or schema identity mismatch".to_owned());
    }

    let mut seen = BTreeSet::new();
    let mut cases = Vec::new();
    let mut accepted_count = 0usize;
    let mut rejected_count = 0usize;
    for case in corpus.cases {
        if !nonempty(&case.case_id) || !seen.insert(case.case_id.clone()) {
            return Err(format!("case identity is empty or duplicated: {}", case.case_id));
        }
        if !matches!(case.expectation.as_str(), "accept" | "reject") {
            return Err(format!("case {} has invalid expectation", case.case_id));
        }
        let schema_errors: Vec<String> = validator
            .iter_errors(&case.event)
            .map(|error| error.to_string())
            .collect();
        let result = if schema_errors.is_empty() {
            match serde_json::from_value::<Event>(case.event) {
                Ok(event) => {
                    let errors = validate_event(&event);
                    if errors.is_empty() {
                        Ok(event)
                    } else {
                        Err(errors)
                    }
                }
                Err(error) => Err(vec![format!("typed event parse: {error}")]),
            }
        } else {
            Err(schema_errors)
        };
        let accepted = result.is_ok();
        if accepted != (case.expectation == "accept") {
            let detail = match &result {
                Ok(_) => "unexpected acceptance".to_owned(),
                Err(errors) => errors.join("; "),
            };
            return Err(format!("case {} expectation mismatch: {}", case.case_id, detail));
        }
        let (event, errors) = match result {
            Ok(event) => {
                accepted_count += 1;
                (Some(event), Vec::new())
            }
            Err(errors) => {
                rejected_count += 1;
                (None, errors)
            }
        };
        let claim_ceiling = match event.as_ref().map(|event| event.evidence_class.as_str()) {
            Some("physical_observation" | "derived_from_physical") => {
                "producer_declared_diagnostic_evidence_not_independently_attested"
            }
            Some("simulation_output") => "simulation_only",
            Some("synthetic_fixture") => "synthetic_software_validation_only",
            Some("vendor_claim") => "vendor_claim_only",
            _ => "rejected_event",
        }
        .to_owned();
        cases.push(CaseAudit {
            case_id: case.case_id,
            expectation: case.expectation,
            accepted,
            errors,
            event_id: event.as_ref().map(|event| event.event_id.clone()),
            diagnosis_state: event.as_ref().map(|event| event.diagnosis.state.clone()),
            reaction_proposal: event.as_ref().map(|event| event.reaction.proposal.clone()),
            authority_status: event.as_ref().map(|event| event.reaction.authority_status.clone()),
            execution_status: event.as_ref().map(|event| event.reaction.execution_status.clone()),
            verification_status: event
                .as_ref()
                .map(|event| event.reaction.verification_status.clone()),
            claim_ceiling,
        });
    }

    let payload = ReportPayload {
        corpus_version: CORPUS_VERSION.to_owned(),
        schema_id: SCHEMA_ID.to_owned(),
        schema_sha256: sha256(schema_bytes),
        canonical_corpus_sha256: sha256(&canonical),
        case_count: cases.len(),
        accepted_count,
        rejected_count,
        cases,
    };
    let bytes =
        serde_json::to_vec(&payload).map_err(|error| format!("report serialization: {error}"))?;
    Ok(CorpusReport {
        payload,
        report_sha256: sha256(&bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SCHEMA: &[u8] =
        include_bytes!("../../../schemas/transport/ev_bms_diagnostic_event_v0_1.schema.json");
    const CORPUS: &[u8] =
        include_bytes!("../../../fixtures/ev_battery/bms_diagnostic_event_scenarios_v0_1.json");

    fn report() -> CorpusReport {
        audit_corpus(SCHEMA, CORPUS).expect("frozen BMS diagnostic event corpus")
    }

    #[test]
    fn frozen_corpus_accepts_five_and_rejects_five_controls() {
        let report = report();
        assert_eq!(report.payload.case_count, 10);
        assert_eq!(report.payload.accepted_count, 5);
        assert_eq!(report.payload.rejected_count, 5);
    }

    #[test]
    fn recommendation_is_not_authorization_or_execution() {
        let report = report();
        let case = report.payload.cases.iter()
            .find(|case| case.case_id == "recommendation-not-authorized")
            .unwrap();
        assert!(case.accepted);
        assert_eq!(case.execution_status.as_deref(), Some("not_executed"));
        assert_eq!(case.authority_status.as_deref(), Some("not_requested"));
    }

    #[test]
    fn unknown_execution_requires_reconciliation_and_mutation_block() {
        let report = report();
        let case = report.payload.cases.iter()
            .find(|case| case.case_id == "unknown-execution-blocks-mutation")
            .unwrap();
        assert!(case.accepted);
        assert_eq!(case.execution_status.as_deref(), Some("unknown"));
    }

    #[test]
    fn service_execution_is_not_independent_verification() {
        let report = report();
        let case = report.payload.cases.iter()
            .find(|case| case.case_id == "service-executed-not-yet-verified")
            .unwrap();
        assert!(case.accepted);
        assert_eq!(case.execution_status.as_deref(), Some("executed"));
        assert_eq!(case.verification_status.as_deref(), Some("pending"));
    }

    #[test]
    fn invalid_reaction_and_provenance_controls_are_rejected() {
        let report = report();
        for id in [
            "unauthorized-execution-rejected",
            "verified-without-post-test-rejected",
            "classified-without-support-rejected",
            "observation-outside-window-rejected",
            "acknowledged-without-authorization-rejected",
        ] {
            let case = report.payload.cases.iter()
                .find(|case| case.case_id == id)
                .unwrap();
            assert!(!case.accepted, "negative control {id} should reject");
            assert_eq!(case.execution_status, None);
        }
    }

    #[test]
    fn canonicalization_is_invariant_to_case_observation_and_candidate_order() {
        let original = report();
        let mut value: Value = serde_json::from_slice(CORPUS).expect("corpus JSON");
        value["cases"].as_array_mut().unwrap().reverse();
        for case in value["cases"].as_array_mut().unwrap() {
            case["event"]["observations"].as_array_mut().unwrap().reverse();
            case["event"]["diagnosis"]["fault_candidates"].as_array_mut().unwrap().reverse();
            for candidate in case["event"]["diagnosis"]["fault_candidates"].as_array_mut().unwrap() {
                candidate["evidence_digests"].as_array_mut().unwrap().reverse();
            }
        }
        let bytes = serde_json::to_vec(&value).expect("permuted corpus");
        let replay = audit_corpus(SCHEMA, &bytes).expect("permuted audit");
        assert_eq!(original.report_sha256, replay.report_sha256);
    }
}
