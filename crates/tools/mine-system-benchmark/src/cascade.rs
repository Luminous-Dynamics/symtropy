// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic cross-service cascade simulation for MINE-CASCADE-001.
//!
//! This is a bounded synthetic dependency simulator, not a ventilation,
//! hydrology, geotechnical, radio-propagation, evacuation, or safety model.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const MAX_CONFIGURED_SERVICES: usize = 512;
const MAX_CONFIGURED_EVENTS: usize = 10_000;
const MAX_DEPENDENCY_EDGES: usize = 8_192;
const MAX_EVALUATION_WORK: usize = 2_000_000;

/// Known local state. Unknown is never interpreted as healthy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalState { Available, Unavailable, Unknown }

/// A service is viable only when its local envelope, evidence, and requirements pass.
#[derive(Clone, Debug, PartialEq)]
pub struct Service {
    pub id: String,
    pub local_state: LocalState,
    pub capacity: f64,
    pub minimum_capacity: f64,
    pub evidence_current: bool,
    pub failure_domain: Option<String>,
    pub requirements: Vec<DependencyRule>,
    pub critical: bool,
}

/// Each rule is required. AllOf represents prerequisites; AtLeast represents redundancy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyRule {
    AllOf(Vec<String>),
    AtLeast { service_ids: Vec<String>, minimum_viable: usize },
}

/// Events at the same tick are ordered by unique event ID for deterministic replay.
#[derive(Clone, Debug, PartialEq)]
pub struct Event { pub at_tick: u64, pub id: String, pub kind: EventKind }

#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    SetLocalState { service_id: String, state: LocalState },
    SetCapacity { service_id: String, capacity: f64 },
    SetEvidenceCurrent { service_id: String, current: bool },
    /// Shared-cause fault affects every service assigned to the declared domain.
    SetFailureDomainAvailable { domain: String, available: bool },
}

/// Frozen input and explicit computation limits.
#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
    pub id: String,
    pub horizon_tick: u64,
    pub max_services: usize,
    pub max_events: usize,
    pub services: Vec<Service>,
    pub events: Vec<Event>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition { Viable, Unavailable, Unknown }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSnapshot {
    pub at_tick: u64,
    pub event_id: String,
    pub dispositions: BTreeMap<String, Disposition>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationReport {
    pub scenario_id: String,
    pub horizon_tick: u64,
    pub ordered_event_ids: Vec<String>,
    pub snapshots: Vec<EventSnapshot>,
    /// Tick × service counts while definitively unavailable; overlapping losses count separately.
    pub unavailable_service_ticks: BTreeMap<String, u64>,
    /// Tick × service counts while classification is unknown.
    pub unknown_service_ticks: BTreeMap<String, u64>,
    pub critical_unavailable_service_ticks: u64,
    pub critical_unknown_service_ticks: u64,
    pub final_dispositions: BTreeMap<String, Disposition>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationError {
    InvalidInput(String),
    LimitExceeded { resource: &'static str, observed: usize, limit: usize },
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "invalid scenario: {message}"),
            Self::LimitExceeded { resource, observed, limit } =>
                write!(f, "{resource} cap exceeded: {observed} > {limit}"),
        }
    }
}
impl std::error::Error for SimulationError {}

#[derive(Clone, Debug)]
struct RuntimeService { local_state: LocalState, capacity: f64, evidence_current: bool }
impl From<&Service> for RuntimeService {
    fn from(s: &Service) -> Self {
        Self { local_state: s.local_state, capacity: s.capacity, evidence_current: s.evidence_current }
    }
}

