//! Fault-domain-aware recovery topology reference auditor.
//!
//! This crate consumes caller-supplied, identity-bound evidence from upstream
//! viability and transition authorities. It does not generate physical
//! transitions, compute viability, authenticate receipts, or certify safety.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_INPUT_ROUTES: usize = 1_024;
pub const MAX_DOMAIN_RECORDS: usize = 256;
pub const MAX_ROUTE_CLASSES: usize = 24;
pub const MAX_FAILURE_GROUPS: usize = 16;
pub const MAX_BRANCHES_PER_ROUTE: usize = 4_096;
pub const MAX_STATES_PER_ROUTE: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDisposition {
    Admitted,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalStateDisposition {
    Viable,
    Nonviable,
    Unknown,
}

/// A component/domain identity with a source-qualified common-cause group.
/// Multiple components in one common-cause group count as one failure domain
/// for route-independence and cut-set analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureDomain {
    pub domain_id: String,
    pub failure_group_id: String,
    pub provenance_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntermediateStateEvidence {
    pub state_id: String,
    pub disposition: PhysicalStateDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuccessorEvidence {
    pub state_id: String,
    pub physically_possible: bool,
    pub disposition: PhysicalStateDisposition,
}

/// One candidate route emitted by an upstream planner/oracle composition.
/// IDs are references, not cryptographic authentication. The caller remains
/// responsible for authenticating and validating upstream evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryRoute {
    pub route_id: String,
    pub action_ids: Vec<String>,
    /// Ordered, source-bound physical transition identities. Action labels are
    /// deliberately excluded from route equivalence.
    pub physical_edge_ids: Vec<String>,
    pub terminal_region_id: String,
    /// Component/domain IDs; the audit expands these to common-cause groups.
    pub failure_domain_ids: Vec<String>,
    pub admission: GateDisposition,
    pub physical_branch_coverage_complete: bool,
    /// Digest/reference to the complete physical successor manifest.
    pub branch_manifest_digest: String,
    pub latency_prefix_complete: bool,
    /// Digest/reference to the complete time-expanded latency-prefix manifest.
    pub latency_prefix_manifest_digest: String,
    pub intermediate_state_inventory_complete: bool,
    pub intermediate_states: Vec<IntermediateStateEvidence>,
    pub successors: Vec<SuccessorEvidence>,
    pub worst_case_steps: Option<u32>,
    pub worst_case_energy_mj: Option<u64>,
    pub context_digest: String,
    pub viability_receipt_id: String,
    pub viability_valid_through_tick: u64,
    pub transition_receipt_id: String,
    pub transition_valid_through_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyInput {
    pub context_digest: String,
    /// Exact source state from which every candidate route begins.
    pub start_state_id: String,
    /// Identity of the namespace in which physical_edge_ids are unique.
    pub edge_namespace_digest: String,
    /// Exact #4083 transition-abstraction/model revision for these edges.
    pub transition_abstraction_digest: String,
    pub evaluation_tick: u64,
    pub max_recovery_steps: Option<u32>,
    pub available_recovery_energy_mj: Option<u64>,
    pub failure_domains: Vec<FailureDomain>,
    pub routes: Vec<RecoveryRoute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualificationStatus {
    Qualified,
    RejectedNoRecoveryRoute,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RouteFinding {
    RejectedByUpstream { route_id: String },
    MissingBranchCoverage { route_id: String },
    IncompleteLatencyPrefix { route_id: String },
    NonviableIntermediate { route_id: String, state_id: String },
    UnknownIntermediate { route_id: String, state_id: String },
    PossibleNonviableSuccessor { route_id: String, state_id: String },
    UnknownSuccessor { route_id: String, state_id: String },
    MissingSuccessorInventory { route_id: String },
    MissingRouteResourceMeasurement { route_id: String },
    StepBudgetExceeded { route_id: String },
    EnergyBudgetExceeded { route_id: String },
    MissingFailureDomain { route_id: String, domain_id: String },
    EmptyFailureDomainSet { route_id: String },
    StaleUpstreamReceipt { route_id: String },
    MissingReceipt { route_id: String },
    UpstreamAdmissionUnknown { route_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalRouteClass {
    pub start_state_id: String,
    pub edge_namespace_digest: String,
    pub transition_abstraction_digest: String,
    /// Canonical identity is the ordered physical-edge sequence plus the
    /// source-bound terminal-region identity.
    pub physical_edge_ids: Vec<String>,
    pub terminal_region_id: String,
    pub represented_route_ids: Vec<String>,
    pub common_cause_dependencies: Vec<String>,
    pub worst_case_steps: u32,
    pub worst_case_energy_mj: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyReport {
    pub context_digest: String,
    pub status: QualificationStatus,
    /// Known complete route classes. Under Unknown status these are only a
    /// lower bound; global topology metrics are intentionally suppressed.
    pub known_route_classes: Vec<PhysicalRouteClass>,
    pub independent_route_count: Option<usize>,
    pub minimum_cut_set_size: Option<usize>,
    pub all_minimal_cut_sets: Vec<Vec<String>>,
    pub distinct_terminal_regions: Vec<String>,
    pub findings: Vec<RouteFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    EmptyContextDigest,
    EmptyStartStateId,
    EmptyEdgeNamespaceDigest,
    EmptyTransitionAbstractionDigest,
    EmptyDomainId,
    EmptyFailureGroupId,
    EmptyDomainProvenance,
    DuplicateDomainId(String),
    EmptyRouteId,
    EmptyActionId(String),
    DuplicateRouteId(String),
    EmptyRouteContext(String),
    RouteContextMismatch(String),
    EmptyPhysicalPath(String),
    EmptyPhysicalEdgeId(String),
    EmptyTerminalRegion(String),
    TooManyInputRoutes { actual: usize, limit: usize },
    TooManyDomainRecords { actual: usize, limit: usize },
    TooManyRouteClasses { actual: usize, limit: usize },
    TooManyFailureGroups { actual: usize, limit: usize },
    TooManyBranches { route_id: String, actual: usize, limit: usize },
    TooManyStates { route_id: String, actual: usize, limit: usize },
    EmptySuccessorId { route_id: String },
    DuplicateSuccessorId { route_id: String, state_id: String },
    EmptyIntermediateStateId { route_id: String },
    DuplicateIntermediateStateId { route_id: String, state_id: String },
    MissingGlobalStepBudget,
    MissingGlobalEnergyBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PhysicalPathKey {
    start_state_id: String,
    edge_namespace_digest: String,
    transition_abstraction_digest: String,
    physical_edge_ids: Vec<String>,
    terminal_region_id: String,
}

#[derive(Debug, Clone)]
struct QualifiedRoute {
    route_id: String,
    key: PhysicalPathKey,
    groups: BTreeSet<String>,
    steps: u32,
    energy_mj: u64,
}

/// Validate, quotient action aliases into physical route classes, then compute
/// exact bounded domain-disjoint route packing and minimal group cut sets.
///
/// Any unresolved route can add or alter recovery topology, so it suppresses
/// global independence/cut-set metrics. Known valid route classes remain
/// available as a lower-bound diagnostic.
pub fn audit(input: &TopologyInput) -> Result<TopologyReport, AuditError> {
    validate_global_input(input)?;

    let domain_by_id = build_domain_map(&input.failure_domains);
    let mut route_ids = BTreeSet::new();
    let mut findings = Vec::new();
    let mut unknown_route_seen = false;
    let mut qualified = Vec::new();

    for route in &input.routes {
        if route.route_id.trim().is_empty() {
            return Err(AuditError::EmptyRouteId);
        }
        if !route_ids.insert(route.route_id.clone()) {
            return Err(AuditError::DuplicateRouteId(route.route_id.clone()));
        }
        validate_route_identity(route, &input.context_digest)?;
        match qualify_route(route, input, &domain_by_id, &mut findings)? {
            RouteQualification::Qualified(route) => qualified.push(route),
            RouteQualification::Rejected => {}
            RouteQualification::Unknown => unknown_route_seen = true,
        }
    }

    let classes = quotient_physical_paths(qualified);
    if classes.len() > MAX_ROUTE_CLASSES {
        return Err(AuditError::TooManyRouteClasses {
            actual: classes.len(),
            limit: MAX_ROUTE_CLASSES,
        });
    }

    let mut group_ids = BTreeSet::new();
    for class in &classes {
        group_ids.extend(class.groups.iter().cloned());
    }
    if group_ids.len() > MAX_FAILURE_GROUPS {
        return Err(AuditError::TooManyFailureGroups {
            actual: group_ids.len(),
            limit: MAX_FAILURE_GROUPS,
        });
    }

    let known_route_classes = classes
        .iter()
        .map(|class| PhysicalRouteClass {
            start_state_id: class.key.start_state_id.clone(),
            edge_namespace_digest: class.key.edge_namespace_digest.clone(),
            transition_abstraction_digest: class.key.transition_abstraction_digest.clone(),
            physical_edge_ids: class.key.physical_edge_ids.clone(),
            terminal_region_id: class.key.terminal_region_id.clone(),
            represented_route_ids: class.route_ids.iter().cloned().collect(),
            common_cause_dependencies: class.groups.iter().cloned().collect(),
            worst_case_steps: class.steps,
            worst_case_energy_mj: class.energy_mj,
        })
        .collect::<Vec<_>>();

    let distinct_terminal_regions = classes
        .iter()
        .map(|class| class.key.terminal_region_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    // The exact counts are intentionally suppressed when an unresolved route
    // might add a route or conceal a shared dependency.
    let (independent_route_count, minimum_cut_set_size, all_minimal_cut_sets, status) =
        if unknown_route_seen {
            (None, None, Vec::new(), QualificationStatus::Unknown)
        } else if classes.is_empty() {
            (
                Some(0),
                None,
                Vec::new(),
                QualificationStatus::RejectedNoRecoveryRoute,
            )
        } else {
            let dependencies = classes
                .iter()
                .map(|class| class.groups.clone())
                .collect::<Vec<_>>();
            let independent = maximum_pairwise_disjoint_routes(&dependencies);
            let cuts = minimal_hitting_sets(&dependencies);
            let min_cut = cuts.first().map(BTreeSet::len);
            let cuts_out = cuts
                .into_iter()
                .map(|cut| cut.into_iter().collect::<Vec<_>>())
                .collect();
            (
                Some(independent),
                min_cut,
                cuts_out,
                QualificationStatus::Qualified,
            )
        };

    findings.sort();
    findings.dedup();
    Ok(TopologyReport {
        context_digest: input.context_digest.clone(),
        status,
        known_route_classes,
        independent_route_count,
        minimum_cut_set_size,
        all_minimal_cut_sets,
        distinct_terminal_regions,
        findings,
    })
}

enum RouteQualification {
    Qualified(QualifiedRoute),
    Rejected,
    Unknown,
}

fn validate_global_input(input: &TopologyInput) -> Result<(), AuditError> {
    if input.context_digest.trim().is_empty() {
        return Err(AuditError::EmptyContextDigest);
    }
    if input.start_state_id.trim().is_empty() {
        return Err(AuditError::EmptyStartStateId);
    }
    if input.edge_namespace_digest.trim().is_empty() {
        return Err(AuditError::EmptyEdgeNamespaceDigest);
    }
    if input.transition_abstraction_digest.trim().is_empty() {
        return Err(AuditError::EmptyTransitionAbstractionDigest);
    }
    if input.routes.len() > MAX_INPUT_ROUTES {
        return Err(AuditError::TooManyInputRoutes {
            actual: input.routes.len(),
            limit: MAX_INPUT_ROUTES,
        });
    }
    if input.failure_domains.len() > MAX_DOMAIN_RECORDS {
        return Err(AuditError::TooManyDomainRecords {
            actual: input.failure_domains.len(),
            limit: MAX_DOMAIN_RECORDS,
        });
    }
    if input.max_recovery_steps.is_none() {
        return Err(AuditError::MissingGlobalStepBudget);
    }
    if input.available_recovery_energy_mj.is_none() {
        return Err(AuditError::MissingGlobalEnergyBudget);
    }

    let mut seen = BTreeSet::new();
    for domain in &input.failure_domains {
        if domain.domain_id.trim().is_empty() {
            return Err(AuditError::EmptyDomainId);
        }
        if domain.failure_group_id.trim().is_empty() {
            return Err(AuditError::EmptyFailureGroupId);
        }
        if domain.provenance_ref.trim().is_empty() {
            return Err(AuditError::EmptyDomainProvenance);
        }
        if !seen.insert(domain.domain_id.clone()) {
            return Err(AuditError::DuplicateDomainId(domain.domain_id.clone()));
        }
    }
    Ok(())
}

fn build_domain_map(domains: &[FailureDomain]) -> BTreeMap<&str, &FailureDomain> {
    domains
        .iter()
        .map(|domain| (domain.domain_id.as_str(), domain))
        .collect()
}

fn validate_route_identity(
    route: &RecoveryRoute,
    context_digest: &str,
) -> Result<(), AuditError> {
    if route.action_ids.is_empty() || route.action_ids.iter().any(|id| id.trim().is_empty()) {
        return Err(AuditError::EmptyActionId(route.route_id.clone()));
    }
    if route.context_digest.trim().is_empty() {
        return Err(AuditError::EmptyRouteContext(route.route_id.clone()));
    }
    if route.context_digest != context_digest {
        return Err(AuditError::RouteContextMismatch(route.route_id.clone()));
    }
    if route.physical_edge_ids.is_empty() {
        return Err(AuditError::EmptyPhysicalPath(route.route_id.clone()));
    }
    let mut intermediate_ids = BTreeSet::new();
    for state in &route.intermediate_states {
        if state.state_id.trim().is_empty() {
            return Err(AuditError::EmptyIntermediateStateId { route_id: route.route_id.clone() });
        }
        if !intermediate_ids.insert(state.state_id.clone()) {
            return Err(AuditError::DuplicateIntermediateStateId { route_id: route.route_id.clone(), state_id: state.state_id.clone() });
        }
    }
    let mut successor_ids = BTreeSet::new();
    for successor in &route.successors {
        if successor.state_id.trim().is_empty() {
            return Err(AuditError::EmptySuccessorId { route_id: route.route_id.clone() });
        }
        if !successor_ids.insert(successor.state_id.clone()) {
            return Err(AuditError::DuplicateSuccessorId { route_id: route.route_id.clone(), state_id: successor.state_id.clone() });
        }
    }
    if route
        .physical_edge_ids
        .iter()
        .any(|edge| edge.trim().is_empty())
    {
        return Err(AuditError::EmptyPhysicalEdgeId(route.route_id.clone()));
    }
    if route.terminal_region_id.trim().is_empty() {
        return Err(AuditError::EmptyTerminalRegion(route.route_id.clone()));
    }
    if route.successors.len() > MAX_BRANCHES_PER_ROUTE {
        return Err(AuditError::TooManyBranches {
            route_id: route.route_id.clone(),
            actual: route.successors.len(),
            limit: MAX_BRANCHES_PER_ROUTE,
        });
    }
    if route.intermediate_states.len() > MAX_STATES_PER_ROUTE {
        return Err(AuditError::TooManyStates {
            route_id: route.route_id.clone(),
            actual: route.intermediate_states.len(),
            limit: MAX_STATES_PER_ROUTE,
        });
    }
    Ok(())
}

fn qualify_route(
    route: &RecoveryRoute,
    input: &TopologyInput,
    domain_by_id: &BTreeMap<&str, &FailureDomain>,
    findings: &mut Vec<RouteFinding>,
) -> Result<RouteQualification, AuditError> {
    let mut rejected = false;
    let mut unknown = false;

    macro_rules! reject {
        ($finding:expr) => {{
            findings.push($finding);
            rejected = true;
        }};
    }
    macro_rules! mark_unknown {
        ($finding:expr) => {{
            findings.push($finding);
            unknown = true;
        }};
    }

    match route.admission {
        GateDisposition::Admitted => {}
        GateDisposition::Rejected => reject!(RouteFinding::RejectedByUpstream {
            route_id: route.route_id.clone(),
        }),
        GateDisposition::Unknown => mark_unknown!(RouteFinding::UpstreamAdmissionUnknown {
            route_id: route.route_id.clone(),
        }),
    }

    if route.branch_manifest_digest.trim().is_empty() {
        mark_unknown!(RouteFinding::MissingBranchCoverage { route_id: route.route_id.clone() });
    }
    if route.latency_prefix_manifest_digest.trim().is_empty() {
        mark_unknown!(RouteFinding::IncompleteLatencyPrefix { route_id: route.route_id.clone() });
    }
    if !route.physical_branch_coverage_complete {
        mark_unknown!(RouteFinding::MissingBranchCoverage {
            route_id: route.route_id.clone(),
        });
    }
    if !route.latency_prefix_complete {
        mark_unknown!(RouteFinding::IncompleteLatencyPrefix {
            route_id: route.route_id.clone(),
        });
    }
    if route.viability_receipt_id.trim().is_empty()
        || route.transition_receipt_id.trim().is_empty()
    {
        mark_unknown!(RouteFinding::MissingReceipt {
            route_id: route.route_id.clone(),
        });
    }
    if route.viability_valid_through_tick < input.evaluation_tick
        || route.transition_valid_through_tick < input.evaluation_tick
    {
        mark_unknown!(RouteFinding::StaleUpstreamReceipt {
            route_id: route.route_id.clone(),
        });
    }

    if !route.intermediate_state_inventory_complete {
        mark_unknown!(RouteFinding::UnknownIntermediate {
            route_id: route.route_id.clone(),
            state_id: "<incomplete-intermediate-state-inventory>".to_owned(),
        });
    }
    for state in &route.intermediate_states {
        match state.disposition {
            PhysicalStateDisposition::Viable => {}
            PhysicalStateDisposition::Nonviable => {
                reject!(RouteFinding::NonviableIntermediate {
                    route_id: route.route_id.clone(),
                    state_id: state.state_id.clone(),
                });
            }
            PhysicalStateDisposition::Unknown => {
                mark_unknown!(RouteFinding::UnknownIntermediate {
                    route_id: route.route_id.clone(),
                    state_id: state.state_id.clone(),
                });
            }
        }
    }

    let physically_possible = route
        .successors
        .iter()
        .filter(|successor| successor.physically_possible)
        .collect::<Vec<_>>();
    if physically_possible.is_empty() {
        mark_unknown!(RouteFinding::MissingSuccessorInventory {
            route_id: route.route_id.clone(),
        });
    }
    for successor in physically_possible {
        match successor.disposition {
            PhysicalStateDisposition::Viable => {}
            PhysicalStateDisposition::Nonviable => {
                reject!(RouteFinding::PossibleNonviableSuccessor {
                    route_id: route.route_id.clone(),
                    state_id: successor.state_id.clone(),
                });
            }
            PhysicalStateDisposition::Unknown => {
                mark_unknown!(RouteFinding::UnknownSuccessor {
                    route_id: route.route_id.clone(),
                    state_id: successor.state_id.clone(),
                });
            }
        }
    }

    let (steps, energy_mj) = match (route.worst_case_steps, route.worst_case_energy_mj) {
        (Some(steps), Some(energy)) => (steps, energy),
        _ => {
            mark_unknown!(RouteFinding::MissingRouteResourceMeasurement {
                route_id: route.route_id.clone(),
            });
            (0, 0)
        }
    };

    if let Some(max_steps) = input.max_recovery_steps {
        if route.worst_case_steps.is_some_and(|steps| steps > max_steps) {
            reject!(RouteFinding::StepBudgetExceeded {
                route_id: route.route_id.clone(),
            });
        }
    }
    if let Some(available) = input.available_recovery_energy_mj {
        if route
            .worst_case_energy_mj
            .is_some_and(|energy| energy > available)
        {
            reject!(RouteFinding::EnergyBudgetExceeded {
                route_id: route.route_id.clone(),
            });
        }
    }

    let mut groups = BTreeSet::new();
    if route.failure_domain_ids.is_empty() {
        mark_unknown!(RouteFinding::EmptyFailureDomainSet {
            route_id: route.route_id.clone(),
        });
    }
    for domain_id in &route.failure_domain_ids {
        match domain_by_id.get(domain_id.as_str()) {
            Some(domain) => {
                groups.insert(domain.failure_group_id.clone());
            }
            None => {
                mark_unknown!(RouteFinding::MissingFailureDomain {
                    route_id: route.route_id.clone(),
                    domain_id: domain_id.clone(),
                });
            }
        }
    }

    if rejected {
        return Ok(RouteQualification::Rejected);
    }
    if unknown || groups.is_empty() {
        return Ok(RouteQualification::Unknown);
    }

    Ok(RouteQualification::Qualified(QualifiedRoute {
        route_id: route.route_id.clone(),
        key: PhysicalPathKey {
            start_state_id: input.start_state_id.clone(),
            edge_namespace_digest: input.edge_namespace_digest.clone(),
            transition_abstraction_digest: input.transition_abstraction_digest.clone(),
            physical_edge_ids: route.physical_edge_ids.clone(),
            terminal_region_id: route.terminal_region_id.clone(),
        },
        groups,
        steps,
        energy_mj,
    }))
}

#[derive(Debug, Clone)]
struct RouteClassAccumulator {
    key: PhysicalPathKey,
    route_ids: BTreeSet<String>,
    groups: BTreeSet<String>,
    steps: u32,
    energy_mj: u64,
}

fn quotient_physical_paths(
    routes: Vec<QualifiedRoute>,
) -> Vec<RouteClassAccumulator> {
    let mut by_key: BTreeMap<PhysicalPathKey, RouteClassAccumulator> = BTreeMap::new();
    for route in routes {
        let entry = by_key
            .entry(route.key.clone())
            .or_insert_with(|| RouteClassAccumulator {
                key: route.key.clone(),
                route_ids: BTreeSet::new(),
                groups: BTreeSet::new(),
                steps: 0,
                energy_mj: 0,
            });
        entry.route_ids.insert(route.route_id);
        // Conservative alias composition: all qualified aliases of one
        // physical path contribute dependencies; aliases never add a route.
        entry.groups.extend(route.groups);
        entry.steps = entry.steps.max(route.steps);
        entry.energy_mj = entry.energy_mj.max(route.energy_mj);
    }
    by_key.into_values().collect()
}

fn maximum_pairwise_disjoint_routes(routes: &[BTreeSet<String>]) -> usize {
    fn visit(
        routes: &[BTreeSet<String>],
        start: usize,
        used: &mut BTreeSet<String>,
        count: usize,
        best: &mut usize,
    ) {
        *best = (*best).max(count);
        if start >= routes.len() || count + routes.len() - start <= *best {
            return;
        }
        for index in start..routes.len() {
            if routes[index].is_disjoint(used) {
                let added = routes[index]
                    .difference(used)
                    .cloned()
                    .collect::<Vec<_>>();
                used.extend(added.iter().cloned());
                visit(routes, index + 1, used, count + 1, best);
                for item in added {
                    used.remove(&item);
                }
            }
        }
    }

    let mut best = 0;
    visit(routes, 0, &mut BTreeSet::new(), 0, &mut best);
    best
}

/// Enumerate every inclusion-minimal hitting set over at most
/// MAX_FAILURE_GROUPS common-cause groups.
fn minimal_hitting_sets(routes: &[BTreeSet<String>]) -> Vec<BTreeSet<String>> {
    if routes.is_empty() || routes.iter().any(BTreeSet::is_empty) {
        return Vec::new();
    }
    let groups = routes
        .iter()
        .flat_map(|route| route.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if groups.len() > MAX_FAILURE_GROUPS {
        return Vec::new();
    }

    let limit = 1usize << groups.len();
    let mut candidates = Vec::new();
    for mask in 1..limit {
        let selected = groups
            .iter()
            .enumerate()
            .filter_map(|(index, group)| ((mask & (1usize << index)) != 0).then_some(group.clone()))
            .collect::<BTreeSet<_>>();
        if routes
            .iter()
            .all(|route| !route.is_disjoint(&selected))
        {
            candidates.push(selected);
        }
    }
    candidates.sort_by(|left, right| {
        left.len()
            .cmp(&right.len())
            .then_with(|| left.iter().cmp(right.iter()))
    });

    let mut minimal: Vec<BTreeSet<String>> = Vec::new();
    for candidate in candidates {
        if !minimal
            .iter()
            .any(|known| known.is_subset(&candidate))
        {
            minimal.push(candidate);
        }
    }
    minimal
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain(id: &str, group: &str) -> FailureDomain {
        FailureDomain {
            domain_id: id.to_owned(),
            failure_group_id: group.to_owned(),
            provenance_ref: format!("source:{id}"),
        }
    }

    fn route(
        id: &str,
        edges: &[&str],
        terminal: &str,
        domains: &[&str],
    ) -> RecoveryRoute {
        RecoveryRoute {
            route_id: id.to_owned(),
            action_ids: vec![format!("action:{id}")],
            physical_edge_ids: edges.iter().map(|value| (*value).to_owned()).collect(),
            terminal_region_id: terminal.to_owned(),
            failure_domain_ids: domains.iter().map(|value| (*value).to_owned()).collect(),
            admission: GateDisposition::Admitted,
            physical_branch_coverage_complete: true,
            branch_manifest_digest: "branch-manifest:v1".to_owned(),
            latency_prefix_complete: true,
            latency_prefix_manifest_digest: "latency-prefix-manifest:v1".to_owned(),
            intermediate_state_inventory_complete: true,
            intermediate_states: vec![IntermediateStateEvidence {
                state_id: format!("intermediate:{id}"),
                disposition: PhysicalStateDisposition::Viable,
            }],
            successors: vec![SuccessorEvidence {
                state_id: format!("successor:{id}"),
                physically_possible: true,
                disposition: PhysicalStateDisposition::Viable,
            }],
            worst_case_steps: Some(3),
            worst_case_energy_mj: Some(40),
            context_digest: "ctx-1".to_owned(),
            viability_receipt_id: format!("viability:{id}"),
            viability_valid_through_tick: 100,
            transition_receipt_id: format!("transition:{id}"),
            transition_valid_through_tick: 100,
        }
    }

    fn input(domains: Vec<FailureDomain>, routes: Vec<RecoveryRoute>) -> TopologyInput {
        TopologyInput {
            context_digest: "ctx-1".to_owned(),
            start_state_id: "state:start".to_owned(),
            edge_namespace_digest: "edge-namespace:v1".to_owned(),
            transition_abstraction_digest: "transition-abstraction:v1".to_owned(),
            evaluation_tick: 10,
            max_recovery_steps: Some(6),
            available_recovery_energy_mj: Some(100),
            failure_domains: domains,
            routes,
        }
    }

    #[test]
    fn r01_action_aliases_collapse_to_one_physical_route() {
        let domains = vec![
            domain("hip_fl", "actuator_hip_fl"),
            domain("bus_a", "power_bus_a"),
        ];
        let a = route("route_a", &["e_start", "e_recover"], "support_tripod_a", &["hip_fl", "bus_a"]);
        let mut b = a.clone();
        b.route_id = "route_b".to_owned();
        b.action_ids = vec!["renamed_policy_action".to_owned()];
        let report = audit(&input(domains, vec![a.clone(), b])).unwrap();
        assert_eq!(report.status, QualificationStatus::Qualified);
        assert_eq!(report.known_route_classes.len(), 1);
        assert_eq!(report.known_route_classes[0].represented_route_ids, vec!["route_a", "route_b"]);
        assert_eq!(report.independent_route_count, Some(1));
        assert_eq!(report.minimum_cut_set_size, Some(1));
        assert_eq!(
            report.all_minimal_cut_sets,
            vec![
                vec!["actuator_hip_fl".to_owned()],
                vec!["power_bus_a".to_owned()]
            ]
        );
    }

    #[test]
    fn r02_shared_power_and_clock_are_single_cut_domains() {
        let domains = vec![
            domain("hip_fl", "actuator_hip_fl"),
            domain("hip_fr", "actuator_hip_fr"),
            domain("bus", "power_bus_shared"),
            domain("imu_clock", "imu_clock_shared"),
        ];
        let left = route("left", &["e0", "e1"], "left_brace", &["hip_fl", "bus", "imu_clock"]);
        let right = route("right", &["e0", "e2"], "right_brace", &["hip_fr", "bus", "imu_clock"]);
        let report = audit(&input(domains, vec![left, right])).unwrap();
        assert_eq!(report.known_route_classes.len(), 2);
        assert_eq!(report.independent_route_count, Some(1));
        assert_eq!(report.minimum_cut_set_size, Some(1));
        assert_eq!(
            report.all_minimal_cut_sets,
            vec![
                vec!["imu_clock_shared".to_owned()],
                vec!["power_bus_shared".to_owned()],
                vec!["actuator_hip_fl".to_owned(), "actuator_hip_fr".to_owned()]
            ]
        );
    }

    #[test]
    fn r03_zero_probability_physical_dead_end_blocks_route() {
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("probe", &["e_probe", "e_recover"], "safe", &["hip"]);
        candidate.successors.push(SuccessorEvidence {
            state_id: "dead_end_zero_mass".to_owned(),
            physically_possible: true,
            disposition: PhysicalStateDisposition::Nonviable,
        });
        let report = audit(&input(domains, vec![candidate])).unwrap();
        assert_eq!(report.status, QualificationStatus::RejectedNoRecoveryRoute);
        assert!(report.known_route_classes.is_empty());
        assert!(report.findings.iter().any(|finding| matches!(
            finding,
            RouteFinding::PossibleNonviableSuccessor { state_id, .. }
                if state_id == "dead_end_zero_mass"
        )));
    }

    #[test]
    fn r04_nonviable_intermediate_latency_state_blocks_route() {
        let domains = vec![domain("knee", "knee")];
        let mut candidate = route("late", &["e_probe", "e_latency", "e_recover"], "safe", &["knee"]);
        candidate.intermediate_states.push(IntermediateStateEvidence {
            state_id: "latency_state".to_owned(),
            disposition: PhysicalStateDisposition::Nonviable,
        });
        let report = audit(&input(domains, vec![candidate])).unwrap();
        assert_eq!(report.status, QualificationStatus::RejectedNoRecoveryRoute);
        assert!(report.known_route_classes.is_empty());
    }

    #[test]
    fn r05_missing_failure_domain_suppresses_global_independence() {
        let domains = vec![domain("hip_fl", "hip_fl"), domain("bus", "power_bus")];
        let known = route("known", &["e1", "e2"], "region_1", &["hip_fl", "bus"]);
        let unknown = route("unknown", &["e3", "e4"], "region_2", &["hip_fr_missing"]);
        let report = audit(&input(domains, vec![known, unknown])).unwrap();
        assert_eq!(report.status, QualificationStatus::Unknown);
        assert_eq!(report.known_route_classes.len(), 1);
        assert_eq!(report.independent_route_count, None);
        assert_eq!(report.minimum_cut_set_size, None);
        assert!(report.all_minimal_cut_sets.is_empty());
        assert!(report.findings.iter().any(|finding| matches!(
            finding,
            RouteFinding::MissingFailureDomain { domain_id, .. }
                if domain_id == "hip_fr_missing"
        )));
    }

    #[test]
    fn r06_recovery_energy_budget_rejects_expensive_route() {
        let domains = vec![domain("ankle", "ankle"), domain("bus", "bus")];
        let mut candidate = route("energy", &["e1", "e2", "e3"], "safe_stop", &["ankle", "bus"]);
        candidate.worst_case_energy_mj = Some(125);
        let report = audit(&input(domains, vec![candidate])).unwrap();
        assert_eq!(report.status, QualificationStatus::RejectedNoRecoveryRoute);
        assert!(report.known_route_classes.is_empty());
        assert!(report.findings.iter().any(|finding| matches!(
            finding,
            RouteFinding::EnergyBudgetExceeded { .. }
        )));
    }

    #[test]
    fn r07_relabeling_and_input_permutation_preserve_results() {
        let domains = vec![
            domain("hip_fl", "hip_fl"),
            domain("hip_fr", "hip_fr"),
            domain("bus", "power"),
        ];
        let a = route("a", &["e0", "e1"], "region_a", &["hip_fl", "bus"]);
        let b = route("b", &["e0", "e2"], "region_b", &["hip_fr", "bus"]);
        let first = audit(&input(domains.clone(), vec![a.clone(), b.clone()])).unwrap();
        let second = audit(&input(
            domains.into_iter().rev().collect(),
            vec![b, a],
        ))
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn r08_stale_viability_receipt_suppresses_qualification() {
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("stale", &["e1"], "safe", &["hip"]);
        candidate.viability_valid_through_tick = 9;
        let report = audit(&input(domains, vec![candidate])).unwrap();
        assert_eq!(report.status, QualificationStatus::Unknown);
        assert_eq!(report.independent_route_count, None);
        assert!(report.findings.iter().any(|finding| matches!(
            finding,
            RouteFinding::StaleUpstreamReceipt { .. }
        )));
    }

    #[test]
    fn r09_duplicate_domain_id_is_invalid() {
        let duplicate = vec![domain("hip", "hip-a"), domain("hip", "hip-b")];
        let result = audit(&input(duplicate, vec![route("r", &["e"], "safe", &["hip"])]));
        assert_eq!(result, Err(AuditError::DuplicateDomainId("hip".to_owned())));
    }

    #[test]
    fn r10_context_mismatch_is_invalid() {
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("r", &["e"], "safe", &["hip"]);
        candidate.context_digest = "different-context".to_owned();
        let result = audit(&input(domains, vec![candidate]));
        assert_eq!(result, Err(AuditError::RouteContextMismatch("r".to_owned())));
    }

    #[test]
    fn r11_common_cause_group_collapses_distinct_components() {
        let domains = vec![
            domain("encoder_position", "shared_encoder_clock"),
            domain("encoder_velocity", "shared_encoder_clock"),
            domain("power", "power_bus"),
        ];
        let a = route("a", &["e1"], "safe_a", &["encoder_position", "power"]);
        let b = route("b", &["e2"], "safe_b", &["encoder_velocity", "power"]);
        let report = audit(&input(domains, vec![a, b])).unwrap();
        assert_eq!(report.independent_route_count, Some(1));
        assert_eq!(report.minimum_cut_set_size, Some(1));
        assert_eq!(
            report.all_minimal_cut_sets,
            vec![
                vec!["power_bus".to_owned()],
                vec!["shared_encoder_clock".to_owned()]
            ]
        );
    }

    #[test]
    fn r12_more_physical_failure_branches_cannot_improve_reserve() {
        let domains = vec![domain("hip", "hip")];
        let base_route = route("r", &["e1", "e2"], "safe", &["hip"]);
        let base = audit(&input(domains.clone(), vec![base_route.clone()])).unwrap();
        let mut expanded_route = base_route;
        expanded_route.successors.push(SuccessorEvidence {
            state_id: "new_possible_dead_end".to_owned(),
            physically_possible: true,
            disposition: PhysicalStateDisposition::Nonviable,
        });
        let expanded = audit(&input(domains, vec![expanded_route])).unwrap();
        assert_eq!(base.independent_route_count, Some(1));
        assert_eq!(expanded.independent_route_count, Some(0));
        assert!(expanded.independent_route_count.unwrap() <= base.independent_route_count.unwrap());
    }

    #[test]
    fn r13_missing_route_resource_measurement_is_unknown() {
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("r", &["e1"], "safe", &["hip"]);
        candidate.worst_case_energy_mj = None;
        let report = audit(&input(domains, vec![candidate])).unwrap();
        assert_eq!(report.status, QualificationStatus::Unknown);
        assert_eq!(report.independent_route_count, None);
    }

    #[test]
    fn r14_duplicate_route_id_is_invalid() {
        let domains = vec![domain("hip", "hip")];
        let a = route("same", &["e1"], "safe_a", &["hip"]);
        let b = route("same", &["e2"], "safe_b", &["hip"]);
        let result = audit(&input(domains, vec![a, b]));
        assert_eq!(result, Err(AuditError::DuplicateRouteId("same".to_owned())));
    }

    #[test]
    fn r15_zero_probability_does_not_remove_a_physical_branch() {
        // Probabilities are intentionally absent from the runtime type. The
        // upstream physical-possibility relation is authoritative here.
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("r", &["e1"], "safe", &["hip"]);
        candidate.successors.push(SuccessorEvidence {
            state_id: "zero_mass_but_possible".to_owned(),
            physically_possible: true,
            disposition: PhysicalStateDisposition::Nonviable,
        });
        let report = audit(&input(domains, vec![candidate])).unwrap();
        assert_eq!(report.status, QualificationStatus::RejectedNoRecoveryRoute);
    }

    #[test]
    fn r16_unknown_successor_suppresses_topology_claim() {
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("r", &["e1"], "safe", &["hip"]);
        candidate.successors.push(SuccessorEvidence {
            state_id: "unresolved".to_owned(),
            physically_possible: true,
            disposition: PhysicalStateDisposition::Unknown,
        });
        let report = audit(&input(domains, vec![candidate])).unwrap();
        assert_eq!(report.status, QualificationStatus::Unknown);
        assert_eq!(report.independent_route_count, None);
    }

    #[test]
    fn r18_two_domain_disjoint_routes_produce_two_route_reserve() {
        let domains = vec![domain("left_leg", "left_leg"), domain("right_leg", "right_leg")];
        let left = route("left", &["e_left"], "left_safe", &["left_leg"]);
        let right = route("right", &["e_right"], "right_safe", &["right_leg"]);
        let report = audit(&input(domains, vec![left, right])).unwrap();
        assert_eq!(report.status, QualificationStatus::Qualified);
        assert_eq!(report.independent_route_count, Some(2));
        assert_eq!(report.minimum_cut_set_size, Some(2));
        assert_eq!(report.all_minimal_cut_sets, vec![vec![
            "left_leg".to_owned(),
            "right_leg".to_owned()
        ]]);
    }

    #[test]
    fn r19_qualified_alias_dependencies_are_unioned_conservatively() {
        let domains = vec![domain("hip", "hip"), domain("bus", "bus")];
        let a = route("a", &["e_same"], "safe", &["hip"]);
        let b = route("b", &["e_same"], "safe", &["bus"]);
        let report = audit(&input(domains, vec![a.clone(), b])).unwrap();
        assert_eq!(report.known_route_classes.len(), 1);
        assert_eq!(
            report.known_route_classes[0].common_cause_dependencies,
            vec!["bus".to_owned(), "hip".to_owned()]
        );
        assert_eq!(report.independent_route_count, Some(1));
        assert_eq!(report.minimum_cut_set_size, Some(1));
    }


    #[test]
    fn r20_report_binds_start_state_edge_namespace_and_transition_abstraction() {
        let domains = vec![domain("hip", "hip")];
        let candidate = route("r", &["edge:0"], "safe", &["hip"]);
        let report = audit(&input(domains, vec![candidate])).unwrap();
        let class = &report.known_route_classes[0];
        assert_eq!(class.start_state_id, "state:start");
        assert_eq!(class.edge_namespace_digest, "edge-namespace:v1");
        assert_eq!(
            class.transition_abstraction_digest,
            "transition-abstraction:v1"
        );
    }

    #[test]
    fn r21_duplicate_successor_identity_is_invalid() {
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("r", &["edge:0"], "safe", &["hip"]);
        candidate.successors.push(candidate.successors[0].clone());
        let result = audit(&input(domains, vec![candidate]));
        assert_eq!(
            result,
            Err(AuditError::DuplicateSuccessorId {
                route_id: "r".to_owned(),
                state_id: "successor:r".to_owned()
            })
        );
    }

    #[test]
    fn r22_duplicate_intermediate_identity_is_invalid() {
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("r", &["edge:0"], "safe", &["hip"]);
        candidate
            .intermediate_states
            .push(candidate.intermediate_states[0].clone());
        let result = audit(&input(domains, vec![candidate]));
        assert_eq!(
            result,
            Err(AuditError::DuplicateIntermediateStateId {
                route_id: "r".to_owned(),
                state_id: "intermediate:r".to_owned()
            })
        );
    }

    #[test]
    fn r23_missing_branch_manifest_suppresses_global_metrics() {
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("r", &["edge:0"], "safe", &["hip"]);
        candidate.branch_manifest_digest.clear();
        let report = audit(&input(domains, vec![candidate])).unwrap();
        assert_eq!(report.status, QualificationStatus::Unknown);
        assert_eq!(report.independent_route_count, None);
        assert!(report.findings.contains(&RouteFinding::MissingBranchCoverage {
            route_id: "r".to_owned()
        }));
    }

    #[test]
    fn r24_missing_latency_manifest_suppresses_global_metrics() {
        let domains = vec![domain("hip", "hip")];
        let mut candidate = route("r", &["edge:0"], "safe", &["hip"]);
        candidate.latency_prefix_manifest_digest.clear();
        let report = audit(&input(domains, vec![candidate])).unwrap();
        assert_eq!(report.status, QualificationStatus::Unknown);
        assert_eq!(report.independent_route_count, None);
        assert!(report.findings.contains(&RouteFinding::IncompleteLatencyPrefix {
            route_id: "r".to_owned()
        }));
    }

    #[test]
    fn r17_resource_bounds_fail_explicitly() {
        let mut domains = vec![domain("d0", "g0")];
        let mut routes = Vec::new();
        for index in 0..=MAX_ROUTE_CLASSES {
            let id = format!("d{index}");
            let group = format!("g{index}");
            if index > 0 {
                domains.push(domain(&id, &group));
            }
            routes.push(route(
                &format!("r{index}"),
                &[&format!("edge{index}")],
                &format!("terminal{index}"),
                &[&id],
            ));
        }
        let result = audit(&input(domains, routes));
        assert!(matches!(result, Err(AuditError::TooManyRouteClasses { .. })));
    }
}
