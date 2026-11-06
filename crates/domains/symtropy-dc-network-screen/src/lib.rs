// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Deterministic DC-network screening for locational load-flexibility delivery.
//!
//! Sign convention: positive non-slack bus injection is net generation or
//! demand reduction; negative injection is net load increase. The slack bus
//! balances the sum of all supplied non-slack injections.
//!
//! This is a linear active-power screening tool. It does not establish AC
//! voltage/reactive feasibility, losses, protection, RMS/EMT stability,
//! distribution-feeder compliance, utility acceptance or ELCC.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// Physical bus in the frozen network model.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcBus {
    id: String,
    is_slack: bool,
}

impl DcBus {
    pub fn new(id: impl Into<String>, is_slack: bool) -> Result<Self, DcNetworkError> {
        let id = id.into();
        require_id("bus ID", &id)?;
        Ok(Self { id, is_slack })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn is_slack(&self) -> bool {
        self.is_slack
    }
}

/// Branch model using a per-unit series reactance and MW flow limit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcBranch {
    id: String,
    from_bus: String,
    to_bus: String,
    reactance_pu: f64,
    normal_limit_mw: f64,
    emergency_limit_mw: f64,
}

impl DcBranch {
    pub fn new(
        id: impl Into<String>,
        from_bus: impl Into<String>,
        to_bus: impl Into<String>,
        reactance_pu: f64,
        normal_limit_mw: f64,
        emergency_limit_mw: f64,
    ) -> Result<Self, DcNetworkError> {
        let branch = Self {
            id: id.into(),
            from_bus: from_bus.into(),
            to_bus: to_bus.into(),
            reactance_pu,
            normal_limit_mw,
            emergency_limit_mw,
        };
        require_id("branch ID", &branch.id)?;
        require_id("branch from bus", &branch.from_bus)?;
        require_id("branch to bus", &branch.to_bus)?;
        if branch.from_bus == branch.to_bus {
            return Err(DcNetworkError::SelfLoop {
                branch_id: branch.id.clone(),
            });
        }
        for (field, value) in [
            ("branch reactance", reactance_pu),
            ("normal branch limit", normal_limit_mw),
            ("emergency branch limit", emergency_limit_mw),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(DcNetworkError::InvalidQuantity { field, value });
            }
        }
        if emergency_limit_mw < normal_limit_mw {
            return Err(DcNetworkError::EmergencyLimitBelowNormal {
                branch_id: branch.id.clone(),
                normal_limit_mw,
                emergency_limit_mw,
            });
        }
        Ok(branch)
    }
}

/// Versioned network and topology snapshot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcNetworkModel {
    model_id: String,
    model_digest: String,
    topology_id: String,
    topology_digest: String,
    topology_effective_at_ns: u64,
    slack_bus_id: String,
    buses: Vec<DcBus>,
    branches: Vec<DcBranch>,
}

