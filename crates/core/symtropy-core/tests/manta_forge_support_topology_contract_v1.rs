include!("manta_forge_policy_surface_topology_adversary_v1.rs");

const TOPOLOGY_FIXTURE: &str =
    include_str!("../fixtures/manta-forge-support-topology-adversary-v1.txt");

fn topology_fixture_scalar(key: &str) -> u64 {
    TOPOLOGY_FIXTURE
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing topology fixture scalar {key}"))
        .parse()
        .unwrap()
}

fn topology_fixture_points(prefix: &str) -> Vec<DynamicPolicySurfacePoint> {
    TOPOLOGY_FIXTURE
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
                let (key, raw) = field.split_once(':').unwrap();
                let value: u64 = raw.parse().unwrap();
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
                    other => panic!("unknown topology fixture field {other}"),
                }
            }
            point
        })
        .collect()
}

#[test]
fn shared_topology_fixture_reexecutes_both_h4_support_graphs() {
    assert_eq!(topology_fixture_scalar("period_duration_ms"), 1);
    assert_eq!(topology_fixture_scalar("source_horizon_periods"), 4);
    assert_eq!(topology_fixture_scalar("forge_tooling_inventory_units"), 4);
    assert_eq!(topology_fixture_scalar("controller_support_inventory_units"), 4);

    let case = h4_case();
    let expected_direct = topology_fixture_points("direct_policy=");
    let expected_hidden = topology_fixture_points("hidden_policy=");
    assert_eq!(expected_direct.len(), 4);
    assert_eq!(expected_hidden.len(), 4);

    let observed_direct: Vec<_> = expected_direct
        .iter()
        .map(|point| {
            simulate_topology_surface_point(
                &case,
                point.maturity_periods,
                ControllerSupportTopology::DirectReactor,
            )
        })
        .collect();
    let observed_hidden: Vec<_> = expected_hidden
        .iter()
        .map(|point| {
            simulate_topology_surface_point(
                &case,
                point.maturity_periods,
                ControllerSupportTopology::FiniteHiddenSupport,
            )
        })
        .collect();

    assert_eq!(observed_direct, expected_direct);
    assert_eq!(observed_hidden, expected_hidden);
    assert_ne!(observed_direct, observed_hidden);
}
