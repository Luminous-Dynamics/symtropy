// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Declarative bearer-neutral energy-service capability matching.
//! It does not authenticate issuers, resolve artifact bytes, control devices or issue permits.

use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process::ExitCode};

const OFFER_HEADER: [&str; 20] = [
    "profile_id",
    "provider_id",
    "profile_version",
    "bearer",
    "field_regime",
    "receiver_hardware_sha256",
    "receiver_geometry_sha256",
    "coordinate_frame_sha256",
    "supported_motion_states",
    "supported_environment_strata",
    "session_modes",
    "evidence_role",
    "receipt_id",
    "receipt_sha256",
    "evidence_class",
    "review_status",
    "disposition",
    "valid_until_us",
    "claim_ceiling",
    "schema_version",
];

const REQUEST_HEADER: [&str; 13] = [
    "request_id",
    "allowed_bearers",
    "allowed_field_regimes",
    "receiver_hardware_sha256",
    "receiver_geometry_sha256",
    "coordinate_frame_sha256",
    "motion_state",
    "environment_stratum",
    "session_mode",
    "required_evidence_roles",
    "minimum_evidence_tier",
    "require_independent_review",
    "decision_time_us",
];

const MAX_PROFILES: usize = 4096;
const MAX_RECEIPTS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
struct OfferMetadata {
    profile_id: String,
    provider_id: String,
    profile_version: u32,
    bearer: String,
    field_regime: String,
    receiver_hardware_sha256: String,
    receiver_geometry_sha256: String,
    coordinate_frame_sha256: String,
    claim_ceiling: String,
    supported_motion_states: BTreeSet<String>,
    supported_environment_strata: BTreeSet<String>,
    session_modes: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EvidenceReceipt {
    role: String,
    receipt_id: String,
    receipt_sha256: String,
    evidence_class: String,
    review_status: String,
    disposition: String,
    valid_until_us: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Offer {
    metadata: OfferMetadata,
    receipts: BTreeMap<String, EvidenceReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Request {
    request_id: String,
    allowed_bearers: BTreeSet<String>,
    allowed_field_regimes: BTreeSet<String>,
    receiver_hardware_sha256: String,
    receiver_geometry_sha256: String,
    coordinate_frame_sha256: String,
    motion_state: String,
    environment_stratum: String,
    session_mode: String,
    required_evidence_roles: BTreeSet<String>,
    minimum_evidence_tier: String,
    require_independent_review: bool,
    decision_time_us: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MatchResult {
    profile_id: String,
    provider_id: String,
    disposition: &'static str,
    reason: &'static str,
    bearer: String,
    matched_roles: BTreeSet<String>,
    matched_receipt_ids: BTreeSet<String>,
    matched_receipt_digests: BTreeSet<String>,
    weakest_evidence_class: Option<String>,
    independently_reviewed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evaluation {
    request_id: String,
    results: Vec<MatchResult>,
}

const BLOCKING_DISPOSITIONS: &[&str] = &[
    "AMBIGUOUS",
    "STALE",
    "OUT_OF_DOMAIN",
    "UNKNOWN",
    "REVOKED",
    "NOT_ADMITTED",
    "NOT_EVALUATED",
];

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("{field}: empty or untrimmed integer"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{field}: expected non-negative integer"))
}

fn parse_u32(value: &str, field: &str) -> Result<u32, String> {
    u32::try_from(parse_u64(value, field)?)
        .map_err(|_| format!("{field}: integer exceeds u32"))
}

fn specific_identifier(value: &str) -> bool {
    valid_id(value) && !matches!(value, "ANY" | "UNKNOWN" | "UNSPECIFIED" | "*")
}

fn parse_set(value: &str, field: &str) -> Result<BTreeSet<String>, String> {
    let items: Vec<&str> = value.split(';').collect();
    if items.is_empty() || items.iter().any(|item| !specific_identifier(item)) {
        return Err(format!("{field}: empty, untrimmed or invalid set member"));
    }
    let parsed: BTreeSet<String> = items.iter().map(|item| (*item).to_string()).collect();
    if parsed.len() != items.len() {
        return Err(format!("{field}: duplicate set member"));
    }
    Ok(parsed)
}

fn allowed_bearer(value: &str) -> bool {
    matches!(
        value,
        "CONDUCTIVE"
            | "IPT"
            | "CPT"
            | "BATTERY_EXCHANGE"
            | "RF_MICROWAVE"
            | "OPTICAL"
            | "POWER_LINE"
    )
}

fn allowed_field_regime(value: &str) -> bool {
    matches!(
        value,
        "CONTACT" | "NEAR_FIELD" | "FAR_FIELD" | "ENERGY_CARRIER" | "NOT_APPLICABLE"
    )
}

fn allowed_motion_state(value: &str) -> bool {
    matches!(
        value,
        "PARKED"
            | "DOCKED"
            | "HOVER"
            | "TRANSLATING"
            | "CLIMB_DESCENT"
            | "DYNAMIC_FLIGHT"
            | "DEPARTURE"
            | "REACQUISITION"
            | "LANDING_APPROACH"
            | "BATTERY_SWAP"
    )
}

fn allowed_session_mode(value: &str) -> bool {
    matches!(
        value,
        "DOCK" | "ACQUIRE" | "TRACK" | "POWER" | "DEPART" | "REACQUIRE" | "SWAP"
    )
}

fn allowed_evidence_role(value: &str) -> bool {
    matches!(
        value,
        "PHYSICAL_ENVELOPE"
            | "OBSERVABILITY"
            | "SESSION_ADMISSION"
            | "AUTHORIZATION"
            | "LOCAL_PROTECTION"
            | "SITE_ALLOCATION"
            | "SOURCE_TO_LOAD_METROLOGY"
            | "STANDARDS_APPLICABILITY"
            | "MECHANICAL_CAPTURE"
            | "CAPTURE_OBSERVABILITY_COMPOSITION"
            | "REPLENISHMENT_STRATEGY"
            | "POSE_CALIBRATION"
            | "ASSET_LIFECYCLE"
            | "INDEPENDENT_REVIEW"
    )
}

fn allowed_evidence_class(value: &str) -> bool {
    matches!(
        value,
        "DECLARED"
            | "SYNTHETIC_ONLY"
            | "SIMULATED"
            | "BENCH_MEASURED"
            | "FIELD_MEASURED"
    )
}

fn evidence_tier(value: &str) -> Option<u8> {
    match value {
        "SYNTHETIC_ONLY" => Some(0),
        "SIMULATED" => Some(1),
        "BENCH_MEASURED" => Some(2),
        "FIELD_MEASURED" => Some(3),
        _ => None,
    }
}

fn parse_offers(input: &str) -> Result<Vec<Offer>, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["offer input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != OFFER_HEADER.as_slice() {
        return Err(vec![format!(
            "offer header must contain exactly {} columns",
            OFFER_HEADER.len()
        )]);
    }

    let mut offers = BTreeMap::<String, Offer>::new();
    let mut receipt_ids = BTreeSet::new();
    let mut errors = Vec::new();

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("offer line {line_no}: blank rows are not allowed"));
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != OFFER_HEADER.len() {
            errors.push(format!(
                "offer line {line_no}: expected {} columns, found {}",
                OFFER_HEADER.len(),
                fields.len()
            ));
            continue;
        }

        let profile_id = fields[0];
        let profile_version = match parse_u32(fields[2], "profile_version") {
            Ok(value) if value == 1 => value,
            Ok(_) => {
                errors.push(format!("offer line {line_no}: unsupported profile_version; v0 accepts exactly 1"));
                continue;
            }
            Err(error) => {
                errors.push(format!("offer line {line_no}: {error}"));
                continue;
            }
        };

        let motion_states = match parse_set(fields[8], "supported_motion_states") {
            Ok(values) => values,
            Err(error) => {
                errors.push(format!("offer line {line_no}: {error}"));
                continue;
            }
        };
        let environments = match parse_set(fields[9], "supported_environment_strata") {
            Ok(values) => values,
            Err(error) => {
                errors.push(format!("offer line {line_no}: {error}"));
                continue;
            }
        };
        let session_modes = match parse_set(fields[10], "session_modes") {
            Ok(values) => values,
            Err(error) => {
                errors.push(format!("offer line {line_no}: {error}"));
                continue;
            }
        };

        let metadata = OfferMetadata {
            profile_id: profile_id.to_string(),
            provider_id: fields[1].to_string(),
            profile_version,
            bearer: fields[3].to_string(),
            field_regime: fields[4].to_string(),
            receiver_hardware_sha256: fields[5].to_string(),
            receiver_geometry_sha256: fields[6].to_string(),
            coordinate_frame_sha256: fields[7].to_string(),
            claim_ceiling: fields[18].to_string(),
            supported_motion_states: motion_states,
            supported_environment_strata: environments,
            session_modes,
        };

        if !valid_id(profile_id)
            || !valid_id(&metadata.provider_id)
            || !allowed_bearer(&metadata.bearer)
            || !allowed_field_regime(&metadata.field_regime)
            || !digest_valid(&metadata.receiver_hardware_sha256)
            || !digest_valid(&metadata.receiver_geometry_sha256)
            || !digest_valid(&metadata.coordinate_frame_sha256)
            || metadata.claim_ceiling.trim().is_empty()
            || metadata.claim_ceiling.len() > 1024
            || metadata.claim_ceiling.trim() != metadata.claim_ceiling
            || fields[19] != "1"
            || metadata
                .supported_motion_states
                .iter()
                .any(|value| !allowed_motion_state(value))
            || metadata.session_modes.iter().any(|value| !allowed_session_mode(value))
        {
            errors.push(format!(
                "offer line {line_no}: invalid profile identity, bearer, field regime, digest, motion state or session mode"
            ));
            continue;
        }

        let offer = offers.entry(profile_id.to_string()).or_insert_with(|| Offer {
            metadata: metadata.clone(),
            receipts: BTreeMap::new(),
        });
        if offer.metadata != metadata {
            errors.push(format!(
                "offer line {line_no}: repeated profile {profile_id} has inconsistent metadata"
            ));
            continue;
        }

        let role = fields[11];
        let receipt_id = fields[12];
        let receipt_sha256 = fields[13];
        let evidence_class = fields[14];
        let review_status = fields[15];
        let disposition = fields[16];
        if !allowed_evidence_role(role)
            || !valid_id(receipt_id)
            || !digest_valid(receipt_sha256)
            || !allowed_evidence_class(evidence_class)
            || !matches!(review_status, "NOT_REVIEWED" | "INDEPENDENTLY_REVIEWED")
            || !(disposition == "QUALIFIED_FOR_DECLARED_SCOPE"
                || disposition == "DECLARED_ONLY"
                || BLOCKING_DISPOSITIONS.contains(&disposition))
        {
            errors.push(format!(
                "offer line {line_no}: invalid evidence role, receipt, class or disposition"
            ));
            continue;
        }

        let valid_until_us = match parse_u64(fields[17], "valid_until_us") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("offer line {line_no}: {error}"));
                continue;
            }
        };
        if offer.receipts.contains_key(role) {
            errors.push(format!(
                "offer line {line_no}: profile {profile_id} repeats evidence role {role}"
            ));
            continue;
        }
        if !receipt_ids.insert(receipt_id.to_string()) {
            errors.push(format!("offer line {line_no}: duplicate receipt_id {receipt_id}"));
            continue;
        }

