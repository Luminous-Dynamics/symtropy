// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent bounded differential/property corpus for periodic geometry.
//!
//! The oracles below intentionally use direct integer-image enumeration and
//! analytic sphere CCD calls instead of calling the production AABB-translation
//! or image-range helpers to construct expected results.
//!
//! These tests establish only the bounded scenarios and numeric envelope in
//! this file. Run on the exact PR head and preserve any failure seed.

use std::collections::{BTreeMap, BTreeSet};

use nalgebra::SVector;
use symtropy_math::{LatticeOffset, PeriodicBox};

use symtropy_physics::broadphase::Aabb;
use symtropy_physics::ccd;
use symtropy_physics::periodic_broadphase::image_offsets_for_aabb_pair;
use symtropy_physics::periodic_collision::sphere_sphere_ccd_periodic_all;

fn domain_x_periodic() -> PeriodicBox<2> {
    PeriodicBox::new(
        SVector::from([0.0, -3.0]),
        SVector::from([10.0, 0.0]),
        [true, false],
    )
    .unwrap()
}

fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn unit(state: &mut u64) -> f64 {
    let bits = lcg_next(state) >> 11;
    bits as f64 / ((1_u64 << 53) as f64)
}

fn range(state: &mut u64, low: f64, high: f64) -> f64 {
    low + (high - low) * unit(state)
}

fn aabb(min: [f64; 2], max: [f64; 2]) -> Aabb<2> {
    Aabb {
        min: SVector::from(min),
        max: SVector::from(max),
    }
}

fn overlaps_direct(a: &Aabb<2>, b: &Aabb<2>) -> bool {
    (0..2).all(|axis| a.max[axis] >= b.min[axis] && a.min[axis] <= b.max[axis])
}

fn translated_b_direct(b: &Aabb<2>, n: i64, period: f64) -> Aabb<2> {
    let shift = n as f64 * period;
    Aabb {
        min: SVector::from([b.min[0] + shift, b.min[1]]),
        max: SVector::from([b.max[0] + shift, b.max[1]]),
    }
}

fn close(a: f64, b: f64, tolerance: f64) {
    assert!(
        (a - b).abs() <= tolerance,
        "{a:.16e} != {b:.16e} within {tolerance:.3e}"
    );
}

#[test]
fn wrapping_and_lattice_offsets_match_raw_coordinate_identity() {
    let domain = PeriodicBox::new(
        SVector::from([13.25, -5.5]),
        SVector::from([2.75, 8.0]),
        [true, true],
    )
    .unwrap();

    let mut seed = 0x5045_5249_4f44_4943_u64;
    for case in 0..1_000 {
        let point = SVector::from([
            range(&mut seed, -120.0, 120.0),
            range(&mut seed, -100.0, 100.0),
        ]);
        let wrapped = domain.wrap_point(&point).unwrap();

        for axis in 0..2 {
            let lower = domain.origin()[axis];
            let upper = lower + domain.periods()[axis];
            assert!(
                wrapped.position[axis] >= lower && wrapped.position[axis] < upper,
                "case={case} axis={axis} wrapped={} interval=[{lower},{upper})",
                wrapped.position[axis]
            );
        }

        let restored = domain
            .apply_offset(&wrapped.position, wrapped.image_offset)
            .unwrap();
        for axis in 0..2 {
            close(restored[axis], point[axis], 2e-10);
        }

        let wrapped_again = domain.wrap_point(&wrapped.position).unwrap();
        assert_eq!(wrapped_again.image_offset, LatticeOffset::zero());
        for axis in 0..2 {
            close(wrapped_again.position[axis], wrapped.position[axis], 2e-12);
        }
    }
}

#[test]
fn minimum_image_matches_independent_integer_search_and_is_odd() {
    let domain = PeriodicBox::new(
        SVector::zeros(),
        SVector::from([7.0, 11.0]),
        [true, true],
    )
    .unwrap();

    let mut seed = 0x4d49_4e49_4d55_4d49_u64;
    for case in 0..2_000 {
        let displacement = SVector::from([
            range(&mut seed, -140.0, 140.0),
            range(&mut seed, -220.0, 220.0),
        ]);
        let actual = domain.minimum_image_displacement(&displacement).unwrap();
        let reverse = domain
            .minimum_image_displacement(&(-displacement))
            .unwrap();

        for axis in 0..2 {
            let period = domain.periods()[axis];
            assert!(actual[axis].abs() <= period * 0.5 + 1e-12);
            close(reverse[axis], -actual[axis], 1e-12);

            let mut best: Option<(f64, f64)> = None;
            for image in -32_i64..=32 {
                let candidate = displacement[axis] + image as f64 * period;
                let score = candidate.abs();
                if best.is_none_or(|(best_score, best_value)| {
                    score < best_score
                        || (score == best_score
                            && candidate.signum() == displacement[axis].signum()
                            && best_value.signum() != displacement[axis].signum())
                }) {
                    best = Some((score, candidate));
                }
            }
            let oracle = best.expect("finite bounded image search has candidates").1;
            close(actual[axis], oracle, 2e-12);
        }
        assert!(actual.iter().all(|value| value.is_finite()), "case={case}");
    }
}

