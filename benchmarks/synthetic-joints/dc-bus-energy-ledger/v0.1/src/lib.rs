#![forbid(unsafe_code)]

//! Multi-control-volume regenerative energy accounting.
//!
//! M1 accepts already-integrated signed energy intervals. Each interval must
//! include an externally validated quadrature/integration error bound and
//! traceable measurement provenance. This crate does not integrate raw power,
//! estimate battery SOC, or prove that a physical system is safe.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundRepresentation {
    ValidatedBound,
    StandardUncertainty,
    ExpandedUncertainty,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationStatus {
    InDate,
    Expired,
    OutOfTolerance,
    Unknown,
    NotCalibrated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceabilityStatus {
    Established,
    Partial,
    NotEstablished,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    IndependentPhysical,
    SourceAuthorPhysical,
    VendorPhysical,
    ModelDerived,
    SyntheticFixture,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceKind {
    ActuatorModule,
    SharedDcBus,
    StorageSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceDisposition {
    ConsistentWithDeclaredBounds,
    InconsistentWithDeclaredBounds,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntegratedEnergyTerm {
    /// Exact port name used by the selected balance equation.
    pub port_id: String,
    /// Signed energy interval before quadrature-error expansion, in joules.
    pub energy_j: NumericInterval,
    pub unit: String,
    pub bound_representation: BoundRepresentation,
    /// Externally justified absolute quadrature/integration error bound.
    pub integration_error_bound_j: Option<f64>,
    pub integration_error_source_reference: String,
    pub integration_error_digest: String,
    pub integration_method: String,
    pub integration_contract_digest: String,

    pub start_time_s: f64,
    pub end_time_s: f64,
    pub time_alignment_digest: String,
    pub mission_interval_digest: String,
    pub system_boundary_digest: String,
    pub operating_condition_digest: String,

    pub calibration_status: CalibrationStatus,
    pub traceability_status: TraceabilityStatus,
    pub evidence_class: EvidenceClass,
    pub source_reference: String,
    pub measurement_method: String,
    pub calibration_digest: String,
    pub raw_trace_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyTermReceipt {
    pub port_id: String,
    pub raw_energy_j: NumericInterval,
    pub integration_error_bound_j: f64,
    pub qualified_energy_j: NumericInterval,
    pub integration_method: String,
    pub integration_contract_digest: String,
    pub integration_error_digest: String,
    pub integration_error_source_reference: String,
    pub evidence_class: EvidenceClass,
    pub source_reference: String,
    pub measurement_method: String,
    pub calibration_digest: String,
    pub raw_trace_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyBalanceReport {
    pub kind: BalanceKind,
    pub disposition: BalanceDisposition,
    pub residual_j: Option<NumericInterval>,
    pub residual_interval_width_j: Option<f64>,
    pub maximum_absolute_residual_j: Option<f64>,
    pub maximum_absolute_term_energy_j: Option<f64>,
    pub relative_residual_bound: Option<f64>,
    pub zero_is_contained: Option<bool>,
    pub system_boundary_digest: Option<String>,
    pub mission_interval_digest: Option<String>,
    pub operating_condition_digest: Option<String>,
    pub time_alignment_digest: Option<String>,
    pub term_receipts: Vec<EnergyTermReceipt>,
    pub reason: Option<String>,
    pub interpretation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryCouplingDisposition {
    ConsistentWithDeclaredBounds,
    InconsistentWithDeclaredBounds,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryCouplingReport {
    pub from_balance: BalanceKind,
    pub from_port: String,
    pub to_balance: BalanceKind,
    pub to_port: String,
    /// Difference interval: energy at the upstream boundary minus energy at
    /// the downstream boundary, after both integration-error expansions.
    pub difference_j: Option<NumericInterval>,
    pub difference_interval_width_j: Option<f64>,
    pub maximum_absolute_difference_j: Option<f64>,
    pub disposition: BoundaryCouplingDisposition,
    pub from_evidence_class: Option<EvidenceClass>,
    pub to_evidence_class: Option<EvidenceClass>,
    pub from_source_reference: Option<String>,
    pub to_source_reference: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompleteEnergyLedgerReport {
    pub actuator_module: EnergyBalanceReport,
    pub shared_dc_bus: EnergyBalanceReport,
    pub storage_system: EnergyBalanceReport,
    pub actuator_to_bus: BoundaryCouplingReport,
    pub bus_to_storage: BoundaryCouplingReport,
    pub overall_disposition: BalanceDisposition,
    pub interpretation: String,
}

#[derive(Debug, Clone, Copy)]
struct PortTermSpec {
    name: &'static str,
    coefficient: i8,
    nonnegative: bool,
}

#[derive(Debug, Clone, Copy)]
struct BalanceSpec {
    kind: BalanceKind,
    ports: &'static [PortTermSpec],
}

const ACTUATOR_PORTS: [PortTermSpec; 4] = [
    PortTermSpec { name: "dc_terminal_in", coefficient: 1, nonnegative: false },
    PortTermSpec { name: "mechanical_output", coefficient: -1, nonnegative: false },
    PortTermSpec { name: "thermal_dissipated", coefficient: -1, nonnegative: true },
    PortTermSpec { name: "delta_internal_stored", coefficient: -1, nonnegative: false },
];

const BUS_PORTS: [PortTermSpec; 6] = [
    PortTermSpec { name: "external_source_in", coefficient: 1, nonnegative: false },
    PortTermSpec { name: "drive_draw_signed", coefficient: -1, nonnegative: false },
    PortTermSpec { name: "storage_draw_signed", coefficient: -1, nonnegative: false },
    PortTermSpec { name: "other_loads", coefficient: -1, nonnegative: true },
    PortTermSpec { name: "brake_dissipated", coefficient: -1, nonnegative: true },
    PortTermSpec { name: "delta_bus_cap", coefficient: -1, nonnegative: false },
];

const STORAGE_PORTS: [PortTermSpec; 3] = [
    PortTermSpec { name: "storage_terminal_in", coefficient: 1, nonnegative: false },
    PortTermSpec { name: "delta_stored_energy", coefficient: -1, nonnegative: false },
    PortTermSpec { name: "storage_losses", coefficient: -1, nonnegative: true },
];

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite_interval(value: NumericInterval) -> bool {
    value.lower.is_finite() && value.upper.is_finite() && value.lower <= value.upper
}

fn unknown_report(
    kind: BalanceKind,
    reason: impl Into<String>,
    term_receipts: Vec<EnergyTermReceipt>,
) -> EnergyBalanceReport {
    EnergyBalanceReport {
        kind,
        disposition: BalanceDisposition::Unknown,
        residual_j: None,
        residual_interval_width_j: None,
        maximum_absolute_residual_j: None,
        maximum_absolute_term_energy_j: None,
        relative_residual_bound: None,
        zero_is_contained: None,
        system_boundary_digest: None,
        mission_interval_digest: None,
        operating_condition_digest: None,
        time_alignment_digest: None,
        term_receipts,
        reason: Some(reason.into()),
        interpretation: "Energy balance is Unknown. Missing evidence is not zero and no regeneration/storage claim is made.".to_owned(),
    }
}

fn validate_and_expand_term(
    term: &IntegratedEnergyTerm,
    nonnegative: bool,
) -> Result<Result<EnergyTermReceipt, String>, String> {
    if !finite_interval(term.energy_j) {
        return Err(format!("port {} has a non-finite or inverted energy interval", term.port_id));
    }
    if nonnegative && term.energy_j.upper < 0.0 {
        return Err(format!("port {} is defined as nonnegative but its interval is entirely negative", term.port_id));
    }
    if term.unit != "J" {
        return Err(format!("port {} unit {:?} is not the required joule unit", term.port_id, term.unit));
    }
    if !term.start_time_s.is_finite()
        || !term.end_time_s.is_finite()
        || term.end_time_s <= term.start_time_s
    {
        return Err(format!("port {} has an invalid mission interval", term.port_id));
    }
    if term.bound_representation != BoundRepresentation::ValidatedBound {
        return Ok(Err(format!(
            "port {} energy interval is not an externally validated deterministic bound",
            term.port_id
        )));
    }
    if term.calibration_status != CalibrationStatus::InDate {
        return Ok(Err(format!(
            "port {} calibration status is {:?}",
            term.port_id, term.calibration_status
        )));
    }
    if term.traceability_status != TraceabilityStatus::Established {
        return Ok(Err(format!(
            "port {} traceability status is {:?}",
            term.port_id, term.traceability_status
        )));
    }
    if term.evidence_class == EvidenceClass::Unknown {
        return Ok(Err(format!("port {} evidence class is Unknown", term.port_id)));
    }

    let Some(integration_error_bound_j) = term.integration_error_bound_j else {
        return Ok(Err(format!("port {} has no integration/quadrature error bound", term.port_id)));
    };
    if !integration_error_bound_j.is_finite() || integration_error_bound_j < 0.0 {
        return Err(format!("port {} has an invalid integration error bound", term.port_id));
    }

    let required_strings = [
        term.port_id.as_str(),
        term.integration_error_source_reference.as_str(),
        term.integration_error_digest.as_str(),
        term.integration_method.as_str(),
        term.integration_contract_digest.as_str(),
        term.time_alignment_digest.as_str(),
        term.mission_interval_digest.as_str(),
        term.system_boundary_digest.as_str(),
        term.operating_condition_digest.as_str(),
        term.source_reference.as_str(),
        term.measurement_method.as_str(),
        term.calibration_digest.as_str(),
        term.raw_trace_digest.as_str(),
    ];
    if required_strings.iter().any(|value| !nonempty(value)) {
        return Ok(Err("one or more energy-term provenance fields are missing".to_owned()));
    }

    let expanded_lower = term.energy_j.lower - integration_error_bound_j;
    let expanded_upper = term.energy_j.upper + integration_error_bound_j;
    let qualified_energy_j = NumericInterval {
        // Dissipative/load terms have a known nonnegative physical domain;
        // intersect the symmetric integration expansion with that domain.
        lower: if nonnegative { expanded_lower.max(0.0) } else { expanded_lower },
        upper: expanded_upper,
    };
    if !finite_interval(qualified_energy_j) {
        return Err(format!("port {} integration expansion overflowed", term.port_id));
    }

    Ok(Ok(EnergyTermReceipt {
        port_id: term.port_id.clone(),
        raw_energy_j: term.energy_j,
        integration_error_bound_j,
        qualified_energy_j,
        integration_method: term.integration_method.clone(),
        integration_contract_digest: term.integration_contract_digest.clone(),
        integration_error_digest: term.integration_error_digest.clone(),
        integration_error_source_reference: term.integration_error_source_reference.clone(),
        evidence_class: term.evidence_class,
        source_reference: term.source_reference.clone(),
        measurement_method: term.measurement_method.clone(),
        calibration_digest: term.calibration_digest.clone(),
        raw_trace_digest: term.raw_trace_digest.clone(),
    }))
}

fn signed_interval(interval: NumericInterval, coefficient: i8) -> NumericInterval {
    if coefficient >= 0 {
        interval
    } else {
        NumericInterval {
            lower: -interval.upper,
            upper: -interval.lower,
        }
    }
}

fn assess(
    spec: BalanceSpec,
    terms: &[IntegratedEnergyTerm],
) -> Result<EnergyBalanceReport, String> {
    let mut by_port = BTreeMap::new();
    for term in terms {
        if by_port.insert(term.port_id.as_str(), term).is_some() {
            return Err(format!("duplicate energy port {:?}", term.port_id));
        }
    }

    for term in terms {
        if !spec.ports.iter().any(|expected| expected.name == term.port_id) {
            return Ok(unknown_report(
                spec.kind,
                format!("unexpected/unaccounted energy port {:?}", term.port_id),
                Vec::new(),
            ));
        }
    }

    for expected in spec.ports {
        if !by_port.contains_key(expected.name) {
            return Ok(unknown_report(
                spec.kind,
                format!("required energy port {:?} is missing", expected.name),
                Vec::new(),
            ));
        }
    }

    let mut receipts = Vec::with_capacity(spec.ports.len());
    for expected in spec.ports {
        let term = by_port.get(expected.name).expect("required port checked");
        match validate_and_expand_term(term, expected.nonnegative)? {
            Ok(receipt) => receipts.push(receipt),
            Err(reason) => return Ok(unknown_report(spec.kind, reason, receipts)),
        }
    }

    let first = by_port.get(spec.ports[0].name).expect("required first port");
    for expected in spec.ports.iter().skip(1) {
        let term = by_port.get(expected.name).expect("required port checked");
        if term.system_boundary_digest != first.system_boundary_digest
            || term.mission_interval_digest != first.mission_interval_digest
            || term.operating_condition_digest != first.operating_condition_digest
            || term.time_alignment_digest != first.time_alignment_digest
            || term.start_time_s != first.start_time_s
            || term.end_time_s != first.end_time_s
        {
            return Ok(unknown_report(
                spec.kind,
                format!(
                    "port {:?} does not share the control-volume boundary, mission interval, conditions and time alignment",
                    expected.name
                ),
                receipts,
            ));
        }
    }

    let mut residual_lower = 0.0;
    let mut residual_upper = 0.0;
    let mut maximum_absolute_term_energy_j = 0.0_f64;

    for expected in spec.ports {
        let receipt = receipts.iter().find(|item| item.port_id == expected.name).unwrap();
        let contribution = signed_interval(receipt.qualified_energy_j, expected.coefficient);
        residual_lower += contribution.lower;
        residual_upper += contribution.upper;
        maximum_absolute_term_energy_j = maximum_absolute_term_energy_j
            .max(receipt.qualified_energy_j.lower.abs())
            .max(receipt.qualified_energy_j.upper.abs());
    }

    let residual_j = NumericInterval {
        lower: residual_lower,
        upper: residual_upper,
    };
    if !finite_interval(residual_j) {
        return Err("energy balance residual is non-finite".to_owned());
    }
    let zero_is_contained = residual_j.lower <= 0.0 && residual_j.upper >= 0.0;
    let maximum_absolute_residual_j = residual_j.lower.abs().max(residual_j.upper.abs());
    let residual_interval_width_j = residual_j.upper - residual_j.lower;
    let relative_residual_bound = if maximum_absolute_term_energy_j == 0.0 {
        None
    } else {
        Some(maximum_absolute_residual_j / maximum_absolute_term_energy_j)
    };
    if !residual_interval_width_j.is_finite()
        || relative_residual_bound.is_some_and(|value| !value.is_finite())
    {
        return Err("energy residual width or normalized residual is non-finite".to_owned());
    }

    Ok(EnergyBalanceReport {
        kind: spec.kind,
        disposition: if zero_is_contained {
            BalanceDisposition::ConsistentWithDeclaredBounds
        } else {
            BalanceDisposition::InconsistentWithDeclaredBounds
        },
        residual_j: Some(residual_j),
        residual_interval_width_j: Some(residual_interval_width_j),
        maximum_absolute_residual_j: Some(maximum_absolute_residual_j),
        maximum_absolute_term_energy_j: Some(maximum_absolute_term_energy_j),
        relative_residual_bound,
        zero_is_contained: Some(zero_is_contained),
        system_boundary_digest: Some(first.system_boundary_digest.clone()),
        mission_interval_digest: Some(first.mission_interval_digest.clone()),
        operating_condition_digest: Some(first.operating_condition_digest.clone()),
        time_alignment_digest: Some(first.time_alignment_digest.clone()),
        term_receipts: receipts,
        reason: None,
        interpretation: "Zero containment means the signed energy balance is consistent with the declared deterministic intervals and integration bounds; it does not prove the model, instrumentation or physical recovery path is correct. A wide residual interval may be uninformative.".to_owned(),
    })
}

/// Actuator-module balance:
/// E_dc_terminal_in - E_mechanical_output - E_thermal_dissipated - Delta_E_internal.
pub fn assess_actuator_module_balance(
    terms: &[IntegratedEnergyTerm],
) -> Result<EnergyBalanceReport, String> {
    assess(BalanceSpec { kind: BalanceKind::ActuatorModule, ports: &ACTUATOR_PORTS }, terms)
}

/// Shared DC bus balance:
/// E_source_in - E_drive_draw_signed - E_storage_draw_signed - E_other_loads
/// - E_brake_dissipated - Delta_E_bus_cap.
pub fn assess_shared_dc_bus_balance(
    terms: &[IntegratedEnergyTerm],
) -> Result<EnergyBalanceReport, String> {
    assess(BalanceSpec { kind: BalanceKind::SharedDcBus, ports: &BUS_PORTS }, terms)
}

/// Storage-system balance:
/// E_storage_terminal_in - Delta_E_stored - E_storage_losses.
pub fn assess_storage_balance(
    terms: &[IntegratedEnergyTerm],
) -> Result<EnergyBalanceReport, String> {
    assess(BalanceSpec { kind: BalanceKind::StorageSystem, ports: &STORAGE_PORTS }, terms)
}

fn coupling(
    from: &EnergyBalanceReport,
    from_port: &str,
    to: &EnergyBalanceReport,
    to_port: &str,
) -> BoundaryCouplingReport {
    let mut result = BoundaryCouplingReport {
        from_balance: from.kind,
        from_port: from_port.to_owned(),
        to_balance: to.kind,
        to_port: to_port.to_owned(),
        difference_j: None,
        difference_interval_width_j: None,
        maximum_absolute_difference_j: None,
        disposition: BoundaryCouplingDisposition::Unknown,
        from_evidence_class: None,
        to_evidence_class: None,
        from_source_reference: None,
        to_source_reference: None,
        reason: None,
    };

    if from.disposition == BalanceDisposition::Unknown
        || to.disposition == BalanceDisposition::Unknown
    {
        result.reason = Some("one or both control-volume balances are Unknown".to_owned());
        return result;
    }
    if from.mission_interval_digest != to.mission_interval_digest
        || from.operating_condition_digest != to.operating_condition_digest
        || from.time_alignment_digest != to.time_alignment_digest
    {
        result.reason = Some("boundary coupling records do not share mission interval, operating conditions and time alignment".to_owned());
        return result;
    }

    let Some(from_term) = from.term_receipts.iter().find(|term| term.port_id == from_port) else {
        result.reason = Some(format!("upstream port {from_port:?} is missing from its receipt"));
        return result;
    };
    let Some(to_term) = to.term_receipts.iter().find(|term| term.port_id == to_port) else {
        result.reason = Some(format!("downstream port {to_port:?} is missing from its receipt"));
        return result;
    };

    let difference = NumericInterval {
        lower: from_term.qualified_energy_j.lower - to_term.qualified_energy_j.upper,
        upper: from_term.qualified_energy_j.upper - to_term.qualified_energy_j.lower,
    };
    if !finite_interval(difference) {
        result.reason = Some("boundary-coupling residual is non-finite or inverted".to_owned());
        return result;
    }
    let width = difference.upper - difference.lower;
    let max_abs = difference.lower.abs().max(difference.upper.abs());
    if !width.is_finite() || !max_abs.is_finite() {
        result.reason = Some("boundary-coupling width or magnitude is non-finite".to_owned());
        return result;
    }
    result.difference_j = Some(difference);
    result.difference_interval_width_j = Some(width);
    result.maximum_absolute_difference_j = Some(max_abs);
    result.from_evidence_class = Some(from_term.evidence_class);
    result.to_evidence_class = Some(to_term.evidence_class);
    result.from_source_reference = Some(from_term.source_reference.clone());
    result.to_source_reference = Some(to_term.source_reference.clone());
    result.disposition = if difference.lower <= 0.0 && difference.upper >= 0.0 {
        BoundaryCouplingDisposition::ConsistentWithDeclaredBounds
    } else {
        BoundaryCouplingDisposition::InconsistentWithDeclaredBounds
    };
    result
}

/// Evaluates all three control volumes and checks their shared physical ports.
///
/// The actuator module's signed DC-terminal energy must reconcile with the
/// corresponding drive port on the shared bus. The shared-bus storage port
/// must reconcile with the storage terminal. Both checks use already-expanded
/// intervals and require identical mission/condition/time-alignment digests.
/// System-boundary digests intentionally differ because each control volume
/// is a different boundary.
pub fn assess_complete_energy_ledger(
    actuator_terms: &[IntegratedEnergyTerm],
    bus_terms: &[IntegratedEnergyTerm],
    storage_terms: &[IntegratedEnergyTerm],
) -> Result<CompleteEnergyLedgerReport, String> {
    let actuator_module = assess_actuator_module_balance(actuator_terms)?;
    let shared_dc_bus = assess_shared_dc_bus_balance(bus_terms)?;
    let storage_system = assess_storage_balance(storage_terms)?;

    let actuator_to_bus = coupling(
        &actuator_module,
        "dc_terminal_in",
        &shared_dc_bus,
        "drive_draw_signed",
    );
    let bus_to_storage = coupling(
        &shared_dc_bus,
        "storage_draw_signed",
        &storage_system,
        "storage_terminal_in",
    );

    let component_dispositions = [
        actuator_module.disposition,
        shared_dc_bus.disposition,
        storage_system.disposition,
    ];
    let coupling_dispositions = [actuator_to_bus.disposition, bus_to_storage.disposition];
    let all_consistent = component_dispositions
        .iter()
        .all(|value| *value == BalanceDisposition::ConsistentWithDeclaredBounds)
        && coupling_dispositions.iter().all(|value| {
            *value == BoundaryCouplingDisposition::ConsistentWithDeclaredBounds
        });
    let any_inconsistent = component_dispositions
        .iter()
        .any(|value| *value == BalanceDisposition::InconsistentWithDeclaredBounds)
        || coupling_dispositions.iter().any(|value| {
            *value == BoundaryCouplingDisposition::InconsistentWithDeclaredBounds
        });
    let overall_disposition = if all_consistent {
        BalanceDisposition::ConsistentWithDeclaredBounds
    } else if any_inconsistent {
        BalanceDisposition::InconsistentWithDeclaredBounds
    } else {
        BalanceDisposition::Unknown
    };

    Ok(CompleteEnergyLedgerReport {
        actuator_module,
        shared_dc_bus,
        storage_system,
        actuator_to_bus,
        bus_to_storage,
        overall_disposition,
        interpretation: "The complete ledger checks each control-volume balance and both inter-boundary energy couplings. Consistency with declared intervals is not proof of a correct physical model, calibrated instrument, electrical regeneration path or safe system.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDARY: &str = "declared-control-volume-v1";
    const MISSION: &str = "mission-interval-001";
    const CONDITIONS: &str = "48V-25C-cooling-A";
    const TIME_ALIGNMENT: &str = "common-clock-map-v1";

    fn term(port: &str, energy: f64) -> IntegratedEnergyTerm {
        IntegratedEnergyTerm {
            port_id: port.to_owned(),
            energy_j: NumericInterval { lower: energy, upper: energy },
            unit: "J".to_owned(),
            bound_representation: BoundRepresentation::ValidatedBound,
            integration_error_bound_j: Some(0.01),
            integration_error_source_reference: "https://example.invalid/integration-bound".to_owned(),
            integration_error_digest: format!("quad-{port}-v1"),
            integration_method: "Trapezoidal integration with validated residual bound".to_owned(),
            integration_contract_digest: "integration-contract-v1".to_owned(),
            start_time_s: 0.0,
            end_time_s: 1.0,
            time_alignment_digest: TIME_ALIGNMENT.to_owned(),
            mission_interval_digest: MISSION.to_owned(),
            system_boundary_digest: BOUNDARY.to_owned(),
            operating_condition_digest: CONDITIONS.to_owned(),
            calibration_status: CalibrationStatus::InDate,
            traceability_status: TraceabilityStatus::Established,
            evidence_class: EvidenceClass::SyntheticFixture,
            source_reference: format!("https://example.invalid/source/{port}"),
            measurement_method: "Synthetic known-answer integrated-energy fixture".to_owned(),
            calibration_digest: format!("cal-{port}-v1"),
            raw_trace_digest: format!("raw-{port}-v1"),
        }
    }

    fn actuator_motoring() -> Vec<IntegratedEnergyTerm> {
        vec![
            term("dc_terminal_in", 100.0),
            term("mechanical_output", 80.0),
            term("thermal_dissipated", 15.0),
            term("delta_internal_stored", 5.0),
        ]
    }

    fn actuator_regeneration() -> Vec<IntegratedEnergyTerm> {
        vec![
            term("dc_terminal_in", -60.0),
            term("mechanical_output", -90.0),
            term("thermal_dissipated", 20.0),
            term("delta_internal_stored", 10.0),
        ]
    }

    fn bus_flow() -> Vec<IntegratedEnergyTerm> {
        vec![
            term("external_source_in", 10.0),
            term("drive_draw_signed", -40.0),
            term("storage_draw_signed", 20.0),
            term("other_loads", 15.0),
            term("brake_dissipated", 15.0),
            term("delta_bus_cap", 0.0),
        ]
    }

    fn storage_flow() -> Vec<IntegratedEnergyTerm> {
        vec![
            term("storage_terminal_in", 20.0),
            term("delta_stored_energy", 18.0),
            term("storage_losses", 2.0),
        ]
    }

    fn close(value: f64, expected: f64) {
        assert!((value - expected).abs() < 1.0e-9, "actual={value}, expected={expected}");
    }

    #[test]
    fn ledger_001_actuator_motoring_balance_is_consistent() {
        let report = assess_actuator_module_balance(&actuator_motoring()).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::ConsistentWithDeclaredBounds);
        assert!(report.zero_is_contained.unwrap());
        assert!(report.residual_j.unwrap().lower < 0.0);
        assert!(report.residual_j.unwrap().upper > 0.0);
    }

    #[test]
    fn ledger_002_actuator_overhauling_energy_is_separate_from_dc_return() {
        let report = assess_actuator_module_balance(&actuator_regeneration()).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::ConsistentWithDeclaredBounds);
        let dc = report.term_receipts.iter().find(|term| term.port_id == "dc_terminal_in").unwrap();
        let mechanical = report.term_receipts.iter().find(|term| term.port_id == "mechanical_output").unwrap();
        assert!(dc.qualified_energy_j.upper < 0.0);
        assert!(mechanical.qualified_energy_j.upper < 0.0);
    }

    #[test]
    fn ledger_003_shared_bus_accounts_for_reuse_storage_and_braking() {
        let report = assess_shared_dc_bus_balance(&bus_flow()).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::ConsistentWithDeclaredBounds);
        close(report.maximum_absolute_term_energy_j.unwrap(), 40.01);
    }

    #[test]
    fn ledger_004_storage_terminal_energy_closes_against_internal_storage_change_and_loss() {
        let report = assess_storage_balance(&storage_flow()).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::ConsistentWithDeclaredBounds);
        assert_eq!(report.term_receipts.len(), 3);
    }

    #[test]
    fn ledger_005_unbalanced_energy_interval_excludes_zero() {
        let mut terms = actuator_motoring();
        terms[1].energy_j = NumericInterval { lower: 60.0, upper: 60.0 };
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::InconsistentWithDeclaredBounds);
        assert!(!report.zero_is_contained.unwrap());
        assert!(report.residual_j.unwrap().lower > 0.0);
    }

    #[test]
    fn ledger_006_quadrature_bounds_expand_each_energy_interval() {
        let mut terms = actuator_motoring();
        terms[0].integration_error_bound_j = Some(0.5);
        let report = assess_actuator_module_balance(&terms).unwrap();
        let dc = report.term_receipts.iter().find(|term| term.port_id == "dc_terminal_in").unwrap();
        assert_eq!(dc.qualified_energy_j, NumericInterval { lower: 99.5, upper: 100.5 });
        assert_eq!(report.disposition, BalanceDisposition::ConsistentWithDeclaredBounds);
    }

    #[test]
    fn ledger_007_missing_quadrature_bound_is_unknown() {
        let mut terms = actuator_motoring();
        terms[0].integration_error_bound_j = None;
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);
        assert!(report.reason.unwrap().contains("no integration/quadrature error bound"));
    }

    #[test]
    fn ledger_008_standard_uncertainty_is_not_a_deterministic_energy_bound() {
        let mut terms = actuator_motoring();
        terms[0].bound_representation = BoundRepresentation::StandardUncertainty;
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);
    }

    #[test]
    fn ledger_009_expired_calibration_is_unknown() {
        let mut terms = actuator_motoring();
        terms[0].calibration_status = CalibrationStatus::Expired;
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);
    }

    #[test]
    fn ledger_010_unestablished_traceability_is_unknown() {
        let mut terms = actuator_motoring();
        terms[0].traceability_status = TraceabilityStatus::Partial;
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);
    }

    #[test]
    fn ledger_011_missing_port_is_unknown_not_zero() {
        let mut terms = actuator_motoring();
        terms.pop();
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);
        assert!(report.reason.unwrap().contains("missing"));
    }

    #[test]
    fn ledger_012_unmodelled_extra_port_is_unknown() {
        let mut terms = actuator_motoring();
        terms.push(term("unmetered_auxiliary", 1.0));
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);
        assert!(report.reason.unwrap().contains("unexpected/unaccounted"));
    }

    #[test]
    fn ledger_013_duplicate_port_rejects() {
        let mut terms = actuator_motoring();
        terms.push(terms[0].clone());
        assert!(assess_actuator_module_balance(&terms).is_err());
    }

    #[test]
    fn ledger_014_wrong_energy_unit_rejects() {
        let mut terms = actuator_motoring();
        terms[0].unit = "Wh".to_owned();
        assert!(assess_actuator_module_balance(&terms).is_err());
    }

    #[test]
    fn ledger_015_mission_boundary_and_time_alignment_must_match() {
        let mut terms = actuator_motoring();
        terms[1].mission_interval_digest = "different-mission-interval".to_owned();
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);

        let mut terms = actuator_motoring();
        terms[1].time_alignment_digest = "different-clock-map".to_owned();
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);
    }

    #[test]
    fn ledger_016_operating_condition_mismatch_is_unknown() {
        let mut terms = actuator_motoring();
        terms[1].operating_condition_digest = "different-cooling".to_owned();
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);
    }

    #[test]
    fn ledger_017_nonfinite_or_inverted_energy_intervals_reject() {
        let mut terms = actuator_motoring();
        terms[0].energy_j = NumericInterval { lower: 10.0, upper: 5.0 };
        assert!(assess_actuator_module_balance(&terms).is_err());

        let mut terms = actuator_motoring();
        terms[0].energy_j.lower = f64::NAN;
        assert!(assess_actuator_module_balance(&terms).is_err());
    }

    #[test]
    fn ledger_018_calibration_or_raw_trace_receipts_survive_balance() {
        let report = assess_actuator_module_balance(&actuator_motoring()).unwrap();
        let dc = report.term_receipts.iter().find(|term| term.port_id == "dc_terminal_in").unwrap();
        assert_eq!(dc.calibration_digest, "cal-dc_terminal_in-v1");
        assert_eq!(dc.raw_trace_digest, "raw-dc_terminal_in-v1");
        assert!(dc.integration_error_source_reference.starts_with("https://"));
        assert!(dc.integration_contract_digest == "integration-contract-v1");
    }

    #[test]
    fn ledger_019_zero_energy_terms_do_not_create_a_fake_relative_residual() {
        let mut terms = actuator_motoring();
        for term in &mut terms {
            term.energy_j = NumericInterval { lower: 0.0, upper: 0.0 };
            term.integration_error_bound_j = Some(0.0);
        }
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::ConsistentWithDeclaredBounds);
        assert_eq!(report.relative_residual_bound, None);
    }

    #[test]
    fn ledger_020_quadrature_expansion_overflow_rejects() {
        let mut terms = actuator_motoring();
        terms[0].integration_error_bound_j = Some(f64::MAX);
        terms[0].energy_j = NumericInterval { lower: 1.0, upper: 2.0 };
        assert!(assess_actuator_module_balance(&terms).is_err());
    }

    #[test]
    fn ledger_021_dissipative_and_load_terms_cannot_be_negative() {
        let mut terms = actuator_motoring();
        terms[2].energy_j = NumericInterval { lower: -1.0, upper: -0.5 };
        assert!(assess_actuator_module_balance(&terms).is_err());

        let mut bus = bus_flow();
        bus[4].energy_j = NumericInterval { lower: -2.0, upper: -1.0 };
        assert!(assess_shared_dc_bus_balance(&bus).is_err());

        let mut storage = storage_flow();
        storage[2].energy_j = NumericInterval { lower: -1.0, upper: -0.5 };
        assert!(assess_storage_balance(&storage).is_err());
    }

    #[test]
    fn ledger_022_uncertainty_expansion_respects_nonnegative_loss_domain() {
        let mut terms = actuator_motoring();
        terms[2].energy_j = NumericInterval { lower: 0.0, upper: 0.0 };
        terms[2].integration_error_bound_j = Some(0.5);
        let report = assess_actuator_module_balance(&terms).unwrap();
        let heat = report.term_receipts.iter().find(|term| term.port_id == "thermal_dissipated").unwrap();
        assert_eq!(heat.qualified_energy_j.lower, 0.0);
        assert_eq!(heat.qualified_energy_j.upper, 0.5);
    }

    fn complete_motoring_case() -> (Vec<IntegratedEnergyTerm>, Vec<IntegratedEnergyTerm>, Vec<IntegratedEnergyTerm>) {
        let mut actuator = actuator_motoring();
        actuator[0].energy_j = NumericInterval { lower: 100.0, upper: 100.0 };
        let bus = vec![
            term("external_source_in", 10.0),
            term("drive_draw_signed", 100.0),
            term("storage_draw_signed", 20.0),
            term("other_loads", 15.0),
            term("brake_dissipated", 15.0),
            term("delta_bus_cap", 0.0),
        ];
        let storage = storage_flow();
        (actuator, bus, storage)
    }

    #[test]
    fn ledger_023_boundary_couplings_reconcile_for_a_complete_mission() {
        let (actuator, bus, storage) = complete_motoring_case();
        let report = assess_complete_energy_ledger(&actuator, &bus, &storage).unwrap();
        assert_eq!(report.overall_disposition, BalanceDisposition::ConsistentWithDeclaredBounds);
        assert_eq!(report.actuator_to_bus.disposition, BoundaryCouplingDisposition::ConsistentWithDeclaredBounds);
        assert_eq!(report.bus_to_storage.disposition, BoundaryCouplingDisposition::ConsistentWithDeclaredBounds);
    }

    #[test]
    fn ledger_024_drive_energy_mismatch_is_a_boundary_coupling_violation() {
        let (actuator, mut bus, storage) = complete_motoring_case();
        bus[1].energy_j = NumericInterval { lower: 90.0, upper: 90.0 };
        let report = assess_complete_energy_ledger(&actuator, &bus, &storage).unwrap();
        assert_eq!(report.actuator_module.disposition, BalanceDisposition::ConsistentWithDeclaredBounds);
        assert_eq!(report.shared_dc_bus.disposition, BalanceDisposition::ConsistentWithDeclaredBounds);
        assert_eq!(report.actuator_to_bus.disposition, BoundaryCouplingDisposition::InconsistentWithDeclaredBounds);
        assert_eq!(report.overall_disposition, BalanceDisposition::InconsistentWithDeclaredBounds);
    }

    #[test]
    fn ledger_025_storage_terminal_mismatch_is_a_boundary_coupling_violation() {
        let (actuator, bus, mut storage) = complete_motoring_case();
        storage[0].energy_j = NumericInterval { lower: 18.0, upper: 18.0 };
        let report = assess_complete_energy_ledger(&actuator, &bus, &storage).unwrap();
        assert_eq!(report.storage_system.disposition, BalanceDisposition::InconsistentWithDeclaredBounds);
        assert_eq!(report.bus_to_storage.disposition, BoundaryCouplingDisposition::InconsistentWithDeclaredBounds);
        assert_eq!(report.overall_disposition, BalanceDisposition::InconsistentWithDeclaredBounds);
    }

    #[test]
    fn ledger_026_boundary_couplings_reject_mismatched_mission_clock() {
        let (actuator, mut bus, storage) = complete_motoring_case();
        bus[1].time_alignment_digest = "different-clock".to_owned();
        // Restore internal bus consistency while retaining the different
        // alignment from the actuator boundary.
        for term in &mut bus {
            term.time_alignment_digest = "different-clock".to_owned();
        }
        let report = assess_complete_energy_ledger(&actuator, &bus, &storage).unwrap();
        assert_eq!(report.actuator_to_bus.disposition, BoundaryCouplingDisposition::Unknown);
        assert_eq!(report.overall_disposition, BalanceDisposition::Unknown);
    }

    #[test]
    fn ledger_027_unknown_evidence_is_unknown_not_a_balance_pass() {
        let mut terms = actuator_motoring();
        terms[0].evidence_class = EvidenceClass::Unknown;
        let report = assess_actuator_module_balance(&terms).unwrap();
        assert_eq!(report.disposition, BalanceDisposition::Unknown);
    }
}
