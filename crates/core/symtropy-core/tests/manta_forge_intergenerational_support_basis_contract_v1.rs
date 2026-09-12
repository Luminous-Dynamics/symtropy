include!("manta_forge_policy_surface_support_basis_adversary_v1.rs");

const SUPPORT_BASIS_FIXTURE: &str =
    include_str!("../fixtures/manta-forge-intergenerational-support-basis-v1.txt");

fn fixture_scalar(key: &str) -> u64 {
    SUPPORT_BASIS_FIXTURE
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing support-basis fixture scalar {key}"))
        .parse()
        .unwrap()
}

fn fixture_bool(key: &str) -> bool {
    SUPPORT_BASIS_FIXTURE
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing support-basis fixture bool {key}"))
        .parse()
        .unwrap()
}

fn fixture_surface(prefix: &str) -> Vec<DynamicPolicySurfacePoint> {
    SUPPORT_BASIS_FIXTURE
        .lines()
        .filter_map(|line| line.strip_prefix(prefix))
        .map(|line| {
            let mut point = DynamicPolicySurfacePoint {
                maturity_periods: 0,
                founded_descendants: 0,
                maturity_completed_descendants: 0,
                descendant_reproduction_transitions: 0,
                terminal_residual_periods: 0,
            };
            for field in line.split('|') {
                let (key, value) = field.split_once(':').unwrap();
                let value: u64 = value.parse().unwrap();
                match key {
                    "maturity_periods" => point.maturity_periods = value,
                    "founded_descendants" => point.founded_descendants = value,
                    "maturity_completed_descendants" => {
                        point.maturity_completed_descendants = value
                    }
                    "descendant_reproduction_transitions" => {
                        point.descendant_reproduction_transitions = value
                    }
                    "terminal_residual_periods" => point.terminal_residual_periods = value,
                    other => panic!("unknown support-basis fixture field {other}"),
                }
            }
            point
        })
        .collect()
}

#[test]
fn shared_support_basis_fixture_reproduces_dynamic_counterexample_and_repair() {
    let case = h4_case();
    let source_period = fixture_scalar("source_period_duration_ms");
    let source_demand = fixture_scalar("source_tooling_demand_units_per_period");
    let source_inventory = fixture_scalar("source_tooling_inventory_units");
    let reactor_inventory = fixture_scalar("nonlimiting_reactor_inventory_units");
    assert_eq!(source_period, 1);

    let source_horizon = straight_horizon(
        IndustrialEpochState::from_spec(basis_v3_spec(
            &case,
            source_demand,
            source_inventory,
            1,
            reactor_inventory,
        ))
        .unwrap(),
    );
    assert_eq!(source_horizon, fixture_scalar("source_horizon_periods"));

    let changed_successor_demand =
        fixture_scalar("changed_successor_tooling_demand_units_per_period");
    let preserved_successor_demand =
        fixture_scalar("preserved_successor_tooling_demand_units_per_period");
    assert!(!fixture_bool("changed_basis_scalar_projection_safe"));
    assert!(fixture_bool("preserved_basis_scalar_projection_safe"));

    let changed: Vec<_> = [1u64, 2, 3, 4]
        .into_iter()
        .map(|maturity| {
            simulate_basis_surface_point(
                &case,
                maturity,
                source_demand,
                source_inventory,
                1,
                reactor_inventory,
                changed_successor_demand,
                1,
            )
        })
        .collect();
    let preserved: Vec<_> = [1u64, 2, 3, 4]
        .into_iter()
        .map(|maturity| {
            simulate_basis_surface_point(
                &case,
                maturity,
                source_demand,
                source_inventory,
                1,
                reactor_inventory,
                preserved_successor_demand,
                1,
            )
        })
        .collect();

    assert_eq!(changed, fixture_surface("changed_basis_policy="));
    assert_eq!(preserved, fixture_surface("preserved_basis_policy="));
    assert_ne!(changed, preserved);
}
