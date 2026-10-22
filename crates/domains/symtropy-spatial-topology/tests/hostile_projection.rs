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
        vec![
            InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Disconnected)
                .unwrap(),
        ],
        vec![source("device:test", 1, "same-source")],
    )
    .unwrap();

    let missing_projection =
        TopologySnapshot::derive(&boundary(missing), &occupancy_profile()).unwrap();
    let blocked_projection =
        TopologySnapshot::derive(&boundary(blocked), &occupancy_profile()).unwrap();

    assert_ne!(missing_projection, blocked_projection);
    assert!(
        missing_projection
            .graph(TopologyFacet::Occupancy)
            .unwrap()
            .interfaces()[0]
            .relation
            .is_unspecified()
    );
    assert!(
        blocked_projection
            .graph(TopologyFacet::Occupancy)
            .unwrap()
            .interfaces()[0]
            .relation
            .is_known_disconnected()
    );
}

#[test]
fn changed_exact_interface_provenance_changes_projection_even_when_relation_matches() {
    let make = |revision, digest| {
        BoundaryInterfaceSnapshot::new(
            BoundaryInterfaceId::new(id("interface:door")).unwrap(),
            region("region:a"),
            region("region:b"),
            vec![
                InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Disconnected)
                    .unwrap(),
            ],
            vec![source("device:door", revision, digest)],
        )
        .unwrap()
    };

    let before =
        TopologySnapshot::derive(&boundary(make(4, "closed-v4")), &occupancy_profile()).unwrap();
    let after =
        TopologySnapshot::derive(&boundary(make(5, "closed-v5")), &occupancy_profile()).unwrap();
    assert_ne!(before, after);
}

#[test]
fn qualified_class_is_explicitly_a_participating_relation() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:leaky-door")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![
            InterfaceFacetState::new(
                TopologyFacet::Occupancy,
                FacetRelation::QualifiedClass {
                    class_id: id("relation:passable-under-profile"),
                },
            )
            .unwrap(),
        ],
        vec![source("device:leaky-door", 1, "digest")],
    )
    .unwrap();

    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();
    assert_eq!(
        projection
            .candidate_neighbors(TopologyFacet::Occupancy, &region("region:a"))
            .unwrap(),
        vec![region("region:b")]
    );
}

#[test]
fn explicit_unspecified_is_rejected_so_missing_evidence_has_one_representation() {
    let result = InterfaceFacetState::new(TopologyFacet::AirPressure, FacetRelation::Unspecified);
    assert!(matches!(
        result,
        Err(TopologyError::ExplicitUnspecifiedFacet(
            TopologyFacet::AirPressure
        ))
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
            InterfaceFacetState::new(TopologyFacet::Visibility, FacetRelation::Disconnected)
                .unwrap(),
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

#[test]
fn same_claimed_provider_identity_cannot_launder_different_boundary_bodies() {
    let make = |relation| {
        boundary(
            BoundaryInterfaceSnapshot::new(
                BoundaryInterfaceId::new(id("interface:body-bound")).unwrap(),
                region("region:a"),
                region("region:b"),
                vec![InterfaceFacetState::new(TopologyFacet::Occupancy, relation).unwrap()],
                vec![source("device:body-bound", 1, "same-source")],
            )
            .unwrap(),
        )
    };

    let blocked = make(FacetRelation::Disconnected);
    let connected = make(FacetRelation::Connected);
    assert_eq!(
        blocked.provider_content_digest(),
        connected.provider_content_digest()
    );
    assert_ne!(blocked.exact_ref(), connected.exact_ref());
}

#[test]
fn same_profile_id_revision_cannot_launder_different_facet_sets() {
    let occupancy = TopologyProfile::new(
        id("topology-profile:same"),
        7,
        vec![TopologyFacet::Occupancy],
    )
    .unwrap();
    let pressure = TopologyProfile::new(
        id("topology-profile:same"),
        7,
        vec![TopologyFacet::AirPressure],
    )
    .unwrap();

    assert_ne!(occupancy.exact_ref(), pressure.exact_ref());
}

#[test]
fn undirected_interface_endpoint_order_is_canonical() {
    let facet = || {
        vec![InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Connected).unwrap()]
    };
    let refs = || vec![source("device:opening", 1, "same")];

    let forward = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:opening")).unwrap(),
        region("region:a"),
        region("region:b"),
        facet(),
        refs(),
    )
    .unwrap();
    let reverse = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:opening")).unwrap(),
        region("region:b"),
        region("region:a"),
        facet(),
        refs(),
    )
    .unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(boundary(forward).exact_ref(), boundary(reverse).exact_ref());
}

#[test]
fn derived_topology_binds_exact_profile_content() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:profile-bound")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Connected).unwrap()],
        vec![],
    )
    .unwrap();
    let profile = occupancy_profile();
    let topology = TopologySnapshot::derive(&boundary(interface), &profile).unwrap();
    assert_eq!(topology.profile_ref(), &profile.exact_ref());
}

