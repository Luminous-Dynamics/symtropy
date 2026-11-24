// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_flexibility_portfolio::{
    AssetAvailability, AssetDeliveryOutcome, FailureDomain, FailureDomainObservation,
    FailureDomainState, PortfolioAsset, PortfolioContract, PortfolioDefinition,
    PortfolioEventObservation, PortfolioIdentity, PortfolioStressEvent,
    evaluate_portfolio_resilience,
};

fn domain(id: &str, kind: &str) -> FailureDomain {
    FailureDomain::new(id, kind).unwrap()
}

fn asset(id: &str, site: &str, offer: f64, domains: &[&str]) -> PortfolioAsset {
    PortfolioAsset::new(
        id,
        site,
        &format!("pcc-{site}"),
        "facility-pcc",
        offer,
        domains.iter().map(|s| (*s).to_owned()).collect(),
        &format!("asset-evidence-{id}"),
    )
    .unwrap()
}

fn definition() -> PortfolioDefinition {
    PortfolioDefinition::new(
        PortfolioIdentity::new(
            "portfolio-1",
            "frequency-response-v1",
            "sha256:assets-v1",
            "sha256:domains-v1",
        )
        .unwrap(),
        PortfolioContract::new("frequency-response-v1", 2, 2, 9_000, 9_000, 1_000).unwrap(),
        vec![
            asset("a", "site-a", 60.0, &["region", "control"]),
            asset("b", "site-b", 50.0, &["region", "control", "telecom"]),
        ],
        vec![
            domain("region", "weather-region"),
            domain("control", "control-plane"),
            domain("telecom", "telecommunications"),
        ],
    )
    .unwrap()
}

fn outcome(
    asset_id: &str,
    delivered: f64,
    availability: AssetAvailability,
) -> AssetDeliveryOutcome {
    AssetDeliveryOutcome::new(
        asset_id,
        delivered,
        availability,
        format!("response-{asset_id}"),
        format!("sha256:response-{asset_id}"),
    )
    .unwrap()
}

fn event(
    event_id: &str,
    group: &str,
    scenario: &str,
    start: u64,
    required: f64,
    a: (f64, AssetAvailability),
    b: (f64, AssetAvailability),
    domain_states: &[(&str, FailureDomainState)],
) -> PortfolioEventObservation {
    PortfolioEventObservation::new(
        PortfolioStressEvent::new(
            event_id,
            group,
            scenario,
            start,
            start + 3_600_000_000_000,
            required,
            format!("event-evidence-{event_id}"),
            format!("sha256:event-{event_id}"),
        )
        .unwrap(),
        vec![outcome("a", a.0, a.1), outcome("b", b.0, b.1)],
        domain_states
            .iter()
            .map(|(id, state)| {
                FailureDomainObservation::new(
                    *id,
                    *state,
                    format!("domain-evidence-{id}-{event_id}"),
                    format!("sha256:domain-{id}-{event_id}"),
                )
                .unwrap()
            })
            .collect(),
    )
}

