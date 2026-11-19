use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FluidClass {
    StorableLiquid,
    CryogenicLiquid,
    HighPressureOrSupercritical,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    StorageOnly,
    TransferDemonstration,
    OperationalService,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Liquid,
    Gas,
    Supercritical,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioStatus {
    TransferAccepted,
    StorageOnlyNotTransferEvidence,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceProfile {
    pub required_phase: Phase,
    pub minimum_temperature_mk: u64,
    pub maximum_temperature_mk: u64,
    pub minimum_pressure_pa: u64,
    pub maximum_pressure_pa: u64,
    pub maximum_measurement_uncertainty_g: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub scenario_id: String,
    pub transfer_event_id: String,
    pub fluid_class: FluidClass,
    pub fluid_id: String,
    pub receiver_id: String,
    pub interface_id: String,
    pub interface_revision: u64,
    pub evidence_state: EvidenceState,
    pub source_initial_g: u64,
    pub source_remaining_g: u64,
    pub receiver_accepted_g: u64,
    pub receiver_rejected_g: u64,
    pub line_residual_g: u64,
    pub vented_g: u64,
    pub leaked_g: u64,
    pub measurement_uncertainty_g: u64,
    pub phase: Phase,
    pub final_temperature_mk: u64,
    pub final_pressure_pa: u64,
    pub acceptance: AcceptanceProfile,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub suite_id: String,
    pub suite_version: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScenarioReport {
    pub scenario_id: String,
    pub transfer_event_id: String,
    pub status: ScenarioStatus,
    pub fluid_class: FluidClass,
    pub fluid_id: String,
    pub receiver_id: String,
    pub interface_id: String,
    pub interface_revision: u64,
    pub phase: Phase,
    pub final_temperature_mk: u64,
    pub final_pressure_pa: u64,
    pub measurement_uncertainty_g: u64,
    pub source_initial_g: u64,
    pub source_remaining_g: u64,
    pub gross_transferred_g: u64,
    pub receiver_accepted_g: u64,
    pub receiver_rejected_g: u64,
    pub line_residual_g: u64,
    pub vented_g: u64,
    pub leaked_g: u64,
    pub mass_balance_residual_g: u64,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Report {
    pub suite_id: String,
    pub suite_version: String,
    pub evidence_class: String,
    pub result: String,
    pub scenarios: Vec<ScenarioReport>,
    pub claim_ceiling: String,
}

fn checked_add(total: u64, value: u64, label: &str) -> Result<u64, String> {
    total.checked_add(value).ok_or_else(|| format!("{label}: integer overflow"))
}

fn non_empty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label}: must not be empty"))
    } else {
        Ok(())
    }
}

fn phase_allowed(class: FluidClass, phase: Phase) -> bool {
    match class {
        FluidClass::StorableLiquid | FluidClass::CryogenicLiquid => phase == Phase::Liquid,
        FluidClass::HighPressureOrSupercritical => {
            phase == Phase::Gas || phase == Phase::Supercritical
        }
    }
}

fn validate_acceptance_profile(profile: &AcceptanceProfile) -> Result<(), String> {
    if profile.minimum_temperature_mk == 0
        || profile.minimum_temperature_mk > profile.maximum_temperature_mk
        || profile.minimum_pressure_pa == 0
        || profile.minimum_pressure_pa > profile.maximum_pressure_pa
    {
        return Err("invalid temperature or pressure acceptance interval".to_owned());
    }
    Ok(())
}

fn evaluate_scenario(scenario: &Scenario) -> Result<ScenarioReport, String> {
    non_empty(&scenario.scenario_id, "scenario_id")?;
    non_empty(&scenario.transfer_event_id, "transfer_event_id")?;
    non_empty(&scenario.fluid_id, "fluid_id")?;
    non_empty(&scenario.receiver_id, "receiver_id")?;
    non_empty(&scenario.interface_id, "interface_id")?;
    if scenario.interface_revision == 0 {
        return Err(format!("{}: interface revision must be nonzero", scenario.scenario_id));
    }
    if scenario.evidence_refs.is_empty() {
        return Err(format!("{}: no evidence references", scenario.scenario_id));
    }
    let mut refs = BTreeSet::new();
    for reference in &scenario.evidence_refs {
        non_empty(reference, "evidence reference")?;
        if !refs.insert(reference.as_str()) {
            return Err(format!("{}: duplicate evidence reference", scenario.scenario_id));
        }
    }

    validate_acceptance_profile(&scenario.acceptance)?;
    if !phase_allowed(scenario.fluid_class, scenario.phase)
        || !phase_allowed(scenario.fluid_class, scenario.acceptance.required_phase)
    {
        return Err(format!(
            "{}: phase is incompatible with declared fluid class",
            scenario.scenario_id
        ));
    }

    if scenario.measurement_uncertainty_g
        > scenario.acceptance.maximum_measurement_uncertainty_g
    {
        return Err(format!(
            "{}: metrology uncertainty exceeds acceptance limit",
            scenario.scenario_id
        ));
    }

    let gross_transferred_g = scenario
        .source_initial_g
        .checked_sub(scenario.source_remaining_g)
        .ok_or_else(|| format!("{}: source remaining exceeds initial inventory", scenario.scenario_id))?;

    let accounted_outputs = [
        scenario.receiver_accepted_g,
        scenario.receiver_rejected_g,
        scenario.line_residual_g,
        scenario.vented_g,
        scenario.leaked_g,
    ]
    .into_iter()
    .try_fold(0_u64, |sum, value| checked_add(sum, value, "transfer output streams"))?;

    if accounted_outputs != gross_transferred_g {
        return Err(format!(
            "{}: mass balance mismatch; source loss {} g, accounted output streams {} g",
            scenario.scenario_id, gross_transferred_g, accounted_outputs
        ));
    }

    if scenario.evidence_state == EvidenceState::OperationalService
        && scenario.evidence_class == "synthetic"
    {
        return Err(format!(
            "{}: synthetic fixture cannot claim operational-service evidence",
            scenario.scenario_id
        ));
    }

    if scenario.evidence_state == EvidenceState::StorageOnly {
        if gross_transferred_g != 0
            || scenario.receiver_accepted_g != 0
            || scenario.receiver_rejected_g != 0
        {
            return Err(format!(
                "{}: storage-only evidence cannot support a transfer claim",
                scenario.scenario_id
            ));
        }
        return Ok(ScenarioReport {
            scenario_id: scenario.scenario_id.clone(),
            transfer_event_id: scenario.transfer_event_id.clone(),
            status: ScenarioStatus::StorageOnlyNotTransferEvidence,
            fluid_class: scenario.fluid_class,
            fluid_id: scenario.fluid_id.clone(),
            receiver_id: scenario.receiver_id.clone(),
            interface_id: scenario.interface_id.clone(),
            interface_revision: scenario.interface_revision,
            phase: scenario.phase,
            final_temperature_mk: scenario.final_temperature_mk,
            final_pressure_pa: scenario.final_pressure_pa,
            measurement_uncertainty_g: scenario.measurement_uncertainty_g,
            source_initial_g: scenario.source_initial_g,
            source_remaining_g: scenario.source_remaining_g,
            gross_transferred_g,
            receiver_accepted_g: scenario.receiver_accepted_g,
            receiver_rejected_g: scenario.receiver_rejected_g,
            line_residual_g: scenario.line_residual_g,
            vented_g: scenario.vented_g,
            leaked_g: scenario.leaked_g,
            mass_balance_residual_g: 0,
            claim_ceiling:
                "Storage evidence only; no transfer, delivery or receiver-acceptance claim is established.",
        });
    }

    if scenario.evidence_state != EvidenceState::TransferDemonstration
        && scenario.evidence_state != EvidenceState::OperationalService
    {
        return Err(format!(
            "{}: unsupported evidence state for a transfer",
            scenario.scenario_id
        ));
    }
    if gross_transferred_g == 0 || scenario.receiver_accepted_g == 0 {
        return Err(format!(
            "{}: transfer demonstration requires positive gross and accepted receiver mass",
            scenario.scenario_id
        ));
    }
    if scenario.phase != scenario.acceptance.required_phase {
        return Err(format!(
            "{}: receiver phase does not match required phase",
            scenario.scenario_id
        ));
    }
    if scenario.final_temperature_mk < scenario.acceptance.minimum_temperature_mk
        || scenario.final_temperature_mk > scenario.acceptance.maximum_temperature_mk
    {
        return Err(format!(
            "{}: final temperature outside receiver acceptance interval",
            scenario.scenario_id
        ));
    }
    if scenario.final_pressure_pa < scenario.acceptance.minimum_pressure_pa
        || scenario.final_pressure_pa > scenario.acceptance.maximum_pressure_pa
    {
        return Err(format!(
            "{}: final pressure outside receiver acceptance interval",
            scenario.scenario_id
        ));
    }

    Ok(ScenarioReport {
        scenario_id: scenario.scenario_id.clone(),
        transfer_event_id: scenario.transfer_event_id.clone(),
        status: ScenarioStatus::TransferAccepted,
        fluid_class: scenario.fluid_class,
        fluid_id: scenario.fluid_id.clone(),
        receiver_id: scenario.receiver_id.clone(),
        interface_id: scenario.interface_id.clone(),
        interface_revision: scenario.interface_revision,
        phase: scenario.phase,
        final_temperature_mk: scenario.final_temperature_mk,
        final_pressure_pa: scenario.final_pressure_pa,
        measurement_uncertainty_g: scenario.measurement_uncertainty_g,
        source_initial_g: scenario.source_initial_g,
        source_remaining_g: scenario.source_remaining_g,
        gross_transferred_g,
        receiver_accepted_g: scenario.receiver_accepted_g,
        receiver_rejected_g: scenario.receiver_rejected_g,
        line_residual_g: scenario.line_residual_g,
        vented_g: scenario.vented_g,
        leaked_g: scenario.leaked_g,
        mass_balance_residual_g: 0,
        claim_ceiling:
            "Synthetic transfer accounting and receiver acceptance contract only; no fluid dynamics, hardware or flight-service claim.",
    })
}

pub fn evaluate(fixture: &Fixture) -> Result<Report, String> {
    if fixture.suite_id != "orbital-fluid-transfer" || fixture.suite_version != "0.1.0" {
        return Err("unsupported suite identity or version".to_owned());
    }
    if fixture.evidence_class != "synthetic" {
        return Err("v0.1 reference fixture must be explicitly synthetic".to_owned());
    }
    non_empty(&fixture.claim_ceiling, "claim_ceiling")?;
    if fixture.scenarios.is_empty() {
        return Err("fixture must contain at least one scenario".to_owned());
    }

    let mut scenario_ids = BTreeSet::new();
    let mut event_ids = BTreeSet::new();
    let mut reports = Vec::with_capacity(fixture.scenarios.len());
    for scenario in &fixture.scenarios {
        if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            return Err(format!("duplicate scenario ID: {}", scenario.scenario_id));
        }
        if !event_ids.insert(scenario.transfer_event_id.as_str()) {
            return Err(format!("duplicate transfer event ID: {}", scenario.transfer_event_id));
        }
        reports.push(evaluate_scenario(scenario)?);
    }

    Ok(Report {
        suite_id: fixture.suite_id.clone(),
        suite_version: fixture.suite_version.clone(),
        evidence_class: fixture.evidence_class.clone(),
        result: "pass".to_owned(),
        scenarios: reports,
        claim_ceiling: fixture.claim_ceiling.clone(),
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(bytes);
    let mut hexadecimal = String::with_capacity(64);
    for byte in digest {
        write!(&mut hexadecimal, "{byte:02x}").expect("writing to String is infallible");
    }
    hexadecimal
}

pub fn valid_source_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../fixtures/v0.1/reference.json"))
            .expect("frozen fluid transfer fixture must parse")
    }

    #[test]
    fn frozen_fixture_separates_transfer_from_storage() {
        let report = evaluate(&fixture()).expect("fixture should pass");
        assert_eq!(report.scenarios.len(), 3);
        assert_eq!(report.scenarios[0].status, ScenarioStatus::TransferAccepted);
        assert_eq!(report.scenarios[0].gross_transferred_g, 400_000);
        assert_eq!(report.scenarios[0].receiver_accepted_g, 390_000);
        assert_eq!(report.scenarios[1].status, ScenarioStatus::StorageOnlyNotTransferEvidence);
        assert_eq!(report.scenarios[2].status, ScenarioStatus::TransferAccepted);
    }

    #[test]
    fn transfer_report_preserves_fluid_identity_and_final_receiver_state() {
        let report = evaluate(&fixture()).expect("fixture should pass");
        let transfer = &report.scenarios[0];
        assert_eq!(transfer.transfer_event_id, "transfer-storable-001");
        assert_eq!(transfer.fluid_class, FluidClass::StorableLiquid);
        assert_eq!(transfer.fluid_id, "synthetic-hydrazine-analogue");
        assert_eq!(transfer.phase, Phase::Liquid);
        assert_eq!(transfer.final_temperature_mk, 293_150);
        assert_eq!(transfer.final_pressure_pa, 250_000);
        assert_eq!(transfer.measurement_uncertainty_g, 1_000);
    }

    #[test]
    fn accepted_transfer_result_carries_receiver_and_prepared_interface_identity() {
        let report = evaluate(&fixture()).expect("fixture should pass");
        let transfer = &report.scenarios[0];
        assert_eq!(transfer.receiver_id, "serviced-client-001");
        assert_eq!(transfer.interface_id, "COOPERATIVE-CAPTURE-PORT-A");
        assert_eq!(transfer.interface_revision, 3);
    }

    #[test]
    fn storage_only_result_preserves_identity_without_claiming_transfer() {
        let report = evaluate(&fixture()).expect("fixture should pass");
        let storage = &report.scenarios[1];
        assert_eq!(storage.status, ScenarioStatus::StorageOnlyNotTransferEvidence);
        assert_eq!(storage.receiver_id, "serviced-client-001");
        assert_eq!(storage.interface_revision, 3);
    }

    #[test]
    fn missing_receiver_or_interface_identity_is_rejected() {
        let mut data = fixture();
        data.scenarios[0].receiver_id.clear();
        assert!(evaluate(&data).unwrap_err().contains("receiver_id"));
        let mut data = fixture();
        data.scenarios[0].interface_revision = 0;
        assert!(evaluate(&data).unwrap_err().contains("interface revision"));
    }

    #[test]
    fn storable_mass_balance_is_exact() {
        let report = evaluate(&fixture()).expect("fixture should pass");
        assert_eq!(report.scenarios[0].mass_balance_residual_g, 0);
        assert_eq!(report.scenarios[0].gross_transferred_g, 400_000);
    }

    #[test]
    fn cryogenic_storage_only_label_alone_cannot_create_transfer() {
        let mut data = fixture();
        data.scenarios[1].evidence_state = EvidenceState::TransferDemonstration;
        assert!(
            evaluate(&data)
                .unwrap_err()
                .contains("transfer demonstration requires positive gross")
        );
    }

    #[test]
    fn storage_only_cannot_support_a_transfer_even_if_mass_is_balanced() {
        let mut data = fixture();
        let scenario = &mut data.scenarios[1];
        scenario.source_remaining_g -= 100;
        scenario.receiver_accepted_g = 100;
        assert!(evaluate(&data).unwrap_err().contains("storage-only evidence"));
    }

    #[test]
    fn reject_mass_imbalance_without_rounding_it_away() {
        let mut data = fixture();
        data.scenarios[0].receiver_accepted_g += 1;
        assert!(evaluate(&data).unwrap_err().contains("mass balance mismatch"));
    }

    #[test]
    fn reject_high_pressure_fluid_declared_as_liquid() {
        let mut data = fixture();
        data.scenarios[2].phase = Phase::Liquid;
        assert!(evaluate(&data).unwrap_err().contains("phase is incompatible"));
    }

    #[test]
    fn reject_receiver_temperature_outside_the_declared_envelope() {
        let mut data = fixture();
        data.scenarios[0].final_temperature_mk = 400_000;
        assert!(evaluate(&data).unwrap_err().contains("final temperature outside"));
    }

    #[test]
    fn reject_receiver_pressure_outside_the_declared_envelope() {
        let mut data = fixture();
        data.scenarios[0].final_pressure_pa = 700_000;
        assert!(evaluate(&data).unwrap_err().contains("final pressure outside"));
    }

    #[test]
    fn missing_meter_evidence_is_rejected() {
        let mut data = fixture();
        data.scenarios[0].evidence_refs.clear();
        assert!(evaluate(&data).unwrap_err().contains("no evidence references"));
    }

    #[test]
    fn duplicate_transfer_event_is_rejected() {
        let mut data = fixture();
        data.scenarios[2].transfer_event_id = data.scenarios[0].transfer_event_id.clone();
        assert!(evaluate(&data).unwrap_err().contains("duplicate transfer event ID"));
    }

    #[test]
    fn operational_service_cannot_be_claimed_by_synthetic_fixture() {
        let mut data = fixture();
        data.scenarios[0].evidence_state = EvidenceState::OperationalService;
        assert!(evaluate(&data).unwrap_err().contains("synthetic fixture cannot claim"));
    }

    #[test]
    fn source_commit_must_be_a_full_sha() {
        assert!(valid_source_commit("0123456789abcdef0123456789abcdef01234567"));
        assert!(!valid_source_commit("main"));
    }
}