/// Run a bounded, deterministic simulation. Reports never hide a cap or invalid input.
pub fn simulate(scenario: &Scenario) -> Result<SimulationReport, SimulationError> {
    let services = validate_and_index(scenario)?;
    let topo = topological_order(&services)?;
    let mut events = scenario.events.clone();
    events.sort_by(|a, b| a.at_tick.cmp(&b.at_tick).then_with(|| a.id.cmp(&b.id)));

    let mut runtime: BTreeMap<String, RuntimeService> = services.iter()
        .map(|(id, s)| (id.clone(), RuntimeService::from(s))).collect();
    let mut down_domains = BTreeSet::<String>::new();
    let mut states = evaluate(&services, &runtime, &down_domains, &topo);
    let mut unavailable: BTreeMap<String, u64> = services.keys().map(|id| (id.clone(), 0)).collect();
    let mut unknown: BTreeMap<String, u64> = services.keys().map(|id| (id.clone(), 0)).collect();
    let mut critical_unavailable = 0_u64;
    let mut critical_unknown = 0_u64;
    let mut snapshots = Vec::with_capacity(events.len());
    let mut event_ids = Vec::with_capacity(events.len());
    let mut last_tick = 0_u64;

    for event in &events {
        accumulate(event.at_tick - last_tick, &services, &states, &mut unavailable, &mut unknown,
            &mut critical_unavailable, &mut critical_unknown);
        apply(event, &mut runtime, &mut down_domains);
        states = evaluate(&services, &runtime, &down_domains, &topo);
        event_ids.push(event.id.clone());
        snapshots.push(EventSnapshot { at_tick: event.at_tick, event_id: event.id.clone(), dispositions: states.clone() });
        last_tick = event.at_tick;
    }
    accumulate(scenario.horizon_tick - last_tick, &services, &states, &mut unavailable, &mut unknown,
        &mut critical_unavailable, &mut critical_unknown);

    Ok(SimulationReport {
        scenario_id: scenario.id.clone(), horizon_tick: scenario.horizon_tick,
        ordered_event_ids: event_ids, snapshots, unavailable_service_ticks: unavailable,
        unknown_service_ticks: unknown, critical_unavailable_service_ticks: critical_unavailable,
        critical_unknown_service_ticks: critical_unknown, final_dispositions: states,
    })
}

fn validate_and_index(s: &Scenario) -> Result<BTreeMap<String, Service>, SimulationError> {
    if s.id.trim().is_empty() { return invalid("scenario ID is empty"); }
    if s.services.is_empty() { return invalid("scenario has no services"); }
    if s.max_services == 0 || s.max_events == 0 { return invalid("service/event caps must be positive"); }
    if s.max_services > MAX_CONFIGURED_SERVICES || s.max_events > MAX_CONFIGURED_EVENTS {
        return invalid("configured caps exceed the simulator hard limits");
    }
    if s.services.len() > s.max_services {
        return Err(SimulationError::LimitExceeded { resource: "service count", observed: s.services.len(), limit: s.max_services });
    }
    if s.events.len() > s.max_events {
        return Err(SimulationError::LimitExceeded { resource: "event count", observed: s.events.len(), limit: s.max_events });
    }
    let mut services = BTreeMap::new();
    for service in &s.services {
        if service.id.trim().is_empty() { return invalid("service ID is empty"); }
        if !service.capacity.is_finite() || !service.minimum_capacity.is_finite()
            || service.capacity < 0.0 || service.minimum_capacity < 0.0 {
            return invalid(format!("service {} has invalid capacity", service.id));
        }
        if service.failure_domain.as_ref().is_some_and(|d| d.trim().is_empty()) {
            return invalid(format!("service {} has empty failure domain", service.id));
        }
        let id = service.id.clone();
        if services.insert(id.clone(), service.clone()).is_some() { return invalid(format!("duplicate service ID {id}")); }
    }
    let ids: BTreeSet<String> = services.keys().cloned().collect();
    let mut dependency_edges = 0usize;
    for service in services.values() {
        for rule in &service.requirements {
            let deps = match rule {
                DependencyRule::AllOf(deps) => {
                    if deps.is_empty() { return invalid(format!("service {} has empty AllOf rule; omit it instead", service.id)); }
                    deps
                }
                DependencyRule::AtLeast { service_ids, minimum_viable } => {
                    if *minimum_viable == 0 || *minimum_viable > service_ids.len() {
                        return invalid(format!("service {} has invalid AtLeast threshold", service.id));
                    }
                    service_ids
                }
            };
            let mut seen = BTreeSet::new();
            for dep in deps {
                dependency_edges = dependency_edges.checked_add(1).ok_or_else(|| SimulationError::InvalidInput("dependency edge count overflow".into()))?;
                if dependency_edges > MAX_DEPENDENCY_EDGES { return invalid("dependency-edge cap exceeded"); }
                if !seen.insert(dep) { return invalid(format!("service {} repeats dependency {dep}", service.id)); }
                if !ids.contains(dep) { return invalid(format!("service {} references missing dependency {dep}", service.id)); }
                if dep == &service.id { return invalid(format!("service {} depends on itself", service.id)); }
            }
        }
    }
    let domains: BTreeSet<String> = services.values().filter_map(|s| s.failure_domain.clone()).collect();
    let mut event_ids = BTreeSet::new();
    for event in &s.events {
        if event.id.trim().is_empty() { return invalid("event ID is empty"); }
        if !event_ids.insert(event.id.clone()) { return invalid(format!("duplicate event ID {}", event.id)); }
        if event.at_tick > s.horizon_tick { return invalid(format!("event {} after horizon", event.id)); }
        match &event.kind {
            EventKind::SetLocalState { service_id, .. } | EventKind::SetEvidenceCurrent { service_id, .. } => {
                if !ids.contains(service_id) { return invalid(format!("event {} references missing service {service_id}", event.id)); }
            }
            EventKind::SetCapacity { service_id, capacity } => {
                if !ids.contains(service_id) { return invalid(format!("event {} references missing service {service_id}", event.id)); }
                if !capacity.is_finite() || *capacity < 0.0 { return invalid(format!("event {} has invalid capacity", event.id)); }
            }
            EventKind::SetFailureDomainAvailable { domain, .. } => {
                if domain.trim().is_empty() || !domains.contains(domain) {
                    return invalid(format!("event {} references unknown failure domain {domain}", event.id));
                }
            }
        }
    }
    let per_tick_work = services.len().checked_add(dependency_edges)
        .ok_or_else(|| SimulationError::InvalidInput("evaluation-work estimate overflow".into()))?;
    let estimated_work = per_tick_work.checked_mul(s.events.len().saturating_add(1))
        .ok_or_else(|| SimulationError::InvalidInput("evaluation-work estimate overflow".into()))?;
    if estimated_work > MAX_EVALUATION_WORK { return invalid("estimated service evaluation work exceeds the hard budget"); }
    Ok(services)
}

