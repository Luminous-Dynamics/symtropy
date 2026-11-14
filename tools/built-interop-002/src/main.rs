// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded IFC4 + IDS 1.0 adapter for a single literal IFCWALL.Name requirement.
//!
//! This is not a general IDS model checker. Any specification outside the
//! declared subset returns Unresolved rather than silently passing.

use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use openbim_core::Detected;
use openbim_ids::{Facet, IdsVersion, IfcVersion, Occurrence, Value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

const IFC4_WALL_NAME_ATTRIBUTE_INDEX: usize = 2;
const SUPPORTED_PROFILE: &str = "IFC4/IDS1.0/IFCWALL.Name/simpleValue-v0.1";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
enum Decision {
    SubsetConformant,
    Blocked,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
enum PropertyOutcome {
    Preserved,
    Changed,
    Missing,
    NotEvaluated,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct PropertyObservation {
    requirement_id: String,
    source_object_id: String,
    source_step_id: String,
    source_entity_type: String,
    source_attribute_index: usize,
    source_attribute_path: String,
    property_key: String,
    expected_value: String,
    observed_value: Option<String>,
    outcome: PropertyOutcome,
    diagnostic_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    supported_profile: String,
    decision: Decision,
    ifc_schema_token: Option<String>,
    ids_version_evidence: String,
    ifc_input_sha256: String,
    ids_input_sha256: String,
    parsed_entity_count: usize,
    applicable_wall_count: Option<usize>,
    observations: Vec<PropertyObservation>,
    diagnostics: Vec<String>,
    semantic_result_sha256: String,
}

#[derive(Serialize)]
struct SemanticDigest<'a> {
    supported_profile: &'a str,
    decision: Decision,
    ifc_schema_token: &'a Option<String>,
    ids_version_evidence: &'a str,
    parsed_entity_count: usize,
    applicable_wall_count: Option<usize>,
    observations: &'a [PropertyObservation],
    diagnostics: &'a [String],
}

fn is_valid_ifc_global_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 22
        && matches!(bytes[0], b'0'..=b'3')
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'$'))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn classify_ids_version(version: &Detected<IdsVersion>) -> (String, Option<Decision>) {
    match version {
        Detected::Declared(IdsVersion::Ids1_0) => ("Declared:IDS1.0".to_string(), None),
        Detected::Inferred(IdsVersion::Ids1_0) => {
            ("Inferred:IDS1.0".to_string(), Some(Decision::Unresolved))
        }
        Detected::Conflict { declared, observed } => (
            format!("Conflict:declared={declared:?}:observed={observed:?}"),
            Some(Decision::Blocked),
        ),
        Detected::Declared(other) => (format!("Declared:{other:?}"), Some(Decision::Blocked)),
        Detected::Inferred(other) => (format!("Inferred:{other:?}"), Some(Decision::Unresolved)),
    }
}

struct UnsupportedContext {
    schema_token: Option<String>,
    ids_version_evidence: String,
    parsed_entity_count: usize,
}

fn unsupported_report(
    input_bytes: (&[u8], &[u8]),
    context: UnsupportedContext,
    code: &str,
    message: &str,
    decision: Decision,
) -> Report {
    let (ifc_bytes, ids_bytes) = input_bytes;
    let schema_token = context.schema_token;
    let ids_version_evidence = context.ids_version_evidence;
    let parsed_entity_count = context.parsed_entity_count;
    let diagnostics = vec![format!("{code}:{message}")];
    let semantic = SemanticDigest {
        supported_profile: SUPPORTED_PROFILE,
        decision,
        ifc_schema_token: &schema_token,
        ids_version_evidence: &ids_version_evidence,
        parsed_entity_count,
        applicable_wall_count: None,
        observations: &[],
        diagnostics: &diagnostics,
    };
    let bytes = serde_json::to_vec(&semantic).expect("semantic report serializes");
    Report {
        schema_version: "0.1.0".to_string(),
        benchmark_id: "BUILT-INTEROP-002-ifc-ids-v0.1".to_string(),
        supported_profile: SUPPORTED_PROFILE.to_string(),
        decision,
        ifc_schema_token: schema_token,
        ids_version_evidence,
        ifc_input_sha256: sha256_hex(ifc_bytes),
        ids_input_sha256: sha256_hex(ids_bytes),
        parsed_entity_count,
        applicable_wall_count: None,
        observations: Vec::new(),
        diagnostics,
        semantic_result_sha256: sha256_hex(&bytes),
    }
}

