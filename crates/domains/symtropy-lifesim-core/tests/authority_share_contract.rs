// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable oracle for Living World exact extensive-authority partitioning.
//!
//! This test intentionally lives independently of the future Level-A product API.
//! It freezes the integer mathematics that implementation must reproduce before
//! authority transfer is allowed to depend on it.

fn cumulative_prefix(total: u64, count: u64, prefix_len: u64) -> Option<u64> {
    if count == 0 || prefix_len > count {
        return None;
    }

    let numerator = u128::from(total) * u128::from(prefix_len);
    let value = numerator / u128::from(count);
    u64::try_from(value).ok()
}

fn slot_share(total: u64, count: u64, slot: u64) -> Option<u64> {
    if slot >= count {
        return None;
    }

    let before = cumulative_prefix(total, count, slot)?;
    let after = cumulative_prefix(total, count, slot + 1)?;
    after.checked_sub(before)
}

fn frozen_partition(total: u64, count: u64) -> Vec<u64> {
    (0..count)
        .map(|slot| slot_share(total, count, slot).expect("valid frozen partition slot"))
        .collect()
}

fn naive_remainder_repricing(mut total: u64, mut count: u64) -> Vec<u64> {
    let mut shares = Vec::new();
    while count != 0 {
        let share = total / count;
        shares.push(share);
        total -= share;
        count -= 1;
    }
    shares
}

#[test]
fn exhaustive_small_partitions_are_exact_balanced_and_prefix_proportional() {
    for count in 1u64..=96 {
        for total in 0u64..=1_024 {
            let floor_share = total / count;
            let ceil_share = floor_share + u64::from(total % count != 0);
            let mut sum = 0u64;

            for slot in 0..count {
                let share = slot_share(total, count, slot).unwrap();
                assert!(
                    share == floor_share || share == ceil_share,
                    "unexpected share: total={total}, count={count}, slot={slot}, share={share}"
                );

                sum = sum.checked_add(share).unwrap();
                assert_eq!(
                    sum,
                    cumulative_prefix(total, count, slot + 1).unwrap(),
                    "prefix mismatch: total={total}, count={count}, slot={slot}"
                );

                let proportional_numerator = u128::from(total) * u128::from(slot + 1);
                let exact_floor = proportional_numerator / u128::from(count);
                assert_eq!(u128::from(sum), exact_floor);
            }

            assert_eq!(sum, total, "partition did not telescope exactly");
        }
    }
}

#[test]
fn b2_n4_freezes_partition_epoch_counterexample() {
    let frozen = frozen_partition(2, 4);
    assert_eq!(frozen, vec![0, 1, 0, 1]);

    // Repricing each subsequent owner from the current remainder is exact in
    // aggregate, but it is a *different authority history* and therefore must
    // not replace the frozen partition-epoch semantics.
    let repriced = naive_remainder_repricing(2, 4);
    assert_eq!(repriced, vec![0, 0, 1, 1]);
    assert_ne!(repriced, frozen);
    assert_eq!(repriced.iter().sum::<u64>(), 2);
    assert_eq!(frozen.iter().sum::<u64>(), 2);
}

#[test]
fn u64_boundaries_use_widened_intermediates_without_overflow() {
    assert_eq!(slot_share(u64::MAX, 1, 0), Some(u64::MAX));
    assert_eq!(cumulative_prefix(u64::MAX, u64::MAX, u64::MAX), Some(u64::MAX));
    assert_eq!(slot_share(u64::MAX, u64::MAX, 0), Some(1));
    assert_eq!(slot_share(u64::MAX, u64::MAX, u64::MAX - 1), Some(1));

    let count = u64::MAX - 1;
    let total = u64::MAX;
    assert_eq!(cumulative_prefix(total, count, count), Some(total));
    assert!(slot_share(total, count, count - 1).is_some());
}

