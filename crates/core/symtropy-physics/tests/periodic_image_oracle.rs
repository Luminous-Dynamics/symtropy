// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent periodic image-enumeration oracle.
//!
//! This deliberately does not call the production AABB translation helper or
//! reuse the production interval formula. It exhaustively enumerates a wider
//! fixed lattice window and checks overlap by directly translating B's bounds.
//! Small outward-rounded extras are tolerated only within a declared ULP band.

use std::collections::BTreeSet;

use nalgebra::SVector;
use symtropy_math::{LatticeOffset, PeriodicBox};
use symtropy_physics::{Aabb, image_offsets_for_aabb_pair};

const ORACLE_RADIUS: i64 = 16;
const ORACLE_MARGIN_ULPS: f64 = 32.0;

fn direct_overlap<const D: usize>(
    a: &Aabb<D>,
    b: &Aabb<D>,
    periods: &SVector<f64, D>,
    periodic_axes: &[bool; D],
    offset: &[i64; D],
    outward_tolerance: f64,
) -> bool {
    for axis in 0..D {
        let shift = if periodic_axes[axis] {
            offset[axis] as f64 * periods[axis]
        } else {
            if offset[axis] != 0 {
                return false;
            }
            0.0
        };
        let b_min = b.min[axis] + shift;
        let b_max = b.max[axis] + shift;
        if !b_min.is_finite() || !b_max.is_finite() {
            return false;
        }
        if a.max[axis] + outward_tolerance < b_min
            || a.min[axis] - outward_tolerance > b_max
        {
            return false;
        }
    }
    true
}

/// Exhaustive reference over a wider range than generated fixtures require.
/// This uses direct coordinate translation, not the production interval-bound
/// derivation or the production AABB translation function.
fn brute_force_images<const D: usize>(
    a: &Aabb<D>,
    b: &Aabb<D>,
    periods: &SVector<f64, D>,
    periodic_axes: &[bool; D],
) -> BTreeSet<[i64; D]> {
    let mut result = BTreeSet::new();
    let mut offset = [0_i64; D];

    fn recurse<const D: usize>(
        axis: usize,
        a: &Aabb<D>,
        b: &Aabb<D>,
        periods: &SVector<f64, D>,
        periodic_axes: &[bool; D],
        offset: &mut [i64; D],
        result: &mut BTreeSet<[i64; D]>,
    ) {
        if axis == D {
            if direct_overlap(a, b, periods, periodic_axes, offset, 0.0) {
                result.insert(*offset);
            }
            return;
        }

        if periodic_axes[axis] {
            for image in -ORACLE_RADIUS..=ORACLE_RADIUS {
                offset[axis] = image;
                recurse(axis + 1, a, b, periods, periodic_axes, offset, result);
            }
        } else {
            offset[axis] = 0;
            recurse(axis + 1, a, b, periods, periodic_axes, offset, result);
        }
    }

    recurse(0, a, b, periods, periodic_axes, &mut offset, &mut result);
    result
}

fn deterministic_unit(seed: &mut u64) -> f64 {
    // Self-contained deterministic test generator, not a cryptographic RNG.
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    ((*seed >> 11) as f64) / ((1_u64 << 53) as f64)
}

fn random_aabb<const D: usize>(
    seed: &mut u64,
    origin: &SVector<f64, D>,
    periods: &SVector<f64, D>,
) -> Aabb<D> {
    let min = SVector::<f64, D>::from_fn(|axis, _| {
        let centre_periods = deterministic_unit(seed) * 4.0 - 2.0;
        origin[axis] + centre_periods * periods[axis]
    });
    let max = SVector::<f64, D>::from_fn(|axis, _| {
        let width_periods = 1.8 * deterministic_unit(seed);
        min[axis] + width_periods * periods[axis]
    });
    Aabb { min, max }
}

fn candidate_set<const D: usize>(
    domain: &PeriodicBox<D>,
    a: &Aabb<D>,
    b: &Aabb<D>,
    budget: usize,
) -> BTreeSet<[i64; D]> {
    image_offsets_for_aabb_pair(domain, a, b, budget)
        .expect("generated corpus must stay inside candidate budget")
        .into_iter()
        .map(|offset| offset.periods)
        .collect()
}

