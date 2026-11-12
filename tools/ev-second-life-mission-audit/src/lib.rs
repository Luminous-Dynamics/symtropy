#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationMode {
    SyntheticKnownAnswer,
    EvidenceReview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    PhysicalMeasurement,
    IndependentLaboratory,
    AuthorityDecision,
    Documentary,
    Simulation,
    Synthetic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    Candidate,
    AdmittedForResearch,
    AdmittedForEvaluation,
    Blocked,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HardGate {
    PackIdentity,
    CellCoverage,
    HistoryIntegrity,
    ThermalPropagationApproval,
    ElectricalIsolationAndProtection,
    BmsCompatibility,
    MissionCapability,
    TransportAndHandling,
    QualityControl,
    RightsAndProvenance,
}

impl HardGate {
    pub const ALL: [Self; 10] = [
        Self::PackIdentity,
        Self::CellCoverage,
        Self::HistoryIntegrity,
        Self::ThermalPropagationApproval,
        Self::ElectricalIsolationAndProtection,
        Self::BmsCompatibility,
        Self::MissionCapability,
        Self::TransportAndHandling,
        Self::QualityControl,
        Self::RightsAndProvenance,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateStatus {
    Pass,
    Fail,
    Unknown,
    NotApplicableWithRationale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryCoverage {
    CompleteSinceFirstCommissioning,
    Partial,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncidentHistory {
    NoAdverseEventsRecordedInCompleteHistory,
    AdverseEventsPresentAndAdjudicated,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalReviewStatus {
    AcceptedByCompetentAuthority,
    CandidateForHumanReview,
    Rejected,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BmsStrategy {
    ReuseOriginalUnchanged,
    AuthorizedFirmwareUpdate,
    GatewayInterface,
    ReplaceWithQualifiedSystem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BmsCompatibility {
    VerifiedForTargetMission,
    NotVerified,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestedUse {
    SecondLifeStationary,
    ThirdLifeBackup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub source_digest: String,
    pub evidence_class: EvidenceClass,
    pub admission: Admission,
    pub claim_scope: String,
    pub rights_ref: String,
    pub metrology_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateAssessment {
    pub gate: HardGate,
    pub status: GateStatus,
    pub evidence_id: String,
    pub rationale: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MissionRequirement {
    pub horizon_months: u16,
    pub required_usable_energy_at_commissioning_kwh: f64,
    pub required_usable_energy_at_horizon_kwh: f64,
    pub required_lifetime_delivered_energy_mwh: f64,
    pub required_peak_power_kw: f64,
    pub temperature_min_c: f64,
    pub temperature_max_c: f64,
    pub minimum_availability_percent: f64,
    pub reserve_percent: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityEnvelope {
    pub lower_bound_usable_energy_at_commissioning_kwh: f64,
    pub lower_bound_usable_energy_at_horizon_kwh: f64,
    pub lower_bound_lifetime_delivered_energy_mwh: f64,
    pub lower_bound_peak_power_kw: f64,
    pub supported_temperature_min_c: f64,
    pub supported_temperature_max_c: f64,
    pub lower_bound_availability_percent: f64,
    pub uncertainty_profile_ref: String,
    pub evidence_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BmsConfiguration {
    pub strategy: BmsStrategy,
    pub hardware_revision: String,
    pub firmware_revision: String,
    pub compatibility: BmsCompatibility,
    pub verification_ref: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualificationCase {
    pub case_id: String,
    pub source_pack_id: String,
    pub pack_revision: String,
    pub passport_ref: String,
    pub passport_evidence_id: String,
    pub configuration_digest: String,
    pub cell_membership_digest: String,
    pub expected_cell_count: usize,
    pub observed_cell_ids: Vec<String>,
    pub history_coverage: HistoryCoverage,
    pub incident_history: IncidentHistory,
    pub incident_adjudication_ref: Option<String>,
    pub requested_use: RequestedUse,
    pub mission: MissionRequirement,
    pub capability: CapabilityEnvelope,
    pub bms: BmsConfiguration,
    pub thermal_review_status: ThermalReviewStatus,
    pub thermal_authority_decision_ref: Option<String>,
    pub gates: Vec<GateAssessment>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Manifest {
    pub campaign_id: String,
    pub evaluation_mode: EvaluationMode,
    pub evidence: Vec<EvidenceRecord>,
    pub cases: Vec<QualificationCase>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    EligibleForSecondLifeMission,
    EligibleForThirdLifeBackup,
    AbstainInsufficientEvidence,
    SyntheticFixtureOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseReport {
    pub case_id: String,
    pub source_pack_id: String,
    pub disposition: Disposition,
    pub blockers: Vec<String>,
    pub gates_passed: usize,
    pub gates_total: usize,
    pub commissioning_energy_margin_milli_kwh: i64,
    pub horizon_energy_margin_milli_kwh: i64,
    pub lifetime_delivered_energy_margin_milli_mwh: i64,
    pub mission_power_margin_milli_kw: i64,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditReport {
    pub campaign_id: String,
    pub case_count: usize,
    pub eligible_second_life_count: usize,
    pub eligible_third_life_count: usize,
    pub abstain_count: usize,
    pub synthetic_fixture_count: usize,
    pub cases: Vec<CaseReport>,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditError(pub String);

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AuditError {}

fn require_nonempty(value: &str, label: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        return Err(AuditError(format!("{label} must not be empty")));
    }
    Ok(())
}

fn finite_nonnegative(value: f64, label: &str) -> Result<(), AuditError> {
    if !value.is_finite() || value < 0.0 {
        return Err(AuditError(format!("{label} must be finite and nonnegative")));
    }
    Ok(())
}

fn finite_positive(value: f64, label: &str) -> Result<(), AuditError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(AuditError(format!("{label} must be finite and positive")));
    }
    Ok(())
}

fn percent(value: f64, label: &str) -> Result<(), AuditError> {
    finite_nonnegative(value, label)?;
    if value > 100.0 {
        return Err(AuditError(format!("{label} must not exceed 100")));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), AuditError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(AuditError(format!("{label} must be lowercase 64-character SHA-256")));
    }
    Ok(())
}

fn validate_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), AuditError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(AuditError(format!("duplicate {label}: {value}")));
        }
    }
    Ok(())
}

fn evidence_registry(
    records: &[EvidenceRecord],
) -> Result<BTreeMap<&str, &EvidenceRecord>, AuditError> {
    if records.is_empty() {
        return Err(AuditError("evidence registry must not be empty".into()));
    }
    validate_unique(records.iter().map(|e| e.evidence_id.as_str()), "evidence ID")?;
    let mut registry = BTreeMap::new();
    for e in records {
        validate_digest(&e.source_digest, "evidence source digest")?;
        require_nonempty(&e.claim_scope, "evidence claim scope")?;
        require_nonempty(&e.rights_ref, "evidence rights reference")?;
        if matches!(e.evidence_class, EvidenceClass::PhysicalMeasurement | EvidenceClass::IndependentLaboratory)
            && e.metrology_ref.as_deref().is_none_or(str::is_empty)
        {
            return Err(AuditError(format!(
                "physical evidence {} requires a metrology reference",
                e.evidence_id
            )));
        }
        registry.insert(e.evidence_id.as_str(), e);
    }
    Ok(registry)
}

fn admitted<'a>(
    registry: &'a BTreeMap<&'a str, &'a EvidenceRecord>,
    id: &str,
    owner: &str,
) -> Result<&'a EvidenceRecord, AuditError> {
    let e = registry
        .get(id)
        .copied()
        .ok_or_else(|| AuditError(format!("{owner} references unknown evidence {id}")))?;
    if e.admission != Admission::AdmittedForEvaluation {
        return Err(AuditError(format!("{owner} evidence {id} is not admitted for evaluation")));
    }
    Ok(e)
}

fn gate_allowed(gate: HardGate, class: EvidenceClass) -> bool {
    match gate {
        HardGate::ThermalPropagationApproval => class == EvidenceClass::AuthorityDecision,
        HardGate::ElectricalIsolationAndProtection | HardGate::BmsCompatibility => matches!(
            class,
            EvidenceClass::PhysicalMeasurement
                | EvidenceClass::IndependentLaboratory
                | EvidenceClass::AuthorityDecision
        ),
        HardGate::MissionCapability | HardGate::QualityControl => matches!(
            class,
            EvidenceClass::PhysicalMeasurement | EvidenceClass::IndependentLaboratory
        ),
        HardGate::PackIdentity
        | HardGate::CellCoverage
        | HardGate::HistoryIntegrity
        | HardGate::TransportAndHandling
        | HardGate::RightsAndProvenance => matches!(
            class,
            EvidenceClass::PhysicalMeasurement
                | EvidenceClass::IndependentLaboratory
                | EvidenceClass::AuthorityDecision
                | EvidenceClass::Documentary
        ),
    }
}

fn validate_mission(m: &MissionRequirement) -> Result<(), AuditError> {
    if m.horizon_months == 0 {
        return Err(AuditError("mission horizon must be nonzero".into()));
    }
    finite_positive(
        m.required_usable_energy_at_commissioning_kwh,
        "mission commissioning usable energy",
    )?;
    finite_positive(
        m.required_usable_energy_at_horizon_kwh,
        "mission end-of-horizon usable energy",
    )?;
    finite_positive(
        m.required_lifetime_delivered_energy_mwh,
        "mission lifetime delivered energy",
    )?;
    finite_positive(m.required_peak_power_kw, "mission peak power")?;
    finite_nonnegative(m.temperature_min_c + 273.15, "mission minimum absolute temperature")?;
    finite_nonnegative(m.temperature_max_c + 273.15, "mission maximum absolute temperature")?;
    if m.temperature_min_c > m.temperature_max_c {
        return Err(AuditError("mission temperature range is inverted".into()));
    }
    percent(m.minimum_availability_percent, "mission minimum availability")?;
    percent(m.reserve_percent, "mission reserve")?;
    Ok(())
}

fn validate_capability(c: &CapabilityEnvelope) -> Result<(), AuditError> {
    finite_nonnegative(
        c.lower_bound_usable_energy_at_commissioning_kwh,
        "lower-bound commissioning usable energy",
    )?;
    finite_nonnegative(
        c.lower_bound_usable_energy_at_horizon_kwh,
        "lower-bound end-of-horizon usable energy",
    )?;
    finite_nonnegative(
        c.lower_bound_lifetime_delivered_energy_mwh,
        "lower-bound lifetime delivered energy",
    )?;
    finite_nonnegative(c.lower_bound_peak_power_kw, "lower-bound peak power")?;
    finite_nonnegative(
        c.supported_temperature_min_c + 273.15,
        "capability minimum absolute temperature",
    )?;
    finite_nonnegative(
        c.supported_temperature_max_c + 273.15,
        "capability maximum absolute temperature",
    )?;
    if c.supported_temperature_min_c > c.supported_temperature_max_c {
        return Err(AuditError("capability temperature range is inverted".into()));
    }
    percent(c.lower_bound_availability_percent, "lower-bound availability")?;
    require_nonempty(&c.uncertainty_profile_ref, "capability uncertainty profile reference")?;
    require_nonempty(&c.evidence_id, "capability evidence ID")?;
    Ok(())
}

fn validate_case_shape(case: &QualificationCase) -> Result<(), AuditError> {
    for (v, label) in [
        (case.case_id.as_str(), "case ID"),
        (case.source_pack_id.as_str(), "source pack ID"),
        (case.pack_revision.as_str(), "pack revision"),
        (case.passport_ref.as_str(), "battery passport reference"),
        (case.passport_evidence_id.as_str(), "passport evidence ID"),
        (case.bms.hardware_revision.as_str(), "BMS hardware revision"),
        (case.bms.firmware_revision.as_str(), "BMS firmware revision"),
        (case.bms.verification_ref.as_str(), "BMS verification reference"),
    ] {
        require_nonempty(v, label)?;
    }
    validate_digest(&case.configuration_digest, "pack configuration digest")?;
    validate_digest(&case.cell_membership_digest, "cell membership digest")?;
    if case.expected_cell_count == 0 {
        return Err(AuditError("expected cell count must be nonzero".into()));
    }
    validate_unique(case.observed_cell_ids.iter().map(String::as_str), "cell ID")?;
    if case.observed_cell_ids.len() != case.expected_cell_count {
        return Err(AuditError(format!(
            "case {} cell membership mismatch: expected {}, observed {}",
            case.case_id,
            case.expected_cell_count,
            case.observed_cell_ids.len()
        )));
    }
    validate_mission(&case.mission)?;
    validate_capability(&case.capability)?;
    if case.thermal_review_status == ThermalReviewStatus::AcceptedByCompetentAuthority {
        require_nonempty(
            case.thermal_authority_decision_ref.as_deref().unwrap_or(""),
            "competent-authority thermal decision reference",
        )?;
    }
    Ok(())
}

fn gate_map<'a>(
    gates: &'a [GateAssessment],
    registry: &BTreeMap<&str, &EvidenceRecord>,
    case_id: &str,
    mode: EvaluationMode,
) -> Result<BTreeMap<HardGate, &'a GateAssessment>, AuditError> {
    if gates.len() != HardGate::ALL.len() {
        return Err(AuditError(format!(
            "case {case_id} must declare all ten hard gates exactly once"
        )));
    }
    let mut map = BTreeMap::new();
    for g in gates {
        require_nonempty(&g.rationale, "hard-gate rationale")?;
        if map.insert(g.gate, g).is_some() {
            return Err(AuditError(format!("case {case_id} repeats hard gate {:?}", g.gate)));
        }
        let evidence = admitted(registry, &g.evidence_id, &format!("case {case_id} gate {:?}", g.gate))?;
        if mode == EvaluationMode::EvidenceReview {
            if matches!(evidence.evidence_class, EvidenceClass::Simulation | EvidenceClass::Synthetic) {
                return Err(AuditError(format!(
                    "case {case_id} cannot use {:?} evidence for a real evidence review",
                    evidence.evidence_class
                )));
            }
            if !gate_allowed(g.gate, evidence.evidence_class) {
                return Err(AuditError(format!(
                    "case {case_id} evidence class {:?} is not allowed for gate {:?}",
                    evidence.evidence_class, g.gate
                )));
            }
        }
    }
    for gate in HardGate::ALL {
        if !map.contains_key(&gate) {
            return Err(AuditError(format!("case {case_id} is missing hard gate {gate:?}")));
        }
    }
    Ok(map)
}

fn milli(value: f64) -> i64 {
    (value * 1000.0).round() as i64
}

fn evaluate_case(
    case: &QualificationCase,
    registry: &BTreeMap<&str, &EvidenceRecord>,
    mode: EvaluationMode,
) -> Result<CaseReport, AuditError> {
    validate_case_shape(case)?;
    let gates = gate_map(&case.gates, registry, &case.case_id, mode)?;
    let mut blockers = Vec::new();
    let mut gates_passed = 0;

    let passport_gate = gates.get(&HardGate::PackIdentity).expect("identity gate exists");
    if case.passport_evidence_id != passport_gate.evidence_id {
        return Err(AuditError(format!(
            "case {} passport evidence ID does not match the pack-identity gate evidence",
            case.case_id
        )));
    }
    let capability_gate = gates.get(&HardGate::MissionCapability).expect("capability gate exists");
    if case.capability.evidence_id != capability_gate.evidence_id {
        return Err(AuditError(format!(
            "case {} capability evidence ID does not match the mission-capability gate evidence",
            case.case_id
        )));
    }
    let bms_gate = gates.get(&HardGate::BmsCompatibility).expect("BMS gate exists");
    if case.bms.verification_ref != bms_gate.evidence_id {
        return Err(AuditError(format!(
            "case {} BMS verification reference does not match the BMS gate evidence",
            case.case_id
        )));
    }
    let thermal_gate = gates
        .get(&HardGate::ThermalPropagationApproval)
        .expect("thermal gate exists");
    if case.thermal_review_status == ThermalReviewStatus::AcceptedByCompetentAuthority
        && case.thermal_authority_decision_ref.as_deref() != Some(thermal_gate.evidence_id.as_str())
    {
        return Err(AuditError(format!(
            "case {} thermal authority decision reference does not match the thermal gate evidence",
            case.case_id
        )));
    }
    let history_gate = gates.get(&HardGate::HistoryIntegrity).expect("history gate exists");
    if case.incident_history == IncidentHistory::AdverseEventsPresentAndAdjudicated
        && case.incident_adjudication_ref.as_deref() != Some(history_gate.evidence_id.as_str())
    {
        return Err(AuditError(format!(
            "case {} adverse-event adjudication reference does not match history-integrity evidence",
            case.case_id
        )));
    }
    if mode == EvaluationMode::EvidenceReview {
        let profile = admitted(
            registry,
            &case.capability.uncertainty_profile_ref,
            &format!("case {} uncertainty profile", case.case_id),
        )?;
        if matches!(profile.evidence_class, EvidenceClass::Synthetic | EvidenceClass::Simulation) {
            return Err(AuditError(format!(
                "case {} cannot use synthetic/simulation uncertainty profile in evidence review",
                case.case_id
            )));
        }
    }

    for gate in HardGate::ALL {
        let assessment = gates.get(&gate).expect("all gate members validated");
        if assessment.status == GateStatus::Pass {
            gates_passed += 1;
        } else {
            blockers.push(format!("{gate:?} is {:?}", assessment.status));
        }
    }

    if case.history_coverage != HistoryCoverage::CompleteSinceFirstCommissioning {
        blockers.push(format!("history coverage is {:?}", case.history_coverage));
    }
    if case.incident_history == IncidentHistory::Unknown {
        blockers.push("incident history is UNKNOWN".into());
    }
    if case.thermal_review_status != ThermalReviewStatus::AcceptedByCompetentAuthority {
        blockers.push(format!(
            "thermal review status is {:?}, not accepted by competent authority",
            case.thermal_review_status
        ));
    }
    if case.bms.compatibility != BmsCompatibility::VerifiedForTargetMission {
        blockers.push(format!("BMS compatibility is {:?}", case.bms.compatibility));
    }

    let reserve_factor = 1.0 + case.mission.reserve_percent / 100.0;
    let commissioning_energy_required =
        case.mission.required_usable_energy_at_commissioning_kwh * reserve_factor;
    let horizon_energy_required =
        case.mission.required_usable_energy_at_horizon_kwh * reserve_factor;
    if case.capability.lower_bound_usable_energy_at_commissioning_kwh
        < commissioning_energy_required
    {
        blockers.push("lower-bound commissioning energy does not cover mission plus reserve".into());
    }
    if case.capability.lower_bound_usable_energy_at_horizon_kwh < horizon_energy_required {
        blockers.push("lower-bound end-of-horizon energy does not cover mission plus reserve".into());
    }
    if case.capability.lower_bound_lifetime_delivered_energy_mwh
        < case.mission.required_lifetime_delivered_energy_mwh
    {
        blockers.push("lower-bound lifetime delivered energy is below mission requirement".into());
    }
    if case.capability.lower_bound_peak_power_kw < case.mission.required_peak_power_kw {
        blockers.push("lower-bound peak power is below mission requirement".into());
    }
    if case.capability.supported_temperature_min_c > case.mission.temperature_min_c
        || case.capability.supported_temperature_max_c < case.mission.temperature_max_c
    {
        blockers.push("capability temperature envelope does not cover the mission".into());
    }
    if case.capability.lower_bound_availability_percent
        < case.mission.minimum_availability_percent
    {
        blockers.push("lower-bound availability is below mission requirement".into());
    }

    blockers.sort();
    let disposition = if mode == EvaluationMode::SyntheticKnownAnswer && blockers.is_empty() {
        Disposition::SyntheticFixtureOnly
    } else if !blockers.is_empty() {
        Disposition::AbstainInsufficientEvidence
    } else {
        match case.requested_use {
            RequestedUse::SecondLifeStationary => Disposition::EligibleForSecondLifeMission,
            RequestedUse::ThirdLifeBackup => Disposition::EligibleForThirdLifeBackup,
        }
    };
    Ok(CaseReport {
        case_id: case.case_id.clone(),
        source_pack_id: case.source_pack_id.clone(),
        disposition,
        blockers,
        gates_passed,
        gates_total: HardGate::ALL.len(),
        commissioning_energy_margin_milli_kwh: milli(
            case.capability.lower_bound_usable_energy_at_commissioning_kwh
                - commissioning_energy_required,
        ),
        horizon_energy_margin_milli_kwh: milli(
            case.capability.lower_bound_usable_energy_at_horizon_kwh - horizon_energy_required,
        ),
        lifetime_delivered_energy_margin_milli_mwh: milli(
            case.capability.lower_bound_lifetime_delivered_energy_mwh
                - case.mission.required_lifetime_delivered_energy_mwh,
        ),
        mission_power_margin_milli_kw: milli(
            case.capability.lower_bound_peak_power_kw - case.mission.required_peak_power_kw,
        ),
        claim_ceiling: "Internal evidence/mission eligibility only; not safety certification, regulatory compliance, commissioning approval or warranty.".into(),
    })
}

pub fn audit_manifest(manifest: &Manifest) -> Result<AuditReport, AuditError> {
    require_nonempty(&manifest.campaign_id, "campaign ID")?;
    if manifest.cases.is_empty() {
        return Err(AuditError("manifest requires at least one qualification case".into()));
    }
    let registry = evidence_registry(&manifest.evidence)?;
    validate_unique(manifest.cases.iter().map(|c| c.case_id.as_str()), "case ID")?;
    let mut reports = Vec::new();
    let mut eligible_second_life_count = 0;
    let mut eligible_third_life_count = 0;
    let mut abstain_count = 0;
    let mut synthetic_fixture_count = 0;
    for case in &manifest.cases {
        let report = evaluate_case(case, &registry, manifest.evaluation_mode)?;
        match report.disposition {
            Disposition::EligibleForSecondLifeMission => eligible_second_life_count += 1,
            Disposition::EligibleForThirdLifeBackup => eligible_third_life_count += 1,
            Disposition::AbstainInsufficientEvidence => abstain_count += 1,
            Disposition::SyntheticFixtureOnly => synthetic_fixture_count += 1,
        }
        reports.push(report);
    }
    reports.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    Ok(AuditReport {
        campaign_id: manifest.campaign_id.clone(),
        case_count: manifest.cases.len(),
        eligible_second_life_count,
        eligible_third_life_count,
        abstain_count,
        synthetic_fixture_count,
        cases: reports,
        claim_ceiling: "Evidence identity, history, hard-gate and mission-envelope consistency only. No physical safety, legal, commissioning, lifetime or economic determination.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn evidence(id: &str, ch: char) -> EvidenceRecord {
        EvidenceRecord {
            evidence_id: id.into(),
            source_digest: digest(ch),
            evidence_class: EvidenceClass::Synthetic,
            admission: Admission::AdmittedForEvaluation,
            claim_scope: "synthetic known-answer fixture".into(),
            rights_ref: "SYNTHETIC-ONLY-NO-PRODUCTION-DATA".into(),
            metrology_ref: None,
        }
    }

    fn gates() -> Vec<GateAssessment> {
        HardGate::ALL
            .into_iter()
            .enumerate()
            .map(|(i, gate)| GateAssessment {
                gate,
                status: GateStatus::Pass,
                evidence_id: format!("E-GATE-{i}"),
                rationale: "synthetic fixture gate".into(),
            })
            .collect()
    }

    fn case(id: &str, requested_use: RequestedUse) -> QualificationCase {
        QualificationCase {
            case_id: id.into(),
            source_pack_id: format!("PACK-{id}"),
            pack_revision: "PACK-R3".into(),
            passport_ref: format!("PASSPORT-{id}"),
            passport_evidence_id: "E-GATE-0".into(),
            configuration_digest: digest('a'),
            cell_membership_digest: digest('b'),
            expected_cell_count: 4,
            observed_cell_ids: vec![
                "CELL-1".into(),
                "CELL-2".into(),
                "CELL-3".into(),
                "CELL-4".into(),
            ],
            history_coverage: HistoryCoverage::CompleteSinceFirstCommissioning,
            incident_history: IncidentHistory::NoAdverseEventsRecordedInCompleteHistory,
            incident_adjudication_ref: None,
            requested_use,
            mission: MissionRequirement {
                horizon_months: 120,
                required_usable_energy_at_commissioning_kwh: 80.0,
                required_usable_energy_at_horizon_kwh: 64.0,
                required_lifetime_delivered_energy_mwh: 450.0,
                required_peak_power_kw: 40.0,
                temperature_min_c: -10.0,
                temperature_max_c: 40.0,
                minimum_availability_percent: 97.0,
                reserve_percent: 10.0,
            },
            capability: CapabilityEnvelope {
                lower_bound_usable_energy_at_commissioning_kwh: 100.0,
                lower_bound_usable_energy_at_horizon_kwh: 90.0,
                lower_bound_lifetime_delivered_energy_mwh: 500.0,
                lower_bound_peak_power_kw: 50.0,
                supported_temperature_min_c: -20.0,
                supported_temperature_max_c: 45.0,
                lower_bound_availability_percent: 98.0,
                uncertainty_profile_ref: "E-UQ-PROFILE".into(),
                evidence_id: format!("E-CAP-{id}"),
            },
            bms: BmsConfiguration {
                strategy: BmsStrategy::GatewayInterface,
                hardware_revision: "BMS-HW-R2".into(),
                firmware_revision: "BMS-FW-R7".into(),
                compatibility: BmsCompatibility::VerifiedForTargetMission,
                verification_ref: format!("BMS-VERIFY-{id}"),
            },
            thermal_review_status: ThermalReviewStatus::AcceptedByCompetentAuthority,
            thermal_authority_decision_ref: Some(format!("AUTH-THERMAL-{id}")),
            gates: {
                let mut values = gates();
                values.iter_mut()
                    .find(|g| g.gate == HardGate::MissionCapability)
                    .unwrap()
                    .evidence_id = format!("E-CAP-{id}");
                values.iter_mut()
                    .find(|g| g.gate == HardGate::BmsCompatibility)
                    .unwrap()
                    .evidence_id = format!("BMS-VERIFY-{id}");
                values.iter_mut()
                    .find(|g| g.gate == HardGate::ThermalPropagationApproval)
                    .unwrap()
                    .evidence_id = format!("AUTH-THERMAL-{id}");
                values
            },
        }
    }

    fn fixture() -> Manifest {
        let mut ev = vec![];
        for i in 0..10 {
            ev.push(evidence(&format!("E-GATE-{i}"), char::from(b'0' + i as u8)));
        }
        for id in ["SECOND-LIFE", "THIRD-LIFE", "UNKNOWN-HISTORY"] {
            ev.push(evidence(&format!("E-CAP-{id}"), 'a'));
            ev.push(evidence(&format!("BMS-VERIFY-{id}"), 'b'));
            ev.push(evidence(&format!("AUTH-THERMAL-{id}"), 'c'));
        }
        ev.push(evidence("E-UQ-PROFILE", 'd'));
        let second = case("SECOND-LIFE", RequestedUse::SecondLifeStationary);
        let third = case("THIRD-LIFE", RequestedUse::ThirdLifeBackup);
        let mut unknown = case("UNKNOWN-HISTORY", RequestedUse::SecondLifeStationary);
        unknown.history_coverage = HistoryCoverage::Partial;
        unknown.incident_history = IncidentHistory::Unknown;
        unknown.thermal_review_status = ThermalReviewStatus::CandidateForHumanReview;
        unknown.gates.iter_mut()
            .find(|g| g.gate == HardGate::HistoryIntegrity)
            .unwrap()
            .status = GateStatus::Unknown;
        unknown.gates.iter_mut()
            .find(|g| g.gate == HardGate::ThermalPropagationApproval)
            .unwrap()
            .status = GateStatus::Unknown;
        Manifest {
            campaign_id: "SECOND-LIFE-KNOWN-ANSWER-001".into(),
            evaluation_mode: EvaluationMode::SyntheticKnownAnswer,
            evidence: ev,
            cases: vec![second, third, unknown],
        }
    }

    #[test]
    fn synthetic_candidate_never_becomes_real_eligible() {
        let r = audit_manifest(&fixture()).expect("synthetic fixture valid");
        assert_eq!(r.synthetic_fixture_count, 2);
        assert_eq!(r.eligible_second_life_count, 0);
        assert_eq!(r.eligible_third_life_count, 0);
    }

    #[test]
    fn partial_history_and_candidate_thermal_review_abstain() {
        let r = audit_manifest(&fixture()).expect("synthetic fixture valid");
        let c = r.cases.iter().find(|c| c.case_id == "UNKNOWN-HISTORY").unwrap();
        assert_eq!(c.disposition, Disposition::AbstainInsufficientEvidence);
        assert!(c.blockers.iter().any(|b| b.contains("history coverage")));
        assert!(c.blockers.iter().any(|b| b.contains("thermal review status")));
    }

    #[test]
    fn incomplete_cell_membership_is_rejected() {
        let mut m = fixture();
        m.cases[0].observed_cell_ids.pop();
        assert!(audit_manifest(&m).is_err());
    }

    #[test]
    fn duplicate_cell_identity_is_rejected() {
        let mut m = fixture();
        m.cases[0].observed_cell_ids[1] = "CELL-1".into();
        assert!(audit_manifest(&m).is_err());
    }

    #[test]
    fn missing_hard_gate_is_rejected() {
        let mut m = fixture();
        m.cases[0].gates.pop();
        assert!(audit_manifest(&m).is_err());
    }

    #[test]
    fn duplicate_hard_gate_is_rejected() {
        let mut m = fixture();
        m.cases[0].gates[1].gate = m.cases[0].gates[0].gate;
        assert!(audit_manifest(&m).is_err());
    }

    #[test]
    fn hard_gate_fail_or_unknown_abstains() {
        let mut m = fixture();
        m.cases[0].gates[0].status = GateStatus::Fail;
        let r = audit_manifest(&m).expect("recorded failure is valid");
        assert_eq!(r.cases[1].disposition, Disposition::AbstainInsufficientEvidence);
    }

    #[test]
    fn mission_energy_plus_reserve_must_be_met() {
        let mut m = fixture();
        m.cases[0].capability.lower_bound_usable_energy_at_commissioning_kwh = 87.9;
        let r = audit_manifest(&m).expect("valid measured declaration");
        assert!(r.cases.iter().find(|c| c.case_id == "SECOND-LIFE").unwrap().blockers
            .iter().any(|b| b.contains("energy")));
    }

    #[test]
    fn end_of_horizon_energy_and_lifetime_service_are_independent_gates() {
        let mut m = fixture();
        m.cases[0].capability.lower_bound_usable_energy_at_horizon_kwh = 70.0;
        let report = audit_manifest(&m).expect("bounded low-horizon capacity is a blocker");
        assert!(
            report
                .cases
                .iter()
                .find(|c| c.case_id == "SECOND-LIFE")
                .unwrap()
                .blockers
                .iter()
                .any(|b| b.contains("end-of-horizon"))
        );

        let mut m = fixture();
        m.cases[0].capability.lower_bound_lifetime_delivered_energy_mwh = 400.0;
        let report = audit_manifest(&m).expect("bounded low lifetime throughput is a blocker");
        assert!(
            report
                .cases
                .iter()
                .find(|c| c.case_id == "SECOND-LIFE")
                .unwrap()
                .blockers
                .iter()
                .any(|b| b.contains("lifetime delivered energy"))
        );
    }

    #[test]
    fn temperature_envelope_must_cover_mission() {
        let mut m = fixture();
        m.cases[0].capability.supported_temperature_min_c = 0.0;
        let r = audit_manifest(&m).expect("valid bounded envelope");
        assert!(r.cases.iter().find(|c| c.case_id == "SECOND-LIFE").unwrap().blockers
            .iter().any(|b| b.contains("temperature")));
    }

    #[test]
    fn bms_must_be_verified_for_target_mission() {
        let mut m = fixture();
        m.cases[0].bms.compatibility = BmsCompatibility::Unknown;
        let r = audit_manifest(&m).expect("unknown compatibility is a valid blocker");
        assert!(r.cases.iter().find(|c| c.case_id == "SECOND-LIFE").unwrap().blockers
            .iter().any(|b| b.contains("BMS compatibility")));
    }

    #[test]
    fn adverse_event_history_requires_adjudication_reference_to_exact_history_evidence() {
        let mut m = fixture();
        m.cases[0].incident_history = IncidentHistory::AdverseEventsPresentAndAdjudicated;
        assert!(audit_manifest(&m).is_err());
        let mut m = fixture();
        m.cases[0].incident_history = IncidentHistory::AdverseEventsPresentAndAdjudicated;
        m.cases[0].incident_adjudication_ref = Some("E-GATE-2".into());
        let r = audit_manifest(&m).expect("exact adjudication evidence is bound");
        assert_eq!(r.synthetic_fixture_count, 3);
    }

    #[test]
    fn capability_bms_and_thermal_refs_must_match_gate_evidence() {
        let mut m = fixture();
        m.cases[0].capability.evidence_id = "OTHER-CAPABILITY".into();
        assert!(audit_manifest(&m).is_err());
        let mut m = fixture();
        m.cases[0].bms.verification_ref = "STALE-BMS".into();
        assert!(audit_manifest(&m).is_err());
        let mut m = fixture();
        m.cases[0].thermal_authority_decision_ref = Some("STALE-THERMAL-REVIEW".into());
        assert!(audit_manifest(&m).is_err());
    }

    #[test]
    fn uncertainty_profile_must_be_admitted() {
        let mut m = fixture();
        m.cases[0].capability.uncertainty_profile_ref = "MISSING-UQ".into();
        assert!(audit_manifest(&m).is_err());
    }

    #[test]
    fn thermal_gate_requires_authority_decision_evidence_in_real_mode() {
        let mut m = fixture();
        m.evaluation_mode = EvaluationMode::EvidenceReview;
        for e in &mut m.evidence {
            e.evidence_class = EvidenceClass::IndependentLaboratory;
            e.metrology_ref = Some("CALIBRATED".into());
        }
        let r = audit_manifest(&m);
        assert!(r.is_err());
    }

    #[test]
    fn admitted_real_evidence_can_support_second_and_third_life_review() {
        let mut m = fixture();
        m.evaluation_mode = EvaluationMode::EvidenceReview;
        for e in &mut m.evidence {
            e.rights_ref = "LICENSE-DECLARATION-001".into();
            let class = if e.evidence_id == "E-GATE-0"
                || e.evidence_id == "E-GATE-2"
                || e.evidence_id == "E-GATE-7"
                || e.evidence_id == "E-GATE-9"
            {
                EvidenceClass::Documentary
            } else if e.evidence_id.starts_with("AUTH-THERMAL-") {
                EvidenceClass::AuthorityDecision
            } else if e.evidence_id.starts_with("BMS-VERIFY-")
                || e.evidence_id == "E-UQ-PROFILE"
            {
                EvidenceClass::IndependentLaboratory
            } else {
                EvidenceClass::PhysicalMeasurement
            };
            e.evidence_class = class;
            if matches!(
                class,
                EvidenceClass::PhysicalMeasurement | EvidenceClass::IndependentLaboratory
            ) {
                e.metrology_ref = Some(format!("MET-{}", e.evidence_id));
            }
        }
        let r = audit_manifest(&m).expect("admitted evidence review should audit");
        assert_eq!(r.eligible_second_life_count, 1);
        assert_eq!(r.eligible_third_life_count, 1);
        assert_eq!(r.abstain_count, 1);
        assert_eq!(
            r.cases
                .iter()
                .find(|c| c.case_id == "SECOND-LIFE")
                .unwrap()
                .disposition,
            Disposition::EligibleForSecondLifeMission
        );
        assert_eq!(
            r.cases
                .iter()
                .find(|c| c.case_id == "THIRD-LIFE")
                .unwrap()
                .disposition,
            Disposition::EligibleForThirdLifeBackup
        );
    }

    #[test]
    fn rights_reference_is_mandatory() {
        let mut m = fixture();
        m.evidence[0].rights_ref.clear();
        assert!(audit_manifest(&m).is_err());
    }

    #[test]
    fn unknown_evidence_admission_is_rejected() {
        let mut m = fixture();
        m.evidence[0].admission = Admission::Unknown;
        assert!(audit_manifest(&m).is_err());
    }

    #[test]
    fn input_permutation_preserves_report() {
        let m = fixture();
        let expected = audit_manifest(&m).expect("fixture valid");
        let mut p = m;
        p.evidence.reverse();
        p.cases.reverse();
        for c in &mut p.cases {
            c.gates.reverse();
        }
        assert_eq!(expected, audit_manifest(&p).expect("permuted fixture valid"));
    }
}
