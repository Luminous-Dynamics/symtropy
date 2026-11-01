// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent synthetic campaign corpus for the Earth-to-orbit chain contract.
//!
//! The reference evaluator below deliberately does not call production helpers.
//! It recomputes the small, frozen semantics from public input fields so the
//! production disposition is checked against a separately understandable oracle.

use std::collections::{BTreeMap, BTreeSet};

use symtropy_earth_orbit_chain::{
    CampaignDisposition, CampaignV1, CargoLotV1, ChainNodeV1, EvidenceStatus, FaultScenarioV1,
    NodeRole, OrbitServiceV1, ResourceQualificationStatus, ResourceQualificationV1, TransferV1,
    evaluate_campaign,
};

fn qualified_orbit() -> EvidenceStatus {
    EvidenceStatus::Qualified {
        receipt_sha256: "a".repeat(64),
    }
}

fn qualified_resource() -> ResourceQualificationStatus {
    ResourceQualificationStatus::QualifiedForPlanning {
        receipt_sha256: "b".repeat(64),
    }
}

fn baseline() -> CampaignV1 {
    CampaignV1 {
        id: "campaign-001".to_owned(),
        cargo_lots: vec![CargoLotV1 {
            id: "payload-001".to_owned(),
            mass_grams: 1_000,
        }],
        nodes: vec![
            ChainNodeV1 {
                id: "factory".to_owned(),
                role: NodeRole::Supplier,
            },
            ChainNodeV1 {
                id: "port".to_owned(),
                role: NodeRole::PortOrAirportGateway,
            },
            ChainNodeV1 {
                id: "range".to_owned(),
                role: NodeRole::LaunchRange,
            },
            ChainNodeV1 {
                id: "orbit".to_owned(),
                role: NodeRole::OrbitDestination,
            },
        ],
        resource_qualifications: vec![
            ResourceQualificationV1 {
                id: "rail-proof".to_owned(),
                resource_id: "rail-section-01".to_owned(),
                time_profile_id: "unix-epoch-utc-v1".to_owned(),
                valid_from_epoch_seconds: 0,
                valid_until_epoch_seconds: 100,
                status: qualified_resource(),
            },
            ResourceQualificationV1 {
                id: "road-proof".to_owned(),
                resource_id: "road-section-01".to_owned(),
                time_profile_id: "unix-epoch-utc-v1".to_owned(),
                valid_from_epoch_seconds: 0,
                valid_until_epoch_seconds: 100,
                status: qualified_resource(),
            },
            ResourceQualificationV1 {
                id: "range-proof".to_owned(),
                resource_id: "range-slot-01".to_owned(),
                time_profile_id: "unix-epoch-utc-v1".to_owned(),
                valid_from_epoch_seconds: 0,
                valid_until_epoch_seconds: 100,
                status: qualified_resource(),
            },
        ],
        transfers: vec![
            TransferV1 {
                id: "leg-rail".to_owned(),
                cargo_lot_id: "payload-001".to_owned(),
                from_node_id: "factory".to_owned(),
                to_node_id: "port".to_owned(),
                resource_id: "rail-section-01".to_owned(),
                time_profile_id: "unix-epoch-utc-v1".to_owned(),
                resource_qualification_id: "rail-proof".to_owned(),
                mass_grams: 1_000,
                starts_at_epoch_seconds: 10,
                ends_at_epoch_seconds: 20,
                dependency_ids: vec!["rail-power".to_owned()],
            },
            TransferV1 {
                id: "leg-road".to_owned(),
                cargo_lot_id: "payload-001".to_owned(),
                from_node_id: "port".to_owned(),
                to_node_id: "range".to_owned(),
                resource_id: "road-section-01".to_owned(),
                time_profile_id: "unix-epoch-utc-v1".to_owned(),
                resource_qualification_id: "road-proof".to_owned(),
                mass_grams: 1_000,
                starts_at_epoch_seconds: 25,
                ends_at_epoch_seconds: 35,
                dependency_ids: vec!["access-road".to_owned()],
            },
            TransferV1 {
                id: "leg-launch".to_owned(),
                cargo_lot_id: "payload-001".to_owned(),
                from_node_id: "range".to_owned(),
                to_node_id: "orbit".to_owned(),
                resource_id: "range-slot-01".to_owned(),
                time_profile_id: "unix-epoch-utc-v1".to_owned(),
                resource_qualification_id: "range-proof".to_owned(),
                mass_grams: 1_000,
                starts_at_epoch_seconds: 40,
                ends_at_epoch_seconds: 50,
                dependency_ids: vec!["range-tracking".to_owned()],
            },
        ],
        orbit_service: OrbitServiceV1 {
            target_orbit_id: "leo-550-km".to_owned(),
            destination_node_id: "orbit".to_owned(),
            compatibility: qualified_orbit(),
        },
        required_delivery_by_epoch_seconds: 80,
        fault_scenarios: vec![FaultScenarioV1 {
            id: "rail-power-outage".to_owned(),
            failed_dependency_ids: vec!["rail-power".to_owned()],
        }],
    }
}