impl DcNetworkModel {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model_id: impl Into<String>,
        model_digest: impl Into<String>,
        topology_id: impl Into<String>,
        topology_digest: impl Into<String>,
        topology_effective_at_ns: u64,
        buses: Vec<DcBus>,
        branches: Vec<DcBranch>,
    ) -> Result<Self, DcNetworkError> {
        let model = Self {
            model_id: model_id.into(),
            model_digest: model_digest.into(),
            topology_id: topology_id.into(),
            topology_digest: topology_digest.into(),
            topology_effective_at_ns,
            slack_bus_id: String::new(),
            buses,
            branches,
        };
        require_id("network model ID", &model.model_id)?;
        require_id("network model digest", &model.model_digest)?;
        require_id("topology ID", &model.topology_id)?;
        require_id("topology digest", &model.topology_digest)?;
        if model.buses.is_empty() {
            return Err(DcNetworkError::EmptyNetwork);
        }

        let mut bus_ids = BTreeSet::new();
        let mut slack_ids = Vec::new();
        for bus in &model.buses {
            if !bus_ids.insert(bus.id.as_str()) {
                return Err(DcNetworkError::DuplicateBus {
                    bus_id: bus.id.clone(),
                });
            }
            if bus.is_slack {
                slack_ids.push(bus.id.clone());
            }
        }
        if slack_ids.len() != 1 {
            return Err(DcNetworkError::InvalidSlackBusCount {
                observed: slack_ids.len(),
            });
        }
        let slack_bus_id = slack_ids[0].clone();

        let mut branch_ids = BTreeSet::new();
        for branch in &model.branches {
            if !branch_ids.insert(branch.id.as_str()) {
                return Err(DcNetworkError::DuplicateBranch {
                    branch_id: branch.id.clone(),
                });
            }
            for bus_id in [&branch.from_bus, &branch.to_bus] {
                if !bus_ids.contains(bus_id.as_str()) {
                    return Err(DcNetworkError::UnknownBus {
                        bus_id: bus_id.clone(),
                    });
                }
            }
        }
        Ok(Self {
            slack_bus_id,
            ..model
        })
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn model_digest(&self) -> &str {
        &self.model_digest
    }

    pub fn topology_id(&self) -> &str {
        &self.topology_id
    }

    pub fn topology_digest(&self) -> &str {
        &self.topology_digest
    }

    pub const fn topology_effective_at_ns(&self) -> u64 {
        self.topology_effective_at_ns
    }

    pub fn slack_bus_id(&self) -> &str {
        &self.slack_bus_id
    }
}

/// Input contract: all tolerances and topology age limits are explicit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcNetworkContract {
    maximum_topology_age_ns: u64,
    flow_tolerance_mw: f64,
    pivot_tolerance: f64,
    require_n_minus_one: bool,
}

impl DcNetworkContract {
    pub fn new(
        maximum_topology_age_ns: u64,
        flow_tolerance_mw: f64,
        pivot_tolerance: f64,
        require_n_minus_one: bool,
    ) -> Result<Self, DcNetworkError> {
        if maximum_topology_age_ns == 0 {
            return Err(DcNetworkError::InvalidTopologyAgeLimit);
        }
        for (field, value) in [
            ("flow tolerance", flow_tolerance_mw),
            ("matrix pivot tolerance", pivot_tolerance),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(DcNetworkError::InvalidQuantity { field, value });
            }
        }
        Ok(Self {
            maximum_topology_age_ns,
            flow_tolerance_mw,
            pivot_tolerance,
            require_n_minus_one,
        })
    }
}

/// One chronological interval. The baseline map contains non-slack injections.
/// Positive flexibility is a data-centre load reduction (injection increase).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcFlexInterval {
    pub interval_start_ns: u64,
    pub interval_duration_ns: u64,
    pub baseline_non_slack_injections_mw: BTreeMap<String, f64>,
    pub load_reductions_mw: BTreeMap<String, f64>,
}

impl DcFlexInterval {
    pub fn new(
        interval_start_ns: u64,
        interval_duration_ns: u64,
        baseline_non_slack_injections_mw: BTreeMap<String, f64>,
        load_reductions_mw: BTreeMap<String, f64>,
    ) -> Result<Self, DcNetworkError> {
        if interval_duration_ns == 0 {
            return Err(DcNetworkError::InvalidIntervalDuration);
        }
        interval_start_ns
            .checked_add(interval_duration_ns)
            .ok_or(DcNetworkError::TimestampOverflow)?;
        for (bus_id, value) in baseline_non_slack_injections_mw
            .iter()
            .chain(load_reductions_mw.iter())
        {
            require_id("injection bus ID", bus_id)?;
            if !value.is_finite() {
                return Err(DcNetworkError::InvalidInjection {
                    bus_id: bus_id.clone(),
                    value: *value,
                });
            }
        }
        Ok(Self {
            interval_start_ns,
            interval_duration_ns,
            baseline_non_slack_injections_mw,
            load_reductions_mw,
        })
    }
}

