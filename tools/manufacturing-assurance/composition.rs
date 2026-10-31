//! Offline structural verifier for a single-article manufacturing assurance manifest.
//!
//! This deliberately verifies composition structure and evidence-reference consistency only.
//! It does NOT hash referenced artifacts, validate signatures, authenticate authorities,
//! verify the underlying physical truth, or certify a manufacturing process.
//!
//! Compile and test without Cargo or external crates:
//!   rustc --edition=2021 --test tools/manufacturing-assurance/composition.rs -o /tmp/mfg-composition-tests
//!   /tmp/mfg-composition-tests
//!
//! Build the CLI:
//!   rustc --edition=2021 tools/manufacturing-assurance/composition.rs -o /tmp/mfg-composition
//!   /tmp/mfg-composition <manifest.tsv>

use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(test))]
use std::env;
#[cfg(not(test))]
use std::fs;
#[cfg(not(test))]
use std::process;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Meta {
    schema_version: String,
    composition_id: String,
    revision: String,
    mode: String,
    context_id: String,
    valid_from: String,
    valid_until: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Component {
    id: String,
    role: String,
    artifact_sha256: String,
    schema_id: String,
    schema_version: String,
    context_id: String,
    vvuq_evidence_id: String,
    peer_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Node {
    kind: String,
    id: String,
    scope_key: String,
    component_id: String,
    evidence_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evidence {
    class: String,
    subject_kind: String,
    subject_id: String,
    authority: String,
    artifact_sha256: String,
    receipt_sha256: String,
    status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Edge {
    from_kind: String,
    from_id: String,
    relation: String,
    to_kind: String,
    to_id: String,
    evidence_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Interface {
    id: String,
    from_component: String,
    from_port: String,
    to_component: String,
    to_port: String,
    mapping_id: String,
    unit_from: String,
    unit_to: String,
    unit_transform: String,
    clock_from: String,
    clock_to: String,
    clock_transform: String,
    join_policy: String,
}

#[derive(Clone, Debug, Default)]
struct Manifest {
    meta: Option<Meta>,
    components: BTreeMap<String, Component>,
    nodes: BTreeMap<String, Node>,
    evidence: BTreeMap<String, Evidence>,
    edges: Vec<Edge>,
    interfaces: BTreeMap<String, Interface>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuditSummary {
    composition_id: String,
    revision: String,
    mode: String,
    components: usize,
    nodes: usize,
    edges: usize,
    evidence_records: usize,
    interfaces: usize,
    scope_key: String,
}

const REQUIRED_ROLES: [&str; 8] = [
    "digital_thread",
    "machine",
    "inspection",
    "physics",
    "metrology",
    "quality",
    "intervention",
    "authority",
];

const REQUIRED_NODE_KINDS: [&str; 14] = [
    "PRODUCT",
    "LOT",
    "OPERATION",
    "MACHINE_CYCLE",
    "MODEL_RUN",
    "SPECIFICATION",
    "INSPECTION_RESULT",
    "MEASUREMENT",
    "UNCERTAINTY_BUDGET",
    "CONFORMITY",
    "ACTION_PROPOSAL",
    "AUTHORIZATION",
    "EXECUTION",
    "POSTCONDITION",
];

const REQUIRED_EDGES: [(&str, &str, &str, &str); 13] = [
    ("PRODUCT", "HAS_LOT", "LOT", "product identity binds the material lot"),
    ("LOT", "PROCESSED_IN", "OPERATION", "lot must be linked to the actual route operation"),
    ("OPERATION", "OBSERVED_BY", "MACHINE_CYCLE", "operation must have machine-cycle evidence"),
    ("OPERATION", "SIMULATED_BY", "MODEL_RUN", "operation must have a scoped physical-model run"),
    ("MACHINE_CYCLE", "EXPLICITLY_LINKED_TO", "INSPECTION_RESULT", "machine cycle and inspection need an explicit source-backed association"),
    ("INSPECTION_RESULT", "HAS_MEASUREMENT", "MEASUREMENT", "inspection result must bind the actual measurement"),
    ("MEASUREMENT", "HAS_UNCERTAINTY", "UNCERTAINTY_BUDGET", "measurement must bind its uncertainty budget"),
    ("MEASUREMENT", "COMPARED_TO", "SPECIFICATION", "measurement must bind the exact specification"),
    ("MEASUREMENT", "DISPOSED_BY", "CONFORMITY", "measurement must bind its conformity disposition"),
    ("CONFORMITY", "JUSTIFIES", "ACTION_PROPOSAL", "proposal must be linked to the decision context"),
    ("ACTION_PROPOSAL", "AUTHORIZED_BY", "AUTHORIZATION", "proposal must have a separately represented authorization"),
    ("AUTHORIZATION", "AUTHORIZES", "EXECUTION", "execution must be linked to its authorization"),
    ("EXECUTION", "VERIFIED_BY", "POSTCONDITION", "execution cannot close without postcondition evidence"),
];

const REQUIRED_INTERFACE_ROLES: [(&str, &str); 6] = [
    ("machine", "inspection"),
    ("inspection", "metrology"),
    ("physics", "quality"),
    ("metrology", "quality"),
    ("quality", "intervention"),
    ("intervention", "authority"),
];

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    if value.is_empty() {
        Err(format!("{context}: empty field"))
    } else {
        Ok(value.to_string())
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_utc(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| matches!(i, 4 | 7 | 10 | 13 | 16 | 19) || b.is_ascii_digit())
}

fn node_key(kind: &str, id: &str) -> String {
    format!("{kind}:{id}")
}

fn edge_key(edge: &Edge) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        edge.from_kind, edge.from_id, edge.relation, edge.to_kind, edge.to_id
    )
}

fn expected_component_role(kind: &str) -> Option<&'static str> {
    match kind {
        "PRODUCT" | "LOT" | "OPERATION" | "SPECIFICATION" => Some("digital_thread"),
        "MACHINE_CYCLE" | "EXECUTION" => Some("machine"),
        "MODEL_RUN" => Some("physics"),
        "INSPECTION_RESULT" => Some("inspection"),
        "MEASUREMENT" | "UNCERTAINTY_BUDGET" | "POSTCONDITION" => Some("metrology"),
        "CONFORMITY" => Some("quality"),
        "ACTION_PROPOSAL" => Some("intervention"),
        "AUTHORIZATION" => Some("authority"),
        _ => None,
    }
}

fn expected_node_evidence_class(kind: &str) -> Option<&'static str> {
    match kind {
        "PRODUCT" => Some("THREAD_IDENTITY"),
        "LOT" => Some("MATERIAL_LOT_IDENTITY"),
        "OPERATION" => Some("ROUTE_OPERATION"),
        "MACHINE_CYCLE" => Some("MACHINE_TELEMETRY"),
        "MODEL_RUN" => Some("SIMULATION_RUN"),
        "SPECIFICATION" => Some("SPECIFICATION"),
        "INSPECTION_RESULT" => Some("QIF_RESULT"),
        "MEASUREMENT" => Some("OBSERVATION"),
        "UNCERTAINTY_BUDGET" => Some("UNCERTAINTY_BUDGET"),
        "CONFORMITY" => Some("CONFORMITY_DECISION"),
        "ACTION_PROPOSAL" => Some("ACTION_PROPOSAL"),
        "AUTHORIZATION" => Some("OFFLINE_AUTHORIZATION"),
        "EXECUTION" => Some("OFFLINE_EXECUTION"),
        "POSTCONDITION" => Some("POSTCONDITION"),
        _ => None,
    }
}

fn parse_manifest(input: &str) -> Result<Manifest, String> {
    let mut manifest = Manifest::default();

    for (line_index, original) in input.lines().enumerate() {
        let line_number = line_index + 1;
        let line = original.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        let record = fields.first().copied().unwrap_or("");

        match record {
            "META" => {
                if fields.len() != 8 {
                    return Err(format!("line {line_number}: META requires 7 fields"));
                }
                if manifest.meta.is_some() {
                    return Err(format!("line {line_number}: duplicate META"));
                }
                let meta = Meta {
                    schema_version: nonempty(fields[1], "META schema_version")?,
                    composition_id: nonempty(fields[2], "META composition_id")?,
                    revision: nonempty(fields[3], "META revision")?,
                    mode: nonempty(fields[4], "META mode")?,
                    context_id: nonempty(fields[5], "META context_id")?,
                    valid_from: nonempty(fields[6], "META valid_from")?,
                    valid_until: nonempty(fields[7], "META valid_until")?,
                };
                manifest.meta = Some(meta);
            }
            "COMP" => {
                if fields.len() != 9 {
                    return Err(format!("line {line_number}: COMP requires 8 fields"));
                }
                let component = Component {
                    id: nonempty(fields[1], "COMP id")?,
                    role: nonempty(fields[2], "COMP role")?,
                    artifact_sha256: nonempty(fields[3], "COMP artifact_sha256")?,
                    schema_id: nonempty(fields[4], "COMP schema_id")?,
                    schema_version: nonempty(fields[5], "COMP schema_version")?,
                    context_id: nonempty(fields[6], "COMP context_id")?,
                    vvuq_evidence_id: nonempty(fields[7], "COMP vvuq_evidence_id")?,
                    peer_id: nonempty(fields[8], "COMP peer_id; use - when not federated")?,
                };
                if manifest.components.insert(component.id.clone(), component).is_some() {
                    return Err(format!("line {line_number}: duplicate component id"));
                }
            }
            "NODE" => {
                if fields.len() != 6 {
                    return Err(format!("line {line_number}: NODE requires 5 fields"));
                }
                let node = Node {
                    kind: nonempty(fields[1], "NODE kind")?,
                    id: nonempty(fields[2], "NODE id")?,
                    scope_key: nonempty(fields[3], "NODE scope_key")?,
                    component_id: nonempty(fields[4], "NODE component_id")?,
                    evidence_id: nonempty(fields[5], "NODE evidence_id")?,
                };
                let key = node_key(&node.kind, &node.id);
                if manifest.nodes.insert(key, node).is_some() {
                    return Err(format!("line {line_number}: duplicate node identity"));
                }
            }
            "EVIDENCE" => {
                if fields.len() != 9 {
                    return Err(format!("line {line_number}: EVIDENCE requires 8 fields"));
                }
                let evidence_id = nonempty(fields[1], "EVIDENCE id")?;
                let evidence = Evidence {
                    class: nonempty(fields[2], "EVIDENCE class")?,
                    subject_kind: nonempty(fields[3], "EVIDENCE subject_kind")?,
                    subject_id: nonempty(fields[4], "EVIDENCE subject_id")?,
                    authority: nonempty(fields[5], "EVIDENCE authority")?,
                    artifact_sha256: nonempty(fields[6], "EVIDENCE artifact_sha256")?,
                    receipt_sha256: nonempty(fields[7], "EVIDENCE receipt_sha256")?,
                    status: nonempty(fields[8], "EVIDENCE status")?,
                };
                if manifest.evidence.insert(evidence_id, evidence).is_some() {
                    return Err(format!("line {line_number}: duplicate evidence id"));
                }
            }
            "EDGE" => {
                if fields.len() != 7 {
                    return Err(format!("line {line_number}: EDGE requires 6 fields"));
                }
                manifest.edges.push(Edge {
                    from_kind: nonempty(fields[1], "EDGE from_kind")?,
                    from_id: nonempty(fields[2], "EDGE from_id")?,
                    relation: nonempty(fields[3], "EDGE relation")?,
                    to_kind: nonempty(fields[4], "EDGE to_kind")?,
                    to_id: nonempty(fields[5], "EDGE to_id")?,
                    evidence_id: nonempty(fields[6], "EDGE evidence_id")?,
                });
            }
            "IFACE" => {
                if fields.len() != 14 {
                    return Err(format!("line {line_number}: IFACE requires 13 fields"));
                }
                let interface = Interface {
                    id: nonempty(fields[1], "IFACE id")?,
                    from_component: nonempty(fields[2], "IFACE from_component")?,
                    from_port: nonempty(fields[3], "IFACE from_port")?,
                    to_component: nonempty(fields[4], "IFACE to_component")?,
                    to_port: nonempty(fields[5], "IFACE to_port")?,
                    mapping_id: nonempty(fields[6], "IFACE mapping_id")?,
                    unit_from: nonempty(fields[7], "IFACE unit_from")?,
                    unit_to: nonempty(fields[8], "IFACE unit_to")?,
                    unit_transform: nonempty(fields[9], "IFACE unit_transform")?,
                    clock_from: nonempty(fields[10], "IFACE clock_from")?,
                    clock_to: nonempty(fields[11], "IFACE clock_to")?,
                    clock_transform: nonempty(fields[12], "IFACE clock_transform")?,
                    join_policy: nonempty(fields[13], "IFACE join_policy")?,
                };
                if manifest.interfaces.insert(interface.id.clone(), interface).is_some() {
                    return Err(format!("line {line_number}: duplicate interface id"));
                }
            }
            _ => return Err(format!("line {line_number}: unknown record type {record:?}")),
        }
    }

    Ok(manifest)
}

fn evidence_for<'a>(
    manifest: &'a Manifest,
    evidence_id: &str,
    subject_kind: &str,
    subject_id: &str,
    expected_class: &str,
) -> Result<&'a Evidence, String> {
    let evidence = manifest
        .evidence
        .get(evidence_id)
        .ok_or_else(|| format!("missing evidence receipt {evidence_id}"))?;
    if evidence.subject_kind != subject_kind || evidence.subject_id != subject_id {
        return Err(format!(
            "evidence {evidence_id} subject mismatch: expected {subject_kind}:{subject_id}"
        ));
    }
    if evidence.class != expected_class {
        return Err(format!(
            "evidence {evidence_id} class mismatch: expected {expected_class}, got {}",
            evidence.class
        ));
    }
    if !is_sha256(&evidence.artifact_sha256) || !is_sha256(&evidence.receipt_sha256) {
        return Err(format!("evidence {evidence_id} has malformed SHA-256 reference"));
    }
    if evidence.authority == "-" {
        return Err(format!("evidence {evidence_id} has no declared authority"));
    }
    if !matches!(
        evidence.status.as_str(),
        "ADMITTED" | "VERIFIED" | "QUALIFIED" | "SYNTHETIC_ONLY"
    ) {
        return Err(format!(
            "evidence {evidence_id} has non-admissible status {}",
            evidence.status
        ));
    }
    Ok(evidence)
}

