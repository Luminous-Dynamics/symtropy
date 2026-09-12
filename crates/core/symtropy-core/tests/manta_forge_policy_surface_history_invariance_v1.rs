include!("manta_forge_policy_sensitivity_surface_v1.rs");

use std::collections::BTreeMap;

#[test]
fn equal_successor_runway_has_equal_policy_surface_across_controlled_histories() {
    let cases = cases();
    let depth = depth_cases();
    assert_eq!(cases.len(), 42);
    assert_eq!(depth.len(), 42);

    let maturities = [1u64, 2, 3, 4];
    let mut representative_surface: BTreeMap<u64, Vec<DynamicPolicySurfacePoint>> =
        BTreeMap::new();
    let mut class_counts: BTreeMap<u64, usize> = BTreeMap::new();
    let mut nonreproductive_cases = 0usize;

    for (case, expected) in cases.iter().zip(&depth) {
        assert_eq!(case.recovery_tick, expected.recovery_tick);
        assert_eq!(case.tooling_stock, expected.tooling_stock);

        let Some(horizon) = expected.expected_successor_horizon else {
            assert!(first_successor(case).is_none());
            nonreproductive_cases += 1;
            continue;
        };

        let observed_surface: Vec<_> = maturities
            .iter()
            .map(|maturity| simulate_fixed_case_policy_surface_point(case, *maturity))
            .collect();

        *class_counts.entry(horizon).or_default() += 1;
        if let Some(representative) = representative_surface.get(&horizon) {
            assert_eq!(
                &observed_surface, representative,
                "hidden history changed policy surface for horizon={horizon}, case={case:?}"
            );
        } else {
            representative_surface.insert(horizon, observed_surface);
        }
    }

    assert_eq!(nonreproductive_cases, 24);
    assert_eq!(class_counts.get(&1), Some(&6));
    assert_eq!(class_counts.get(&2), Some(&5));
    assert_eq!(class_counts.get(&3), Some(&4));
    assert_eq!(class_counts.get(&4), Some(&3));
    assert_eq!(representative_surface.len(), 4);

    // The exact four-period class must also reproduce the already-shared surface
    // fixture, anchoring the equivalence-class theorem to the cross-repo contract.
    let expected_h4 = expected_surface_points();
    assert_eq!(representative_surface.get(&4).unwrap(), &expected_h4);
}

#[test]
fn controlled_history_invariance_does_not_collapse_distinct_runway_classes() {
    let cases = cases();
    let depth = depth_cases();
    let maturities = [1u64, 2, 3, 4];
    let mut representative_surface: BTreeMap<u64, Vec<DynamicPolicySurfacePoint>> =
        BTreeMap::new();

    for (case, expected) in cases.iter().zip(&depth) {
        let Some(horizon) = expected.expected_successor_horizon else {
            continue;
        };
        representative_surface.entry(horizon).or_insert_with(|| {
            maturities
                .iter()
                .map(|maturity| simulate_fixed_case_policy_surface_point(case, *maturity))
                .collect()
        });
    }

    let fingerprints: Vec<_> = representative_surface.values().collect();
    for left in 0..fingerprints.len() {
        for right in (left + 1)..fingerprints.len() {
            assert_ne!(fingerprints[left], fingerprints[right]);
        }
    }
}
