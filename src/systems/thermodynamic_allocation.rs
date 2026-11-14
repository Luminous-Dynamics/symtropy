// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Pure, deterministic allocation of a finite regeneration budget.
//!
//! Each recipient supplies its current energy headroom. The allocator applies
//! a per-recipient request cap and computes a deterministic max-min/equal-share
//! allocation. Recipients with less remaining headroom saturate first; leftover
//! supply is then shared among the remaining recipients.
//!
//! This module does not inspect ECS state, mutate energy stores, or decide
//! spatial eligibility. Callers must first identify valid recipients and
//! compute actual headroom, then debit a source only by the returned accepted
//! allocations. For multiple overlapping wells, the caller must still define
//! whether supplies are allocated independently or jointly.

/// Eligible recipient and its current accepted-energy headroom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecipientDemand {
    pub id: u64,
    pub headroom: f64,
}

/// Allocated amount for one stable recipient ID.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecipientAllocation {
    pub id: u64,
    pub amount: f64,
}

/// Result of allocating one finite source budget.
#[derive(Clone, Debug, PartialEq)]
pub struct AllocationResult {
    /// Contains one entry per input recipient, sorted by stable ID (including
    /// zero allocations).
    pub allocations: Vec<RecipientAllocation>,
    /// Sum of accepted allocations using stable-ID accumulation order.
    pub total_allocated: f64,
    /// Supply that could not be accepted because demand/headroom was too small.
    pub unallocated_supply: f64,
}

/// Rejected input to the regeneration allocator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AllocationError {
    InvalidAvailableSupply,
    InvalidPerRecipientLimit,
    InvalidHeadroom { id: u64 },
    DuplicateRecipientId { id: u64 },
}

impl core::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidAvailableSupply => {
                write!(f, "available supply must be finite and non-negative")
            }
            Self::InvalidPerRecipientLimit => {
                write!(f, "per-recipient limit must be finite and non-negative")
            }
            Self::InvalidHeadroom { id } => {
                write!(f, "recipient {id} headroom must be finite and non-negative")
            }
            Self::DuplicateRecipientId { id } => {
                write!(f, "duplicate allocation recipient ID {id}")
            }
        }
    }
}

impl std::error::Error for AllocationError {}