fn validate_transform(from: &str, to: &str, transform: &str, label: &str) -> Result<(), String> {
    if from == to {
        if transform != "IDENTITY" {
            return Err(format!("{label}: equal domains require IDENTITY transform"));
        }
    } else if !transform.starts_with("MAP:") || transform.len() <= 4 {
        return Err(format!(
            "{label}: differing domains require an explicit versioned MAP:<id> transform"
        ));
    }
    Ok(())
}

fn audit_manifest(manifest: &Manifest) -> Result<AuditSummary, String> {
    let meta = manifest.meta.as_ref().ok_or("missing META record")?;
    if meta.schema_version != "1" {
        return Err(format!("unsupported manifest schema version {}", meta.schema_version));
    }
    if !matches!(meta.mode.as_str(), "integrated" | "unified" | "federated") {
        return Err(format!("unsupported composition mode {}", meta.mode));
    }
    if !valid_utc(&meta.valid_from) || !valid_utc(&meta.valid_until) {
        return Err("META validity timestamps must be UTC YYYY-MM-DDTHH:MM:SSZ".to_string());
    }
    if meta.valid_from >= meta.valid_until {
        return Err("META validity interval must be non-empty and increasing".to_string());
    }

    if manifest.components.len() != REQUIRED_ROLES.len() {
        return Err(format!(
            "expected exactly {} component roles, got {}",
            REQUIRED_ROLES.len(),
            manifest.components.len()
        ));
    }

    let mut roles = BTreeSet::new();
    let mut peer_ids = BTreeSet::new();
    let mut used_evidence = BTreeSet::new();

    for component in manifest.components.values() {
        if !roles.insert(component.role.as_str()) {
            return Err(format!("duplicate component role {}", component.role));
        }
        if !REQUIRED_ROLES.contains(&component.role.as_str()) {
            return Err(format!("unsupported component role {}", component.role));
        }
        if component.context_id != meta.context_id {
            return Err(format!(
                "component {} context does not match composition context",
                component.id
            ));
        }
        if !is_sha256(&component.artifact_sha256) {
            return Err(format!("component {} has malformed artifact digest", component.id));
        }

        if meta.mode == "integrated"
            && (component.schema_id != "mfg.canonical" || component.schema_version != "1")
        {
            return Err(format!(
                "integrated composition requires common mfg.canonical/1 schema: {}",
                component.id
            ));
        }
        if meta.mode == "federated" {
            if component.peer_id == "-" || !peer_ids.insert(component.peer_id.as_str()) {
                return Err(format!(
                    "federated component {} requires a unique peer_id",
                    component.id
                ));
            }
        } else if component.peer_id != "-" {
            return Err(format!(
                "peer_id is only permitted for federated composition ({})",
                component.id
            ));
        }

        evidence_for(
            manifest,
            &component.vvuq_evidence_id,
            "COMPONENT",
            &component.id,
            "COMPONENT_VVUQ",
        )?;
        if receipt.status == "ADMITTED" && receipt.authority == "untrusted" {
            return Err(format!("component {} VVUQ authority is untrusted", component.id));
        }
        if !used_evidence.insert(component.vvuq_evidence_id.as_str()) {
            return Err(format!("evidence {} is reused", component.vvuq_evidence_id));
        }
    }
    for role in REQUIRED_ROLES {
        if !roles.contains(role) {
            return Err(format!("missing required component role {role}"));
        }
    }

    if manifest.nodes.len() != REQUIRED_NODE_KINDS.len() {
        return Err(format!(
            "expected exactly {} lifecycle nodes, got {}",
            REQUIRED_NODE_KINDS.len(),
            manifest.nodes.len()
        ));
    }
    let mut kinds = BTreeSet::new();
    let mut scope: Option<&str> = None;

    for node in manifest.nodes.values() {
        if !REQUIRED_NODE_KINDS.contains(&node.kind.as_str()) {
            return Err(format!("unsupported node kind {}", node.kind));
        }
        if !kinds.insert(node.kind.as_str()) {
            return Err(format!("multiple nodes of kind {}", node.kind));
        }
        if let Some(existing_scope) = scope {
            if existing_scope != node.scope_key {
                return Err(format!(
                    "node {}:{} has a different product/lot/operation scope",
                    node.kind, node.id
                ));
            }
        } else {
            scope = Some(node.scope_key.as_str());
        }

        let component = manifest
            .components
            .get(&node.component_id)
            .ok_or_else(|| format!("node {} references unknown component", node.id))?;
        let required_role = expected_component_role(&node.kind)
            .ok_or_else(|| format!("node kind {} has no component owner", node.kind))?;
        if component.role != required_role {
            return Err(format!(
                "node {}:{} must be produced by role {}, not {}",
                node.kind, node.id, required_role, component.role
            ));
        }
        let expected_class = expected_node_evidence_class(&node.kind)
            .ok_or_else(|| format!("node kind {} has no evidence class", node.kind))?;
        let subject_id = format!("{}:{}", node.kind, node.id);
        evidence_for(
            manifest,
            &node.evidence_id,
            "NODE",
            &subject_id,
            expected_class,
        )?;
        if !used_evidence.insert(node.evidence_id.as_str()) {
            return Err(format!("evidence {} is reused", node.evidence_id));
        }
    }
    for kind in REQUIRED_NODE_KINDS {
        if !kinds.contains(kind) {
            return Err(format!("missing required lifecycle node {kind}"));
        }
    }
    let product_id = manifest.nodes.values().find(|node| node.kind == "PRODUCT")
        .map(|node| node.id.as_str()).ok_or("missing PRODUCT identity")?;
    let lot_id = manifest.nodes.values().find(|node| node.kind == "LOT")
        .map(|node| node.id.as_str()).ok_or("missing LOT identity")?;
    let operation_id = manifest.nodes.values().find(|node| node.kind == "OPERATION")
        .map(|node| node.id.as_str()).ok_or("missing OPERATION identity")?;
    let expected_scope = format!("{product_id}/{lot_id}/{operation_id}");
    if scope != Some(expected_scope.as_str()) {
        return Err(format!(
            "scope_key must encode the exact product/lot/operation identity: expected {expected_scope}"
        ));
    }

    let mut seen_edges = BTreeSet::new();
    for edge in &manifest.edges {
        let from_key = node_key(&edge.from_kind, &edge.from_id);
        let to_key = node_key(&edge.to_kind, &edge.to_id);
        if !manifest.nodes.contains_key(&from_key) || !manifest.nodes.contains_key(&to_key) {
            return Err(format!("edge references unknown node: {}", edge_key(edge)));
        }
        if !seen_edges.insert(edge_key(edge)) {
            return Err(format!("duplicate edge {}", edge_key(edge)));
        }
        let subject_id = edge_key(edge);
        evidence_for(
            manifest,
            &edge.evidence_id,
            "EDGE",
            &subject_id,
            "EXPLICIT_ASSOCIATION",
        )?;
        if !used_evidence.insert(edge.evidence_id.as_str()) {
            return Err(format!("evidence {} is reused", edge.evidence_id));
        }
    }

    for (from_kind, relation, to_kind, rationale) in REQUIRED_EDGES {
        let matches: Vec<&Edge> = manifest
            .edges
            .iter()
            .filter(|edge| {
                edge.from_kind == from_kind
                    && edge.relation == relation
                    && edge.to_kind == to_kind
            })
            .collect();
        if matches.len() != 1 {
            return Err(format!(
                "required lifecycle relation missing or ambiguous ({from_kind} -{relation}-> {to_kind}): {rationale}"
            ));
        }
    }

    let mut interface_role_pairs = BTreeSet::new();
    for interface in manifest.interfaces.values() {
        if interface.from_component == interface.to_component {
            return Err(format!("interface {} cannot connect a component to itself", interface.id));
        }
        let from = manifest
            .components
            .get(&interface.from_component)
            .ok_or_else(|| format!("interface {} has unknown source component", interface.id))?;
        let to = manifest
            .components
            .get(&interface.to_component)
            .ok_or_else(|| format!("interface {} has unknown target component", interface.id))?;
        if interface.mapping_id == "-" || interface.from_port == "-" || interface.to_port == "-" {
            return Err(format!("interface {} is missing a typed mapping or port", interface.id));
        }
        validate_transform(
            &interface.unit_from,
            &interface.unit_to,
            &interface.unit_transform,
            &format!("interface {} unit", interface.id),
        )?;
        validate_transform(
            &interface.clock_from,
            &interface.clock_to,
            &interface.clock_transform,
            &format!("interface {} clock", interface.id),
        )?;
        if interface.join_policy != "EXPLICIT_ID" {
            return Err(format!(
                "interface {} must use explicit identity joins, not {}",
                interface.id, interface.join_policy
            ));
        }
        interface_role_pairs.insert((from.role.as_str(), to.role.as_str()));
    }

    if manifest.interfaces.len() < REQUIRED_INTERFACE_ROLES.len() {
        return Err(format!(
            "expected at least {} typed interfaces, got {}",
            REQUIRED_INTERFACE_ROLES.len(),
            manifest.interfaces.len()
        ));
    }
    for pair in REQUIRED_INTERFACE_ROLES {
        if !interface_role_pairs.contains(&pair) {
            return Err(format!("missing interface role pair {} -> {}", pair.0, pair.1));
        }
    }

    if used_evidence.len() != manifest.evidence.len() {
        let orphaned = manifest
            .evidence
            .keys()
            .filter(|id| !used_evidence.contains(id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!("orphan or unreferenced evidence records: {}", orphaned.join(",")));
    }

    Ok(AuditSummary {
        composition_id: meta.composition_id.clone(),
        revision: meta.revision.clone(),
        mode: meta.mode.clone(),
        components: manifest.components.len(),
        nodes: manifest.nodes.len(),
        edges: manifest.edges.len(),
        evidence_records: manifest.evidence.len(),
        interfaces: manifest.interfaces.len(),
        scope_key: scope.unwrap_or("-").to_string(),
    })
}

fn audit_text(input: &str) -> Result<AuditSummary, String> {
    let manifest = parse_manifest(input)?;
    audit_manifest(&manifest)
}

#[cfg(not(test))]
fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(format!(
            "usage: {} <manifest.tsv>",
            args.first().map(String::as_str).unwrap_or("mfg-composition-verifier")
        ));
    }
    let content = fs::read_to_string(&args[1])
        .map_err(|error| format!("cannot read {}: {error}", args[1]))?;
    let summary = audit_text(&content)?;
    println!(
        "MANIFEST_STRUCTURALLY_VALID composition={} revision={} mode={} components={} nodes={} edges={} interfaces={} evidence={} scope={}",
        summary.composition_id,
        summary.revision,
        summary.mode,
        summary.components,
        summary.nodes,
        summary.edges,
        summary.interfaces,
        summary.evidence_records,
        summary.scope_key
    );
    println!(
        "CLAIM_CEILING structural_consistency_only=true artifact_bytes_hashed=false signatures_verified=false authority_authenticated=false physical_validation_performed=false production_release_authorized=false"
    );
    Ok(())
}

