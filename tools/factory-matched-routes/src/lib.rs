// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic matched-route benchmark for Symtropy FACTORY-MATCHED-ROUTES-001.
//!
//! This crate evaluates sequential, single-unit capability coverage. It is not a
//! production scheduler, physical model, safety case, or production authorization.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    ReceiveIdentify,
    PrecisionMachining,
    Clean,
    Assembly,
    PressureFlowTest,
    DimensionalMetrology,
    Disassembly,
    Reconditioning,
    FinalInspection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModuleRole {
    MaterialReceipt,
    PrimaryMachining,
    BackupMachining,
    SharedCell,
    Cleaning,
    Assembly,
    PressureTest,
    Metrology,
    BackupMetrology,
    Disassembly,
    Reconditioning,
    PartnerMachining,
    PartnerDisassembly,
    PartnerReconditioning,
    FinalInspection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    ProcessQualification,
    Calibration,
    Workforce,
    Utilities,
    Segregation,
    QualityEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateStatus {
    Current,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub id: &'static str,
    pub role: ModuleRole,
    pub partner_node: Option<&'static str>,
    pub capabilities: Vec<Capability>,
    pub material_profiles: Vec<&'static str>,
    pub processing_ticks: u64,
    pub setup_ticks: u64,
    pub transfer_ticks: u64,
    pub cost_units: u64,
    pub energy_units: u64,
    pub process_qualification: GateStatus,
    pub calibration_required: bool,
    pub calibration: Option<GateStatus>,
    pub workforce: GateStatus,
    pub utilities: GateStatus,
    pub segregation: GateStatus,
    pub quality_evidence: GateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Architecture {
    pub id: &'static str,
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    pub id: &'static str,
    pub capability: Capability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductRoute {
    pub id: &'static str,
    pub material_profile: &'static str,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateOverride {
    pub role: ModuleRole,
    pub gate: Gate,
    pub status: GateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub id: &'static str,
    pub disabled_module_ids: Vec<&'static str>,
    pub disabled_roles: Vec<ModuleRole>,
    pub unavailable_partner_nodes: Vec<&'static str>,
    pub gate_overrides: Vec<GateOverride>,
}

impl Scenario {
    fn disables(&self, module: &Module) -> bool {
        self.disabled_module_ids.contains(&module.id)
            || self.disabled_roles.contains(&module.role)
            || module
                .partner_node
                .is_some_and(|node| self.unavailable_partner_nodes.contains(&node))
    }

    fn gate_status(&self, module: &Module, gate: Gate) -> Option<GateStatus> {
        if let Some(override_) = self
            .gate_overrides
            .iter()
            .rev()
            .find(|entry| entry.role == module.role && entry.gate == gate)
        {
            return Some(override_.status);
        }
        match gate {
            Gate::ProcessQualification => Some(module.process_qualification),
            Gate::Calibration => module.calibration,
            Gate::Workforce => Some(module.workforce),
            Gate::Utilities => Some(module.utilities),
            Gate::Segregation => Some(module.segregation),
            Gate::QualityEvidence => Some(module.quality_evidence),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleFailure {
    CapabilityMissing,
    MaterialEnvelopeMismatch,
    PartnerNodeMissing,
    UnavailableByScenario,
    GateNotCurrent {
        gate: Gate,
        status: Option<GateStatus>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleRejection {
    pub module_id: &'static str,
    pub failures: Vec<ModuleFailure>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedOperation {
    pub operation_id: &'static str,
    pub capability: Capability,
    pub rejected_modules: Vec<ModuleRejection>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Assignment {
    pub operation_id: &'static str,
    pub module_id: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCandidate {
    pub assignments: Vec<Assignment>,
    pub elapsed_ticks: u64,
    pub cost_units: u64,
    pub energy_units: u64,
}

impl RouteCandidate {
    fn dominates(&self, other: &Self) -> bool {
        let no_worse = self.elapsed_ticks <= other.elapsed_ticks
            && self.cost_units <= other.cost_units
            && self.energy_units <= other.energy_units;
        let strictly_better = self.elapsed_ticks < other.elapsed_ticks
            || self.cost_units < other.cost_units
            || self.energy_units < other.energy_units;
        no_worse && strictly_better
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    SyntheticAccepted,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputFailure {
    EmptyArchitectureId,
    EmptyModuleId,
    DuplicateModuleId { module_id: &'static str },
    EmptyProductId,
    EmptyMaterialProfile,
    EmptyScenarioId,
    EmptyRoute,
    EmptyOperationId,
    DuplicateOperationId { operation_id: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEvaluation {
    pub disposition: Disposition,
    pub input_failures: Vec<InputFailure>,
    pub feasible_assignment_count: usize,
    pub pareto_frontier: Vec<RouteCandidate>,
    pub blocked_operations: Vec<BlockedOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkRow {
    pub architecture_id: &'static str,
    pub product_id: &'static str,
    pub scenario_id: &'static str,
    pub evaluation: RouteEvaluation,
}

#[derive(Debug, Clone)]
struct ModuleChoice {
    module_id: &'static str,
    elapsed_ticks: u64,
    cost_units: u64,
    energy_units: u64,
}

fn module_failures(
    module: &Module,
    route: &ProductRoute,
    operation: Operation,
    scenario: &Scenario,
) -> Vec<ModuleFailure> {
    let mut failures = Vec::new();
    if !module.capabilities.contains(&operation.capability) {
        failures.push(ModuleFailure::CapabilityMissing);
    }
    if !module.material_profiles.contains(&route.material_profile) {
        failures.push(ModuleFailure::MaterialEnvelopeMismatch);
    }
    if matches!(
        module.role,
        ModuleRole::PartnerMachining
            | ModuleRole::PartnerDisassembly
            | ModuleRole::PartnerReconditioning
    ) && module
        .partner_node
        .is_none_or(|node| node.trim().is_empty())
    {
        failures.push(ModuleFailure::PartnerNodeMissing);
    }
    if scenario.disables(module) {
        failures.push(ModuleFailure::UnavailableByScenario);
    }

    for gate in [
        Gate::ProcessQualification,
        Gate::Workforce,
        Gate::Utilities,
        Gate::Segregation,
        Gate::QualityEvidence,
    ] {
        let status = scenario.gate_status(module, gate);
        if status != Some(GateStatus::Current) {
            failures.push(ModuleFailure::GateNotCurrent { gate, status });
        }
    }
    if module.calibration_required {
        // A scenario may stale or invalidate an existing calibration record,
        // but it cannot create a missing declaration.
        let status = if module.calibration.is_none() {
            None
        } else {
            scenario.gate_status(module, Gate::Calibration)
        };
        if status != Some(GateStatus::Current) {
            failures.push(ModuleFailure::GateNotCurrent {
                gate: Gate::Calibration,
                status,
            });
        }
    }
    failures
}

// This fixture enumerator carries the complete additive objective and assignment state.
#[allow(clippy::too_many_arguments)]
fn enumerate_assignments(
    choices: &[Vec<ModuleChoice>],
    index: usize,
    assignments: &mut Vec<Assignment>,
    elapsed_ticks: u64,
    cost_units: u64,
    energy_units: u64,
    output: &mut Vec<RouteCandidate>,
    route: &ProductRoute,
) {
    if index == choices.len() {
        output.push(RouteCandidate {
            assignments: assignments.clone(),
            elapsed_ticks,
            cost_units,
            energy_units,
        });
        return;
    }
    for choice in &choices[index] {
        let Some(next_elapsed) = elapsed_ticks.checked_add(choice.elapsed_ticks) else {
            continue;
        };
        let Some(next_cost) = cost_units.checked_add(choice.cost_units) else {
            continue;
        };
        let Some(next_energy) = energy_units.checked_add(choice.energy_units) else {
            continue;
        };
        assignments.push(Assignment {
            operation_id: route.operations[index].id,
            module_id: choice.module_id,
        });
        enumerate_assignments(
            choices,
            index + 1,
            assignments,
            next_elapsed,
            next_cost,
            next_energy,
            output,
            route,
        );
        assignments.pop();
    }
}

fn pareto_frontier(mut candidates: Vec<RouteCandidate>) -> Vec<RouteCandidate> {
    candidates.sort_by(|a, b| {
        (
            a.elapsed_ticks,
            a.cost_units,
            a.energy_units,
            &a.assignments,
        )
            .cmp(&(
                b.elapsed_ticks,
                b.cost_units,
                b.energy_units,
                &b.assignments,
            ))
    });
    let snapshot = candidates.clone();
    candidates
        .into_iter()
        .filter(|candidate| !snapshot.iter().any(|other| other.dominates(candidate)))
        .collect()
}

fn validate_inputs(
    architecture: &Architecture,
    route: &ProductRoute,
    scenario: &Scenario,
) -> Vec<InputFailure> {
    let mut failures = Vec::new();
    if architecture.id.trim().is_empty() {
        failures.push(InputFailure::EmptyArchitectureId);
    }
    if route.id.trim().is_empty() {
        failures.push(InputFailure::EmptyProductId);
    }
    if route.material_profile.trim().is_empty() {
        failures.push(InputFailure::EmptyMaterialProfile);
    }
    if scenario.id.trim().is_empty() {
        failures.push(InputFailure::EmptyScenarioId);
    }
    if route.operations.is_empty() {
        failures.push(InputFailure::EmptyRoute);
    }

    let mut module_ids = BTreeSet::new();
    for module in &architecture.modules {
        if module.id.trim().is_empty() {
            failures.push(InputFailure::EmptyModuleId);
        } else if !module_ids.insert(module.id) {
            failures.push(InputFailure::DuplicateModuleId {
                module_id: module.id,
            });
        }
    }

    let mut operation_ids = BTreeSet::new();
    for operation in &route.operations {
        if operation.id.trim().is_empty() {
            failures.push(InputFailure::EmptyOperationId);
        } else if !operation_ids.insert(operation.id) {
            failures.push(InputFailure::DuplicateOperationId {
                operation_id: operation.id,
            });
        }
    }
    failures
}

/// Evaluate one sequential product route against one fixed architecture and scenario.
/// Every required gate must be current; stale or unknown evidence fails closed.
pub fn evaluate(
    architecture: &Architecture,
    route: &ProductRoute,
    scenario: &Scenario,
) -> RouteEvaluation {
    let input_failures = validate_inputs(architecture, route, scenario);
    if !input_failures.is_empty() {
        return RouteEvaluation {
            disposition: Disposition::Blocked,
            input_failures,
            feasible_assignment_count: 0,
            pareto_frontier: Vec::new(),
            blocked_operations: Vec::new(),
        };
    }

    let mut choices_by_operation = Vec::with_capacity(route.operations.len());
    let mut blocked_operations = Vec::new();

    for operation in &route.operations {
        let mut choices = Vec::new();
        let mut rejected_modules = Vec::new();
        for module in &architecture.modules {
            let failures = module_failures(module, route, *operation, scenario);
            if failures.is_empty() {
                let Some(elapsed_ticks) = module
                    .processing_ticks
                    .checked_add(module.setup_ticks)
                    .and_then(|ticks| ticks.checked_add(module.transfer_ticks))
                else {
                    continue;
                };
                choices.push(ModuleChoice {
                    module_id: module.id,
                    elapsed_ticks,
                    cost_units: module.cost_units,
                    energy_units: module.energy_units,
                });
            } else {
                rejected_modules.push(ModuleRejection {
                    module_id: module.id,
                    failures,
                });
            }
        }
        choices.sort_by_key(|choice| {
            (
                choice.elapsed_ticks,
                choice.cost_units,
                choice.energy_units,
                choice.module_id,
            )
        });
        if choices.is_empty() {
            blocked_operations.push(BlockedOperation {
                operation_id: operation.id,
                capability: operation.capability,
                rejected_modules,
            });
        }
        choices_by_operation.push(choices);
    }

    if !blocked_operations.is_empty() {
        return RouteEvaluation {
            disposition: Disposition::Blocked,
            input_failures: Vec::new(),
            feasible_assignment_count: 0,
            pareto_frontier: Vec::new(),
            blocked_operations,
        };
    }

    let mut candidates = Vec::new();
    enumerate_assignments(
        &choices_by_operation,
        0,
        &mut Vec::with_capacity(route.operations.len()),
        0,
        0,
        0,
        &mut candidates,
        route,
    );
    let feasible_assignment_count = candidates.len();
    let frontier = pareto_frontier(candidates);
    RouteEvaluation {
        disposition: if frontier.is_empty() {
            Disposition::Blocked
        } else {
            Disposition::SyntheticAccepted
        },
        input_failures: Vec::new(),
        feasible_assignment_count,
        pareto_frontier: frontier,
        blocked_operations: Vec::new(),
    }
}

// Positional arguments keep the small, frozen reference fixtures compact and reviewable.
#[allow(clippy::too_many_arguments)]
fn module(
    id: &'static str,
    role: ModuleRole,
    capabilities: &[Capability],
    processing_ticks: u64,
    setup_ticks: u64,
    transfer_ticks: u64,
    cost_units: u64,
    energy_units: u64,
    requires_calibration: bool,
    partner_node: Option<&'static str>,
) -> Module {
    Module {
        id,
        role,
        partner_node,
        capabilities: capabilities.to_vec(),
        material_profiles: vec!["SYNTHETIC-FERROUS-A", "SYNTHETIC-RETURNED-FERROUS-A"],
        processing_ticks,
        setup_ticks,
        transfer_ticks,
        cost_units,
        energy_units,
        process_qualification: GateStatus::Current,
        calibration_required: requires_calibration,
        calibration: requires_calibration.then_some(GateStatus::Current),
        workforce: GateStatus::Current,
        utilities: GateStatus::Current,
        segregation: GateStatus::Current,
        quality_evidence: GateStatus::Current,
    }
}

/// Four intentionally different synthetic profiles. All quantities are dimensionless relative units.
pub fn reference_architectures() -> Vec<Architecture> {
    use Capability::*;
    vec![
        Architecture {
            id: "DEDICATED_LINE",
            modules: vec![
                module(
                    "DL-RECEIVE",
                    ModuleRole::MaterialReceipt,
                    &[ReceiveIdentify],
                    1,
                    0,
                    0,
                    1,
                    0,
                    false,
                    None,
                ),
                module(
                    "DL-CNC",
                    ModuleRole::PrimaryMachining,
                    &[PrecisionMachining],
                    6,
                    2,
                    0,
                    8,
                    6,
                    false,
                    None,
                ),
                module(
                    "DL-CLEAN",
                    ModuleRole::Cleaning,
                    &[Clean],
                    2,
                    1,
                    0,
                    2,
                    2,
                    false,
                    None,
                ),
                module(
                    "DL-ASSEMBLY",
                    ModuleRole::Assembly,
                    &[Assembly],
                    5,
                    1,
                    0,
                    4,
                    1,
                    false,
                    None,
                ),
                module(
                    "DL-TEST",
                    ModuleRole::PressureTest,
                    &[PressureFlowTest],
                    3,
                    1,
                    0,
                    3,
                    3,
                    true,
                    None,
                ),
                module(
                    "DL-CMM",
                    ModuleRole::Metrology,
                    &[DimensionalMetrology],
                    2,
                    1,
                    0,
                    2,
                    1,
                    true,
                    None,
                ),
                module(
                    "DL-FINAL",
                    ModuleRole::FinalInspection,
                    &[FinalInspection],
                    1,
                    1,
                    0,
                    1,
                    0,
                    false,
                    None,
                ),
            ],
        },
        Architecture {
            id: "FUNCTIONAL_JOB_SHOP",
            modules: vec![
                module(
                    "JS-RECEIVE",
                    ModuleRole::MaterialReceipt,
                    &[ReceiveIdentify],
                    1,
                    0,
                    0,
                    1,
                    0,
                    false,
                    None,
                ),
                module(
                    "JS-CNC-PRIMARY",
                    ModuleRole::PrimaryMachining,
                    &[PrecisionMachining],
                    8,
                    2,
                    0,
                    7,
                    6,
                    false,
                    None,
                ),
                module(
                    "JS-CNC-BACKUP",
                    ModuleRole::BackupMachining,
                    &[PrecisionMachining],
                    11,
                    2,
                    0,
                    6,
                    7,
                    false,
                    None,
                ),
                module(
                    "JS-CLEAN",
                    ModuleRole::Cleaning,
                    &[Clean],
                    3,
                    1,
                    0,
                    2,
                    2,
                    false,
                    None,
                ),
                module(
                    "JS-ASSEMBLY",
                    ModuleRole::Assembly,
                    &[Assembly],
                    6,
                    2,
                    0,
                    3,
                    1,
                    false,
                    None,
                ),
                module(
                    "JS-TEST",
                    ModuleRole::PressureTest,
                    &[PressureFlowTest],
                    4,
                    1,
                    0,
                    3,
                    3,
                    true,
                    None,
                ),
                module(
                    "JS-CMM-PRIMARY",
                    ModuleRole::Metrology,
                    &[DimensionalMetrology],
                    3,
                    1,
                    0,
                    2,
                    1,
                    true,
                    None,
                ),
                module(
                    "JS-CMM-BACKUP",
                    ModuleRole::BackupMetrology,
                    &[DimensionalMetrology],
                    4,
                    1,
                    0,
                    3,
                    1,
                    true,
                    None,
                ),
                module(
                    "JS-DISASSEMBLY",
                    ModuleRole::Disassembly,
                    &[Disassembly],
                    4,
                    1,
                    0,
                    2,
                    1,
                    false,
                    None,
                ),
                module(
                    "JS-RECONDITION",
                    ModuleRole::Reconditioning,
                    &[Reconditioning],
                    5,
                    2,
                    0,
                    5,
                    4,
                    false,
                    None,
                ),
                module(
                    "JS-FINAL",
                    ModuleRole::FinalInspection,
                    &[FinalInspection],
                    2,
                    1,
                    0,
                    1,
                    0,
                    false,
                    None,
                ),
            ],
        },
        Architecture {
            id: "MULTIPURPOSE_MICROFACTORY",
            modules: vec![
                module(
                    "MF-RECEIVE",
                    ModuleRole::MaterialReceipt,
                    &[ReceiveIdentify],
                    1,
                    0,
                    0,
                    1,
                    0,
                    false,
                    None,
                ),
                module(
                    "MF-SHARED-CELL",
                    ModuleRole::SharedCell,
                    &[PrecisionMachining, Reconditioning],
                    9,
                    3,
                    0,
                    5,
                    7,
                    false,
                    None,
                ),
                module(
                    "MF-CLEAN",
                    ModuleRole::Cleaning,
                    &[Clean],
                    4,
                    1,
                    0,
                    2,
                    2,
                    false,
                    None,
                ),
                module(
                    "MF-ASSEMBLY",
                    ModuleRole::Assembly,
                    &[Assembly],
                    7,
                    1,
                    0,
                    3,
                    2,
                    false,
                    None,
                ),
                module(
                    "MF-TEST",
                    ModuleRole::PressureTest,
                    &[PressureFlowTest],
                    5,
                    1,
                    0,
                    4,
                    3,
                    true,
                    None,
                ),
                module(
                    "MF-CMM",
                    ModuleRole::Metrology,
                    &[DimensionalMetrology],
                    4,
                    1,
                    0,
                    3,
                    2,
                    true,
                    None,
                ),
                module(
                    "MF-DISASSEMBLY",
                    ModuleRole::Disassembly,
                    &[Disassembly],
                    5,
                    1,
                    0,
                    3,
                    1,
                    false,
                    None,
                ),
                module(
                    "MF-FINAL",
                    ModuleRole::FinalInspection,
                    &[FinalInspection],
                    2,
                    1,
                    0,
                    1,
                    0,
                    false,
                    None,
                ),
            ],
        },
        Architecture {
            id: "HYBRID_LOCAL_PARTNER",
            modules: vec![
                module(
                    "HY-RECEIVE",
                    ModuleRole::MaterialReceipt,
                    &[ReceiveIdentify],
                    1,
                    0,
                    0,
                    1,
                    0,
                    false,
                    None,
                ),
                module(
                    "HY-PARTNER-CNC",
                    ModuleRole::PartnerMachining,
                    &[PrecisionMachining],
                    6,
                    2,
                    6,
                    10,
                    5,
                    false,
                    Some("PARTNER-PRECISION"),
                ),
                module(
                    "HY-CLEAN",
                    ModuleRole::Cleaning,
                    &[Clean],
                    3,
                    1,
                    0,
                    2,
                    2,
                    false,
                    None,
                ),
                module(
                    "HY-ASSEMBLY",
                    ModuleRole::Assembly,
                    &[Assembly],
                    6,
                    1,
                    0,
                    3,
                    2,
                    false,
                    None,
                ),
                module(
                    "HY-TEST",
                    ModuleRole::PressureTest,
                    &[PressureFlowTest],
                    4,
                    1,
                    0,
                    3,
                    3,
                    true,
                    None,
                ),
                module(
                    "HY-METROLOGY",
                    ModuleRole::Metrology,
                    &[DimensionalMetrology],
                    3,
                    1,
                    0,
                    2,
                    1,
                    true,
                    None,
                ),
                module(
                    "HY-PARTNER-DISASSEMBLY",
                    ModuleRole::PartnerDisassembly,
                    &[Disassembly],
                    4,
                    2,
                    6,
                    8,
                    3,
                    false,
                    Some("PARTNER-PRECISION"),
                ),
                module(
                    "HY-PARTNER-RECONDITION",
                    ModuleRole::PartnerReconditioning,
                    &[Reconditioning],
                    7,
                    2,
                    6,
                    12,
                    5,
                    false,
                    Some("PARTNER-PRECISION"),
                ),
                module(
                    "HY-FINAL",
                    ModuleRole::FinalInspection,
                    &[FinalInspection],
                    2,
                    1,
                    0,
                    1,
                    0,
                    false,
                    None,
                ),
            ],
        },
    ]
}

pub fn reference_routes() -> Vec<ProductRoute> {
    use Capability::*;
    vec![
        ProductRoute {
            id: "PUMP_VALVE_SUBASSEMBLY",
            material_profile: "SYNTHETIC-FERROUS-A",
            operations: vec![
                Operation {
                    id: "RECEIVE_IDENTIFY",
                    capability: ReceiveIdentify,
                },
                Operation {
                    id: "PRECISION_MACHINING",
                    capability: PrecisionMachining,
                },
                Operation {
                    id: "CLEAN",
                    capability: Clean,
                },
                Operation {
                    id: "ASSEMBLY",
                    capability: Assembly,
                },
                Operation {
                    id: "PRESSURE_FLOW_TEST",
                    capability: PressureFlowTest,
                },
                Operation {
                    id: "DIMENSIONAL_METROLOGY",
                    capability: DimensionalMetrology,
                },
                Operation {
                    id: "FINAL_INSPECTION",
                    capability: FinalInspection,
                },
            ],
        },
        ProductRoute {
            id: "RETURNED_PUMP_REPAIR",
            material_profile: "SYNTHETIC-RETURNED-FERROUS-A",
            operations: vec![
                Operation {
                    id: "RETURN_INTAKE",
                    capability: ReceiveIdentify,
                },
                Operation {
                    id: "DISASSEMBLY",
                    capability: Disassembly,
                },
                Operation {
                    id: "CLEAN",
                    capability: Clean,
                },
                Operation {
                    id: "PRE_REPAIR_METROLOGY",
                    capability: DimensionalMetrology,
                },
                Operation {
                    id: "RECONDITIONING",
                    capability: Reconditioning,
                },
                Operation {
                    id: "REASSEMBLY",
                    capability: Assembly,
                },
                Operation {
                    id: "PRESSURE_FLOW_TEST",
                    capability: PressureFlowTest,
                },
                Operation {
                    id: "POST_REPAIR_METROLOGY",
                    capability: DimensionalMetrology,
                },
                Operation {
                    id: "FINAL_INSPECTION",
                    capability: FinalInspection,
                },
            ],
        },
    ]
}

pub fn reference_scenarios() -> Vec<Scenario> {
    use Gate::*;
    use GateStatus::*;
    use ModuleRole::*;
    vec![
        Scenario {
            id: "BASELINE",
            disabled_module_ids: vec![],
            disabled_roles: vec![],
            unavailable_partner_nodes: vec![],
            gate_overrides: vec![],
        },
        Scenario {
            id: "PRIMARY_MACHINING_OUTAGE",
            disabled_module_ids: vec![
                "DL-CNC",
                "JS-CNC-PRIMARY",
                "MF-SHARED-CELL",
                "HY-PARTNER-CNC",
            ],
            disabled_roles: vec![],
            unavailable_partner_nodes: vec![],
            gate_overrides: vec![],
        },
        Scenario {
            id: "SHARED_CELL_OUTAGE",
            disabled_module_ids: vec![],
            disabled_roles: vec![SharedCell],
            unavailable_partner_nodes: vec![],
            gate_overrides: vec![],
        },
        Scenario {
            id: "PARTNER_NODE_OUTAGE",
            disabled_module_ids: vec![],
            disabled_roles: vec![],
            unavailable_partner_nodes: vec!["PARTNER-PRECISION"],
            gate_overrides: vec![],
        },
        Scenario {
            id: "METROLOGY_CALIBRATION_STALE",
            disabled_module_ids: vec![],
            disabled_roles: vec![],
            unavailable_partner_nodes: vec![],
            gate_overrides: vec![
                GateOverride {
                    role: Metrology,
                    gate: Calibration,
                    status: Stale,
                },
                GateOverride {
                    role: BackupMetrology,
                    gate: Calibration,
                    status: Stale,
                },
            ],
        },
        Scenario {
            id: "PRESSURE_TEST_SKILL_UNKNOWN",
            disabled_module_ids: vec![],
            disabled_roles: vec![],
            unavailable_partner_nodes: vec![],
            gate_overrides: vec![GateOverride {
                role: PressureTest,
                gate: Workforce,
                status: Unknown,
            }],
        },
        Scenario {
            id: "POWER_UTILITY_INTERRUPTION",
            disabled_module_ids: vec![],
            disabled_roles: vec![],
            unavailable_partner_nodes: vec![],
            gate_overrides: vec![
                GateOverride {
                    role: PrimaryMachining,
                    gate: Utilities,
                    status: Stale,
                },
                GateOverride {
                    role: BackupMachining,
                    gate: Utilities,
                    status: Stale,
                },
                GateOverride {
                    role: SharedCell,
                    gate: Utilities,
                    status: Stale,
                },
                GateOverride {
                    role: PartnerMachining,
                    gate: Utilities,
                    status: Stale,
                },
                GateOverride {
                    role: PartnerReconditioning,
                    gate: Utilities,
                    status: Stale,
                },
                GateOverride {
                    role: PressureTest,
                    gate: Utilities,
                    status: Stale,
                },
            ],
        },
    ]
}

pub fn run_reference_benchmark() -> Vec<BenchmarkRow> {
    let architectures = reference_architectures();
    let routes = reference_routes();
    let scenarios = reference_scenarios();
    let mut rows = Vec::with_capacity(architectures.len() * routes.len() * scenarios.len());
    for architecture in &architectures {
        for route in &routes {
            for scenario in &scenarios {
                rows.push(BenchmarkRow {
                    architecture_id: architecture.id,
                    product_id: route.id,
                    scenario_id: scenario.id,
                    evaluation: evaluate(architecture, route, scenario),
                });
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(architecture_id: &str, route_id: &str, scenario_id: &str) -> RouteEvaluation {
        let architectures = reference_architectures();
        let routes = reference_routes();
        let scenarios = reference_scenarios();
        let architecture = architectures
            .iter()
            .find(|a| a.id == architecture_id)
            .unwrap();
        let route = routes.iter().find(|r| r.id == route_id).unwrap();
        let scenario = scenarios.iter().find(|s| s.id == scenario_id).unwrap();
        evaluate(architecture, route, scenario)
    }

    #[test]
    fn reference_matrix_has_four_architectures_two_routes_and_seven_scenarios() {
        assert_eq!(run_reference_benchmark().len(), 4 * 2 * 7);
    }

    #[test]
    fn dedicated_line_handles_pump_but_not_repair() {
        assert_eq!(
            eval("DEDICATED_LINE", "PUMP_VALVE_SUBASSEMBLY", "BASELINE").disposition,
            Disposition::SyntheticAccepted
        );
        let repair = eval("DEDICATED_LINE", "RETURNED_PUMP_REPAIR", "BASELINE");
        assert_eq!(repair.disposition, Disposition::Blocked);
        assert!(
            repair
                .blocked_operations
                .iter()
                .any(|op| op.capability == Capability::Disassembly)
        );
    }

    #[test]
    fn job_shop_microfactory_and_hybrid_cover_both_baseline_routes() {
        for architecture in [
            "FUNCTIONAL_JOB_SHOP",
            "MULTIPURPOSE_MICROFACTORY",
            "HYBRID_LOCAL_PARTNER",
        ] {
            for route in ["PUMP_VALVE_SUBASSEMBLY", "RETURNED_PUMP_REPAIR"] {
                assert_eq!(
                    eval(architecture, route, "BASELINE").disposition,
                    Disposition::SyntheticAccepted
                );
            }
        }
    }

    #[test]
    fn job_shop_backup_preserves_pump_route_after_primary_machine_outage() {
        let out = eval(
            "FUNCTIONAL_JOB_SHOP",
            "PUMP_VALVE_SUBASSEMBLY",
            "PRIMARY_MACHINING_OUTAGE",
        );
        assert_eq!(out.disposition, Disposition::SyntheticAccepted);
        assert!(out.pareto_frontier.iter().any(|candidate| {
            candidate
                .assignments
                .iter()
                .any(|a| a.module_id == "JS-CNC-BACKUP")
        }));
    }

    #[test]
    fn primary_machine_outage_is_matched_to_each_architecture() {
        // The same functional bottleneck is mapped to each architecture's
        // designated primary cell; only the job shop has a backup machine.
        assert_eq!(
            eval(
                "DEDICATED_LINE",
                "PUMP_VALVE_SUBASSEMBLY",
                "PRIMARY_MACHINING_OUTAGE"
            )
            .disposition,
            Disposition::Blocked
        );
        assert_eq!(
            eval(
                "FUNCTIONAL_JOB_SHOP",
                "PUMP_VALVE_SUBASSEMBLY",
                "PRIMARY_MACHINING_OUTAGE"
            )
            .disposition,
            Disposition::SyntheticAccepted
        );
        assert_eq!(
            eval(
                "MULTIPURPOSE_MICROFACTORY",
                "PUMP_VALVE_SUBASSEMBLY",
                "PRIMARY_MACHINING_OUTAGE"
            )
            .disposition,
            Disposition::Blocked
        );
        assert_eq!(
            eval(
                "HYBRID_LOCAL_PARTNER",
                "PUMP_VALVE_SUBASSEMBLY",
                "PRIMARY_MACHINING_OUTAGE"
            )
            .disposition,
            Disposition::Blocked
        );
    }

    #[test]
    fn shared_cell_outage_only_blocks_microfactory_route_in_reference_profiles() {
        assert_eq!(
            eval(
                "MULTIPURPOSE_MICROFACTORY",
                "PUMP_VALVE_SUBASSEMBLY",
                "SHARED_CELL_OUTAGE"
            )
            .disposition,
            Disposition::Blocked
        );
        assert_eq!(
            eval(
                "FUNCTIONAL_JOB_SHOP",
                "PUMP_VALVE_SUBASSEMBLY",
                "SHARED_CELL_OUTAGE"
            )
            .disposition,
            Disposition::SyntheticAccepted
        );
    }

    #[test]
    fn partner_node_outage_blocks_hybrid_routes() {
        for route in ["PUMP_VALVE_SUBASSEMBLY", "RETURNED_PUMP_REPAIR"] {
            assert_eq!(
                eval("HYBRID_LOCAL_PARTNER", route, "PARTNER_NODE_OUTAGE").disposition,
                Disposition::Blocked
            );
        }
    }

    #[test]
    fn stale_metrology_calibration_blocks_acceptance() {
        for architecture in [
            "DEDICATED_LINE",
            "FUNCTIONAL_JOB_SHOP",
            "MULTIPURPOSE_MICROFACTORY",
            "HYBRID_LOCAL_PARTNER",
        ] {
            assert_eq!(
                eval(
                    architecture,
                    "PUMP_VALVE_SUBASSEMBLY",
                    "METROLOGY_CALIBRATION_STALE"
                )
                .disposition,
                Disposition::Blocked
            );
        }
    }

    #[test]
    fn unknown_test_workforce_evidence_fails_closed() {
        for architecture in [
            "DEDICATED_LINE",
            "FUNCTIONAL_JOB_SHOP",
            "MULTIPURPOSE_MICROFACTORY",
            "HYBRID_LOCAL_PARTNER",
        ] {
            assert_eq!(
                eval(
                    architecture,
                    "PUMP_VALVE_SUBASSEMBLY",
                    "PRESSURE_TEST_SKILL_UNKNOWN"
                )
                .disposition,
                Disposition::Blocked
            );
        }
    }

    #[test]
    fn utility_interruption_blocks_affected_routes() {
        for architecture in [
            "DEDICATED_LINE",
            "FUNCTIONAL_JOB_SHOP",
            "MULTIPURPOSE_MICROFACTORY",
            "HYBRID_LOCAL_PARTNER",
        ] {
            assert_eq!(
                eval(
                    architecture,
                    "PUMP_VALVE_SUBASSEMBLY",
                    "POWER_UTILITY_INTERRUPTION"
                )
                .disposition,
                Disposition::Blocked
            );
        }
    }

    #[test]
    fn unknown_process_qualification_is_not_admitted() {
        let mut architectures = reference_architectures();
        let route = reference_routes().remove(0);
        let scenario = reference_scenarios().remove(0);
        let architecture = architectures
            .iter_mut()
            .find(|a| a.id == "DEDICATED_LINE")
            .unwrap();
        architecture
            .modules
            .iter_mut()
            .find(|m| m.id == "DL-CNC")
            .unwrap()
            .process_qualification = GateStatus::Unknown;
        let result = evaluate(architecture, &route, &scenario);
        assert_eq!(result.disposition, Disposition::Blocked);
        let blocked = result
            .blocked_operations
            .iter()
            .find(|op| op.operation_id == "PRECISION_MACHINING")
            .unwrap();
        assert!(
            blocked
                .rejected_modules
                .iter()
                .any(|module| module.module_id == "DL-CNC"
                    && module.failures.contains(&ModuleFailure::GateNotCurrent {
                        gate: Gate::ProcessQualification,
                        status: Some(GateStatus::Unknown)
                    }))
        );
    }

    #[test]
    fn material_outside_module_envelope_is_not_admitted() {
        let architecture = reference_architectures().remove(0);
        let mut route = reference_routes().remove(0);
        route.material_profile = "UNQUALIFIED-MATERIAL";
        let scenario = reference_scenarios().remove(0);
        let result = evaluate(&architecture, &route, &scenario);
        assert_eq!(result.disposition, Disposition::Blocked);
        assert!(
            result
                .blocked_operations
                .iter()
                .any(|op| op.rejected_modules.iter().any(|module| module
                    .failures
                    .contains(&ModuleFailure::MaterialEnvelopeMismatch)))
        );
    }

    #[test]
    fn accepted_routes_have_complete_operation_lineage() {
        let architecture = reference_architectures()
            .into_iter()
            .find(|a| a.id == "FUNCTIONAL_JOB_SHOP")
            .unwrap();
        let route = reference_routes()
            .into_iter()
            .find(|r| r.id == "RETURNED_PUMP_REPAIR")
            .unwrap();
        let scenario = reference_scenarios()
            .into_iter()
            .find(|s| s.id == "BASELINE")
            .unwrap();
        let result = evaluate(&architecture, &route, &scenario);
        assert_eq!(result.disposition, Disposition::SyntheticAccepted);
        assert!(!result.pareto_frontier.is_empty());
        assert!(
            result
                .pareto_frontier
                .iter()
                .all(|candidate| candidate.assignments.len() == route.operations.len())
        );
    }

    #[test]
    fn deterministic_replay_produces_identical_frontiers() {
        assert_eq!(
            eval("FUNCTIONAL_JOB_SHOP", "RETURNED_PUMP_REPAIR", "BASELINE"),
            eval("FUNCTIONAL_JOB_SHOP", "RETURNED_PUMP_REPAIR", "BASELINE")
        );
    }

    #[test]
    fn missing_required_calibration_declaration_fails_closed() {
        let mut architecture = reference_architectures().remove(0);
        let route = reference_routes().remove(0);
        let mut scenario = reference_scenarios().remove(0);
        let cmm = architecture
            .modules
            .iter_mut()
            .find(|module| module.id == "DL-CMM")
            .unwrap();
        cmm.calibration = None;
        scenario.gate_overrides.push(GateOverride {
            role: ModuleRole::Metrology,
            gate: Gate::Calibration,
            status: GateStatus::Current,
        });
        let result = evaluate(&architecture, &route, &scenario);
        assert_eq!(result.disposition, Disposition::Blocked);
        let blocked = result
            .blocked_operations
            .iter()
            .find(|operation| operation.operation_id == "DIMENSIONAL_METROLOGY")
            .unwrap();
        assert!(
            blocked
                .rejected_modules
                .iter()
                .any(|module| module.module_id == "DL-CMM"
                    && module.failures.contains(&ModuleFailure::GateNotCurrent {
                        gate: Gate::Calibration,
                        status: None
                    }))
        );
    }

    #[test]
    fn partner_module_without_node_identity_is_rejected() {
        let mut architecture = reference_architectures()
            .into_iter()
            .find(|a| a.id == "HYBRID_LOCAL_PARTNER")
            .unwrap();
        let route = reference_routes()
            .into_iter()
            .find(|r| r.id == "PUMP_VALVE_SUBASSEMBLY")
            .unwrap();
        let scenario = reference_scenarios().remove(0);
        architecture
            .modules
            .iter_mut()
            .find(|module| module.id == "HY-PARTNER-CNC")
            .unwrap()
            .partner_node = None;
        let result = evaluate(&architecture, &route, &scenario);
        assert_eq!(result.disposition, Disposition::Blocked);
        let blocked = result
            .blocked_operations
            .iter()
            .find(|operation| operation.operation_id == "PRECISION_MACHINING")
            .unwrap();
        assert!(
            blocked
                .rejected_modules
                .iter()
                .any(|module| module.module_id == "HY-PARTNER-CNC"
                    && module.failures.contains(&ModuleFailure::PartnerNodeMissing))
        );
    }

    #[test]
    fn duplicate_module_identity_blocks_ambiguous_lineage() {
        let mut architecture = reference_architectures().remove(0);
        architecture.modules.push(architecture.modules[0].clone());
        let route = reference_routes().remove(0);
        let scenario = reference_scenarios().remove(0);
        let result = evaluate(&architecture, &route, &scenario);
        assert_eq!(result.disposition, Disposition::Blocked);
        assert!(
            result
                .input_failures
                .contains(&InputFailure::DuplicateModuleId {
                    module_id: "DL-RECEIVE"
                })
        );
    }

    #[test]
    fn duplicate_operation_identity_blocks_ambiguous_lineage() {
        let architecture = reference_architectures().remove(0);
        let mut route = reference_routes().remove(0);
        route.operations[1].id = route.operations[0].id;
        let scenario = reference_scenarios().remove(0);
        let result = evaluate(&architecture, &route, &scenario);
        assert_eq!(result.disposition, Disposition::Blocked);
        assert!(
            result
                .input_failures
                .contains(&InputFailure::DuplicateOperationId {
                    operation_id: "RECEIVE_IDENTIFY"
                })
        );
    }

    #[test]
    fn empty_route_is_rejected_with_explicit_input_failure() {
        let architecture = reference_architectures().remove(0);
        let mut route = reference_routes().remove(0);
        route.operations.clear();
        let scenario = reference_scenarios().remove(0);
        let result = evaluate(&architecture, &route, &scenario);
        assert_eq!(result.disposition, Disposition::Blocked);
        assert!(result.input_failures.contains(&InputFailure::EmptyRoute));
    }

    #[test]
    fn pareto_frontier_contains_no_dominated_candidate() {
        let result = eval("FUNCTIONAL_JOB_SHOP", "PUMP_VALVE_SUBASSEMBLY", "BASELINE");
        assert!(!result.pareto_frontier.is_empty());
        for a in &result.pareto_frontier {
            for b in &result.pareto_frontier {
                if a != b {
                    assert!(!a.dominates(b));
                }
            }
        }
    }
}