        offer.receipts.insert(
            role.to_string(),
            EvidenceReceipt {
                role: role.to_string(),
                receipt_id: receipt_id.to_string(),
                receipt_sha256: receipt_sha256.to_string(),
                evidence_class: evidence_class.to_string(),
                review_status: review_status.to_string(),
                disposition: disposition.to_string(),
                valid_until_us,
            },
        );

        if offer.receipts.len() > MAX_RECEIPTS {
            errors.push(format!(
                "profile {profile_id} exceeds {MAX_RECEIPTS} evidence receipts"
            ));
            break;
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    if offers.is_empty() {
        return Err(vec!["offer file has no service profiles".to_string()]);
    }
    if offers.len() > MAX_PROFILES {
        return Err(vec![format!("offer file exceeds {MAX_PROFILES} profiles")]);
    }
    Ok(offers.into_values().collect())
}

fn parse_request(input: &str) -> Result<Request, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["request input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != REQUEST_HEADER.as_slice() {
        return Err(vec![format!(
            "request header must contain exactly {} columns",
            REQUEST_HEADER.len()
        )]);
    }
    let Some(raw_line) = lines.next() else {
        return Err(vec!["request file has no demand descriptor".to_string()]);
    };
    let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != REQUEST_HEADER.len() {
        return Err(vec![format!(
            "request row must contain exactly {} columns",
            REQUEST_HEADER.len()
        )]);
    }
    if lines.next().is_some() {
        return Err(vec!["request file must contain exactly one demand descriptor".to_string()]);
    }