#[cfg(not(test))]
fn main() {
    if let Err(error) = run() {
        eprintln!("mfg-composition-verifier: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(mode: &str) -> &'static str {
        match mode {
            "integrated" => include_str!("fixtures/valid-integrated.tsv"),
            "unified" => include_str!("fixtures/valid-unified.tsv"),
            "federated" => include_str!("fixtures/valid-federated.tsv"),
            _ => panic!("unknown fixture mode"),
        }
    }

    #[test]
    fn accepts_all_three_composition_modes() {
        for mode in ["integrated", "unified", "federated"] {
            let result = audit_text(fixture(mode));
            assert!(result.is_ok(), "{mode}: {result:?}");
            let summary = result.unwrap();
            assert_eq!(summary.composition_id, "mfg-demo-001");
            assert_eq!(summary.revision, "1");
            assert_eq!(summary.mode, mode);
            assert_eq!(summary.components, 8);
            assert_eq!(summary.nodes, 14);
            assert_eq!(summary.edges, 13);
            assert_eq!(summary.interfaces, 6);
            assert_eq!(summary.evidence_records, 35);
            assert_eq!(summary.scope_key, "product-001/lot-001/op-001");
        }
    }

    #[test]
    fn rejects_missing_explicit_cycle_to_inspection_link() {
        let input = fixture("integrated").replace(
            "EDGE|MACHINE_CYCLE|cycle-001|EXPLICITLY_LINKED_TO|INSPECTION_RESULT|qif-001|",
            "# EDGE deliberately removed: ",
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("MACHINE_CYCLE"));
    }

    #[test]
    fn rejects_cross_lot_scope_mismatch() {
        let input = fixture("integrated").replace(
            "NODE|MEASUREMENT|meas-001|product-001/lot-001/op-001|",
            "NODE|MEASUREMENT|meas-001|product-001/lot-999/op-001|",
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("different product/lot/operation scope"));
    }

    #[test]
    fn rejects_unqualified_component_vvuq_receipt() {
        let input = fixture("integrated").replace(
            "EVIDENCE|ev-vvuq-machine|COMPONENT_VVUQ|COMPONENT|cmp-machine|",
            "EVIDENCE|ev-vvuq-machine|WRONG_CLASS|COMPONENT|cmp-machine|",
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("class mismatch"));
    }

    #[test]
    fn rejects_non_explicit_join_policy() {
        let input = fixture("integrated").replace(
            "|shop_utc|shop_utc|IDENTITY|EXPLICIT_ID",
            "|shop_utc|shop_utc|IDENTITY|NEAREST_TIMESTAMP",
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("explicit identity joins"));
    }

    #[test]
    fn rejects_unit_change_without_versioned_transform() {
        let input = fixture("integrated").replace(
            "|mm|um|MAP:length-mm-to-um-v1|",
            "|mm|um|IDENTITY|",
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("differing domains require"));
    }

    #[test]
    fn rejects_wrong_integrated_schema() {
        let input = fixture("integrated").replace(
            "mfg.canonical|1|ctx-mfg-001",
            "qif-native|3.0|ctx-mfg-001",
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("common mfg.canonical/1 schema"));
    }

    #[test]
    fn rejects_federated_peer_identity_collision() {
        let input = fixture("federated").replace(
            "COMP|cmp-inspection|inspection|",
            "COMP|cmp-inspection|inspection|",
        ).replace(
            "|peer-inspection",
            "|peer-machine",
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unique peer_id"));
    }

    #[test]
    fn rejects_orphan_evidence_record() {
        let input = format!(
            "{}\nEVIDENCE|ev-orphan|UNUSED|NODE|UNUSED|fixture|{}|{}|SYNTHETIC_ONLY\n",
            fixture("integrated"),
            "f".repeat(64),
            "e".repeat(64)
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("orphan or unreferenced"));
    }

    #[test]
    fn rejects_malformed_sha256_reference() {
        let input = fixture("integrated").replace(
            "EVIDENCE|ev-node-product|THREAD_IDENTITY|NODE|PRODUCT:product-001|fixture|",
            "EVIDENCE|ev-node-product|THREAD_IDENTITY|NODE|PRODUCT:product-001|fixture|not-a-digest|",
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("malformed SHA-256"));
    }

    #[test]
    fn rejects_fuzzy_or_nearest_timestamp_link_semantics() {
        let input = fixture("integrated").replace(
            "EXPLICIT_ID",
            "FUZZY_MATCH",
        );
        let result = audit_text(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("explicit identity joins"));
    }

    #[test]
    fn repeated_input_is_deterministic() {
        let first = audit_text(fixture("unified")).unwrap();
        let second = audit_text(fixture("unified")).unwrap();
        assert_eq!(first, second);
    }
}