#[test]
fn invalid_partition_coordinates_fail_closed() {
    assert_eq!(cumulative_prefix(10, 0, 0), None);
    assert_eq!(cumulative_prefix(10, 4, 5), None);
    assert_eq!(slot_share(10, 0, 0), None);
    assert_eq!(slot_share(10, 4, 4), None);
}

#[test]
fn exact_partition_can_still_be_too_coarse_for_individual_causality() {
    let total = 3u64;
    let count = 10u64;
    let shares = frozen_partition(total, count);

    assert_eq!(shares.iter().sum::<u64>(), total);
    assert_eq!(shares.iter().filter(|share| **share == 1).count(), 3);
    assert_eq!(shares.iter().filter(|share| **share == 0).count(), 7);

    // This is mathematically exact conservation, but it demonstrates why the
    // authority unit must also be sufficiently resolved for the requested
    // Level-A granularity. A zero-unit share must not be disguised with a
    // floating-point modeled body mass.
}

#[test]
fn frozen_prefix_share_is_independent_of_future_budget_requests() {
    let total = 1_003u64;
    let count = 17u64;
    let first_five = frozen_partition(total, count)[..5].to_vec();
    let first_twelve = frozen_partition(total, count)[..12].to_vec();

    assert_eq!(&first_twelve[..5], first_five.as_slice());
    assert_eq!(
        first_five.iter().sum::<u64>(),
        cumulative_prefix(total, count, 5).unwrap()
    );
    assert_eq!(
        first_twelve.iter().sum::<u64>(),
        cumulative_prefix(total, count, 12).unwrap()
    );
}

#[test]
fn ecological_mutation_rebases_only_unowned_authority() {
    let original_total = 17u64;
    let original_count = 5u64;
    let epoch_zero = frozen_partition(original_total, original_count);
    assert_eq!(epoch_zero, vec![3, 3, 4, 3, 4]);

    // Two ordinary reservations consume the first two slots of the frozen
    // epoch. Their ownership is now historical canonical state.
    let active_a = epoch_zero[0];
    let active_b = epoch_zero[1];
    let transferred = active_a + active_b;
    let remainder_before_mutation = original_total - transferred;
    let remainder_count = original_count - 2;
    assert_eq!(remainder_before_mutation, 11);
    assert_eq!(remainder_count, 3);

    // A later ecological settlement adds biomass to the *unowned* remainder.
    // Existing active owners must not be repriced. Only the remainder becomes
    // the origin of a new partition epoch.
    let ecological_input = 5u64;
    let rebased_remainder = remainder_before_mutation + ecological_input;
    let epoch_one = frozen_partition(rebased_remainder, remainder_count);
    assert_eq!(epoch_one, vec![5, 5, 6]);

    assert_eq!(active_a, 3);
    assert_eq!(active_b, 3);
    assert_eq!(epoch_one.iter().sum::<u64>(), rebased_remainder);
    assert_eq!(
        active_a + active_b + epoch_one.iter().sum::<u64>(),
        original_total + ecological_input
    );
}

#[test]
fn reservation_without_ecological_mutation_consumes_frozen_epoch_slots() {
    let total = 2u64;
    let count = 4u64;
    let epoch = frozen_partition(total, count);

    // A growing active budget reveals a longer prefix of one immutable epoch;
    // it does not repartition the remainder after each reservation.
    assert_eq!(&epoch[..1], &[0]);
    assert_eq!(&epoch[..2], &[0, 1]);
    assert_eq!(&epoch[..3], &[0, 1, 0]);
    assert_eq!(&epoch[..4], &[0, 1, 0, 1]);

    for prefix_len in 0..=count {
        let owned = epoch[..prefix_len as usize].iter().sum::<u64>();
        assert_eq!(
            owned,
            cumulative_prefix(total, count, prefix_len).unwrap(),
            "frozen epoch prefix changed at prefix_len={prefix_len}"
        );
    }
}