#[test]
fn cross_scope_exact_source_equivocation_fails_closed() {
    let shared_a = source("source:shared", 9, "digest-a");
    let shared_b = source("source:shared", 9, "digest-b");
    let regions = vec![
        SpatialRegionSnapshot::new(region("region:a"), vec![shared_a]).unwrap(),
        SpatialRegionSnapshot::new(region("region:b"), vec![]).unwrap(),
    ];
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:cross-scope")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Connected).unwrap()],
        vec![shared_b],
    )
    .unwrap();

    let result = BoundarySnapshot::new(
        id("boundary:cross-scope-conflict"),
        1,
        "provider-digest",
        source("frame:cross-scope", 1, "frame"),
        source("environment:cross-scope", 1, "environment"),
        regions,
        vec![interface],
        vec![],
    );

    assert!(matches!(
        result,
        Err(TopologyError::ConflictingExactRef {
            field: "boundary.all_exact_refs",
            ..
        })
    ));
}

#[test]
fn identical_exact_source_may_support_multiple_derived_facts() {
    let shared = source("source:shared", 9, "same-digest");
    let regions = vec![
        SpatialRegionSnapshot::new(region("region:a"), vec![shared.clone()]).unwrap(),
        SpatialRegionSnapshot::new(region("region:b"), vec![]).unwrap(),
    ];
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:shared-source")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![
            InterfaceFacetState::new(TopologyFacet::Visibility, FacetRelation::Connected).unwrap(),
        ],
        vec![shared],
    )
    .unwrap();

    let result = BoundarySnapshot::new(
        id("boundary:shared-source"),
        1,
        "provider-digest",
        source("frame:shared-source", 1, "frame"),
        source("environment:shared-source", 1, "environment"),
        regions,
        vec![interface],
        vec![],
    );
    assert!(result.is_ok());
}

#[test]
fn qualified_relation_is_candidate_but_not_definite_connectivity() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:qualified-only")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![
            InterfaceFacetState::new(
                TopologyFacet::Occupancy,
                FacetRelation::QualifiedClass {
                    class_id: id("relation:passable-under-profile"),
                },
            )
            .unwrap(),
        ],
        vec![],
    )
    .unwrap();
    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();

    assert!(
        projection
            .definite_neighbors(TopologyFacet::Occupancy, &region("region:a"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        projection
            .candidate_neighbors(TopologyFacet::Occupancy, &region("region:a"))
            .unwrap(),
        vec![region("region:b")]
    );
}

#[test]
fn unconditional_connected_relation_is_both_definite_and_candidate() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:definite")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Connected).unwrap()],
        vec![],
    )
    .unwrap();
    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();

    let expected = vec![region("region:b")];
    assert_eq!(
        projection
            .definite_neighbors(TopologyFacet::Occupancy, &region("region:a"))
            .unwrap(),
        expected
    );
    assert_eq!(
        projection
            .candidate_neighbors(TopologyFacet::Occupancy, &region("region:a"))
            .unwrap(),
        vec![region("region:b")]
    );
}

#[test]
fn unknown_region_is_not_silently_treated_as_isolated() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:known-regions")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![
            InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Disconnected)
                .unwrap(),
        ],
        vec![],
    )
    .unwrap();
    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();

    assert!(matches!(
        projection.definite_neighbors(TopologyFacet::Occupancy, &region("region:missing")),
        Err(TopologyError::UnknownProjectedRegion(_))
    ));
}

#[test]
fn facet_omitted_by_profile_is_not_silently_treated_as_disconnected() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:facet-profile")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![
            InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Disconnected)
                .unwrap(),
            InterfaceFacetState::new(TopologyFacet::Visibility, FacetRelation::Connected).unwrap(),
        ],
        vec![],
    )
    .unwrap();
    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();

    assert!(matches!(
        projection.definite_neighbors(TopologyFacet::Visibility, &region("region:a")),
        Err(TopologyError::FacetNotProjected(TopologyFacet::Visibility))
    ));
}

#[test]
fn sealed_identity_accessors_preserve_exact_snapshot_subject() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:sealed-subject")).unwrap(),
        region("region:b"),
        region("region:a"),
        vec![
            InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Disconnected)
                .unwrap(),
        ],
        vec![],
    )
    .unwrap();
    assert_eq!(interface.id().0.as_str(), "interface:sealed-subject");
    assert_eq!(interface.first_region(), &region("region:a"));
    assert_eq!(interface.second_region(), &region("region:b"));

    let boundary = boundary(interface);
    let exact = boundary.exact_ref();

    assert_eq!(boundary.snapshot_id(), &exact.snapshot_id);
    assert_eq!(boundary.revision(), exact.revision);
    assert_eq!(boundary.content_digest(), exact.content_digest);
    assert_eq!(boundary.schema_version(), 1);
    assert_eq!(boundary.regions()[0].id(), &region("region:a"));
}
