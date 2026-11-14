// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable oracle for deterministic same-tick scarcity arbitration.
//!
//! This intentionally freezes arithmetic/invariance properties independently
//! of a future product API. Ecological policy decides which intents compete in
//! one arbitration domain and supplies a canonical rotation; execution/thread
//! order must never become that policy accidentally.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Claim {
    key: u128,
    demand: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArbitrationError {
    ZeroDemand { key: u128 },
    DuplicateClaimant { key: u128 },
    TotalDemandOverflow,
    ClaimCountOverflow,
}

fn arbitrate(
    available: u64,
    claims: &[Claim],
    canonical_rotation: u64,
) -> Result<(BTreeMap<u128, u64>, u64), ArbitrationError> {
    if claims.is_empty() {
        return Ok((BTreeMap::new(), available));
    }

    let mut ordered = claims.to_vec();
    ordered.sort_by_key(|claim| claim.key);

    let mut previous_key = None;
    let mut total_demand = 0u64;
    for claim in &ordered {
        if claim.demand == 0 {
            return Err(ArbitrationError::ZeroDemand { key: claim.key });
        }
        if previous_key == Some(claim.key) {
            return Err(ArbitrationError::DuplicateClaimant { key: claim.key });
        }
        previous_key = Some(claim.key);
        total_demand = total_demand
            .checked_add(claim.demand)
            .ok_or(ArbitrationError::TotalDemandOverflow)?;
    }

    if total_demand <= available {
        let grants = ordered
            .into_iter()
            .map(|claim| (claim.key, claim.demand))
            .collect();
        return Ok((grants, available - total_demand));
    }

    let claim_count = u64::try_from(ordered.len()).map_err(|_| ArbitrationError::ClaimCountOverflow)?;
    let rotation = usize::try_from(canonical_rotation % claim_count)
        .map_err(|_| ArbitrationError::ClaimCountOverflow)?;
    ordered.rotate_left(rotation);

    let mut grants = BTreeMap::new();
    let mut prefix_demand = 0u64;
    let mut previous_prefix_grant = 0u64;

    for claim in ordered {
        prefix_demand = prefix_demand
            .checked_add(claim.demand)
            .ok_or(ArbitrationError::TotalDemandOverflow)?;

        let numerator = u128::from(available) * u128::from(prefix_demand);
        let prefix_grant = u64::try_from(numerator / u128::from(total_demand))
            .expect("proportional prefix grant cannot exceed available u64 quantity");
        let grant = prefix_grant
            .checked_sub(previous_prefix_grant)
            .expect("proportional cumulative grant is monotonic");

        grants.insert(claim.key, grant);
        previous_prefix_grant = prefix_grant;
    }

    debug_assert_eq!(previous_prefix_grant, available);
    Ok((grants, 0))
}

