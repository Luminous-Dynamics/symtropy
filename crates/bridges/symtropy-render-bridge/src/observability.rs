// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Read-only physics contact overlays for Bevy engineering views.
//!
//! The extraction API only borrows contact manifolds immutably and emits
//! presentation glyphs. It does not modify physics state, contact data,
//! bodies, constraints, or simulation cadence.

use crate::sync::{PhysicsWorldRes2D, PhysicsWorldRes3D};
use bevy::prelude::*;
use symtropy_physics::contact::ContactManifold;

/// Configuration for the optional contact-overlay renderer.
#[derive(Resource, Clone, Debug)]
pub struct PhysicsOverlayConfig {
    /// Master switch. Turn this off to remove all overlay work from the frame.
    pub enabled: bool,
    /// Draw a small cross at each contact point.
    pub show_points: bool,
    /// Draw the contact normal from each point.
    pub show_normals: bool,
    /// Maximum total contact points drawn per rendered frame across supported worlds.
    pub max_contact_points: usize,
    /// Half-width of each contact-point cross in world units.
    pub point_half_extent: f32,
    /// Base normal length in world units; deeper contacts extend up to 2× this value.
    pub normal_length: f32,
}

impl Default for PhysicsOverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_points: true,
            show_normals: true,
            max_contact_points: 512,
            point_half_extent: 0.035,
            normal_length: 0.2,
        }
    }
}

/// A finite presentation-space representation of one physics contact point.
///
/// Positions and normals are projected into the first three world coordinates.
/// In 2D, the third coordinate is zero. Normals are normalized during extraction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactGlyph {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub depth: f32,
}

/// Extract bounded, finite contact glyphs without mutation or rendering.
///
/// Contacts are visited in source order. Invalid positions, normals, and depths
/// are skipped instead of generating non-finite gizmo coordinates. The caller
/// can compare the resulting glyphs to a recorded reference independently of
/// Bevy's renderer.
pub fn extract_contact_glyphs<const D: usize>(
    contacts: &[ContactManifold<D>],
    max_contact_points: usize,
) -> Vec<ContactGlyph> {
    if max_contact_points == 0 {
        return Vec::new();
    }

    let mut glyphs = Vec::with_capacity(max_contact_points.min(contacts.len()));
    'manifolds: for manifold in contacts {
        let Some(mut normal) = project_vector(&manifold.normal) else {
            continue;
        };
        let normal_length = vector_length(normal);
        if !normal_length.is_finite() || normal_length <= f32::EPSILON {
            continue;
        }
        for coordinate in &mut normal {
            *coordinate /= normal_length;
        }

        for point in &manifold.points {
            let Some(position) = project_vector(&point.position) else {
                continue;
            };
            if !point.depth.is_finite() {
                continue;
            }

            glyphs.push(ContactGlyph {
                position,
                normal,
                // Avoid propagating negative penetration or f64→f32 overflow
                // into visual scale calculations.
                depth: point.depth.clamp(0.0, f64::from(f32::MAX)) as f32,
            });
            if glyphs.len() >= max_contact_points {
                break 'manifolds;
            }
        }
    }
    glyphs
}

fn project_vector<const D: usize>(vector: &nalgebra::SVector<f64, D>) -> Option<[f32; 3]> {
    let mut result = [0.0_f32; 3];
    for index in 0..D.min(3) {
        let value = vector[index];
        if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
            return None;
        }
        result[index] = value as f32;
    }
    Some(result)
}

fn vector_length(vector: [f32; 3]) -> f32 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

/// Add the read-only contact overlay to a Bevy app.
///
/// The app must enable Bevy gizmo rendering and insert either or both of
/// `PhysicsWorldRes2D` and `PhysicsWorldRes3D`. Missing world resources are
/// supported: the system becomes a no-op for that dimension.
pub struct PhysicsObservabilityPlugin;

impl Plugin for PhysicsObservabilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhysicsOverlayConfig>()
            .add_systems(Update, draw_contact_overlays);
    }
}