/// Reproducible input provenance for the compared load trajectories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DcFlexibilityIdentity {
    site_id: String,
    pcc_id: String,
    baseline_trace_digest: String,
    flexible_trace_digest: String,
}

impl DcFlexibilityIdentity {
    pub fn new(
        site_id: impl Into<String>,
        pcc_id: impl Into<String>,
        baseline_trace_digest: impl Into<String>,
        flexible_trace_digest: impl Into<String>,
    ) -> Result<Self, DcNetworkError> {
        let identity = Self {
            site_id: site_id.into(),
            pcc_id: pcc_id.into(),
            baseline_trace_digest: baseline_trace_digest.into(),
            flexible_trace_digest: flexible_trace_digest.into(),
        };
        for (field, value) in [
            ("site ID", identity.site_id.as_str()),
            ("PCC ID", identity.pcc_id.as_str()),
            ("baseline trace digest", identity.baseline_trace_digest.as_str()),
            ("flexible trace digest", identity.flexible_trace_digest.as_str()),
        ] {
            require_id(field, value)?;
        }
        if identity.baseline_trace_digest == identity.flexible_trace_digest {
            return Err(DcNetworkError::TraceDigestCollision);
        }
        Ok(identity)
    }
}

/// Branch flow before/after flexibility. Flow is signed from branch from_bus
/// to to_bus; relief is positive when absolute flow magnitude falls.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcBranchFlowReceipt {
    pub branch_id: String,
    pub from_bus: String,
    pub to_bus: String,
    pub baseline_flow_mw: f64,
    pub flexible_flow_mw: f64,
    pub flow_delta_mw: f64,
    pub absolute_flow_relief_mw: f64,
    pub normal_limit_mw: f64,
    pub baseline_over_limit: bool,
    pub flexible_over_limit: bool,
}

/// N-1 outage result. An islanded network is reported as not deliverable;
/// it is not silently omitted from the contingency set.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcContingencyReceipt {
    pub outaged_branch_id: String,
    pub feasible: bool,
    pub islanded_or_singular: bool,
    pub overloaded_branch_ids: Vec<String>,
    pub diagnostic: Option<String>,
}

/// Per-interval locational network response.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcNetworkIntervalReceipt {
    pub interval_start_ns: u64,
    pub interval_duration_ns: u64,
    pub requested_load_reductions_mw: BTreeMap<String, f64>,
    pub baseline_slack_injection_mw: f64,
    pub flexible_slack_injection_mw: f64,
    pub branch_flows: Vec<DcBranchFlowReceipt>,
    pub n_minus_one: Vec<DcContingencyReceipt>,
    pub basecase_flexible_feasible: bool,
    pub n_minus_one_feasible: bool,
    pub interval_network_deliverable: bool,
}

/// Summary keeps site response distinct from network deliverability.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DcNetworkDeliverabilityReport {
    pub identity: DcFlexibilityIdentity,
    pub network_model_id: String,
    pub network_model_digest: String,
    pub topology_id: String,
    pub topology_digest: String,
    pub topology_effective_at_ns: u64,
    pub slack_bus_id: String,
    pub interval_count: usize,
    pub requested_load_reduction_energy_mwh: f64,
    pub maximum_requested_load_reduction_mw: f64,
    pub basecase_feasible_interval_count: usize,
    pub n_minus_one_feasible_interval_count: usize,
    pub network_deliverable_interval_count: usize,
    pub network_deliverable: bool,
    pub n_minus_one_required: bool,
    pub claim_boundary: String,
    pub intervals: Vec<DcNetworkIntervalReceipt>,
}