fn evaluate(ifc_bytes: &[u8], ids_bytes: &[u8]) -> Result<Report, String> {
    let model: Model = StepCodec
        .read_bytes(ifc_bytes)
        .map_err(|error| format!("IFC_STEP_PARSE_FAILED:{error}"))?;
    let ids =
        openbim_ids::from_slice(ids_bytes).map_err(|error| format!("IDS_PARSE_FAILED:{error}"))?;

    let schema_token = model.header().schema_token().map(str::to_string);
    let (ids_version_evidence, version_decision) = classify_ids_version(&ids.version);
    if !model.is_complete() {
        return Ok(unsupported_report(
            (ifc_bytes, ids_bytes),
            UnsupportedContext {
                schema_token,
                ids_version_evidence,
                parsed_entity_count: model.len(),
            },
            "IFC_RECOVERY_DIAGNOSTICS",
            "strict parser returned a model with recovery diagnostics",
            Decision::Unresolved,
        ));
    }
    if schema_token.as_deref() != Some("IFC4") {
        return Ok(unsupported_report(
            (ifc_bytes, ids_bytes),
            UnsupportedContext {
                schema_token,
                ids_version_evidence,
                parsed_entity_count: model.len(),
            },
            "IFC_SCHEMA_PROFILE_MISMATCH",
            "v0.1 requires an exact IFC4 header token",
            Decision::Blocked,
        ));
    }
    if let Some(decision) = version_decision {
        return Ok(unsupported_report(
            (ifc_bytes, ids_bytes),
            UnsupportedContext {
                schema_token,
                ids_version_evidence,
                parsed_entity_count: model.len(),
            },
            "IDS_VERSION_NOT_EXPLICITLY_CONFIRMED",
            "v0.1 requires IDS 1.0 declared and consistent with document shape",
            decision,
        ));
    }

    if ids.specifications.len() != 1 {
        return Ok(unsupported_report(
            (ifc_bytes, ids_bytes),
            UnsupportedContext {
                schema_token,
                ids_version_evidence,
                parsed_entity_count: model.len(),
            },
            "IDS_SPECIFICATION_COUNT_UNSUPPORTED",
            "v0.1 supports exactly one IDS specification",
            Decision::Unresolved,
        ));
    }

    let specification = &ids.specifications[0];
    if specification.ifc_versions.as_slice() != [IfcVersion::Ifc4] {
        return Ok(unsupported_report(
            (ifc_bytes, ids_bytes),
            UnsupportedContext {
                schema_token,
                ids_version_evidence,
                parsed_entity_count: model.len(),
            },
            "IDS_IFC_VERSION_PROFILE_MISMATCH",
            "the single IDS specification must declare IFC4 only",
            Decision::Blocked,
        ));
    }

    let applicability_is_supported = match specification.applicability.facets.as_slice() {
        [Facet::Entity(entity)] => {
            entity.name.as_simple() == Some("IFCWALL") && entity.predefined_type.is_none()
        }
        _ => false,
    };
    if !applicability_is_supported
        || specification.applicability.min_occurs == 0
        || specification.applicability.max_occurs.is_some()
    {
        return Ok(unsupported_report(
            (ifc_bytes, ids_bytes),
            UnsupportedContext {
                schema_token,
                ids_version_evidence,
                parsed_entity_count: model.len(),
            },
            "IDS_APPLICABILITY_UNSUPPORTED",
            "v0.1 supports required, unbounded IFCWALL entity applicability only",
            Decision::Unresolved,
        ));
    }

    let requirements = match specification.requirements.as_ref() {
        Some(requirements) if requirements.facets.len() == 1 => requirements,
        _ => {
            return Ok(unsupported_report(
                (ifc_bytes, ids_bytes),
                UnsupportedContext {
                    schema_token,
                    ids_version_evidence,
                    parsed_entity_count: model.len(),
                },
                "IDS_REQUIREMENT_SHAPE_UNSUPPORTED",
                "v0.1 supports exactly one requirement facet",
                Decision::Unresolved,
            ));
        }
    };

    let (attribute_name, expected_value) = match &requirements.facets[0] {
        requirement if requirement.occurrence == Occurrence::Required => match &requirement.facet {
            Facet::Attribute(attribute) if attribute.name.as_simple() == Some("Name") => {
                match attribute.value.as_ref() {
                    Some(Value::Simple(value)) => ("Name", value.clone()),
                    _ => {
                        return Ok(unsupported_report(
                            (ifc_bytes, ids_bytes),
                            UnsupportedContext {
                                schema_token,
                                ids_version_evidence,
                                parsed_entity_count: model.len(),
                            },
                            "IDS_ATTRIBUTE_VALUE_UNSUPPORTED",
                            "v0.1 supports a literal simpleValue attribute requirement",
                            Decision::Unresolved,
                        ));
                    }
                }
            }
            _ => {
                return Ok(unsupported_report(
                    (ifc_bytes, ids_bytes),
                    UnsupportedContext {
                        schema_token,
                        ids_version_evidence,
                        parsed_entity_count: model.len(),
                    },
                    "IDS_REQUIREMENT_FACET_UNSUPPORTED",
                    "v0.1 supports only a required IFC attribute named Name",
                    Decision::Unresolved,
                ));
            }
        },
        _ => {
            return Ok(unsupported_report(
                (ifc_bytes, ids_bytes),
                UnsupportedContext {
                    schema_token,
                    ids_version_evidence,
                    parsed_entity_count: model.len(),
                },
                "IDS_REQUIREMENT_CARDINALITY_UNSUPPORTED",
                "v0.1 supports required cardinality only",
                Decision::Unresolved,
            ));
        }
    };

    let walls: Vec<_> = model.of_type("IFCWALL").collect();
    if walls.is_empty() {
        return Ok(unsupported_report(
            (ifc_bytes, ids_bytes),
            UnsupportedContext {
                schema_token,
                ids_version_evidence,
                parsed_entity_count: model.len(),
            },
            "NO_APPLICABLE_WALL",
            "IDS applicability requires at least one IFCWALL",
            Decision::Blocked,
        ));
    }

    let mut observations = Vec::new();
    let mut diagnostics = Vec::new();
    let mut decision = Decision::SubsetConformant;
    let mut global_id_counts = BTreeMap::<String, usize>::new();
    for (_, wall) in &walls {
        if let Some(id) = wall.text(0).filter(|value| !value.trim().is_empty()) {
            *global_id_counts.entry(id.to_string()).or_insert(0) += 1;
        }
    }
    let requirement_id = format!("{}::attribute::{attribute_name}", specification.name);

    for (entity_id, wall) in &walls {
        let step_id = format!("{entity_id:?}");
        let global_id = wall
            .text(0)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);
        let stable_id = global_id
            .clone()
            .unwrap_or_else(|| format!("STEP:{step_id}"));
        let identity_diagnostic = match global_id.as_deref() {
            None => Some("GLOBAL_ID_MISSING"),
            Some(id) if !is_valid_ifc_global_id(id) => Some("GLOBAL_ID_INVALID"),
            Some(id) if global_id_counts.get(id).copied().unwrap_or(0) > 1 => {
                Some("DUPLICATE_GLOBAL_ID")
            }
            Some(_) => None,
        };
        let observed_name = wall
            .text(IFC4_WALL_NAME_ATTRIBUTE_INDEX)
            .map(str::to_string);
        let (outcome, diagnostic_code, observed_value) = if let Some(code) = identity_diagnostic {
            decision = Decision::Blocked;
            diagnostics.push(format!("{code}:{requirement_id}:{step_id}:{stable_id}"));
            (
                PropertyOutcome::NotEvaluated,
                Some(code.to_string()),
                observed_name,
            )
        } else if wall.attributes.len() != 9 {
            decision = Decision::Blocked;
            diagnostics.push(format!(
                "IFCWALL_ATTRIBUTE_ARITY_MISMATCH:{requirement_id}:{stable_id}"
            ));
            (
                PropertyOutcome::NotEvaluated,
                Some("IFCWALL_ATTRIBUTE_ARITY_MISMATCH".to_string()),
                None,
            )
        } else {
            let value = wall
                .text(IFC4_WALL_NAME_ATTRIBUTE_INDEX)
                .map(str::to_string);
            let (outcome, diagnostic_code) = match value.as_deref() {
                None => {
                    decision = Decision::Blocked;
                    diagnostics.push(format!(
                        "MISSING_TARGET_PROPERTY:{requirement_id}:{stable_id}"
                    ));
                    (
                        PropertyOutcome::Missing,
                        Some("MISSING_TARGET_PROPERTY".to_string()),
                    )
                }
                Some(value) if value != expected_value => {
                    decision = Decision::Blocked;
                    diagnostics.push(format!(
                        "CHANGED_TARGET_PROPERTY:{requirement_id}:{stable_id}"
                    ));
                    (
                        PropertyOutcome::Changed,
                        Some("CHANGED_TARGET_PROPERTY".to_string()),
                    )
                }
                Some(_) => (PropertyOutcome::Preserved, None),
            };
            (outcome, diagnostic_code, value)
        };

        observations.push(PropertyObservation {
            requirement_id: requirement_id.clone(),
            source_object_id: stable_id,
            source_step_id: step_id,
            source_entity_type: wall.type_name.to_string(),
            source_attribute_index: IFC4_WALL_NAME_ATTRIBUTE_INDEX,
            source_attribute_path: format!("{}.Name", wall.type_name),
            property_key: attribute_name.to_string(),
            expected_value: expected_value.clone(),
            observed_value,
            outcome,
            diagnostic_code,
        });
    }

    observations.sort_by(|a, b| {
        a.source_object_id
            .cmp(&b.source_object_id)
            .then_with(|| a.source_step_id.cmp(&b.source_step_id))
    });
    diagnostics.sort();
    diagnostics.dedup();

    let semantic = SemanticDigest {
        supported_profile: SUPPORTED_PROFILE,
        decision,
        ifc_schema_token: &schema_token,
        ids_version_evidence: &ids_version_evidence,
        parsed_entity_count: model.len(),
        applicable_wall_count: Some(walls.len()),
        observations: &observations,
        diagnostics: &diagnostics,
    };
    let semantic_bytes = serde_json::to_vec(&semantic).expect("semantic report serializes");
    Ok(Report {
        schema_version: "0.1.0".to_string(),
        benchmark_id: "BUILT-INTEROP-002-ifc-ids-v0.1".to_string(),
        supported_profile: SUPPORTED_PROFILE.to_string(),
        decision,
        ifc_schema_token: schema_token,
        ids_version_evidence,
        ifc_input_sha256: sha256_hex(ifc_bytes),
        ids_input_sha256: sha256_hex(ids_bytes),
        parsed_entity_count: model.len(),
        applicable_wall_count: Some(walls.len()),
        observations,
        diagnostics,
        semantic_result_sha256: sha256_hex(&semantic_bytes),
    })
}