    let allowed_bearers = parse_set(fields[1], "allowed_bearers").map_err(|e| vec![e])?;
    let allowed_field_regimes =
        parse_set(fields[2], "allowed_field_regimes").map_err(|e| vec![e])?;
    let required_evidence_roles =
        parse_set(fields[9], "required_evidence_roles").map_err(|e| vec![e])?;
    let require_independent_review = match fields[11] {
        "true" => true,
        "false" => false,
        other => {
            return Err(vec![format!(
                "require_independent_review must be true or false, found {other:?}"
            )]);
        }
    };

    let request = Request {
        request_id: fields[0].to_string(),
        allowed_bearers,
        allowed_field_regimes,
        receiver_hardware_sha256: fields[3].to_string(),
        receiver_geometry_sha256: fields[4].to_string(),
        coordinate_frame_sha256: fields[5].to_string(),
        motion_state: fields[6].to_string(),
        environment_stratum: fields[7].to_string(),
        session_mode: fields[8].to_string(),
        required_evidence_roles,
        minimum_evidence_tier: fields[10].to_string(),
        require_independent_review,
        decision_time_us: parse_u64(fields[12], "decision_time_us")
            .map_err(|error| vec![error])?,
    };

    if !valid_id(&request.request_id)
        || request.allowed_bearers.iter().any(|value| !allowed_bearer(value))
        || request
            .allowed_field_regimes
            .iter()
            .any(|value| !allowed_field_regime(value))
        || !digest_valid(&request.receiver_hardware_sha256)
        || !digest_valid(&request.receiver_geometry_sha256)
        || !digest_valid(&request.coordinate_frame_sha256)
        || !allowed_motion_state(&request.motion_state)
        || !specific_identifier(&request.environment_stratum)
        || !allowed_session_mode(&request.session_mode)
        || request
            .required_evidence_roles
            .iter()
            .any(|value| !allowed_evidence_role(value) || value == "INDEPENDENT_REVIEW")
        || evidence_tier(&request.minimum_evidence_tier).is_none()
    {
        return Err(vec![
            "request has invalid identity, bearer/field/motion/session enum, digest, evidence role or minimum evidence tier".to_string()
        ]);
    }
    if request.require_independent_review
        && request.required_evidence_roles.contains("INDEPENDENT_REVIEW")
    {
        return Err(vec![
            "INDEPENDENT_REVIEW is a separate review predicate, not a required measurement role"
                .to_string(),
        ]);
    }

