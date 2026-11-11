use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessStage {
    SlurryPreparation,
    Coating,
    Drying,
    Calendering,
    Slitting,
    CellAssembly,
    Formation,
    Aging,
    Inspection,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Measured,
    Calculated,
    ModelPredicted,
    OperatorDeclared,
    Unknown,
    SyntheticFixture,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRecord {
    pub field: String,
    pub value: String,
    pub unit: String,
    pub kind: FieldKind,
    pub evidence_ref: String,
    pub instrument_id: Option<String>,
    pub calibration_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialLot {
    pub lot_id: String,
    pub material_family: String,
    pub source_ref: String,
    pub received_at_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessEvent {
    pub event_id: String,
    pub stage: ProcessStage,
    pub recipe_revision: String,
    pub equipment_id: String,
    pub timestamp_ns: u64,
    pub parent_ids: Vec<String>,
    pub child_ids: Vec<String>,
    pub field_records: Vec<FieldRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Specimen {
    pub specimen_id: String,
    pub specimen_type: String,
    pub created_at_ns: u64,
    pub created_by_event_id: String,
    pub parent_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellBuild {
    pub cell_id: String,
    pub created_at_ns: u64,
    pub configuration_ref: String,
    pub electrode_specimen_ids: Vec<String>,
    pub assembly_event_id: String,
    pub formation_event_id: String,
    pub parent_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultEvidenceClass {
    SyntheticFixture,
    Assumption,
    VendorClaim,
    LiteratureReported,
    Simulation,
    PhysicalMeasurement,
    IndependentLaboratory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestLink {
    pub test_id: String,
    pub cell_id: String,
    pub configuration_ref: String,
    pub protocol_evidence_ref: String,
    pub metrology_evidence_ref: String,
    pub raw_data_sha256: String,
    pub result_evidence_class: ResultEvidenceClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenealogyManifest {
    pub manifest_id: String,
    pub material_lots: Vec<MaterialLot>,
    pub process_events: Vec<ProcessEvent>,
    pub specimens: Vec<Specimen>,
    pub cell_builds: Vec<CellBuild>,
    pub test_links: Vec<TestLink>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenealogyReport {
    pub manifest_id: String,
    pub node_count: usize,
    pub event_count: usize,
    pub test_count: usize,
    pub topological_event_order: Vec<String>,
    pub tested_cell_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenealogyError(pub String);

impl std::fmt::Display for GenealogyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for GenealogyError {}

#[derive(Clone, Debug)]
struct NodeRecord {
    created_at_ns: u64,
    producer_event_id: Option<String>,
    parent_ids: BTreeSet<String>,
    specimen_type: Option<String>,
}

fn nonempty(value: &str, field: &str) -> Result<(), GenealogyError> {
    if value.trim().is_empty() {
        return Err(GenealogyError(format!("{field} must not be empty")));
    }
    Ok(())
}

fn unique_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), GenealogyError> {
    let mut seen = BTreeSet::new();

    for value in values {
        nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(GenealogyError(format!("duplicate {label}: {value}")));
        }
    }

    Ok(())
}

fn as_set(values: &[String], label: &str) -> Result<BTreeSet<String>, GenealogyError> {
    let mut result = BTreeSet::new();

    for value in values {
        nonempty(value, label)?;
        if !result.insert(value.clone()) {
            return Err(GenealogyError(format!("duplicate {label}: {value}")));
        }
    }

    Ok(result)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn insert_node(
    nodes: &mut BTreeMap<String, NodeRecord>,
    id: &str,
    record: NodeRecord,
) -> Result<(), GenealogyError> {
    nonempty(id, "node ID")?;
    if nodes.insert(id.to_string(), record).is_some() {
        return Err(GenealogyError(format!("duplicate node ID: {id}")));
    }
    Ok(())
}

fn topological_order(
    events: &BTreeMap<String, &ProcessEvent>,
    producer_by_node: &BTreeMap<String, String>,
) -> Result<Vec<String>, GenealogyError> {
    let mut outgoing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut indegree: BTreeMap<String, usize> =
        events.keys().map(|id| (id.clone(), 0)).collect();

    for event in events.values() {
        let mut dependencies = BTreeSet::new();

        for parent_id in &event.parent_ids {
            if let Some(producer) = producer_by_node.get(parent_id) {
                dependencies.insert(producer.clone());
            }
        }

        for dependency in dependencies {
            if !events.contains_key(&dependency) {
                return Err(GenealogyError(format!(
                    "node producer event {dependency} is missing"
                )));
            }

            if outgoing
                .entry(dependency.clone())
                .or_default()
                .insert(event.event_id.clone())
            {
                *indegree
                    .get_mut(&event.event_id)
                    .expect("all events have indegree entries") += 1;
            }
        }
    }

    let mut ready: BTreeSet<(u64, String)> = BTreeSet::new();
    for (event_id, degree) in &indegree {
        if *degree == 0 {
            let event = events.get(event_id).expect("event index is complete");
            ready.insert((event.timestamp_ns, event_id.clone()));
        }
    }

    let mut order = Vec::with_capacity(events.len());

    while let Some((timestamp, event_id)) = ready.iter().next().cloned() {
        ready.remove(&(timestamp, event_id.clone()));
        order.push(event_id.clone());

        if let Some(children) = outgoing.get(&event_id) {
            for child in children {
                let degree = indegree
                    .get_mut(child)
                    .expect("dependency child is a known event");
                *degree -= 1;

                if *degree == 0 {
                    let child_event = events.get(child).expect("child event is known");
                    ready.insert((child_event.timestamp_ns, child.clone()));
                }
            }
        }
    }

    if order.len() != events.len() {
        return Err(GenealogyError(
            "process-event dependency graph contains a cycle".into(),
        ));
    }

    Ok(order)
}

/// Audit a declared process-to-specimen-to-cell-to-test genealogy.
///
/// This is a read-only lineage check. It does not validate physical accuracy,
/// manufacturing capability, causal effects, battery safety, or test quality.
pub fn audit_genealogy(manifest: &GenealogyManifest) -> Result<GenealogyReport, GenealogyError> {
    nonempty(&manifest.manifest_id, "manifest ID")?;

    if manifest.material_lots.is_empty() || manifest.process_events.is_empty() {
        return Err(GenealogyError(
            "at least one material lot and process event are required".into(),
        ));
    }

    unique_ids(
        manifest.material_lots.iter().map(|item| item.lot_id.as_str()),
        "material lot ID",
    )?;
    unique_ids(
        manifest.process_events.iter().map(|item| item.event_id.as_str()),
        "process event ID",
    )?;
    unique_ids(
        manifest.specimens.iter().map(|item| item.specimen_id.as_str()),
        "specimen ID",
    )?;
    unique_ids(
        manifest.cell_builds.iter().map(|item| item.cell_id.as_str()),
        "cell ID",
    )?;
    unique_ids(
        manifest.test_links.iter().map(|item| item.test_id.as_str()),
        "test ID",
    )?;

    let mut nodes = BTreeMap::new();

    for lot in &manifest.material_lots {
        nonempty(&lot.material_family, "material family")?;
        nonempty(&lot.source_ref, "material source reference")?;
        insert_node(
            &mut nodes,
            &lot.lot_id,
            NodeRecord {
                created_at_ns: lot.received_at_ns,
                producer_event_id: None,
                parent_ids: BTreeSet::new(),
                specimen_type: None,
            },
        )?;
    }

    let mut producer_by_node = BTreeMap::new();

    for specimen in &manifest.specimens {
        nonempty(&specimen.specimen_type, "specimen type")?;
        nonempty(&specimen.created_by_event_id, "specimen producer event")?;
        let parents = as_set(&specimen.parent_ids, "specimen parent ID")?;

        producer_by_node.insert(
            specimen.specimen_id.clone(),
            specimen.created_by_event_id.clone(),
        );

        insert_node(
            &mut nodes,
            &specimen.specimen_id,
            NodeRecord {
                created_at_ns: specimen.created_at_ns,
                producer_event_id: Some(specimen.created_by_event_id.clone()),
                parent_ids: parents,
                specimen_type: Some(specimen.specimen_type.clone()),
            },
        )?;
    }

    for cell in &manifest.cell_builds {
        nonempty(&cell.configuration_ref, "cell configuration reference")?;
        nonempty(&cell.assembly_event_id, "cell assembly event")?;
        nonempty(&cell.formation_event_id, "cell formation event")?;

        if cell.electrode_specimen_ids.is_empty() {
            return Err(GenealogyError(format!(
                "cell {} has no electrode specimen references",
                cell.cell_id
            )));
        }

        let parents = as_set(&cell.parent_ids, "cell parent ID")?;
        producer_by_node.insert(cell.cell_id.clone(), cell.assembly_event_id.clone());

        insert_node(
            &mut nodes,
            &cell.cell_id,
            NodeRecord {
                created_at_ns: cell.created_at_ns,
                producer_event_id: Some(cell.assembly_event_id.clone()),
                parent_ids: parents,
                specimen_type: Some("cell".into()),
            },
        )?;
    }

    let mut events = BTreeMap::new();

    for event in &manifest.process_events {
        nonempty(&event.recipe_revision, "recipe revision")?;
        nonempty(&event.equipment_id, "equipment ID")?;

        if event.parent_ids.is_empty() || event.child_ids.is_empty() {
            return Err(GenealogyError(format!(
                "event {} must declare at least one parent and child",
                event.event_id
            )));
        }

        as_set(&event.parent_ids, "event parent ID")?;
        as_set(&event.child_ids, "event child ID")?;

        for field in &event.field_records {
            nonempty(&field.field, "process field name")?;
            nonempty(&field.unit, "process field unit")?;
            nonempty(&field.evidence_ref, "process field evidence reference")?;

            if field.kind == FieldKind::Measured {
                nonempty(
                    field.instrument_id.as_deref().unwrap_or(""),
                    "measured field instrument ID",
                )?;
                nonempty(
                    field.calibration_ref.as_deref().unwrap_or(""),
                    "measured field calibration reference",
                )?;
            }
        }

        events.insert(event.event_id.clone(), event);
    }

    for event in events.values() {
        let event_parents = as_set(&event.parent_ids, "event parent ID")?;

        for parent_id in &event_parents {
            let parent = nodes.get(parent_id).ok_or_else(|| {
                GenealogyError(format!(
                    "event {} references unknown parent node {}",
                    event.event_id, parent_id
                ))
            })?;

            if parent.created_at_ns > event.timestamp_ns {
                return Err(GenealogyError(format!(
                    "event {} precedes parent node {}",
                    event.event_id, parent_id
                )));
            }
        }

        for child_id in &event.child_ids {
            let child = nodes.get(child_id).ok_or_else(|| {
                GenealogyError(format!(
                    "event {} references unknown child node {}",
                    event.event_id, child_id
                ))
            })?;

            if child.producer_event_id.as_deref() != Some(event.event_id.as_str()) {
                return Err(GenealogyError(format!(
                    "event {} is not the declared producer of child {}",
                    event.event_id, child_id
                )));
            }

            if child.created_at_ns != event.timestamp_ns {
                return Err(GenealogyError(format!(
                    "child {} creation time does not match event {}",
                    child_id, event.event_id
                )));
            }

            if child.parent_ids != event_parents {
                return Err(GenealogyError(format!(
                    "child {} parent lineage does not match event {}",
                    child_id, event.event_id
                )));
            }
        }
    }

    for (node_id, node) in &nodes {
        if let Some(producer_id) = &node.producer_event_id {
            let producer = events.get(producer_id).ok_or_else(|| {
                GenealogyError(format!(
                    "node {} references missing producer event {}",
                    node_id, producer_id
                ))
            })?;

            if !producer.child_ids.iter().any(|child| child == node_id) {
                return Err(GenealogyError(format!(
                    "producer event {} does not list child {}",
                    producer_id, node_id
                )));
            }
        }
    }

    for cell in &manifest.cell_builds {
        let assembly = events.get(&cell.assembly_event_id).ok_or_else(|| {
            GenealogyError(format!(
                "cell {} references missing assembly event",
                cell.cell_id
            ))
        })?;

        if assembly.stage != ProcessStage::CellAssembly
            || !assembly.child_ids.iter().any(|child| child == &cell.cell_id)
        {
            return Err(GenealogyError(format!(
                "cell {} has invalid assembly event linkage",
                cell.cell_id
            )));
        }

        let formation = events.get(&cell.formation_event_id).ok_or_else(|| {
            GenealogyError(format!(
                "cell {} references missing formation event",
                cell.cell_id
            ))
        })?;

        if formation.stage != ProcessStage::Formation
            || !formation.parent_ids.iter().any(|parent| parent == &cell.cell_id)
        {
            return Err(GenealogyError(format!(
                "cell {} has invalid formation event linkage",
                cell.cell_id
            )));
        }

        for electrode_id in &cell.electrode_specimen_ids {
            let electrode = nodes.get(electrode_id).ok_or_else(|| {
                GenealogyError(format!(
                    "cell {} references unknown electrode {}",
                    cell.cell_id, electrode_id
                ))
            })?;

            if electrode.specimen_type.as_deref() != Some("electrode") {
                return Err(GenealogyError(format!(
                    "cell {} references a non-electrode specimen {}",
                    cell.cell_id, electrode_id
                )));
            }
        }
    }

    for test in &manifest.test_links {
        nonempty(&test.configuration_ref, "test configuration reference")?;
        nonempty(&test.protocol_evidence_ref, "test protocol evidence reference")?;
        nonempty(&test.metrology_evidence_ref, "test metrology evidence reference")?;

        let cell = manifest
            .cell_builds
            .iter()
            .find(|item| item.cell_id == test.cell_id)
            .ok_or_else(|| {
                GenealogyError(format!(
                    "test {} references unknown cell {}",
                    test.test_id, test.cell_id
                ))
            })?;

        if cell.configuration_ref != test.configuration_ref {
            return Err(GenealogyError(format!(
                "test {} configuration does not match cell {}",
                test.test_id, test.cell_id
            )));
        }

        if !is_sha256(&test.raw_data_sha256) {
            return Err(GenealogyError(format!(
                "test {} has an invalid lowercase SHA-256 digest",
                test.test_id
            )));
        }
    }

    let ordered_event_ids = topological_order(&events, &producer_by_node)?;
    let mut tested_cell_ids: Vec<String> = manifest
        .test_links
        .iter()
        .map(|test| test.cell_id.clone())
        .collect();
    tested_cell_ids.sort();
    tested_cell_ids.dedup();

    Ok(GenealogyReport {
        manifest_id: manifest.manifest_id.clone(),
        node_count: nodes.len(),
        event_count: events.len(),
        test_count: manifest.test_links.len(),
        topological_event_order: ordered_event_ids,
        tested_cell_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(field: &str) -> FieldRecord {
        FieldRecord {
            field: field.into(),
            value: "synthetic".into(),
            unit: "declared".into(),
            kind: FieldKind::SyntheticFixture,
            evidence_ref: "SYNTH-EVIDENCE".into(),
            instrument_id: None,
            calibration_ref: None,
        }
    }

    fn event(
        id: &str,
        stage: ProcessStage,
        timestamp_ns: u64,
        parents: &[&str],
        children: &[&str],
    ) -> ProcessEvent {
        ProcessEvent {
            event_id: id.into(),
            stage,
            recipe_revision: "synthetic-recipe-r1".into(),
            equipment_id: format!("equipment-{id}"),
            timestamp_ns,
            parent_ids: parents.iter().map(|value| (*value).into()).collect(),
            child_ids: children.iter().map(|value| (*value).into()).collect(),
            field_records: vec![field("synthetic_field")],
        }
    }

    fn valid_manifest() -> GenealogyManifest {
        GenealogyManifest {
            manifest_id: "SYNTH-GENEALOGY".into(),
            material_lots: vec![
                MaterialLot {
                    lot_id: "LOT-CATHODE".into(),
                    material_family: "synthetic-LFP".into(),
                    source_ref: "SYNTHETIC-DECLARATION".into(),
                    received_at_ns: 10,
                },
                MaterialLot {
                    lot_id: "LOT-ANODE".into(),
                    material_family: "synthetic-graphite".into(),
                    source_ref: "SYNTHETIC-DECLARATION".into(),
                    received_at_ns: 10,
                },
            ],
            process_events: vec![
                event(
                    "E1",
                    ProcessStage::SlurryPreparation,
                    20,
                    &["LOT-CATHODE"],
                    &["SLURRY"],
                ),
                event("E2", ProcessStage::Coating, 30, &["SLURRY"], &["WEB"]),
                event("E3", ProcessStage::Drying, 40, &["WEB"], &["DRIED-WEB"]),
                event(
                    "E4",
                    ProcessStage::Calendering,
                    50,
                    &["DRIED-WEB"],
                    &["ELECTRODE"],
                ),
                event(
                    "E5",
                    ProcessStage::CellAssembly,
                    60,
                    &["ELECTRODE", "LOT-ANODE"],
                    &["CELL"],
                ),
                event(
                    "E6",
                    ProcessStage::Formation,
                    70,
                    &["CELL"],
                    &["FORMED-CELL"],
                ),
            ],
            specimens: vec![
                Specimen {
                    specimen_id: "SLURRY".into(),
                    specimen_type: "slurry".into(),
                    created_at_ns: 20,
                    created_by_event_id: "E1".into(),
                    parent_ids: vec!["LOT-CATHODE".into()],
                },
                Specimen {
                    specimen_id: "WEB".into(),
                    specimen_type: "coated_web".into(),
                    created_at_ns: 30,
                    created_by_event_id: "E2".into(),
                    parent_ids: vec!["SLURRY".into()],
                },
                Specimen {
                    specimen_id: "DRIED-WEB".into(),
                    specimen_type: "dried_web".into(),
                    created_at_ns: 40,
                    created_by_event_id: "E3".into(),
                    parent_ids: vec!["WEB".into()],
                },
                Specimen {
                    specimen_id: "ELECTRODE".into(),
                    specimen_type: "electrode".into(),
                    created_at_ns: 50,
                    created_by_event_id: "E4".into(),
                    parent_ids: vec!["DRIED-WEB".into()],
                },
                Specimen {
                    specimen_id: "FORMED-CELL".into(),
                    specimen_type: "formed_cell".into(),
                    created_at_ns: 70,
                    created_by_event_id: "E6".into(),
                    parent_ids: vec!["CELL".into()],
                },
            ],
            cell_builds: vec![CellBuild {
                cell_id: "CELL".into(),
                created_at_ns: 60,
                configuration_ref: "CELL-CONFIG-R1".into(),
                electrode_specimen_ids: vec!["ELECTRODE".into()],
                assembly_event_id: "E5".into(),
                formation_event_id: "E6".into(),
                parent_ids: vec!["ELECTRODE".into(), "LOT-ANODE".into()],
            }],
            test_links: vec![TestLink {
                test_id: "TEST-1".into(),
                cell_id: "CELL".into(),
                configuration_ref: "CELL-CONFIG-R1".into(),
                protocol_evidence_ref: "SYNTH-PROTOCOL".into(),
                metrology_evidence_ref: "SYNTH-METROLOGY".into(),
                raw_data_sha256: "a".repeat(64),
                result_evidence_class: ResultEvidenceClass::SyntheticFixture,
            }],
        }
    }

    #[test]
    fn accepts_complete_synthetic_lineage() {
        let report = audit_genealogy(&valid_manifest()).expect("valid synthetic lineage");
        assert_eq!(report.node_count, 8);
        assert_eq!(report.event_count, 6);
        assert_eq!(report.test_count, 1);
        assert_eq!(report.topological_event_order, vec!["E1".to_string(), "E2".to_string(), "E3".to_string(), "E4".to_string(), "E5".to_string(), "E6".to_string()]);
        assert_eq!(report.tested_cell_ids, vec!["CELL"]);
    }

    #[test]
    fn rejects_orphan_parent_node() {
        let mut manifest = valid_manifest();
        manifest.process_events[0].parent_ids = vec!["MISSING-LOT".into()];

        let error = audit_genealogy(&manifest).expect_err("orphan must fail");
        assert!(error.0.contains("unknown parent node"));
    }

    #[test]
    fn rejects_duplicate_node_identity() {
        let mut manifest = valid_manifest();
        manifest.material_lots.push(MaterialLot {
            lot_id: "CELL".into(),
            material_family: "impossible-duplicate".into(),
            source_ref: "SYNTHETIC".into(),
            received_at_ns: 1,
        });

        let error = audit_genealogy(&manifest).expect_err("duplicate IDs must fail");
        assert!(error.0.contains("duplicate node ID"));
    }

    #[test]
    fn rejects_event_before_parent_creation() {
        let mut manifest = valid_manifest();
        manifest.process_events[1].timestamp_ns = 15;

        let error = audit_genealogy(&manifest).expect_err("time reversal must fail");
        assert!(error.0.contains("precedes parent node"));
    }

    #[test]
    fn rejects_child_with_different_parent_lineage() {
        let mut manifest = valid_manifest();
        manifest.specimens[1].parent_ids = vec!["LOT-ANODE".into()];

        let error = audit_genealogy(&manifest).expect_err("substituted parent must fail");
        assert!(error.0.contains("parent lineage does not match"));
    }

    #[test]
    fn rejects_wrong_cell_configuration_at_test_boundary() {
        let mut manifest = valid_manifest();
        manifest.test_links[0].configuration_ref = "CELL-CONFIG-R2".into();

        let error = audit_genealogy(&manifest).expect_err("configuration drift must fail");
        assert!(error.0.contains("configuration does not match"));
    }

    #[test]
    fn rejects_invalid_raw_data_digest() {
        let mut manifest = valid_manifest();
        manifest.test_links[0].raw_data_sha256 = "not-a-digest".into();

        let error = audit_genealogy(&manifest).expect_err("invalid digest must fail");
        assert!(error.0.contains("invalid lowercase SHA-256"));
    }

    #[test]
    fn measured_process_field_requires_instrument_and_calibration() {
        let mut manifest = valid_manifest();
        manifest.process_events[0].field_records[0].kind = FieldKind::Measured;

        let error = audit_genealogy(&manifest).expect_err("measurement requires metrology");
        assert!(error.0.contains("instrument ID"));
    }

    #[test]
    fn rejects_cyclic_event_dependencies() {
        let mut manifest = valid_manifest();
        manifest.material_lots = vec![MaterialLot {
            lot_id: "ROOT".into(),
            material_family: "synthetic".into(),
            source_ref: "synthetic".into(),
            received_at_ns: 1,
        }];
        manifest.process_events = vec![
            event("A", ProcessStage::Other("a".into()), 2, &["NODE-B"], &["NODE-A"]),
            event("B", ProcessStage::Other("b".into()), 2, &["NODE-A"], &["NODE-B"]),
        ];
        manifest.specimens = vec![
            Specimen {
                specimen_id: "NODE-A".into(),
                specimen_type: "intermediate".into(),
                created_at_ns: 2,
                created_by_event_id: "A".into(),
                parent_ids: vec!["NODE-B".into()],
            },
            Specimen {
                specimen_id: "NODE-B".into(),
                specimen_type: "intermediate".into(),
                created_at_ns: 2,
                created_by_event_id: "B".into(),
                parent_ids: vec!["NODE-A".into()],
            },
        ];
        manifest.cell_builds.clear();
        manifest.test_links.clear();

        let error = audit_genealogy(&manifest).expect_err("cycle must fail");
        assert!(error.0.contains("contains a cycle"));
    }

    #[test]
    fn reports_are_invariant_to_independent_input_order() {
        let baseline = valid_manifest();
        let baseline_report = audit_genealogy(&baseline).expect("valid baseline");

        let mut permuted = baseline;
        permuted.material_lots.reverse();
        permuted.process_events.reverse();
        permuted.specimens.reverse();
        permuted.cell_builds.reverse();
        permuted.test_links.reverse();

        let permuted_report = audit_genealogy(&permuted).expect("valid permuted manifest");
        assert_eq!(baseline_report, permuted_report);
    }
}