/// Fail-closed network input and numerical errors.
#[derive(Debug, Clone, PartialEq)]
pub enum DcNetworkError {
    MissingIdentity { field: &'static str },
    InvalidQuantity { field: &'static str, value: f64 },
    InvalidInjection { bus_id: String, value: f64 },
    EmptyNetwork,
    EmptyTrajectory,
    InvalidSlackBusCount { observed: usize },
    DuplicateBus { bus_id: String },
    DuplicateBranch { branch_id: String },
    UnknownBus { bus_id: String },
    UnknownInjectionBus { bus_id: String },
    SlackBusInjectionSupplied { bus_id: String },
    UnknownContingencyBranch { branch_id: String },
    SelfLoop { branch_id: String },
    EmergencyLimitBelowNormal { branch_id: String, normal_limit_mw: f64, emergency_limit_mw: f64 },
    InvalidTopologyAgeLimit,
    StaleTopology { interval_start_ns: u64, topology_effective_at_ns: u64, maximum_age_ns: u64 },
    NonContiguousIntervals { index: usize, expected_start_ns: u64, observed_start_ns: u64 },
    InvalidIntervalDuration,
    MissingRequiredContingencies,
    SingularOrIslandedTopology { outage_branch_id: Option<String>, pivot: f64 },
    PowerMatrixDimensionMismatch,
    TimestampOverflow,
    NumericalOverflow(&'static str),
    TraceDigestCollision,
}

impl fmt::Display for DcNetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { field } => write!(f, "{field} must not be empty"),
            Self::InvalidQuantity { field, value } => write!(f, "{field} must be finite and positive; got {value}"),
            Self::InvalidInjection { bus_id, value } => write!(f, "injection at bus {bus_id} is non-finite: {value}"),
            Self::EmptyNetwork => write!(f, "network requires at least one bus"),
            Self::EmptyTrajectory => write!(f, "network deliverability requires at least one interval"),
            Self::InvalidSlackBusCount { observed } => write!(f, "network requires exactly one slack bus; got {observed}"),
            Self::DuplicateBus { bus_id } => write!(f, "duplicate bus ID {bus_id}"),
            Self::DuplicateBranch { branch_id } => write!(f, "duplicate branch ID {branch_id}"),
            Self::UnknownBus { bus_id } => write!(f, "network references unknown bus {bus_id}"),
            Self::UnknownInjectionBus { bus_id } => write!(f, "injection references unknown bus {bus_id}"),
            Self::SlackBusInjectionSupplied { bus_id } => write!(f, "slack bus {bus_id} is balanced implicitly and must not appear in the injection map"),
            Self::UnknownContingencyBranch { branch_id } => write!(f, "contingency references unknown branch {branch_id}"),
            Self::SelfLoop { branch_id } => write!(f, "branch {branch_id} connects a bus to itself"),
            Self::EmergencyLimitBelowNormal { branch_id, normal_limit_mw, emergency_limit_mw } => write!(f, "branch {branch_id} emergency limit {emergency_limit_mw} MW is below normal limit {normal_limit_mw} MW"),
            Self::InvalidTopologyAgeLimit => write!(f, "maximum topology age must be positive"),
            Self::StaleTopology { interval_start_ns, topology_effective_at_ns, maximum_age_ns } => write!(f, "topology effective at {topology_effective_at_ns} is stale at interval {interval_start_ns}; maximum age is {maximum_age_ns} ns"),
            Self::NonContiguousIntervals { index, expected_start_ns, observed_start_ns } => write!(f, "network interval {index} starts at {observed_start_ns}; expected contiguous start {expected_start_ns}"),
            Self::InvalidIntervalDuration => write!(f, "interval duration must be positive"),
            Self::MissingRequiredContingencies => write!(f, "N-1 policy requires an explicit non-empty contingency set"),
            Self::SingularOrIslandedTopology { outage_branch_id, pivot } => write!(f, "DC susceptance matrix is singular/islanded for outage {outage_branch_id:?}; pivot {pivot}"),
            Self::PowerMatrixDimensionMismatch => write!(f, "DC susceptance matrix and injection vector dimensions differ"),
            Self::TimestampOverflow => write!(f, "timestamp arithmetic overflow"),
            Self::NumericalOverflow(field) => write!(f, "numerical overflow computing {field}"),
            Self::TraceDigestCollision => write!(f, "baseline and flexible trace digests must differ"),
        }
    }
}

impl std::error::Error for DcNetworkError {}

