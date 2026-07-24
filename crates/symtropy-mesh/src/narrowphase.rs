// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! BVH-based narrowphase for triangle meshes and multi-physics meshlet meshes.

use crate::Triangle;
use crate::TriangleMesh;
use crate::meshlet_physics::MultiPhysicsMeshletMesh;
use arrayvec::ArrayVec;
use nalgebra::SVector;
use symtropy_math::{Point, Shape, Transform};
use symtropy_physics::body::BodyHandle;
use symtropy_physics::broadphase::Aabb;
use symtropy_physics::contact::{ContactManifold, ContactPoint};
use symtropy_physics::support_map::{TransformedShape, WorldSupportMap, support_aabb};
use symtropy_physics::{epa, gjk};

/// Generate contacts using the legacy translation-only interface.
pub fn generate_mesh_contacts(
    mesh: &TriangleMesh,
    mesh_handle: BodyHandle,
    mesh_pos: &SVector<f64, 3>,
    other_shape: &dyn Shape<3>,
    other_handle: BodyHandle,
    other_pos: &SVector<f64, 3>,
) -> Vec<ContactManifold<3>> {
    let mesh_transform = Transform::from_translation(Point(*mesh_pos));
    let other_transform = Transform::from_translation(Point(*other_pos));
    generate_mesh_contacts_transformed(
        mesh,
        mesh_handle,
        &mesh_transform,
        other_shape,
        other_handle,
        &other_transform,
    )
}

/// Generate contacts between a fully transformed triangle mesh and convex shape.
pub fn generate_mesh_contacts_transformed(
    mesh: &TriangleMesh,
    mesh_handle: BodyHandle,
    mesh_transform: &Transform<3>,
    other_shape: &dyn Shape<3>,
    other_handle: BodyHandle,
    other_transform: &Transform<3>,
) -> Vec<ContactManifold<3>> {
    let mut manifolds = Vec::new();

    // 1. Compute a tight world AABB for the transformed convex shape.
    let other_map = TransformedShape::new(other_shape, other_transform);
    let (other_min, other_max) = support_aabb(&other_map);
    let other_aabb_world = Aabb {
        min: other_min,
        max: other_max,
    };

    // 2. Transform all world-AABB corners into mesh local space to obtain a
    // conservative local query box even when the mesh itself is rotated.
    let other_aabb_local = aabb_to_local(&other_aabb_world, mesh_transform);

    let tri_indices = mesh.query_overlap(&other_aabb_local);

    // 3. Perform GJK/EPA for each candidate triangle
    for idx in tri_indices {
        let tri = &mesh.triangles[idx];

        // GJK in world space
        let gjk_res =
            gjk::intersects_transformed(tri, mesh_transform, other_shape, other_transform);

        if gjk_res.intersecting {
            // EPA for penetration
            if let Some(epa_res) = epa::penetration_transformed(
                tri,
                mesh_transform,
                other_shape,
                other_transform,
                &gjk_res.simplex,
            ) {
                // Generate multi-point manifold (simplified: single point for now)
                let mut points = ArrayVec::new();

                let tri_map = TransformedShape::new(tri, mesh_transform);
                let other_map = TransformedShape::new(other_shape, other_transform);
                let contact_pos = (tri_map.support_world(&epa_res.normal)
                    + other_map.support_world(&(-epa_res.normal)))
                    * 0.5;

                points.push(ContactPoint {
                    position: contact_pos,
                    depth: epa_res.depth,
                    lambda: 0.0,
                    restitution_bias: 0.0,
                });

                manifolds.push(ContactManifold {
                    body_a: mesh_handle,
                    body_b: other_handle,
                    normal: epa_res.normal,
                    points,
                    elasticity: None,
                });
            }
        }
    }

    manifolds
}

fn aabb_to_local(aabb: &Aabb<3>, transform: &Transform<3>) -> Aabb<3> {
    let inverse = transform.inverse();
    let mut local = Aabb::empty();
    for bits in 0..8usize {
        let corner = SVector::<f64, 3>::from_fn(|axis, _| {
            if bits & (1 << axis) != 0 {
                aabb.max[axis]
            } else {
                aabb.min[axis]
            }
        });
        let point = inverse.transform_point(&Point(corner)).0;
        for axis in 0..3 {
            local.min[axis] = local.min[axis].min(point[axis]);
            local.max[axis] = local.max[axis].max(point[axis]);
        }
    }
    local
}