fn is_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Test-only reference model. It has no dependency on the production evaluator.
fn reference_disposition(campaign: &CampaignV1) -> CampaignDisposition {
    let mut infeasible = false;
    let mut needs_evidence = false;

    if !is_id(&campaign.id)
        || !is_id(&campaign.orbit_service.target_orbit_id)
        || !is_id(&campaign.orbit_service.destination_node_id)
        || campaign.cargo_lots.is_empty()
        || campaign.transfers.is_empty()
    {
        infeasible = true;
    }

    let mut cargo = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    let mut proofs = BTreeMap::new();
    let mut all_dependencies = BTreeSet::new();
    let mut transfer_ids = BTreeSet::new();

    for lot in &campaign.cargo_lots {
        if !is_id(&lot.id) || lot.mass_grams == 0 {
            infeasible = true;
        }
        if cargo.insert(lot.id.as_str(), lot).is_some() {
            infeasible = true;
        }
    }

    for node in &campaign.nodes {
        if !is_id(&node.id) || nodes.insert(node.id.as_str(), node).is_some() {
            infeasible = true;
        }
    }

    for proof in &campaign.resource_qualifications {
        if !is_id(&proof.id)
            || !is_id(&proof.resource_id)
            || !is_id(&proof.time_profile_id)
            || proof.valid_from_epoch_seconds >= proof.valid_until_epoch_seconds
        {
            infeasible = true;
        }
        if proofs.insert(proof.id.as_str(), proof).is_some() {
            infeasible = true;
        }
        match &proof.status {
            ResourceQualificationStatus::QualifiedForPlanning { receipt_sha256 } => {
                if !is_digest(receipt_sha256) {
                    needs_evidence = true;
                }
            }
            ResourceQualificationStatus::Unknown
            | ResourceQualificationStatus::Unsupported { .. } => {
                needs_evidence = true;
            }
            ResourceQualificationStatus::Rejected { .. } => {
                infeasible = true;
            }
        }
    }

    let mut by_cargo: BTreeMap<&str, Vec<&TransferV1>> = BTreeMap::new();

    for transfer in &campaign.transfers {
        if !is_id(&transfer.id)
            || !is_id(&transfer.cargo_lot_id)
            || !is_id(&transfer.from_node_id)
            || !is_id(&transfer.to_node_id)
            || !is_id(&transfer.resource_id)
            || !is_id(&transfer.time_profile_id)
            || !is_id(&transfer.resource_qualification_id)
            || !transfer_ids.insert(transfer.id.as_str())
        {
            infeasible = true;
        }

        let Some(lot) = cargo.get(transfer.cargo_lot_id.as_str()) else {
            infeasible = true;
            continue;
        };
        if transfer.mass_grams != lot.mass_grams {
            infeasible = true;
        }
        by_cargo
            .entry(transfer.cargo_lot_id.as_str())
            .or_default()
            .push(transfer);

        if !nodes.contains_key(transfer.from_node_id.as_str())
            || !nodes.contains_key(transfer.to_node_id.as_str())
            || transfer.from_node_id == transfer.to_node_id
            || transfer.starts_at_epoch_seconds >= transfer.ends_at_epoch_seconds
        {
            infeasible = true;
        }

        let mut seen_dependencies = BTreeSet::new();
        for dependency in &transfer.dependency_ids {
            if !is_id(dependency) || !seen_dependencies.insert(dependency.as_str()) {
                infeasible = true;
            }
            all_dependencies.insert(dependency.as_str());
        }

        match proofs.get(transfer.resource_qualification_id.as_str()) {
            None => needs_evidence = true,
            Some(proof) => {
                if proof.resource_id != transfer.resource_id
                    || proof.time_profile_id != transfer.time_profile_id
                    || transfer.starts_at_epoch_seconds < proof.valid_from_epoch_seconds
                    || transfer.ends_at_epoch_seconds > proof.valid_until_epoch_seconds
                {
                    infeasible = true;
                }
            }
        }
    }

    for lot in &campaign.cargo_lots {
        let Some(legs) = by_cargo.get_mut(lot.id.as_str()) else {
            infeasible = true;
            continue;
        };
        if legs.is_empty() {
            infeasible = true;
            continue;
        }
        legs.sort_by_key(|leg| {
            (
                leg.starts_at_epoch_seconds,
                leg.ends_at_epoch_seconds,
                leg.id.clone(),
            )
        });
        let mut seen_nodes = BTreeSet::new();
        if let Some(first) = legs.first() {
            seen_nodes.insert(first.from_node_id.as_str());
        }
        for (index, leg) in legs.iter().enumerate() {
            if !seen_nodes.insert(leg.to_node_id.as_str()) {
                infeasible = true;
            }
            if index > 0 {
                let previous = legs[index - 1];
                if previous.to_node_id != leg.from_node_id
                    || previous.ends_at_epoch_seconds > leg.starts_at_epoch_seconds
                {
                    infeasible = true;
                }
            }
        }
        if let Some(last) = legs.last() {
            if last.to_node_id != campaign.orbit_service.destination_node_id
                || last.ends_at_epoch_seconds > campaign.required_delivery_by_epoch_seconds
            {
                infeasible = true;
            }
        }
    }

    match nodes.get(campaign.orbit_service.destination_node_id.as_str()) {
        Some(node) if node.role == NodeRole::OrbitDestination => {}
        _ => infeasible = true,
    }

    match &campaign.orbit_service.compatibility {
        EvidenceStatus::Qualified { receipt_sha256 } => {
            if !is_digest(receipt_sha256) {
                needs_evidence = true;
            }
        }
        EvidenceStatus::Unknown | EvidenceStatus::Unsupported { .. } => {
            needs_evidence = true;
        }
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &campaign.fault_scenarios {
        if !is_id(&scenario.id) || !scenario_ids.insert(scenario.id.as_str()) {
            infeasible = true;
        }
        let mut scenario_dependencies = BTreeSet::new();
        for dependency in &scenario.failed_dependency_ids {
            if !is_id(dependency)
                || !scenario_dependencies.insert(dependency.as_str())
                || !all_dependencies.contains(dependency.as_str())
            {
                infeasible = true;
            }
        }
    }

    if infeasible {
        CampaignDisposition::Infeasible
    } else if needs_evidence {
        CampaignDisposition::NeedsEvidence
    } else {
        CampaignDisposition::Ready
    }
}

fn corpus() -> Vec<(&'static str, CampaignV1)> {
    let mut cases = vec![("01-valid-baseline", baseline())];

    let mut c = baseline();
    c.orbit_service.destination_node_id = "range".to_owned();
    cases.push(("02-wrong-destination", c));

    let mut c = baseline();
    c.required_delivery_by_epoch_seconds = 45;
    cases.push(("03-delivery-after-deadline", c));

    let mut c = baseline();
    c.transfers[1].mass_grams -= 1;
    cases.push(("04-mass-dropped-at-transfer", c));

    let mut c = baseline();
    c.transfers[1].from_node_id = "factory".to_owned();
    cases.push(("05-discontinuous-route", c));

    let mut c = baseline();
    c.transfers[1].starts_at_epoch_seconds = 19;
    cases.push(("06-overlapping-handoffs", c));

    let mut c = baseline();
    c.nodes.push(ChainNodeV1 {
        id: "detour".to_owned(),
        role: NodeRole::Other,
    });
    c.transfers[1].to_node_id = "detour".to_owned();
    c.transfers[2].from_node_id = "detour".to_owned();
    c.transfers[2].to_node_id = "factory".to_owned();
    cases.push(("07-cyclic-cargo-path", c));

    let mut c = baseline();
    c.transfers[0].resource_qualification_id = "absent-proof".to_owned();
    cases.push(("08-missing-resource-proof", c));

    let mut c = baseline();
    c.resource_qualifications[0].status = ResourceQualificationStatus::Unknown;
    cases.push(("09-unknown-resource-proof", c));

    let mut c = baseline();
    c.resource_qualifications[0].status = ResourceQualificationStatus::Rejected {
        reason: "resource owner rejected".to_owned(),
    };
    cases.push(("10-rejected-resource-proof", c));

    let mut c = baseline();
    c.resource_qualifications[0].status = ResourceQualificationStatus::Unsupported {
        reason: "operation class unsupported".to_owned(),
    };
    cases.push(("11-unsupported-resource-proof", c));

    let mut c = baseline();
    c.resource_qualifications[0].resource_id = "different-resource".to_owned();
    cases.push(("12-resource-identity-mismatch", c));

    let mut c = baseline();
    c.resource_qualifications[0].valid_until_epoch_seconds = 15;
    cases.push(("13-resource-proof-expired-before-transfer-end", c));

    let mut c = baseline();
    c.resource_qualifications[0].status = ResourceQualificationStatus::QualifiedForPlanning {
        receipt_sha256: "bad-digest".to_owned(),
    };
    cases.push(("14-malformed-resource-receipt", c));

    let mut c = baseline();
    c.orbit_service.compatibility = EvidenceStatus::Unknown;
    cases.push(("15-unknown-orbit-compatibility", c));

    let mut c = baseline();
    c.orbit_service.compatibility = EvidenceStatus::Unsupported {
        reason: "orbit model not supported".to_owned(),
    };
    cases.push(("16-unsupported-orbit-compatibility", c));

    let mut c = baseline();
    c.orbit_service.compatibility = EvidenceStatus::Qualified {
        receipt_sha256: "not-sha256".to_owned(),
    };
    cases.push(("17-malformed-orbit-receipt", c));

    let mut c = baseline();
    c.transfers[1].id = c.transfers[0].id.clone();
    cases.push(("18-duplicate-transfer-id", c));

    let mut c = baseline();
    let duplicate_node = c.nodes[0].clone();
    c.nodes.push(duplicate_node);
    cases.push(("19-duplicate-node-id", c));

    let mut c = baseline();
    c.transfers[0].to_node_id = "missing-node".to_owned();
    cases.push(("20-dangling-node-reference", c));

    let mut c = baseline();
    let duplicate_cargo = c.cargo_lots[0].clone();
    c.cargo_lots.push(duplicate_cargo);
    cases.push(("21-duplicate-cargo-id", c));

    let mut c = baseline();
    c.cargo_lots[0].mass_grams = 0;
    c.transfers.iter_mut().for_each(|leg| leg.mass_grams = 0);
    cases.push(("22-zero-mass-lot", c));

    let mut c = baseline();
    c.fault_scenarios[0].failed_dependency_ids = vec!["typo-dependency".to_owned()];
    cases.push(("23-unknown-failure-domain", c));

    let mut c = baseline();
    c.transfers[0].dependency_ids.push("rail-power".to_owned());
    cases.push(("24-duplicate-dependency", c));

    let mut c = baseline();
    c.transfers[0].starts_at_epoch_seconds = 20;
    c.transfers[0].ends_at_epoch_seconds = 20;
    cases.push(("25-empty-transfer-interval", c));

    let mut c = baseline();
    c.transfers[0].to_node_id = "factory".to_owned();
    cases.push(("26-self-transfer", c));

    let mut c = baseline();
    c.transfers.clear();
    cases.push(("27-empty-chain", c));

    let mut c = baseline();
    c.cargo_lots.clear();
    c.transfers.clear();
    cases.push(("28-empty-campaign", c));

    let mut c = baseline();
    c.resource_qualifications[0].valid_from_epoch_seconds = 100;
    c.resource_qualifications[0].valid_until_epoch_seconds = 100;
    cases.push(("29-invalid-resource-window", c));

    let mut c = baseline();
    c.resource_qualifications
        .push(c.resource_qualifications[0].clone());
    cases.push(("30-duplicate-resource-proof", c));

    let mut c = baseline();
    c.nodes[3].role = NodeRole::Other;
    cases.push(("31-destination-has-wrong-role", c));

    let mut c = baseline();
    c.id.clear();
    cases.push(("32-invalid-campaign-id", c));

    let mut c = baseline();
    c.transfers[0].time_profile_id = "tai-epoch-v1".to_owned();
    cases.push(("33-time-profile-mismatch", c));

    cases
}

fn frozen_expected_disposition(case_id: &str) -> CampaignDisposition {
    match case_id {
        "01-valid-baseline" => CampaignDisposition::Ready,
        "08-missing-resource-proof"
        | "09-unknown-resource-proof"
        | "11-unsupported-resource-proof"
        | "14-malformed-resource-receipt"
        | "15-unknown-orbit-compatibility"
        | "16-unsupported-orbit-compatibility"
        | "17-malformed-orbit-receipt" => CampaignDisposition::NeedsEvidence,
        "02-wrong-destination"
        | "03-delivery-after-deadline"
        | "04-mass-dropped-at-transfer"
        | "05-discontinuous-route"
        | "06-overlapping-handoffs"
        | "07-cyclic-cargo-path"
        | "10-rejected-resource-proof"
        | "12-resource-identity-mismatch"
        | "13-resource-proof-expired-before-transfer-end"
        | "18-duplicate-transfer-id"
        | "19-duplicate-node-id"
        | "20-dangling-node-reference"
        | "21-duplicate-cargo-id"
        | "22-zero-mass-lot"
        | "23-unknown-failure-domain"
        | "24-duplicate-dependency"
        | "25-empty-transfer-interval"
        | "26-self-transfer"
        | "27-empty-chain"
        | "28-empty-campaign"
        | "29-invalid-resource-window"
        | "30-duplicate-resource-proof"
        | "31-destination-has-wrong-role"
        | "32-invalid-campaign-id"
        | "33-time-profile-mismatch" => CampaignDisposition::Infeasible,
        _ => panic!("unexpected frozen corpus case: {case_id}"),
    }
}

#[test]
fn frozen_campaign_corpus_matches_independent_reference_oracle() {
    let cases = corpus();
    assert!(cases.len() >= 24, "corpus must cover at least 24 cases");

    for (case_id, campaign) in cases {
        let frozen_expected = frozen_expected_disposition(case_id);
        let oracle_expected = reference_disposition(&campaign);
        assert_eq!(
            oracle_expected, frozen_expected,
            "independent oracle differs from frozen manual disposition for {case_id}"
        );
        let actual = evaluate_campaign(&campaign);
        assert_eq!(
            actual.disposition, frozen_expected,
            "disposition mismatch for synthetic case {case_id}: findings={:?}",
            actual.findings
        );

        let expected_mass = campaign
            .cargo_lots
            .iter()
            .try_fold(0_u64, |sum, lot| sum.checked_add(lot.mass_grams))
            .unwrap_or(0);
        assert_eq!(
            actual.total_cargo_mass_grams, expected_mass,
            "mass total mismatch for synthetic case {case_id}"
        );

        let delivered = if frozen_expected == CampaignDisposition::Ready {
            expected_mass
        } else {
            0
        };
        assert_eq!(
            actual.accepted_delivered_cargo_mass_grams, delivered,
            "accepted delivery mismatch for synthetic case {case_id}"
        );
    }
}

#[test]
fn input_vector_permutations_do_not_change_the_canonical_report() {
    let mut forward = baseline();
    let mut reverse = baseline();

    reverse.cargo_lots.reverse();
    reverse.nodes.reverse();
    reverse.resource_qualifications.reverse();
    reverse.transfers.reverse();
    reverse.fault_scenarios.reverse();

    assert_eq!(evaluate_campaign(&forward), evaluate_campaign(&reverse));

    // Also check that the reference evaluator agrees on both permutations.
    assert_eq!(
        reference_disposition(&forward),
        reference_disposition(&reverse)
    );
    forward.transfers.swap(0, 2);
    assert_eq!(
        evaluate_campaign(&forward).disposition,
        reference_disposition(&forward)
    );
}
