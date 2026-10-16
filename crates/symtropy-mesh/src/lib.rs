// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Triangle mesh collider and BVH-based narrowphase.

pub mod meshlet_physics;
pub mod narrowphase;

use nalgebra::SVector;
use std::any::Any;
use symtropy_math::{Point, Shape, Transform};
use symtropy_physics::broadphase::Aabb;

pub use narrowphase::{generate_mesh_contacts, generate_mesh_contacts_transformed};
pub use narrowphase::{generate_meshlet_contacts, generate_meshlet_contacts_transformed};

/// A single triangle in 3D.
#[derive(Clone, Debug)]
pub struct Triangle {
    pub vertices: [SVector<f64, 3>; 3],
}

impl Shape<3> for Triangle {
    fn support(&self, direction: &SVector<f64, 3>) -> SVector<f64, 3> {
        let mut best_dist = f64::MIN;
        let mut best_v = self.vertices[0];
        for v in &self.vertices {
            let dist = v.dot(direction);
            if dist > best_dist {
                best_dist = dist;
                best_v = *v;
            }
        }
        best_v
    }

    fn bounding_sphere(&self) -> (Point<3>, f64) {
        let center_v = (self.vertices[0] + self.vertices[1] + self.vertices[2]) / 3.0;
        let mut max_r2 = 0.0;
        for v in &self.vertices {
            let r2 = (v - center_v).norm_squared();
            if r2 > max_r2 {
                max_r2 = r2;
            }
        }
        (
            Point::new([center_v[0], center_v[1], center_v[2]]),
            max_r2.sqrt(),
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Shape<3>> {
        Box::new(self.clone())
    }
}

/// Node in a Bounding Volume Hierarchy for meshes.
#[derive(Clone, Debug)]
pub struct MeshBvhNode {
    pub aabb: Aabb<3>,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub triangle_idx: Option<usize>,
}

/// Deterministic operation counters for one BVH overlap query.
///
/// Counters are intended for diagnostics and benchmark attribution; use
/// TriangleMesh::query_overlap for the uninstrumented hot path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshBvhQueryStats {
    /// Number of node AABBs tested, including misses.
    pub bounds_tested: usize,
    /// Intersecting internal nodes whose children were visited.
    pub overlapping_internal_nodes: usize,
    /// Intersecting leaves, each corresponding to a returned triangle candidate.
    pub overlapping_leaves: usize,
    /// Number of returned triangle indices.
    pub candidate_count: usize,
}

/// Strategy used to construct a triangle-mesh BVH.
///
/// `Median` preserves the default deterministic longest-axis median split.
/// `BinnedSah` selects a split using a binned surface-area cost estimate,
/// with median fallback when a useful non-empty partition cannot be formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshBvhBuildStrategy {
    /// Deterministic longest-axis median split.
    Median,
    /// Binned surface-area heuristic with the requested number of bins.
    /// The implementation clamps this to 2..=64.
    BinnedSah { bin_count: usize },
}

/// Concave triangle mesh.
#[derive(Clone)]
pub struct TriangleMesh {
    pub triangles: Vec<Triangle>,
    pub bvh_nodes: Vec<MeshBvhNode>,
    pub root_idx: Option<usize>,
    pub center: Point<3>,
    pub radius: f64,
}

impl TriangleMesh {
    /// Build using the existing deterministic longest-axis median strategy.
    pub fn new(triangles: Vec<Triangle>) -> Self {
        Self::with_bvh_strategy(triangles, MeshBvhBuildStrategy::Median)
    }