fn usage() -> &'static str {
    "Usage: built-interop-ifc-ids [panel.ifc] [panel.ids] [--output report.json]\n\
     Supports only a declared IFC4/IDS 1.0 IFCWALL.Name literal requirement."
}

fn main() {
    let mut args = env::args().skip(1);
    let mut inputs = Vec::new();
    let mut output_path: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{}", usage());
                return;
            }
            "--output" | "-o" => {
                let Some(path) = args.next() else {
                    eprintln!("--output requires a path");
                    process::exit(2);
                };
                output_path = Some(PathBuf::from(path));
            }
            _ if arg.starts_with('-') => {
                eprintln!("Unknown option: {arg}\n{}", usage());
                process::exit(2);
            }
            _ => inputs.push(PathBuf::from(arg)),
        }
    }
    if inputs.len() > 2 {
        eprintln!("At most two input paths are supported.\n{}", usage());
        process::exit(2);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ifc_path = inputs
        .first()
        .cloned()
        .unwrap_or_else(|| root.join("fixtures/panel.ifc"));
    let ids_path = inputs
        .get(1)
        .cloned()
        .unwrap_or_else(|| root.join("fixtures/panel.ids"));

    let ifc_bytes = fs::read(&ifc_path).unwrap_or_else(|error| {
        eprintln!("Cannot read {}: {error}", ifc_path.display());
        process::exit(2);
    });
    let ids_bytes = fs::read(&ids_path).unwrap_or_else(|error| {
        eprintln!("Cannot read {}: {error}", ids_path.display());
        process::exit(2);
    });
    let report = evaluate(&ifc_bytes, &ids_bytes).unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(2);
    });
    let json = serde_json::to_vec_pretty(&report).expect("report serialization should not fail");
    if let Some(path) = output_path
        && let Err(error) = fs::write(&path, &json)
    {
        eprintln!("Cannot write {}: {error}", path.display());
        process::exit(2);
    }
    println!("{}", String::from_utf8_lossy(&json));
    if report.decision != Decision::SubsetConformant {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (Vec<u8>, Vec<u8>) {
        (
            include_bytes!("../fixtures/panel.ifc").to_vec(),
            include_bytes!("../fixtures/panel.ids").to_vec(),
        )
    }

    #[test]
    fn exact_ifc4_wall_name_requirement_is_conformant() {
        let (ifc, ids) = fixtures();
        let report = evaluate(&ifc, &ids).expect("fixtures parse");
        assert_eq!(report.decision, Decision::SubsetConformant);
        assert_eq!(report.ifc_schema_token.as_deref(), Some("IFC4"));
        assert_eq!(report.ids_version_evidence, "Declared:IDS1.0");
        assert_eq!(report.applicable_wall_count, Some(1));
        assert_eq!(report.observations.len(), 1);
        assert_eq!(report.observations[0].expected_value, "Panel 001");
        assert_eq!(report.observations[0].source_attribute_index, 2);
        assert_eq!(report.observations[0].source_attribute_path, "IFCWALL.Name");
        assert_eq!(
            report.observations[0].observed_value.as_deref(),
            Some("Panel 001")
        );
        assert_eq!(report.observations[0].outcome, PropertyOutcome::Preserved);
    }

    #[test]
    fn wrong_ifc_value_blocks_instead_of_silently_passing() {
        let (ifc, ids) = fixtures();
        let wrong = String::from_utf8(ifc)
            .expect("fixture is UTF-8")
            .replace("'Panel 001'", "'Panel 002'")
            .into_bytes();
        let report = evaluate(&wrong, &ids).expect("mutated IFC still parses");
        assert_eq!(report.decision, Decision::Blocked);
        assert_eq!(report.observations[0].outcome, PropertyOutcome::Changed);
        assert_eq!(
            report.observations[0].diagnostic_code.as_deref(),
            Some("CHANGED_TARGET_PROPERTY")
        );
    }

    #[test]
    fn missing_ifc_value_blocks_and_reports_missing() {
        let (ifc, ids) = fixtures();
        let missing = String::from_utf8(ifc)
            .expect("fixture is UTF-8")
            .replace("'Panel 001'", "$")
            .into_bytes();
        let report = evaluate(&missing, &ids).expect("missing attribute fixture parses");
        assert_eq!(report.decision, Decision::Blocked);
        assert_eq!(report.observations[0].outcome, PropertyOutcome::Missing);
        assert_eq!(
            report.observations[0].diagnostic_code.as_deref(),
            Some("MISSING_TARGET_PROPERTY")
        );
    }

    #[test]
    fn wrong_ifcwall_attribute_arity_blocks_instead_of_guessing_slots() {
        let (ifc, ids) = fixtures();
        let shortened = String::from_utf8(ifc)
            .expect("fixture is UTF-8")
            .replace("'Panel 001',$,$,$,$,$,.NOTDEFINED.", "'Panel 001',$,$,$");
        let report = evaluate(shortened.as_bytes(), &ids)
            .expect("syntactically valid shortened record reaches profile guard");
        assert_eq!(report.decision, Decision::Blocked);
        assert_eq!(
            report.observations[0].diagnostic_code.as_deref(),
            Some("IFCWALL_ATTRIBUTE_ARITY_MISMATCH")
        );
        assert_eq!(
            report.observations[0].outcome,
            PropertyOutcome::NotEvaluated
        );
    }

    #[test]
    fn missing_global_id_blocks_even_when_name_matches() {
        let (ifc, ids) = fixtures();
        let missing_id = String::from_utf8(ifc)
            .expect("fixture is UTF-8")
            .replace("'0000000000000000000000'", "$")
            .into_bytes();
        let report = evaluate(&missing_id, &ids).expect("synthetic IFC still parses");
        assert_eq!(report.decision, Decision::Blocked);
        assert_eq!(
            report.observations[0].outcome,
            PropertyOutcome::NotEvaluated
        );
        assert_eq!(
            report.observations[0].diagnostic_code.as_deref(),
            Some("GLOBAL_ID_MISSING")
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.starts_with("GLOBAL_ID_MISSING:"))
        );
    }

    #[test]
    fn malformed_global_id_blocks_even_when_name_matches() {
        let (ifc, ids) = fixtures();
        let invalid_ids = [
            "not-a-guid",
            "4000000000000000000000",
            "000000000000000000000!",
        ];
        for invalid_id in invalid_ids {
            let source = String::from_utf8(ifc.clone()).expect("fixture is UTF-8");
            let mutated = source.replace("'0000000000000000000000'", &format!("'{invalid_id}'"));
            let report = evaluate(mutated.as_bytes(), &ids).expect("synthetic IFC still parses");
            assert_eq!(report.decision, Decision::Blocked, "ID={invalid_id}");
            assert_eq!(
                report.observations[0].outcome,
                PropertyOutcome::NotEvaluated,
                "ID={invalid_id}"
            );
            assert_eq!(
                report.observations[0].diagnostic_code.as_deref(),
                Some("GLOBAL_ID_INVALID"),
                "ID={invalid_id}"
            );
        }
    }

    #[test]
    fn duplicate_global_id_blocks_and_step_ids_remain_distinguishable() {
        let (ifc, ids) = fixtures();
        let second_wall =
            "#2=IFCWALL('0000000000000000000000',$,'Panel 001',$,$,$,$,$,.NOTDEFINED.);\n";
        let duplicate = String::from_utf8(ifc).expect("fixture is UTF-8").replace(
            "ENDSEC;\nEND-ISO-10303-21;",
            &format!("{second_wall}ENDSEC;\nEND-ISO-10303-21;"),
        );
        let report = evaluate(duplicate.as_bytes(), &ids).expect("duplicate-identity IFC parses");
        assert_eq!(report.decision, Decision::Blocked);
        assert_eq!(report.applicable_wall_count, Some(2));
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.starts_with("DUPLICATE_GLOBAL_ID:"))
        );
        assert_ne!(
            report.observations[0].source_step_id,
            report.observations[1].source_step_id
        );
        assert_eq!(
            report
                .observations
                .iter()
                .filter(|observation| observation.diagnostic_code.as_deref()
                    == Some("DUPLICATE_GLOBAL_ID"))
                .count(),
            2,
            "every member of a duplicate-identity set must be marked ambiguous"
        );
        assert!(report.observations.iter().all(|observation| {
            observation.diagnostic_code.as_deref() == Some("DUPLICATE_GLOBAL_ID")
                && observation.outcome == PropertyOutcome::NotEvaluated
        }));
    }

    #[test]
    fn schema_mismatch_blocks() {
        let (ifc, ids) = fixtures();
        let other_schema = String::from_utf8(ifc)
            .expect("fixture is UTF-8")
            .replace("FILE_SCHEMA(('IFC4'));", "FILE_SCHEMA(('IFC4X3_ADD2'));")
            .into_bytes();
        let report = evaluate(&other_schema, &ids).expect("schema-mutated IFC parses");
        assert_eq!(report.decision, Decision::Blocked);
        assert!(report.diagnostics[0].starts_with("IFC_SCHEMA_PROFILE_MISMATCH"));
    }

    #[test]
    fn unsupported_ids_facet_is_unresolved_not_passed() {
        let (ifc, ids) = fixtures();
        let xml = String::from_utf8(ids).expect("IDS fixture is UTF-8");
        let with_property_requirement = xml.replace(
            "</requirements>",
            concat!(
                "<property cardinality=\"required\">",
                "<propertySet><simpleValue>Pset_WallCommon</simpleValue></propertySet>",
                "<baseName><simpleValue>FireRating</simpleValue></baseName>",
                "<value><simpleValue>EI30</simpleValue></value>",
                "</property></requirements>"
            ),
        );
        let report = evaluate(&ifc, with_property_requirement.as_bytes())
            .expect("additional property facet is valid IDS");
        assert_eq!(report.decision, Decision::Unresolved);
        assert!(report.diagnostics[0].starts_with("IDS_REQUIREMENT_SHAPE_UNSUPPORTED"));
    }

    #[test]
    fn absent_applicable_wall_blocks() {
        let (ifc, ids) = fixtures();
        let no_wall = String::from_utf8(ifc)
            .expect("fixture is UTF-8")
            .replace("IFCWALL", "IFCBEAM");
        let report =
            evaluate(no_wall.as_bytes(), &ids).expect("modified STEP is syntactically valid");
        assert_eq!(report.decision, Decision::Blocked);
        assert!(report.diagnostics[0].starts_with("NO_APPLICABLE_WALL"));
    }

    #[test]
    fn malformed_step_is_rejected_at_parse_boundary() {
        let (ifc, ids) = fixtures();
        let malformed = String::from_utf8(ifc)
            .expect("fixture is UTF-8")
            .replace("#1=IFCWALL", "#1=IFCWALL(");
        assert!(evaluate(malformed.as_bytes(), &ids).is_err());
    }

    #[test]
    fn row_order_only_changes_raw_hash_not_semantic_result() {
        let (ifc, ids) = fixtures();
        let second_wall =
            "#2=IFCWALL('1111111111111111111111',$,'Panel 001',$,$,$,$,$,.NOTDEFINED.);\n";
        let with_two = String::from_utf8(ifc.clone())
            .expect("fixture is UTF-8")
            .replace(
                "ENDSEC;\nEND-ISO-10303-21;",
                &format!("{second_wall}ENDSEC;\nEND-ISO-10303-21;"),
            );
        let reversed = with_two.replace(
            "#1=IFCWALL('0000000000000000000000',$,'Panel 001',$,$,$,$,$,.NOTDEFINED.);\n#2=IFCWALL('1111111111111111111111',$,'Panel 001',$,$,$,$,$,.NOTDEFINED.);",
            "#2=IFCWALL('1111111111111111111111',$,'Panel 001',$,$,$,$,$,.NOTDEFINED.);\n#1=IFCWALL('0000000000000000000000',$,'Panel 001',$,$,$,$,$,.NOTDEFINED.);"
        );
        let a = evaluate(with_two.as_bytes(), &ids).expect("two-wall fixture parses");
        let b = evaluate(reversed.as_bytes(), &ids).expect("reordered fixture parses");
        assert_eq!(a.decision, b.decision);
        assert_eq!(a.semantic_result_sha256, b.semantic_result_sha256);
        assert_ne!(a.ifc_input_sha256, b.ifc_input_sha256);
    }
}