    Ok(request)
}

fn no_match(offer: &Offer, reason: &'static str) -> MatchResult {
    MatchResult {
        profile_id: offer.metadata.profile_id.clone(),
        provider_id: offer.metadata.provider_id.clone(),
        disposition: "NO_MATCH",
        reason,
        bearer: offer.metadata.bearer.clone(),
        matched_roles: BTreeSet::new(),
        matched_receipt_ids: BTreeSet::new(),
        matched_receipt_digests: BTreeSet::new(),
        weakest_evidence_class: None,
        independently_reviewed: false,
    }
}

fn match_offer(offer: &Offer, request: &Request) -> MatchResult {
    let metadata = &offer.metadata;
    if !request.allowed_bearers.contains(&metadata.bearer) {
        return no_match(offer, "BEARER_NOT_ALLOWED");
    }
    if !request.allowed_field_regimes.contains(&metadata.field_regime) {
        return no_match(offer, "FIELD_REGIME_NOT_ALLOWED");
    }
    if request.receiver_hardware_sha256 != metadata.receiver_hardware_sha256 {
        return no_match(offer, "RECEIVER_HARDWARE_MISMATCH");
    }
    if request.receiver_geometry_sha256 != metadata.receiver_geometry_sha256 {
        return no_match(offer, "RECEIVER_GEOMETRY_MISMATCH");
    }
    if request.coordinate_frame_sha256 != metadata.coordinate_frame_sha256 {
        return no_match(offer, "COORDINATE_FRAME_MISMATCH");
    }
    if !metadata
        .supported_motion_states
        .contains(&request.motion_state)
    {
        return no_match(offer, "MOTION_STATE_UNSUPPORTED");
    }
    if !metadata
        .supported_environment_strata
        .contains(&request.environment_stratum)
    {
        return no_match(offer, "ENVIRONMENT_UNSUPPORTED");
    }
    if !metadata.session_modes.contains(&request.session_mode) {
        return no_match(offer, "SESSION_MODE_UNSUPPORTED");
    }

    let mut matched_roles = BTreeSet::new();
    let mut matched_receipt_ids = BTreeSet::new();
    let mut matched_receipt_digests = BTreeSet::new();
    let mut evidence_tiers = Vec::new();
    let mut declared_only = false;
    let mut independently_reviewed = false;

    for role in &request.required_evidence_roles {
        let Some(receipt) = offer.receipts.get(role) else {
            return no_match(offer, "MISSING_REQUIRED_EVIDENCE_ROLE");
        };
        if BLOCKING_DISPOSITIONS.contains(&receipt.disposition.as_str()) {
            return no_match(offer, "BLOCKING_EVIDENCE_DISPOSITION");
        }
        if receipt.valid_until_us < request.decision_time_us {
            return no_match(offer, "REQUIRED_EVIDENCE_EXPIRED");
        }
        matched_receipt_ids.insert(receipt.receipt_id.clone());
        matched_receipt_digests.insert(receipt.receipt_sha256.clone());
        if receipt.disposition == "DECLARED_ONLY" || receipt.evidence_class == "DECLARED" {
            declared_only = true;
            matched_roles.insert(role.clone());
            continue;
        }
        let Some(tier) = evidence_tier(&receipt.evidence_class) else {
            return no_match(offer, "REQUIRED_ROLE_NOT_MEASUREMENT_EVIDENCE");
        };
        matched_roles.insert(role.clone());
        evidence_tiers.push((tier, receipt.evidence_class.clone()));
    }

    if let Some(review) = offer.receipts.get("INDEPENDENT_REVIEW") {
        matched_receipt_ids.insert(review.receipt_id.clone());
        matched_receipt_digests.insert(review.receipt_sha256.clone());
        if BLOCKING_DISPOSITIONS.contains(&review.disposition.as_str()) {
            return no_match(offer, "INDEPENDENT_REVIEW_BLOCKED");
        }
        if review.valid_until_us < request.decision_time_us {
            return no_match(offer, "INDEPENDENT_REVIEW_EXPIRED");
        }
        independently_reviewed =
            review.disposition == "QUALIFIED_FOR_DECLARED_SCOPE"
                && review.review_status == "INDEPENDENTLY_REVIEWED";
    }

    if request.require_independent_review && !independently_reviewed {
        declared_only = true;
    }
    if declared_only || evidence_tiers.len() != request.required_evidence_roles.len() {
        return MatchResult {
            profile_id: metadata.profile_id.clone(),
            provider_id: metadata.provider_id.clone(),
            disposition: "DECLARED_ONLY",
            reason: "DECLARATION_OR_EVIDENCE_TIER_INSUFFICIENT",
            bearer: metadata.bearer.clone(),
            matched_roles,
            matched_receipt_ids,
            matched_receipt_digests,
            weakest_evidence_class: evidence_tiers
                .iter()
                .min_by_key(|(tier, _)| *tier)
                .map(|(_, class)| class.clone()),
            independently_reviewed,
        };
    }

    let (weakest_tier, weakest_class) = evidence_tiers
        .iter()
        .min_by_key(|(tier, _)| *tier)
        .expect("non-empty required role set validated");
    let required_tier = evidence_tier(&request.minimum_evidence_tier)
        .expect("request minimum tier validated");
    if *weakest_tier < required_tier {
        return MatchResult {
            profile_id: metadata.profile_id.clone(),
            provider_id: metadata.provider_id.clone(),
            disposition: "DECLARED_ONLY",
            reason: "EVIDENCE_BELOW_REQUESTED_MINIMUM",
            bearer: metadata.bearer.clone(),
            matched_roles,
            matched_receipt_ids,
            matched_receipt_digests,
            weakest_evidence_class: Some(weakest_class.clone()),
            independently_reviewed,
        };
    }

    let disposition = match *weakest_tier {
        0 => "MATCH_SYNTHETIC_ONLY",
        1 => "MATCH_SIMULATED",
        2 => "MATCH_BENCH",
        3 => "MATCH_FIELD",
        _ => "NO_MATCH",
    };
    MatchResult {
        profile_id: metadata.profile_id.clone(),
        provider_id: metadata.provider_id.clone(),
        disposition,
        reason: "ALL_DECLARED_COMPATIBILITY_AND_EVIDENCE_PREDICATES_MATCH",
        bearer: metadata.bearer.clone(),
        matched_roles,
        matched_receipt_ids,
        matched_receipt_digests,
        weakest_evidence_class: Some(weakest_class.clone()),
        independently_reviewed,
    }
}