    /// Build using an explicitly selected BVH strategy. Query and collision
    /// semantics are unchanged by the construction strategy.
    pub fn with_bvh_strategy(
        triangles: Vec<Triangle>,
        strategy: MeshBvhBuildStrategy,
    ) -> Self {
        let strategy = match strategy {
            MeshBvhBuildStrategy::Median => MeshBvhBuildStrategy::Median,
            MeshBvhBuildStrategy::BinnedSah { bin_count } => {
                MeshBvhBuildStrategy::BinnedSah {
                    bin_count: bin_count.clamp(2, 64),
                }
            }
        };
        let mut world_min = SVector::<f64, 3>::from_element(f64::MAX);
        let mut world_max = SVector::<f64, 3>::from_element(f64::MIN);

        for tri in &triangles {
            for v in &tri.vertices {
                for i in 0..3 {
                    if v[i] < world_min[i] {
                        world_min[i] = v[i];
                    }
                    if v[i] > world_max[i] {
                        world_max[i] = v[i];
                    }
                }
            }
        }

        let center_v = (world_min + world_max) * 0.5;
        let center = Point::new([center_v[0], center_v[1], center_v[2]]);
        let mut max_r2 = 0.0;
        for tri in &triangles {
            for v in &tri.vertices {
                let r2 = (v - center_v).norm_squared();
                if r2 > max_r2 {
                    max_r2 = r2;
                }
            }
        }

        // Compute tight triangle bounds and centroids once. The old recursive
        // builder recomputed bounding spheres at every tree level, which both
        // enlarged leaf/query bounds and repeated square-root work O(n log n).
        let triangle_aabbs: Vec<Aabb<3>> =
            triangles.iter().map(Self::triangle_aabb).collect();
        let triangle_centers: Vec<SVector<f64, 3>> = triangles
            .iter()
            .map(|tri| (tri.vertices[0] + tri.vertices[1] + tri.vertices[2]) / 3.0)
            .collect();

        let mut bvh_nodes = Vec::new();
        let mut indices: Vec<usize> = (0..triangles.len()).collect();
        let root_idx = if !triangles.is_empty() {
            Some(Self::build_bvh_rec(
                &triangle_aabbs,
                &triangle_centers,
                &mut indices,
                &mut bvh_nodes,
                strategy,
            ))
        } else {
            None
        };

        Self {
            triangles,
            bvh_nodes,
            root_idx,
            center,
            radius: max_r2.sqrt(),
        }
    }

    fn triangle_aabb(triangle: &Triangle) -> Aabb<3> {
        let mut min = triangle.vertices[0];
        let mut max = triangle.vertices[0];

        for vertex in &triangle.vertices[1..] {
            for axis in 0..3 {
                if vertex[axis] < min[axis] {
                    min[axis] = vertex[axis];
                }
                if vertex[axis] > max[axis] {
                    max[axis] = vertex[axis];
                }
            }
        }

        Aabb { min, max }
    }

    fn surface_area(aabb: &Aabb<3>) -> f64 {
        let extent = aabb.max - aabb.min;
        let x = extent[0].max(0.0);
        let y = extent[1].max(0.0);
        let z = extent[2].max(0.0);
        2.0 * (x * y + y * z + z * x)
    }

    fn centroid_bin(value: f64, minimum: f64, extent: f64, bin_count: usize) -> usize {
        let bin = (((value - minimum) / extent).clamp(0.0, 1.0) * bin_count as f64) as usize;
        bin.min(bin_count - 1)
    }

