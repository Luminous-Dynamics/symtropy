// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_basin::{BasinIntervention, BasinWorld};
use symtropy_sim_contracts::{
    AuthorityId, ObservationEvidence, ReferenceFrameId, RepresentationId, ScopeId, SimInstant,
    TypedDigest32,
};
use symtropy_world::{
    BasinCausalStateIdentity, BasinEnvironmentalIngestReceipt, BasinIngestEffect, BodyCellIdentity,
    BodyId, ClimateCellSummary, DerivedDomainView, GridSystem, HexCellId, HydrologyCellSummary,
    LivingWatershedPolicyV1, LivingWatershedReason, PlanetCellAuthorityView, TerrainCellSummary,
    WatershedConnectionEvidence, WatershedTopologySnapshot,
};

fn hydrology_authority() -> AuthorityId {
    AuthorityId::parse("hydrology.authority.v1").unwrap()
}

fn basin_authority() -> AuthorityId {
    AuthorityId::parse("basin.authority.v1").unwrap()
}

fn frame() -> ReferenceFrameId {
    ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap()
}

fn identity(name: &str) -> BodyCellIdentity {
    BodyCellIdentity {
        id: HexCellId::new(
            BodyId::earth(),
            GridSystem::BodyIcosahedral,
            7,
            name,
        ),
        center_lat_deg: -26.2,
        center_lon_deg: 28.0,
        area_m2: 100.0,
    }
}

fn scope(name: &str) -> ScopeId {
    identity(name).scope_id().unwrap()
}

