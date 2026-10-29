// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nalgebra::{SMatrix, SVector};
use std::cell::Cell;
use std::collections::BTreeMap;
use symtropy_physics::gjk::{intersects_support_maps, GjkTermination};
use symtropy_physics::support_map::WorldSupportMap;

const CORPUS_SIZE: usize = 64;

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

struct CountingSupportMap<'a, const D: usize> {
    inner: &'a dyn WorldSupportMap<D>,
    support_calls: Cell<usize>,
}

impl<'a, const D: usize> CountingSupportMap<'a, D> {
    fn new(inner: &'a dyn WorldSupportMap<D>) -> Self {
        Self {
            inner,
            support_calls: Cell::new(0),
        }
    }
}

impl<'a, const D: usize> WorldSupportMap<D> for CountingSupportMap<'a, D> {
    fn support_world(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        self.support_calls.set(self.support_calls.get() + 1);
        self.inner.support_world(direction)
    }

    fn center_world(&self) -> SVector<f64, D> {
        self.inner.center_world()
    }

    fn bounding_sphere_world(&self) -> (SVector<f64, D>, f64) {
        self.inner.bounding_sphere_world()
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

fn make_corpus<const D: usize>(
    seed: &mut u64,
    x_offset: f64,
) -> Vec<(OrientedBox<D>, OrientedBox<D>)> {
    (0..CORPUS_SIZE)
        .map(|_| (make_box::<D>(seed, 0.0), make_box::<D>(seed, x_offset)))
        .collect()
}

fn quantile(values: &mut [usize], quantile: f64) -> usize {
    values.sort_unstable();
    let index = ((quantile * values.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(values.len().saturating_sub(1));
    values[index]
}

/// Emit deterministic work counters outside timed Criterion loops. Wrappers only
/// count support-map calls in this preflight; timed queries use the original maps.
fn print_work_summary<const D: usize>(
    label: &str,
    corpus: &[(OrientedBox<D>, OrientedBox<D>)],
) {
    let mut support_evaluations = Vec::with_capacity(corpus.len());
    let mut iterations = Vec::with_capacity(corpus.len());
    let mut active_set_major_steps = Vec::with_capacity(corpus.len());
    let mut active_set_minor_steps = Vec::with_capacity(corpus.len());
    let mut affine_qr_solves = Vec::with_capacity(corpus.len());
    let mut exhaustive_fallbacks_total = 0usize;
    let mut exhaustive_faces_examined_total = 0usize;
    let mut work_counter_mismatches = 0usize;
    let mut terminal_counts = BTreeMap::<String, usize>::new();
    let mut indeterminate_reasons = BTreeMap::<String, usize>::new();

    for (a, b) in corpus {
        let counted_a = CountingSupportMap::new(a);
        let counted_b = CountingSupportMap::new(b);
        let result = intersects_support_maps(&counted_a, &counted_b);
        let observed_support_calls = counted_a.support_calls.get() + counted_b.support_calls.get();
        support_evaluations.push(observed_support_calls);
        iterations.push(result.iterations);
        active_set_major_steps.push(result.work.active_set_major_steps);
        active_set_minor_steps.push(result.work.active_set_minor_steps);
        affine_qr_solves.push(result.work.affine_qr_solves);
        exhaustive_fallbacks_total += result.work.exhaustive_fallbacks;
        exhaustive_faces_examined_total += result.work.exhaustive_faces_examined;
        if (D == 1 || D >= 5) && result.work.support_map_evaluations != observed_support_calls {
            work_counter_mismatches += 1;
        }

        let terminal = match result.termination {
            GjkTermination::Intersecting => "Intersecting".to_owned(),
            GjkTermination::Separated => "Separated".to_owned(),
            GjkTermination::Indeterminate(reason) => {
                *indeterminate_reasons.entry(format!("{reason:?}")).or_default() += 1;
                format!("Indeterminate({reason:?})")
            }
        };
        *terminal_counts.entry(terminal).or_default() += 1;
    }

    assert_eq!(
        work_counter_mismatches,
        0,
        "D={D} {label}: result-side support counter disagrees with the independent wrapper"
    );

    let support_p50 = quantile(&mut support_evaluations, 0.50);
    let support_p95 = quantile(&mut support_evaluations, 0.95);
    let support_p99 = quantile(&mut support_evaluations, 0.99);
    let iterations_p50 = quantile(&mut iterations, 0.50);
    let iterations_p95 = quantile(&mut iterations, 0.95);
    let iterations_p99 = quantile(&mut iterations, 0.99);
    let major_p50 = quantile(&mut active_set_major_steps, 0.50);
    let major_p95 = quantile(&mut active_set_major_steps, 0.95);
    let major_p99 = quantile(&mut active_set_major_steps, 0.99);
    let minor_p50 = quantile(&mut active_set_minor_steps, 0.50);
    let minor_p95 = quantile(&mut active_set_minor_steps, 0.95);
    let minor_p99 = quantile(&mut active_set_minor_steps, 0.99);
    let qr_p50 = quantile(&mut affine_qr_solves, 0.50);
    let qr_p95 = quantile(&mut affine_qr_solves, 0.95);
    let qr_p99 = quantile(&mut affine_qr_solves, 0.99);

    let terminal_fields = terminal_counts
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join(";");
    let reason_fields = indeterminate_reasons
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join(";");

    // Versioned, whitespace-free key/value records are easy to archive and
    // compare in CI without scraping Rust Debug formatting.
    eprintln!(
        "GJK_WORK,v2,dimension={D},scenario={label},corpus={},support_map_evaluations_p50={support_p50},support_map_evaluations_p95={support_p95},support_map_evaluations_p99={support_p99},iterations_p50={iterations_p50},iterations_p95={iterations_p95},iterations_p99={iterations_p99},active_set_major_steps_p50={major_p50},active_set_major_steps_p95={major_p95},active_set_major_steps_p99={major_p99},active_set_minor_steps_p50={minor_p50},active_set_minor_steps_p95={minor_p95},active_set_minor_steps_p99={minor_p99},affine_qr_solves_p50={qr_p50},affine_qr_solves_p95={qr_p95},affine_qr_solves_p99={qr_p99},exhaustive_fallbacks_total={exhaustive_fallbacks_total},exhaustive_faces_examined_total={exhaustive_faces_examined_total},support_counter_mismatches={work_counter_mismatches},terminal_counts={terminal_fields},indeterminate_reasons={reason_fields}",
        corpus.len()
    );
}

fn bench_dimension<const D: usize>(criterion: &mut Criterion) {
    let mut seed = 0x4e44_474a_4b5f_4245_u64 ^ (D as u64);
    let overlap_corpus = make_corpus::<D>(&mut seed, 0.25);
    let separated_corpus = make_corpus::<D>(&mut seed, 10.0);

    print_work_summary("rotated_overlap", &overlap_corpus);
    print_work_summary("rotated_separated", &separated_corpus);

    let mut group = criterion.benchmark_group(format!("gjk_nd/dimension_{D}"));
    let mut overlap_cursor = 0usize;
    group.bench_function("rotated_overlap_64", |bench| {
        bench.iter(|| {
            let (a, b) = &overlap_corpus[overlap_cursor % overlap_corpus.len()];
            overlap_cursor += 1;
            black_box(intersects_support_maps(black_box(a), black_box(b)))
        })
    });

    let mut separated_cursor = 0usize;
    group.bench_function("rotated_separated_64", |bench| {
        bench.iter(|| {
            let (a, b) = &separated_corpus[separated_cursor % separated_corpus.len()];
            separated_cursor += 1;
            black_box(intersects_support_maps(black_box(a), black_box(b)))
        })
    });
    group.finish();
}

fn bench_gjk_nd(criterion: &mut Criterion) {
    // D=3/4 are specialized-path controls; D>=5 exercises the active set.
    bench_dimension::<3>(criterion);
    bench_dimension::<4>(criterion);
    bench_dimension::<5>(criterion);
    bench_dimension::<6>(criterion);
    bench_dimension::<8>(criterion);
    bench_dimension::<16>(criterion);

}

criterion_group!(benches, bench_gjk_nd);
criterion_main!(benches);