/// Allocate a finite supply with a deterministic max-min/equal-share policy.
///
/// For each recipient, its cap is the minimum of headroom and per-recipient
/// limit. Total accepted allocation cannot exceed available supply. If supply
/// is scarce, recipients receive equal shares until a recipient reaches its
/// cap; that recipient saturates and the remaining supply is reallocated.
///
/// Results are canonicalized by recipient ID before calculations, so input
/// permutation does not affect any recipient's result. Floating-point residuals
/// are resolved in stable-ID order; tiny last-bit differences are not a policy
/// priority and should not be interpreted as a fairness signal.
pub fn allocate_max_min(
    recipients: impl IntoIterator<Item = RecipientDemand>,
    available_supply: f64,
    per_recipient_limit: f64,
) -> Result<AllocationResult, AllocationError> {
    if !available_supply.is_finite() || available_supply < 0.0 {
        return Err(AllocationError::InvalidAvailableSupply);
    }
    if !per_recipient_limit.is_finite() || per_recipient_limit < 0.0 {
        return Err(AllocationError::InvalidPerRecipientLimit);
    }

    let mut recipients: Vec<_> = recipients.into_iter().collect();
    recipients.sort_unstable_by_key(|recipient| recipient.id);
    for recipient in &recipients {
        if !recipient.headroom.is_finite() || recipient.headroom < 0.0 {
            return Err(AllocationError::InvalidHeadroom { id: recipient.id });
        }
    }
    for pair in recipients.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(AllocationError::DuplicateRecipientId { id: pair[0].id });
        }
    }

    let caps: Vec<f64> = recipients
        .iter()
        .map(|recipient| recipient.headroom.min(per_recipient_limit))
        .collect();
    let mut target = 0.0_f64;
    for cap in &caps {
        let remaining_to_target = (available_supply - target).max(0.0);
        if remaining_to_target <= 0.0 {
            break;
        }
        if *cap >= remaining_to_target {
            target = available_supply;
            break;
        }
        target = (target + *cap).min(available_supply);
    }

    let mut amounts = vec![0.0_f64; recipients.len()];
    let mut active: Vec<usize> = caps
        .iter()
        .enumerate()
        .filter_map(|(index, cap)| (*cap > 0.0).then_some(index))
        .collect();
    let mut remaining_target = target;

    while !active.is_empty() && remaining_target > 0.0 {
        let equal_share = remaining_target / active.len() as f64;
        if equal_share <= 0.0 || !equal_share.is_finite() {
            break;
        }

        let mut saturated_any = false;
        for &index in &active {
            if caps[index] <= equal_share {
                let amount = caps[index].min(remaining_target);
                amounts[index] = amount;
                remaining_target = (remaining_target - amount).max(0.0);
                saturated_any = true;
            }
        }

        if saturated_any {
            active.retain(|&index| amounts[index] < caps[index]);
            continue;
        }

        // No remaining recipient would saturate at this equal share. Apply the
        // common share in stable ID order; the last recipient receives only the
        // residual that remains, preventing aggregate overspend from rounding.
        for &index in active.iter().take(active.len().saturating_sub(1)) {
            let amount = equal_share.min(caps[index]).min(remaining_target);
            amounts[index] = amount;
            remaining_target = (remaining_target - amount).max(0.0);
        }
        if let Some(&last) = active.last() {
            amounts[last] = caps[last].min(remaining_target);
        }
        break;
    }

    // Guard the budget invariant against cumulative floating-point rounding.
    let mut total_allocated = 0.0_f64;
    let mut allocations = Vec::with_capacity(recipients.len());
    for (recipient, amount) in recipients.iter().zip(amounts) {
        let mut room = (available_supply - total_allocated).max(0.0);
        // If accumulated allocation is less than half the supply, subtraction
        // may round the remaining room back up to the original supply. Keep one
        // representable step of slack so a tiny previous allocation cannot be
        // silently lost when adding a near-full-supply final allocation.
        if total_allocated > 0.0 && total_allocated < available_supply * 0.5 {
            room = next_down_nonnegative(room);
        }
        let accepted = amount.min(caps_for(recipient, per_recipient_limit)).min(room);
        total_allocated += accepted;
        allocations.push(RecipientAllocation {
            id: recipient.id,
            amount: accepted,
        });
    }

    Ok(AllocationResult {
        allocations,
        total_allocated,
        unallocated_supply: (available_supply - total_allocated).max(0.0),
    })
}

fn caps_for(recipient: &RecipientDemand, per_recipient_limit: f64) -> f64 {
    recipient.headroom.min(per_recipient_limit)
}

