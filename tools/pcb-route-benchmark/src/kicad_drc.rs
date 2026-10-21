use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const DRC_POLICY_SCHEMA: &str = "kicad-drc-routing-policy-v0.1";
pub const DRC_RECEIPT_SCHEMA: &str = "kicad-drc-evidence-receipt-v0.1";
pub const KICAD_DRC_SCHEMA_SUFFIX: &str = "/drc.v1.json";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KicadDrcRoutingPolicy {
    pub schema: String,
    pub routing_violation_types: Vec<String>,
    pub nonrouting_violation_types: Vec<String>,
    pub blocking_severities: Vec<String>,
    pub block_unconnected_items: bool,
    pub block_schematic_parity: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KicadDrcReport {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub source: String,
    pub date: String,
    pub kicad_version: String,
    pub violations: Vec<KicadViolation>,
    pub unconnected_items: Vec<KicadViolation>,
    pub schematic_parity: Vec<KicadViolation>,
    pub coordinate_units: String,
    #[serde(default)]
    pub included_severities: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KicadViolation {
    #[serde(rename = "type")]
    pub violation_type: String,
    pub description: String,
    pub severity: String,
    pub items: Vec<KicadAffectedItem>,
    #[serde(default)]
    pub excluded: bool,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KicadAffectedItem {
    pub uuid: String,
    pub description: String,
    pub pos: KicadCoordinate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KicadCoordinate {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct KicadDrcEvidenceReceipt {
    pub schema: String,
    pub raw_report_sha256: String,
    pub policy_sha256: String,
    pub source: String,
    pub kicad_version: String,
    pub coordinate_units: String,
    pub violations_total: usize,
    pub violations_by_severity: BTreeMap<String, usize>,
    pub violations_by_type: BTreeMap<String, usize>,
    pub routing_blocking_violation_types: Vec<String>,
    pub nonrouting_blocking_violation_types: Vec<String>,
    pub unconnected_item_count: usize,
    pub schematic_parity_count: usize,
    pub route_drc_pass: bool,
    pub board_drc_clean: bool,
    pub authority: String,
    pub claim_ceiling: String,
}

/// Parse the bytes produced by KiCad's documented DRC JSON schema and derive
/// route-facing counts using a separately pinned policy. Router-provided
/// "success" flags are not inputs to this function.
pub fn evaluate_kicad_drc_report_bytes(
    report_bytes: &[u8],
    policy_bytes: &[u8],
    expected_source: &str,
    expected_kicad_version: &str,
) -> Result<KicadDrcEvidenceReceipt, String> {
    let report: KicadDrcReport = serde_json::from_slice(report_bytes)
        .map_err(|error| format!("KiCad DRC JSON is invalid or incomplete: {error}"))?;
    let policy: KicadDrcRoutingPolicy = serde_json::from_slice(policy_bytes)
        .map_err(|error| format!("KiCad DRC routing policy JSON invalid: {error}"))?;

    validate_policy(&policy)?;
    if !report.schema.ends_with(KICAD_DRC_SCHEMA_SUFFIX) {
        return Err(format!(
            "unexpected KiCad DRC report schema: {}",
            report.schema
        ));
    }
    if report.source.trim().is_empty() || report.source != expected_source {
        return Err(format!(
            "KiCad DRC source mismatch: expected {expected_source:?}, observed {:?}",
            report.source
        ));
    }
    if report.kicad_version != expected_kicad_version {
        return Err(format!(
            "KiCad DRC version mismatch: expected {expected_kicad_version:?}, observed {:?}",
            report.kicad_version
        ));
    }
    if report.date.trim().is_empty() || !report.date.contains('T') {
        return Err("KiCad DRC report date must be a non-empty date-time string".into());
    }
    if !matches!(report.coordinate_units.as_str(), "mm" | "mils" | "in") {
        return Err(format!(
            "unsupported KiCad DRC coordinate_units: {}",
            report.coordinate_units
        ));
    }
    for severity in &report.included_severities {
        if !matches!(severity.as_str(), "error" | "warning" | "exclusion") {
            return Err(format!(
                "unknown KiCad DRC included severity: {severity}"
            ));
        }
    }

    let routing_types: BTreeSet<&str> = policy
        .routing_violation_types
        .iter()
        .map(String::as_str)
        .collect();
    let nonrouting_types: BTreeSet<&str> = policy
        .nonrouting_violation_types
        .iter()
        .map(String::as_str)
        .collect();
    let blocking_severities: BTreeSet<&str> = policy
        .blocking_severities
        .iter()
        .map(String::as_str)
        .collect();

    let mut violations_by_severity = BTreeMap::new();
    let mut violations_by_type = BTreeMap::new();
    let mut routing_blocking_violation_types = Vec::new();
    let mut nonrouting_blocking_violation_types = Vec::new();

    for violation in &report.violations {
        validate_violation(violation)?;
        let kind = violation.violation_type.as_str();
        if !routing_types.contains(kind) && !nonrouting_types.contains(kind) {
            return Err(format!(
                "unclassified KiCad DRC violation type {kind:?}; update the pinned policy explicitly"
            ));
        }
        if routing_types.contains(kind) && nonrouting_types.contains(kind) {
            return Err(format!(
                "KiCad DRC type {kind:?} is classified as both routing and non-routing"
            ));
        }

        *violations_by_severity
            .entry(violation.severity.clone())
            .or_insert(0) += 1;
        *violations_by_type.entry(kind.to_string()).or_insert(0) += 1;

        if blocking_severities.contains(violation.severity.as_str()) {
            if routing_types.contains(kind) {
                routing_blocking_violation_types.push(kind.to_string());
            } else {
                nonrouting_blocking_violation_types.push(kind.to_string());
            }
        }
    }

    for violation in report
        .unconnected_items
        .iter()
        .chain(report.schematic_parity.iter())
    {
        validate_violation(violation)?;
    }

    routing_blocking_violation_types.sort();
    nonrouting_blocking_violation_types.sort();

    let route_drc_pass = routing_blocking_violation_types.is_empty()
        && (!policy.block_unconnected_items || report.unconnected_items.is_empty())
        && (!policy.block_schematic_parity || report.schematic_parity.is_empty());
    let board_drc_clean = report.violations.is_empty()
        && report.unconnected_items.is_empty()
        && report.schematic_parity.is_empty();

    Ok(KicadDrcEvidenceReceipt {
        schema: DRC_RECEIPT_SCHEMA.into(),
        raw_report_sha256: sha256_hex(report_bytes),
        policy_sha256: sha256_hex(policy_bytes),
        source: report.source,
        kicad_version: report.kicad_version,
        coordinate_units: report.coordinate_units,
        violations_total: report.violations.len(),
        violations_by_severity,
        violations_by_type,
        routing_blocking_violation_types,
        nonrouting_blocking_violation_types,
        unconnected_item_count: report.unconnected_items.len(),
        schematic_parity_count: report.schematic_parity.len(),
        route_drc_pass,
        board_drc_clean,
        authority: "native_kicad_drc_json_interpretation_under_pinned_policy_only_no_physical_qualification_or_release_authority".into(),
        claim_ceiling: "The receipt derives counts from the raw KiCad DRC JSON bytes under the policy digest. It does not prove that the chosen policy is sufficient, nor schematic intent, signal/power integrity, manufacturability, physical quality, electrical safety or production readiness.".into(),
    })
}

fn validate_policy(policy: &KicadDrcRoutingPolicy) -> Result<(), String> {
    if policy.schema != DRC_POLICY_SCHEMA {
        return Err(format!("unsupported KiCad DRC policy schema: {}", policy.schema));
    }
    if policy.routing_violation_types.is_empty() {
        return Err("policy routing_violation_types must not be empty".into());
    }
    if policy.blocking_severities.is_empty() {
        return Err("policy blocking_severities must not be empty".into());
    }
    let mut routing = BTreeSet::new();
    for kind in &policy.routing_violation_types {
        if !valid_rule_type(kind) || !routing.insert(kind.as_str()) {
            return Err(format!("invalid or duplicate routing DRC type: {kind:?}"));
        }
    }
    let mut nonrouting = BTreeSet::new();
    for kind in &policy.nonrouting_violation_types {
        if !valid_rule_type(kind) || !nonrouting.insert(kind.as_str()) {
            return Err(format!("invalid or duplicate non-routing DRC type: {kind:?}"));
        }
        if routing.contains(kind.as_str()) {
            return Err(format!("DRC type {kind:?} is present in both policy categories"));
        }
    }
    let mut severities = BTreeSet::new();
    for severity in &policy.blocking_severities {
        if !matches!(severity.as_str(), "error" | "warning")
            || !severities.insert(severity.as_str())
        {
            return Err(format!("invalid or duplicate blocking severity: {severity:?}"));
        }
    }
    Ok(())
}

fn validate_violation(violation: &KicadViolation) -> Result<(), String> {
    if !valid_rule_type(&violation.violation_type)
        || violation.description.trim().is_empty()
        || !matches!(violation.severity.as_str(), "error" | "warning")
        || violation.items.iter().any(|item| {
            item.uuid.trim().is_empty()
                || item.description.trim().is_empty()
                || !item.pos.x.is_finite()
                || !item.pos.y.is_finite()
        })
    {
        return Err(format!(
            "malformed KiCad DRC violation record of type {:?}",
            violation.violation_type
        ));
    }
    Ok(())
}

fn valid_rule_type(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(result, "{byte:02x}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: &str = r#"{
      "schema":"kicad-drc-routing-policy-v0.1",
      "routing_violation_types":["tracks_crossing","shorting_items","clearance"],
      "nonrouting_violation_types":["silk_over_copper","lib_footprint_mismatch"],
      "blocking_severities":["error"],
      "block_unconnected_items":true,
      "block_schematic_parity":true
    }"#;

    fn report(violations: &str, unconnected: &str, parity: &str) -> String {
        format!(
            r#"{{
              "$schema":"https://schemas.kicad.org/drc.v1.json",
              "source":"coupon.kicad_pcb",
              "date":"2026-10-21T10:00:00+00:00",
              "kicad_version":"9.0.9",
              "coordinate_units":"mm",
              "included_severities":["error","warning"],
              "violations":{violations},
              "unconnected_items":{unconnected},
              "schematic_parity":{parity},
              "ignored_checks":[]
            }}"#
        )
    }

    fn violation(kind: &str, severity: &str) -> String {
        format!(
            r#"[{{"type":"{kind}","description":"fixture {kind}","severity":"{severity}","items":[{{"uuid":"11111111-1111-4111-8111-111111111111","description":"track","pos":{{"x":1.0,"y":2.0}}}}]}}]"#
        )
    }

    #[test]
    fn clean_native_report_has_zero_findings_and_raw_byte_receipt() {
        let bytes = report("[]", "[]", "[]");
        let receipt = evaluate_kicad_drc_report_bytes(
            bytes.as_bytes(),
            POLICY.as_bytes(),
            "coupon.kicad_pcb",
            "9.0.9",
        )
        .unwrap();
        assert!(receipt.route_drc_pass);
        assert!(receipt.board_drc_clean);
        assert_eq!(receipt.violations_total, 0);
        assert_eq!(receipt.unconnected_item_count, 0);
        assert_eq!(receipt.schematic_parity_count, 0);
        assert_eq!(receipt.raw_report_sha256, sha256_hex(bytes.as_bytes()));
        assert_eq!(receipt.policy_sha256, sha256_hex(POLICY.as_bytes()));
    }

    #[test]
    fn derives_routing_and_nonrouting_types_and_does_not_hide_open_items() {
        let bytes = report(
            &format!("{}{}", violation("tracks_crossing", "error").trim_matches(&['[', ']'][..]), &violation("silk_over_copper", "warning")),
            &violation("unconnected_items", "error"),
            "[]",
        );
        // The fixture builder above is only for test construction; check a valid
        // serialized report structure explicitly with a stable JSON payload.
        let json = format!(
            r#"{{"$schema":"https://schemas.kicad.org/drc.v1.json","source":"coupon.kicad_pcb","date":"2026-10-21T10:00:00+00:00","kicad_version":"9.0.9","coordinate_units":"mm","included_severities":["error","warning"],"violations":[{{"type":"tracks_crossing","description":"crossing","severity":"error","items":[{{"uuid":"11111111-1111-4111-8111-111111111111","description":"track","pos":{{"x":1,"y":2}}}}]}},{{"type":"silk_over_copper","description":"silk","severity":"warning","items":[{{"uuid":"22222222-2222-4222-8222-222222222222","description":"text","pos":{{"x":3,"y":4}}}}]}}],"unconnected_items":[{{"type":"unconnected_items","description":"open net","severity":"error","items":[{{"uuid":"33333333-3333-4333-8333-333333333333","description":"pad","pos":{{"x":5,"y":6}}}}]}}],"schematic_parity":[]}}"#
        );
        let receipt = evaluate_kicad_drc_report_bytes(
            json.as_bytes(),
            POLICY.as_bytes(),
            "coupon.kicad_pcb",
            "9.0.9",
        )
        .unwrap();
        assert!(!receipt.route_drc_pass);
        assert!(!receipt.board_drc_clean);
        assert_eq!(receipt.routing_blocking_violation_types, vec!["tracks_crossing"]);
        assert_eq!(receipt.nonrouting_blocking_violation_types, Vec::<String>::new());
        assert_eq!(receipt.unconnected_item_count, 1);
    }

    #[test]
    fn rejects_unknown_types_source_mismatch_version_mismatch_and_invalid_json() {
        let unknown = report(&violation("future_unknown_rule", "error"), "[]", "[]");
        assert!(evaluate_kicad_drc_report_bytes(
            unknown.as_bytes(), POLICY.as_bytes(), "coupon.kicad_pcb", "9.0.9"
        ).unwrap_err().contains("unclassified"));

        let clean = report("[]", "[]", "[]");
        assert!(evaluate_kicad_drc_report_bytes(
            clean.as_bytes(), POLICY.as_bytes(), "wrong-board.kicad_pcb", "9.0.9"
        ).unwrap_err().contains("source mismatch"));
        assert!(evaluate_kicad_drc_report_bytes(
            clean.as_bytes(), POLICY.as_bytes(), "coupon.kicad_pcb", "9.0.8"
        ).unwrap_err().contains("version mismatch"));
        assert!(evaluate_kicad_drc_report_bytes(
            b"{broken", POLICY.as_bytes(), "coupon.kicad_pcb", "9.0.9"
        ).unwrap_err().contains("invalid or incomplete"));
    }
}