fn invalid<T>(message: impl Into<String>) -> Result<T, SimulationError> {
    Err(SimulationError::InvalidInput(message.into()))
}

fn topological_order(services: &BTreeMap<String, Service>) -> Result<Vec<String>, SimulationError> {
    fn visit(id: &str, services: &BTreeMap<String, Service>, marks: &mut BTreeMap<String, u8>, order: &mut Vec<String>) -> Result<(), SimulationError> {
        match marks.get(id).copied() {
            Some(1) => return invalid(format!("dependency cycle reaches service {id}")),
            Some(2) => return Ok(()),
            _ => {}
        }
        marks.insert(id.into(), 1);
        let mut deps: Vec<String> = services[id].requirements.iter().flat_map(|rule| rule_ids(rule).iter()).cloned().collect();
        deps.sort();
        for dep in deps { visit(&dep, services, marks, order)?; }
        marks.insert(id.into(), 2);
        order.push(id.into());
        Ok(())
    }
    let mut marks = BTreeMap::new();
    let mut order = Vec::with_capacity(services.len());
    for id in services.keys() { visit(id, services, &mut marks, &mut order)?; }
    Ok(order)
}

fn rule_ids(rule: &DependencyRule) -> &[String] {
    match rule { DependencyRule::AllOf(ids) => ids, DependencyRule::AtLeast { service_ids, .. } => service_ids }
}

fn evaluate(services: &BTreeMap<String, Service>, runtime: &BTreeMap<String, RuntimeService>, down_domains: &BTreeSet<String>, topo: &[String]) -> BTreeMap<String, Disposition> {
    let mut out = BTreeMap::new();
    for id in topo {
        let service = &services[id];
        let state = &runtime[id];
        let local = if state.local_state == LocalState::Unavailable
            || service.failure_domain.as_ref().is_some_and(|d| down_domains.contains(d)) {
            Disposition::Unavailable
        } else if state.local_state == LocalState::Unknown || !state.evidence_current {
            Disposition::Unknown
        } else if state.capacity < service.minimum_capacity {
            Disposition::Unavailable
        } else { Disposition::Viable };
        let disposition = if local != Disposition::Viable { local } else {
            let requirements: Vec<Disposition> = service.requirements.iter().map(|r| eval_rule(r, &out)).collect();
            if requirements.contains(&Disposition::Unavailable) { Disposition::Unavailable }
            else if requirements.contains(&Disposition::Unknown) { Disposition::Unknown }
            else { Disposition::Viable }
        };
        out.insert(id.clone(), disposition);
    }
    out
}