#[test]
fn periodic_aabb_candidates_include_independent_brute_force_images() {
    let domain = domain_x_periodic();
    let mut seed = 0x4141_4242_5045_5249_u64;

    for case in 0..2_000 {
        let ax = range(&mut seed, -15.0, 15.0);
        let ay = range(&mut seed, -3.0, 3.0);
        let aw = range(&mut seed, 0.0, 2.0);
        let ah = range(&mut seed, 0.0, 2.0);
        let bx = range(&mut seed, -15.0, 15.0);
        let by = range(&mut seed, -3.0, 3.0);
        let bw = range(&mut seed, 0.0, 2.0);
        let bh = range(&mut seed, 0.0, 2.0);
        let a = aabb([ax, ay], [ax + aw, ay + ah]);
        let b = aabb([bx, by], [bx + bw, by + bh]);

        let actual: BTreeSet<[i64; 2]> = image_offsets_for_aabb_pair(&domain, &a, &b, 64)
            .unwrap_or_else(|error| panic!("case={case} unexpected broadphase error: {error}"))
            .into_iter()
            .map(|offset| offset.periods)
            .collect();

        // The fixture bounds stay within a range where +/-8 images is a
        // complete oracle envelope. Translation and overlap are recomputed
        // directly, without production helpers.
        let mut oracle = BTreeSet::new();
        for image in -8_i64..=8 {
            let shifted = translated_b_direct(&b, image, 10.0);
            if overlaps_direct(&a, &shifted) {
                oracle.insert([image, 0]);
            }
        }

        assert!(
            actual.is_superset(&oracle),
            "candidate set omitted an overlapping image at deterministic case {case}; actual={actual:?} oracle={oracle:?}"
        );
    }
}

#[test]
fn periodic_sphere_ccd_all_images_matches_direct_shift_oracle() {
    let domain = domain_x_periodic();
    let mut seed = 0x4343_4444_435f_4f52_u64;

    for case in 0..500 {
        let position_a = SVector::from([
            range(&mut seed, -12.0, 22.0),
            range(&mut seed, -2.0, 2.0),
        ]);
        let position_b = SVector::from([
            range(&mut seed, -12.0, 22.0),
            range(&mut seed, -2.0, 2.0),
        ]);
        let velocity_a = SVector::from([
            range(&mut seed, -15.0, 15.0),
            range(&mut seed, -4.0, 4.0),
        ]);
        let velocity_b = SVector::from([
            range(&mut seed, -15.0, 15.0),
            range(&mut seed, -4.0, 4.0),
        ]);
        let radius_a = range(&mut seed, 0.1, 0.7);
        let radius_b = range(&mut seed, 0.1, 0.7);
        let dt = range(&mut seed, 0.05, 0.8);

        let actual_hits = sphere_sphere_ccd_periodic_all(
            &domain,
            &position_a,
            &velocity_a,
            radius_a,
            &position_b,
            &velocity_b,
            radius_b,
            dt,
            64,
        )
        .unwrap_or_else(|error| panic!("case={case} periodic CCD error: {error}"));

        assert!(
            actual_hits.windows(2).all(|pair| {
                pair[0].toi_seconds < pair[1].toi_seconds
                    || (pair[0].toi_seconds == pair[1].toi_seconds
                        && pair[0].image_offset.periods <= pair[1].image_offset.periods)
            }),
            "CCD results are not deterministically ordered at case {case}"
        );

        let actual: BTreeMap<[i64; 2], f64> = actual_hits
            .into_iter()
            .map(|hit| (hit.image_offset.periods, hit.toi_seconds))
            .collect();

        // Independent reference: try every image in a deliberately wider
        // envelope and call only the existing analytic Euclidean sphere CCD.
        let mut oracle = BTreeMap::new();
        for image in -16_i64..=16 {
            let shifted_b = SVector::from([
                position_b[0] + image as f64 * 10.0,
                position_b[1],
            ]);
            if let Some(hit) = ccd::sphere_sphere(
                &position_a,
                &velocity_a,
                radius_a,
                &shifted_b,
                &velocity_b,
                radius_b,
                dt,
            ) {
                oracle.insert([image, 0], hit.toi);
            }
        }

        assert_eq!(
            actual.keys().copied().collect::<BTreeSet<_>>(),
            oracle.keys().copied().collect::<BTreeSet<_>>(),
            "CCD image set mismatch at deterministic case {case}"
        );
        for (image, oracle_toi) in oracle {
            close(actual[&image], oracle_toi, 1e-10);
        }
    }
}