/// Screen a complete chronological paired PCC injection trajectory against
/// the same frozen network and an explicit N-1 contingency set.
pub fn evaluate_dc_network_deliverability(
    identity: &DcFlexibilityIdentity,
    network: &DcNetworkModel,
    contract: &DcNetworkContract,
    intervals: &[DcFlexInterval],
    n_minus_one_contingencies: &[String],
) -> Result<DcNetworkDeliverabilityReport, DcNetworkError> {
    if intervals.is_empty() {
        return Err(DcNetworkError::EmptyTrajectory);
    }
    if contract.require_n_minus_one && n_minus_one_contingencies.is_empty() {
        return Err(DcNetworkError::MissingRequiredContingencies);
    }
    let branch_ids: BTreeSet<&str> = network.branches.iter().map(|branch| branch.id.as_str()).collect();
    let mut unique_contingencies = BTreeSet::new();
    for branch_id in n_minus_one_contingencies {
        if !branch_ids.contains(branch_id.as_str()) {
            return Err(DcNetworkError::UnknownContingencyBranch {
                branch_id: branch_id.clone(),
            });
        }
        if !unique_contingencies.insert(branch_id.as_str()) {
            return Err(DcNetworkError::UnknownContingencyBranch {
                branch_id: branch_id.clone(),
            });
        }
    }
    let mut previous_end_ns: Option<u64> = None;
    let mut receipts = Vec::with_capacity(intervals.len());
    let mut requested_reduction_energy_mwh = 0.0;
    let mut maximum_requested_load_reduction_mw: f64 = 0.0;
    let mut basecase_feasible_interval_count = 0usize;
    let mut n_minus_one_feasible_interval_count = 0usize;
    let mut network_deliverable_interval_count = 0usize;

    for (index, interval) in intervals.iter().enumerate() {
        if let Some(expected_start_ns) = previous_end_ns
            && interval.interval_start_ns != expected_start_ns
        {
            return Err(DcNetworkError::NonContiguousIntervals {
                index,
                expected_start_ns,
                observed_start_ns: interval.interval_start_ns,
            });
        }
        let end_ns = interval.interval_start_ns
            .checked_add(interval.interval_duration_ns)
            .ok_or(DcNetworkError::TimestampOverflow)?;
        previous_end_ns = Some(end_ns);
        if interval.interval_start_ns < network.topology_effective_at_ns {
            return Err(DcNetworkError::StaleTopology {
                interval_start_ns: interval.interval_start_ns,
                topology_effective_at_ns: network.topology_effective_at_ns,
                maximum_age_ns: contract.maximum_topology_age_ns,
            });
        }
        let topology_age_ns = interval.interval_start_ns - network.topology_effective_at_ns;
        if topology_age_ns > contract.maximum_topology_age_ns {
            return Err(DcNetworkError::StaleTopology {
                interval_start_ns: interval.interval_start_ns,
                topology_effective_at_ns: network.topology_effective_at_ns,
                maximum_age_ns: contract.maximum_topology_age_ns,
            });
        }
        validate_injection_map(network, &interval.baseline_non_slack_injections_mw)?;
        validate_injection_map(network, &interval.load_reductions_mw)?;

        let mut flexible_injections = interval.baseline_non_slack_injections_mw.clone();
        for (bus_id, reduction_mw) in &interval.load_reductions_mw {
            *flexible_injections.entry(bus_id.clone()).or_insert(0.0) += reduction_mw;
        }

        let baseline_flows = solve_dc_flows(network, &interval.baseline_non_slack_injections_mw, None, contract.pivot_tolerance)?;
        let flexible_flows = solve_dc_flows(network, &flexible_injections, None, contract.pivot_tolerance)?;
        let branch_flows = build_branch_flow_receipts(network, &baseline_flows, &flexible_flows, contract.flow_tolerance_mw)?;
        let basecase_flexible_feasible = branch_flows.iter().all(|flow| !flow.flexible_over_limit);
        if basecase_flexible_feasible {
            basecase_feasible_interval_count += 1;
        }

        let mut contingency_receipts = Vec::with_capacity(n_minus_one_contingencies.len());
        for outage in n_minus_one_contingencies {
            match solve_dc_flows(network, &flexible_injections, Some(outage), contract.pivot_tolerance) {
                Ok(flows) => {
                    let overloaded: Vec<String> = flows
                        .iter()
                        .filter_map(|flow| {
                            let branch = network.branches.iter().find(|branch| branch.id == flow.branch_id)?;
                            (flow.flow_mw.abs() > branch.emergency_limit_mw + contract.flow_tolerance_mw)
                                .then(|| branch.id.clone())
                        })
                        .collect();
                    contingency_receipts.push(DcContingencyReceipt {
                        outaged_branch_id: outage.clone(),
                        feasible: overloaded.is_empty(),
                        islanded_or_singular: false,
                        overloaded_branch_ids: overloaded,
                        diagnostic: None,
                    });
                }
                Err(DcNetworkError::SingularOrIslandedTopology { pivot, .. }) => {
                    contingency_receipts.push(DcContingencyReceipt {
                        outaged_branch_id: outage.clone(),
                        feasible: false,
                        islanded_or_singular: true,
                        overloaded_branch_ids: Vec::new(),
                        diagnostic: Some(format!("islanded or singular DC topology (pivot {pivot})")),
                    });
                }
                Err(error) => return Err(error),
            }
        }
        let n_minus_one_feasible = contingency_receipts.iter().all(|receipt| receipt.feasible);
        if n_minus_one_feasible {
            n_minus_one_feasible_interval_count += 1;
        }
        let interval_network_deliverable = basecase_flexible_feasible
            && (!contract.require_n_minus_one || n_minus_one_feasible);
        if interval_network_deliverable {
            network_deliverable_interval_count += 1;
        }
        let requested_reduction_mw = interval.load_reductions_mw.values().sum::<f64>();
        maximum_requested_load_reduction_mw =
            maximum_requested_load_reduction_mw.max(requested_reduction_mw.abs());
        let duration_hours = interval.interval_duration_ns as f64 / 3_600_000_000_000.0;
        requested_reduction_energy_mwh += requested_reduction_mw * duration_hours / 1_000.0;
        if !requested_reduction_energy_mwh.is_finite() {
            return Err(DcNetworkError::NumericalOverflow("requested flexibility energy"));
        }

        let baseline_slack_injection_mw =
            -interval.baseline_non_slack_injections_mw.values().sum::<f64>();
        let flexible_slack_injection_mw = -flexible_injections.values().sum::<f64>();
        receipts.push(DcNetworkIntervalReceipt {
            interval_start_ns: interval.interval_start_ns,
            interval_duration_ns: interval.interval_duration_ns,
            requested_load_reductions_mw: interval.load_reductions_mw.clone(),
            baseline_slack_injection_mw,
            flexible_slack_injection_mw,
            branch_flows,
            n_minus_one: contingency_receipts,
            basecase_flexible_feasible,
            n_minus_one_feasible,
            interval_network_deliverable,
        });
    }

    let network_deliverable = network_deliverable_interval_count == intervals.len();
    Ok(DcNetworkDeliverabilityReport {
        identity: identity.clone(),
        network_model_id: network.model_id.clone(),
        network_model_digest: network.model_digest.clone(),
        topology_id: network.topology_id.clone(),
        topology_digest: network.topology_digest.clone(),
        topology_effective_at_ns: network.topology_effective_at_ns,
        slack_bus_id: network.slack_bus_id.clone(),
        interval_count: intervals.len(),
        requested_load_reduction_energy_mwh: requested_reduction_energy_mwh,
        maximum_requested_load_reduction_mw,
        basecase_feasible_interval_count,
        n_minus_one_feasible_interval_count,
        network_deliverable_interval_count,
        network_deliverable,
        n_minus_one_required: contract.require_n_minus_one,
        claim_boundary: "deterministic active-power DC network screening only; not AC feasibility, voltage/reactive validation, protection, RMS/EMT stability, ELCC or market accreditation".to_owned(),
        intervals: receipts,
    })
}