    /// Return a split index after sorting indices by the selected binned-SAH
    /// split. Invalid or severely unbalanced partitions return None so the
    /// caller can use a deterministic median fallback.
    fn binned_sah_partition(
        indices: &mut [usize],
        triangle_aabbs: &[Aabb<3>],
        triangle_centers: &[SVector<f64, 3>],
        requested_bins: usize,
    ) -> Option<usize> {
        let n = indices.len();
        if n < 4 {
            return None;
        }
        let bin_count = requested_bins.clamp(2, 64);
        let mut best: Option<(f64, usize, usize, f64, f64)> = None;

        for axis in 0..3 {
            let (mut centroid_min, mut centroid_max) =
                (f64::INFINITY, f64::NEG_INFINITY);
            for &index in indices.iter() {
                centroid_min = centroid_min.min(triangle_centers[index][axis]);
                centroid_max = centroid_max.max(triangle_centers[index][axis]);
            }
            let centroid_extent = centroid_max - centroid_min;
            if !centroid_min.is_finite()
                || !centroid_max.is_finite()
                || !centroid_extent.is_finite()
                || centroid_extent <= f64::EPSILON
            {
                continue;
            }

            let mut bin_bounds: Vec<Aabb<3>> =
                (0..bin_count).map(|_| Aabb::empty()).collect();
            let mut bin_counts = vec![0usize; bin_count];
            for &index in indices.iter() {
                let bin = Self::centroid_bin(
                    triangle_centers[index][axis],
                    centroid_min,
                    centroid_extent,
                    bin_count,
                );
                bin_bounds[bin] = bin_bounds[bin].union(&triangle_aabbs[index]);
                bin_counts[bin] += 1;
            }

            let mut left_bounds: Vec<Aabb<3>> =
                (0..bin_count).map(|_| Aabb::empty()).collect();
            let mut left_counts = vec![0usize; bin_count];
            let mut running_bounds = Aabb::empty();
            let mut running_count = 0usize;
            for bin in 0..bin_count {
                running_bounds = running_bounds.union(&bin_bounds[bin]);
                running_count += bin_counts[bin];
                left_bounds[bin] = running_bounds.clone();
                left_counts[bin] = running_count;
            }

            let mut right_bounds: Vec<Aabb<3>> =
                (0..bin_count).map(|_| Aabb::empty()).collect();
            let mut right_counts = vec![0usize; bin_count];
            running_bounds = Aabb::empty();
            running_count = 0;
            for bin in (0..bin_count).rev() {
                running_bounds = running_bounds.union(&bin_bounds[bin]);
                running_count += bin_counts[bin];
                right_bounds[bin] = running_bounds.clone();
                right_counts[bin] = running_count;
            }

            for split_bin in 0..(bin_count - 1) {
                let left_count = left_counts[split_bin];
                let right_count = right_counts[split_bin + 1];
                if left_count == 0 || right_count == 0 {
                    continue;
                }
                let cost = Self::surface_area(&left_bounds[split_bin]) * left_count as f64
                    + Self::surface_area(&right_bounds[split_bin + 1]) * right_count as f64;
                if cost.is_finite() && best.is_none_or(|(best_cost, ..)| cost < best_cost) {
                    best = Some((cost, axis, split_bin, centroid_min, centroid_extent));
                }
            }
        }

        let (_, axis, split_bin, centroid_min, centroid_extent) = best?;
        indices.sort_by_key(|&index| {
            Self::centroid_bin(
                triangle_centers[index][axis],
                centroid_min,
                centroid_extent,
                bin_count,
            )
        });
        let split = indices.partition_point(|&index| {
            Self::centroid_bin(
                triangle_centers[index][axis],
                centroid_min,
                centroid_extent,
                bin_count,
            ) <= split_bin
        });

        // Keep recursion depth bounded under highly skewed centroids.
        if split == 0 || split >= n || (n >= 32 && split.min(n - split) < n / 16) {
            None
        } else {
            Some(split)
        }
    }

    fn build_bvh_rec(
        triangle_aabbs: &[Aabb<3>],
        triangle_centers: &[SVector<f64, 3>],
        indices: &mut [usize],
        nodes: &mut Vec<MeshBvhNode>,
        strategy: MeshBvhBuildStrategy,
    ) -> usize {
        let n = indices.len();
        let aabb = indices.iter().fold(Aabb::empty(), |bounds, &idx| {
            bounds.union(&triangle_aabbs[idx])
        });

        if n == 1 {
            let idx = nodes.len();
            nodes.push(MeshBvhNode {
                aabb,
                left: None,
                right: None,
                triangle_idx: Some(indices[0]),
            });
            return idx;
        }

        let extents = aabb.max - aabb.min;
        let median_axis = if extents[0] > extents[1] && extents[0] > extents[2] {
            0
        } else if extents[1] > extents[2] {
            1
        } else {
            2
        };

        let split = match strategy {
            MeshBvhBuildStrategy::Median => None,
            MeshBvhBuildStrategy::BinnedSah { bin_count } => Self::binned_sah_partition(
                indices,
                triangle_aabbs,
                triangle_centers,
                bin_count,
            ),
        }
        .unwrap_or_else(|| {
            // Centroids were computed once during construction. total_cmp
            // gives a deterministic order for all f64 bit patterns.
            indices.sort_by(|&a, &b| {
                triangle_centers[a][median_axis].total_cmp(&triangle_centers[b][median_axis])
            });
            n / 2
        });

        let left_idx = Self::build_bvh_rec(
            triangle_aabbs,
            triangle_centers,
            &mut indices[..split],
            nodes,
            strategy,
        );
        let right_idx = Self::build_bvh_rec(
            triangle_aabbs,
            triangle_centers,
            &mut indices[split..],
            nodes,
            strategy,
        );

        let idx = nodes.len();
        nodes.push(MeshBvhNode {
            aabb,
            left: Some(left_idx),
            right: Some(right_idx),
            triangle_idx: None,
        });
        idx
    }

    pub fn query_overlap(&self, query_aabb: &Aabb<3>) -> Vec<usize> {
        let mut result = Vec::new();
        let mut stack = Vec::new();
        if let Some(root) = self.root_idx {
            stack.push(root);
        }

        while let Some(idx) = stack.pop() {
            let node = &self.bvh_nodes[idx];
            if !node.aabb.overlaps(query_aabb) {
                continue;
            }

            if let Some(tri_idx) = node.triangle_idx {
                result.push(tri_idx);
            } else {
                if let Some(l) = node.left {
                    stack.push(l);
                }
                if let Some(r) = node.right {
                    stack.push(r);
                }
            }
        }
        result
    }

