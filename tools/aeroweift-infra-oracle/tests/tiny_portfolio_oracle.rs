use aeroweift_infra_oracle::{
    failure_set, minimal_failure_sets, portfolio_has_complete_chain,
    site_complete_chain_available, site_landing_available, FailureDomain, OracleError,
    ALL_FAILURE_DOMAINS, PORTFOLIO_DIVERSE_AC, PORTFOLIO_NORTH_AB,
    PORTFOLIO_UNKNOWN_ONLY, SITE_NORTH_A,
};

const GOLDEN_FIXTURE: &str = include_str!(
    "../../../fixtures/aeroweift-infrastructure-resilience/tiny-portfolio-v0.json"
);

#[test]
fn frozen_fixture_identity_and_expected_cutset_counts_are_bound() {
    assert!(GOLDEN_FIXTURE.contains("AW-IR-TINY-001"));
    assert!(GOLDEN_FIXTURE.contains("nominal-coverage-a-plus-b"));
    assert!(GOLDEN_FIXTURE.contains("recovery-diverse-a-plus-c"));
    assert!(GOLDEN_FIXTURE.contains("\"nominal_site_count\": 2"));
    assert!(GOLDEN_FIXTURE.contains("\"nominal_coverage_a_plus_b\": false"));
}

#[test]
fn nominally_distinct_north_sites_have_three_singleton_cut_sets() {
    let cut_sets = minimal_failure_sets(&PORTFOLIO_NORTH_AB, &ALL_FAILURE_DOMAINS)
        .expect("fixture domain universe must be valid");
    assert_eq!(cut_sets.len(), 3);
    assert_eq!(
        cut_sets,
        vec![
            vec![FailureDomain::WxCoastal],
            vec![FailureDomain::PowerNorth],
            vec![FailureDomain::BridgeNorth],
        ]
    );
}

#[test]
fn diverse_sites_require_two_declared_failure_domains() {
    let cut_sets = minimal_failure_sets(&PORTFOLIO_DIVERSE_AC, &ALL_FAILURE_DOMAINS)
        .expect("fixture domain universe must be valid");
    assert_eq!(cut_sets.len(), 9);
    assert!(cut_sets.iter().all(|set| set.len() == 2));

    for domain in ALL_FAILURE_DOMAINS {
        assert!(portfolio_has_complete_chain(
            &PORTFOLIO_DIVERSE_AC,
            &failure_set(&[domain])
        ));
    }
}

#[test]
fn compound_coastal_weather_and_south_power_outage_removes_both_diverse_chains() {
    let compound = failure_set(&[
        FailureDomain::WxCoastal,
        FailureDomain::PowerSouth,
    ]);
    assert!(!portfolio_has_complete_chain(&PORTFOLIO_DIVERSE_AC, &compound));
}

#[test]
fn power_failure_removes_repeat_service_but_not_the_landing_surface() {
    let north_power = failure_set(&[FailureDomain::PowerNorth]);
    assert!(site_landing_available(&SITE_NORTH_A, &north_power));
    assert!(!site_complete_chain_available(&SITE_NORTH_A, &north_power));
}

#[test]
fn unknown_status_geometry_never_becomes_a_recovery_resource() {
    assert!(!portfolio_has_complete_chain(
        &PORTFOLIO_UNKNOWN_ONLY,
        &BTreeSet::new()
    ));
}

#[test]
fn adding_failures_never_restores_a_complete_chain() {
    let empty = BTreeSet::new();
    let mut checked = Vec::new();

    for mask in 0usize..(1usize << ALL_FAILURE_DOMAINS.len()) {
        let domains: Vec<_> = ALL_FAILURE_DOMAINS
            .iter()
            .enumerate()
            .filter_map(|(index, domain)| {
                ((mask & (1usize << index)) != 0).then_some(*domain)
            })
            .collect();
        let failed = failure_set(&domains);
        let service = portfolio_has_complete_chain(&PORTFOLIO_DIVERSE_AC, &failed);
        checked.push((failed, service));
    }

    for (failed, service) in &checked {
        if !service {
            continue;
        }
        for domain in ALL_FAILURE_DOMAINS {
            let mut expanded = failed.clone();
            expanded.insert(domain);
            if let Some((_, expanded_service)) =
                checked.iter().find(|(candidate, _)| candidate == &expanded)
            {
                assert!(*expanded_service);
            }
        }
    }

    assert!(portfolio_has_complete_chain(&PORTFOLIO_DIVERSE_AC, &empty));
}

#[test]
fn invalid_failure_domain_universes_are_rejected() {
    assert_eq!(
        minimal_failure_sets(
            &PORTFOLIO_NORTH_AB,
            &[FailureDomain::WxCoastal, FailureDomain::WxCoastal]
        ),
        Err(OracleError::DuplicateFailureDomain)
    );
}