fn eval_rule(rule: &DependencyRule, states: &BTreeMap<String, Disposition>) -> Disposition {
    match rule {
        DependencyRule::AllOf(ids) => {
            let values: Vec<Disposition> = ids.iter().map(|id| states[id]).collect();
            if values.contains(&Disposition::Unavailable) { Disposition::Unavailable }
            else if values.contains(&Disposition::Unknown) { Disposition::Unknown }
            else { Disposition::Viable }
        }
        DependencyRule::AtLeast { service_ids, minimum_viable } => {
            let mut viable = 0usize;
            let mut unknown = 0usize;
            for id in service_ids { match states[id] {
                Disposition::Viable => viable += 1,
                Disposition::Unknown => unknown += 1,
                Disposition::Unavailable => {}
            }}
            if viable >= *minimum_viable { Disposition::Viable }
            else if viable + unknown >= *minimum_viable { Disposition::Unknown }
            else { Disposition::Unavailable }
        }
    }
}

fn apply(event: &Event, runtime: &mut BTreeMap<String, RuntimeService>, down_domains: &mut BTreeSet<String>) {
    match &event.kind {
        EventKind::SetLocalState { service_id, state } => runtime.get_mut(service_id).expect("validated service").local_state = *state,
        EventKind::SetCapacity { service_id, capacity } => runtime.get_mut(service_id).expect("validated service").capacity = *capacity,
        EventKind::SetEvidenceCurrent { service_id, current } => runtime.get_mut(service_id).expect("validated service").evidence_current = *current,
        EventKind::SetFailureDomainAvailable { domain, available } => {
            if *available { down_domains.remove(domain); } else { down_domains.insert(domain.clone()); }
        }
    }
}

