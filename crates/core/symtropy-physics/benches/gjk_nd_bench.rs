// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nalgebra::{SMatrix, SVector};
use symtropy_physics::gjk::intersects_support_maps;
use symtropy_physics::support_map::WorldSupportMap;

#[derive(Clone)]
struct OrientedBox<const D: usize> {
    rotation: SMatrix<f64, D, D>,
    center: SVector<f64, D>,
    half_extents: SVector<f64, D>,
}

impl<const D: usize> WorldSupportMap<D> for OrientedBox<D> {
    fn support_world(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        let local_direction = self.rotation.transpose() * direction;
        let local_support = SVector::from_fn(|axis, _| {
            if local_direction[axis] >= 0.0 {
                self.half_extents[axis]
            } else {
                -self.half_extents[axis]
            }
        });
        self.rotation * local_support + self.center
    }

    fn center_world(&self) -> SVector<f64, D> {
        self.center
    }

    fn bounding_sphere_world(&self) -> (SVector<f64, D>, f64) {
        (self.center, self.half_extents.norm())
    }
}

fn next_random(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*seed >> 11) as f64) / ((1u64 << 53) as f64)
}

fn make_box<const D: usize>(seed: &mut u64, x_offset: f64) -> OrientedBox<D> {
    let mut rotation = SMatrix::<f64, D, D>::identity();
    for i in 0..D {
        for j in (i + 1)..D {
            let angle = (2.0 * next_random(seed) - 1.0) * std::f64::consts::PI;
            let (sin, cos) = angle.sin_cos();
            let mut plane = SMatrix::<f64, D, D>::identity();
            plane[(i, i)] = cos;
            plane[(j, j)] = cos;
            plane[(i, j)] = -sin;
            plane[(j, i)] = sin;
            rotation *= plane;
        }
    }
    OrientedBox {
        rotation,
        center: SVector::from_fn(|axis, _| if axis == 0 { x_offset } else { 0.0 }),
        half_extents: SVector::from_fn(|_, _| 0.4 + 0.8 * next_random(seed)),
    }
}

fn bench_dimension<const D: usize>(criterion: &mut Criterion) {
    let mut seed = 0x4e44_474a_4b5f_4245_u64 ^ (D as u64);
    let a = make_box::<D>(&mut seed, 0.0);
    // Each oriented box contains a ball of radius at least 0.4. A 0.25 centre
    // offset is an overlap positive control; an offset of 10 is separated.
    let overlap = make_box::<D>(&mut seed, 0.25);
    let separated = make_box::<D>(&mut seed, 10.0);
    let mut group = criterion.benchmark_group(format!("gjk_nd/dimension_{D}"));

    group.bench_function("rotated_overlap", |bench| {
        bench.iter(|| {
            black_box(intersects_support_maps(
                black_box(&a),
                black_box(&overlap),
            ))
        })
    });
    group.bench_function("rotated_separated", |bench| {
        bench.iter(|| {
            black_box(intersects_support_maps(
                black_box(&a),
                black_box(&separated),
            ))
        })
    });
    group.finish();
}

fn bench_gjk_nd(criterion: &mut Criterion) {
    // D=3/4 serve as specialized-path controls; D>=5 exercises the new active set.
    bench_dimension::<3>(criterion);
    bench_dimension::<4>(criterion);
    bench_dimension::<5>(criterion);
    bench_dimension::<6>(criterion);
    bench_dimension::<8>(criterion);
}

criterion_group!(benches, bench_gjk_nd);
criterion_main!(benches);