#[test]
fn exhaustive_small_scarcity_is_exact_bounded_and_within_one_unit_of_ideal() {
    for a in 1u64..=5 {
        for b in 1u64..=5 {
            for c in 1u64..=5 {
                for d in 1u64..=5 {
                    let claims = [
                        Claim { key: 10, demand: a },
                        Claim { key: 20, demand: b },
                        Claim { key: 30, demand: c },
                        Claim { key: 40, demand: d },
                    ];
                    let total = a + b + c + d;

                    for available in 0..=total {
                        for rotation in 0..claims.len() as u64 {
                            let (grants, leftover) =
                                arbitrate(available, &claims, rotation).unwrap();

                            assert_eq!(
                                grants.values().copied().sum::<u64>() + leftover,
                                available,
                                "conservation failure: demands={a},{b},{c},{d} available={available} rotation={rotation}"
                            );

                            if available < total {
                                assert_eq!(leftover, 0);
                                for claim in claims {
                                    let grant = grants[&claim.key];
                                    assert!(grant <= claim.demand);

                                    // |grant - available*demand/total| < 1 exact unit,
                                    // expressed without floating point.
                                    let granted_scaled =
                                        u128::from(grant) * u128::from(total);
                                    let ideal_scaled =
                                        u128::from(available) * u128::from(claim.demand);
                                    assert!(
                                        granted_scaled.abs_diff(ideal_scaled) < u128::from(total),
                                        "rounding error >= 1 unit: key={} demand={} grant={} available={} total={} rotation={}",
                                        claim.key,
                                        claim.demand,
                                        grant,
                                        available,
                                        total,
                                        rotation
                                    );
                                }
                            } else {
                                assert_eq!(leftover, available - total);
                                for claim in claims {
                                    assert_eq!(grants[&claim.key], claim.demand);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn input_iteration_order_cannot_change_same_tick_grants() {
    let canonical = [
        Claim { key: 3, demand: 9 },
        Claim { key: 1, demand: 2 },
        Claim { key: 7, demand: 11 },
        Claim { key: 4, demand: 5 },
    ];
    let reversed = [canonical[3], canonical[2], canonical[1], canonical[0]];
    let shuffled = [canonical[1], canonical[3], canonical[0], canonical[2]];

    let expected = arbitrate(13, &canonical, 2).unwrap();
    assert_eq!(arbitrate(13, &reversed, 2).unwrap(), expected);
    assert_eq!(arbitrate(13, &shuffled, 2).unwrap(), expected);

    assert_eq!(expected.0[&1], 1);
    assert_eq!(expected.0[&3], 5);
    assert_eq!(expected.0[&4], 2);
    assert_eq!(expected.0[&7], 5);
    assert_eq!(expected.1, 0);
}

#[test]
fn canonical_rotation_changes_rounding_recipient_without_changing_conservation() {
    let claims = [
        Claim { key: 1, demand: 1 },
        Claim { key: 2, demand: 1 },
        Claim { key: 3, demand: 1 },
    ];

    let r0 = arbitrate(1, &claims, 0).unwrap().0;
    let r1 = arbitrate(1, &claims, 1).unwrap().0;
    let r2 = arbitrate(1, &claims, 2).unwrap().0;

    assert_eq!(r0.values().sum::<u64>(), 1);
    assert_eq!(r1.values().sum::<u64>(), 1);
    assert_eq!(r2.values().sum::<u64>(), 1);
    assert_ne!(r0, r1);
    assert_ne!(r1, r2);

    // Rotation is an explicit policy input. Runtime system order is not.
    assert_eq!(r0[&3], 1);
    assert_eq!(r1[&1], 1);
    assert_eq!(r2[&2], 1);
}

#[test]
fn surplus_resource_grants_every_claim_in_full_and_retains_remainder() {
    let claims = [
        Claim { key: 10, demand: 3 },
        Claim { key: 20, demand: 5 },
    ];
    let (grants, leftover) = arbitrate(20, &claims, 999).unwrap();

    assert_eq!(grants[&10], 3);
    assert_eq!(grants[&20], 5);
    assert_eq!(leftover, 12);
}

#[test]
fn empty_intent_set_leaves_source_untouched() {
    assert_eq!(arbitrate(91, &[], 0).unwrap(), (BTreeMap::new(), 91));
}

#[test]
fn duplicate_and_zero_demand_intents_fail_closed() {
    assert_eq!(
        arbitrate(
            10,
            &[
                Claim { key: 4, demand: 2 },
                Claim { key: 4, demand: 3 },
            ],
            0,
        ),
        Err(ArbitrationError::DuplicateClaimant { key: 4 })
    );

    assert_eq!(
        arbitrate(10, &[Claim { key: 8, demand: 0 }], 0),
        Err(ArbitrationError::ZeroDemand { key: 8 })
    );
}

#[test]
fn total_demand_overflow_fails_closed() {
    assert_eq!(
        arbitrate(
            u64::MAX,
            &[
                Claim {
                    key: 1,
                    demand: u64::MAX,
                },
                Claim { key: 2, demand: 1 },
            ],
            0,
        ),
        Err(ArbitrationError::TotalDemandOverflow)
    );
}

#[test]
fn u64_boundary_arbitration_uses_widened_products() {
    let claims = [
        Claim {
            key: 1,
            demand: u64::MAX - 1,
        },
        Claim { key: 2, demand: 1 },
    ];
    let available = u64::MAX - 1;
    let (grants, leftover) = arbitrate(available, &claims, 0).unwrap();

    assert_eq!(leftover, 0);
    assert_eq!(grants[&1], u64::MAX - 2);
    assert_eq!(grants[&2], 1);
    assert_eq!(grants.values().copied().sum::<u64>(), available);
}