#[derive(Debug, Clone, PartialEq)]
struct SolvedBranchFlow {
    branch_id: String,
    flow_mw: f64,
}

fn validate_injection_map(
    network: &DcNetworkModel,
    injections: &BTreeMap<String, f64>,
) -> Result<(), DcNetworkError> {
    let bus_by_id: BTreeMap<&str, &DcBus> =
        network.buses.iter().map(|bus| (bus.id.as_str(), bus)).collect();
    for (bus_id, value) in injections {
        let bus = bus_by_id.get(bus_id.as_str()).ok_or_else(|| {
            DcNetworkError::UnknownInjectionBus {
                bus_id: bus_id.clone(),
            }
        })?;
        if bus.is_slack {
            return Err(DcNetworkError::SlackBusInjectionSupplied {
                bus_id: bus_id.clone(),
            });
        }
        if !value.is_finite() {
            return Err(DcNetworkError::InvalidInjection {
                bus_id: bus_id.clone(),
                value: *value,
            });
        }
    }
    Ok(())
}

fn solve_dc_flows(
    network: &DcNetworkModel,
    injections: &BTreeMap<String, f64>,
    outage_branch_id: Option<&str>,
    pivot_tolerance: f64,
) -> Result<Vec<SolvedBranchFlow>, DcNetworkError> {
    let non_slack_buses: Vec<&DcBus> =
        network.buses.iter().filter(|bus| !bus.is_slack).collect();
    let indices: BTreeMap<&str, usize> = non_slack_buses
        .iter()
        .enumerate()
        .map(|(index, bus)| (bus.id.as_str(), index))
        .collect();
    let dimension = non_slack_buses.len();
    let mut matrix = vec![vec![0.0; dimension]; dimension];
    let mut rhs = vec![0.0; dimension];
    for (bus_id, value) in injections {
        let index = *indices.get(bus_id.as_str()).ok_or_else(|| {
            DcNetworkError::UnknownInjectionBus {
                bus_id: bus_id.clone(),
            }
        })?;
        rhs[index] = *value;
    }

    for branch in &network.branches {
        if outage_branch_id == Some(branch.id.as_str()) {
            continue;
        }
        let susceptance = 1.0 / branch.reactance_pu;
        let from = indices.get(branch.from_bus.as_str()).copied();
        let to = indices.get(branch.to_bus.as_str()).copied();
        if let Some(index) = from {
            matrix[index][index] += susceptance;
        }
        if let Some(index) = to {
            matrix[index][index] += susceptance;
        }
        if let (Some(from_index), Some(to_index)) = (from, to) {
            matrix[from_index][to_index] -= susceptance;
            matrix[to_index][from_index] -= susceptance;
        }
    }
    let angles = solve_linear_system(matrix, rhs, pivot_tolerance, outage_branch_id)?;
    let angle_by_bus: BTreeMap<&str, f64> = non_slack_buses
        .iter()
        .enumerate()
        .map(|(index, bus)| (bus.id.as_str(), angles[index]))
        .collect();
    let mut receipts = Vec::new();
    for branch in &network.branches {
        let flow_mw = if outage_branch_id == Some(branch.id.as_str()) {
            continue;
        } else {
            let from_angle = angle_by_bus.get(branch.from_bus.as_str()).copied().unwrap_or(0.0);
            let to_angle = angle_by_bus.get(branch.to_bus.as_str()).copied().unwrap_or(0.0);
            (from_angle - to_angle) / branch.reactance_pu
        };
        if !flow_mw.is_finite() {
            return Err(DcNetworkError::NumericalOverflow("branch flow"));
        }
        receipts.push(SolvedBranchFlow {
            branch_id: branch.id.clone(),
            flow_mw,
        });
    }
    Ok(receipts)
}

