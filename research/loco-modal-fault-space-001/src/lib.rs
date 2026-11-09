//! Bounded exact enumeration of common-cause sensor/actuator health signatures.
//!
//! Consumes source-qualified complete joint relations per common-cause group.
//! It enumerates a conservative set product across groups; it does not infer
//! probabilities, independence, or physical completeness of upstream relations.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_COMPONENTS: usize = 16;
pub const MAX_GROUP_TUPLES: usize = 4_096;
pub const MAX_GLOBAL_SIGNATURES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentKind {
    SensorD1,
    SensorD2,
    SensorD3,
    ExteroceptiveX1,
    NavigationX2,
    Actuator,
    Power,
    Clock,
    Compute,
    Communications,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthState {
    Valid,
    Degraded,
    Ood,
    Unavailable,
    Unknown,
    ThermalDerated,
    VoltageSag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultComponent {
    pub component_id: String,
    pub component_kind: ComponentKind,
    pub common_cause_group_id: String,
    pub allowed_health_states: Vec<HealthState>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentState {
    pub component_id: String,
    pub health_state: HealthState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointTuple {
    pub states: Vec<ComponentState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupReceipt {
    pub receipt_id: String,
    pub group_id: String,
    /// Opaque source-issued identities. This crate compares them; it does not
    /// calculate or authenticate cryptographic hashes.
    pub relation_digest: String,
    pub scenario_digest: String,
    pub model_digest: String,
    pub platform_physics_digest: String,
    pub actuator_digest: String,
    pub calibration_digest: String,
    pub diagnostic_model_digest: String,
    pub result_digest: String,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub complete_relation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultGroup {
    pub group_id: String,
    pub member_component_ids: Vec<String>,
    /// Each tuple must specify one state for every member component.
    pub allowed_joint_tuples: Vec<JointTuple>,
    pub receipt: GroupReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultSpaceContext {
    pub now_ns: u64,
    pub scenario_digest: String,
    pub model_digest: String,
    pub platform_physics_digest: String,
    pub actuator_digest: String,
    pub calibration_digest: String,
    pub diagnostic_model_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultSpaceProblem {
    pub context: FaultSpaceContext,
    pub components: Vec<FaultComponent>,
    pub groups: Vec<FaultGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupCount {
    pub group_id: String,
    pub joint_tuple_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthSignature {
    /// Canonical, stable component-order representation. Hashing/authentication
    /// is an upstream responsibility and is deliberately not claimed here.
    pub canonical_encoding: String,
    pub states: Vec<ComponentState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultEnvelopeReport {
    pub signatures: Vec<HealthSignature>,
    pub group_counts: Vec<GroupCount>,
    pub global_signature_count: usize,
    pub conservative_cross_group_overapproximation: bool,
    /// Canonical encoding for an upstream cryptographic hasher.
    pub canonical_set_encoding: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultSpaceError {
    EmptyIdentifier,
    NoComponents,
    NoGroups,
    TooManyComponents,
    DuplicateComponentId,
    EmptyHealthDomain,
    DuplicateHealthState,
    UnknownGroupComponent,
    ComponentGroupMismatch,
    ComponentMissingFromGroups,
    ComponentInMultipleGroups,
    DuplicateGroupId,
    EmptyGroupMembers,
    EmptyJointRelation,
    TooManyGroupTuples,
    DuplicateGroupMember,
    TupleCardinalityMismatch,
    DuplicateTupleComponent,
    UnknownTupleComponent,
    TupleComponentNotInGroup,
    TupleStateOutsideDomain,
    DuplicateJointTuple,
    InvalidReceiptInterval,
    StaleGroupReceipt,
    IdentityMismatch,
    IncompleteGroupRelation,
    DuplicateReceiptId,
    ProductOverflow,
    ResourceExhausted,
    DuplicateGlobalSignature,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn context_identifiers_valid(context: &FaultSpaceContext) -> bool {
    [
        context.scenario_digest.as_str(),
        context.model_digest.as_str(),
        context.platform_physics_digest.as_str(),
        context.actuator_digest.as_str(),
        context.calibration_digest.as_str(),
        context.diagnostic_model_digest.as_str(),
    ]
    .iter()
    .all(|value| nonempty(value))
}

fn health_name(state: HealthState) -> &'static str {
    match state {
        HealthState::Valid => "valid",
        HealthState::Degraded => "degraded",
        HealthState::Ood => "ood",
        HealthState::Unavailable => "unavailable",
        HealthState::Unknown => "unknown",
        HealthState::ThermalDerated => "thermal_derated",
        HealthState::VoltageSag => "voltage_sag",
    }
}

fn validate_group_receipt(
    context: &FaultSpaceContext,
    group: &FaultGroup,
) -> Result<(), FaultSpaceError> {
    let receipt = &group.receipt;
    if !nonempty(&receipt.receipt_id)
        || !nonempty(&receipt.group_id)
        || !nonempty(&receipt.relation_digest)
        || !nonempty(&receipt.scenario_digest)
        || !nonempty(&receipt.model_digest)
        || !nonempty(&receipt.platform_physics_digest)
        || !nonempty(&receipt.actuator_digest)
        || !nonempty(&receipt.calibration_digest)
        || !nonempty(&receipt.diagnostic_model_digest)
        || !nonempty(&receipt.result_digest)
    {
        return Err(FaultSpaceError::EmptyIdentifier);
    }
    if receipt.group_id != group.group_id {
        return Err(FaultSpaceError::IdentityMismatch);
    }
    if receipt.valid_from_ns > receipt.valid_until_ns {
        return Err(FaultSpaceError::InvalidReceiptInterval);
    }
    if context.now_ns < receipt.valid_from_ns || context.now_ns > receipt.valid_until_ns {
        return Err(FaultSpaceError::StaleGroupReceipt);
    }
    if receipt.scenario_digest != context.scenario_digest
        || receipt.model_digest != context.model_digest
        || receipt.platform_physics_digest != context.platform_physics_digest
        || receipt.actuator_digest != context.actuator_digest
        || receipt.calibration_digest != context.calibration_digest
        || receipt.diagnostic_model_digest != context.diagnostic_model_digest
    {
        return Err(FaultSpaceError::IdentityMismatch);
    }
    if !receipt.complete_relation {
        return Err(FaultSpaceError::IncompleteGroupRelation);
    }
    Ok(())
}

fn canonical_tuple(
    group: &FaultGroup,
    components: &BTreeMap<&str, &FaultComponent>,
    tuple: &JointTuple,
) -> Result<Vec<ComponentState>, FaultSpaceError> {
    if tuple.states.len() != group.member_component_ids.len() {
        return Err(FaultSpaceError::TupleCardinalityMismatch);
    }
    let member_ids: BTreeSet<&str> = group
        .member_component_ids
        .iter()
        .map(String::as_str)
        .collect();
    let mut tuple_ids = BTreeSet::new();
    for state in &tuple.states {
        if !nonempty(&state.component_id) {
            return Err(FaultSpaceError::EmptyIdentifier);
        }
        if !tuple_ids.insert(state.component_id.as_str()) {
            return Err(FaultSpaceError::DuplicateTupleComponent);
        }
        if !member_ids.contains(state.component_id.as_str()) {
            return Err(FaultSpaceError::TupleComponentNotInGroup);
        }
        let component = components
            .get(state.component_id.as_str())
            .ok_or(FaultSpaceError::UnknownTupleComponent)?;
        if !component
            .allowed_health_states
            .contains(&state.health_state)
        {
            return Err(FaultSpaceError::TupleStateOutsideDomain);
        }
    }
    if tuple_ids != member_ids {
        return Err(FaultSpaceError::TupleCardinalityMismatch);
    }
    let mut canonical = tuple.states.clone();
    canonical.sort();
    Ok(canonical)
}

fn encode_signature(states: &[ComponentState]) -> String {
    let mut encoding = String::new();
    for (index, state) in states.iter().enumerate() {
        if index > 0 {
            encoding.push('|');
        }
        encoding.push_str(&state.component_id);
        encoding.push('=');
        encoding.push_str(health_name(state.health_state));
    }
    encoding
}

fn canonicalize_problem(
    problem: &FaultSpaceProblem,
) -> Result<(Vec<FaultGroup>, BTreeMap<String, Vec<Vec<ComponentState>>>), FaultSpaceError> {
    if !context_identifiers_valid(&problem.context) {
        return Err(FaultSpaceError::EmptyIdentifier);
    }
    if problem.components.is_empty() {
        return Err(FaultSpaceError::NoComponents);
    }
    if problem.components.len() > MAX_COMPONENTS {
        return Err(FaultSpaceError::TooManyComponents);
    }
    if problem.groups.is_empty() {
        return Err(FaultSpaceError::NoGroups);
    }

    let mut components = BTreeMap::new();
    for component in &problem.components {
        if !nonempty(&component.component_id) || !nonempty(&component.common_cause_group_id) {
            return Err(FaultSpaceError::EmptyIdentifier);
        }
        if component.allowed_health_states.is_empty() {
            return Err(FaultSpaceError::EmptyHealthDomain);
        }
        let mut domain = BTreeSet::new();
        for state in &component.allowed_health_states {
            if !domain.insert(*state) {
                return Err(FaultSpaceError::DuplicateHealthState);
            }
        }
        if components
            .insert(component.component_id.as_str(), component)
            .is_some()
        {
            return Err(FaultSpaceError::DuplicateComponentId);
        }
    }

    let mut group_ids = BTreeSet::new();
    let mut receipt_ids = BTreeSet::new();
    let mut membership = BTreeMap::<&str, usize>::new();
    let mut groups = problem.groups.clone();
    let mut canonical_relations = BTreeMap::new();

    for group in &mut groups {
        if !nonempty(&group.group_id) {
            return Err(FaultSpaceError::EmptyIdentifier);
        }
        if !group_ids.insert(group.group_id.as_str()) {
            return Err(FaultSpaceError::DuplicateGroupId);
        }
        if group.member_component_ids.is_empty() {
            return Err(FaultSpaceError::EmptyGroupMembers);
        }
        if group.allowed_joint_tuples.is_empty() {
            return Err(FaultSpaceError::EmptyJointRelation);
        }
        if group.allowed_joint_tuples.len() > MAX_GROUP_TUPLES {
            return Err(FaultSpaceError::TooManyGroupTuples);
        }
        if !receipt_ids.insert(group.receipt.receipt_id.as_str()) {
            return Err(FaultSpaceError::DuplicateReceiptId);
        }
        validate_group_receipt(&problem.context, group)?;

        let mut members = BTreeSet::new();
        for component_id in &group.member_component_ids {
            if !nonempty(component_id) {
                return Err(FaultSpaceError::EmptyIdentifier);
            }
            if !members.insert(component_id.as_str()) {
                return Err(FaultSpaceError::DuplicateGroupMember);
            }
            let component = components
                .get(component_id.as_str())
                .ok_or(FaultSpaceError::UnknownGroupComponent)?;
            if component.common_cause_group_id != group.group_id {
                return Err(FaultSpaceError::ComponentGroupMismatch);
            }
            let count = membership.get(component_id.as_str()).copied().unwrap_or(0);
            membership.insert(
                component_id.as_str(),
                count.checked_add(1).ok_or(FaultSpaceError::ProductOverflow)?,
            );
        }
        group.member_component_ids.sort();

        let mut relation = Vec::with_capacity(group.allowed_joint_tuples.len());
        let mut encoded_tuples = BTreeSet::new();
        for tuple in &group.allowed_joint_tuples {
            let canonical = canonical_tuple(group, &components, tuple)?;
            let encoded = encode_signature(&canonical);
            if !encoded_tuples.insert(encoded) {
                return Err(FaultSpaceError::DuplicateJointTuple);
            }
            relation.push(canonical);
        }
        relation.sort();
        group.allowed_joint_tuples = relation
            .iter()
            .map(|states| JointTuple {
                states: states.clone(),
            })
            .collect();
        canonical_relations.insert(group.group_id.clone(), relation);
    }

    for component_id in components.keys() {
        match membership.get(component_id).copied().unwrap_or(0) {
            0 => return Err(FaultSpaceError::ComponentMissingFromGroups),
            1 => {}
            _ => return Err(FaultSpaceError::ComponentInMultipleGroups),
        }
    }

    groups.sort_by(|left, right| left.group_id.cmp(&right.group_id));
    Ok((groups, canonical_relations))
}

/// Enumerates the conservative set product of complete source-qualified
/// common-cause groups. If the product exceeds the cap, no partial report is
/// returned.
pub fn enumerate_fault_envelope(
    problem: &FaultSpaceProblem,
) -> Result<FaultEnvelopeReport, FaultSpaceError> {
    let (groups, relations) = canonicalize_problem(problem)?;
    let mut product_size = 1_usize;
    let mut group_counts = Vec::with_capacity(groups.len());
    for group in &groups {
        let count = relations
            .get(&group.group_id)
            .ok_or(FaultSpaceError::EmptyJointRelation)?
            .len();
        product_size = product_size
            .checked_mul(count)
            .ok_or(FaultSpaceError::ProductOverflow)?;
        if product_size > MAX_GLOBAL_SIGNATURES {
            return Err(FaultSpaceError::ResourceExhausted);
        }
        group_counts.push(GroupCount {
            group_id: group.group_id.clone(),
            joint_tuple_count: count,
        });
    }

    let mut partial: Vec<Vec<ComponentState>> = vec![Vec::new()];
    for group in &groups {
        let relation = relations
            .get(&group.group_id)
            .ok_or(FaultSpaceError::EmptyJointRelation)?;
        let mut next = Vec::with_capacity(
            partial
                .len()
                .checked_mul(relation.len())
                .ok_or(FaultSpaceError::ProductOverflow)?,
        );
        for prefix in &partial {
            for tuple in relation {
                let mut combined = Vec::with_capacity(prefix.len() + tuple.len());
                combined.extend(prefix.iter().cloned());
                combined.extend(tuple.iter().cloned());
                combined.sort();
                next.push(combined);
            }
        }
        partial = next;
    }

    if partial.len() != product_size {
        return Err(FaultSpaceError::ProductOverflow);
    }

    let mut seen = BTreeSet::new();
    let mut signatures = Vec::with_capacity(partial.len());
    for states in partial {
        let canonical_encoding = encode_signature(&states);
        if !seen.insert(canonical_encoding.clone()) {
            return Err(FaultSpaceError::DuplicateGlobalSignature);
        }
        signatures.push(HealthSignature {
            canonical_encoding,
            states,
        });
    }
    signatures.sort_by(|left, right| left.canonical_encoding.cmp(&right.canonical_encoding));

    let canonical_set_encoding = signatures
        .iter()
        .map(|signature| signature.canonical_encoding.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(FaultEnvelopeReport {
        global_signature_count: signatures.len(),
        signatures,
        group_counts,
        conservative_cross_group_overapproximation: groups.len() > 1,
        canonical_set_encoding,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(group_id: &str) -> GroupReceipt {
        GroupReceipt {
            receipt_id: format!("receipt-{group_id}"),
            group_id: group_id.to_owned(),
            relation_digest: format!("relation-{group_id}-v1"),
            scenario_digest: "scenario-v1".to_owned(),
            model_digest: "model-v1".to_owned(),
            platform_physics_digest: "platform-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
            diagnostic_model_digest: "diagnostic-v1".to_owned(),
            result_digest: format!("result-{group_id}-v1"),
            valid_from_ns: 100,
            valid_until_ns: 1_000,
            complete_relation: true,
        }
    }

    fn component(id: &str, group: &str, domain: &[HealthState]) -> FaultComponent {
        FaultComponent {
            component_id: id.to_owned(),
            component_kind: ComponentKind::SensorD1,
            common_cause_group_id: group.to_owned(),
            allowed_health_states: domain.to_vec(),
        }
    }

    fn tuple(items: &[(&str, HealthState)]) -> JointTuple {
        JointTuple {
            states: items
                .iter()
                .map(|(component_id, health_state)| ComponentState {
                    component_id: (*component_id).to_owned(),
                    health_state: *health_state,
                })
                .collect(),
        }
    }

    fn group(id: &str, members: &[&str], tuples: Vec<JointTuple>) -> FaultGroup {
        FaultGroup {
            group_id: id.to_owned(),
            member_component_ids: members.iter().map(|id| (*id).to_owned()).collect(),
            allowed_joint_tuples: tuples,
            receipt: receipt(id),
        }
    }

    fn context() -> FaultSpaceContext {
        FaultSpaceContext {
            now_ns: 500,
            scenario_digest: "scenario-v1".to_owned(),
            model_digest: "model-v1".to_owned(),
            platform_physics_digest: "platform-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
            diagnostic_model_digest: "diagnostic-v1".to_owned(),
        }
    }

    fn camera_pair_problem() -> FaultSpaceProblem {
        FaultSpaceProblem {
            context: context(),
            components: vec![
                component("rgb", "camera-unit", &[HealthState::Valid, HealthState::Ood, HealthState::Unavailable]),
                component("depth", "camera-unit", &[HealthState::Valid, HealthState::Ood, HealthState::Unavailable]),
            ],
            groups: vec![group(
                "camera-unit",
                &["rgb", "depth"],
                vec![
                    tuple(&[("rgb", HealthState::Valid), ("depth", HealthState::Valid)]),
                    tuple(&[("rgb", HealthState::Ood), ("depth", HealthState::Ood)]),
                    tuple(&[("rgb", HealthState::Unavailable), ("depth", HealthState::Unavailable)]),
                ],
            )],
        }
    }

    fn four_group_problem(tuple_count: usize) -> FaultSpaceProblem {
        let mut components = Vec::new();
        let mut groups = Vec::new();
        for index in 0..4 {
            let group_id = format!("group-{index}");
            let a = format!("component-{index}-a");
            let b = format!("component-{index}-b");
            let domain = [
                HealthState::Valid,
                HealthState::Degraded,
                HealthState::Ood,
                HealthState::Unavailable,
                HealthState::Unknown,
            ];
            components.push(component(&a, &group_id, &domain));
            components.push(component(&b, &group_id, &domain));
            let mut tuples = Vec::new();
            for tuple_index in 0..tuple_count {
                let left = domain[tuple_index % domain.len()];
                let right = domain[(tuple_index / domain.len()) % domain.len()];
                tuples.push(tuple(&[(&a, left), (&b, right)]));
            }
            groups.push(group(&group_id, &[&a, &b], tuples));
        }
        FaultSpaceProblem {
            context: context(),
            components,
            groups,
        }
    }

    #[test]
    fn common_camera_faults_are_preserved_as_joint_states() {
        let report = enumerate_fault_envelope(&camera_pair_problem()).expect("camera group");
        assert_eq!(report.global_signature_count, 3);
        assert_eq!(report.signatures[0].states.len(), 2);
        assert_eq!(report.signatures[0].states[0].health_state, HealthState::Valid);
        assert_eq!(
            report.conservative_cross_group_overapproximation,
            false
        );
    }

    #[test]
    fn derived_encoder_signals_can_share_one_joint_fault_group() {
        let problem = FaultSpaceProblem {
            context: context(),
            components: vec![
                component("encoder-position", "encoder-bus", &[HealthState::Valid, HealthState::Degraded, HealthState::Unavailable]),
                component("derived-velocity", "encoder-bus", &[HealthState::Valid, HealthState::Degraded, HealthState::Unavailable]),
            ],
            groups: vec![group(
                "encoder-bus",
                &["encoder-position", "derived-velocity"],
                vec![
                    tuple(&[("encoder-position", HealthState::Valid), ("derived-velocity", HealthState::Valid)]),
                    tuple(&[("encoder-position", HealthState::Degraded), ("derived-velocity", HealthState::Degraded)]),
                    tuple(&[("encoder-position", HealthState::Unavailable), ("derived-velocity", HealthState::Unavailable)]),
                ],
            )],
        };
        let report = enumerate_fault_envelope(&problem).expect("encoder group");
        assert_eq!(report.global_signature_count, 3);
    }

    #[test]
    fn d1_d2_shared_power_group_preserves_correlated_states() {
        let problem = FaultSpaceProblem {
            context: context(),
            components: vec![
                component("foot-pressure", "foot-bus", &[HealthState::Valid, HealthState::Degraded, HealthState::Unavailable]),
                component("foot-wrench", "foot-bus", &[HealthState::Valid, HealthState::Degraded, HealthState::Unavailable]),
            ],
            groups: vec![group(
                "foot-bus",
                &["foot-pressure", "foot-wrench"],
                vec![
                    tuple(&[("foot-pressure", HealthState::Valid), ("foot-wrench", HealthState::Valid)]),
                    tuple(&[("foot-pressure", HealthState::Degraded), ("foot-wrench", HealthState::Degraded)]),
                    tuple(&[("foot-pressure", HealthState::Unavailable), ("foot-wrench", HealthState::Unavailable)]),
                ],
            )],
        };
        let report = enumerate_fault_envelope(&problem).expect("shared power group");
        assert_eq!(report.global_signature_count, 3);
    }

    #[test]
    fn state_outside_component_domain_is_rejected() {
        let mut problem = camera_pair_problem();
        problem.groups[0].allowed_joint_tuples[0] =
            tuple(&[("rgb", HealthState::Valid), ("depth", HealthState::Degraded)]);
        assert_eq!(
            enumerate_fault_envelope(&problem),
            Err(FaultSpaceError::TupleStateOutsideDomain)
        );
    }

    #[test]
    fn duplicate_component_id_is_rejected() {
        let mut problem = camera_pair_problem();
        problem.components[1].component_id = "rgb".to_owned();
        assert_eq!(
            enumerate_fault_envelope(&problem),
            Err(FaultSpaceError::DuplicateComponentId)
        );
    }

    #[test]
    fn component_in_two_groups_is_rejected() {
        let mut problem = camera_pair_problem();
        problem.components.push(component(
            "other",
            "camera-unit",
            &[HealthState::Valid],
        ));
        problem.groups.push(group(
            "second-group",
            &["rgb", "other"],
            vec![tuple(&[
                ("rgb", HealthState::Valid),
                ("other", HealthState::Valid),
            ])],
        ));
        assert_eq!(
            enumerate_fault_envelope(&problem),
            Err(FaultSpaceError::ComponentGroupMismatch)
        );
    }

    #[test]
    fn component_missing_from_group_partition_is_rejected() {
        let mut problem = camera_pair_problem();
        problem.components.push(component(
            "imu",
            "imu-group",
            &[HealthState::Valid],
        ));
        assert_eq!(
            enumerate_fault_envelope(&problem),
            Err(FaultSpaceError::ComponentMissingFromGroups)
        );
    }

    #[test]
    fn duplicate_joint_tuple_is_rejected() {
        let mut problem = camera_pair_problem();
        problem.groups[0].allowed_joint_tuples.push(
            problem.groups[0].allowed_joint_tuples[0].clone(),
        );
        assert_eq!(
            enumerate_fault_envelope(&problem),
            Err(FaultSpaceError::DuplicateJointTuple)
        );
    }

    #[test]
    fn stale_group_receipt_is_rejected() {
        let mut problem = camera_pair_problem();
        problem.groups[0].receipt.valid_until_ns = 499;
        assert_eq!(
            enumerate_fault_envelope(&problem),
            Err(FaultSpaceError::StaleGroupReceipt)
        );
    }

    #[test]
    fn group_context_mismatch_is_rejected() {
        let mut problem = camera_pair_problem();
        problem.groups[0].receipt.actuator_digest = "actuator-v2".to_owned();
        assert_eq!(
            enumerate_fault_envelope(&problem),
            Err(FaultSpaceError::IdentityMismatch)
        );
    }

    #[test]
    fn incomplete_group_relation_is_unknown() {
        let mut problem = camera_pair_problem();
        problem.groups[0].receipt.complete_relation = false;
        assert_eq!(
            enumerate_fault_envelope(&problem),
            Err(FaultSpaceError::IncompleteGroupRelation)
        );
    }

    #[test]
    fn exact_global_cap_is_enumerated() {
        let report = enumerate_fault_envelope(&four_group_problem(16)).expect("at cap");
        assert_eq!(report.global_signature_count, MAX_GLOBAL_SIGNATURES);
        assert_eq!(report.signatures.len(), MAX_GLOBAL_SIGNATURES);
    }

    #[test]
    fn one_over_global_cap_returns_no_partial_output() {
        assert_eq!(
            enumerate_fault_envelope(&four_group_problem(17)),
            Err(FaultSpaceError::ResourceExhausted)
        );
    }

    #[test]
    fn input_permutation_preserves_canonical_output() {
        let original = enumerate_fault_envelope(&four_group_problem(2)).expect("original");
        let mut permuted = four_group_problem(2);
        permuted.components.reverse();
        permuted.groups.reverse();
        for group in &mut permuted.groups {
            group.member_component_ids.reverse();
            group.allowed_joint_tuples.reverse();
            for tuple in &mut group.allowed_joint_tuples {
                tuple.states.reverse();
            }
        }
        let reordered = enumerate_fault_envelope(&permuted).expect("permuted");
        assert_eq!(original, reordered);
    }

    #[test]
    fn cross_group_product_is_conservative_set_not_probability() {
        let problem = FaultSpaceProblem {
            context: context(),
            components: vec![
                component("rgb", "camera", &[HealthState::Valid, HealthState::Ood]),
                component("imu", "inertial", &[HealthState::Valid, HealthState::Degraded]),
            ],
            groups: vec![
                group(
                    "camera",
                    &["rgb"],
                    vec![
                        tuple(&[("rgb", HealthState::Valid)]),
                        tuple(&[("rgb", HealthState::Ood)]),
                    ],
                ),
                group(
                    "inertial",
                    &["imu"],
                    vec![
                        tuple(&[("imu", HealthState::Valid)]),
                        tuple(&[("imu", HealthState::Degraded)]),
                    ],
                ),
            ],
        };
        let report = enumerate_fault_envelope(&problem).expect("cross product");
        assert_eq!(report.global_signature_count, 4);
        assert!(report.conservative_cross_group_overapproximation);
        assert!(!report.canonical_set_encoding.is_empty());
    }

    #[test]
    fn actuator_states_are_enumerated_like_sensor_states() {
        let problem = FaultSpaceProblem {
            context: context(),
            components: vec![
                component("camera", "camera", &[HealthState::Valid, HealthState::Ood]),
                FaultComponent {
                    component_id: "drive".to_owned(),
                    component_kind: ComponentKind::Actuator,
                    common_cause_group_id: "drive-power".to_owned(),
                    allowed_health_states: vec![
                        HealthState::Valid,
                        HealthState::ThermalDerated,
                        HealthState::VoltageSag,
                    ],
                },
            ],
            groups: vec![
                group(
                    "camera",
                    &["camera"],
                    vec![
                        tuple(&[("camera", HealthState::Valid)]),
                        tuple(&[("camera", HealthState::Ood)]),
                    ],
                ),
                group(
                    "drive-power",
                    &["drive"],
                    vec![
                        tuple(&[("drive", HealthState::Valid)]),
                        tuple(&[("drive", HealthState::ThermalDerated)]),
                        tuple(&[("drive", HealthState::VoltageSag)]),
                    ],
                ),
            ],
        };
        let report = enumerate_fault_envelope(&problem).expect("actuator domain");
        assert_eq!(report.global_signature_count, 6);
        assert!(report.conservative_cross_group_overapproximation);
    }
}