fn evaluate(offers: &[Offer], request: &Request) -> Evaluation {
    let mut results: Vec<MatchResult> = offers
        .iter()
        .map(|offer| match_offer(offer, request))
        .collect();
    results.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
    Evaluation {
        request_id: request.request_id.clone(),
        results,
    }
}

fn run(offers_path: &str, request_path: &str) -> Result<(), String> {
    let offers_text = fs::read_to_string(offers_path)
        .map_err(|error| format!("cannot read {offers_path}: {error}"))?;
    let request_text = fs::read_to_string(request_path)
        .map_err(|error| format!("cannot read {request_path}: {error}"))?;
    let offers = parse_offers(&offers_text).map_err(|errors| errors.join("\n"))?;
    let request = parse_request(&request_text).map_err(|errors| errors.join("\n"))?;
    let report = evaluate(&offers, &request);

    println!("request_id={}", report.request_id);
    println!("offer_count={}", report.results.len());
    for result in &report.results {
        println!(
            "profile={}|provider={}|bearer={}|disposition={}|reason={}|weakest_evidence={}|independently_reviewed={}|matched_roles={}|matched_receipt_ids={}|matched_receipt_digests={}|claim_ceiling={}",
            result.profile_id,
            result.provider_id,
            result.bearer,
            result.disposition,
            result.reason,
            result.weakest_evidence_class.as_deref().unwrap_or("NONE"),
            result.independently_reviewed,
            result.matched_roles.iter().cloned().collect::<Vec<_>>().join(";"),
            result.matched_receipt_ids.iter().cloned().collect::<Vec<_>>().join(";"),
            result.matched_receipt_digests.iter().cloned().collect::<Vec<_>>().join(";"),
            offers.iter().find(|offer| offer.metadata.profile_id == result.profile_id).map(|offer| offer.metadata.claim_ceiling.as_str()).unwrap_or("MISSING")
        );
    }
    println!("compatibility_is_not_authorization=true");
    println!("device_actuation=false");
    println!("power_enable=false");
    println!("flight_authority=false");
    println!("artifact_bytes_verified=false");
    println!("authority=NONE_DECLARATIVE_CAPABILITY_MATCHING_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: energy-service-capability-profile <offers.tsv> <receiver-request.tsv>");
        return ExitCode::from(2);
    }
    match run(&args[1], &args[2]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("REJECTED_ENERGY_SERVICE_PROFILE {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFERS: &str =
        include_str!("../../../research/energy-service-capability-profile-v0/synthetic-offers.tsv");
    const REQUEST: &str =
        include_str!("../../../research/energy-service-capability-profile-v0/docked-receiver-request.tsv");

    fn offers(text: &str) -> Vec<Offer> {
        parse_offers(text).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    fn request(text: &str) -> Request {
        parse_request(text).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    fn by_id<'a>(results: &'a [MatchResult], id: &str) -> &'a MatchResult {
        results.iter().find(|item| item.profile_id == id).expect("profile result")
    }

    #[test]
    fn contact_and_ipt_match_at_the_weakest_required_evidence_tier() {
        let report = evaluate(&offers(OFFERS), &request(REQUEST));
        assert_eq!(by_id(&report.results, "contact_pad_001").disposition, "MATCH_SIMULATED");
        assert_eq!(by_id(&report.results, "ipt_dock_001").disposition, "MATCH_SIMULATED");
        assert_eq!(
            by_id(&report.results, "contact_pad_001").weakest_evidence_class.as_deref(),
            Some("SIMULATED")
        );
    }

    #[test]
    fn optical_hover_offer_does_not_match_docked_receiver_request() {
        let report = evaluate(&offers(OFFERS), &request(REQUEST));
        assert_eq!(
            by_id(&report.results, "optical_hover_001").disposition,
            "NO_MATCH"
        );
        assert_eq!(
            by_id(&report.results, "optical_hover_001").reason,
            "BEARER_NOT_ALLOWED"
        );
    }

    #[test]
    fn incomplete_declared_profile_is_not_promoted_to_evidence_backed_match() {
        let cpt_request = REQUEST.replace("CONDUCTIVE;IPT", "CPT").replace("CONTACT;NEAR_FIELD", "NEAR_FIELD");
        let report = evaluate(&offers(OFFERS), &request(&cpt_request));
        assert_eq!(
            by_id(&report.results, "cpt_dock_001").disposition,
            "NO_MATCH"
        );
        assert_eq!(
            by_id(&report.results, "cpt_dock_001").reason,
            "MISSING_REQUIRED_EVIDENCE_ROLE"
        );
    }

    #[test]
    fn stale_evidence_cannot_match_current_receiver_demand() {
        let report = evaluate(&offers(OFFERS), &request(REQUEST));
        assert_eq!(
            by_id(&report.results, "stale_contact_001").disposition,
            "NO_MATCH"
        );
        assert_eq!(
            by_id(&report.results, "stale_contact_001").reason,
            "REQUIRED_EVIDENCE_EXPIRED"
        );
    }

    #[test]
    fn declared_only_evidence_does_not_meet_simulated_minimum() {
        let declared = OFFERS.replace("SIMULATED\tNOT_REVIEWED\tQUALIFIED_FOR_DECLARED_SCOPE", "DECLARED\tNOT_REVIEWED\tDECLARED_ONLY");
        let report = evaluate(&offers(&declared), &request(REQUEST));
        assert_eq!(
            by_id(&report.results, "contact_pad_001").disposition,
            "DECLARED_ONLY"
        );
    }

    #[test]
    fn explicit_independent_review_is_a_separate_requirement() {
        let request_text = REQUEST.replace("\tfalse\t100", "\ttrue\t100");
        let report = evaluate(&offers(OFFERS), &request(&request_text));
        assert_eq!(
            by_id(&report.results, "contact_pad_001").disposition,
            "DECLARED_ONLY"
        );
        assert!(!by_id(&report.results, "contact_pad_001").independently_reviewed);
    }


    #[test]
    fn independent_review_receipt_satisfies_review_requirement_without_changing_measurement_tier() {
        let first_contact = OFFERS.lines().nth(1).expect("contact physical receipt")
            .replace("PHYSICAL_ENVELOPE", "INDEPENDENT_REVIEW")
            .replace("contact_physical", "contact_review")
            .replace("2222222222222222222222222222222222222222222222222222222222222222", "8888888888888888888888888888888888888888888888888888888888888888")
            .replace("NOT_REVIEWED", "INDEPENDENTLY_REVIEWED");
        let reviewed_offers = format!("{OFFERS}{first_contact}\n");
        let request_text = REQUEST.replace("\tfalse\t100", "\ttrue\t100");
        let report = evaluate(&offers(&reviewed_offers), &request(&request_text));
        let contact = by_id(&report.results, "contact_pad_001");
        assert_eq!(contact.disposition, "MATCH_SIMULATED");
        assert!(contact.independently_reviewed);
        assert!(contact.matched_receipt_ids.contains("contact_review"));
        assert_eq!(contact.matched_receipt_ids.len(), 4);
        assert_eq!(contact.matched_receipt_digests.len(), 4);
    }

    #[test]
    fn receiver_geometry_and_frame_must_match_exactly() {
        let geometry = REQUEST.replace(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        );
        let report = evaluate(&offers(OFFERS), &request(&geometry));
        assert_eq!(
            by_id(&report.results, "contact_pad_001").reason,
            "RECEIVER_GEOMETRY_MISMATCH"
        );

        let frame = REQUEST.replace(
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        );
        let report = evaluate(&offers(OFFERS), &request(&frame));
        assert_eq!(
            by_id(&report.results, "contact_pad_001").reason,
            "COORDINATE_FRAME_MISMATCH"
        );
    }

    #[test]
    fn unknown_bearer_is_rejected_not_treated_as_wildcard() {
        let invalid = REQUEST.replace("CONDUCTIVE;IPT", "*");
        assert!(parse_request(&invalid).is_err());
    }

    #[test]
    fn unknown_motion_environment_and_session_values_are_rejected() {
        for invalid in [
            REQUEST.replace("DOCKED", "ANY"),
            REQUEST.replace("DRY_INDOOR", "ANY"),
            REQUEST.replace("\tDOCK\t", "\tANY\t"),
        ] {
            assert!(parse_request(&invalid).is_err());
        }
    }

    #[test]
    fn blocking_evidence_dispositions_never_match() {
        let blocked = OFFERS.replace(
            "contact_pose\t3333333333333333333333333333333333333333333333333333333333333333\tSIMULATED\tNOT_REVIEWED\tQUALIFIED_FOR_DECLARED_SCOPE",
            "contact_pose\t3333333333333333333333333333333333333333333333333333333333333333\tSIMULATED\tNOT_REVIEWED\tAMBIGUOUS",
        );
        let report = evaluate(&offers(&blocked), &request(REQUEST));
        assert_eq!(
            by_id(&report.results, "contact_pad_001").disposition,
            "NO_MATCH"
        );
        assert_eq!(
            by_id(&report.results, "contact_pad_001").reason,
            "BLOCKING_EVIDENCE_DISPOSITION"
        );
    }

    #[test]
    fn duplicate_evidence_role_and_receipt_id_are_rejected() {
        let duplicate_role = format!(
            "{OFFERS}{}",
            OFFERS.lines().nth(1).expect("first offer receipt")
        );
        assert!(parse_offers(&duplicate_role).is_err());
    }

    #[test]
    fn inconsistent_profile_metadata_is_rejected() {
        let mismatch = OFFERS.replacen("DRY_INDOOR;DRY_OUTDOOR", "WET", 1);
        assert!(parse_offers(&mismatch).is_err());
    }


    #[test]
    fn battery_exchange_offer_matches_exact_swap_service_request() {
        let swap_request = REQUEST
            .replace("CONDUCTIVE;IPT", "BATTERY_EXCHANGE")
            .replace("CONTACT;NEAR_FIELD", "CONTACT")
            .replace("\tDOCKED\t", "\tBATTERY_SWAP\t")
            .replace("DRY_INDOOR", "DRY_INDOOR")
            .replace("\tDOCK\t", "\tSWAP\t")
            .replace("PHYSICAL_ENVELOPE;OBSERVABILITY;SESSION_ADMISSION", "PHYSICAL_ENVELOPE")
            .replace("SIMULATED", "BENCH_MEASURED");
        let report = evaluate(&offers(OFFERS), &request(&swap_request));
        assert_eq!(
            by_id(&report.results, "battery_swap_001").disposition,
            "MATCH_BENCH"
        );
    }

    #[test]
    fn optical_and_rf_offers_match_only_their_declared_airborne_regimes() {
        let optical_request = REQUEST
            .replace("CONDUCTIVE;IPT", "OPTICAL")
            .replace("CONTACT;NEAR_FIELD", "FAR_FIELD")
            .replace("1111111111111111111111111111111111111111111111111111111111111111", "9999999999999999999999999999999999999999999999999999999999999999")
            .replace("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")
            .replace("\tDOCKED\t", "\tHOVER\t")
            .replace("DRY_INDOOR", "CLEAR_AIR")
            .replace("\tDOCK\t", "\tTRACK\t")
            .replace("PHYSICAL_ENVELOPE;OBSERVABILITY;SESSION_ADMISSION", "PHYSICAL_ENVELOPE")
            .replace("SIMULATED", "FIELD_MEASURED");
        let report = evaluate(&offers(OFFERS), &request(&optical_request));
        assert_eq!(
            by_id(&report.results, "optical_hover_001").disposition,
            "MATCH_FIELD"
        );

        let rf_request = optical_request
            .replace("OPTICAL", "RF_MICROWAVE")
            .replace("FIELD_MEASURED", "SIMULATED");
        let report = evaluate(&offers(OFFERS), &request(&rf_request));
        assert_eq!(
            by_id(&report.results, "rf_hover_001").disposition,
            "MATCH_SIMULATED"
        );
    }

    #[test]
    fn power_line_profile_is_visible_but_declared_only() {
        let line_request = REQUEST
            .replace("CONDUCTIVE;IPT", "POWER_LINE")
            .replace("CONTACT;NEAR_FIELD", "ENERGY_CARRIER")
            .replace("1111111111111111111111111111111111111111111111111111111111111111", "9999999999999999999999999999999999999999999999999999999999999999")
            .replace("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")
            .replace("\tDOCKED\t", "\tTRANSLATING\t")
            .replace("DRY_INDOOR", "POWERLINE_CORRIDOR")
            .replace("\tDOCK\t", "\tTRACK\t")
            .replace("PHYSICAL_ENVELOPE;OBSERVABILITY;SESSION_ADMISSION", "PHYSICAL_ENVELOPE");
        let report = evaluate(&offers(OFFERS), &request(&line_request));
        assert_eq!(
            by_id(&report.results, "powerline_corridor_001").disposition,
            "DECLARED_ONLY"
        );
    }

    #[test]
    fn row_permutation_preserves_compatibility_results() {
        let mut rows: Vec<&str> = OFFERS.lines().collect();
        let header = rows.remove(0);
        rows.reverse();
        let permuted = format!("{header}\n{}\n", rows.join("\n"));
        assert_eq!(
            evaluate(&offers(OFFERS), &request(REQUEST)),
            evaluate(&offers(&permuted), &request(REQUEST))
        );
    }
}