/// Generate contacts between a multi-physics virtual geometry meshlet mesh and another convex shape.
///
/// Accelerates collision detection using a midphase sphere-sphere check at the meshlet
/// cluster level, running GJK/EPA only on triangles belonging to overlapping meshlets.
pub fn generate_meshlet_contacts(
    mesh: &MultiPhysicsMeshletMesh,
    mesh_handle: BodyHandle,
    mesh_pos: &SVector<f64, 3>,
    other_shape: &dyn Shape<3>,
    other_handle: BodyHandle,
    other_pos: &SVector<f64, 3>,
) -> Vec<ContactManifold<3>> {
    let mesh_transform = Transform::from_translation(Point(*mesh_pos));
    let other_transform = Transform::from_translation(Point(*other_pos));
    generate_meshlet_contacts_transformed(
        mesh,
        mesh_handle,
        &mesh_transform,
        other_shape,
        other_handle,
        &other_transform,
    )
}

pub fn generate_meshlet_contacts_transformed(
    mesh: &MultiPhysicsMeshletMesh,
    mesh_handle: BodyHandle,
    mesh_transform: &Transform<3>,
    other_shape: &dyn Shape<3>,
    other_handle: BodyHandle,
    other_transform: &Transform<3>,
) -> Vec<ContactManifold<3>> {
    let mut manifolds = Vec::new();

    // 1. Compute bounding sphere for the other shape in world space
    let other_map = TransformedShape::new(other_shape, other_transform);
    let (other_sphere_world_center, other_radius) = other_map.bounding_sphere_world();

    // 2. Iterate through meshlets and perform midphase sphere-sphere check
    for meshlet in &mesh.meshlets {
        // Calculate max distance from center of mass to vertices in the cluster to find the cluster radius
        let mut max_r2 = 0.0f32;
        let start_v = meshlet.vertex_offset as usize;
        let end_v = (meshlet.vertex_offset + meshlet.vertex_count) as usize;

        for v in &mesh.vertices[start_v..end_v] {
            let dx = v.position[0] - meshlet.center_of_mass[0];
            let dy = v.position[1] - meshlet.center_of_mass[1];
            let dz = v.position[2] - meshlet.center_of_mass[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq > max_r2 {
                max_r2 = dist_sq;
            }
        }
        let meshlet_radius = (max_r2.sqrt() as f64).max(0.001);
        let meshlet_center_local = SVector::<f64, 3>::from([
            meshlet.center_of_mass[0] as f64,
            meshlet.center_of_mass[1] as f64,
            meshlet.center_of_mass[2] as f64,
        ]);
        let meshlet_center_world = mesh_transform
            .transform_point(&Point(meshlet_center_local))
            .0;

        // Check sphere-sphere intersection
        let dist = (meshlet_center_world - other_sphere_world_center).norm();
        if dist > meshlet_radius + other_radius {
            continue; // No overlap with this meshlet cluster
        }

        // 3. Perform GJK/EPA on each triangle inside this overlapping meshlet
        let start_idx = meshlet.index_offset as usize;
        let num_triangles = meshlet.index_count as usize / 3;

        for t in 0..num_triangles {
            let i0 = meshlet.vertex_offset as usize + mesh.indices[start_idx + t * 3] as usize;
            let i1 = meshlet.vertex_offset as usize + mesh.indices[start_idx + t * 3 + 1] as usize;
            let i2 = meshlet.vertex_offset as usize + mesh.indices[start_idx + t * 3 + 2] as usize;

            let v0 = &mesh.vertices[i0];
            let v1 = &mesh.vertices[i1];
            let v2 = &mesh.vertices[i2];

            let tri = Triangle {
                vertices: [
                    SVector::<f64, 3>::from([
                        v0.position[0] as f64,
                        v0.position[1] as f64,
                        v0.position[2] as f64,
                    ]),
                    SVector::<f64, 3>::from([
                        v1.position[0] as f64,
                        v1.position[1] as f64,
                        v1.position[2] as f64,
                    ]),
                    SVector::<f64, 3>::from([
                        v2.position[0] as f64,
                        v2.position[1] as f64,
                        v2.position[2] as f64,
                    ]),
                ],
            };

            let gjk_res =
                gjk::intersects_transformed(&tri, mesh_transform, other_shape, other_transform);

            if gjk_res.intersecting {
                if let Some(epa_res) = epa::penetration_transformed(
                    &tri,
                    mesh_transform,
                    other_shape,
                    other_transform,
                    &gjk_res.simplex,
                ) {
                    let mut points = ArrayVec::new();
                    let tri_map = TransformedShape::new(&tri, mesh_transform);
                    let contact_pos = (tri_map.support_world(&epa_res.normal)
                        + other_map.support_world(&(-epa_res.normal)))
                        * 0.5;

                    points.push(ContactPoint {
                        position: contact_pos,
                        depth: epa_res.depth,
                        lambda: 0.0,
                        restitution_bias: 0.0,
                    });

                    manifolds.push(ContactManifold {
                        body_a: mesh_handle,
                        body_b: other_handle,
                        normal: epa_res.normal,
                        points,
                        elasticity: Some(meshlet.average_elasticity as f64),
                    });
                }
            }
        }
    }

    manifolds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meshlet_physics::MultiPhysicsVertex;
    use symtropy_math::{Point, Sphere};

    #[test]
    fn test_meshlet_narrowphase_collision() {
        // Construct simple triangle representing a flat floor
        let v1 = MultiPhysicsVertex {
            position: [-5.0, 0.0, -5.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            mass_density: 7800.0,
            elastic_modulus: 200e9,
            thermal_conductivity: 50.0,
            acoustic_impedance: 46e6,
        };
        let v2 = MultiPhysicsVertex {
            position: [5.0, 0.0, -5.0],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 0.0],
            mass_density: 7800.0,
            elastic_modulus: 200e9,
            thermal_conductivity: 50.0,
            acoustic_impedance: 46e6,
        };
        let v3 = MultiPhysicsVertex {
            position: [0.0, 0.0, 5.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.5, 1.0],
            mass_density: 7800.0,
            elastic_modulus: 200e9,
            thermal_conductivity: 50.0,
            acoustic_impedance: 46e6,
        };

        let vertices = vec![v1, v2, v3];
        let indices = vec![0, 1, 2];

        let mesh = MultiPhysicsMeshletMesh::build_prototype(vertices, indices);

        // A sphere colliding downward into the floor
        let collider_sphere = Sphere::new(Point::new([0.0, 0.5, 0.0]), 1.0); // center: [0, 0.5, 0], radius: 1.0 (penetration depth = 0.5)
        let mesh_pos = SVector::from([0.0, 0.0, 0.0]);
        let other_pos = SVector::from([0.0, 0.0, 0.0]);

        let manifolds = generate_meshlet_contacts(
            &mesh,
            BodyHandle(0),
            &mesh_pos,
            &collider_sphere,
            BodyHandle(1),
            &other_pos,
        );

        assert!(
            !manifolds.is_empty(),
            "Collision should have generated contact manifolds"
        );
        let manifold = &manifolds[0];
        assert_eq!(manifold.body_a, BodyHandle(0));
        assert_eq!(manifold.body_b, BodyHandle(1));
        assert!(
            manifold.normal[1] > 0.0,
            "Normal should point upward from the mesh triangle"
        );
        assert!(
            manifold.points[0].depth > 0.1,
            "Penetration depth should be positive, got {}",
            manifold.points[0].depth
        );
    }

    #[test]
    fn test_material_aware_compliance() {
        // Construct flat floor with steel-like modulus (stiff: 200e9)
        let v1_stiff = MultiPhysicsVertex {
            position: [-5.0, 0.0, -5.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            mass_density: 7800.0,
            elastic_modulus: 200e9,
            thermal_conductivity: 50.0,
            acoustic_impedance: 46e6,
        };
        let v2_stiff = MultiPhysicsVertex {
            position: [5.0, 0.0, -5.0],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 0.0],
            mass_density: 7800.0,
            elastic_modulus: 200e9,
            thermal_conductivity: 50.0,
            acoustic_impedance: 46e6,
        };
        let v3_stiff = MultiPhysicsVertex {
            position: [0.0, 0.0, 5.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.5, 1.0],
            mass_density: 7800.0,
            elastic_modulus: 200e9,
            thermal_conductivity: 50.0,
            acoustic_impedance: 46e6,
        };
        let mesh_stiff = MultiPhysicsMeshletMesh::build_prototype(
            vec![v1_stiff, v2_stiff, v3_stiff],
            vec![0, 1, 2],
        );

        // Construct flat floor with rubber-like modulus (soft: 100e3)
        let v1_soft = MultiPhysicsVertex {
            position: [-5.0, 0.0, -5.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            mass_density: 1100.0,
            elastic_modulus: 100e3,
            thermal_conductivity: 0.15,
            acoustic_impedance: 1.5e6,
        };
        let v2_soft = MultiPhysicsVertex {
            position: [5.0, 0.0, -5.0],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 0.0],
            mass_density: 1100.0,
            elastic_modulus: 100e3,
            thermal_conductivity: 0.15,
            acoustic_impedance: 1.5e6,
        };
        let v3_soft = MultiPhysicsVertex {
            position: [0.0, 0.0, 5.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.5, 1.0],
            mass_density: 1100.0,
            elastic_modulus: 100e3,
            thermal_conductivity: 0.15,
            acoustic_impedance: 1.5e6,
        };
        let mesh_soft = MultiPhysicsMeshletMesh::build_prototype(
            vec![v1_soft, v2_soft, v3_soft],
            vec![0, 1, 2],
        );

        // Generate contact manifolds
        let collider_sphere = Sphere::new(Point::new([0.0, 0.5, 0.0]), 1.0);
        let mesh_pos = SVector::from([0.0, 0.0, 0.0]);
        let other_pos = SVector::from([0.0, 0.0, 0.0]);

        let stiff_manifolds = generate_meshlet_contacts(
            &mesh_stiff,
            BodyHandle(0),
            &mesh_pos,
            &collider_sphere,
            BodyHandle(1),
            &other_pos,
        );

        let soft_manifolds = generate_meshlet_contacts(
            &mesh_soft,
            BodyHandle(0),
            &mesh_pos,
            &collider_sphere,
            BodyHandle(2),
            &other_pos,
        );

        assert!(!stiff_manifolds.is_empty());
        assert!(!soft_manifolds.is_empty());

        let stiff_m = &stiff_manifolds[0];
        let soft_m = &soft_manifolds[0];

        // Check that elasticity values are correctly propagated to manifolds (with float tolerance)
        assert!((stiff_m.elasticity.unwrap() - 200e9).abs() < 1e5);
        assert!((soft_m.elasticity.unwrap() - 100e3).abs() < 1e1);
    }

    #[test]
    fn world_aabb_is_conservatively_mapped_into_rotated_mesh_space() {
        use std::f64::consts::FRAC_PI_2;
        use symtropy_math::{Bivector, Rotor};

        let world = Aabb {
            min: SVector::from([-1.0, -0.5, -0.25]),
            max: SVector::from([1.0, 0.5, 0.25]),
        };
        let transform = Transform {
            translation: Point::new([3.0, 0.0, 0.0]),
            rotation: Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), FRAC_PI_2),
        };
        let local = aabb_to_local(&world, &transform);

        // Translation shifts the world box before inverse rotation; all eight
        // corners must still be enclosed by the returned local AABB.
        for bits in 0..8usize {
            let corner = SVector::<f64, 3>::from_fn(|axis, _| {
                if bits & (1 << axis) != 0 {
                    world.max[axis]
                } else {
                    world.min[axis]
                }
            });
            let point = transform.inverse().transform_point(&Point(corner)).0;
            for axis in 0..3 {
                assert!(point[axis] >= local.min[axis] - 1e-12);
                assert!(point[axis] <= local.max[axis] + 1e-12);
            }
        }
    }
}
