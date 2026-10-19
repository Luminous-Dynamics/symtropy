// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conservative IFC4 structural semantic diff. This is not geometric,
//! complete EXPRESS, IDS, code-compliance or engineering qualification.

use ifc_model::guid::Guid;
use ifc_model::{Codec, Entity, EntityId, Model, Value as IfcValue};
use ifc_schema::attribute::{AggregateKind, Aggregation};
use ifc_schema::{Schema, ifc4};
use ifc_step::StepCodec;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

const PROFILE: &str = "IFC4/schema-directed-structural-diff-v0.1";
const BENCHMARK: &str = "BUILT-INTEROP-008";
const POLICY: &str = "IFC4-only;strict-step;root-identity=unique-valid-guid;nonroot-identity=step-id;SET-BAG unordered;LIST-ARRAY ordered;coverage-gaps=UNRESOLVED";
const TOOL_VERSION: &str = "ifc-model=0.3.1;ifc-step=0.5.0;ifc-schema=0.3.2;profile=v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Status {
    Equal,
    Different,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum IdentityBasis {
    GlobalId,
    StepIdOnly,
    AmbiguousGlobalId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ChangeKind {
    EntityAdded,
    EntityRemoved,
    EntityTypeChanged,
    AttributeChanged,
}

#[derive(Debug, Clone, Serialize)]
struct Artifact {
    locator: String,
    sha256: String,
    byte_length: usize,
}

#[derive(Debug, Clone, Serialize)]
struct Counts {
    source_entities: usize,
    target_entities: usize,
    entities_added: usize,
    entities_removed: usize,
    entity_types_changed: usize,
    attributes_changed: usize,
    identity_ambiguities: usize,
    schema_coverage_gaps: usize,
}

#[derive(Debug, Clone, Serialize)]
struct CoverageIssue {
    side: String,
    code: String,
    entity_type: Option<String>,
    step_id: Option<u64>,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct Change {
    kind: ChangeKind,
    entity_key: String,
    entity_type: String,
    path: String,
    attribute_name: Option<String>,
    before: Option<Value>,
    after: Option<Value>,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct IdentityCoverage {
    root_entities: usize,
    roots_with_unique_valid_global_id: usize,
    roots_with_ambiguous_identity: usize,
    non_root_entities_keyed_by_step_id: usize,
    source_unresolved_references: usize,
    target_unresolved_references: usize,
    fraction_of_roots_with_stable_identity: f64,
}

#[derive(Debug, Clone, Serialize)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    profile: String,
    tool_version: String,
    config_sha256: String,
    status: Status,
    source_artifact: Artifact,
    target_artifact: Artifact,
    source_schema_token: Option<String>,
    target_schema_token: Option<String>,
    identity_coverage: IdentityCoverage,
    counts: Counts,
    changes: Vec<Change>,
    coverage_issues: Vec<CoverageIssue>,
    source_semantic_sha256: Option<String>,
    target_semantic_sha256: Option<String>,
    semantic_result_sha256: String,
    decision_reason: String,
    limitations: Vec<String>,
}

#[derive(Debug, Clone)]
struct Identity {
    key: String,
    basis: IdentityBasis,
    guid: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct EntitySnapshot {
    key: String,
    identity_basis: IdentityBasis,
    entity_type: String,
    global_id: Option<String>,
    attributes: BTreeMap<String, Value>,
}

struct Snapshot {
    entities: BTreeMap<String, EntitySnapshot>,
    root_count: usize,
    stable_root_count: usize,
    nonroot_count: usize,
    ambiguous_count: usize,
    unresolved_refs: usize,
    issues: Vec<CoverageIssue>,
    semantic_sha256: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("{:x}", hash.finalize())
}

fn config_sha256() -> String {
    sha256_hex(format!("{TOOL_VERSION};{POLICY}").as_bytes())
}

fn artifact(locator: &str, bytes: &[u8]) -> Artifact {
    Artifact {
        locator: locator.to_owned(),
        sha256: sha256_hex(bytes),
        byte_length: bytes.len(),
    }
}

fn canonical_value(
    value: &IfcValue,
    ids: &BTreeMap<EntityId, Identity>,
    aggregation: &[Aggregation],
) -> Value {
    match value {
        IfcValue::Null => json!({"kind":"null"}),
        IfcValue::Derived => json!({"kind":"derived"}),
        IfcValue::Bool(v) => json!({"kind":"boolean","value":v}),
        IfcValue::LogicalUnknown => json!({"kind":"logical_unknown"}),
        IfcValue::Integer(v) => json!({"kind":"integer","value":v}),
        IfcValue::Real(v) => json!({"kind":"real","value":v.to_string()}),
        IfcValue::Text(v) => json!({"kind":"text","value":v.as_ref()}),
        IfcValue::Binary(v) => json!({"kind":"binary","value":v.as_ref()}),
        IfcValue::Enum(v) => json!({"kind":"enum","value":v.to_ascii_uppercase()}),
        IfcValue::Ref(id) => match ids.get(id) {
            Some(identity) => json!({
                "kind":"reference",
                "target_key":identity.key,
                "identity_basis":identity.basis,
                "global_id":identity.guid
            }),
            None => json!({"kind":"unresolved_reference","step_id":id.0}),
        },
        IfcValue::List(items) => {
            let rest = aggregation.get(1..).unwrap_or(&[]);
            let mut values: Vec<Value> = items
                .iter()
                .map(|v| canonical_value(v, ids, rest))
                .collect();
            let kind = aggregation.first().map(|a| format!("{:?}", a.kind));
            if aggregation
                .first()
                .is_some_and(|a| matches!(a.kind, AggregateKind::Set | AggregateKind::Bag))
            {
                values.sort_by_key(|v| serde_json::to_string(v).unwrap_or_default());
            }
            json!({"kind":"aggregate","aggregate_kind":kind,"values":values})
        }
        IfcValue::Typed { type_name, value } => json!({
            "kind":"typed","type_name":type_name.to_ascii_uppercase(),
            "value":canonical_value(value, ids, aggregation)
        }),
    }
}

fn global_id(entity: &Entity, schema: &Schema) -> Option<String> {
    let names = schema.attribute_names(&entity.type_name);
    let slot = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case("GlobalId"))?;
    entity.text(slot).map(str::to_owned)
}

fn is_valid_guid(value: &str) -> bool {
    Guid::parse(value).is_some()
}

fn identity_index(
    model: &Model,
    schema: &Schema,
    side: &str,
) -> (
    BTreeMap<EntityId, Identity>,
    usize,
    usize,
    usize,
    usize,
    Vec<CoverageIssue>,
) {
    let mut roots = HashMap::<EntityId, Option<String>>::new();
    let mut counts = HashMap::<String, usize>::new();
    let mut root_count = 0;
    let mut issues = Vec::new();

    for (id, entity) in model.iter() {
        if !schema.is_a(&entity.type_name, "IfcRoot") {
            continue;
        }
        root_count += 1;
        let value = global_id(entity, schema);
        match &value {
            Some(guid) if is_valid_guid(guid) => *counts.entry(guid.clone()).or_default() += 1,
            Some(guid) => issues.push(CoverageIssue {
                side: side.into(),
                code: "INVALID_GLOBAL_ID".into(),
                entity_type: Some(entity.type_name.to_string()),
                step_id: Some(id.0),
                detail: format!("GlobalId {guid} is not valid IFC compressed-GUID syntax."),
            }),
            None => issues.push(CoverageIssue {
                side: side.into(),
                code: "MISSING_GLOBAL_ID".into(),
                entity_type: Some(entity.type_name.to_string()),
                step_id: Some(id.0),
                detail: "IfcRoot has no readable GlobalId attribute.".into(),
            }),
        }
        roots.insert(id, value);
    }

    let mut map = BTreeMap::new();
    let mut stable = 0;
    let mut ambiguous = 0;
    for (id, entity) in model.iter() {
        let name = entity.type_name.to_string();
        if schema.is_a(&entity.type_name, "IfcRoot") {
            let candidate = roots.get(&id).cloned().flatten();
            match candidate.filter(|g| is_valid_guid(g)) {
                Some(guid) if counts.get(&guid).copied().unwrap_or(0) == 1 => {
                    stable += 1;
                    map.insert(
                        id,
                        Identity {
                            key: format!("global-id:{guid}"),
                            basis: IdentityBasis::GlobalId,
                            guid: Some(guid),
                        },
                    );
                }
                Some(guid) => {
                    ambiguous += 1;
                    issues.push(CoverageIssue {
                        side: side.into(),
                        code: "DUPLICATE_GLOBAL_ID".into(),
                        entity_type: Some(name.clone()),
                        step_id: Some(id.0),
                        detail: format!("GlobalId {guid} is not unique in this model."),
                    });
                    map.insert(
                        id,
                        Identity {
                            key: format!("ambiguous-root:{name}:#{}", id.0),
                            basis: IdentityBasis::AmbiguousGlobalId,
                            guid: Some(guid),
                        },
                    );
                }
                None => {
                    ambiguous += 1;
                    map.insert(
                        id,
                        Identity {
                            key: format!("ambiguous-root:{name}:#{}", id.0),
                            basis: IdentityBasis::AmbiguousGlobalId,
                            guid: None,
                        },
                    );
                }
            }
        } else {
            map.insert(
                id,
                Identity {
                    key: format!("step:{name}:#{}", id.0),
                    basis: IdentityBasis::StepIdOnly,
                    guid: None,
                },
            );
        }
    }
    let nonroot = model.len().saturating_sub(root_count);
    (map, root_count, stable, nonroot, ambiguous, issues)
}

fn count_unresolved_refs(value: &Value, count: &mut usize) {
    match value {
        Value::Object(m) => {
            if m.get("kind").and_then(Value::as_str) == Some("unresolved_reference") {
                *count += 1;
            }
            for v in m.values() {
                count_unresolved_refs(v, count);
            }
        }
        Value::Array(xs) => {
            for v in xs {
                count_unresolved_refs(v, count);
            }
        }
        _ => {}
    }
}

fn snapshot(model: &Model, schema: &Schema, side: &str) -> Snapshot {
    let (ids, root_count, stable, nonroot, ambiguous, mut issues) =
        identity_index(model, schema, side);
    let mut entities = BTreeMap::new();
    let mut unresolved_refs = 0;

    for (id, entity) in model.iter() {
        let ty = entity.type_name.to_string();
        let identity = ids.get(&id).cloned().unwrap_or_else(|| Identity {
            key: format!("missing-identity:{ty}:#{}", id.0),
            basis: IdentityBasis::AmbiguousGlobalId,
            guid: None,
        });
        let known = schema.entity(&ty).is_some();
        let names = schema.attribute_names(&ty);
        let definitions = schema.attributes(&ty);
        if !known {
            issues.push(CoverageIssue {
                side: side.into(),
                code: "UNKNOWN_ENTITY_DECLARATION".into(),
                entity_type: Some(ty.clone()),
                step_id: Some(id.0),
                detail: "Entity type is absent from the selected IFC4 schema table.".into(),
            });
        } else if schema.attribute_count(&ty) != entity.attributes.len() {
            issues.push(CoverageIssue {
                side: side.into(),
                code: "ATTRIBUTE_ARITY_MISMATCH".into(),
                entity_type: Some(ty.clone()),
                step_id: Some(id.0),
                detail: format!(
                    "Schema expects {} slots; input has {}.",
                    schema.attribute_count(&ty),
                    entity.attributes.len()
                ),
            });
        }

        let mut attrs = BTreeMap::new();
        for (slot, value) in entity.attributes.iter().enumerate() {
            let name = names
                .get(slot)
                .map(|s| (*s).to_owned())
                .unwrap_or_else(|| format!("@slot:{slot}"));
            let levels = definitions
                .get(slot)
                .map_or(&[][..], |a| a.aggregation.as_slice());
            let canonical = canonical_value(value, &ids, levels);
            count_unresolved_refs(&canonical, &mut unresolved_refs);
            attrs.insert(name, canonical);
        }
        entities.insert(
            identity.key.clone(),
            EntitySnapshot {
                key: identity.key,
                identity_basis: identity.basis,
                entity_type: ty,
                global_id: identity.guid,
                attributes: attrs,
            },
        );
    }

    let bytes = serde_json::to_vec(&entities).expect("snapshot map serializes");
    Snapshot {
        entities,
        root_count,
        stable_root_count: stable,
        nonroot_count: nonroot,
        ambiguous_count: ambiguous,
        unresolved_refs,
        issues,
        semantic_sha256: sha256_hex(&bytes),
    }
}

fn snapshot_value(s: &EntitySnapshot) -> Value {
    serde_json::to_value(s).expect("entity snapshot serializes")
}

fn compare_snapshots(source: &Snapshot, target: &Snapshot) -> (Vec<Change>, Counts) {
    let mut changes = Vec::new();
    let mut added = 0;
    let mut removed = 0;
    let mut types_changed = 0;
    let mut attrs_changed = 0;

    for (key, before) in &source.entities {
        match target.entities.get(key) {
            None => {
                removed += 1;
                changes.push(Change {
                    kind: ChangeKind::EntityRemoved,
                    entity_key: key.clone(),
                    entity_type: before.entity_type.clone(),
                    path: key.clone(),
                    attribute_name: None,
                    before: Some(snapshot_value(before)),
                    after: None,
                    reason: "No target entity matches this profile identity.".into(),
                });
            }
            Some(after) => {
                if before.entity_type != after.entity_type {
                    types_changed += 1;
                    changes.push(Change {
                        kind: ChangeKind::EntityTypeChanged,
                        entity_key: key.clone(),
                        entity_type: before.entity_type.clone(),
                        path: format!("{key}.$type"),
                        attribute_name: None,
                        before: Some(json!(before.entity_type)),
                        after: Some(json!(after.entity_type)),
                        reason: "A matched identity has a different IFC entity type.".into(),
                    });
                }
                let names: BTreeSet<&String> = before
                    .attributes
                    .keys()
                    .chain(after.attributes.keys())
                    .collect();
                for name in names {
                    let a = before.attributes.get(name);
                    let b = after.attributes.get(name);
                    if a != b {
                        attrs_changed += 1;
                        changes.push(Change {
                            kind: ChangeKind::AttributeChanged,
                            entity_key: key.clone(),
                            entity_type: after.entity_type.clone(),
                            path: format!("{key}.{name}"),
                            attribute_name: Some(name.clone()),
                            before: a.cloned(),
                            after: b.cloned(),
                            reason: "Canonical typed attribute differs between source and target."
                                .into(),
                        });
                    }
                }
            }
        }
    }
    for (key, after) in &target.entities {
        if !source.entities.contains_key(key) {
            added += 1;
            changes.push(Change {
                kind: ChangeKind::EntityAdded,
                entity_key: key.clone(),
                entity_type: after.entity_type.clone(),
                path: key.clone(),
                attribute_name: None,
                before: None,
                after: Some(snapshot_value(after)),
                reason: "Target entity has no matching source identity in this profile.".into(),
            });
        }
    }
    changes.sort_by_key(|c| {
        (
            c.entity_key.clone(),
            c.path.clone(),
            format!("{:?}", c.kind),
        )
    });
    let ambiguity_count = source.ambiguous_count + target.ambiguous_count;
    let coverage_count = source
        .issues
        .iter()
        .chain(target.issues.iter())
        .filter(|i| {
            matches!(
                i.code.as_str(),
                "UNKNOWN_ENTITY_DECLARATION" | "ATTRIBUTE_ARITY_MISMATCH"
            )
        })
        .count();
    let counts = Counts {
        source_entities: source.entities.len(),
        target_entities: target.entities.len(),
        entities_added: added,
        entities_removed: removed,
        entity_types_changed: types_changed,
        attributes_changed: attrs_changed,
        identity_ambiguities: ambiguity_count,
        schema_coverage_gaps: coverage_count,
    };
    (changes, counts)
}

fn limitations() -> Vec<String> {
    vec![
        "Only structural attributes in the pinned IFC4 schema table are compared; this is not full EXPRESS or IDS conformance.".into(),
        "IfcRoot entities use valid unique GlobalId keys; non-root entities use local STEP IDs, so renumbering may produce conservative false-positive differences.".into(),
        "References to entities without stable GlobalIds retain STEP-ID sensitivity; no graph-isomorphism claim is made.".into(),
        "Schema-defined SET/BAG order is normalized; LIST/ARRAY order is preserved. No unit conversion, tolerance policy or geometry equivalence is inferred.".into(),
        "A clean result is not building-code, engineering, life-safety, manufacturing, cost, carbon or construction qualification.".into(),
    ]
}

fn early_unresolved(
    source_bytes: &[u8],
    target_bytes: &[u8],
    source_path: &str,
    target_path: &str,
    source_token: Option<String>,
    target_token: Option<String>,
    reason: String,
) -> Report {
    let counts = Counts {
        source_entities: 0,
        target_entities: 0,
        entities_added: 0,
        entities_removed: 0,
        entity_types_changed: 0,
        attributes_changed: 0,
        identity_ambiguities: 0,
        schema_coverage_gaps: 0,
    };
    let coverage = IdentityCoverage {
        root_entities: 0,
        roots_with_unique_valid_global_id: 0,
        roots_with_ambiguous_identity: 0,
        non_root_entities_keyed_by_step_id: 0,
        source_unresolved_references: 0,
        target_unresolved_references: 0,
        fraction_of_roots_with_stable_identity: 0.0,
    };
    let payload = json!({"profile":PROFILE,"status":"UNRESOLVED","reason":reason,
        "source_schema_token":source_token,"target_schema_token":target_token});
    let digest = sha256_hex(&serde_json::to_vec(&payload).expect("payload serializes"));
    Report {
        schema_version: "0.1.0".into(),
        benchmark_id: BENCHMARK.into(),
        profile: PROFILE.into(),
        tool_version: TOOL_VERSION.into(),
        config_sha256: config_sha256(),
        status: Status::Unresolved,
        source_artifact: artifact(source_path, source_bytes),
        target_artifact: artifact(target_path, target_bytes),
        source_schema_token: source_token,
        target_schema_token: target_token,
        identity_coverage: coverage,
        counts,
        changes: Vec::new(),
        coverage_issues: vec![CoverageIssue {
            side: "both".into(),
            code: "PRECONDITION_FAILED".into(),
            entity_type: None,
            step_id: None,
            detail: reason.clone(),
        }],
        source_semantic_sha256: None,
        target_semantic_sha256: None,
        semantic_result_sha256: digest,
        decision_reason: reason,
        limitations: limitations(),
    }
}

fn compare_bytes(
    source_bytes: &[u8],
    target_bytes: &[u8],
    source_path: &str,
    target_path: &str,
) -> Report {
    let schema = ifc4();
    let source_model = match StepCodec.read_bytes(source_bytes) {
        Ok(m) => m,
        Err(e) => {
            return early_unresolved(
                source_bytes,
                target_bytes,
                source_path,
                target_path,
                None,
                None,
                format!("SOURCE_STEP_PARSE_FAILED:{e}"),
            );
        }
    };
    let target_model = match StepCodec.read_bytes(target_bytes) {
        Ok(m) => m,
        Err(e) => {
            return early_unresolved(
                source_bytes,
                target_bytes,
                source_path,
                target_path,
                source_model.header().schema_token().map(str::to_owned),
                None,
                format!("TARGET_STEP_PARSE_FAILED:{e}"),
            );
        }
    };
    let source_token = source_model.header().schema_token().map(str::to_owned);
    let target_token = target_model.header().schema_token().map(str::to_owned);
    if !source_model.is_complete() || !target_model.is_complete() {
        return early_unresolved(
            source_bytes,
            target_bytes,
            source_path,
            target_path,
            source_token,
            target_token,
            format!(
                "INCOMPLETE_STEP_MODEL:source_diagnostics={};target_diagnostics={}",
                source_model.diagnostics().len(),
                target_model.diagnostics().len()
            ),
        );
    }
    if source_token.as_deref() != Some("IFC4") || target_token.as_deref() != Some("IFC4") {
        return early_unresolved(
            source_bytes,
            target_bytes,
            source_path,
            target_path,
            source_token,
            target_token,
            "SCHEMA_PROFILE_MISMATCH_OR_UNSUPPORTED_SCHEMA: IFC4 is required for both inputs"
                .into(),
        );
    }

    let source = snapshot(&source_model, schema, "source");
    let target = snapshot(&target_model, schema, "target");
    let mut issues = source.issues.clone();
    issues.extend(target.issues.clone());
    issues.sort_by(|a, b| {
        (&a.side, &a.code, &a.entity_type, &a.step_id, &a.detail).cmp(&(
            &b.side,
            &b.code,
            &b.entity_type,
            &b.step_id,
            &b.detail,
        ))
    });
    let (changes, counts) = compare_snapshots(&source, &target);
    let root_total = source.root_count + target.root_count;
    let stable_roots = source.stable_root_count + target.stable_root_count;
    let ambiguous = source.ambiguous_count + target.ambiguous_count;
    let identity_coverage = IdentityCoverage {
        root_entities: root_total,
        roots_with_unique_valid_global_id: stable_roots,
        roots_with_ambiguous_identity: ambiguous,
        non_root_entities_keyed_by_step_id: source.nonroot_count + target.nonroot_count,
        source_unresolved_references: source.unresolved_refs,
        target_unresolved_references: target.unresolved_refs,
        fraction_of_roots_with_stable_identity: if root_total == 0 {
            1.0
        } else {
            stable_roots as f64 / root_total as f64
        },
    };
    let unresolved = !issues.is_empty() || source.unresolved_refs > 0 || target.unresolved_refs > 0;
    let status = if unresolved {
        Status::Unresolved
    } else if changes.is_empty() {
        Status::Equal
    } else {
        Status::Different
    };
    let reason = match status {
        Status::Equal => "Source and target canonical structural snapshots are equal in the supported IFC4 profile.",
        Status::Different => "One or more canonical entities or attributes differ in the supported IFC4 profile.",
        Status::Unresolved => "Comparison is incomplete due to identity, schema-coverage or reference uncertainty."
    }.to_owned();
    let payload = json!({"profile":PROFILE,"status":status,"source_semantic_sha256":source.semantic_sha256,
        "target_semantic_sha256":target.semantic_sha256,"identity_coverage":identity_coverage,
        "counts":counts,"changes":changes,"coverage_issues":issues});
    let digest = sha256_hex(&serde_json::to_vec(&payload).expect("semantic result serializes"));
    Report {
        schema_version: "0.1.0".into(),
        benchmark_id: BENCHMARK.into(),
        profile: PROFILE.into(),
        tool_version: TOOL_VERSION.into(),
        config_sha256: config_sha256(),
        status,
        source_artifact: artifact(source_path, source_bytes),
        target_artifact: artifact(target_path, target_bytes),
        source_schema_token: source_token,
        target_schema_token: target_token,
        identity_coverage,
        counts,
        changes,
        coverage_issues: issues,
        source_semantic_sha256: Some(source.semantic_sha256),
        target_semantic_sha256: Some(target.semantic_sha256),
        semantic_result_sha256: digest,
        decision_reason: reason,
        limitations: limitations(),
    }
}

fn usage() -> &'static str {
    "Usage: built-interop-semantic-diff --source-ifc PATH --target-ifc PATH [--output PATH]\nExit codes: 0=EQUAL, 1=DIFFERENT, 2=UNRESOLVED, 64=usage/I-O error."
}
fn run() -> Result<i32, String> {
    let mut args = env::args().skip(1);
    let first = args.next();
    if matches!(first.as_deref(), Some("--help" | "-h")) {
        println!("{}", usage());
        return Ok(0);
    }
    let mut args = first.into_iter().chain(args);
    let mut source = None;
    let mut target = None;
    let mut output = None;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--source-ifc" | "--target-ifc" | "--output" => {
                let value = args.next().ok_or_else(|| format!("MISSING_VALUE:{flag}"))?;
                let slot = match flag.as_str() {
                    "--source-ifc" => &mut source,
                    "--target-ifc" => &mut target,
                    _ => &mut output,
                };
                if slot.replace(PathBuf::from(value)).is_some() {
                    return Err(format!("DUPLICATE_ARGUMENT:{flag}"));
                }
            }
            _ => return Err(format!("UNKNOWN_ARGUMENT:{flag}")),
        }
    }
    let sp = source.ok_or("MISSING_REQUIRED_ARGUMENT:--source-ifc")?;
    let tp = target.ok_or("MISSING_REQUIRED_ARGUMENT:--target-ifc")?;
    let sb = fs::read(&sp).map_err(|e| format!("CANNOT_READ_SOURCE_IFC:{}:{e}", sp.display()))?;
    let tb = fs::read(&tp).map_err(|e| format!("CANNOT_READ_TARGET_IFC:{}:{e}", tp.display()))?;
    let report = compare_bytes(
        &sb,
        &tb,
        &sp.display().to_string(),
        &tp.display().to_string(),
    );
    let bytes = serde_json::to_vec_pretty(&report)
        .map_err(|e| format!("REPORT_SERIALIZATION_FAILED:{e}"))?;
    if let Some(path) = output {
        fs::write(&path, &bytes)
            .map_err(|e| format!("CANNOT_WRITE_OUTPUT:{}:{e}", path.display()))?;
    }
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(match report.status {
        Status::Equal => 0,
        Status::Different => 1,
        Status::Unresolved => 2,
    })
}
fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("{e}");
            process::exit(64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = include_bytes!("../fixtures/two-walls.ifc");
    fn compare(a: &[u8], b: &[u8]) -> Report {
        compare_bytes(a, b, "fixture://source.ifc", "fixture://target.ifc")
    }

    #[test]
    fn identical_fixture_is_equal_and_raw_hashes_are_separate() {
        let r = compare(FIXTURE, FIXTURE);
        assert_eq!(r.status, Status::Equal);
        assert_eq!(r.counts.source_entities, 2);
        assert_eq!(r.counts.target_entities, 2);
        assert_eq!(r.changes.len(), 0);
        assert_eq!(r.source_artifact.sha256, sha256_hex(FIXTURE));
        assert_eq!(
            r.identity_coverage.fraction_of_roots_with_stable_identity,
            1.0
        );
    }
    #[test]
    fn reordered_data_records_preserve_semantics_when_global_ids_are_stable() {
        let source = String::from_utf8(FIXTURE.to_vec()).expect("fixture is UTF-8");
        let first = "#1=IFCWALL('0000000000000000000000',$,'Panel 001',$,$,$,$,$,.NOTDEFINED.);\n";
        let second = "#2=IFCWALL('0000000000000000000001',$,'Panel 002',$,$,$,$,$,.NOTDEFINED.);\n";
        let source_records = format!("{first}{second}");
        let reordered_records = format!("{second}{first}");
        let target = source.replace(&source_records, &reordered_records);
        assert_ne!(source, target);
        let report = compare(source.as_bytes(), target.as_bytes());
        assert_eq!(report.status, Status::Equal);
        assert_ne!(report.source_artifact.sha256, report.target_artifact.sha256);
        assert_eq!(report.source_semantic_sha256, report.target_semantic_sha256);
    }

    #[test]
    fn renumbering_ifcroot_step_ids_does_not_change_guid_keyed_semantics() {
        let source = String::from_utf8(FIXTURE.to_vec()).expect("fixture is UTF-8");
        let target = source
            .replace("#1=IFCWALL", "#17=IFCWALL")
            .replace("#2=IFCWALL", "#18=IFCWALL");
        let report = compare(source.as_bytes(), target.as_bytes());
        assert_eq!(report.status, Status::Equal);
        assert_ne!(report.source_artifact.sha256, report.target_artifact.sha256);
        assert_eq!(report.source_semantic_sha256, report.target_semantic_sha256);
    }

    #[test]
    fn invalid_global_id_is_unresolved_not_mispaired() {
        let source = String::from_utf8(FIXTURE.to_vec()).expect("fixture is UTF-8");
        let target = source.replace("'0000000000000000000000'", "'not-a-guid'");
        let report = compare(target.as_bytes(), target.as_bytes());
        assert_eq!(report.status, Status::Unresolved);
        assert!(
            report
                .coverage_issues
                .iter()
                .any(|issue| issue.code == "INVALID_GLOBAL_ID")
        );
        assert!(report.identity_coverage.roots_with_ambiguous_identity > 0);
    }

    #[test]
    fn property_change_is_named_and_structured() {
        let a = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let b = a.replace("Panel 001", "Panel 001 Changed");
        let r = compare(a.as_bytes(), b.as_bytes());
        assert_eq!(r.status, Status::Different);
        assert!(
            r.changes
                .iter()
                .any(|c| c.kind == ChangeKind::AttributeChanged
                    && c.attribute_name.as_deref() == Some("Name"))
        );
    }
    #[test]
    fn header_only_byte_change_can_remain_semantically_equal() {
        let a = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let b = a.replace(
            "ViewDefinition [CoordinationView]",
            "ViewDefinition [ReferenceView]",
        );
        let r = compare(a.as_bytes(), b.as_bytes());
        assert_ne!(r.source_artifact.sha256, r.target_artifact.sha256);
        assert_eq!(r.status, Status::Equal);
    }
    #[test]
    fn changed_global_id_is_added_and_removed_not_guessed() {
        let a = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let b = a.replace("0000000000000000000000", "0000000000000000000002");
        let r = compare(a.as_bytes(), b.as_bytes());
        assert_eq!(r.status, Status::Different);
        assert!(r.counts.entities_added > 0);
        assert!(r.counts.entities_removed > 0);
    }
    #[test]
    fn duplicate_guids_are_unresolved() {
        let a = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let b = a.replace("0000000000000000000001", "0000000000000000000000");
        let r = compare(a.as_bytes(), b.as_bytes());
        assert_eq!(r.status, Status::Unresolved);
        assert!(r.counts.identity_ambiguities > 0);
        assert!(
            r.coverage_issues
                .iter()
                .any(|i| i.code == "DUPLICATE_GLOBAL_ID")
        );
    }
    #[test]
    fn unsupported_schema_is_not_guessed() {
        let a = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let b = a.replace("FILE_SCHEMA(('IFC4'));", "FILE_SCHEMA(('IFC4X3_ADD2'));");
        let r = compare(a.as_bytes(), b.as_bytes());
        assert_eq!(r.status, Status::Unresolved);
        assert_eq!(r.target_schema_token.as_deref(), Some("IFC4X3_ADD2"));
    }
    #[test]
    fn malformed_step_retains_exact_digest_and_is_unresolved() {
        let r = compare(b"not IFC", FIXTURE);
        assert_eq!(r.status, Status::Unresolved);
        assert_eq!(r.source_artifact.sha256, sha256_hex(b"not IFC"));
        assert!(r.decision_reason.starts_with("SOURCE_STEP_PARSE_FAILED:"));
    }
    #[test]
    fn attribute_arity_mismatch_is_coverage_gap() {
        let a = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let b = a.replace(
            "'Panel 001',$,$,$,$,$,.NOTDEFINED.",
            "'Panel 001',$,$,$,$,.NOTDEFINED.",
        );
        let r = compare(a.as_bytes(), b.as_bytes());
        assert_eq!(r.status, Status::Unresolved);
        assert!(
            r.coverage_issues
                .iter()
                .any(|i| i.code == "ATTRIBUTE_ARITY_MISMATCH")
        );
    }
    #[test]
    fn references_resolve_through_stable_global_id() {
        let mut map = BTreeMap::new();
        map.insert(
            EntityId(7),
            Identity {
                key: "global-id:2O2Fr$t4X7Zf8NOew3FLOH".into(),
                basis: IdentityBasis::GlobalId,
                guid: Some("2O2Fr$t4X7Zf8NOew3FLOH".into()),
            },
        );
        let value = canonical_value(&IfcValue::Ref(EntityId(7)), &map, &[]);
        assert_eq!(value["target_key"], "global-id:2O2Fr$t4X7Zf8NOew3FLOH");
    }
    #[test]
    fn schema_set_and_bag_order_is_ignored_but_list_order_is_preserved() {
        use ifc_schema::attribute::{Aggregation, Bound};
        let a = IfcValue::Integer(1);
        let b = IfcValue::Integer(2);
        let list = Aggregation::new(AggregateKind::List, Bound::Integer(0), Bound::Unbounded);
        let set = Aggregation::new(AggregateKind::Set, Bound::Integer(0), Bound::Unbounded);
        let l1 = canonical_value(
            &IfcValue::List(vec![a.clone(), b.clone()]),
            &BTreeMap::new(),
            std::slice::from_ref(&list),
        );
        let l2 = canonical_value(
            &IfcValue::List(vec![b.clone(), a.clone()]),
            &BTreeMap::new(),
            &[list],
        );
        let s1 = canonical_value(
            &IfcValue::List(vec![a.clone(), b.clone()]),
            &BTreeMap::new(),
            std::slice::from_ref(&set),
        );
        let s2 = canonical_value(&IfcValue::List(vec![b, a]), &BTreeMap::new(), &[set]);
        assert_ne!(l1, l2);
        assert_eq!(s1, s2);
    }
    #[test]
    fn semantic_digest_is_deterministic() {
        let a = compare(FIXTURE, FIXTURE);
        let b = compare(FIXTURE, FIXTURE);
        assert_eq!(a.semantic_result_sha256, b.semantic_result_sha256);
    }
}
