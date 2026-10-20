// SPDX-License-Identifier: AGPL-3.0-or-later
//! Vector-valued design-for-disassembly (DfD) metadata assessment.
//!
//! This evaluates declared connector metadata and completeness only. It does
//! not establish structural safety, a safe removal order, or demolition approval.

use crate::{Check, World, validate_token};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectorKind {
    ReversibleFastener,
    CutRequired,
    BondedOrMonolithic,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessState {
    Accessible,
    Constrained,
    Blocked,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequiredToolClass {
    ToolFree,
    StandardHandTool,
    PoweredHandTool,
    ThermalCutting,
    SpecializedRig,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DamageRisk {
    Low,
    Moderate,
    High,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectorReuse {
    Reusable,
    NotReusable,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorProfile {
    pub connection_id: String,
    pub kind: ConnectorKind,
    /// Declared labor estimate. It is not a measured work rate.
    pub disassembly_effort_person_minutes: Option<u64>,
    pub tool_access: AccessState,
    pub required_tool_class: RequiredToolClass,
    pub inspection_access: AccessState,
    pub damage_risk: DamageRisk,
    pub connector_reuse: ConnectorReuse,
}

impl ConnectorProfile {
    pub fn unknown(connection_id: impl Into<String>) -> Self {
        Self {
            connection_id: connection_id.into(),
            kind: ConnectorKind::Unknown,
            disassembly_effort_person_minutes: None,
            tool_access: AccessState::Unknown,
            required_tool_class: RequiredToolClass::Unknown,
            inspection_access: AccessState::Unknown,
            damage_risk: DamageRisk::Unknown,
            connector_reuse: ConnectorReuse::Unknown,
        }
    }

    fn metadata_complete(&self) -> bool {
        self.kind != ConnectorKind::Unknown
            && self.disassembly_effort_person_minutes.is_some()
            && self.tool_access != AccessState::Unknown
            && self.required_tool_class != RequiredToolClass::Unknown
            && self.inspection_access != AccessState::Unknown
            && self.damage_risk != DamageRisk::Unknown
            && self.connector_reuse != ConnectorReuse::Unknown
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConnectorKindCounts {
    pub reversible_fastener: usize,
    pub cut_required: usize,
    pub bonded_or_monolithic: usize,
    pub unknown: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AccessCounts {
    pub accessible: usize,
    pub constrained: usize,
    pub blocked: usize,
    pub unknown: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RequiredToolCounts {
    pub tool_free: usize,
    pub standard_hand_tool: usize,
    pub powered_hand_tool: usize,
    pub thermal_cutting: usize,
    pub specialized_rig: usize,
    pub unknown: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DamageRiskCounts {
    pub low: usize,
    pub moderate: usize,
    pub high: usize,
    pub unknown: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConnectorReuseCounts {
    pub reusable: usize,
    pub not_reusable: usize,
    pub unknown: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DfdVector {
    pub connection_count: usize,
    pub connector_kinds: ConnectorKindCounts,
    pub effort_declared_count: usize,
    /// Present only when every connection has a declared effort estimate.
    pub declared_effort_total_person_minutes: Option<u64>,
    pub tool_access: AccessCounts,
    pub required_tool_class: RequiredToolCounts,
    pub inspection_access: AccessCounts,
    pub damage_risk: DamageRiskCounts,
    pub connector_reuse: ConnectorReuseCounts,
    pub metadata_complete_count: usize,
    pub metadata_incomplete_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssessmentStatus {
    Complete,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DfdAssessment {
    pub source_revision: u64,
    /// Canonical state binding for comparison/replay, not a cryptographic or
    /// physical-safety certificate.
    pub source_state_snapshot: String,
    pub status: AssessmentStatus,
    pub vector: DfdVector,
    pub unresolved_connection_ids: Vec<String>,
    pub claim_ceiling: &'static str,
}

/// Assess all connections in the current world. Missing metadata is retained as
/// Unknown and makes the assessment Unresolved; duplicate profiles and profiles
/// for connections outside the scene are invalid input and are rejected.
pub fn assess(world: &World, profiles: Vec<ConnectorProfile>) -> Check<DfdAssessment> {
    let mut by_connection = BTreeMap::<String, ConnectorProfile>::new();
    for profile in profiles {
        validate_token(&profile.connection_id, "DfD connection ID")?;
        let id = profile.connection_id.clone();
        if by_connection.insert(id.clone(), profile).is_some() {
            return Err(format!("duplicate DfD profile for connection: {id}"));
        }
    }

    for id in by_connection.keys() {
        if !world.connections.contains_key(id) {
            return Err(format!("DfD profile references unknown connection: {id}"));
        }
    }

    let mut vector = DfdVector {
        declared_effort_total_person_minutes: Some(0),
        ..DfdVector::default()
    };
    let mut unresolved = BTreeSet::new();

    // World connections are stored in a BTreeMap, making traversal stable.
    for connection in world.connections.values() {
        vector.connection_count += 1;
        let profile = by_connection
            .get(&connection.id)
            .cloned()
            .unwrap_or_else(|| ConnectorProfile::unknown(connection.id.clone()));

        match profile.kind {
            ConnectorKind::ReversibleFastener => vector.connector_kinds.reversible_fastener += 1,
            ConnectorKind::CutRequired => vector.connector_kinds.cut_required += 1,
            ConnectorKind::BondedOrMonolithic => vector.connector_kinds.bonded_or_monolithic += 1,
            ConnectorKind::Unknown => vector.connector_kinds.unknown += 1,
        }

        match profile.disassembly_effort_person_minutes {
            Some(minutes) => {
                vector.effort_declared_count += 1;
                if let Some(current) = vector.declared_effort_total_person_minutes {
                    vector.declared_effort_total_person_minutes = Some(
                        current
                            .checked_add(minutes)
                            .ok_or_else(|| "declared DfD effort total overflow".to_owned())?,
                    );
                }
            }
            None => vector.declared_effort_total_person_minutes = None,
        }

        count_access(profile.tool_access, &mut vector.tool_access);
        count_required_tool(profile.required_tool_class, &mut vector.required_tool_class);
        count_access(profile.inspection_access, &mut vector.inspection_access);
        count_risk(profile.damage_risk, &mut vector.damage_risk);
        count_reuse(profile.connector_reuse, &mut vector.connector_reuse);

        if profile.metadata_complete() {
            vector.metadata_complete_count += 1;
        } else {
            vector.metadata_incomplete_count += 1;
            unresolved.insert(connection.id.clone());
        }
    }

    // No scalar score: the output preserves each dimension so users cannot hide
    // missing metadata or a weak dimension behind a weighted aggregate.
    let status = if vector.metadata_incomplete_count == 0 {
        AssessmentStatus::Complete
    } else {
        AssessmentStatus::Unresolved
    };

    Ok(DfdAssessment {
        source_revision: world.revision,
        source_state_snapshot: world.canonical_snapshot(),
        status,
        vector,
        unresolved_connection_ids: unresolved.into_iter().collect(),
        claim_ceiling: "declared DfD metadata completeness only; not structural safety or a safe-removal sequence",
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DfdFixtureSummary {
    pub status: AssessmentStatus,
    pub connection_count: usize,
    pub reversible_fastener_count: usize,
    pub cut_required_count: usize,
    pub declared_effort_total_person_minutes: Option<u64>,
    pub metadata_incomplete_count: usize,
}

/// Run a frozen synthetic DfD known-answer fixture for machine-readable receipts.
pub fn run_reference_fixture() -> Check<DfdFixtureSummary> {
    let mut world = World::default();
    for (id, material, mass_mg) in [
        ("A", "steel", 1_000),
        ("B", "concrete", 2_000),
        ("C", "timber", 1_500),
    ] {
        world.apply(crate::Event {
            id: format!("dfd-install-{id}"),
            operation: crate::Operation::InstallComponent(crate::Component {
                id: id.to_owned(),
                material: material.to_owned(),
                mass_mg,
            }),
        })?;
    }
    for (id, a, b) in [("AB", "A", "B"), ("BC", "B", "C")] {
        world.apply(crate::Event {
            id: format!("dfd-connect-{id}"),
            operation: crate::Operation::CreateConnection(crate::Connection {
                id: id.to_owned(),
                a: a.to_owned(),
                b: b.to_owned(),
            }),
        })?;
    }

    let assessment = assess(
        &world,
        vec![
            ConnectorProfile {
                connection_id: "AB".to_owned(),
                kind: ConnectorKind::ReversibleFastener,
                disassembly_effort_person_minutes: Some(5),
                tool_access: AccessState::Accessible,
                required_tool_class: RequiredToolClass::StandardHandTool,
                inspection_access: AccessState::Accessible,
                damage_risk: DamageRisk::Low,
                connector_reuse: ConnectorReuse::Reusable,
            },
            ConnectorProfile {
                connection_id: "BC".to_owned(),
                kind: ConnectorKind::CutRequired,
                disassembly_effort_person_minutes: Some(12),
                tool_access: AccessState::Constrained,
                required_tool_class: RequiredToolClass::ThermalCutting,
                inspection_access: AccessState::Blocked,
                damage_risk: DamageRisk::High,
                connector_reuse: ConnectorReuse::NotReusable,
            },
        ],
    )?;

    let summary = DfdFixtureSummary {
        status: assessment.status,
        connection_count: assessment.vector.connection_count,
        reversible_fastener_count: assessment.vector.connector_kinds.reversible_fastener,
        cut_required_count: assessment.vector.connector_kinds.cut_required,
        declared_effort_total_person_minutes: assessment
            .vector
            .declared_effort_total_person_minutes,
        metadata_incomplete_count: assessment.vector.metadata_incomplete_count,
    };
    if summary.status != AssessmentStatus::Complete
        || summary.connection_count != 2
        || summary.reversible_fastener_count != 1
        || summary.cut_required_count != 1
        || summary.declared_effort_total_person_minutes != Some(17)
        || summary.metadata_incomplete_count != 0
    {
        return Err(format!(
            "frozen DfD fixture failed its known-answer contract: {summary:?}"
        ));
    }
    Ok(summary)
}

fn count_access(value: AccessState, counts: &mut AccessCounts) {
    match value {
        AccessState::Accessible => counts.accessible += 1,
        AccessState::Constrained => counts.constrained += 1,
        AccessState::Blocked => counts.blocked += 1,
        AccessState::Unknown => counts.unknown += 1,
    }
}

fn count_required_tool(value: RequiredToolClass, counts: &mut RequiredToolCounts) {
    match value {
        RequiredToolClass::ToolFree => counts.tool_free += 1,
        RequiredToolClass::StandardHandTool => counts.standard_hand_tool += 1,
        RequiredToolClass::PoweredHandTool => counts.powered_hand_tool += 1,
        RequiredToolClass::ThermalCutting => counts.thermal_cutting += 1,
        RequiredToolClass::SpecializedRig => counts.specialized_rig += 1,
        RequiredToolClass::Unknown => counts.unknown += 1,
    }
}

fn count_risk(value: DamageRisk, counts: &mut DamageRiskCounts) {
    match value {
        DamageRisk::Low => counts.low += 1,
        DamageRisk::Moderate => counts.moderate += 1,
        DamageRisk::High => counts.high += 1,
        DamageRisk::Unknown => counts.unknown += 1,
    }
}

fn count_reuse(value: ConnectorReuse, counts: &mut ConnectorReuseCounts) {
    match value {
        ConnectorReuse::Reusable => counts.reusable += 1,
        ConnectorReuse::NotReusable => counts.not_reusable += 1,
        ConnectorReuse::Unknown => counts.unknown += 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Component, Connection, Event, Operation, World};

    fn world_with_two_connections() -> World {
        let mut world = World::default();
        for id in ["A", "B", "C"] {
            world
                .apply(Event {
                    id: format!("install-{id}"),
                    operation: Operation::InstallComponent(Component {
                        id: id.to_owned(),
                        material: "steel".to_owned(),
                        mass_mg: 1_000,
                    }),
                })
                .expect("component installation");
        }
        for (id, a, b) in [("AB", "A", "B"), ("BC", "B", "C")] {
            world
                .apply(Event {
                    id: format!("connect-{id}"),
                    operation: Operation::CreateConnection(Connection {
                        id: id.to_owned(),
                        a: a.to_owned(),
                        b: b.to_owned(),
                    }),
                })
                .expect("connection creation");
        }
        world
    }

    fn known_profile(id: &str, effort: u64, kind: ConnectorKind) -> ConnectorProfile {
        ConnectorProfile {
            connection_id: id.to_owned(),
            kind,
            disassembly_effort_person_minutes: Some(effort),
            tool_access: AccessState::Accessible,
            required_tool_class: RequiredToolClass::StandardHandTool,
            inspection_access: AccessState::Accessible,
            damage_risk: DamageRisk::Low,
            connector_reuse: ConnectorReuse::Reusable,
        }
    }

    #[test]
    fn complete_profiles_produce_a_vector_not_an_aggregate_score() {
        let world = world_with_two_connections();
        let result = assess(
            &world,
            vec![
                known_profile("AB", 5, ConnectorKind::ReversibleFastener),
                ConnectorProfile {
                    tool_access: AccessState::Constrained,
                    required_tool_class: RequiredToolClass::ThermalCutting,
                    inspection_access: AccessState::Blocked,
                    damage_risk: DamageRisk::High,
                    connector_reuse: ConnectorReuse::NotReusable,
                    ..known_profile("BC", 12, ConnectorKind::CutRequired)
                },
            ],
        )
        .expect("known profiles");
        assert_eq!(result.status, AssessmentStatus::Complete);
        assert_eq!(result.vector.connection_count, 2);
        assert_eq!(result.vector.connector_kinds.reversible_fastener, 1);
        assert_eq!(result.vector.connector_kinds.cut_required, 1);
        assert_eq!(result.vector.declared_effort_total_person_minutes, Some(17));
        assert_eq!(result.vector.tool_access.constrained, 1);
        assert_eq!(result.vector.required_tool_class.standard_hand_tool, 1);
        assert_eq!(result.vector.required_tool_class.thermal_cutting, 1);
        assert_eq!(result.vector.inspection_access.blocked, 1);
        assert_eq!(result.vector.damage_risk.high, 1);
        assert_eq!(result.vector.connector_reuse.not_reusable, 1);
        assert!(result.unresolved_connection_ids.is_empty());
        assert!(result.claim_ceiling.contains("not structural safety"));
    }

    #[test]
    fn missing_connector_semantics_are_unresolved_not_silently_scored() {
        let world = world_with_two_connections();
        let result = assess(
            &world,
            vec![known_profile("AB", 5, ConnectorKind::ReversibleFastener)],
        )
        .expect("missing profile remains assessable as unknown");
        assert_eq!(result.status, AssessmentStatus::Unresolved);
        assert_eq!(result.vector.connector_kinds.unknown, 1);
        assert_eq!(result.vector.tool_access.unknown, 1);
        assert_eq!(result.vector.required_tool_class.unknown, 1);
        assert_eq!(result.vector.metadata_incomplete_count, 1);
        assert_eq!(result.unresolved_connection_ids, vec!["BC"]);
        assert_eq!(result.vector.declared_effort_total_person_minutes, None);
    }

    #[test]
    fn explicit_unknown_fields_keep_the_entire_assessment_unresolved() {
        let world = world_with_two_connections();
        let mut profile = known_profile("AB", 3, ConnectorKind::ReversibleFastener);
        profile.damage_risk = DamageRisk::Unknown;
        let result = assess(
            &world,
            vec![profile, known_profile("BC", 4, ConnectorKind::CutRequired)],
        )
        .expect("unknown risk is representable");
        assert_eq!(result.status, AssessmentStatus::Unresolved);
        assert_eq!(result.unresolved_connection_ids, vec!["AB"]);
        assert_eq!(result.vector.damage_risk.unknown, 1);
        assert_eq!(result.vector.metadata_complete_count, 1);
    }

    #[test]
    fn unknown_required_tool_class_blocks_complete_assessment() {
        let world = world_with_two_connections();
        let mut profile = known_profile("AB", 5, ConnectorKind::ReversibleFastener);
        profile.required_tool_class = RequiredToolClass::Unknown;
        let result = assess(
            &world,
            vec![profile, known_profile("BC", 12, ConnectorKind::CutRequired)],
        )
        .expect("unknown tool class is representable");
        assert_eq!(result.status, AssessmentStatus::Unresolved);
        assert_eq!(result.vector.required_tool_class.unknown, 1);
        assert_eq!(result.unresolved_connection_ids, vec!["AB"]);
    }

    #[test]
    fn duplicate_and_foreign_connection_profiles_fail_closed() {
        let world = world_with_two_connections();
        let duplicate = assess(
            &world,
            vec![
                known_profile("AB", 5, ConnectorKind::ReversibleFastener),
                known_profile("AB", 6, ConnectorKind::CutRequired),
            ],
        );
        assert!(duplicate.unwrap_err().contains("duplicate DfD profile"));
        let foreign = assess(
            &world,
            vec![known_profile("XZ", 2, ConnectorKind::CutRequired)],
        );
        assert!(foreign.unwrap_err().contains("unknown connection"));
    }

    #[test]
    fn profile_input_order_does_not_change_the_vector_or_source_binding() {
        let world = world_with_two_connections();
        let a = known_profile("AB", 5, ConnectorKind::ReversibleFastener);
        let b = known_profile("BC", 12, ConnectorKind::CutRequired);
        let left = assess(&world, vec![a.clone(), b.clone()]).expect("left order");
        let right = assess(&world, vec![b, a]).expect("right order");
        assert_eq!(left, right);
    }

    #[test]
    fn missing_effort_stays_unresolved_even_when_later_profiles_have_effort() {
        let world = world_with_two_connections();
        let mut first = known_profile("AB", 5, ConnectorKind::ReversibleFastener);
        first.disassembly_effort_person_minutes = None;
        let second = known_profile("BC", 12, ConnectorKind::CutRequired);
        let result = assess(&world, vec![first, second]).expect("missing effort is representable");
        assert_eq!(result.status, AssessmentStatus::Unresolved);
        assert_eq!(result.vector.effort_declared_count, 1);
        assert_eq!(result.vector.declared_effort_total_person_minutes, None);
        assert_eq!(result.unresolved_connection_ids, vec!["AB"]);
    }

    #[test]
    fn frozen_dfd_receipt_fixture_matches_declared_vector() {
        let result = run_reference_fixture().expect("frozen DfD fixture");
        assert_eq!(result.status, AssessmentStatus::Complete);
        assert_eq!(result.connection_count, 2);
        assert_eq!(result.reversible_fastener_count, 1);
        assert_eq!(result.cut_required_count, 1);
        assert_eq!(result.declared_effort_total_person_minutes, Some(17));
        assert_eq!(result.metadata_incomplete_count, 0);
    }

    #[test]
    fn effort_total_overflow_is_rejected() {
        let world = world_with_two_connections();
        let result = assess(
            &world,
            vec![
                known_profile("AB", u64::MAX, ConnectorKind::ReversibleFastener),
                known_profile("BC", 1, ConnectorKind::CutRequired),
            ],
        );
        assert!(result.unwrap_err().contains("effort total overflow"));
    }
}
