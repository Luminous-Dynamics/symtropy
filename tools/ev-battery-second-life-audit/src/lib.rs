use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticFixture,
    Assumption,
    VendorClaim,
    LiteratureReported,
    Surrogate,
    Calculation,
    PhysicalMeasurement,
    IndependentLab,
}

impl EvidenceClass {
    fn is_physical(self) -> bool {
        matches!(self, Self::PhysicalMeasurement | Self::IndependentLab)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightsState {
    CandidateUnreviewed,
    ResearchUseAdmitted,
    EvaluationOnly,
    CommercialUseBlocked,
    RedistributionBlocked,
    Rejected,
}

impl RightsState {
    fn evaluation_admitted(self) -> bool {
        matches!(self, Self::ResearchUseAdmitted | Self::EvaluationOnly)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryCompleteness {
    Complete,
    Partial,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncidentState {
    EventRecorded,
    VerifiedAbsent,
    Unknown,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BmsStrategy {
    ReuseUnchanged,
    AuthorizedFirmwareUpdate,
    GatewayInterface,
    ReplaceBms,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateDisposition {
    Pass,
    Fail,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionApplication {
    PvSelfConsumption,
    MicrogridStorage,
    IndustrialPeakReduction,
    TelecomBackup,
    ClinicEssentialLoad,
    ThirdLifeUps,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionFeasibility {
    Feasible,
    Infeasible,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteDisposition {
    ContinueFirstLife,
    RepairAndReassess,
    EligibleSecondLifeMission,
    EligibleThirdLifeBackup,
    RecycleOrRecover,
    AbstainInsufficientEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub evidence_class: EvidenceClass,
    pub source_digest: String,
    pub claim_scope: String,
    pub validity_domain: String,
    pub rights_state: RightsState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellObservation {
    pub cell_id: String,
    pub pack_id: String,
    pub configuration_ref: String,
    pub chemistry: String,
    pub capacity_soh_pct: f64,
    pub dcir_mohm: f64,
    pub cell_dispersion_pct: Option<f64>,
    pub evidence_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncidentRecord {
    pub event_type: String,
    pub state: IncidentState,
    pub evidence_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardGate {
    pub gate_id: String,
    pub disposition: GateDisposition,
    pub evidence_id: String,
    pub requires_physical_evidence: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackCandidate {
    pub pack_id: String,
    pub vehicle_id: String,
    pub original_configuration_ref: String,
    pub current_configuration_ref: String,
    pub chemistry: String,
    pub cell_ids: Vec<String>,
    pub history_completeness: HistoryCompleteness,
    pub incident_history: Vec<IncidentRecord>,
    pub bms_strategy: BmsStrategy,
    pub bms_compatibility_disposition: GateDisposition,
    pub bms_evidence_id: String,
    pub capability_envelope_ref: String,
    pub capability_evidence_id: String,
    pub hard_gates: Vec<HardGate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mission {
    pub mission_id: String,
    pub application: MissionApplication,
    pub duty_cycle_ref: String,
    pub system_boundary: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoutingDecision {
    pub decision_id: String,
    pub pack_id: String,
    pub target_mission_id: Option<String>,
    pub route: RouteDisposition,
    pub rationale_evidence_id: String,
    pub mission_feasibility: Option<MissionFeasibility>,
    pub lower_bound_usable_energy_kwh: Option<f64>,
    pub lower_bound_peak_power_kw: Option<f64>,
    pub lower_bound_reserve_kwh: Option<f64>,
    pub worst_case_thermal_margin_c: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionPolicy {
    pub unknown_hard_gate_blocks_eligibility: bool,
    pub hard_gates_not_compensable: bool,
    pub read_only: bool,
    pub minimum_cell_count_per_pack: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SecondLifeManifest {
    pub manifest_id: String,
    pub missions: Vec<Mission>,
    pub cells: Vec<CellObservation>,
    pub packs: Vec<PackCandidate>,
    pub routing_decisions: Vec<RoutingDecision>,
    pub evidence_registry: Vec<EvidenceRecord>,
    pub policy: DecisionPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecondLifeAuditReport {
    pub manifest_id: String,
    pub pack_count: usize,
    pub cell_count: usize,
    pub second_life_eligible_count: usize,
    pub third_life_eligible_count: usize,
    pub repair_and_reassess_count: usize,
    pub recycle_or_recover_count: usize,
    pub abstain_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecondLifeAuditError(pub String);

impl std::fmt::Display for SecondLifeAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SecondLifeAuditError {}

fn require_nonempty(value: &str, label: &str) -> Result<(), SecondLifeAuditError> {
    if value.trim().is_empty() {
        return Err(SecondLifeAuditError(format!("{label} must not be empty")));
    }
    Ok(())
}

fn unique_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<BTreeSet<String>, SecondLifeAuditError> {
    let mut result = BTreeSet::new();

    for value in values {
        require_nonempty(value, label)?;
        if !result.insert(value.to_string()) {
            return Err(SecondLifeAuditError(format!("duplicate {label}: {value}")));
        }
    }

    Ok(result)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn admitted_evidence<'a>(
    registry: &'a BTreeMap<&str, &EvidenceRecord>,
    evidence_id: &str,
    owner: &str,
) -> Result<&'a EvidenceRecord, SecondLifeAuditError> {
    let record = registry.get(evidence_id).copied().ok_or_else(|| {
        SecondLifeAuditError(format!("{owner} references unknown evidence {evidence_id}"))
    })?;

    if !record.rights_state.evaluation_admitted() {
        return Err(SecondLifeAuditError(format!(
            "{owner} references evidence not admitted for evaluation"
        )));
    }

    Ok(record)
}

fn mission_matches_route(
    route: RouteDisposition,
    mission: &Mission,
) -> bool {
    match route {
        RouteDisposition::EligibleSecondLifeMission => {
            mission.application != MissionApplication::ThirdLifeUps
        }
        RouteDisposition::EligibleThirdLifeBackup => {
            mission.application == MissionApplication::ThirdLifeUps
        }
        _ => true,
    }
}

fn eligible_hard_gates(
    pack: &PackCandidate,
    registry: &BTreeMap<&str, &EvidenceRecord>,
) -> Result<bool, SecondLifeAuditError> {
    if pack.hard_gates.is_empty() {
        return Ok(false);
    }

    unique_ids(
        pack.hard_gates.iter().map(|gate| gate.gate_id.as_str()),
        "hard-gate ID",
    )?;

    let mut all_pass = true;

    for gate in &pack.hard_gates {
        let evidence = admitted_evidence(
            registry,
            &gate.evidence_id,
            &format!("pack {} gate {}", pack.pack_id, gate.gate_id),
        )?;

        if gate.disposition != GateDisposition::Pass {
            all_pass = false;
        }

        if gate.requires_physical_evidence && !evidence.evidence_class.is_physical() {
            all_pass = false;
        }
    }

    Ok(all_pass)
}

/// Audit second-life routing declarations. This is a read-only integrity
/// evaluator and cannot authorize hazardous operations or product release.
pub fn audit_second_life_manifest(
    manifest: &SecondLifeManifest,
) -> Result<SecondLifeAuditReport, SecondLifeAuditError> {
    require_nonempty(&manifest.manifest_id, "manifest ID")?;

    if manifest.missions.is_empty()
        || manifest.cells.is_empty()
        || manifest.packs.is_empty()
        || manifest.routing_decisions.is_empty()
        || manifest.evidence_registry.is_empty()
    {
        return Err(SecondLifeAuditError(
            "missions, cells, packs, routing decisions and evidence registry are required".into(),
        ));
    }

    if !manifest.policy.unknown_hard_gate_blocks_eligibility
        || !manifest.policy.hard_gates_not_compensable
        || !manifest.policy.read_only
        || manifest.policy.minimum_cell_count_per_pack == 0
    {
        return Err(SecondLifeAuditError(
            "decision policy must be fail-closed, read-only and require physical specimens".into(),
        ));
    }

    unique_ids(
        manifest.missions.iter().map(|item| item.mission_id.as_str()),
        "mission ID",
    )?;
    unique_ids(
        manifest.cells.iter().map(|item| item.cell_id.as_str()),
        "cell ID",
    )?;
    unique_ids(
        manifest.packs.iter().map(|item| item.pack_id.as_str()),
        "pack ID",
    )?;
    unique_ids(
        manifest
            .routing_decisions
            .iter()
            .map(|item| item.decision_id.as_str()),
        "routing decision ID",
    )?;
    unique_ids(
        manifest
            .evidence_registry
            .iter()
            .map(|item| item.evidence_id.as_str()),
        "evidence ID",
    )?;

    let mut evidence_by_id = BTreeMap::new();
    for evidence in &manifest.evidence_registry {
        if !valid_sha256(&evidence.source_digest) {
            return Err(SecondLifeAuditError(format!(
                "evidence {} has malformed SHA-256 digest",
                evidence.evidence_id
            )));
        }

        require_nonempty(&evidence.claim_scope, "evidence claim scope")?;
        require_nonempty(&evidence.validity_domain, "evidence validity domain")?;
        evidence_by_id.insert(evidence.evidence_id.as_str(), evidence);
    }

    let mission_by_id: BTreeMap<&str, &Mission> = manifest
        .missions
        .iter()
        .map(|mission| (mission.mission_id.as_str(), mission))
        .collect();

    for mission in &manifest.missions {
        require_nonempty(&mission.duty_cycle_ref, "mission duty-cycle reference")?;
    }

    let cell_by_id: BTreeMap<&str, &CellObservation> = manifest
        .cells
        .iter()
        .map(|cell| (cell.cell_id.as_str(), cell))
        .collect();

    let mut cell_membership = BTreeSet::new();
    let mut pack_by_id = BTreeMap::new();

    for pack in &manifest.packs {
        require_nonempty(&pack.vehicle_id, "vehicle ID")?;
        require_nonempty(&pack.original_configuration_ref, "original configuration reference")?;
        require_nonempty(&pack.current_configuration_ref, "current configuration reference")?;
        require_nonempty(&pack.chemistry, "pack chemistry")?;
        require_nonempty(&pack.bms_evidence_id, "BMS evidence reference")?;
        require_nonempty(&pack.capability_envelope_ref, "capability envelope reference")?;

        if pack.cell_ids.is_empty() {
            return Err(SecondLifeAuditError(format!(
                "pack {} has no cell membership",
                pack.pack_id
            )));
        }

        unique_ids(pack.cell_ids.iter().map(String::as_str), "pack cell ID")?;

        admitted_evidence(
            &evidence_by_id,
            &pack.bms_evidence_id,
            &format!("pack {} BMS strategy", pack.pack_id),
        )?;
        admitted_evidence(
            &evidence_by_id,
            &pack.capability_evidence_id,
            &format!("pack {} capability envelope", pack.pack_id),
        )?;

        let mut physical_cell_count = 0usize;
        for cell_id in &pack.cell_ids {
            let cell = cell_by_id.get(cell_id.as_str()).ok_or_else(|| {
                SecondLifeAuditError(format!(
                    "pack {} references unknown cell {}",
                    pack.pack_id, cell_id
                ))
            })?;

            if cell.pack_id != pack.pack_id {
                return Err(SecondLifeAuditError(format!(
                    "cell {} parent pack identity does not match membership",
                    cell_id
                )));
            }

            if cell.configuration_ref != pack.current_configuration_ref {
                return Err(SecondLifeAuditError(format!(
                    "cell {} configuration does not match current pack configuration",
                    cell_id
                )));
            }

            if !cell.capacity_soh_pct.is_finite()
                || !(0.0..=150.0).contains(&cell.capacity_soh_pct)
                || !cell.dcir_mohm.is_finite()
                || cell.dcir_mohm < 0.0
                || cell
                    .cell_dispersion_pct
                    .is_some_and(|value| !value.is_finite() || value < 0.0)
            {
                return Err(SecondLifeAuditError(format!(
                    "cell {} has invalid health observation",
                    cell_id
                )));
            }

            admitted_evidence(
                &evidence_by_id,
                &cell.evidence_id,
                &format!("cell {} health observation", cell_id),
            )?;

            if !cell_membership.insert(cell_id.as_str()) {
                return Err(SecondLifeAuditError(format!(
                    "cell {} is assigned to more than one pack",
                    cell_id
                )));
            }
            physical_cell_count += 1;
        }

        if physical_cell_count < manifest.policy.minimum_cell_count_per_pack {
            return Err(SecondLifeAuditError(format!(
                "pack {} does not meet the minimum independent physical specimen count",
                pack.pack_id
            )));
        }

        for incident in &pack.incident_history {
            require_nonempty(&incident.event_type, "incident event type")?;
            admitted_evidence(
                &evidence_by_id,
                &incident.evidence_id,
                &format!("pack {} incident history", pack.pack_id),
            )?;
        }

        if pack.bms_compatibility_disposition == GateDisposition::Pass
            && pack.bms_strategy == BmsStrategy::ReuseUnchanged
            && pack.bms_evidence_id.trim().is_empty()
        {
            return Err(SecondLifeAuditError(format!(
                "pack {} has no BMS compatibility evidence",
                pack.pack_id
            )));
        }

        pack_by_id.insert(pack.pack_id.as_str(), pack);
    }

    let all_cells: BTreeSet<&str> = cell_by_id.keys().copied().collect();
    if cell_membership != all_cells {
        return Err(SecondLifeAuditError(
            "every cell must belong to exactly one declared pack".into(),
        ));
    }

    unique_ids(
        manifest
            .routing_decisions
            .iter()
            .map(|item| item.pack_id.as_str()),
        "routing decision pack ID",
    )?;

    let decision_packs: BTreeSet<&str> = manifest
        .routing_decisions
        .iter()
        .map(|decision| decision.pack_id.as_str())
        .collect();
    let known_packs: BTreeSet<&str> = pack_by_id.keys().copied().collect();

    if decision_packs != known_packs {
        return Err(SecondLifeAuditError(
            "every pack must have exactly one routing decision".into(),
        ));
    }

    let mut second_life_eligible_count = 0usize;
    let mut third_life_eligible_count = 0usize;
    let mut repair_and_reassess_count = 0usize;
    let mut recycle_or_recover_count = 0usize;
    let mut abstain_count = 0usize;

    for decision in &manifest.routing_decisions {
        let pack = pack_by_id
            .get(decision.pack_id.as_str())
            .expect("routing decision pack closure was validated");

        admitted_evidence(
            &evidence_by_id,
            &decision.rationale_evidence_id,
            &format!("decision {}", decision.decision_id),
        )?;

        if let Some(mission_id) = &decision.target_mission_id {
            let mission = mission_by_id.get(mission_id.as_str()).ok_or_else(|| {
                SecondLifeAuditError(format!(
                    "decision {} references unknown target mission {}",
                    decision.decision_id, mission_id
                ))
            })?;

            if !mission_matches_route(decision.route, mission) {
                return Err(SecondLifeAuditError(format!(
                    "decision {} route does not match target mission class",
                    decision.decision_id
                )));
            }
        }

        let route_requires_eligibility = matches!(
            decision.route,
            RouteDisposition::EligibleSecondLifeMission
                | RouteDisposition::EligibleThirdLifeBackup
        );

        if route_requires_eligibility {
            let mission_id = decision.target_mission_id.as_deref().ok_or_else(|| {
                SecondLifeAuditError(format!(
                    "eligible decision {} has no target mission",
                    decision.decision_id
                ))
            })?;
            let _mission = mission_by_id
                .get(mission_id)
                .expect("eligible decision mission was validated");

            if decision.mission_feasibility != Some(MissionFeasibility::Feasible) {
                return Err(SecondLifeAuditError(format!(
                    "eligible decision {} has no feasible mission envelope",
                    decision.decision_id
                )));
            }

            if pack.history_completeness != HistoryCompleteness::Complete {
                return Err(SecondLifeAuditError(format!(
                    "eligible decision {} has incomplete first-life history",
                    decision.decision_id
                )));
            }

            if pack
                .incident_history
                .iter()
                .any(|record| record.state == IncidentState::Unknown)
            {
                return Err(SecondLifeAuditError(format!(
                    "eligible decision {} has unknown incident history",
                    decision.decision_id
                )));
            }

            for incident in &pack.incident_history {
                if incident.state == IncidentState::VerifiedAbsent {
                    let evidence = admitted_evidence(
                        &evidence_by_id,
                        &incident.evidence_id,
                        &format!("pack {} verified-absent incident history", pack.pack_id),
                    )?;
                    if !evidence.evidence_class.is_physical() {
                        return Err(SecondLifeAuditError(format!(
                            "eligible decision {} relies on non-physical evidence for verified absence",
                            decision.decision_id
                        )));
                    }
                }
            }

            for cell_id in &pack.cell_ids {
                let cell = cell_by_id
                    .get(cell_id.as_str())
                    .expect("pack cell closure validated");
                let evidence = admitted_evidence(
                    &evidence_by_id,
                    &cell.evidence_id,
                    &format!("eligible pack {} cell {}", pack.pack_id, cell_id),
                )?;
                if !evidence.evidence_class.is_physical() {
                    return Err(SecondLifeAuditError(format!(
                        "eligible decision {} uses non-physical cell-health evidence",
                        decision.decision_id
                    )));
                }
            }

            if pack.bms_compatibility_disposition != GateDisposition::Pass {
                return Err(SecondLifeAuditError(format!(
                    "eligible decision {} lacks BMS compatibility PASS",
                    decision.decision_id
                )));
            }

            if !eligible_hard_gates(pack, &evidence_by_id)? {
                return Err(SecondLifeAuditError(format!(
                    "eligible decision {} has a failed, unknown or non-physical required hard gate",
                    decision.decision_id
                )));
            }

            let bounded_metrics = [
                decision.lower_bound_usable_energy_kwh,
                decision.lower_bound_peak_power_kw,
                decision.lower_bound_reserve_kwh,
                decision.worst_case_thermal_margin_c,
            ];

            if bounded_metrics.iter().any(|value| {
                value.is_none_or(|number| !number.is_finite())
            }) {
                return Err(SecondLifeAuditError(format!(
                    "eligible decision {} lacks complete bounded mission metrics",
                    decision.decision_id
                )));
            }

            let positive_metrics = [
                decision.lower_bound_usable_energy_kwh.unwrap_or(0.0),
                decision.lower_bound_peak_power_kw.unwrap_or(0.0),
                decision.lower_bound_reserve_kwh.unwrap_or(0.0),
            ];
            if positive_metrics.iter().any(|value| *value < 0.0) {
                return Err(SecondLifeAuditError(format!(
                    "eligible decision {} has a negative capability bound",
                    decision.decision_id
                )));
            }

            match decision.route {
                RouteDisposition::EligibleSecondLifeMission => second_life_eligible_count += 1,
                RouteDisposition::EligibleThirdLifeBackup => third_life_eligible_count += 1,
                _ => {}
            }
        } else {
            match decision.route {
                RouteDisposition::RepairAndReassess => repair_and_reassess_count += 1,
                RouteDisposition::RecycleOrRecover => recycle_or_recover_count += 1,
                RouteDisposition::AbstainInsufficientEvidence => abstain_count += 1,
                RouteDisposition::ContinueFirstLife => {}
                RouteDisposition::EligibleSecondLifeMission
                | RouteDisposition::EligibleThirdLifeBackup => unreachable!(),
            }
        }
    }

    Ok(SecondLifeAuditReport {
        manifest_id: manifest.manifest_id.clone(),
        pack_count: manifest.packs.len(),
        cell_count: manifest.cells.len(),
        second_life_eligible_count,
        third_life_eligible_count,
        repair_and_reassess_count,
        recycle_or_recover_count,
        abstain_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> SecondLifeManifest {
        let evidence = vec![
            evidence_record("E-ID", EvidenceClass::SyntheticFixture, RightsState::EvaluationOnly, "1"),
            evidence_record("E-HISTORY", EvidenceClass::SyntheticFixture, RightsState::EvaluationOnly, "2"),
            evidence_record("E-SAFETY", EvidenceClass::SyntheticFixture, RightsState::EvaluationOnly, "3"),
            evidence_record("E-BMS", EvidenceClass::SyntheticFixture, RightsState::EvaluationOnly, "4"),
            evidence_record("E-CAP", EvidenceClass::SyntheticFixture, RightsState::EvaluationOnly, "5"),
            evidence_record("E-ROUTE", EvidenceClass::SyntheticFixture, RightsState::EvaluationOnly, "6"),
        ];

        let cells = vec![
            cell("A1", "PACK-A", "PACK-A-R1"),
            cell("A2", "PACK-A", "PACK-A-R1"),
            cell("B1", "PACK-B", "PACK-B-R2"),
            cell("B2", "PACK-B", "PACK-B-R2"),
            cell("C1", "PACK-C", "PACK-C-R1"),
            cell("C2", "PACK-C", "PACK-C-R1"),
        ];

        let packs = vec![
            pack(
                "PACK-A",
                vec!["A1", "A2"],
                HistoryCompleteness::Partial,
                BmsStrategy::GatewayInterface,
                GateDisposition::Unknown,
                vec![gate("safety", GateDisposition::Unknown, "E-SAFETY", true)],
                vec![incident(IncidentState::Unknown)],
            ),
            pack(
                "PACK-B",
                vec!["B1", "B2"],
                HistoryCompleteness::Complete,
                BmsStrategy::ReplaceBms,
                GateDisposition::Fail,
                vec![gate("safety", GateDisposition::Fail, "E-SAFETY", false)],
                vec![incident(IncidentState::EventRecorded)],
            ),
            pack(
                "PACK-C",
                vec!["C1", "C2"],
                HistoryCompleteness::Complete,
                BmsStrategy::AuthorizedFirmwareUpdate,
                GateDisposition::Unknown,
                vec![gate("cell_matching", GateDisposition::Fail, "E-HISTORY", false)],
                vec![incident(IncidentState::VerifiedAbsent)],
            ),
        ];

        let decisions = vec![
            decision(
                "D-A",
                "PACK-A",
                Some("PV"),
                RouteDisposition::AbstainInsufficientEvidence,
                MissionFeasibility::Unknown,
            ),
            decision(
                "D-B",
                "PACK-B",
                None,
                RouteDisposition::RecycleOrRecover,
                MissionFeasibility::Infeasible,
            ),
            decision(
                "D-C",
                "PACK-C",
                Some("UPS"),
                RouteDisposition::RepairAndReassess,
                MissionFeasibility::Unknown,
            ),
        ];

        SecondLifeManifest {
            manifest_id: "SYNTH-SECOND-LIFE".into(),
            missions: vec![
                Mission {
                    mission_id: "PV".into(),
                    application: MissionApplication::PvSelfConsumption,
                    duty_cycle_ref: "DUTY-PV".into(),
                    system_boundary: "pack".into(),
                },
                Mission {
                    mission_id: "UPS".into(),
                    application: MissionApplication::ThirdLifeUps,
                    duty_cycle_ref: "DUTY-UPS".into(),
                    system_boundary: "pack".into(),
                },
            ],
            cells,
            packs,
            routing_decisions: decisions,
            evidence_registry: evidence,
            policy: DecisionPolicy {
                unknown_hard_gate_blocks_eligibility: true,
                hard_gates_not_compensable: true,
                read_only: true,
                minimum_cell_count_per_pack: 2,
            },
        }
    }

    fn evidence_record(
        id: &str,
        class: EvidenceClass,
        rights: RightsState,
        digest_char: &str,
    ) -> EvidenceRecord {
        EvidenceRecord {
            evidence_id: id.into(),
            evidence_class: class,
            source_digest: digest_char.repeat(64),
            claim_scope: "synthetic fixture scope".into(),
            validity_domain: "test-only".into(),
            rights_state: rights,
        }
    }

    fn cell(id: &str, pack_id: &str, config: &str) -> CellObservation {
        CellObservation {
            cell_id: id.into(),
            pack_id: pack_id.into(),
            configuration_ref: config.into(),
            chemistry: "NMC".into(),
            capacity_soh_pct: 75.0,
            dcir_mohm: 1.5,
            cell_dispersion_pct: Some(5.0),
            evidence_id: "E-HISTORY".into(),
        }
    }

    fn gate(
        id: &str,
        disposition: GateDisposition,
        evidence_id: &str,
        physical: bool,
    ) -> HardGate {
        HardGate {
            gate_id: id.into(),
            disposition,
            evidence_id: evidence_id.into(),
            requires_physical_evidence: physical,
        }
    }

    fn incident(state: IncidentState) -> IncidentRecord {
        IncidentRecord {
            event_type: "thermal_event".into(),
            state,
            evidence_id: "E-HISTORY".into(),
        }
    }

    fn pack(
        id: &str,
        cell_ids: Vec<&str>,
        history: HistoryCompleteness,
        bms: BmsStrategy,
        bms_disposition: GateDisposition,
        hard_gates: Vec<HardGate>,
        incidents: Vec<IncidentRecord>,
    ) -> PackCandidate {
        PackCandidate {
            pack_id: id.into(),
            vehicle_id: format!("VEHICLE-{id}"),
            original_configuration_ref: format!("ORIGINAL-{id}"),
            current_configuration_ref: format!("{id}-R1"),
            chemistry: "NMC".into(),
            cell_ids: cell_ids.into_iter().map(str::to_string).collect(),
            history_completeness: history,
            incident_history: incidents,
            bms_strategy: bms,
            bms_compatibility_disposition: bms_disposition,
            bms_evidence_id: "E-BMS".into(),
            capability_envelope_ref: format!("CAPABILITY-{id}"),
            capability_evidence_id: "E-CAP".into(),
            hard_gates,
        }
    }

    fn decision(
        id: &str,
        pack_id: &str,
        mission_id: Option<&str>,
        route: RouteDisposition,
        feasibility: MissionFeasibility,
    ) -> RoutingDecision {
        RoutingDecision {
            decision_id: id.into(),
            pack_id: pack_id.into(),
            target_mission_id: mission_id.map(str::to_string),
            route,
            rationale_evidence_id: "E-ROUTE".into(),
            mission_feasibility: Some(feasibility),
            lower_bound_usable_energy_kwh: None,
            lower_bound_peak_power_kw: None,
            lower_bound_reserve_kwh: None,
            worst_case_thermal_margin_c: None,
        }
    }

    fn make_pack_eligible(manifest: &mut SecondLifeManifest, pack_id: &str) {
        let pack = manifest.packs.iter_mut().find(|x| x.pack_id == pack_id).unwrap();
        pack.history_completeness = HistoryCompleteness::Complete;
        pack.incident_history[0].state = IncidentState::VerifiedAbsent;
        pack.incident_history[0].evidence_id = "E-PHYSICAL".into();
        pack.bms_compatibility_disposition = GateDisposition::Pass;
        pack.hard_gates[0].disposition = GateDisposition::Pass;
        pack.hard_gates[0].evidence_id = "E-PHYSICAL".into();
        pack.hard_gates[0].requires_physical_evidence = true;

        let decision = manifest
            .routing_decisions
            .iter_mut()
            .find(|x| x.pack_id == pack_id)
            .unwrap();
        decision.route = RouteDisposition::EligibleSecondLifeMission;
        decision.target_mission_id = Some("PV".into());
        decision.mission_feasibility = Some(MissionFeasibility::Feasible);
        decision.lower_bound_usable_energy_kwh = Some(12.0);
        decision.lower_bound_peak_power_kw = Some(4.0);
        decision.lower_bound_reserve_kwh = Some(2.0);
        decision.worst_case_thermal_margin_c = Some(8.0);

        manifest.evidence_registry.push(evidence_record(
            "E-PHYSICAL",
            EvidenceClass::IndependentLab,
            RightsState::EvaluationOnly,
            "a",
        ));
        for cell in &mut manifest.cells {
            if cell.pack_id == pack_id {
                cell.evidence_id = "E-PHYSICAL".into();
            }
        }
    }

    #[test]
    fn accepts_abstain_repair_and_recycle_routes_without_eligible_reuse() {
        let report = audit_second_life_manifest(&valid_manifest()).expect("valid routing fixture");
        assert_eq!(report.pack_count, 3);
        assert_eq!(report.cell_count, 6);
        assert_eq!(report.second_life_eligible_count, 0);
        assert_eq!(report.third_life_eligible_count, 0);
        assert_eq!(report.abstain_count, 1);
        assert_eq!(report.recycle_or_recover_count, 1);
        assert_eq!(report.repair_and_reassess_count, 1);
    }

    #[test]
    fn rejects_eligible_route_with_unknown_safety_gate() {
        let mut manifest = valid_manifest();
        let decision = manifest
            .routing_decisions
            .iter_mut()
            .find(|x| x.pack_id == "PACK-A")
            .unwrap();
        decision.route = RouteDisposition::EligibleSecondLifeMission;
        decision.target_mission_id = Some("PV".into());
        decision.mission_feasibility = Some(MissionFeasibility::Feasible);
        decision.lower_bound_usable_energy_kwh = Some(10.0);
        decision.lower_bound_peak_power_kw = Some(3.0);
        decision.lower_bound_reserve_kwh = Some(1.0);
        decision.worst_case_thermal_margin_c = Some(5.0);

        let error = audit_second_life_manifest(&manifest)
            .expect_err("unknown history/safety must not allow eligibility");
        assert!(error.0.contains("incomplete first-life history"));
    }

    #[test]
    fn rejects_synthetic_evidence_for_physical_safety_gate() {
        let mut manifest = valid_manifest();
        make_pack_eligible(&mut manifest, "PACK-A");
        manifest.evidence_registry
            .iter_mut()
            .find(|x| x.evidence_id == "E-PHYSICAL")
            .unwrap()
            .evidence_class = EvidenceClass::SyntheticFixture;

        let error =
            audit_second_life_manifest(&manifest).expect_err("synthetic safety evidence rejected");
        assert!(error.0.contains("non-physical required hard gate"));
    }

    #[test]
    fn rejects_verified_absent_history_backed_only_by_synthetic_evidence() {
        let mut manifest = valid_manifest();
        make_pack_eligible(&mut manifest, "PACK-A");
        manifest.packs[0].incident_history[0].evidence_id = "E-HISTORY".into();

        let error = audit_second_life_manifest(&manifest)
            .expect_err("synthetic evidence cannot establish verified absence");
        assert!(error.0.contains("non-physical evidence for verified absence"));
    }

    #[test]
    fn rejects_synthetic_cell_health_evidence_for_eligible_pack() {
        let mut manifest = valid_manifest();
        make_pack_eligible(&mut manifest, "PACK-A");
        manifest.cells[0].evidence_id = "E-HISTORY".into();

        let error = audit_second_life_manifest(&manifest)
            .expect_err("synthetic cell health cannot qualify reuse");
        assert!(error.0.contains("non-physical cell-health evidence"));
    }

    #[test]
    fn accepts_eligible_route_only_with_admitted_physical_evidence_and_complete_history() {
        let mut manifest = valid_manifest();
        make_pack_eligible(&mut manifest, "PACK-A");

        let report = audit_second_life_manifest(&manifest).expect("physical fixture admitted");
        assert_eq!(report.second_life_eligible_count, 1);
    }

    #[test]
    fn rejects_unknown_evidence_rights() {
        let mut manifest = valid_manifest();
        manifest.evidence_registry[0].rights_state = RightsState::CandidateUnreviewed;

        let error = audit_second_life_manifest(&manifest).expect_err("rights not admitted");
        assert!(error.0.contains("not admitted for evaluation"));
    }

    #[test]
    fn rejects_cell_configuration_mismatch() {
        let mut manifest = valid_manifest();
        manifest.cells[0].configuration_ref = "OLD-CONFIG".into();

        let error = audit_second_life_manifest(&manifest).expect_err("stale cell config");
        assert!(error.0.contains("configuration does not match"));
    }

    #[test]
    fn rejects_pack_membership_gap() {
        let mut manifest = valid_manifest();
        manifest.packs[0].cell_ids.pop();

        let error = audit_second_life_manifest(&manifest).expect_err("unassigned cell");
        assert!(error.0.contains("every cell must belong to exactly one declared pack"));
    }

    #[test]
    fn rejects_missing_cell_history_for_pack() {
        let mut manifest = valid_manifest();
        manifest.packs[0].cell_ids.push("MISSING-CELL".into());

        let error = audit_second_life_manifest(&manifest).expect_err("unknown member");
        assert!(error.0.contains("references unknown cell"));
    }

    #[test]
    fn rejects_third_life_route_for_non_ups_mission() {
        let mut manifest = valid_manifest();
        manifest.routing_decisions[0].route = RouteDisposition::EligibleThirdLifeBackup;
        manifest.routing_decisions[0].target_mission_id = Some("PV".into());

        let error =
            audit_second_life_manifest(&manifest).expect_err("third-life mission mismatch");
        assert!(error.0.contains("route does not match target mission class"));
    }

    #[test]
    fn rejects_eligible_route_without_bms_compatibility() {
        let mut manifest = valid_manifest();
        make_pack_eligible(&mut manifest, "PACK-A");
        manifest.packs[0].bms_compatibility_disposition = GateDisposition::Unknown;

        let error = audit_second_life_manifest(&manifest).expect_err("BMS compatibility required");
        assert!(error.0.contains("lacks BMS compatibility PASS"));
    }

    #[test]
    fn rejects_eligible_route_without_full_lifecycle_history() {
        let mut manifest = valid_manifest();
        make_pack_eligible(&mut manifest, "PACK-A");
        manifest.packs[0].history_completeness = HistoryCompleteness::Partial;

        let error = audit_second_life_manifest(&manifest).expect_err("complete history required");
        assert!(error.0.contains("incomplete first-life history"));
    }

    #[test]
    fn reports_are_invariant_to_input_order() {
        let mut manifest = valid_manifest();
        let first = audit_second_life_manifest(&manifest).expect("first report");

        manifest.cells.reverse();
        manifest.packs.reverse();
        manifest.routing_decisions.reverse();
        manifest.missions.reverse();
        manifest.evidence_registry.reverse();

        let second = audit_second_life_manifest(&manifest).expect("permuted report");
        assert_eq!(first, second);
    }
}