fn compare_candidate_set<const D: usize>(
    case: &str,
    domain: &PeriodicBox<D>,
    periods: &SVector<f64, D>,
    periodic_axes: &[bool; D],
    a: &Aabb<D>,
    b: &Aabb<D>,
) {
    let oracle = brute_force_images(a, b, periods, periodic_axes);
    let actual = candidate_set(domain, a, b, 4096);

    // Outward rounding may add an ULP-near-boundary candidate, but no exact
    // brute-force image may be missing.
    for missing in oracle.difference(&actual) {
        panic!("{case}: missing brute-force image {missing:?}");
    }

    for extra in actual.difference(&oracle) {
        let mut scale = 1.0_f64;
        for axis in 0..D {
            scale = scale
                .max(a.min[axis].abs())
                .max(a.max[axis].abs())
                .max(b.min[axis].abs())
                .max(b.max[axis].abs())
                .max(periods[axis].abs())
                .max((extra[axis] as f64 * periods[axis]).abs());
        }
        let tolerance = ORACLE_MARGIN_ULPS * f64::EPSILON * scale;
        assert!(
            direct_overlap(a, b, periods, periodic_axes, extra, tolerance),
            "{case}: non-overlapping image incorrectly emitted: {extra:?}"
        );
    }

    let repeated = candidate_set(domain, a, b, 4096);
    assert_eq!(actual, repeated, "{case}: enumeration is not deterministic");
}

fn domain2(mask: [bool; 2]) -> PeriodicBox<2> {
    PeriodicBox::new(
        SVector::from([100.0, -40.0]),
        SVector::from([10.0, 6.0]),
        mask,
    )
    .unwrap()
}

#[test]
fn exact_seam_and_corner_touching_images_are_enumerated() {
    let periods = SVector::from([10.0, 6.0]);
    let mask = [true, true];
    let domain = domain2(mask);

    let a = Aabb {
        min: SVector::from([100.0, -40.0]),
        max: SVector::from([100.5, -39.5]),
    };
    let b = Aabb {
        min: SVector::from([109.5, -34.5]),
        max: SVector::from([110.0, -34.0]),
    };

    compare_candidate_set("exact-corner-touch", &domain, &periods, &mask, &a, &b);
    let actual = candidate_set(&domain, &a, &b, 64);
    assert!(actual.contains(&[-1, -1]));
}

#[test]
fn mixed_periodic_axes_never_translate_nonperiodic_coordinates() {
    let periods = SVector::from([10.0, 6.0]);
    for mask in [[true, false], [false, true], [false, false]] {
        let domain = domain2(mask);
        let a = Aabb {
            min: SVector::from([99.5, -40.5]),
            max: SVector::from([100.5, -39.5]),
        };
        let b = Aabb {
            min: SVector::from([109.5, -40.5]),
            max: SVector::from([110.5, -39.5]),
        };
        compare_candidate_set("mixed-axis", &domain, &periods, &mask, &a, &b);

        for offset in candidate_set(&domain, &a, &b, 64) {
            for axis in 0..2 {
                if !mask[axis] {
                    assert_eq!(offset[axis], 0);
                }
            }
        }
    }
}

#[test]
fn randomized_aabb_image_sets_match_exhaustive_covering_space_oracle() {
    let origin = SVector::from([100.0, -40.0]);
    let periods = SVector::from([10.0, 6.0]);
    let masks = [[true, true], [true, false], [false, true], [false, false]];
    let mut seed = 0x7a31_9c4d_2f85_b601_u64;

    for mask in masks {
        let domain = domain2(mask);
        for fixture in 0..128 {
            let a = random_aabb(&mut seed, &origin, &periods);
            let b = random_aabb(&mut seed, &origin, &periods);
            let label = format!("mask={mask:?}, fixture={fixture}");
            compare_candidate_set(&label, &domain, &periods, &mask, &a, &b);
        }
    }
}

#[test]
fn explicit_lattice_offsets_restore_points_without_using_production_translation() {
    let domain = domain2([true, true]);
    let periods = SVector::from([10.0, 6.0]);
    let original = SVector::from([341.25, -124.75]);
    let wrapped = domain.wrap_point(&original).unwrap();

    for axis in 0..2 {
        let restored =
            wrapped.position[axis] + wrapped.image_offset.periods[axis] as f64 * periods[axis];
        assert!(
            (restored - original[axis]).abs()
                <= 8.0 * f64::EPSILON * original[axis].abs().max(1.0)
        );
        assert!(wrapped.position[axis] >= domain.origin()[axis]);
        assert!(wrapped.position[axis] < domain.origin()[axis] + periods[axis]);
    }

    let wrapped_again = domain.wrap_point(&wrapped.position).unwrap();
    assert_eq!(wrapped_again.image_offset, LatticeOffset::zero());
    assert_eq!(wrapped_again.position, wrapped.position);
}
