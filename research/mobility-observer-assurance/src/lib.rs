// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fault-model-relative observer and measurement-channel assurance.
//!
//! Independence is evaluated against a declared dependency graph and a finite
//! set of declared fault scenarios. Two observer IDs are not independent merely
//! because their strings differ: shared power, clock, host, firmware, network,
//! calibration, installation, operator or other declared dependencies can make
//! their evidence common-cause vulnerable.
//!
//! This contract checks a supplied model and supplied sample records. It does
//! not authenticate them, prove the dependency inventory complete, measure real
//! sensor performance, establish NoEffect, or authorize an action.

use std::collections::BTreeSet;

pub const CONTRACT_VERSION: &str = "0.1";
pub const MAX_COMPONENTS: usize = 128;
pub const MAX_DEPENDENCY_EDGES: usize = 512;
pub const MAX_OBSERVER_CHANNELS: usize = 16;
pub const MAX_FAULT_SCENARIOS: usize = 128;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Commitment(pub [u8; 32]);

impl Commitment {
    fn is_nonzero(self) -> bool {
        self.0.iter().any(|byte| *byte != 0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyComponent {
    pub component_id: String,
    /// A common-cause label for every failure mechanism attributed to this
    /// component. Reusing a domain ID across components intentionally links
    /// their failure behavior for this analysis.
    pub failure_domain_id: String,
    pub kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyEdge {
    /// A component depends on another component.
    pub component_id: String,
    pub depends_on_component_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObserverChannel {
    pub channel_id: String,
    pub source_component_id: String,
    pub measurement_scope_commitment: Commitment,
    pub calibration_commitment: Commitment,
    pub clock_profile_id: String,
    pub maximum_time_uncertainty_ns: u64,
    pub maximum_sample_age_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObserverSample {
    pub channel_id: String,
    pub measurement_scope_commitment: Commitment,
    pub calibration_commitment: Commitment,
    pub clock_profile_id: String,
    pub time_uncertainty_ns: u64,
    pub observed_at_ns: u64,
    pub evaluated_at_ns: u64,
    pub calibration_valid: bool,
    pub placement_scope_matches: bool,
    pub operating_envelope_matches: bool,
    pub raw_evidence_commitment: Commitment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaultScenario {
    pub scenario_id: String,
    /// The declared components that fail or become untrustworthy together.
    pub failed_component_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObserverAssurancePacket {
    pub claim_id: String,
    /// Commitment to the exact physical claim, platform, effect predicate,
    /// operating scope and validity interval, computed by an external owner.
    pub claim_scope_commitment: Commitment,
    /// Commitment to the exact dependency graph, channel profiles and
    /// fault-scenario set, computed by an external owner.
    pub dependency_profile_commitment: Commitment,
    pub required_independent_observers: usize,
    pub components: Vec<DependencyComponent>,
    pub dependency_edges: Vec<DependencyEdge>,
    pub channels: Vec<ObserverChannel>,
    pub scenarios: Vec<FaultScenario>,
    pub samples: Vec<ObserverSample>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoldReason {
    MissingIdentity,
    ZeroProfileCommitment,
    EmptyDependencyGraph,
    TooManyComponents,
    TooManyDependencyEdges,
    TooManyObserverChannels,
    TooManyFaultScenarios,
    InvalidComponent,
    DuplicateComponent,
    InvalidDependencyEdge,
    DuplicateDependencyEdge,
    UnknownDependencyEndpoint,
    DependencyCycle,
    UnreferencedComponent,
    InvalidObserverChannel,
    MeasurementScopeMismatch,
    DuplicateObserverChannel,
    UnknownObserverSource,
    InvalidQuorum,
    MissingFaultScenarios,
    DuplicateFaultScenario,
    UnknownFaultComponent,
    DuplicateFaultComponent,
    MissingBaselineScenario,
    DuplicateSample,
    UnknownSampleChannel,
    NoSurvivingIndependentQuorum,
    InsufficientValidChannels,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObserverAssuranceDecision {
    /// The supplied samples and dependency model support the requested number
    /// of independent channels under every declared fault scenario. This is a
    /// scoped model result, not proof of physical sensor independence.
    QuorumQualifiedForDeclaredFaultModel {
        minimum_independent_observers: usize,
        evaluated_scenarios: usize,
    },
    Hold {
        reason: HoldReason,
        scenario_id: Option<String>,
        maximum_surviving_independent_observers: Option<usize>,
    },
}

#[derive(Clone, Debug)]
struct ChannelFootprint {
    channel_id: String,
    components: BTreeSet<String>,
    failure_domains: BTreeSet<String>,
    valid_sample: bool,
}

fn hold(reason: HoldReason) -> ObserverAssuranceDecision {
    ObserverAssuranceDecision::Hold {
        reason,
        scenario_id: None,
        maximum_surviving_independent_observers: None,
    }
}

fn scenario_hold(
    reason: HoldReason,
    scenario_id: &str,
    maximum_survivors: Option<usize>,
) -> ObserverAssuranceDecision {
    ObserverAssuranceDecision::Hold {
        reason,
        scenario_id: Some(scenario_id.to_owned()),
        maximum_surviving_independent_observers: maximum_survivors,
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn find_component<'a>(
    components: &'a [DependencyComponent],
    component_id: &str,
) -> Option<&'a DependencyComponent> {
    components
        .iter()
        .find(|component| component.component_id == component_id)
}

fn validate_packet(packet: &ObserverAssurancePacket) -> Result<(), HoldReason> {
    if !nonempty(&packet.claim_id) {
        return Err(HoldReason::MissingIdentity);
    }
    if !packet.claim_scope_commitment.is_nonzero()
        || !packet.dependency_profile_commitment.is_nonzero()
    {
        return Err(HoldReason::ZeroProfileCommitment);
    }
    if packet.components.is_empty() {
        return Err(HoldReason::EmptyDependencyGraph);
    }
    if packet.components.len() > MAX_COMPONENTS {
        return Err(HoldReason::TooManyComponents);
    }
    if packet.dependency_edges.len() > MAX_DEPENDENCY_EDGES {
        return Err(HoldReason::TooManyDependencyEdges);
    }
    if packet.channels.is_empty() || packet.channels.len() > MAX_OBSERVER_CHANNELS {
        return Err(HoldReason::TooManyObserverChannels);
    }
    if packet.scenarios.is_empty() || packet.scenarios.len() > MAX_FAULT_SCENARIOS {
        return Err(HoldReason::MissingFaultScenarios);
    }
    if packet.required_independent_observers < 2
        || packet.required_independent_observers > packet.channels.len()
    {
        return Err(HoldReason::InvalidQuorum);
    }

    for (index, component) in packet.components.iter().enumerate() {
        if !nonempty(&component.component_id)
            || !nonempty(&component.failure_domain_id)
            || !nonempty(&component.kind)
        {
            return Err(HoldReason::InvalidComponent);
        }
        if packet.components[..index]
            .iter()
            .any(|prior| prior.component_id == component.component_id)
        {
            return Err(HoldReason::DuplicateComponent);
        }
    }

    for (index, edge) in packet.dependency_edges.iter().enumerate() {
        if !nonempty(&edge.component_id)
            || !nonempty(&edge.depends_on_component_id)
            || edge.component_id == edge.depends_on_component_id
        {
            return Err(HoldReason::InvalidDependencyEdge);
        }
        if packet.dependency_edges[..index].iter().any(|prior| {
            prior.component_id == edge.component_id
                && prior.depends_on_component_id == edge.depends_on_component_id
        }) {
            return Err(HoldReason::DuplicateDependencyEdge);
        }
        if find_component(&packet.components, &edge.component_id).is_none()
            || find_component(&packet.components, &edge.depends_on_component_id).is_none()
        {
            return Err(HoldReason::UnknownDependencyEndpoint);
        }
    }

    for (index, channel) in packet.channels.iter().enumerate() {
        if !nonempty(&channel.channel_id)
            || !nonempty(&channel.source_component_id)
            || !nonempty(&channel.clock_profile_id)
            || !channel.measurement_scope_commitment.is_nonzero()
            || !channel.calibration_commitment.is_nonzero()
            || channel.maximum_time_uncertainty_ns == 0
            || channel.maximum_sample_age_ns == 0
        {
            return Err(HoldReason::InvalidObserverChannel);
        }
        if channel.measurement_scope_commitment != packet.claim_scope_commitment {
            return Err(HoldReason::MeasurementScopeMismatch);
        }
        if packet.channels[..index]
            .iter()
            .any(|prior| prior.channel_id == channel.channel_id)
        {
            return Err(HoldReason::DuplicateObserverChannel);
        }
        if find_component(&packet.components, &channel.source_component_id).is_none() {
            return Err(HoldReason::UnknownObserverSource);
        }
    }

    let mut baseline_count = 0_usize;
    for (index, scenario) in packet.scenarios.iter().enumerate() {
        if !nonempty(&scenario.scenario_id)
            || packet.scenarios[..index]
                .iter()
                .any(|prior| prior.scenario_id == scenario.scenario_id)
        {
            return Err(HoldReason::DuplicateFaultScenario);
        }
        if scenario.failed_component_ids.is_empty() {
            baseline_count += 1;
        }
        for (failed_index, component_id) in scenario.failed_component_ids.iter().enumerate() {
            if find_component(&packet.components, component_id).is_none() {
                return Err(HoldReason::UnknownFaultComponent);
            }
            if scenario.failed_component_ids[..failed_index]
                .iter()
                .any(|prior| prior == component_id)
            {
                return Err(HoldReason::DuplicateFaultComponent);
            }
        }
    }
    if baseline_count == 0 {
        return Err(HoldReason::MissingBaselineScenario);
    }

    for (index, sample) in packet.samples.iter().enumerate() {
        if packet.samples[..index]
            .iter()
            .any(|prior| prior.channel_id == sample.channel_id)
        {
            return Err(HoldReason::DuplicateSample);
        }
        if !packet
            .channels
            .iter()
            .any(|channel| channel.channel_id == sample.channel_id)
        {
            return Err(HoldReason::UnknownSampleChannel);
        }
    }

    // Detect cycles globally, including cycles disconnected from all observer
    // sources. Every declared component must also be reachable from a channel.
    let mut state = vec![0_u8; packet.components.len()];
    fn visit(
        index: usize,
        components: &[DependencyComponent],
        edges: &[DependencyEdge],
        state: &mut [u8],
        reachable: &mut [bool],
    ) -> Result<(), HoldReason> {
        if state[index] == 1 {
            return Err(HoldReason::DependencyCycle);
        }
        if state[index] == 2 {
            return Ok(());
        }
        state[index] = 1;
        reachable[index] = true;
        let component_id = &components[index].component_id;
        for edge in edges
            .iter()
            .filter(|edge| edge.component_id == *component_id)
        {
            let next = components
                .iter()
                .position(|component| component.component_id == edge.depends_on_component_id)
                .ok_or(HoldReason::UnknownDependencyEndpoint)?;
            visit(next, components, edges, state, reachable)?;
        }
        state[index] = 2;
        Ok(())
    }

    for index in 0..packet.components.len() {
        if state[index] == 0 {
            let mut local_reachable = vec![false; packet.components.len()];
            visit(
                index,
                &packet.components,
                &packet.dependency_edges,
                &mut state,
                &mut local_reachable,
            )?;
        }
    }

    let mut referenced = vec![false; packet.components.len()];
    for channel in &packet.channels {
        let root = packet
            .components
            .iter()
            .position(|component| component.component_id == channel.source_component_id)
            .ok_or(HoldReason::UnknownObserverSource)?;
        let mut local_seen = vec![false; packet.components.len()];
        let mut stack = vec![root];
        while let Some(current) = stack.pop() {
            if local_seen[current] {
                continue;
            }
            local_seen[current] = true;
            referenced[current] = true;
            let component_id = &packet.components[current].component_id;
            for edge in packet
                .dependency_edges
                .iter()
                .filter(|edge| edge.component_id == *component_id)
            {
                let next = packet
                    .components
                    .iter()
                    .position(|component| {
                        component.component_id == edge.depends_on_component_id
                    })
                    .expect("dependency endpoints were validated");
                stack.push(next);
            }
        }
    }
    if referenced.iter().any(|seen| !seen) {
        return Err(HoldReason::UnreferencedComponent);
    }

    Ok(())
}

fn sample_is_valid(channel: &ObserverChannel, sample: Option<&ObserverSample>) -> bool {
    let Some(sample) = sample else {
        return false;
    };
    sample.measurement_scope_commitment == channel.measurement_scope_commitment
        && sample.calibration_commitment == channel.calibration_commitment
        && sample.clock_profile_id == channel.clock_profile_id
        && sample.time_uncertainty_ns <= channel.maximum_time_uncertainty_ns
        && sample.observed_at_ns <= sample.evaluated_at_ns
        && sample.evaluated_at_ns - sample.observed_at_ns <= channel.maximum_sample_age_ns
        && sample.calibration_valid
        && sample.placement_scope_matches
        && sample.operating_envelope_matches
        && sample.raw_evidence_commitment.is_nonzero()
}

fn channel_footprint(
    packet: &ObserverAssurancePacket,
    channel: &ObserverChannel,
) -> ChannelFootprint {
    let mut component_ids = BTreeSet::new();
    let mut failure_domains = BTreeSet::new();
    let mut stack = vec![channel.source_component_id.clone()];
    while let Some(component_id) = stack.pop() {
        if !component_ids.insert(component_id.clone()) {
            continue;
        }
        if let Some(component) = find_component(&packet.components, &component_id) {
            failure_domains.insert(component.failure_domain_id.clone());
        }
        for edge in packet
            .dependency_edges
            .iter()
            .filter(|edge| edge.component_id == component_id)
        {
            stack.push(edge.depends_on_component_id.clone());
        }
    }
    let sample = packet
        .samples
        .iter()
        .find(|sample| sample.channel_id == channel.channel_id);
    ChannelFootprint {
        channel_id: channel.channel_id.clone(),
        components: component_ids,
        failure_domains,
        valid_sample: sample_is_valid(channel, sample),
    }
}

fn maximum_independent_channel_set(channels: &[ChannelFootprint]) -> Vec<String> {
    fn search(
        index: usize,
        channels: &[ChannelFootprint],
        used_domains: &mut BTreeSet<String>,
        selected: &mut Vec<String>,
        best: &mut Vec<String>,
    ) {
        if selected.len() + channels.len().saturating_sub(index) <= best.len() {
            return;
        }
        if index == channels.len() {
            if selected.len() > best.len() {
                *best = selected.clone();
            }
            return;
        }

        let channel = &channels[index];
        let independent = channel
            .failure_domains
            .iter()
            .all(|domain| !used_domains.contains(domain));
        if independent {
            for domain in &channel.failure_domains {
                used_domains.insert(domain.clone());
            }
            selected.push(channel.channel_id.clone());
            search(index + 1, channels, used_domains, selected, best);
            selected.pop();
            for domain in &channel.failure_domains {
                used_domains.remove(domain);
            }
        }
        search(index + 1, channels, used_domains, selected, best);
    }

    let mut best = Vec::new();
    search(
        0,
        channels,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut best,
    );
    best
}

/// Assess measurement-channel availability and common-cause resilience relative
/// to the exact declared component graph, observer profiles and fault scenarios.
///
/// The result qualifies only the declared fault model and scope. It is not
/// physical NoEffect, proof of inventory completeness, cryptographic trust, or
/// action authority.
pub fn assess_observer_assurance(
    packet: &ObserverAssurancePacket,
) -> ObserverAssuranceDecision {
    if let Err(reason) = validate_packet(packet) {
        return hold(reason);
    }

    let footprints: Vec<ChannelFootprint> = packet
        .channels
        .iter()
        .map(|channel| channel_footprint(packet, channel))
        .collect();

    for scenario in &packet.scenarios {
        let failed: BTreeSet<&str> = scenario
            .failed_component_ids
            .iter()
            .map(String::as_str)
            .collect();
        let survivors: Vec<ChannelFootprint> = footprints
            .iter()
            .filter(|channel| {
                channel.valid_sample
                    && !channel
                        .components
                        .iter()
                        .any(|component| failed.contains(component.as_str()))
            })
            .cloned()
            .collect();
        let independent_set = maximum_independent_channel_set(&survivors);
        if independent_set.len() < packet.required_independent_observers {
            let reason = if survivors.len() < packet.required_independent_observers {
                HoldReason::InsufficientValidChannels
            } else {
                HoldReason::NoSurvivingIndependentQuorum
            };
            return scenario_hold(reason, &scenario.scenario_id, Some(independent_set.len()));
        }
    }

    ObserverAssuranceDecision::QuorumQualifiedForDeclaredFaultModel {
        minimum_independent_observers: packet.required_independent_observers,
        evaluated_scenarios: packet.scenarios.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commitment(value: u8) -> Commitment {
        Commitment([value; 32])
    }

    fn component(id: &str, domain: &str, kind: &str) -> DependencyComponent {
        DependencyComponent {
            component_id: id.to_owned(),
            failure_domain_id: domain.to_owned(),
            kind: kind.to_owned(),
        }
    }

    fn edge(from: &str, dependency: &str) -> DependencyEdge {
        DependencyEdge {
            component_id: from.to_owned(),
            depends_on_component_id: dependency.to_owned(),
        }
    }

    fn channel(id: &str, source: &str) -> ObserverChannel {
        ObserverChannel {
            channel_id: id.to_owned(),
            source_component_id: source.to_owned(),
            measurement_scope_commitment: commitment(4),
            calibration_commitment: commitment(2),
            clock_profile_id: "clock-profile-independent".to_owned(),
            maximum_time_uncertainty_ns: 20,
            maximum_sample_age_ns: 500,
        }
    }

    fn sample(channel_id: &str) -> ObserverSample {
        ObserverSample {
            channel_id: channel_id.to_owned(),
            measurement_scope_commitment: commitment(4),
            calibration_commitment: commitment(2),
            clock_profile_id: "clock-profile-independent".to_owned(),
            time_uncertainty_ns: 10,
            observed_at_ns: 900,
            evaluated_at_ns: 1_000,
            calibration_valid: true,
            placement_scope_matches: true,
            operating_envelope_matches: true,
            raw_evidence_commitment: commitment(3),
        }
    }

    fn packet() -> ObserverAssurancePacket {
        ObserverAssurancePacket {
            claim_id: "old-attempt-no-effect".to_owned(),
            claim_scope_commitment: commitment(4),
            dependency_profile_commitment: commitment(5),
            required_independent_observers: 2,
            components: vec![
                component("sensor-a", "sensor-domain-a", "encoder"),
                component("power-a", "power-domain-a", "power"),
                component("clock-a", "clock-domain-a", "clock"),
                component("sensor-b", "sensor-domain-b", "laser"),
                component("power-b", "power-domain-b", "power"),
                component("clock-b", "clock-domain-b", "clock"),
                component("sensor-c", "sensor-domain-c", "camera"),
                component("power-c", "power-domain-c", "power"),
                component("clock-c", "clock-domain-c", "clock"),
            ],
            dependency_edges: vec![
                edge("sensor-a", "power-a"),
                edge("sensor-a", "clock-a"),
                edge("sensor-b", "power-b"),
                edge("sensor-b", "clock-b"),
                edge("sensor-c", "power-c"),
                edge("sensor-c", "clock-c"),
            ],
            channels: vec![
                channel("observer-a", "sensor-a"),
                channel("observer-b", "sensor-b"),
                channel("observer-c", "sensor-c"),
            ],
            scenarios: vec![
                FaultScenario {
                    scenario_id: "baseline".to_owned(),
                    failed_component_ids: Vec::new(),
                },
                FaultScenario {
                    scenario_id: "single-sensor-fault".to_owned(),
                    failed_component_ids: vec!["sensor-a".to_owned()],
                },
            ],
            samples: vec![sample("observer-a"), sample("observer-b"), sample("observer-c")],
        }
    }

    fn assert_hold(packet: &ObserverAssurancePacket, reason: HoldReason) {
        assert!(matches!(
            assess_observer_assurance(packet),
            ObserverAssuranceDecision::Hold { reason: actual, .. } if actual == reason
        ));
    }

    #[test]
    fn independent_channels_survive_declared_single_faults() {
        assert_eq!(
            assess_observer_assurance(&packet()),
            ObserverAssuranceDecision::QuorumQualifiedForDeclaredFaultModel {
                minimum_independent_observers: 2,
                evaluated_scenarios: 2
            }
        );
    }

    #[test]
    fn shared_power_dependency_is_not_independent() {
        let mut input = packet();
        input.required_independent_observers = 3;
        input.components.push(component("shared-power", "shared-power-domain", "power"));
        input.dependency_edges.push(edge("sensor-a", "shared-power"));
        input.dependency_edges.push(edge("sensor-b", "shared-power"));
        assert_hold(&input, HoldReason::NoSurvivingIndependentQuorum);
    }

    #[test]
    fn shared_clock_dependency_is_not_independent() {
        let mut input = packet();
        input.required_independent_observers = 3;
        input.components.push(component("shared-clock", "shared-clock-domain", "clock"));
        input.dependency_edges.push(edge("sensor-a", "shared-clock"));
        input.dependency_edges.push(edge("sensor-b", "shared-clock"));
        assert_hold(&input, HoldReason::NoSurvivingIndependentQuorum);
    }

    #[test]
    fn duplicate_failure_domain_labels_reveal_hidden_common_cause() {
        let mut input = packet();
        input.required_independent_observers = 3;
        input.components[4].failure_domain_id =
            input.components[1].failure_domain_id.clone();
        assert_hold(&input, HoldReason::NoSurvivingIndependentQuorum);
    }

    #[test]
    fn two_declared_faults_in_one_observer_path_leave_two_independent_channels() {
        let mut input = packet();
        input.required_independent_observers = 2;
        input.scenarios.push(FaultScenario {
            scenario_id: "two-faults-one-observer-path".to_owned(),
            failed_component_ids: vec!["sensor-a".to_owned(), "power-a".to_owned()],
        });
        assert_eq!(
            assess_observer_assurance(&input),
            ObserverAssuranceDecision::QuorumQualifiedForDeclaredFaultModel {
                minimum_independent_observers: 2,
                evaluated_scenarios: 3
            }
        );
    }

    #[test]
    fn common_cause_fault_that_drops_the_required_quorum_holds() {
        let mut input = packet();
        input.scenarios.push(FaultScenario {
            scenario_id: "shared-bus-loss".to_owned(),
            failed_component_ids: vec!["power-a".to_owned(), "power-b".to_owned()],
        });
        input.channels.pop();
        input.samples.pop();
        assert!(matches!(
            assess_observer_assurance(&input),
            ObserverAssuranceDecision::Hold {
                reason: HoldReason::InsufficientValidChannels,
                scenario_id: Some(id),
                ..
            } if id == "shared-bus-loss"
        ));
    }

    #[test]
    fn stale_sample_is_not_evidence_of_observer_availability() {
        let mut input = packet();
        input.required_independent_observers = 3;
        input.samples[0].observed_at_ns = 1;
        assert_eq!(
            assess_observer_assurance(&input),
            ObserverAssuranceDecision::Hold {
                reason: HoldReason::InsufficientValidChannels,
                scenario_id: Some("baseline".to_owned()),
                maximum_surviving_independent_observers: Some(2)
            }
        );
    }

    #[test]
    fn wrong_calibration_scope_or_clock_invalidates_a_channel() {
        let mut input = packet();
        input.required_independent_observers = 3;
        input.samples[0].calibration_commitment = commitment(99);
        assert_hold(&input, HoldReason::InsufficientValidChannels);

        let mut input = packet();
        input.samples[0].measurement_scope_commitment = commitment(88);
        assert_hold(&input, HoldReason::InsufficientValidChannels);

        let mut input = packet();
        input.samples[0].clock_profile_id = "different-clock".to_owned();
        assert_hold(&input, HoldReason::InsufficientValidChannels);
    }

    #[test]
    fn calibration_placement_and_operating_envelope_must_be_valid() {
        let mut input = packet();
        input.required_independent_observers = 3;
        input.samples[0].calibration_valid = false;
        assert_hold(&input, HoldReason::InsufficientValidChannels);

        let mut input = packet();
        input.samples[0].placement_scope_matches = false;
        assert_hold(&input, HoldReason::InsufficientValidChannels);

        let mut input = packet();
        input.samples[0].operating_envelope_matches = false;
        assert_hold(&input, HoldReason::InsufficientValidChannels);
    }

    #[test]
    fn time_uncertainty_and_future_samples_are_rejected() {
        let mut input = packet();
        input.required_independent_observers = 3;
        input.samples[0].time_uncertainty_ns = 21;
        assert_hold(&input, HoldReason::InsufficientValidChannels);

        let mut input = packet();
        input.samples[0].observed_at_ns = 1_001;
        assert_hold(&input, HoldReason::InsufficientValidChannels);
    }

    #[test]
    fn missing_or_zero_raw_evidence_does_not_count() {
        let mut input = packet();
        input.required_independent_observers = 3;
        input.samples.remove(0);
        assert_hold(&input, HoldReason::InsufficientValidChannels);

        let mut input = packet();
        input.samples[0].raw_evidence_commitment = commitment(0);
        assert_hold(&input, HoldReason::InsufficientValidChannels);
    }

    #[test]
    fn dependency_cycles_and_unknown_edges_hold() {
        let mut input = packet();
        input.dependency_edges.push(edge("power-a", "sensor-a"));
        assert_hold(&input, HoldReason::DependencyCycle);

        let mut input = packet();
        input.dependency_edges[0] = edge("sensor-a", "unknown-power");
        assert_hold(&input, HoldReason::UnknownDependencyEndpoint);
    }

    #[test]
    fn unreferenced_dependency_component_holds() {
        let mut input = packet();
        input.components.push(component("hidden-power", "hidden-domain", "power"));
        assert_hold(&input, HoldReason::UnreferencedComponent);
    }

    #[test]
    fn scenario_must_name_known_unique_failed_components() {
        let mut input = packet();
        input.scenarios.push(FaultScenario {
            scenario_id: "unknown-fault".to_owned(),
            failed_component_ids: vec!["nonexistent".to_owned()],
        });
        assert_hold(&input, HoldReason::UnknownFaultComponent);

        let mut input = packet();
        input.scenarios[1].failed_component_ids =
            vec!["sensor-a".to_owned(), "sensor-a".to_owned()];
        assert_hold(&input, HoldReason::DuplicateFaultComponent);
    }

    #[test]
    fn baseline_fault_scenario_is_mandatory() {
        let mut input = packet();
        input.scenarios.retain(|scenario| !scenario.failed_component_ids.is_empty());
        assert_hold(&input, HoldReason::MissingBaselineScenario);
    }

    #[test]
    fn duplicate_component_channel_edge_scenario_and_sample_ids_hold() {
        let mut input = packet();
        input.components[1].component_id = input.components[0].component_id.clone();
        assert_hold(&input, HoldReason::DuplicateComponent);

        let mut input = packet();
        input.channels[1].channel_id = input.channels[0].channel_id.clone();
        assert_hold(&input, HoldReason::DuplicateObserverChannel);

        let mut input = packet();
        input.dependency_edges[1] = input.dependency_edges[0].clone();
        assert_hold(&input, HoldReason::DuplicateDependencyEdge);

        let mut input = packet();
        input.scenarios[1].scenario_id = input.scenarios[0].scenario_id.clone();
        assert_hold(&input, HoldReason::DuplicateFaultScenario);

        let mut input = packet();
        input.samples[1].channel_id = input.samples[0].channel_id.clone();
        assert_hold(&input, HoldReason::DuplicateSample);
    }

    #[test]
    fn profile_commitments_and_channel_quorum_are_required() {
        let mut input = packet();
        input.claim_scope_commitment = commitment(0);
        assert_hold(&input, HoldReason::ZeroProfileCommitment);

        let mut input = packet();
        input.required_independent_observers = 1;
        assert_hold(&input, HoldReason::InvalidQuorum);

        let mut input = packet();
        input.required_independent_observers = 4;
        assert_hold(&input, HoldReason::InvalidQuorum);
    }

    #[test]
    fn source_component_and_clock_profile_are_required() {
        let mut input = packet();
        input.channels[0].source_component_id = "missing-sensor".to_owned();
        assert_hold(&input, HoldReason::UnknownObserverSource);

        let mut input = packet();
        input.channels[0].clock_profile_id.clear();
        assert_hold(&input, HoldReason::InvalidObserverChannel);
    }

    #[test]
    fn failure_scenarios_are_all_required_and_report_minimum_surviving_quorum() {
        let mut input = packet();
        input.scenarios.push(FaultScenario {
            scenario_id: "fail-all-observers".to_owned(),
            failed_component_ids: vec![
                "sensor-a".to_owned(),
                "sensor-b".to_owned(),
                "sensor-c".to_owned(),
            ],
        });
        assert_eq!(
            assess_observer_assurance(&input),
            ObserverAssuranceDecision::Hold {
                reason: HoldReason::InsufficientValidChannels,
                scenario_id: Some("fail-all-observers".to_owned()),
                maximum_surviving_independent_observers: Some(0)
            }
        );
    }

    #[test]
    fn sample_cannot_be_for_an_unknown_channel() {
        let mut input = packet();
        input.samples[0].channel_id = "unregistered-observer".to_owned();
        assert_hold(&input, HoldReason::UnknownSampleChannel);
    }

    #[test]
    fn graph_and_channel_inventory_limits_fail_closed() {
        let mut input = packet();
        for index in 0..MAX_OBSERVER_CHANNELS {
            let id = format!("extra-channel-{index}");
            let source = format!("extra-sensor-{index}");
            input.components.push(component(&source, &format!("extra-domain-{index}"), "sensor"));
            input.channels.push(channel(&id, &source));
            input.samples.push(sample(&id));
        }
        assert_hold(&input, HoldReason::TooManyObserverChannels);

        let mut input = packet();
        input.components.clear();
        input.components.extend((0..=MAX_COMPONENTS).map(|index| {
            component(&format!("c{index}"), &format!("d{index}"), "node")
        }));
        assert_hold(&input, HoldReason::TooManyComponents);
    }
}