    /// Query overlap while collecting traversal-work counters.
    ///
    /// The returned counters do not alter candidate semantics. This method is
    /// intended for diagnostics and benchmarks, not as a substitute for the
    /// lower-overhead query_overlap hot path.
    pub fn query_overlap_with_stats(
        &self,
        query_aabb: &Aabb<3>,
    ) -> (Vec<usize>, MeshBvhQueryStats) {
        let mut result = Vec::new();
        let mut stats = MeshBvhQueryStats::default();
        let mut stack = Vec::new();
        if let Some(root) = self.root_idx {
            stack.push(root);
        }

        while let Some(idx) = stack.pop() {
            let node = &self.bvh_nodes[idx];
            stats.bounds_tested += 1;
            if !node.aabb.overlaps(query_aabb) {
                continue;
            }

            if let Some(tri_idx) = node.triangle_idx {
                stats.overlapping_leaves += 1;
                result.push(tri_idx);
            } else {
                stats.overlapping_internal_nodes += 1;
                if let Some(l) = node.left {
                    stack.push(l);
                }
                if let Some(r) = node.right {
                    stack.push(r);
                }
            }
        }
        stats.candidate_count = result.len();
        (result, stats)
    }
}

impl symtropy_physics::manifold_gen::MeshColliderMetadata<3> for TriangleMesh {
    fn generate_mesh_contacts(
        &self,
        self_handle: symtropy_physics::body::BodyHandle,
        self_transform: &Transform<3>,
        other_shape: &dyn Shape<3>,
        other_handle: symtropy_physics::body::BodyHandle,
        other_transform: &Transform<3>,
    ) -> Vec<symtropy_physics::contact::ContactManifold<3>> {
        crate::narrowphase::generate_mesh_contacts_transformed(
            self,
            self_handle,
            self_transform,
            other_shape,
            other_handle,
            other_transform,
        )
    }
}

impl Shape<3> for TriangleMesh {
    fn support(&self, direction: &SVector<f64, 3>) -> SVector<f64, 3> {
        let mut best_dist = f64::MIN;
        let mut best_v = self.triangles[0].vertices[0];
        for tri in &self.triangles {
            for v in &tri.vertices {
                let dist = v.dot(direction);
                if dist > best_dist {
                    best_dist = dist;
                    best_v = *v;
                }
            }
        }
        best_v
    }

    fn bounding_sphere(&self) -> (Point<3>, f64) {
        (self.center, self.radius)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Shape<3>> {
        Box::new(self.clone())
    }
}

/// Wire triangle-mesh narrowphase into a 3D `PhysicsWorld`.
///
/// Call this once during world setup:
/// ```ignore
/// symtropy_mesh::install_triangle_mesh_narrowphase(&mut world);
/// ```
/// After this call, any body whose collider is a [`TriangleMesh`] will use
/// the BVH narrowphase instead of silently falling back to its convex hull.
///
/// Internally this registers a closure with
/// [`PhysicsWorld::register_mesh_contact_transform_fn`] that can perform the concrete
/// `downcast_ref::<TriangleMesh>()` — something `symtropy-physics` itself
/// cannot do due to the circular crate dependency (this crate depends on
/// `symtropy-physics`, not the reverse).
pub fn install_triangle_mesh_narrowphase(world: &mut symtropy_physics::world::PhysicsWorld<3>) {
    world.register_mesh_contact_transform_fn(
        |shape_a, transform_a, shape_b, transform_b, handle_a, handle_b| {
            use symtropy_physics::manifold_gen::MeshColliderMetadata;
            if let Some(mesh) = shape_a.as_any().downcast_ref::<TriangleMesh>() {
                return Some(mesh.generate_mesh_contacts(
                    handle_a,
                    transform_a,
                    shape_b,
                    handle_b,
                    transform_b,
                ));
            }
            if let Some(mesh) = shape_b.as_any().downcast_ref::<TriangleMesh>() {
                return Some(mesh.generate_mesh_contacts(
                    handle_b,
                    transform_b,
                    shape_a,
                    handle_a,
                    transform_a,
                ));
            }
            // Neither shape is a TriangleMesh — fall through to GJK.
            None
        },
    );
}
