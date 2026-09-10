// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_game_state::StableId;
use symtropy_spatial_topology::{
    BoundaryInterfaceId, BoundaryInterfaceSnapshot, BoundarySnapshot, ExactSourceRef,
    FacetRelation, InterfaceFacetState, SpatialRegionId, SpatialRegionSnapshot, TopologyError,
    TopologyFacet, TopologyProfile, TopologySnapshot,
};

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn region(value: &str) -> SpatialRegionId {
    SpatialRegionId::new(id(value)).unwrap()
}

fn source(subject: &str, revision: u64, digest: &str) -> ExactSourceRef {
    ExactSourceRef::new(id("authority:test"), id(subject), revision, digest).unwrap()
}

fn boundary(interface: BoundaryInterfaceSnapshot) -> BoundarySnapshot {
    BoundarySnapshot::new(
        id("boundary:test"),
        1,
        "boundary-content",
        source("frame:test", 1, "frame"),
        source("environment:test", 1, "environment"),
        vec![
            SpatialRegionSnapshot::new(region("region:a"), vec![]).unwrap(),
            SpatialRegionSnapshot::new(region("region:b"), vec![]).unwrap(),
        ],
        vec![interface],
        vec![],
    )
    .unwrap()
}

fn occupancy_profile() -> TopologyProfile {
    TopologyProfile::new(
        id("topology-profile:occupancy"),
        1,
        vec![TopologyFacet::Occupancy],
    )
    .unwrap()
}

#[test]
fn missing_and_known_disconnected_are_not_equal() {
    let missing = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:test")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![],
        vec![source("device:test", 1, "same-source")],
    )
    .unwrap();
    let blocked = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:test")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(
            TopologyFacet::Occupancy,
            FacetRelation::Disconnected,
        )
        .unwrap()],
        vec![source("device:test", 1, "same-source")],
    )
    .unwrap();

    let missing_projection = TopologySnapshot::derive(&boundary(missing), &occupancy_profile()).unwrap();
    let blocked_projection = TopologySnapshot::derive(&boundary(blocked), &occupancy_profile()).unwrap();

    assert_ne!(missing_projection, blocked_projection);
    assert!(missing_projection.graph(TopologyFacet::Occupancy).unwrap().interfaces()[0]
        .relation
        .is_unspecified());
    assert!(blocked_projection.graph(TopologyFacet::Occupancy).unwrap().interfaces()[0]
        .relation
        .is_known_disconnected());
}

#[test]
fn changed_exact_interface_provenance_changes_projection_even_when_relation_matches() {
    let make = |revision, digest| {
        BoundaryInterfaceSnapshot::new(
            BoundaryInterfaceId::new(id("interface:door")).unwrap(),
            region("region:a"),
            region("region:b"),
            vec![InterfaceFacetState::new(
                TopologyFacet::Occupancy,
                FacetRelation::Disconnected,
            )
            .unwrap()],
            vec![source("device:door", revision, digest)],
        )
        .unwrap()
    };

    let before = TopologySnapshot::derive(&boundary(make(4, "closed-v4")), &occupancy_profile()).unwrap();
    let after = TopologySnapshot::derive(&boundary(make(5, "closed-v5")), &occupancy_profile()).unwrap();
    assert_ne!(before, after);
}

#[test]
fn qualified_class_is_explicitly_a_participating_relation() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:leaky-door")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(
            TopologyFacet::Occupancy,
            FacetRelation::QualifiedClass {
                class_id: id("relation:passable-under-profile"),
            },
        )
        .unwrap()],
        vec![source("device:leaky-door", 1, "digest")],
    )
    .unwrap();

    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();
    assert_eq!(
        projection.graph(TopologyFacet::Occupancy).unwrap().neighbors(&region("region:a")),
        vec![region("region:b")]
    );
}

#[test]
fn explicit_unspecified_is_rejected_so_missing_evidence_has_one_representation() {
    let result = InterfaceFacetState::new(TopologyFacet::AirPressure, FacetRelation::Unspecified);
    assert!(matches!(
        result,
        Err(TopologyError::ExplicitUnspecifiedFacet(TopologyFacet::AirPressure))
    ));
}

#[test]
fn duplicate_facet_claim_is_rejected_instead_of_last_writer_wins() {
    let result = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:conflict")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![
            InterfaceFacetState::new(TopologyFacet::Visibility, FacetRelation::Connected).unwrap(),
            InterfaceFacetState::new(TopologyFacet::Visibility, FacetRelation::Disconnected).unwrap(),
        ],
        vec![],
    );
    assert!(matches!(result, Err(TopologyError::DuplicateFacet { .. })));
}

#[test]
fn self_interface_is_rejected() {
    let same = region("region:a");
    let result = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:self")).unwrap(),
        same.clone(),
        same,
        vec![],
        vec![],
    );
    assert!(matches!(result, Err(TopologyError::SelfInterface(_))));
}