fn accumulate(duration: u64, services: &BTreeMap<String, Service>, states: &BTreeMap<String, Disposition>, unavailable: &mut BTreeMap<String, u64>, unknown: &mut BTreeMap<String, u64>, critical_unavailable: &mut u64, critical_unknown: &mut u64) {
    for (id, service) in services {
        match states[id] {
            Disposition::Viable => {}
            Disposition::Unavailable => {
                *unavailable.get_mut(id).expect("initialized service") += duration;
                if service.critical { *critical_unavailable += duration; }
            }
            Disposition::Unknown => {
                *unknown.get_mut(id).expect("initialized service") += duration;
                if service.critical { *critical_unknown += duration; }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn svc(id: &str) -> Service { Service { id: id.into(), local_state: LocalState::Available, capacity: 10.0, minimum_capacity: 5.0, evidence_current: true, failure_domain: None, requirements: Vec::new(), critical: false } }
    fn scenario(services: Vec<Service>, events: Vec<Event>, horizon_tick: u64) -> Scenario { Scenario { id: "SYNTHETIC-CASCADE".into(), horizon_tick, max_services: 32, max_events: 64, services, events } }
    fn ev(at_tick: u64, id: &str, kind: EventKind) -> Event { Event { at_tick, id: id.into(), kind } }

    #[test]
    fn prerequisites_gate_recovery_and_outage_ticks() {
        let pump = svc("pump"); let mut mill = svc("mill"); mill.requirements = vec![DependencyRule::AllOf(vec!["pump".into()])];
        let input = scenario(vec![mill, pump], vec![ev(2, "pump-down", EventKind::SetLocalState { service_id: "pump".into(), state: LocalState::Unavailable }), ev(5, "pump-up", EventKind::SetLocalState { service_id: "pump".into(), state: LocalState::Available })], 8);
        let report = simulate(&input).unwrap();
        assert_eq!(report.snapshots[0].dispositions["mill"], Disposition::Unavailable);
        assert_eq!(report.snapshots[1].dispositions["mill"], Disposition::Viable);
        assert_eq!(report.unavailable_service_ticks["mill"], 3);
    }

    #[test]
    fn common_cause_defeats_nominal_redundancy() {
        let mut a = svc("pump-a"); a.failure_domain = Some("shared-power".into());
        let mut b = svc("pump-b"); b.failure_domain = Some("shared-power".into());
        let mut dewatering = svc("dewatering"); dewatering.critical = true; dewatering.requirements = vec![DependencyRule::AtLeast { service_ids: vec!["pump-a".into(), "pump-b".into()], minimum_viable: 1 }];
        let input = scenario(vec![a, b, dewatering], vec![ev(1, "shared-down", EventKind::SetFailureDomainAvailable { domain: "shared-power".into(), available: false }), ev(4, "shared-up", EventKind::SetFailureDomainAvailable { domain: "shared-power".into(), available: true })], 6);
        let report = simulate(&input).unwrap();
        assert_eq!(report.snapshots[0].dispositions["dewatering"], Disposition::Unavailable);
        assert_eq!(report.critical_unavailable_service_ticks, 3);
        assert_eq!(report.final_dispositions["dewatering"], Disposition::Viable);
    }

    #[test]
    fn independent_redundancy_survives_single_domain_loss() {
        let mut a = svc("pump-a"); a.failure_domain = Some("power-a".into());
        let mut b = svc("pump-b"); b.failure_domain = Some("power-b".into());
        let mut dewatering = svc("dewatering"); dewatering.requirements = vec![DependencyRule::AtLeast { service_ids: vec!["pump-a".into(), "pump-b".into()], minimum_viable: 1 }];
        let input = scenario(vec![dewatering, b, a], vec![ev(1, "power-a-down", EventKind::SetFailureDomainAvailable { domain: "power-a".into(), available: false })], 3);
        let report = simulate(&input).unwrap();
        assert_eq!(report.snapshots[0].dispositions["dewatering"], Disposition::Viable);
    }

    #[test]
    fn stale_evidence_propagates_unknown() {
        let mut sensor = svc("flow-sensor"); sensor.critical = true;
        let mut pump = svc("pump"); pump.requirements = vec![DependencyRule::AllOf(vec!["flow-sensor".into()])];
        let input = scenario(vec![pump, sensor], vec![ev(2, "stale", EventKind::SetEvidenceCurrent { service_id: "flow-sensor".into(), current: false })], 5);
        let report = simulate(&input).unwrap();
        assert_eq!(report.snapshots[0].dispositions["pump"], Disposition::Unknown);
        assert_eq!(report.critical_unknown_service_ticks, 3);
    }

    #[test]
    fn event_input_order_does_not_change_replay() {
        let input = scenario(vec![svc("a"), svc("b")], vec![ev(1, "z-capacity", EventKind::SetCapacity { service_id: "a".into(), capacity: 4.0 }), ev(1, "a-unknown", EventKind::SetLocalState { service_id: "b".into(), state: LocalState::Unknown })], 2);
        let mut reversed = input.clone(); reversed.events.reverse();
        assert_eq!(simulate(&input).unwrap(), simulate(&reversed).unwrap());
    }

    #[test]
    fn cyclic_graph_and_caps_fail_closed() {
        let mut a = svc("a"); a.requirements = vec![DependencyRule::AllOf(vec!["b".into()])];
        let mut b = svc("b"); b.requirements = vec![DependencyRule::AllOf(vec!["a".into()])];
        assert!(matches!(simulate(&scenario(vec![a, b], vec![], 1)), Err(SimulationError::InvalidInput(_))));
        let mut cap = scenario(vec![svc("a"), svc("b")], vec![], 1); cap.max_services = 1;
        assert!(matches!(simulate(&cap), Err(SimulationError::LimitExceeded { resource: "service count", .. })));
    }

    #[test]
    fn unknown_redundant_candidate_is_not_counted_as_viable() {
        let mut a = svc("a"); a.local_state = LocalState::Unknown; let b = svc("b");
        let mut c = svc("consumer"); c.requirements = vec![DependencyRule::AtLeast { service_ids: vec!["a".into(), "b".into()], minimum_viable: 2 }];
        let report = simulate(&scenario(vec![c, a, b], vec![], 1)).unwrap();
        assert_eq!(report.final_dispositions["consumer"], Disposition::Unknown);
    }


    // Independent recursive reference evaluator. It does not reuse the
    // production topological-order or evaluate/eval_rule implementation.
    fn reference_disposition(
        id: &str,
        services: &BTreeMap<String, Service>,
        visiting: &mut BTreeSet<String>,
    ) -> Disposition {
        if !visiting.insert(id.to_owned()) {
            return Disposition::Unknown; // cycles are separately rejected
        }
        let service = &services[id];
        let local = if service.local_state == LocalState::Unavailable {
            Disposition::Unavailable
        } else if service.local_state == LocalState::Unknown || !service.evidence_current {
            Disposition::Unknown
        } else if service.capacity < service.minimum_capacity {
            Disposition::Unavailable
        } else {
            Disposition::Viable
        };
        let result = if local != Disposition::Viable {
            local
        } else {
            let mut requirements = Vec::new();
            for rule in &service.requirements {
                requirements.push(reference_rule(rule, services, visiting));
            }
            if requirements.contains(&Disposition::Unavailable) {
                Disposition::Unavailable
            } else if requirements.contains(&Disposition::Unknown) {
                Disposition::Unknown
            } else {
                Disposition::Viable
            }
        };
        visiting.remove(id);
        result
    }

    fn reference_rule(
        rule: &DependencyRule,
        services: &BTreeMap<String, Service>,
        visiting: &mut BTreeSet<String>,
    ) -> Disposition {
        match rule {
            DependencyRule::AllOf(ids) => {
                let results: Vec<Disposition> = ids.iter()
                    .map(|id| reference_disposition(id, services, visiting))
                    .collect();
                if results.contains(&Disposition::Unavailable) {
                    Disposition::Unavailable
                } else if results.contains(&Disposition::Unknown) {
                    Disposition::Unknown
                } else {
                    Disposition::Viable
                }
            }
            DependencyRule::AtLeast { service_ids, minimum_viable } => {
                let mut viable = 0usize;
                let mut unknown = 0usize;
                for id in service_ids {
                    match reference_disposition(id, services, visiting) {
                        Disposition::Viable => viable += 1,
                        Disposition::Unknown => unknown += 1,
                        Disposition::Unavailable => {}
                    }
                }
                if viable >= *minimum_viable {
                    Disposition::Viable
                } else if viable + unknown >= *minimum_viable {
                    Disposition::Unknown
                } else {
                    Disposition::Unavailable
                }
            }
        }
    }

    #[test]
    fn production_evaluator_matches_independent_oracle_for_exhaustive_small_states() {
        let local_states = [LocalState::Available, LocalState::Unavailable, LocalState::Unknown];
        let capacities = [0.0, 10.0];
        let freshness = [false, true];
        let mut checked = 0usize;

        for a_state in local_states {
            for b_state in local_states {
                for a_capacity in capacities {
                    for b_capacity in capacities {
                        for a_fresh in freshness {
                            for b_fresh in freshness {
                                let mut a = svc("a");
                                a.local_state = a_state;
                                a.capacity = a_capacity;
                                a.evidence_current = a_fresh;

                                let mut b = svc("b");
                                b.local_state = b_state;
                                b.capacity = b_capacity;
                                b.evidence_current = b_fresh;

                                let mut all = svc("all-of-a-b");
                                all.requirements = vec![DependencyRule::AllOf(vec![
                                    "a".into(), "b".into(),
                                ])];
                                let mut one = svc("at-least-one");
                                one.requirements = vec![DependencyRule::AtLeast {
                                    service_ids: vec!["a".into(), "b".into()],
                                    minimum_viable: 1,
                                }];
                                let mut both = svc("at-least-two");
                                both.requirements = vec![DependencyRule::AtLeast {
                                    service_ids: vec!["a".into(), "b".into()],
                                    minimum_viable: 2,
                                }];

                                // Deliberately scramble input order; identity, not vector
                                // position, determines dependency resolution.
                                let input = scenario(
                                    vec![both, b, all, one, a],
                                    vec![],
                                    1,
                                );
                                let report = simulate(&input).unwrap();
                                let services: BTreeMap<String, Service> = input.services
                                    .iter()
                                    .map(|s| (s.id.clone(), s.clone()))
                                    .collect();
                                let expected: BTreeMap<String, Disposition> = services.keys()
                                    .map(|id| (
                                        id.clone(),
                                        reference_disposition(id, &services, &mut BTreeSet::new()),
                                    ))
                                    .collect();

                                assert_eq!(report.final_dispositions, expected);
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 144);
    }

    #[test]
    fn oversized_declared_caps_are_rejected_before_simulation() {
        let mut input = scenario(vec![svc("a")], vec![], 1);
        input.max_services = MAX_CONFIGURED_SERVICES + 1;
        assert!(matches!(simulate(&input), Err(SimulationError::InvalidInput(_))));

        input.max_services = 4;
        input.max_events = MAX_CONFIGURED_EVENTS + 1;
        assert!(matches!(simulate(&input), Err(SimulationError::InvalidInput(_))));
    }

}