fn relation(upstream: &str, downstream: &str, at: SimInstant) -> WatershedConnectionEvidence {
    WatershedConnectionEvidence::new(
        hydrology_authority(),
        scope(upstream),
        scope(downstream),
        frame(),
        at,
        TypedDigest32::sha256(
            "symtropy.hydrology.watershed-connectivity.edge.v1",
            1,
            format!("{upstream}->{downstream}").as_bytes(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn cell_view(
    name: &str,
    at: SimInstant,
    surface_water_m: f32,
    flow_accumulation: f32,
    hydrology_state_label: &[u8],
) -> PlanetCellAuthorityView {
    let identity = identity(name);
    let scope = identity.scope_id().unwrap();
    let terrain = DerivedDomainView::new(
        AuthorityId::parse("terrain.authority.v1").unwrap(),
        scope.clone(),
        frame(),
        RepresentationId::parse("terrain.local.v1").unwrap(),
        at,
        TypedDigest32::sha256("terrain.state.v1", 1, b"stable-terrain").unwrap(),
        TerrainCellSummary {
            elevation_m: 100.0,
            slope: 0.05,
        },
    )
    .unwrap();
    let hydrology = DerivedDomainView::new(
        hydrology_authority(),
        scope.clone(),
        frame(),
        RepresentationId::parse("hydrology.local.v1").unwrap(),
        at,
        TypedDigest32::sha256("hydrology.state.v1", 1, hydrology_state_label).unwrap(),
        HydrologyCellSummary {
            surface_water_m,
            groundwater_m: 1.0,
            flow_accumulation,
            salinity: 0.02,
        },
    )
    .unwrap();
    let climate = DerivedDomainView::new(
        AuthorityId::parse("climate.authority.v1").unwrap(),
        scope,
        frame(),
        RepresentationId::parse("climate.local.v1").unwrap(),
        at,
        TypedDigest32::sha256("climate.state.v1", 1, b"stable-climate").unwrap(),
        ClimateCellSummary {
            temperature_k: 291.0,
            atmosphere_pressure_pa: 100_000.0,
        },
    )
    .unwrap();

    PlanetCellAuthorityView {
        identity,
        terrain: None,
        hydrology: None,
        climate: None,
        ecology: None,
    }
    .with_terrain(terrain)
    .unwrap()
    .with_hydrology(hydrology)
    .unwrap()
    .with_climate(climate)
    .unwrap()
}

fn upstream_disturbance(at: SimInstant) -> ObservationEvidence {
    ObservationEvidence::new(
        hydrology_authority(),
        scope("a"),
        frame(),
        RepresentationId::parse("hydrology.local.v1").unwrap(),
        at,
        TypedDigest32::sha256("hydrology.state.v1", 1, b"a-upstream-flood").unwrap(),
    )
    .unwrap()
}

fn run_reference_chain() -> BasinEnvironmentalIngestReceipt {
    let topology_at = SimInstant::new(3_000, 0).unwrap();
    let topology = WatershedTopologySnapshot::new(
        hydrology_authority(),
        frame(),
        topology_at,
        vec![
            relation("a", "b", topology_at),
            relation("b", "c", topology_at),
        ],
    )
    .unwrap();
    let upstream = upstream_disturbance(topology_at);

    let reachable = topology.downstream_reachability(&scope("a")).unwrap();
    assert_eq!(reachable.len(), 2);
    assert_eq!(reachable[0].scope, scope("b"));
    assert_eq!(reachable[0].minimum_hops, 1);
    assert_eq!(reachable[1].scope, scope("c"));
    assert_eq!(reachable[1].minimum_hops, 2);

    // Connectivity alone changes no downstream physical state. C still has its
    // previously supplied benign Hydrology-authority observation.
    let before_at = topology_at;
    let before_c = cell_view("c", before_at, 0.02, 0.10, b"c-benign");
    let policy = LivingWatershedPolicyV1;
    let before = policy.evaluate(&before_c).unwrap();
    assert_eq!(before.proposal.intervention, None);
    assert_eq!(before.proposal.reason, LivingWatershedReason::Observe);

    // Only after Hydrology authority supplies a fresh downstream state does the
    // policy see floodplain conditions. Terrain/climate are also re-observed at
    // the same exact instant to satisfy the v0.5 coherence contract.
    let after_at = SimInstant::new(3_010, 0).unwrap();
    let after_c = cell_view("c", after_at, 1.0, 3.0, b"c-downstream-flood");
    let after = policy.evaluate(&after_c).unwrap();
    assert_eq!(
        after.proposal.intervention,
        Some(BasinIntervention::EcologicalReroute)
    );
    assert_eq!(after.proposal.reason, LivingWatershedReason::FloodplainPonding);

    // Basin ownership remains explicit: the proof harness records before state,
    // applies the proposed existing Basin intervention as the owner/executor,
    // and records resulting state before asking the policy to mint evidence.
    let mut basin = BasinWorld::old_waterworks(8, 5);
    let prior = basin.causal_state_digest().unwrap();
    basin.apply(after.proposal.intervention.unwrap());
    let resulting = basin.causal_state_digest().unwrap();

    policy
        .receipt_after_execution(
            basin_authority(),
            &after,
            prior,
            resulting,
            vec![topology.digest().unwrap(), upstream.digest().unwrap()],
        )
        .unwrap()
}

#[test]
fn upstream_event_requires_real_downstream_hydrology_before_policy_changes() {
    let receipt = run_reference_chain();
    assert_eq!(receipt.effect(), BasinIngestEffect::StateChanged);
    assert_eq!(receipt.scope, scope("c"));
    assert_eq!(receipt.at, SimInstant::new(3_010, 0).unwrap());
    assert_eq!(receipt.causal_parents.len(), 2);
    assert_eq!(
        receipt.causal_parents[0].domain,
        "symtropy.watershed.topology.v1"
    );
    assert_eq!(
        receipt.causal_parents[1].domain,
        "symtropy.observation-evidence.digest.v1"
    );
}

#[test]
fn complete_three_cell_proof_is_deterministic() {
    let left = run_reference_chain();
    let right = run_reference_chain();
    assert_eq!(left.digest().unwrap(), right.digest().unwrap());
}