#[test]
fn minimum_image_is_invariant_under_integer_lattice_translation() {
    let domain = PeriodicBox::new(
        SVector::from([13.25, -5.5]),
        SVector::from([2.75, 8.0]),
        [true, true],
    )
    .unwrap();

    let mut seed = 0x4c41_5454_4943_4553_u64;
    for _ in 0..1_000 {
        let displacement = SVector::from([
            range(&mut seed, -100.0, 100.0),
            range(&mut seed, -100.0, 100.0),
        ]);
        let shifts = [
            (lcg_next(&mut seed) % 81) as i64 - 40,
            (lcg_next(&mut seed) % 81) as i64 - 40,
        ];
        let translated = SVector::from([
            displacement[0] + shifts[0] as f64 * domain.periods()[0],
            displacement[1] + shifts[1] as f64 * domain.periods()[1],
        ]);

        let original_image = domain.minimum_image_displacement(&displacement).unwrap();
        let translated_image = domain.minimum_image_displacement(&translated).unwrap();
        for axis in 0..2 {
            close(
                translated_image[axis],
                original_image[axis],
                2e-10,
            );
        }

        let reverse = domain.minimum_image_displacement(&(-translated)).unwrap();
        for axis in 0..2 {
            close(reverse[axis], -translated_image[axis], 2e-10);
        }
    }
}

#[test]
fn wrapping_and_minimum_image_are_uniform_scale_covariant() {
    let origin = SVector::from([13.25, -5.5]);
    let periods = SVector::from([2.75, 8.0]);
    let domain = PeriodicBox::new(origin, periods, [true, true]).unwrap();
    let scales = [0.125_f64, 0.5, 2.0, 8.0];
    let mut seed = 0x5343_414c_455f_5045_u64;

    for scale in scales {
        let scaled_domain = PeriodicBox::new(
            origin * scale,
            periods * scale,
            [true, true],
        )
        .unwrap();

        for _ in 0..250 {
            let point = SVector::from([
                range(&mut seed, -100.0, 100.0),
                range(&mut seed, -100.0, 100.0),
            ]);
            let displacement = SVector::from([
                range(&mut seed, -100.0, 100.0),
                range(&mut seed, -100.0, 100.0),
            ]);

            let wrapped = domain.wrap_point(&point).unwrap();
            let scaled_wrapped = scaled_domain.wrap_point(&(point * scale)).unwrap();
            assert_eq!(wrapped.image_offset, scaled_wrapped.image_offset);
            for axis in 0..2 {
                close(
                    scaled_wrapped.position[axis],
                    wrapped.position[axis] * scale,
                    2e-9,
                );
            }

            let image = domain.minimum_image_displacement(&displacement).unwrap();
            let scaled_image = scaled_domain
                .minimum_image_displacement(&(displacement * scale))
                .unwrap();
            for axis in 0..2 {
                close(scaled_image[axis], image[axis] * scale, 2e-9);
            }
        }
    }
}

#[test]
fn periodic_aabb_image_sets_are_invariant_under_global_lattice_translation() {
    let domain = domain_x_periodic();
    let mut seed = 0x474c_4f42_414c_5f50_u64;

    for case in 0..500 {
        let ax = range(&mut seed, -15.0, 15.0);
        let ay = range(&mut seed, -3.0, 3.0);
        let aw = range(&mut seed, 0.0, 2.0);
        let ah = range(&mut seed, 0.0, 2.0);
        let bx = range(&mut seed, -15.0, 15.0);
        let by = range(&mut seed, -3.0, 3.0);
        let bw = range(&mut seed, 0.0, 2.0);
        let bh = range(&mut seed, 0.0, 2.0);
        let a = aabb([ax, ay], [ax + aw, ay + ah]);
        let b = aabb([bx, by], [bx + bw, by + bh]);
        let shift_cells = (lcg_next(&mut seed) % 41) as i64 - 20;
        let shift = shift_cells as f64 * 10.0;
        let shifted_a = aabb(
            [a.min[0] + shift, a.min[1]],
            [a.max[0] + shift, a.max[1]],
        );
        let shifted_b = aabb(
            [b.min[0] + shift, b.min[1]],
            [b.max[0] + shift, b.max[1]],
        );

        let original: BTreeSet<[i64; 2]> =
            image_offsets_for_aabb_pair(&domain, &a, &b, 64)
                .unwrap()
                .into_iter()
                .map(|offset| offset.periods)
                .collect();
        let translated: BTreeSet<[i64; 2]> =
            image_offsets_for_aabb_pair(&domain, &shifted_a, &shifted_b, 64)
                .unwrap()
                .into_iter()
                .map(|offset| offset.periods)
                .collect();

        assert_eq!(
            original, translated,
            "global lattice shift changed image offsets at case {case}"
        );
    }
}