fn draw_contact_overlays(
    physics_2d: Option<Res<PhysicsWorldRes2D>>,
    physics_3d: Option<Res<PhysicsWorldRes3D>>,
    config: Res<PhysicsOverlayConfig>,
    mut gizmos: Gizmos,
) {
    if !config.enabled || (!config.show_points && !config.show_normals) {
        return;
    }

    let mut glyphs = Vec::new();
    if let Some(physics) = physics_2d {
        glyphs.extend(extract_contact_glyphs(
            &physics.world.contacts,
            config.max_contact_points,
        ));
    }
    let remaining = config.max_contact_points.saturating_sub(glyphs.len());
    if remaining > 0 {
        if let Some(physics) = physics_3d {
            glyphs.extend(extract_contact_glyphs(&physics.world.contacts, remaining));
        }
    }

    let extent = if config.point_half_extent.is_finite() {
        config.point_half_extent.clamp(0.0, 10.0)
    } else {
        0.0
    };
    let normal_scale = if config.normal_length.is_finite() {
        config.normal_length.clamp(0.0, 100.0)
    } else {
        0.0
    };

    for glyph in glyphs {
        let point = Vec3::new(glyph.position[0], glyph.position[1], glyph.position[2]);
        let normal = Vec3::new(glyph.normal[0], glyph.normal[1], glyph.normal[2]);

        if config.show_points && extent > 0.0 {
            let point_color = Color::srgb(1.0, 0.2, 0.2);
            gizmos.line(point - Vec3::X * extent, point + Vec3::X * extent, point_color);
            gizmos.line(point - Vec3::Y * extent, point + Vec3::Y * extent, point_color);
            if glyph.normal[2] != 0.0 || glyph.position[2] != 0.0 {
                gizmos.line(point - Vec3::Z * extent, point + Vec3::Z * extent, point_color);
            }
        }

        if config.show_normals && normal_scale > 0.0 {
            let length = normal_scale * (1.0 + glyph.depth.min(1.0));
            gizmos.line(point, point + normal * length, Color::srgb(1.0, 0.85, 0.1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::SVector;
    use symtropy_physics::body::BodyHandle;

    #[test]
    fn three_dimensional_contacts_project_to_finite_unit_normals() {
        let contact = ContactManifold::<3>::single(
            BodyHandle(4),
            BodyHandle(9),
            SVector::from([0.0, 2.0, 0.0]),
            SVector::from([1.25, 2.5, -3.0]),
            0.4,
        );
        let glyphs = extract_contact_glyphs(&[contact], 8);
        assert_eq!(glyphs.len(), 1);
        assert_eq!(glyphs[0].position, [1.25, 2.5, -3.0]);
        assert_eq!(glyphs[0].normal, [0.0, 1.0, 0.0]);
        assert!((glyphs[0].depth - 0.4).abs() < 1e-6);
    }

    #[test]
    fn two_dimensional_contacts_have_zero_z_coordinate() {
        let contact = ContactManifold::<2>::single(
            BodyHandle(1),
            BodyHandle(2),
            SVector::from([3.0, 4.0]),
            SVector::from([5.0, -6.0]),
            0.2,
        );
        let glyphs = extract_contact_glyphs(&[contact], 8);
        assert_eq!(glyphs[0].position, [5.0, -6.0, 0.0]);
        assert_eq!(glyphs[0].normal, [0.6, 0.8, 0.0]);
    }

    #[test]
    fn point_limit_and_zero_limit_are_enforced() {
        let mut contact = ContactManifold::<3>::single(
            BodyHandle(0),
            BodyHandle(1),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([0.0, 0.0, 0.0]),
            0.1,
        );
        contact.points.push(symtropy_physics::contact::ContactPoint {
            position: SVector::from([1.0, 0.0, 0.0]),
            depth: 0.2,
            lambda: 0.0,
            restitution_bias: 0.0,
        });
        assert!(extract_contact_glyphs(&[contact.clone()], 0).is_empty());
        assert_eq!(extract_contact_glyphs(&[contact], 1).len(), 1);
    }

    #[test]
    fn malformed_contact_values_are_skipped() {
        let mut contact = ContactManifold::<3>::single(
            BodyHandle(0),
            BodyHandle(1),
            SVector::from([f64::NAN, 0.0, 0.0]),
            SVector::from([1.0, 2.0, 3.0]),
            0.1,
        );
        contact.points[0].depth = f64::INFINITY;
        assert!(extract_contact_glyphs(&[contact], 8).is_empty());
    }

    #[test]
    fn extraction_is_read_only_and_deterministic() {
        let contact = ContactManifold::<3>::single(
            BodyHandle(4),
            BodyHandle(9),
            SVector::from([0.0, 1.0, 0.0]),
            SVector::from([1.0, 2.0, 3.0]),
            0.25,
        );
        let before = (
            contact.body_a,
            contact.body_b,
            contact.points[0].position,
            contact.points[0].depth,
        );
        let first = extract_contact_glyphs(std::slice::from_ref(&contact), 8);
        let second = extract_contact_glyphs(std::slice::from_ref(&contact), 8);
        let after = (
            contact.body_a,
            contact.body_b,
            contact.points[0].position,
            contact.points[0].depth,
        );

        assert_eq!(first, second);
        assert_eq!(before, after);
    }
}