fn solve_linear_system(
    mut matrix: Vec<Vec<f64>>,
    mut rhs: Vec<f64>,
    pivot_tolerance: f64,
    outage_branch_id: Option<&str>,
) -> Result<Vec<f64>, DcNetworkError> {
    let n = matrix.len();
    if rhs.len() != n || matrix.iter().any(|row| row.len() != n) {
        return Err(DcNetworkError::PowerMatrixDimensionMismatch);
    }
    for column in 0..n {
        let mut pivot_row = column;
        for row in (column + 1)..n {
            if matrix[row][column].abs() > matrix[pivot_row][column].abs() {
                pivot_row = row;
            }
        }
        let pivot = matrix[pivot_row][column].abs();
        if !pivot.is_finite() || pivot <= pivot_tolerance {
            return Err(DcNetworkError::SingularOrIslandedTopology {
                outage_branch_id: outage_branch_id.map(str::to_owned),
                pivot,
            });
        }
        if pivot_row != column {
            matrix.swap(pivot_row, column);
            rhs.swap(pivot_row, column);
        }
        let diagonal = matrix[column][column];
        for col in column..n {
            matrix[column][col] /= diagonal;
        }
        rhs[column] /= diagonal;
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for col in column..n {
                matrix[row][col] -= factor * matrix[column][col];
            }
            rhs[row] -= factor * rhs[column];
        }
    }
    if rhs.iter().any(|value| !value.is_finite()) {
        return Err(DcNetworkError::NumericalOverflow("bus voltage angle"));
    }
    Ok(rhs)
}