fn next_down_nonnegative(value: f64) -> f64 {
    if value <= 0.0 || !value.is_finite() {
        value.max(0.0)
    } else {
        f64::from_bits(value.to_bits() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demands() -> Vec<RecipientDemand> {
        vec![
            RecipientDemand {
                id: 30,
                headroom: 40.0,
            },
            RecipientDemand {
                id: 10,
                headroom: 100.0,
            },
            RecipientDemand {
                id: 20,
                headroom: 10.0,
            },
            RecipientDemand {
                id: 40,
                headroom: 0.0,
            },
        ]
    }

    fn total(result: &AllocationResult) -> f64 {
        result
            .allocations
            .iter()
            .map(|allocation| allocation.amount)
            .sum()
    }

    #[test]
    fn output_is_identical_under_input_permutation() {
        let input = demands();
        let first = allocate_max_min(input.clone(), 35.0, 20.0).unwrap();
        let second = allocate_max_min(input.into_iter().rev(), 35.0, 20.0).unwrap();
        assert_eq!(first, second);
        assert!(
            first
                .allocations
                .windows(2)
                .all(|window| window[0].id < window[1].id)
        );
    }

    #[test]
    fn constrained_recipient_saturates_and_remainder_is_redistributed() {
        let result = allocate_max_min(
            [
                RecipientDemand {
                    id: 1,
                    headroom: 100.0,
                },
                RecipientDemand {
                    id: 2,
                    headroom: 10.0,
                },
                RecipientDemand {
                    id: 3,
                    headroom: 100.0,
                },
            ],
            35.0,
            20.0,
        )
        .unwrap();

        let amounts: Vec<_> = result
            .allocations
            .iter()
            .map(|allocation| allocation.amount)
            .collect();
        assert_eq!(amounts, vec![12.5, 10.0, 12.5]);
        assert!((result.total_allocated - 35.0).abs() < 1e-12);
        assert_eq!(result.unallocated_supply, 0.0);
    }

    #[test]
    fn headroom_and_per_recipient_limit_bound_every_allocation() {
        let mut recipients = demands();
        recipients.sort_unstable_by_key(|recipient| recipient.id);
        let result = allocate_max_min(recipients.clone(), 10_000.0, 7.0).unwrap();
        for (recipient, allocation) in recipients.iter().zip(&result.allocations) {
            assert!(allocation.amount >= 0.0);
            assert!(allocation.amount <= recipient.headroom);
            assert!(allocation.amount <= 7.0);
            assert_eq!(allocation.id, recipient.id);
        }
        assert!((result.total_allocated - total(&result)).abs() < 1e-12);
        assert!((result.total_allocated + result.unallocated_supply - 10_000.0).abs() < 1e-9);
    }

    #[test]
    fn no_recipient_or_no_demand_leaves_supply_unallocated() {
        let empty = allocate_max_min([], 12.0, 3.0).unwrap();
        assert!(empty.allocations.is_empty());
        assert_eq!(empty.total_allocated, 0.0);
        assert_eq!(empty.unallocated_supply, 12.0);

        let zero_headroom = allocate_max_min(
            [RecipientDemand {
                id: 1,
                headroom: 0.0,
            }],
            12.0,
            3.0,
        )
        .unwrap();
        assert_eq!(zero_headroom.allocations[0].amount, 0.0);
        assert_eq!(zero_headroom.unallocated_supply, 12.0);
    }

    #[test]
    fn tiny_prior_allocation_does_not_get_lost_at_large_supply_scale() {
        let result = allocate_max_min(
            [
                RecipientDemand {
                    id: 1,
                    headroom: 1.0e-16,
                },
                RecipientDemand {
                    id: 2,
                    headroom: 10.0,
                },
            ],
            1.0,
            10.0,
        )
        .unwrap();
        let sum = total(&result);
        assert!(sum <= 1.0);
        assert!(result.total_allocated <= 1.0);
        assert!(result.unallocated_supply >= 0.0);
        assert!((result.total_allocated + result.unallocated_supply - 1.0).abs() < 1e-12);
    }

    #[test]
    fn one_recipient_receives_only_accepted_headroom() {
        let result = allocate_max_min(
            [RecipientDemand {
                id: 5,
                headroom: 0.25,
            }],
            10.0,
            4.0,
        )
        .unwrap();
        assert_eq!(result.allocations[0].amount, 0.25);
        assert_eq!(result.total_allocated, 0.25);
        assert_eq!(result.unallocated_supply, 9.75);
    }

    #[test]
    fn seeded_randomized_cases_preserve_order_caps_and_supply() {
        let mut seed = 0x7a31_89c5_d4e2_106f_u64;
        for trial in 0..64 {
            let count = (next_random(&mut seed) % 129) as usize;
            let mut recipients = Vec::with_capacity(count);
            for id in 0..count {
                let raw = next_random(&mut seed);
                let headroom = ((raw >> 16) % 100_001) as f64 / 100.0;
                recipients.push(RecipientDemand {
                    id: id as u64,
                    headroom,
                });
            }
            let supply = ((next_random(&mut seed) >> 16) % 200_001) as f64 / 100.0;
            let limit = ((next_random(&mut seed) >> 16) % 25_001) as f64 / 100.0;

            let forward = allocate_max_min(recipients.clone(), supply, limit).unwrap();
            let reverse =
                allocate_max_min(recipients.iter().rev().copied(), supply, limit).unwrap();
            assert_eq!(forward, reverse, "input-order mismatch in trial {trial}");
            assert!(forward.total_allocated <= supply);
            assert!(forward.unallocated_supply >= 0.0);

            let mut returned_total = 0.0;
            for (recipient, allocation) in recipients.iter().zip(&forward.allocations) {
                let cap = recipient.headroom.min(limit);
                assert!(allocation.amount.is_finite());
                assert!(allocation.amount >= 0.0);
                assert!(allocation.amount <= cap);
                returned_total += allocation.amount;
            }
            let tolerance = 1.0e-9 * supply.max(1.0);
            assert!((returned_total - forward.total_allocated).abs() <= tolerance);
            assert!(
                (forward.total_allocated + forward.unallocated_supply - supply).abs() <= tolerance
            );
        }
    }

    /// Adversarial property sweep across the IEEE-754 exponent range.
    ///
    /// This checks hard safety invariants and exact ID-order determinism, not
    /// equivalence to real-number max-min fairness. Subnormal budgets may not
    /// be divisible into equal non-zero f64 shares; leaving such a residual
    /// unallocated is conservative and is measured separately from overspend.
    #[test]
    fn wide_exponent_inputs_preserve_safety_and_permutation_invariance() {
        let mut seed = 0xd1b5_4a32_d192_ed03_u64;
        let smallest_subnormal = f64::from_bits(1);
        let edge_values = [
            0.0,
            smallest_subnormal,
            f64::MIN_POSITIVE,
            f64::EPSILON,
            1.0e-300,
            1.0e-16,
            1.0,
            1.0e16,
            1.0e300,
            f64::MAX,
        ];

        // Pair boundary values with unrelated exponent scales to exercise
        // cancellation, saturation, and representability transitions.
        for (case, &supply) in edge_values.iter().enumerate() {
            let limit = edge_values[(case * 7 + 3) % edge_values.len()];
            let recipients: Vec<_> = (0..edge_values.len())
                .map(|index| RecipientDemand {
                    id: index as u64 + 1,
                    headroom: edge_values[(index + case) % edge_values.len()],
                })
                .collect();
            assert_allocation_safety(&recipients, supply, limit);
        }

        // Deterministic random finite non-negative values, with exponents
        // sampled across the representable range rather than only [0, 1e3].
        for trial in 0..4096 {
            let count = (next_random(&mut seed) % 65) as usize;
            let supply = next_finite_nonnegative(&mut seed);
            let limit = next_finite_nonnegative(&mut seed);
            let recipients: Vec<_> = (0..count)
                .map(|id| RecipientDemand {
                    id: id as u64,
                    headroom: next_finite_nonnegative(&mut seed),
                })
                .collect();

            let result = assert_allocation_safety(&recipients, supply, limit);
            assert_eq!(
                result,
                allocate_max_min(recipients.iter().rev().copied(), supply, limit).unwrap(),
                "wide-exponent permutation mismatch in trial {trial}"
            );
        }
    }

    fn assert_allocation_safety(
        recipients: &[RecipientDemand],
        supply: f64,
        limit: f64,
    ) -> AllocationResult {
        let result = allocate_max_min(recipients.iter().copied(), supply, limit).unwrap();
        assert!(result.total_allocated.is_finite());
        assert!(result.unallocated_supply.is_finite());
        assert!(result.total_allocated >= 0.0);
        assert!(result.total_allocated <= supply);
        assert!(result.unallocated_supply >= 0.0);
        assert!(result.unallocated_supply <= supply);

        // The result itself reports allocations in stable-ID order. Recompute
        // the total in that same order so accounting uses one canonical sum.
        let mut recomputed_total = 0.0_f64;
        for allocation in &result.allocations {
            recomputed_total += allocation.amount;
        }
        assert_eq!(result.total_allocated, recomputed_total);
        let accounted = result.total_allocated + result.unallocated_supply;
        assert!(accounted.is_finite());
        assert!(accounted <= supply, "accounting exceeded source supply");

        let mut sorted = recipients.to_vec();
        sorted.sort_unstable_by_key(|recipient| recipient.id);
        assert_eq!(result.allocations.len(), sorted.len());
        assert!(
            result
                .allocations
                .windows(2)
                .all(|pair| pair[0].id < pair[1].id)
        );
        for (recipient, allocation) in sorted.iter().zip(&result.allocations) {
            let cap = recipient.headroom.min(limit);
            assert!(allocation.amount.is_finite());
            assert!(allocation.amount >= 0.0);
            assert!(allocation.amount <= cap);
            assert_eq!(allocation.id, recipient.id);
        }
        result
    }

    /// Differentially compare the f64 implementation against an independent
    /// exact-rational water-filling oracle on quarter-unit inputs. Inputs are
    /// small enough that their source values and caps are exactly representable
    /// in binary64; only the equal-share division itself may round.
    #[test]
    fn seeded_quarter_unit_cases_match_exact_rational_waterfill_oracle() {
        let mut seed = 0x6a09_e667_f3bc_c909_u64;
        for trial in 0..4096 {
            let count = (next_random(&mut seed) % 65) as usize;
            let recipients: Vec<_> = (0..count)
                .map(|id| RecipientDemand {
                    id: id as u64,
                    headroom: (next_random(&mut seed) % 16_001) as f64 / 4.0,
                })
                .collect();
            let supply = (next_random(&mut seed) % 16_001) as f64 / 4.0;
            let limit = (next_random(&mut seed) % 16_001) as f64 / 4.0;

            let actual = allocate_max_min(recipients.iter().copied(), supply, limit).unwrap();
            let expected = exact_quarter_unit_waterfill_oracle(&recipients, supply, limit);
            assert_eq!(actual.allocations.len(), expected.len());

            let tolerance = 1.0e-11 * supply.max(1.0);
            for (got, want) in actual.allocations.iter().zip(&expected) {
                assert_eq!(got.id, want.0, "ID mismatch in trial {trial}");
                assert!(
                    (got.amount - want.1).abs() <= tolerance,
                    "oracle mismatch in trial {trial}, id {}: got {}, expected {}, tolerance {}",
                    got.id,
                    got.amount,
                    want.1,
                    tolerance
                );
            }
            assert!(actual.total_allocated <= supply);
        }
    }

    #[test]
    fn rational_waterline_oracle_covers_explicit_boundary_cases() {
        let cases: Vec<(&str, Vec<RecipientDemand>, f64, f64, Vec<(u64, f64)>)> = vec![
            ("empty demand", vec![], 3.0, 5.0, vec![]),
            (
                "zero supply",
                vec![
                    RecipientDemand {
                        id: 1,
                        headroom: 2.0,
                    },
                    RecipientDemand {
                        id: 2,
                        headroom: 7.0,
                    },
                ],
                0.0,
                10.0,
                vec![(1, 0.0), (2, 0.0)],
            ),
            (
                "zero caps",
                vec![
                    RecipientDemand {
                        id: 1,
                        headroom: 0.0,
                    },
                    RecipientDemand {
                        id: 2,
                        headroom: 0.0,
                    },
                ],
                5.0,
                10.0,
                vec![(1, 0.0), (2, 0.0)],
            ),
            (
                "supply above total capacity",
                vec![
                    RecipientDemand {
                        id: 1,
                        headroom: 1.0,
                    },
                    RecipientDemand {
                        id: 2,
                        headroom: 2.0,
                    },
                    RecipientDemand {
                        id: 3,
                        headroom: 3.0,
                    },
                ],
                20.0,
                10.0,
                vec![(1, 1.0), (2, 2.0), (3, 3.0)],
            ),
            (
                "repeated caps",
                vec![
                    RecipientDemand {
                        id: 1,
                        headroom: 2.0,
                    },
                    RecipientDemand {
                        id: 2,
                        headroom: 2.0,
                    },
                    RecipientDemand {
                        id: 3,
                        headroom: 2.0,
                    },
                    RecipientDemand {
                        id: 4,
                        headroom: 2.0,
                    },
                ],
                5.0,
                10.0,
                vec![(1, 1.25), (2, 1.25), (3, 1.25), (4, 1.25)],
            ),
            (
                "exact saturation tie",
                vec![
                    RecipientDemand {
                        id: 1,
                        headroom: 2.0,
                    },
                    RecipientDemand {
                        id: 2,
                        headroom: 2.0,
                    },
                    RecipientDemand {
                        id: 3,
                        headroom: 10.0,
                    },
                ],
                6.0,
                10.0,
                vec![(1, 2.0), (2, 2.0), (3, 2.0)],
            ),
            (
                "constrained then equal share",
                vec![
                    RecipientDemand {
                        id: 1,
                        headroom: 1.0,
                    },
                    RecipientDemand {
                        id: 2,
                        headroom: 4.0,
                    },
                    RecipientDemand {
                        id: 3,
                        headroom: 10.0,
                    },
                ],
                5.0,
                10.0,
                vec![(1, 1.0), (2, 2.0), (3, 2.0)],
            ),
            (
                "zero per-recipient limit",
                vec![
                    RecipientDemand {
                        id: 1,
                        headroom: 1.0,
                    },
                    RecipientDemand {
                        id: 2,
                        headroom: 2.0,
                    },
                ],
                3.0,
                0.0,
                vec![(1, 0.0), (2, 0.0)],
            ),
            (
                "second saturation threshold",
                vec![
                    RecipientDemand {
                        id: 1,
                        headroom: 2.0,
                    },
                    RecipientDemand {
                        id: 2,
                        headroom: 5.0,
                    },
                    RecipientDemand {
                        id: 3,
                        headroom: 8.0,
                    },
                ],
                7.0,
                10.0,
                vec![(1, 2.0), (2, 2.5), (3, 2.5)],
            ),
        ];

        for (name, recipients, supply, limit, expected) in cases {
            let actual = allocate_max_min(recipients.iter().copied(), supply, limit).unwrap();
            let oracle = exact_quarter_unit_waterfill_oracle(&recipients, supply, limit);
            assert_eq!(oracle, expected, "reference oracle fixture mismatch: {name}");
            assert_eq!(actual.allocations.len(), expected.len(), "length: {name}");
            let tolerance = 1.0e-12 * supply.max(1.0);
            for ((got, wanted), expected_item) in
                actual.allocations.iter().zip(&oracle).zip(&expected)
            {
                assert_eq!(got.id, wanted.0, "oracle id: {name}");
                assert_eq!(got.id, expected_item.0, "expected id: {name}");
                assert!(
                    (got.amount - wanted.1).abs() <= tolerance,
                    "allocation mismatch in {name}: got {}, expected {}, tolerance {}",
                    got.amount,
                    wanted.1,
                    tolerance
                );
            }
        }
    }

    /// Independent reference calculation using exact integer quarter-units and
    /// a rational waterline numerator/denominator. It does not reuse the
    /// production allocator's saturation/reallocation loop.
    fn exact_quarter_unit_waterfill_oracle(
        recipients: &[RecipientDemand],
        supply: f64,
        limit: f64,
    ) -> Vec<(u64, f64)> {
        fn quarter_units(value: f64) -> i128 {
            (value * 4.0).round() as i128
        }

        let caps: Vec<(u64, i128)> = recipients
            .iter()
            .map(|recipient| {
                (
                    recipient.id,
                    quarter_units(recipient.headroom).min(quarter_units(limit)),
                )
            })
            .collect();
        let total_capacity: i128 = caps.iter().map(|(_, cap)| *cap).sum();
        let target = quarter_units(supply).min(total_capacity);

        if target == total_capacity {
            return caps
                .iter()
                .map(|(id, cap)| (*id, *cap as f64 / 4.0))
                .collect();
        }

        let mut by_cap = caps.clone();
        by_cap.sort_unstable_by_key(|(id, cap)| (*cap, *id));
        let mut saturated = 0_i128;
        let mut active = by_cap.len() as i128;
        let mut waterline = None;
        for (_, cap) in &by_cap {
            let remaining = target - saturated;
            if remaining <= *cap * active {
                waterline = Some((remaining, active));
                break;
            }
            saturated += *cap;
            active -= 1;
        }
        let (waterline_num, waterline_den) =
            waterline.expect("target below total capacity has a waterline");

        caps.iter()
            .map(|(id, cap)| {
                let allocation_num = (*cap * waterline_den).min(waterline_num);
                (*id, (allocation_num as f64 / waterline_den as f64) / 4.0)
            })
            .collect()
    }

    /// Manual benchmark. Run with
    /// cargo test allocator_scaling_benchmark -- --ignored --nocapture
    ///
    /// This intentionally records timings instead of asserting a machine-
    /// dependent threshold. Compare only runs with compatible build/toolchain
    /// and hardware manifests.
    #[test]
    #[ignore = "manual scalability benchmark; timing is environment-dependent"]
    fn allocator_scaling_benchmark() {
        use std::time::Instant;

        fn percentile_ns(sorted: &[u128], percentile: usize) -> u128 {
            assert!(!sorted.is_empty());
            let rank = (sorted.len() * percentile).div_ceil(100).max(1);
            sorted[rank.min(sorted.len()) - 1]
        }

        for count in [0_usize, 1, 2, 10, 1_000, 10_000, 100_000] {
            let recipients: Vec<_> = (0..count)
                .map(|id| RecipientDemand {
                    id: id as u64,
                    headroom: ((id.wrapping_mul(7_919) % 901) as f64) / 100.0,
                })
                .collect();
            let supply = count as f64 * 2.5;
            let limit = 5.0;
            let samples = if count >= 100_000 {
                15
            } else if count >= 10_000 {
                20
            } else {
                50
            };

            for _ in 0..2 {
                let warmup = allocate_max_min(recipients.iter().copied(), supply, limit)
                    .expect("generated benchmark input is valid");
                assert!(warmup.total_allocated <= supply);
            }

            let mut durations_ns = Vec::with_capacity(samples);
            let mut final_result = None;
            for _ in 0..samples {
                let start = Instant::now();
                let result = allocate_max_min(recipients.iter().copied(), supply, limit)
                    .expect("generated benchmark input is valid");
                durations_ns.push(start.elapsed().as_nanos());
                assert!(result.total_allocated <= supply);
                final_result = Some(result);
            }
            durations_ns.sort_unstable();
            let result = final_result.expect("at least one benchmark sample");
            println!(
                "allocator_bench recipients={count} samples={samples} p50_ns={} p95_ns={} p99_ns={} total_allocated={:.17e} unallocated_supply={:.17e}",
                percentile_ns(&durations_ns, 50),
                percentile_ns(&durations_ns, 95),
                percentile_ns(&durations_ns, 99),
                result.total_allocated,
                result.unallocated_supply,
            );
        }
    }

    fn next_finite_nonnegative(seed: &mut u64) -> f64 {
        let raw = next_random(seed);
        const FRACTION_MASK: u64 = (1_u64 << 52) - 1;
        if raw & 0b111 == 0 {
            // Include zero and subnormals with deliberate frequency.
            return f64::from_bits(raw & FRACTION_MASK);
        }
        // Exponent 0..=2046 excludes NaN and infinity. The sign bit is zero.
        let exponent = (raw >> 53).min(2046) << 52;
        f64::from_bits(exponent | (raw & FRACTION_MASK))
    }

    fn next_random(seed: &mut u64) -> u64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *seed
    }

    #[test]
    fn malformed_supply_limit_headroom_and_duplicate_ids_fail_closed() {
        assert_eq!(
            allocate_max_min(demands(), f64::NAN, 1.0),
            Err(AllocationError::InvalidAvailableSupply)
        );
        assert_eq!(
            allocate_max_min(demands(), 1.0, f64::INFINITY),
            Err(AllocationError::InvalidPerRecipientLimit)
        );
        assert_eq!(
            allocate_max_min(
                [RecipientDemand {
                    id: 7,
                    headroom: f64::NAN,
                }],
                1.0,
                1.0
            ),
            Err(AllocationError::InvalidHeadroom { id: 7 })
        );
        assert_eq!(
            allocate_max_min(
                [
                    RecipientDemand {
                        id: 7,
                        headroom: 1.0,
                    },
                    RecipientDemand {
                        id: 7,
                        headroom: 2.0,
                    },
                ],
                1.0,
                1.0
            ),
            Err(AllocationError::DuplicateRecipientId { id: 7 })
        );
    }
}