fn normal_states() -> [(&'static str, FailureDomainState); 3] {
    [
        ("region", FailureDomainState::Normal),
        ("control", FailureDomainState::Normal),
        ("telecom", FailureDomainState::Normal),
    ]
}

#[test]
fn complete_independent_assets_satisfy_portfolio_commitment() {
    let states = normal_states();
    let report = evaluate_portfolio_resilience(
        &definition(),
        &[
            event(
                "e1",
                "storm-1",
                "heatwave",
                0,
                100.0,
                (60.0, AssetAvailability::Available),
                (50.0, AssetAvailability::Available),
                &states,
            ),
            event(
                "e2",
                "storm-2",
                "grid-event",
                10_000_000_000_000,
                100.0,
                (60.0, AssetAvailability::Available),
                (50.0, AssetAvailability::Available),
                &states,
            ),
        ],
    )
    .unwrap();

    assert_eq!(report.total_offered_capacity_kw, 110.0);
    assert_eq!(report.passed_event_count, 2);
    assert_eq!(report.passed_incident_group_count, 2);
    assert_eq!(report.incident_group_pass_fraction_basis_points, 10_000.0);
    assert!(report.event_threshold_passed);
    assert!(report.incident_group_threshold_passed);
}

#[test]
fn shared_control_plane_failure_causes_observed_joint_shortfall_without_causal_claim() {
    let report = evaluate_portfolio_resilience(
        &definition(),
        &[
            event(
                "normal",
                "incident-normal",
                "normal",
                0,
                100.0,
                (60.0, AssetAvailability::Available),
                (50.0, AssetAvailability::Available),
                &normal_states(),
            ),
            event(
                "control-failure",
                "incident-control",
                "control-plane-outage",
                10_000_000_000_000,
                100.0,
                (0.0, AssetAvailability::Unavailable),
                (0.0, AssetAvailability::Unavailable),
                &[
                    ("region", FailureDomainState::Normal),
                    ("control", FailureDomainState::Failed),
                    ("telecom", FailureDomainState::Normal),
                ],
            ),
        ],
    )
    .unwrap();

    assert_eq!(report.passed_event_count, 1);
    assert_eq!(report.passed_incident_group_count, 1);
    assert_eq!(report.maximum_event_shortfall_kw, 100.0);
    let control = report
        .failure_domains
        .iter()
        .find(|d| d.domain_id == "control")
        .unwrap();
    assert_eq!(control.failed_event_count, 1);
    assert_eq!(control.portfolio_passes_when_failed, 0);
}

#[test]
fn missing_asset_outcome_is_rejected_not_assumed_zero_or_nominal() {
    let mut observation = event(
        "e1",
        "g1",
        "normal",
        0,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &normal_states(),
    );
    observation.asset_outcomes.pop();
    let error = evaluate_portfolio_resilience(&definition(), &[observation]).unwrap_err();
    assert!(matches!(
        error,
        symtropy_flexibility_portfolio::PortfolioResilienceError::MissingAssetOutcome { .. }
    ));
}

#[test]
fn duplicate_asset_outcome_is_rejected() {
    let observation = event(
        "e1",
        "g1",
        "normal",
        0,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &normal_states(),
    );
    observation
        .asset_outcomes
        .push(outcome("a", 60.0, AssetAvailability::Available));
    let error = evaluate_portfolio_resilience(&definition(), &[observation]).unwrap_err();
    assert!(matches!(
        error,
        symtropy_flexibility_portfolio::PortfolioResilienceError::DuplicateAssetOutcome { .. }
    ));
}

#[test]
fn omitted_failure_domain_state_is_rejected_but_explicit_unknown_is_preserved() {
    let missing = event(
        "e1",
        "g1",
        "normal",
        0,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &[
            ("region", FailureDomainState::Normal),
            ("control", FailureDomainState::Normal),
        ],
    );
    assert!(matches!(
        evaluate_portfolio_resilience(&definition(), &[missing]).unwrap_err(),
        symtropy_flexibility_portfolio::PortfolioResilienceError::MissingDomainObservation { .. }
    ));

    let unknown = event(
        "unknown",
        "g-unknown",
        "normal",
        0,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &[
            ("region", FailureDomainState::Unknown),
            ("control", FailureDomainState::Normal),
            ("telecom", FailureDomainState::Normal),
        ],
    );
    let normal = event(
        "normal",
        "g-normal",
        "normal",
        10_000_000_000_000,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &normal_states(),
    );
    let report = evaluate_portfolio_resilience(&definition(), &[unknown, normal]).unwrap();
    let region = report
        .failure_domains
        .iter()
        .find(|domain| domain.domain_id == "region")
        .unwrap();
    assert_eq!(region.unknown_event_count, 1);
    assert_eq!(region.normal_event_count, 1);
    let region_state = report.events[0]
        .failure_domain_observations
        .iter()
        .find(|domain| domain.domain_id == "region")
        .unwrap()
        .state;
    assert_eq!(region_state, FailureDomainState::Unknown);
}
#[test]
fn unavailable_asset_cannot_report_nonzero_lower_bound_delivery() {
    let error = AssetDeliveryOutcome::new(
        "a",
        1.0,
        AssetAvailability::Unavailable,
        "response-a",
        "sha256:a",
    )
    .unwrap_err();
    assert!(matches!(
        error,
        symtropy_flexibility_portfolio::PortfolioResilienceError::AvailabilityDeliveryMismatch { .. }
    ));
}

#[test]
fn unknown_asset_availability_cannot_mint_delivery_credit() {
    let result = AssetDeliveryOutcome::new(
        "a",
        0.0,
        AssetAvailability::Unknown,
        "response-a",
        "sha256:a",
    )
    .unwrap();
    assert_eq!(result.delivered_lower_bound_kw(), 0.0);
}

#[test]
fn delivered_lower_bound_cannot_exceed_offered_capacity() {
    let observation = event(
        "e1",
        "g1",
        "normal",
        0,
        100.0,
        (61.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &normal_states(),
    );
    let error = evaluate_portfolio_resilience(&definition(), &[observation]).unwrap_err();
    assert!(matches!(
        error,
        symtropy_flexibility_portfolio::PortfolioResilienceError::DeliveryExceedsOffer { .. }
    ));
}

#[test]
fn repeated_dispatch_episodes_share_one_incident_group_and_use_worst_margin() {
    let states = normal_states();
    let report = evaluate_portfolio_resilience(
        &definition(),
        &[
            event(
                "episode-1",
                "one-heatwave",
                "heatwave",
                0,
                100.0,
                (60.0, AssetAvailability::Available),
                (50.0, AssetAvailability::Available),
                &states,
            ),
            event(
                "episode-2",
                "one-heatwave",
                "heatwave",
                10_000_000_000_000,
                100.0,
                (60.0, AssetAvailability::Available),
                (0.0, AssetAvailability::Unavailable),
                &states,
            ),
            event(
                "independent",
                "separate-storm",
                "storm",
                20_000_000_000_000,
                100.0,
                (60.0, AssetAvailability::Available),
                (50.0, AssetAvailability::Available),
                &states,
            ),
        ],
    )
    .unwrap();

    assert_eq!(report.event_count, 3);
    assert_eq!(report.independent_incident_group_count, 2);
    assert_eq!(report.passed_event_count, 2);
    assert_eq!(report.passed_incident_group_count, 1);
    let heatwave = report
        .incident_groups
        .iter()
        .find(|g| g.incident_group_id == "one-heatwave")
        .unwrap();
    assert!(!heatwave.passed);
    assert_eq!(heatwave.maximum_shortfall_kw, 40.0);
    assert_eq!(report.empirical_upper_tail_mean_incident_shortfall_kw, 40.0);
}

#[test]
fn overlapping_domain_memberships_are_not_summed_into_portfolio_capacity() {
    let definition = definition();
    assert_eq!(definition.total_offered_capacity_kw(), 110.0);
    assert_eq!(definition.largest_failure_domain_capacity_kw(), 110.0);
    assert_eq!(
        definition.largest_failure_domain_share_basis_points(),
        10_000.0
    );
}

#[test]
fn event_and_incident_group_counts_are_separate_contract_gates() {
    let contract =
        PortfolioContract::new("frequency-response-v1", 3, 2, 9_000, 9_000, 1_000).unwrap();
    let definition = PortfolioDefinition::new(
        PortfolioIdentity::new("p", "frequency-response-v1", "sha256:a", "sha256:d").unwrap(),
        contract,
        vec![
            asset("a", "sa", 100.0, &["region"]),
            asset("b", "sb", 100.0, &["region"]),
        ],
        vec![domain("region", "weather")],
    )
    .unwrap();
    let mut observation = event(
        "e1",
        "g1",
        "normal",
        0,
        100.0,
        (100.0, AssetAvailability::Available),
        (100.0, AssetAvailability::Available),
        &[("region", FailureDomainState::Normal)],
    );
    let error = evaluate_portfolio_resilience(&definition, &[observation]).unwrap_err();
    assert!(matches!(
        error,
        symtropy_flexibility_portfolio::PortfolioResilienceError::InsufficientEventCount { .. }
    ));
}

#[test]
fn input_order_does_not_change_sorted_event_or_group_receipts() {
    let states = normal_states();
    let a = event(
        "z",
        "group-z",
        "storm",
        10_000_000_000_000,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &states,
    );
    let b = event(
        "a",
        "group-a",
        "heatwave",
        0,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &states,
    );
    let first = evaluate_portfolio_resilience(&definition(), &[a.clone(), b.clone()]).unwrap();
    let second = evaluate_portfolio_resilience(&definition(), &[b, a]).unwrap();
    assert_eq!(first.events, second.events);
    assert_eq!(first.incident_groups, second.incident_groups);
    assert_eq!(first.scenarios, second.scenarios);
}

#[test]
fn duplicate_event_id_and_inconsistent_incident_scenario_are_rejected() {
    let states = normal_states();
    let a = event(
        "same",
        "g1",
        "heatwave",
        0,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &states,
    );
    let b = event(
        "same",
        "g2",
        "heatwave",
        10_000_000_000_000,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &states,
    );
    assert!(matches!(
        evaluate_portfolio_resilience(&definition(), &[a, b]).unwrap_err(),
        symtropy_flexibility_portfolio::PortfolioResilienceError::DuplicateEvent { .. }
    ));

    let a = event(
        "e1",
        "same-group",
        "heatwave",
        0,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &states,
    );
    let b = event(
        "e2",
        "same-group",
        "grid-event",
        10_000_000_000_000,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &states,
    );
    assert!(matches!(
        evaluate_portfolio_resilience(&definition(), &[a, b]).unwrap_err(),
        symtropy_flexibility_portfolio::PortfolioResilienceError::DuplicateIncidentGroupScenario { .. }
    ));
}

#[test]
fn domain_observed_state_is_descriptive_even_when_outcome_does_not_follow_it() {
    let degraded = event(
        "weather-degraded",
        "heatwave-1",
        "heatwave",
        0,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &[
            ("region", FailureDomainState::Degraded),
            ("control", FailureDomainState::Normal),
            ("telecom", FailureDomainState::Normal),
        ],
    );
    let normal = event(
        "weather-normal",
        "separate-storm",
        "storm",
        10_000_000_000_000,
        100.0,
        (60.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &normal_states(),
    );
    let report = evaluate_portfolio_resilience(&definition(), &[degraded, normal]).unwrap();
    let region = report
        .failure_domains
        .iter()
        .find(|domain| domain.domain_id == "region")
        .unwrap();
    assert_eq!(region.degraded_event_count, 1);
    assert_eq!(region.portfolio_passes_when_degraded, 1);
    assert_eq!(report.passed_event_count, 2);
}
#[test]
fn invalid_portfolio_offer_and_product_contract_mismatch_are_rejected() {
    assert!(matches!(
        PortfolioAsset::new("a", "s", "p", "pcc", 0.0, vec!["region".into()], "e").unwrap_err(),
        symtropy_flexibility_portfolio::PortfolioResilienceError::InvalidQuantity { .. }
    ));
    let identity = PortfolioIdentity::new("p", "product-a", "sha256:a", "sha256:d").unwrap();
    let contract = PortfolioContract::new("product-b", 1, 1, 9_000, 9_000, 1_000).unwrap();
    assert!(matches!(
        PortfolioDefinition::new(
            identity,
            contract,
            vec![asset("a", "sa", 1.0, &["region"])],
            vec![domain("region", "weather")],
        )
        .unwrap_err(),
        symtropy_flexibility_portfolio::PortfolioResilienceError::ProductContractMismatch
    ));
}

#[test]
fn event_receipt_retains_each_response_and_domain_evidence_reference() {
    let report = evaluate_portfolio_resilience(
        &definition(),
        &[
            event(
                "e1",
                "g1",
                "heatwave",
                0,
                100.0,
                (60.0, AssetAvailability::Available),
                (50.0, AssetAvailability::Available),
                &normal_states(),
            ),
            event(
                "e2",
                "g2",
                "storm",
                10_000_000_000_000,
                100.0,
                (60.0, AssetAvailability::Available),
                (50.0, AssetAvailability::Available),
                &normal_states(),
            ),
        ],
    )
    .unwrap();
    let receipt = &report.events[0];
    assert_eq!(receipt.asset_outcomes.len(), 2);
    assert_eq!(receipt.asset_outcomes[0].response_receipt_id, "response-a");
    assert_eq!(receipt.asset_outcomes[0].source_digest, "sha256:response-a");
    assert_eq!(receipt.failure_domain_observations.len(), 3);
    assert_eq!(
        receipt.failure_domain_observations[0].evidence_id,
        "domain-evidence-control-e1"
    );
}

#[test]
fn repeated_dispatch_events_do_not_satisfy_independent_incident_group_minimum() {
    let states = normal_states();
    let events = [
        event(
            "episode-1",
            "same-storm",
            "storm",
            0,
            100.0,
            (60.0, AssetAvailability::Available),
            (50.0, AssetAvailability::Available),
            &states,
        ),
        event(
            "episode-2",
            "same-storm",
            "storm",
            10_000_000_000_000,
            100.0,
            (60.0, AssetAvailability::Available),
            (50.0, AssetAvailability::Available),
            &states,
        ),
    ];
    let error = evaluate_portfolio_resilience(&definition(), &events).unwrap_err();
    assert!(matches!(
        error,
        symtropy_flexibility_portfolio::PortfolioResilienceError::InsufficientIncidentGroups { .. }
    ));
}

#[test]
fn duplicate_site_pcc_and_duplicate_asset_failure_memberships_are_rejected() {
    let duplicate_site = PortfolioDefinition::new(
        PortfolioIdentity::new("p", "product", "sha256:a", "sha256:d").unwrap(),
        PortfolioContract::new("product", 1, 1, 9_000, 9_000, 1_000).unwrap(),
        vec![
            asset("a1", "same-site", 50.0, &["region"]),
            asset("a2", "same-site", 60.0, &["region"]),
        ],
        vec![domain("region", "weather")],
    )
    .unwrap_err();
    assert!(matches!(
        duplicate_site,
        symtropy_flexibility_portfolio::PortfolioResilienceError::DuplicateSitePcc { .. }
    ));
    assert!(matches!(
        PortfolioAsset::new(
            "a",
            "site-a",
            "pcc-a",
            "pcc",
            50.0,
            vec!["region".into(), "region".into()],
            "asset-evidence-a",
        )
        .unwrap_err(),
        symtropy_flexibility_portfolio::PortfolioResilienceError::DuplicateAssetFailureDomain { .. }
    ));
}

#[test]
fn tiny_positive_delivery_is_rejected_for_unavailable_or_unknown_assets() {
    for availability in [AssetAvailability::Unavailable, AssetAvailability::Unknown] {
        let error = AssetDeliveryOutcome::new(
            "asset-a",
            f64::EPSILON,
            availability,
            "response-a",
            "sha256:response-a",
        )
        .unwrap_err();
        assert!(matches!(
            error,
            symtropy_flexibility_portfolio::PortfolioResilienceError::AvailabilityDeliveryMismatch { .. }
        ));
    }
}

#[test]
fn arbitrarily_small_offer_overrun_is_rejected_without_tolerance_credit() {
    let observation = event(
        "e-overrun", "g-overrun", "normal", 0, 100.0,
        (60.0 + f64::EPSILON * 16.0, AssetAvailability::Available),
        (50.0, AssetAvailability::Available),
        &normal_states(),
    );
    let error = evaluate_portfolio_resilience(&definition(), &[observation]).unwrap_err();
    assert!(matches!(
        error,
        symtropy_flexibility_portfolio::PortfolioResilienceError::DeliveryExceedsOffer { .. }
    ));
}
