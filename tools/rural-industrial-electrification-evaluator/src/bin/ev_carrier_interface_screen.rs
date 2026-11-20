// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const SCHEMA_VERSION: &str = "0.1.0";
const MAX_PORTS: usize = 256;
const MAX_PAIRS: usize = 256;
const CLAIM_CEILING: &str = "Interface profile/evidence admission only. Does not measure or qualify physical fit, structural loads, HV safety, fire/thermal, EMC, functional safety, cybersecurity, roadworthiness, airworthiness or production release.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PortSide {
    Vehicle,
    Module,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum InterfaceFamily {
    MechanicalMount,
    HighVoltageDc,
    LowVoltagePower,
    ThermalFluid,
    DataControl,
    CargoRetention,
    Environmental,
    ServiceAccess,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    IndependentlyQualified,
    MeasuredUnreviewed,
    Synthetic,
    Unknown,
    Stale,
    Conflicted,
    Revoked,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    configuration_id: String,
    revision: String,
    configuration_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PortEvidence {
    classification: EvidenceClass,
    source_reference: String,
    scoped_configuration_digest: String,
    profile_digest: String,
    report_digest: Option<String>,
    valid_until: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InterfacePort {
    port_id: String,
    side: PortSide,
    family: InterfaceFamily,
    revision: String,
    mating_profile_id: String,
    controlled_profile_reference: String,
    profile_digest: String,
    evidence: PortEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PairEvidence {
    classification: EvidenceClass,
    source_reference: String,
    vehicle_configuration_digest: String,
    module_configuration_digest: String,
    vehicle_profile_digest: String,
    module_profile_digest: String,
    compatibility_profile_digest: String,
    report_digest: Option<String>,
    valid_until: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InterfacePair {
    pair_id: String,
    vehicle_port_id: String,
    module_port_id: String,
    compatibility_profile_id: String,
    compatibility_profile_reference: String,
    compatibility_profile_digest: String,
    qualification_evidence: PairEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InterfaceControlInput {
    schema_version: String,
    programme_id: String,
    evaluated_at_utc: String,
    claim_ceiling: String,
    vehicle_configuration: Configuration,
    module_configuration: Configuration,
    ports: Vec<InterfacePort>,
    interface_pairs: Vec<InterfacePair>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Disposition {
    Compatible,
    Incompatible,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct PairResult {
    pair_id: String,
    vehicle_port_id: String,
    module_port_id: String,
    vehicle_revision: String,
    module_revision: String,
    vehicle_mating_profile_id: String,
    module_mating_profile_id: String,
    vehicle_profile_digest: String,
    module_profile_digest: String,
    compatibility_profile_id: String,
    compatibility_profile_reference: String,
    compatibility_profile_digest: String,
    family: Option<InterfaceFamily>,
    disposition: Disposition,
    reason_codes: Vec<String>,
    evidence_references: Vec<String>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct InterfaceControlReport {
    schema_version: &'static str,
    programme_id: String,
    evaluated_at_utc: String,
    input_vehicle_configuration_digest: String,
    input_module_configuration_digest: String,
    pair_count: usize,
    compatible_count: usize,
    incompatible_count: usize,
    indeterminate_count: usize,
    results: Vec<PairResult>,
    production_release_ready: bool,
    physical_effect_authorized: bool,
    commands_emitted: bool,
    claim_ceiling: &'static str,
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
    {
        return false;
    }
    let parse = |range: std::ops::Range<usize>| {
        value.get(range).and_then(|part| part.parse::<u32>().ok())
    };
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        parse(0..4), parse(5..7), parse(8..10), parse(11..13), parse(14..16), parse(17..19),
    ) else {
        return false;
    };
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 59
}

fn validate_input(input: &InterfaceControlInput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {SCHEMA_VERSION}, got {}",
            input.schema_version
        ));
    }
    if input.programme_id != "EV-CARRIER-ICD-001" {
        errors.push("programme_id: unsupported programme identifier".into());
    }
    if input.claim_ceiling.trim().is_empty() {
        errors.push("claim_ceiling: must not be empty".into());
    }
    if !valid_utc_timestamp(&input.evaluated_at_utc) {
        errors.push("evaluated_at_utc: expected UTC timestamp YYYY-MM-DDTHH:MM:SSZ".into());
    }
    for (name, configuration) in [
        ("vehicle_configuration", &input.vehicle_configuration),
        ("module_configuration", &input.module_configuration),
    ] {
        if configuration.configuration_id.trim().is_empty()
            || configuration.revision.trim().is_empty()
        {
            errors.push(format!("{name}: configuration ID and revision are required"));
        }
        if !valid_digest(&configuration.configuration_digest) {
            errors.push(format!("{name}.configuration_digest: invalid SHA-256 digest"));
        }
    }
    if input.ports.is_empty() || input.ports.len() > MAX_PORTS {
        errors.push(format!("ports: expected 1..={MAX_PORTS}"));
    }
    if input.interface_pairs.is_empty() || input.interface_pairs.len() > MAX_PAIRS {
        errors.push(format!("interface_pairs: expected 1..={MAX_PAIRS}"));
    }

    let mut port_ids = BTreeSet::new();
    for (index, port) in input.ports.iter().enumerate() {
        let prefix = format!("ports[{index}]");
        if port.port_id.trim().is_empty() || !port_ids.insert(port.port_id.as_str()) {
            errors.push(format!("{prefix}.port_id: nonempty unique identifier required"));
        }
        for (field, value) in [
            ("revision", port.revision.as_str()),
            ("mating_profile_id", port.mating_profile_id.as_str()),
            ("controlled_profile_reference", port.controlled_profile_reference.as_str()),
            ("evidence.source_reference", port.evidence.source_reference.as_str()),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("{prefix}.{field}: must not be empty"));
            }
        }
        if !valid_digest(&port.profile_digest) {
            errors.push(format!("{prefix}.profile_digest: invalid SHA-256 digest"));
        }
        if !valid_digest(&port.evidence.scoped_configuration_digest) {
            errors.push(format!("{prefix}.evidence.scoped_configuration_digest: invalid SHA-256 digest"));
        }
        if !valid_digest(&port.evidence.profile_digest) {
            errors.push(format!("{prefix}.evidence.profile_digest: invalid SHA-256 digest"));
        }
        if let Some(digest) = &port.evidence.report_digest {
            if !valid_digest(digest) {
                errors.push(format!("{prefix}.evidence.report_digest: invalid SHA-256 digest"));
            }
        }
        if let Some(valid_until) = &port.evidence.valid_until {
            if !valid_utc_timestamp(valid_until) {
                errors.push(format!("{prefix}.evidence.valid_until: expected UTC timestamp"));
            }
        }
    }

    let mut pair_ids = BTreeSet::new();
    for (index, pair) in input.interface_pairs.iter().enumerate() {
        let prefix = format!("interface_pairs[{index}]");
        if pair.pair_id.trim().is_empty() || !pair_ids.insert(pair.pair_id.as_str()) {
            errors.push(format!("{prefix}.pair_id: nonempty unique identifier required"));
        }
        for (field, value) in [
            ("vehicle_port_id", pair.vehicle_port_id.as_str()),
            ("module_port_id", pair.module_port_id.as_str()),
            ("compatibility_profile_id", pair.compatibility_profile_id.as_str()),
            ("compatibility_profile_reference", pair.compatibility_profile_reference.as_str()),
            ("qualification_evidence.source_reference", pair.qualification_evidence.source_reference.as_str()),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("{prefix}.{field}: must not be empty"));
            }
        }
        if !valid_digest(&pair.compatibility_profile_digest) {
            errors.push(format!("{prefix}.compatibility_profile_digest: invalid SHA-256 digest"));
        }
        for (field, value) in [
            ("qualification_evidence.vehicle_configuration_digest", pair.qualification_evidence.vehicle_configuration_digest.as_str()),
            ("qualification_evidence.module_configuration_digest", pair.qualification_evidence.module_configuration_digest.as_str()),
            ("qualification_evidence.vehicle_profile_digest", pair.qualification_evidence.vehicle_profile_digest.as_str()),
            ("qualification_evidence.module_profile_digest", pair.qualification_evidence.module_profile_digest.as_str()),
            ("qualification_evidence.compatibility_profile_digest", pair.qualification_evidence.compatibility_profile_digest.as_str()),
        ] {
            if !valid_digest(value) {
                errors.push(format!("{prefix}.{field}: invalid SHA-256 digest"));
            }
        }
        if let Some(digest) = &pair.qualification_evidence.report_digest {
            if !valid_digest(digest) {
                errors.push(format!("{prefix}.qualification_evidence.report_digest: invalid SHA-256 digest"));
            }
        }
        if let Some(valid_until) = &pair.qualification_evidence.valid_until {
            if !valid_utc_timestamp(valid_until) {
                errors.push(format!("{prefix}.qualification_evidence.valid_until: expected UTC timestamp"));
            }
        }
        if !port_ids.contains(pair.vehicle_port_id.as_str()) {
            errors.push(format!("{prefix}.vehicle_port_id: undeclared port"));
        }
        if !port_ids.contains(pair.module_port_id.as_str()) {
            errors.push(format!("{prefix}.module_port_id: undeclared port"));
        }
        if pair.vehicle_port_id == pair.module_port_id {
            errors.push(format!("{prefix}: vehicle and module ports must be distinct"));
        }
        if let Some(port) = input.ports.iter().find(|port| port.port_id == pair.vehicle_port_id) {
            if port.side != PortSide::Vehicle {
                errors.push(format!("{prefix}.vehicle_port_id: port side must be vehicle"));
            }
        }
        if let Some(port) = input.ports.iter().find(|port| port.port_id == pair.module_port_id) {
            if port.side != PortSide::Module {
                errors.push(format!("{prefix}.module_port_id: port side must be module"));
            }
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn evidence_is_current_and_qualified(
    classification: EvidenceClass,
    report_digest: Option<&str>,
    valid_until: Option<&str>,
    evaluated_at: &str,
) -> bool {
    classification == EvidenceClass::IndependentlyQualified
        && report_digest.is_some_and(valid_digest)
        && valid_until.is_some_and(|expiry| valid_utc_timestamp(expiry) && expiry > evaluated_at)
}

fn assess_pair(
    input: &InterfaceControlInput,
    pair: &InterfacePair,
) -> Result<PairResult, String> {
    let vehicle = input.ports.iter()
        .find(|port| port.port_id == pair.vehicle_port_id)
        .ok_or_else(|| format!("pair {} vehicle port disappeared", pair.pair_id))?;
    let module = input.ports.iter()
        .find(|port| port.port_id == pair.module_port_id)
        .ok_or_else(|| format!("pair {} module port disappeared", pair.pair_id))?;

    let mut reasons = Vec::new();
    let disposition = if vehicle.family != module.family {
        reasons.push("FAMILY_MISMATCH".to_string());
        Disposition::Incompatible
    } else if vehicle.mating_profile_id != module.mating_profile_id {
        reasons.push("MATING_PROFILE_MISMATCH".to_string());
        Disposition::Incompatible
    } else if vehicle.revision != module.revision {
        reasons.push("REVISION_UNRECONCILED".to_string());
        Disposition::Indeterminate
    } else {
        let vehicle_digest = &input.vehicle_configuration.configuration_digest;
        let module_digest = &input.module_configuration.configuration_digest;
        let mut evidence_ready = true;

        for (name, port, expected_scope) in [
            ("vehicle", vehicle, vehicle_digest.as_str()),
            ("module", module, module_digest.as_str()),
        ] {
            if port.evidence.scoped_configuration_digest != expected_scope {
                reasons.push(format!("{name}:EVIDENCE_SCOPE_MISMATCH"));
                evidence_ready = false;
            }
            if port.evidence.profile_digest != port.profile_digest {
                reasons.push(format!("{name}:PROFILE_EVIDENCE_DIGEST_MISMATCH"));
                evidence_ready = false;
            }
            if !evidence_is_current_and_qualified(
                port.evidence.classification,
                port.evidence.report_digest.as_deref(),
                port.evidence.valid_until.as_deref(),
                &input.evaluated_at_utc,
            ) {
                reasons.push(format!("{name}:EVIDENCE_UNQUALIFIED_OR_NOT_CURRENT"));
                evidence_ready = false;
            }
        }

        let pair_evidence = &pair.qualification_evidence;
        if pair_evidence.vehicle_configuration_digest != *vehicle_digest
            || pair_evidence.module_configuration_digest != *module_digest
        {
            reasons.push("PAIR_EVIDENCE_SCOPE_MISMATCH".into());
            evidence_ready = false;
        }
        if pair_evidence.vehicle_profile_digest != vehicle.profile_digest
            || pair_evidence.module_profile_digest != module.profile_digest
            || pair_evidence.compatibility_profile_digest != pair.compatibility_profile_digest
        {
            reasons.push("PAIR_PROFILE_EVIDENCE_DIGEST_MISMATCH".into());
            evidence_ready = false;
        }
        if !evidence_is_current_and_qualified(
            pair_evidence.classification,
            pair_evidence.report_digest.as_deref(),
            pair_evidence.valid_until.as_deref(),
            &input.evaluated_at_utc,
        ) {
            reasons.push("PAIR_EVIDENCE_UNQUALIFIED_OR_NOT_CURRENT".into());
            evidence_ready = false;
        }

        if evidence_ready {
            reasons.push("PROFILE_AND_EVIDENCE_ADMITTED".into());
            Disposition::Compatible
        } else {
            Disposition::Indeterminate
        }
    };

    reasons.sort();
    let mut evidence_references = vec![
        vehicle.evidence.source_reference.clone(),
        module.evidence.source_reference.clone(),
        pair.qualification_evidence.source_reference.clone(),
    ];
    evidence_references.sort();
    evidence_references.dedup();

    Ok(PairResult {
        pair_id: pair.pair_id.clone(),
        vehicle_port_id: vehicle.port_id.clone(),
        module_port_id: module.port_id.clone(),
        vehicle_revision: vehicle.revision.clone(),
        module_revision: module.revision.clone(),
        vehicle_mating_profile_id: vehicle.mating_profile_id.clone(),
        module_mating_profile_id: module.mating_profile_id.clone(),
        vehicle_profile_digest: vehicle.profile_digest.clone(),
        module_profile_digest: module.profile_digest.clone(),
        compatibility_profile_id: pair.compatibility_profile_id.clone(),
        compatibility_profile_reference: pair.compatibility_profile_reference.clone(),
        compatibility_profile_digest: pair.compatibility_profile_digest.clone(),
        family: (vehicle.family == module.family).then_some(vehicle.family),
        disposition,
        reason_codes: reasons,
        evidence_references,
        claim_ceiling: CLAIM_CEILING,
    })
}

fn evaluate(input: &InterfaceControlInput) -> Result<InterfaceControlReport, Vec<String>> {
    validate_input(input)?;
    let mut pairs = input.interface_pairs.clone();
    pairs.sort_by(|left, right| left.pair_id.cmp(&right.pair_id));
    let mut results = Vec::with_capacity(pairs.len());
    for pair in &pairs {
        results.push(assess_pair(input, pair).map_err(|error| vec![error])?);
    }
    let compatible_count = results.iter().filter(|result| result.disposition == Disposition::Compatible).count();
    let incompatible_count = results.iter().filter(|result| result.disposition == Disposition::Incompatible).count();
    let indeterminate_count = results.iter().filter(|result| result.disposition == Disposition::Indeterminate).count();
    Ok(InterfaceControlReport {
        schema_version: SCHEMA_VERSION,
        programme_id: input.programme_id.clone(),
        evaluated_at_utc: input.evaluated_at_utc.clone(),
        input_vehicle_configuration_digest: input.vehicle_configuration.configuration_digest.clone(),
        input_module_configuration_digest: input.module_configuration.configuration_digest.clone(),
        pair_count: results.len(),
        compatible_count,
        incompatible_count,
        indeterminate_count,
        results,
        production_release_ready: false,
        physical_effect_authorized: false,
        commands_emitted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <interface-control.json>", program.to_string_lossy());
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one interface-control input path");
        return ExitCode::from(2);
    }
    let path = path.to_string_lossy();
    let raw = match fs::read_to_string(path.as_ref()) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read interface-control input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: InterfaceControlInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid interface-control input JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate(&input) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("interface-control input rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot serialize interface-control report: {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../fixtures/ev_carrier_interface_examples_v0_1.json");

    fn fixture() -> InterfaceControlInput {
        serde_json::from_str(FIXTURE).expect("interface fixture must deserialize")
    }

    #[test]
    fn fixture_keeps_profile_mismatch_incompatible_and_unqualified_ports_indeterminate() {
        let report = evaluate(&fixture()).expect("fixture must evaluate");
        assert_eq!(report.pair_count, 3);
        assert_eq!(report.compatible_count, 0);
        assert_eq!(report.incompatible_count, 1);
        assert_eq!(report.indeterminate_count, 2);
        assert!(!report.production_release_ready);
        assert!(!report.physical_effect_authorized);
        assert!(!report.commands_emitted);
    }

    #[test]
    fn mismatched_mating_profiles_never_become_compatible() {
        let report = evaluate(&fixture()).expect("fixture must evaluate");
        let mechanical = report.results.iter()
            .find(|item| item.pair_id == "vehicle-pod-mechanical-mount")
            .expect("mechanical result");
        assert_eq!(mechanical.disposition, Disposition::Incompatible);
        assert!(mechanical.reason_codes.contains(&"MATING_PROFILE_MISMATCH".into()));
    }

    #[test]
    fn synthetic_or_unknown_evidence_never_grants_compatibility() {
        let report = evaluate(&fixture()).expect("fixture must evaluate");
        assert!(report.results.iter().filter(|item| item.disposition == Disposition::Indeterminate).count() >= 2);
    }

    #[test]
    fn expired_independent_evidence_remains_indeterminate() {
        let mut input = fixture();
        let vehicle_digest = input.vehicle_configuration.configuration_digest.clone();
        let module_digest = input.module_configuration.configuration_digest.clone();
        for port in &mut input.ports {
            port.evidence.classification = EvidenceClass::IndependentlyQualified;
            port.evidence.report_digest = Some(port.profile_digest.clone());
            port.evidence.valid_until = Some("2026-11-19T00:00:00Z".into());
            port.evidence.scoped_configuration_digest = if port.side == PortSide::Vehicle {
                vehicle_digest.clone()
            } else {
                module_digest.clone()
            };
            port.evidence.profile_digest = port.profile_digest.clone();
        }
        for pair in &mut input.interface_pairs {
            pair.qualification_evidence.classification = EvidenceClass::IndependentlyQualified;
            pair.qualification_evidence.report_digest = Some(pair.compatibility_profile_digest.clone());
            pair.qualification_evidence.valid_until = Some("2026-11-19T00:00:00Z".into());
        }
        let report = evaluate(&input).expect("expired evidence is a classification, not malformed input");
        assert_eq!(report.compatible_count, 0);
        assert!(report.indeterminate_count >= 2);
    }

    #[test]
    fn exact_current_evidence_can_admit_a_profile_match_but_never_release_product() {
        let mut input = fixture();
        let vehicle_digest = input.vehicle_configuration.configuration_digest.clone();
        let module_digest = input.module_configuration.configuration_digest.clone();
        for port in &mut input.ports {
            if port.mating_profile_id == "HV-CONNECTOR-PROFILE-UNSELECTED" {
                port.evidence.classification = EvidenceClass::IndependentlyQualified;
                port.evidence.report_digest = Some(port.profile_digest.clone());
                port.evidence.valid_until = Some("2027-01-01T00:00:00Z".into());
                port.evidence.scoped_configuration_digest = if port.side == PortSide::Vehicle {
                    vehicle_digest.clone()
                } else {
                    module_digest.clone()
                };
            }
        }
        let (vehicle_port_id, module_port_id, compatibility_profile_digest) = input
            .interface_pairs
            .iter()
            .find(|pair| pair.pair_id == "vehicle-pod-hv-power")
            .map(|pair| (
                pair.vehicle_port_id.clone(),
                pair.module_port_id.clone(),
                pair.compatibility_profile_digest.clone(),
            ))
            .expect("HV pair");
        let vehicle_profile = input.ports.iter()
            .find(|port| port.port_id == vehicle_port_id)
            .expect("vehicle port").profile_digest.clone();
        let module_profile = input.ports.iter()
            .find(|port| port.port_id == module_port_id)
            .expect("module port").profile_digest.clone();
        let pair = input.interface_pairs.iter_mut()
            .find(|pair| pair.pair_id == "vehicle-pod-hv-power")
            .expect("HV pair");
        pair.qualification_evidence.classification = EvidenceClass::IndependentlyQualified;
        pair.qualification_evidence.report_digest = Some(compatibility_profile_digest.clone());
        pair.qualification_evidence.valid_until = Some("2027-01-01T00:00:00Z".into());
        pair.qualification_evidence.vehicle_configuration_digest = vehicle_digest;
        pair.qualification_evidence.module_configuration_digest = module_digest;
        pair.qualification_evidence.vehicle_profile_digest = vehicle_profile;
        pair.qualification_evidence.module_profile_digest = module_profile;
        pair.qualification_evidence.compatibility_profile_digest = compatibility_profile_digest;
        let report = evaluate(&input).expect("complete current evidence");
        let hv = report.results.iter().find(|item| item.pair_id == "vehicle-pod-hv-power").expect("HV result");
        assert_eq!(hv.disposition, Disposition::Compatible);
        assert!(!report.production_release_ready);
        assert!(!report.physical_effect_authorized);
    }

    #[test]
    fn qualification_receipt_for_an_old_profile_digest_cannot_admit_current_port() {
        let mut input = fixture();
        let vehicle_port = input.ports.iter_mut()
            .find(|port| port.port_id == "vehicle-hv-output-01")
            .expect("vehicle HV port");
        vehicle_port.profile_digest = "sha256:abababababababababababababababababababababababababababababababab".into();
        let report = evaluate(&input).expect("stale profile receipt is an indeterminate result");
        let hv = report.results.iter().find(|item| item.pair_id == "vehicle-pod-hv-power").expect("HV result");
        assert_eq!(hv.disposition, Disposition::Indeterminate);
        assert!(hv.reason_codes.iter().any(|reason| reason.contains("PROFILE_EVIDENCE_DIGEST_MISMATCH")));
    }

    #[test]
    fn pair_receipt_must_bind_both_port_profiles_and_pair_profile() {
        let mut input = fixture();
        input.interface_pairs[1].qualification_evidence.vehicle_profile_digest =
            "sha256:abababababababababababababababababababababababababababababababab".into();
        let report = evaluate(&input).expect("mismatched evidence digest is indeterminate");
        let hv = report.results.iter().find(|item| item.pair_id == "vehicle-pod-hv-power").expect("HV result");
        assert_eq!(hv.disposition, Disposition::Indeterminate);
        assert!(hv.reason_codes.iter().any(|reason| reason == "PAIR_PROFILE_EVIDENCE_DIGEST_MISMATCH"));
    }

    #[test]
    fn rejects_vehicle_and_module_port_side_inversion() {
        let mut input = fixture();
        input.ports[0].side = PortSide::Module;
        assert!(validate_input(&input).is_err());
    }

    #[test]
    fn rejects_duplicate_port_ids_and_dangling_pair_references() {
        let mut input = fixture();
        input.ports.push(input.ports[0].clone());
        assert!(validate_input(&input).is_err());
        let mut dangling = fixture();
        dangling.interface_pairs[0].vehicle_port_id = "missing-port".into();
        assert!(validate_input(&dangling).is_err());
    }

    #[test]
    fn rejects_unknown_fields_and_invalid_digests() {
        let mut value: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<InterfaceControlInput>(value).is_err());
        let mut invalid = fixture();
        invalid.ports[0].profile_digest = "not-a-digest".into();
        assert!(validate_input(&invalid).is_err());
    }

    #[test]
    fn output_order_is_deterministic_under_pair_input_permutation() {
        let mut first = fixture();
        let first_report = evaluate(&first).expect("first report");
        first.interface_pairs.reverse();
        let second_report = evaluate(&first).expect("permuted pair order");
        assert_eq!(first_report.results, second_report.results);
    }
}