fn build_branch_flow_receipts(
    network: &DcNetworkModel,
    baseline_flows: &[SolvedBranchFlow],
    flexible_flows: &[SolvedBranchFlow],
    tolerance_mw: f64,
) -> Result<Vec<DcBranchFlowReceipt>, DcNetworkError> {
    let baseline: BTreeMap<&str, f64> = baseline_flows
        .iter()
        .map(|flow| (flow.branch_id.as_str(), flow.flow_mw))
        .collect();
    let flexible: BTreeMap<&str, f64> = flexible_flows
        .iter()
        .map(|flow| (flow.branch_id.as_str(), flow.flow_mw))
        .collect();
    let mut receipts = Vec::with_capacity(network.branches.len());
    for branch in &network.branches {
        let baseline_flow_mw = *baseline.get(branch.id.as_str()).ok_or_else(|| {
            DcNetworkError::UnknownBranchFlow {
                branch_id: branch.id.clone(),
            }
        })?;
        let flexible_flow_mw = *flexible.get(branch.id.as_str()).ok_or_else(|| {
            DcNetworkError::UnknownBranchFlow {
                branch_id: branch.id.clone(),
            }
        })?;
        let flow_delta_mw = flexible_flow_mw - baseline_flow_mw;
        let absolute_flow_relief_mw = baseline_flow_mw.abs() - flexible_flow_mw.abs();
        if !flow_delta_mw.is_finite() || !absolute_flow_relief_mw.is_finite() {
            return Err(DcNetworkError::NumericalOverflow("branch flow delta"));
        }
        receipts.push(DcBranchFlowReceipt {
            branch_id: branch.id.clone(),
            from_bus: branch.from_bus.clone(),
            to_bus: branch.to_bus.clone(),
            baseline_flow_mw,
            flexible_flow_mw,
            flow_delta_mw,
            absolute_flow_relief_mw,
            normal_limit_mw: branch.normal_limit_mw,
            baseline_over_limit: baseline_flow_mw.abs() > branch.normal_limit_mw + tolerance_mw,
            flexible_over_limit: flexible_flow_mw.abs() > branch.normal_limit_mw + tolerance_mw,
        });
    }
    Ok(receipts)
}

fn mean_not_used() {}

fn require_id(field: &'static str, value: &str) -> Result<(), DcNetworkError> {
    if value.trim().is_empty() {
        Err(DcNetworkError::MissingIdentity { field })
    } else {
        Ok(())
    }
}
