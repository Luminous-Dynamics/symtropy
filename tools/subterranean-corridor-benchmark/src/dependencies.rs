//! Conservative service-capacity accounting for explicit route dependencies.
//!
//! Paths are credited only when every declared dependency is known satisfied. A
//! missing dependency record is Unknown. If unresolved paths could change a demand
//! shortfall, the evaluator returns NeedsEvidence instead of assuming they work.

use std::collections::{BTreeMap, BTreeSet};

use super::{EvidenceClass, GateState};

#[derive(Debug, Clone, PartialEq)]
pub struct ServicePath {
    pub path_id: String,
    /// Nominal service capacity in declared service units per hour.
    pub capacity_units_per_hour: f64,
    /// Dependencies required for this path to deliver safe service.
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyRecord {
    pub dependency_id: String,
    pub state: GateState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetworkCapacityScenario {
    pub scenario_id: String,
    pub evidence_class: EvidenceClass,
    /// Provenance references are required for non-synthetic evidence but are not
    /// authenticated by this module.
    pub source_refs: Vec<String>,
    pub demand_units_per_hour: f64,
    pub paths: Vec<ServicePath>,
    pub dependencies: Vec<DependencyRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCapacityDisposition {
    InvalidInput,
    Ineligible,
    NeedsEvidence,
    ComparableSynthetic,
    ComparableDeclaredEvidence,
    ConservativeLowerBound,
}

impl NetworkCapacityDisposition {
    pub fn label(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::Ineligible => "INELIGIBLE",
            Self::NeedsEvidence => "NEEDS_EVIDENCE",
            Self::ComparableSynthetic => "COMPARABLE_SYNTHETIC_ONLY",
            Self::ComparableDeclaredEvidence => "COMPARABLE_DECLARED_EVIDENCE",
            Self::ConservativeLowerBound => "CONSERVATIVE_LOWER_BOUND",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetworkCapacityMetrics {
    pub scenario_id: String,
    pub evidence_class: EvidenceClass,
    pub demand_units_per_hour: f64,
    /// Capacity from paths whose required dependencies are all Satisfied.
    pub credited_safe_capacity_units_per_hour: f64,
    pub safe_delivered_units_per_hour: f64,
    pub safe_delivered_fraction_lower_bound: f64,
    pub available_path_ids: Vec<String>,
    pub unavailable_path_ids: Vec<String>,
    pub unresolved_path_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetworkCapacityEvaluation {
    pub scenario_id: String,
    pub disposition: NetworkCapacityDisposition,
    pub reasons: Vec<String>,
    pub metrics: Option<NetworkCapacityMetrics>,
}

fn invalid(scenario_id: &str, reason: impl Into<String>) -> NetworkCapacityEvaluation {
    NetworkCapacityEvaluation {
        scenario_id: scenario_id.to_owned(),
        disposition: NetworkCapacityDisposition::InvalidInput,
        reasons: vec![reason.into()],
        metrics: None,
    }
}

fn validate(scenario: &NetworkCapacityScenario) -> Result<(), String> {
    if scenario.scenario_id.trim().is_empty() {
        return Err("EMPTY_SCENARIO_ID".to_owned());
    }
    if !scenario.demand_units_per_hour.is_finite() || scenario.demand_units_per_hour <= 0.0 {
        return Err("DEMAND_RATE_MUST_BE_FINITE_AND_POSITIVE".to_owned());
    }
    if scenario.paths.is_empty() {
        return Err("AT_LEAST_ONE_SERVICE_PATH_REQUIRED".to_owned());
    }
    if scenario.evidence_class != EvidenceClass::Synthetic && scenario.source_refs.is_empty() {
        return Err("DECLARED_EVIDENCE_REQUIRES_SOURCE_REFERENCES".to_owned());
    }

    let mut refs = BTreeSet::new();
    for source_ref in &scenario.source_refs {
        if source_ref.trim().is_empty() || !refs.insert(source_ref) {
            return Err("EMPTY_OR_DUPLICATE_SOURCE_REFERENCE".to_owned());
        }
    }

    let mut path_ids = BTreeSet::new();
    let mut referenced_dependencies = BTreeSet::new();
    for path in &scenario.paths {
        if path.path_id.trim().is_empty() || !path_ids.insert(&path.path_id) {
            return Err("EMPTY_OR_DUPLICATE_PATH_ID".to_owned());
        }
        if !path.capacity_units_per_hour.is_finite() || path.capacity_units_per_hour <= 0.0 {
            return Err("PATH_CAPACITY_MUST_BE_FINITE_AND_POSITIVE".to_owned());
        }
        if path.dependencies.is_empty() {
            return Err("PATH_MUST_DECLARE_REQUIRED_DEPENDENCIES".to_owned());
        }
        let mut path_dependencies = BTreeSet::new();
        for dependency_id in &path.dependencies {
            if dependency_id.trim().is_empty() || !path_dependencies.insert(dependency_id) {
                return Err("EMPTY_OR_DUPLICATE_PATH_DEPENDENCY".to_owned());
            }
            referenced_dependencies.insert(dependency_id);
        }
    }

    let mut dependency_ids = BTreeSet::new();
    for dependency in &scenario.dependencies {
        if dependency.dependency_id.trim().is_empty()
            || !dependency_ids.insert(&dependency.dependency_id)
        {
            return Err("EMPTY_OR_DUPLICATE_DEPENDENCY_ID".to_owned());
        }
    }
    if dependency_ids
        .iter()
        .any(|id| !referenced_dependencies.contains(id))
    {
        return Err("UNREFERENCED_DEPENDENCY_RECORD".to_owned());
    }
    Ok(())
}

/// Evaluate conservative deliverable capacity under one declared dependency state.
///
/// A path is unavailable if any known dependency Failed. Otherwise, a path with one
/// or more Unknown/NotApplicable/missing dependencies is unresolved and contributes
/// no capacity. If the known-safe paths cannot meet demand and unresolved paths could
/// close the gap, metrics are withheld with NeedsEvidence. If the known-safe paths do
/// meet demand, a lower-bound report is possible but unresolved alternatives remain
/// visible and the report is not classified as an ordinary comparable result.
pub fn evaluate_network_capacity(scenario: &NetworkCapacityScenario) -> NetworkCapacityEvaluation {
    if let Err(reason) = validate(scenario) {
        return invalid(&scenario.scenario_id, reason);
    }

    let dependency_states: BTreeMap<&str, GateState> = scenario
        .dependencies
        .iter()
        .map(|dependency| (dependency.dependency_id.as_str(), dependency.state))
        .collect();

    let mut available_path_ids = Vec::new();
    let mut unavailable_path_ids = Vec::new();
    let mut unresolved_path_ids = Vec::new();
    let mut credited_capacity = 0.0;

    for path in &scenario.paths {
        let states = path.dependencies.iter().map(|dependency_id| {
            dependency_states
                .get(dependency_id.as_str())
                .copied()
                .unwrap_or(GateState::Unknown)
        });
        let mut failed = false;
        let mut unresolved = false;
        for state in states {
            match state {
                GateState::Satisfied => {}
                GateState::Failed => failed = true,
                GateState::Unknown | GateState::NotApplicable => unresolved = true,
            }
        }

        if failed {
            unavailable_path_ids.push(path.path_id.clone());
        } else if unresolved {
            unresolved_path_ids.push(path.path_id.clone());
        } else {
            available_path_ids.push(path.path_id.clone());
            credited_capacity += path.capacity_units_per_hour;
            if !credited_capacity.is_finite() {
                return invalid(
                    &scenario.scenario_id,
                    "ARITHMETIC_OVERFLOW:CREDITED_CAPACITY",
                );
            }
        }
    }

    // Stable ordering makes reports reproducible regardless of input path order.
    available_path_ids.sort();
    unavailable_path_ids.sort();
    unresolved_path_ids.sort();

    if credited_capacity + 1.0e-9 < scenario.demand_units_per_hour
        && !unresolved_path_ids.is_empty()
    {
        return NetworkCapacityEvaluation {
            scenario_id: scenario.scenario_id.clone(),
            disposition: NetworkCapacityDisposition::NeedsEvidence,
            reasons: unresolved_path_ids
                .iter()
                .map(|path_id| format!("NEEDS_EVIDENCE:PATH_DEPENDENCIES:{path_id}"))
                .collect(),
            metrics: None,
        };
    }

    let safe_delivered = credited_capacity.min(scenario.demand_units_per_hour);
    let fraction = safe_delivered / scenario.demand_units_per_hour;
    if !safe_delivered.is_finite() || !fraction.is_finite() {
        return invalid(
            &scenario.scenario_id,
            "ARITHMETIC_OVERFLOW:SAFE_DELIVERED_CAPACITY",
        );
    }

    let disposition = if !unresolved_path_ids.is_empty() {
        NetworkCapacityDisposition::ConservativeLowerBound
    } else {
        match scenario.evidence_class {
            EvidenceClass::Synthetic => NetworkCapacityDisposition::ComparableSynthetic,
            EvidenceClass::Sourced
            | EvidenceClass::Measured
            | EvidenceClass::IndependentlyReviewed => {
                NetworkCapacityDisposition::ComparableDeclaredEvidence
            }
        }
    };
    let reasons = if unresolved_path_ids.is_empty() {
        Vec::new()
    } else {
        vec!["UNRESOLVED_PATHS_NOT_CREDITED_IN_LOWER_BOUND".to_owned()]
    };
    NetworkCapacityEvaluation {
        scenario_id: scenario.scenario_id.clone(),
        disposition,
        reasons,
        metrics: Some(NetworkCapacityMetrics {
            scenario_id: scenario.scenario_id.clone(),
            evidence_class: scenario.evidence_class,
            demand_units_per_hour: scenario.demand_units_per_hour,
            credited_safe_capacity_units_per_hour: credited_capacity,
            safe_delivered_units_per_hour: safe_delivered,
            safe_delivered_fraction_lower_bound: fraction.clamp(0.0, 1.0),
            available_path_ids,
            unavailable_path_ids,
            unresolved_path_ids,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_scenario() -> NetworkCapacityScenario {
        NetworkCapacityScenario {
            scenario_id: "common-cause-test".to_owned(),
            evidence_class: EvidenceClass::Synthetic,
            source_refs: Vec::new(),
            demand_units_per_hour: 100.0,
            paths: vec![
                ServicePath {
                    path_id: "west-route".to_owned(),
                    capacity_units_per_hour: 60.0,
                    dependencies: vec!["traction-power".to_owned(), "west-portal".to_owned()],
                },
                ServicePath {
                    path_id: "east-route".to_owned(),
                    capacity_units_per_hour: 50.0,
                    dependencies: vec!["traction-power".to_owned(), "east-portal".to_owned()],
                },
                ServicePath {
                    path_id: "surface-backup".to_owned(),
                    capacity_units_per_hour: 40.0,
                    dependencies: vec!["backup-power".to_owned(), "surface-connection".to_owned()],
                },
            ],
            dependencies: vec![
                DependencyRecord {
                    dependency_id: "traction-power".to_owned(),
                    state: GateState::Failed,
                },
                DependencyRecord {
                    dependency_id: "west-portal".to_owned(),
                    state: GateState::Satisfied,
                },
                DependencyRecord {
                    dependency_id: "east-portal".to_owned(),
                    state: GateState::Satisfied,
                },
                DependencyRecord {
                    dependency_id: "backup-power".to_owned(),
                    state: GateState::Satisfied,
                },
                DependencyRecord {
                    dependency_id: "surface-connection".to_owned(),
                    state: GateState::Satisfied,
                },
            ],
        }
    }

    #[test]
    fn shared_dependency_failure_removes_multiple_nominally_redundant_paths() {
        let evaluation = evaluate_network_capacity(&base_scenario());
        assert_eq!(
            evaluation.disposition,
            NetworkCapacityDisposition::ComparableSynthetic
        );
        let metrics = evaluation.metrics.unwrap();
        assert!((metrics.credited_safe_capacity_units_per_hour - 40.0).abs() < 1.0e-9);
        assert!((metrics.safe_delivered_fraction_lower_bound - 0.4).abs() < 1.0e-9);
        assert_eq!(
            metrics.unavailable_path_ids,
            vec!["east-route".to_owned(), "west-route".to_owned()]
        );
        assert_eq!(
            metrics.available_path_ids,
            vec!["surface-backup".to_owned()]
        );
    }

    #[test]
    fn unknown_path_blocks_result_when_it_is_needed_to_meet_demand() {
        let mut scenario = base_scenario();
        scenario.dependencies[0].state = GateState::Unknown;
        let evaluation = evaluate_network_capacity(&scenario);
        assert_eq!(
            evaluation.disposition,
            NetworkCapacityDisposition::NeedsEvidence
        );
        assert!(evaluation.metrics.is_none());
        assert!(evaluation.reasons.iter().any(|reason| {
            reason == "NEEDS_EVIDENCE:PATH_DEPENDENCIES:west-route"
                || reason == "NEEDS_EVIDENCE:PATH_DEPENDENCIES:east-route"
        }));
    }

    #[test]
    fn enough_known_safe_capacity_is_reported_as_a_conservative_lower_bound() {
        let mut scenario = base_scenario();
        scenario.paths[2].capacity_units_per_hour = 120.0;
        scenario.dependencies[0].state = GateState::Unknown;
        let evaluation = evaluate_network_capacity(&scenario);
        assert_eq!(
            evaluation.disposition,
            NetworkCapacityDisposition::ConservativeLowerBound
        );
        let metrics = evaluation.metrics.unwrap();
        assert!((metrics.safe_delivered_fraction_lower_bound - 1.0).abs() < 1.0e-9);
        assert!(
            metrics
                .unresolved_path_ids
                .contains(&"west-route".to_owned())
        );
        assert!(
            metrics
                .unresolved_path_ids
                .contains(&"east-route".to_owned())
        );
    }

    #[test]
    fn missing_dependency_record_is_unknown_not_assumed_healthy() {
        let mut scenario = base_scenario();
        scenario
            .dependencies
            .retain(|dependency| dependency.dependency_id != "traction-power");
        let evaluation = evaluate_network_capacity(&scenario);
        assert_eq!(
            evaluation.disposition,
            NetworkCapacityDisposition::NeedsEvidence
        );
        assert!(evaluation.metrics.is_none());
    }

    #[test]
    fn duplicate_path_ids_and_invalid_capacity_are_rejected() {
        let mut duplicate = base_scenario();
        duplicate.paths[1].path_id = duplicate.paths[0].path_id.clone();
        assert_eq!(
            evaluate_network_capacity(&duplicate).disposition,
            NetworkCapacityDisposition::InvalidInput
        );

        let mut invalid_capacity = base_scenario();
        invalid_capacity.paths[0].capacity_units_per_hour = f64::INFINITY;
        assert_eq!(
            evaluate_network_capacity(&invalid_capacity).disposition,
            NetworkCapacityDisposition::InvalidInput
        );
    }
}
