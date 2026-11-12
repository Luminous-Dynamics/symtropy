use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticFixture,
    Assumption,
    VendorClaim,
    LiteratureReported,
    Simulation,
    PhysicalMeasurement,
    IndependentLaboratory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialLineageRef {
    pub sample_lineage_id: String,
    pub mat013_schema_version: String,
    pub sample_scale: String,
    pub lineage_digest: String,
    pub source_evidence_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessStructureProjectionRef {
    pub pspp_subject_id: String,
    pub pspp_schema_version: String,
    pub sample_lineage_id: String,
    pub process_window_ref: String,
    pub realized_structure_ref: String,
    pub conditioned_property_evidence_refs: Vec<String>,
    pub mat_uq_profile_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceEvidenceRef {
    pub interface_subject_id: String,
    pub mat_interface_schema_version: String,
    pub interface_class: String,
    pub condition_state_ref: String,
    pub evidence_refs: Vec<String>,
    pub mat_uq_profile_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatteryCellConfig {
    pub cell_id: String,
    pub configuration_ref: String,
    pub process_window_ref: String,
    pub material_lineage: MaterialLineageRef,
    pub process_structure_projection: ProcessStructureProjectionRef,
    pub interface_refs: Vec<InterfaceEvidenceRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellTestEvidence {
    pub test_id: String,
    pub cell_id: String,
    pub configuration_ref: String,
    pub protocol_evidence_ref: String,
    pub metrology_evidence_ref: String,
    pub raw_data_sha256: String,
    pub result_evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackCellMembership {
    pub cell_id: String,
    pub cell_configuration_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackAssembly {
    pub pack_id: String,
    pub configuration_ref: String,
    pub topology_ref: String,
    pub cell_members: Vec<PackCellMembership>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionOutcomeEvidence {
    pub mission_id: String,
    pub pack_id: String,
    pub duty_cycle_ref: String,
    pub test_campaign_ref: String,
    pub system_boundary: String,
    pub outcome_evidence_ref: String,
    pub metrology_evidence_ref: String,
    pub raw_data_sha256: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvBatteryManifest {
    pub manifest_id: String,
    pub cells: Vec<BatteryCellConfig>,
    pub cell_tests: Vec<CellTestEvidence>,
    pub packs: Vec<PackAssembly>,
    pub mission_outcomes: Vec<MissionOutcomeEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvBatteryAuditReport {
    pub manifest_id: String,
    pub cell_ids: Vec<String>,
    pub cell_test_ids: Vec<String>,
    pub pack_ids: Vec<String>,
    pub mission_ids: Vec<String>,
    pub interface_reference_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvBatteryAuditError(pub String);

impl std::fmt::Display for EvBatteryAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for EvBatteryAuditError {}

fn require_nonempty(value: &str, field: &str) -> Result<(), EvBatteryAuditError> {
    if value.trim().is_empty() {
        return Err(EvBatteryAuditError(format!("{field} must not be empty")));
    }
    Ok(())
}

fn unique_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), EvBatteryAuditError> {
    let mut seen = BTreeSet::new();

    for value in values {
        require_nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(EvBatteryAuditError(format!("duplicate {label}: {value}")));
        }
    }

    Ok(())
}

fn require_digest(value: &str, label: &str) -> Result<(), EvBatteryAuditError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));

    if !valid {
        return Err(EvBatteryAuditError(format!(
            "{label} must be a lowercase 64-character SHA-256 digest"
        )));
    }

    Ok(())
}

fn require_refs(values: &[String], label: &str) -> Result<(), EvBatteryAuditError> {
    if values.is_empty() {
        return Err(EvBatteryAuditError(format!(
            "at least one {label} reference is required"
        )));
    }

    unique_ids(values.iter().map(String::as_str), label)
}

fn validate_cell(cell: &BatteryCellConfig) -> Result<usize, EvBatteryAuditError> {
    require_nonempty(&cell.cell_id, "cell ID")?;
    require_nonempty(&cell.configuration_ref, "cell configuration reference")?;
    require_nonempty(&cell.process_window_ref, "cell process-window reference")?;

    let lineage = &cell.material_lineage;
    require_nonempty(&lineage.sample_lineage_id, "MAT-013 sample-lineage ID")?;
    require_nonempty(&lineage.mat013_schema_version, "MAT-013 schema version")?;
    require_nonempty(&lineage.source_evidence_ref, "MAT-013 source evidence reference")?;
    require_digest(&lineage.lineage_digest, "MAT-013 lineage digest")?;

    if !matches!(
        lineage.sample_scale.as_str(),
        "coupon" | "lab_batch" | "device" | "pilot" | "industrial"
    ) {
        return Err(EvBatteryAuditError(format!(
            "cell {} has unsupported MAT-013 scale state {}",
            cell.cell_id, lineage.sample_scale
        )));
    }

    let projection = &cell.process_structure_projection;
    require_nonempty(&projection.pspp_subject_id, "MAT-PSPP subject ID")?;
    require_nonempty(&projection.pspp_schema_version, "MAT-PSPP schema version")?;
    require_nonempty(&projection.sample_lineage_id, "MAT-PSPP process lineage ID")?;
    require_nonempty(&projection.realized_structure_ref, "realized-structure reference")?;
    require_nonempty(&projection.mat_uq_profile_ref, "MAT-UQ profile reference")?;
    require_refs(
        &projection.conditioned_property_evidence_refs,
        "conditioned-property evidence",
    )?;

    if projection.sample_lineage_id != lineage.sample_lineage_id {
        return Err(EvBatteryAuditError(format!(
            "cell {} MAT-PSPP lineage does not match its MAT-013 sample lineage",
            cell.cell_id
        )));
    }

    if projection.process_window_ref != cell.process_window_ref {
        return Err(EvBatteryAuditError(format!(
            "cell {} process-window reference does not match MAT-PSPP projection",
            cell.cell_id
        )));
    }

    if cell.interface_refs.is_empty() {
        return Err(EvBatteryAuditError(format!(
            "cell {} has no MAT-INTERFACE references",
            cell.cell_id
        )));
    }

    unique_ids(
        cell.interface_refs
            .iter()
            .map(|item| item.interface_subject_id.as_str()),
        "MAT-INTERFACE subject ID",
    )?;

    for interface in &cell.interface_refs {
        require_nonempty(
            &interface.mat_interface_schema_version,
            "MAT-INTERFACE schema version",
        )?;
        require_nonempty(&interface.interface_class, "interface class")?;
        require_nonempty(&interface.condition_state_ref, "interface condition-state reference")?;
        require_nonempty(&interface.mat_uq_profile_ref, "interface MAT-UQ profile reference")?;
        require_refs(&interface.evidence_refs, "interface evidence")?;
    }

    Ok(cell.interface_refs.len())
}

/// Audit EV-specific references to canonical MAT-013, MAT-PSPP,
/// MAT-INTERFACE and MAT-UQ material evidence, then join those references
/// to cell tests, pack membership and mission outcomes.
///
/// This is a read-only consumer adapter. It does not recreate the upstream
/// sample DAG, property ontology, uncertainty engine, solver or test authority.
pub fn audit_ev_battery_manifest(
    manifest: &EvBatteryManifest,
) -> Result<EvBatteryAuditReport, EvBatteryAuditError> {
    require_nonempty(&manifest.manifest_id, "manifest ID")?;

    if manifest.cells.is_empty() || manifest.packs.is_empty() {
        return Err(EvBatteryAuditError(
            "at least one cell configuration and pack assembly are required".into(),
        ));
    }

    unique_ids(
        manifest.cells.iter().map(|item| item.cell_id.as_str()),
        "cell ID",
    )?;
    unique_ids(
        manifest.cell_tests.iter().map(|item| item.test_id.as_str()),
        "cell test ID",
    )?;
    unique_ids(
        manifest.packs.iter().map(|item| item.pack_id.as_str()),
        "pack ID",
    )?;
    unique_ids(
        manifest
            .mission_outcomes
            .iter()
            .map(|item| item.mission_id.as_str()),
        "mission ID",
    )?;

    let mut all_ids = BTreeSet::new();
    for id in manifest
        .cells
        .iter()
        .map(|item| item.cell_id.as_str())
        .chain(manifest.cell_tests.iter().map(|item| item.test_id.as_str()))
        .chain(manifest.packs.iter().map(|item| item.pack_id.as_str()))
        .chain(
            manifest
                .mission_outcomes
                .iter()
                .map(|item| item.mission_id.as_str()),
        )
    {
        if !all_ids.insert(id) {
            return Err(EvBatteryAuditError(format!(
                "ambiguous cross-kind identity: {id}"
            )));
        }
    }

    let mut interface_reference_count = 0usize;
    for cell in &manifest.cells {
        interface_reference_count += validate_cell(cell)?;
    }

    let cell_by_id: BTreeMap<&str, &BatteryCellConfig> = manifest
        .cells
        .iter()
        .map(|cell| (cell.cell_id.as_str(), cell))
        .collect();

    for test in &manifest.cell_tests {
        require_nonempty(&test.protocol_evidence_ref, "cell-test protocol evidence")?;
        if test.result_evidence_class.is_physical() {
            require_nonempty(&test.metrology_evidence_ref, "cell-test metrology evidence")?;
        }
        require_digest(&test.raw_data_sha256, "cell-test raw-data digest")?;

        let cell = cell_by_id.get(test.cell_id.as_str()).ok_or_else(|| {
            EvBatteryAuditError(format!(
                "cell test {} references unknown cell {}",
                test.test_id, test.cell_id
            ))
        })?;

        if test.configuration_ref != cell.configuration_ref {
            return Err(EvBatteryAuditError(format!(
                "cell test {} configuration does not match cell {}",
                test.test_id, test.cell_id
            )));
        }

        if test.result_evidence_class.is_physical()
            && test.metrology_evidence_ref.trim().is_empty()
        {
            return Err(EvBatteryAuditError(format!(
                "physical cell test {} lacks metrology evidence",
                test.test_id
            )));
        }
    }

    let tested_cell_ids: BTreeSet<&str> = manifest
        .cell_tests
        .iter()
        .map(|test| test.cell_id.as_str())
        .collect();

    let mut packed_cell_ids = BTreeSet::new();
    let pack_by_id: BTreeMap<&str, &PackAssembly> = manifest
        .packs
        .iter()
        .map(|pack| (pack.pack_id.as_str(), pack))
        .collect();

    for pack in &manifest.packs {
        require_nonempty(&pack.configuration_ref, "pack configuration reference")?;
        require_nonempty(&pack.topology_ref, "pack topology reference")?;

        if pack.cell_members.is_empty() {
            return Err(EvBatteryAuditError(format!(
                "pack {} has no cell memberships",
                pack.pack_id
            )));
        }

        unique_ids(
            pack.cell_members
                .iter()
                .map(|member| member.cell_id.as_str()),
            "pack cell membership",
        )?;

        for member in &pack.cell_members {
            let cell = cell_by_id.get(member.cell_id.as_str()).ok_or_else(|| {
                EvBatteryAuditError(format!(
                    "pack {} references unknown cell {}",
                    pack.pack_id, member.cell_id
                ))
            })?;

            if member.cell_configuration_ref != cell.configuration_ref {
                return Err(EvBatteryAuditError(format!(
                    "pack {} cell {} configuration does not match",
                    pack.pack_id, member.cell_id
                )));
            }

            if !tested_cell_ids.contains(member.cell_id.as_str()) {
                return Err(EvBatteryAuditError(format!(
                    "pack {} cell {} has no linked cell test",
                    pack.pack_id, member.cell_id
                )));
            }

            if !packed_cell_ids.insert(member.cell_id.as_str()) {
                return Err(EvBatteryAuditError(format!(
                    "cell {} is assigned to more than one pack",
                    member.cell_id
                )));
            }
        }
    }

    for mission in &manifest.mission_outcomes {
        require_nonempty(&mission.duty_cycle_ref, "mission duty-cycle reference")?;
        require_nonempty(&mission.test_campaign_ref, "mission test-campaign reference")?;
        require_nonempty(&mission.system_boundary, "mission system boundary")?;
        require_nonempty(&mission.outcome_evidence_ref, "mission outcome evidence")?;
        if mission.evidence_class.is_physical() {
            require_nonempty(&mission.metrology_evidence_ref, "mission metrology evidence")?;
        }
        require_digest(&mission.raw_data_sha256, "mission raw-data digest")?;

        if !matches!(
            mission.system_boundary.as_str(),
            "pack" | "installed_vehicle" | "installed_system"
        ) {
            return Err(EvBatteryAuditError(format!(
                "mission {} has unsupported system boundary",
                mission.mission_id
            )));
        }

        if !pack_by_id.contains_key(mission.pack_id.as_str()) {
            return Err(EvBatteryAuditError(format!(
                "mission {} references unknown pack {}",
                mission.mission_id, mission.pack_id
            )));
        }
    }

    let mut cell_ids: Vec<String> = manifest.cells.iter().map(|item| item.cell_id.clone()).collect();
    let mut cell_test_ids: Vec<String> =
        manifest.cell_tests.iter().map(|item| item.test_id.clone()).collect();
    let mut pack_ids: Vec<String> = manifest.packs.iter().map(|item| item.pack_id.clone()).collect();
    let mut mission_ids: Vec<String> = manifest
        .mission_outcomes
        .iter()
        .map(|item| item.mission_id.clone())
        .collect();

    cell_ids.sort();
    cell_test_ids.sort();
    pack_ids.sort();
    mission_ids.sort();

    Ok(EvBatteryAuditReport {
        manifest_id: manifest.manifest_id.clone(),
        cell_ids,
        cell_test_ids,
        pack_ids,
        mission_ids,
        interface_reference_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> EvBatteryManifest {
        EvBatteryManifest {
            manifest_id: "SYNTH-EV-OUTCOME".into(),
            cells: vec![BatteryCellConfig {
                cell_id: "CELL-1".into(),
                configuration_ref: "CELL-CONFIG-R1".into(),
                process_window_ref: "PROCESS-WINDOW-1".into(),
                material_lineage: MaterialLineageRef {
                    sample_lineage_id: "MAT013-SAMPLE-1".into(),
                    mat013_schema_version: "0.1".into(),
                    sample_scale: "device".into(),
                    lineage_digest: "a".repeat(64),
                    source_evidence_ref: "SYNTH-MAT013".into(),
                },
                process_structure_projection: ProcessStructureProjectionRef {
                    pspp_subject_id: "PSPP-1".into(),
                    pspp_schema_version: "0.1".into(),
                    sample_lineage_id: "MAT013-SAMPLE-1".into(),
                    process_window_ref: "PROCESS-WINDOW-1".into(),
                    realized_structure_ref: "STRUCTURE-1".into(),
                    conditioned_property_evidence_refs: vec!["PROPERTY-1".into()],
                    mat_uq_profile_ref: "MAT-UQ-1".into(),
                },
                interface_refs: vec![InterfaceEvidenceRef {
                    interface_subject_id: "INTERFACE-1".into(),
                    mat_interface_schema_version: "0.1".into(),
                    interface_class: "electrode_electrolyte_interface".into(),
                    condition_state_ref: "INTERFACE-STATE-1".into(),
                    evidence_refs: vec!["INTERFACE-EVIDENCE-1".into()],
                    mat_uq_profile_ref: "MAT-UQ-1".into(),
                }],
            }],
            cell_tests: vec![CellTestEvidence {
                test_id: "CELL-TEST-1".into(),
                cell_id: "CELL-1".into(),
                configuration_ref: "CELL-CONFIG-R1".into(),
                protocol_evidence_ref: "SYNTH-PROTOCOL".into(),
                metrology_evidence_ref: "SYNTH-METROLOGY".into(),
                raw_data_sha256: "b".repeat(64),
                result_evidence_class: EvidenceClass::SyntheticFixture,
            }],
            packs: vec![PackAssembly {
                pack_id: "PACK-1".into(),
                configuration_ref: "PACK-CONFIG-R1".into(),
                topology_ref: "PACK-TOPOLOGY-1".into(),
                cell_members: vec![PackCellMembership {
                    cell_id: "CELL-1".into(),
                    cell_configuration_ref: "CELL-CONFIG-R1".into(),
                }],
            }],
            mission_outcomes: vec![MissionOutcomeEvidence {
                mission_id: "MISSION-1".into(),
                pack_id: "PACK-1".into(),
                duty_cycle_ref: "DUTY-CYCLE-1".into(),
                test_campaign_ref: "MISSION-CAMPAIGN-1".into(),
                system_boundary: "pack".into(),
                outcome_evidence_ref: "SYNTH-MISSION-RESULT".into(),
                metrology_evidence_ref: "SYNTH-PACK-METROLOGY".into(),
                raw_data_sha256: "c".repeat(64),
                evidence_class: EvidenceClass::SyntheticFixture,
            }],
        }
    }

    #[test]
    fn accepts_closed_synthetic_ev_battery_reference_chain() {
        let report = audit_ev_battery_manifest(&valid_manifest()).expect("valid fixture");
        assert_eq!(report.cell_ids, vec!["CELL-1".to_string()]);
        assert_eq!(report.cell_test_ids, vec!["CELL-TEST-1".to_string()]);
        assert_eq!(report.pack_ids, vec!["PACK-1".to_string()]);
        assert_eq!(report.mission_ids, vec!["MISSION-1".to_string()]);
        assert_eq!(report.interface_reference_count, 1);
    }

    #[test]
    fn rejects_missing_mat013_identity() {
        let mut manifest = valid_manifest();
        manifest.cells[0].material_lineage.sample_lineage_id.clear();

        let error = audit_ev_battery_manifest(&manifest).expect_err("missing lineage fails");
        assert!(error.0.contains("MAT-013 sample-lineage ID"));
    }

    #[test]
    fn rejects_pspp_to_mat013_lineage_mismatch() {
        let mut manifest = valid_manifest();
        manifest.cells[0]
            .process_structure_projection
            .sample_lineage_id = "DIFFERENT-LINEAGE".into();

        let error = audit_ev_battery_manifest(&manifest).expect_err("lineage mismatch fails");
        assert!(error.0.contains("MAT-PSPP lineage does not match"));
    }

    #[test]
    fn rejects_missing_interface_uncertainty_profile() {
        let mut manifest = valid_manifest();
        manifest.cells[0].interface_refs[0]
            .mat_uq_profile_ref
            .clear();

        let error = audit_ev_battery_manifest(&manifest).expect_err("missing UQ ref fails");
        assert!(error.0.contains("interface MAT-UQ profile"));
    }

    #[test]
    fn rejects_cell_test_for_different_configuration() {
        let mut manifest = valid_manifest();
        manifest.cell_tests[0].configuration_ref = "CELL-CONFIG-R2".into();

        let error = audit_ev_battery_manifest(&manifest).expect_err("test config mismatch");
        assert!(error.0.contains("configuration does not match cell"));
    }

    #[test]
    fn rejects_pack_membership_for_different_cell_configuration() {
        let mut manifest = valid_manifest();
        manifest.packs[0].cell_members[0].cell_configuration_ref = "CELL-CONFIG-R2".into();

        let error = audit_ev_battery_manifest(&manifest).expect_err("pack mismatch");
        assert!(error.0.contains("configuration does not match"));
    }

    #[test]
    fn rejects_unknown_pack_in_mission_outcome() {
        let mut manifest = valid_manifest();
        manifest.mission_outcomes[0].pack_id = "MISSING-PACK".into();

        let error = audit_ev_battery_manifest(&manifest).expect_err("unknown pack fails");
        assert!(error.0.contains("references unknown pack"));
    }

    #[test]
    fn rejects_invalid_raw_test_digest() {
        let mut manifest = valid_manifest();
        manifest.cell_tests[0].raw_data_sha256 = "not-a-digest".into();

        let error = audit_ev_battery_manifest(&manifest).expect_err("invalid digest");
        assert!(error.0.contains("lowercase 64-character SHA-256"));
    }

    #[test]
    fn physical_mission_class_requires_declared_metrology() {
        let mut manifest = valid_manifest();
        manifest.mission_outcomes[0].evidence_class = EvidenceClass::PhysicalMeasurement;
        manifest.mission_outcomes[0].metrology_evidence_ref.clear();

        let error = audit_ev_battery_manifest(&manifest).expect_err("physical metrology required");
        assert!(error.0.contains("mission metrology evidence"));
    }

    #[test]
    fn rejects_pack_member_without_cell_test() {
        let mut manifest = valid_manifest();
        manifest.cell_tests.clear();

        let error =
            audit_ev_battery_manifest(&manifest).expect_err("untested pack member fails");
        assert!(error.0.contains("has no linked cell test"));
    }

    #[test]
    fn cell_cannot_be_assigned_to_two_packs() {
        let mut manifest = valid_manifest();
        manifest.packs.push(PackAssembly {
            pack_id: "PACK-2".into(),
            configuration_ref: "PACK-CONFIG-R2".into(),
            topology_ref: "PACK-TOPOLOGY-2".into(),
            cell_members: vec![PackCellMembership {
                cell_id: "CELL-1".into(),
                cell_configuration_ref: "CELL-CONFIG-R1".into(),
            }],
        });

        let error = audit_ev_battery_manifest(&manifest).expect_err("double membership fails");
        assert!(error.0.contains("assigned to more than one pack"));
    }

    #[test]
    fn report_is_invariant_to_input_order() {
        let original = valid_manifest();
        let original_report = audit_ev_battery_manifest(&original).expect("original manifest");

        let mut permuted = original;
        permuted.cells.reverse();
        permuted.cell_tests.reverse();
        permuted.packs.reverse();
        permuted.mission_outcomes.reverse();

        let permuted_report = audit_ev_battery_manifest(&permuted).expect("permuted manifest");
        assert_eq!(original_report, permuted_report);
    }
}
