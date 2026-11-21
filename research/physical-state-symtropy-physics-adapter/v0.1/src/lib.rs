// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only adapter from Symtropy's native 3D physics world to the
//! evidence-aware physical state feedback contract.
//!
//! Native engine limitations are preserved as explicit unavailable fields.
//! Simulation-generated state is never labelled as measured.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use nalgebra::SVector;
use physical_state_feedback_contract::{
    ContactObservation, ContactStatus, EntityState, FieldMetadata, FieldStatus, FieldValue,
    Provenance, ReadOnlyStateQuery, SnapshotIdentity, StateQuery, StateSnapshot, Uncertainty,
    ValidationError,
};
use symtropy_math::Point;
use symtropy_physics::body::{BodyHandle, BodyType, RigidBody};
use symtropy_physics::contact::ContactManifold;
use symtropy_physics::world::PhysicsWorld;

const ROTATION_TOLERANCE: f64 = 1.0e-7;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StableEntityIdentity {
    pub entity_id: String,
    pub generation: u64,
}

#[derive(Clone, Debug)]
pub struct AdapterContext {
    pub identity: SnapshotIdentity,
    /// Keyed by the current world's BodyHandle.0. Every live body must have
    /// exactly one stable external identity.
    pub stable_entities: BTreeMap<usize, StableEntityIdentity>,
    /// Caller attests whether PhysicsWorld.contacts belongs to this snapshot.
    pub contacts_fresh: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterError {
    pub reason: String,
}

impl AdapterError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl Display for AdapterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.reason)
    }
}

impl Error for AdapterError {}

impl From<ValidationError> for AdapterError {
    fn from(value: ValidationError) -> Self {
        Self::new(value.to_string())
    }
}

pub struct SymtropyPhysicsAdapter<'a> {
    world: &'a PhysicsWorld<3>,
    context: AdapterContext,
}

impl<'a> SymtropyPhysicsAdapter<'a> {
    pub fn new(
        world: &'a PhysicsWorld<3>,
        context: AdapterContext,
    ) -> Result<Self, AdapterError> {
        validate_stable_identity_map(world, &context)?;
        Ok(Self { world, context })
    }

    pub fn snapshot(&self) -> Result<StateSnapshot, AdapterError> {
        validate_stable_identity_map(self.world, &self.context)?;
        let stable_by_handle = &self.context.stable_entities;
        let mut partners_by_handle = BTreeMap::<usize, BTreeSet<String>>::new();

        for manifold in &self.world.contacts {
            let handle_a = manifold.body_a.0;
            let handle_b = manifold.body_b.0;
            let body_a = self
                .world
                .bodies
                .iter()
                .find(|body| body.handle.0 == handle_a)
                .ok_or_else(|| {
                    AdapterError::new(format!(
                        "contact manifold refers to unknown body handle {handle_a}"
                    ))
                })?;
            let body_b = self
                .world
                .bodies
                .iter()
                .find(|body| body.handle.0 == handle_b)
                .ok_or_else(|| {
                    AdapterError::new(format!(
                        "contact manifold refers to unknown body handle {handle_b}"
                    ))
                })?;

            // Sensor overlaps are not force-bearing physical contacts.
            if body_a.is_sensor || body_b.is_sensor {
                continue;
            }

            let id_a = stable_by_handle
                .get(&handle_a)
                .expect("stable ID map was validated")
                .entity_id
                .clone();
            let id_b = stable_by_handle
                .get(&handle_b)
                .expect("stable ID map was validated")
                .entity_id
                .clone();

            partners_by_handle
                .entry(handle_a)
                .or_default()
                .insert(id_b.clone());
            partners_by_handle
                .entry(handle_b)
                .or_default()
                .insert(id_a);
        }

        let mut entities = self
            .world
            .bodies
            .iter()
            .map(|body| {
                let stable = stable_by_handle
                    .get(&body.handle.0)
                    .expect("stable ID map was validated");
                let contact_partners = partners_by_handle
                    .get(&body.handle.0)
                    .cloned()
                    .unwrap_or_default();
                self.map_entity(body, stable, contact_partners)
            })
            .collect::<Result<Vec<_>, _>>()?;
        entities.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));

        let snapshot = StateSnapshot {
            schema_version: physical_state_feedback_contract::SCHEMA_VERSION.to_owned(),
            identity: self.context.identity.clone(),
            entities,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    fn map_entity(
        &self,
        body: &RigidBody<3>,
        stable: &StableEntityIdentity,
        contact_partners: BTreeSet<String>,
    ) -> Result<EntityState, AdapterError> {
        let entity_frame = format!(
            "entity/{}/generation/{}",
            stable.entity_id, stable.generation
        );
        let translation = [
            body.transform.translation.0[0],
            body.transform.translation.0[1],
            body.transform.translation.0[2],
        ];
        let angular_matrix = body.angular_velocity.to_matrix();
        if translation.iter().any(|value| !value.is_finite())
            || body.linear_velocity.iter().any(|value| !value.is_finite())
            || angular_matrix.iter().any(|value| !value.is_finite())
        {
            return Err(AdapterError::new(format!(
                "body {:?} contains non-finite native state",
                stable.entity_id
            )));
        }

        let rotor = &body.transform.rotation;
        let orthogonality_error = rotor.orthogonality_error();
        let determinant = rotor.determinant();
        if !orthogonality_error.is_finite()
            || orthogonality_error > ROTATION_TOLERANCE
            || !determinant.is_finite()
            || (determinant - 1.0).abs() > ROTATION_TOLERANCE
        {
            return Err(AdapterError::new(format!(
                "body {:?} has an invalid rotation matrix (orthogonality={orthogonality_error}, determinant={determinant})",
                stable.entity_id
            )));
        }

        let rotation = rotor.to_matrix();
        let matrix = [
            [rotation[(0, 0)], rotation[(0, 1)], rotation[(0, 2)]],
            [rotation[(1, 0)], rotation[(1, 1)], rotation[(1, 2)]],
            [rotation[(2, 0)], rotation[(2, 1)], rotation[(2, 2)]],
        ];
        let quaternion = matrix_to_quaternion_xyzw(matrix).ok_or_else(|| {
            AdapterError::new(format!(
                "body {:?} rotation could not be represented as a unit quaternion",
                stable.entity_id
            ))
        })?;

        let translation_m = field_value(
            translation,
            predicted_metadata(
                &self.context.identity,
                "m",
                Some("world"),
                "read directly from the native simulation transform",
            ),
        );
        let rotation_xyzw = field_value(
            quaternion,
            predicted_metadata(
                &self.context.identity,
                "1",
                Some("world"),
                "converted from the native proper-rotation matrix",
            ),
        );

        let linear_velocity = [
            body.linear_velocity[0],
            body.linear_velocity[1],
            body.linear_velocity[2],
        ];
        let linear_velocity_m_s = field_value(
            linear_velocity,
            predicted_metadata(
                &self.context.identity,
                "m/s",
                Some("world"),
                "read directly from the native rigid-body velocity state",
            ),
        );

        // Bivector::apply_to_vector uses B * v. The axial dual consistent
        // with the right-handed cross-product generator is [-B12, +B02, -B01]
        // for the native lexicographic plane basis.
        let angular_velocity = [
            -angular_matrix[(1, 2)],
            angular_matrix[(0, 2)],
            -angular_matrix[(0, 1)],
        ];
        let angular_velocity_rad_s = field_value(
            angular_velocity,
            predicted_metadata(
                &self.context.identity,
                "rad/s",
                Some("world"),
                "axial dual of the native angular-velocity bivector",
            ),
        );

        let linear_acceleration_m_s2 = unavailable_field(
            "m/s^2",
            Some("world"),
            "RigidBody does not retain acceleration; this adapter does not infer it by finite differencing",
        );

        let mass_kg = if !body.mass.is_finite() {
            return Err(AdapterError::new(format!(
                "body {:?} has non-finite configured mass",
                stable.entity_id
            )));
        } else if body.body_type == BodyType::Static || body.mass <= 0.0 {
            unavailable_field(
                "kg",
                None,
                "static or non-positive native mass is a solver sentinel/non-physical value",
            )
        } else {
            field_value(
                body.mass,
                assumed_metadata(
                    &self.context.identity,
                    "kg",
                    None,
                    "configured engine parameter; not an independently measured physical mass",
                ),
            )
        };

        // The current native integrator applies a plane-average inertia
        // approximation, not a complete physically applied 3x3 tensor.
        let inertia_kg_m2 = unavailable_field(
            "kg*m^2",
            Some(&entity_frame),
            "native angular dynamics average per-axis inertia over each rotation plane; a physically applied full 3x3 inertia tensor is not exposed",
        );

        let (contact_status, contact_metadata, partner_entity_ids) =
            self.contact_state(body, contact_partners);

        Ok(EntityState {
            entity_id: stable.entity_id.clone(),
            generation: stable.generation,
            parent_entity_id: None,
            pose: physical_state_feedback_contract::Pose3 {
                translation_m,
                rotation_xyzw,
                child_frame_id: entity_frame,
                direction: physical_state_feedback_contract::TransformDirection::WorldFromEntity,
            },
            linear_velocity_m_s,
            angular_velocity_rad_s,
            linear_acceleration_m_s2,
            mass_kg,
            inertia_kg_m2,
            contact: ContactObservation {
                status: contact_status,
                metadata: contact_metadata,
                partner_entity_ids,
                // ContactPoint.lambda is a scalar normal solver multiplier.
                // No vector impulse or per-body sign is asserted here.
                impulse_ns: None,
            },
        })
    }

    fn contact_state(
        &self,
        body: &RigidBody<3>,
        contact_partners: BTreeSet<String>,
    ) -> (ContactStatus, FieldMetadata, Vec<String>) {
        if !self.context.contacts_fresh {
            let reason =
                "caller did not attest that the native last-step contact cache is fresh for this snapshot";
            return (
                ContactStatus::Unavailable {
                    reason: reason.to_owned(),
                },
                unavailable_metadata(&self.context.identity, "1", Some("world"), reason),
                Vec::new(),
            );
        }
        if body.is_sensor {
            let reason =
                "sensor overlap is not asserted as a force-bearing physical contact manifold";
            return (
                ContactStatus::Unavailable {
                    reason: reason.to_owned(),
                },
                unavailable_metadata(&self.context.identity, "1", Some("world"), reason),
                Vec::new(),
            );
        }

        let partners = contact_partners.into_iter().collect::<Vec<_>>();
        let metadata = predicted_metadata(
            &self.context.identity,
            "1",
            Some("world"),
            "classification from the native last-step force-bearing contact manifold",
        );
        let status = if partners.is_empty() {
            ContactStatus::NoContact
        } else {
            ContactStatus::InContact
        };
        (status, metadata, partners)
    }
}

impl ReadOnlyStateQuery for SymtropyPhysicsAdapter<'_> {
    type Error = AdapterError;

    fn query(&self, request: &StateQuery) -> Result<StateSnapshot, Self::Error> {
        request.validate()?;
        if request.snapshot_sha256 != self.context.identity.snapshot_sha256 {
            return Err(AdapterError::new(
                "query snapshot digest does not match the adapter's immutable context",
            ));
        }

        let full = self.snapshot()?;
        let by_id = full
            .entities
            .iter()
            .map(|entity| (entity.entity_id.as_str(), entity))
            .collect::<BTreeMap<_, _>>();
        let mut included = BTreeSet::<String>::new();
        let mut pending = request.entity_ids.clone();

        while let Some(entity_id) = pending.pop() {
            if !included.insert(entity_id.clone()) {
                continue;
            }
            let entity = by_id.get(entity_id.as_str()).ok_or_else(|| {
                AdapterError::new(format!("requested unknown entity {entity_id:?}"))
            })?;
            pending.extend(entity.contact.partner_entity_ids.iter().cloned());
        }

        let entities = full
            .entities
            .into_iter()
            .filter(|entity| included.contains(&entity.entity_id))
            .collect::<Vec<_>>();
        let result = StateSnapshot {
            schema_version: full.schema_version,
            identity: full.identity,
            entities,
        };
        request.validate_result(&result)?;
        Ok(result)
    }
}

fn validate_stable_identity_map(
    world: &PhysicsWorld<3>,
    context: &AdapterContext,
) -> Result<(), AdapterError> {
    let handles = world
        .bodies
        .iter()
        .map(|body| body.handle.0)
        .collect::<BTreeSet<_>>();
    let mapped_handles = context
        .stable_entities
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    if handles != mapped_handles {
        return Err(AdapterError::new(
            "stable entity map must contain exactly one entry for every live BodyHandle.0",
        ));
    }

    let mut ids = BTreeSet::new();
    for stable in context.stable_entities.values() {
        if stable.entity_id.trim().is_empty() || !ids.insert(stable.entity_id.as_str()) {
            return Err(AdapterError::new(
                "stable entity IDs must be non-empty and unique in the current world",
            ));
        }
    }
    Ok(())
}

fn field_value<T>(value: T, metadata: FieldMetadata) -> FieldValue<T> {
    FieldValue {
        value: Some(value),
        metadata,
    }
}

fn predicted_metadata(
    identity: &SnapshotIdentity,
    unit: &str,
    frame_id: Option<&str>,
    derivation: &str,
) -> FieldMetadata {
    FieldMetadata {
        status: FieldStatus::Predicted,
        unit: unit.to_owned(),
        frame_id: frame_id.map(str::to_owned),
        uncertainty: Some(Uncertainty::NotQuantified {
            reason: "native simulator state has no calibrated predictive uncertainty".to_owned(),
        }),
        provenance: Provenance {
            model_build_sha256: Some(identity.model_build_sha256.clone()),
            derivation: Some(derivation.to_owned()),
            ..Provenance::default()
        },
        unavailable_reason: None,
    }
}

fn assumed_metadata(
    identity: &SnapshotIdentity,
    unit: &str,
    frame_id: Option<&str>,
    derivation: &str,
) -> FieldMetadata {
    FieldMetadata {
        status: FieldStatus::Assumed,
        unit: unit.to_owned(),
        frame_id: frame_id.map(str::to_owned),
        uncertainty: Some(Uncertainty::NotQuantified {
            reason: "configured model parameter is not calibrated against an independent physical reference".to_owned(),
        }),
        provenance: Provenance {
            model_build_sha256: Some(identity.model_build_sha256.clone()),
            derivation: Some(derivation.to_owned()),
            ..Provenance::default()
        },
        unavailable_reason: None,
    }
}

fn unavailable_metadata(
    identity: &SnapshotIdentity,
    unit: &str,
    frame_id: Option<&str>,
    reason: &str,
) -> FieldMetadata {
    FieldMetadata {
        status: FieldStatus::Unavailable,
        unit: unit.to_owned(),
        frame_id: frame_id.map(str::to_owned),
        uncertainty: None,
        provenance: Provenance {
            model_build_sha256: Some(identity.model_build_sha256.clone()),
            derivation: Some(reason.to_owned()),
            ..Provenance::default()
        },
        unavailable_reason: Some(reason.to_owned()),
    }
}

fn unavailable_field<T>(
    unit: &str,
    frame_id: Option<&str>,
    reason: &str,
) -> FieldValue<T> {
    FieldValue {
        value: None,
        metadata: FieldMetadata {
            status: FieldStatus::Unavailable,
            unit: unit.to_owned(),
            frame_id: frame_id.map(str::to_owned),
            uncertainty: None,
            provenance: Provenance {
                source_sha256: None,
                model_build_sha256: None,
                synthetic_fixture_id: None,
                derivation: Some(reason.to_owned()),
            },
            unavailable_reason: Some(reason.to_owned()),
        },
    }
}

fn matrix_to_quaternion_xyzw(matrix: [[f64; 3]; 3]) -> Option<[f64; 4]> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return None;
    }

    let trace = matrix[0][0] + matrix[1][1] + matrix[2][2];
    let (x, y, z, w);

    if trace > 0.0 {
        let scale = (trace + 1.0).sqrt() * 2.0;
        if scale <= f64::EPSILON {
            return None;
        }
        w = 0.25 * scale;
        x = (matrix[2][1] - matrix[1][2]) / scale;
        y = (matrix[0][2] - matrix[2][0]) / scale;
        z = (matrix[1][0] - matrix[0][1]) / scale;
    } else if matrix[0][0] > matrix[1][1] && matrix[0][0] > matrix[2][2] {
        let scale = (1.0 + matrix[0][0] - matrix[1][1] - matrix[2][2]).sqrt() * 2.0;
        if scale <= f64::EPSILON {
            return None;
        }
        w = (matrix[2][1] - matrix[1][2]) / scale;
        x = 0.25 * scale;
        y = (matrix[0][1] + matrix[1][0]) / scale;
        z = (matrix[0][2] + matrix[2][0]) / scale;
    } else if matrix[1][1] > matrix[2][2] {
        let scale = (1.0 + matrix[1][1] - matrix[0][0] - matrix[2][2]).sqrt() * 2.0;
        if scale <= f64::EPSILON {
            return None;
        }
        w = (matrix[0][2] - matrix[2][0]) / scale;
        x = (matrix[0][1] + matrix[1][0]) / scale;
        y = 0.25 * scale;
        z = (matrix[1][2] + matrix[2][1]) / scale;
    } else {
        let scale = (1.0 + matrix[2][2] - matrix[0][0] - matrix[1][1]).sqrt() * 2.0;
        if scale <= f64::EPSILON {
            return None;
        }
        w = (matrix[1][0] - matrix[0][1]) / scale;
        x = (matrix[0][2] + matrix[2][0]) / scale;
        y = (matrix[1][2] + matrix[2][1]) / scale;
        z = 0.25 * scale;
    }

    let mut quaternion = [x, y, z, w];
    let norm = quaternion
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= f64::EPSILON {
        return None;
    }
    for value in &mut quaternion {
        *value /= norm;
    }
    Some(quaternion)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Bivector;

    fn identity() -> SnapshotIdentity {
        SnapshotIdentity {
            snapshot_sha256: "a".repeat(64),
            model_build_sha256: "b".repeat(64),
            configuration_sha256: "c".repeat(64),
            world_id: "world-1".to_owned(),
            episode_id: "episode-1".to_owned(),
            trajectory_id: "trajectory-1".to_owned(),
            step_index: 3,
            simulation_time_ns: 50_000_000,
            clock_source: "symtropy_physics_step_context".to_owned(),
            world_frame: "world".to_owned(),
            transform_convention:
                physical_state_feedback_contract::TransformConvention::RightHandedXyzw,
            unit_system: "SI".to_owned(),
        }
    }

    fn adapter<'a>(
        world: &'a PhysicsWorld<3>,
        handles: &[BodyHandle],
        contacts_fresh: bool,
    ) -> SymtropyPhysicsAdapter<'a> {
        let stable_entities = handles
            .iter()
            .enumerate()
            .map(|(index, handle)| {
                (
                    handle.0,
                    StableEntityIdentity {
                        entity_id: format!("entity-{index}"),
                        generation: 0,
                    },
                )
            })
            .collect();
        SymtropyPhysicsAdapter::new(
            world,
            AdapterContext {
                identity: identity(),
                stable_entities,
                contacts_fresh,
            },
        )
        .unwrap()
    }

    fn world() -> PhysicsWorld<3> {
        PhysicsWorld::<3>::new(SVector::<f64, 3>::new(0.0, -9.81, 0.0))
    }

    #[test]
    fn dynamic_sphere_maps_pose_mass_and_explicitly_unavailable_fields() {
        let mut world = world();
        let handle = world.add_sphere(Point::new([1.0, 2.0, 3.0]), 0.25, 2.0);
        let adapter = adapter(&world, &[handle], true);
        let snapshot = adapter.snapshot().unwrap();
        assert!(snapshot.validate().is_ok());

        let entity = &snapshot.entities[0];
        assert_eq!(entity.entity_id, "entity-0");
        assert_eq!(entity.pose.translation_m.value, Some([1.0, 2.0, 3.0]));
        assert_eq!(entity.mass_kg.value, Some(2.0));
        assert_eq!(entity.mass_kg.metadata.status, FieldStatus::Assumed);
        assert_eq!(entity.linear_acceleration_m_s2.value, None);
        assert_eq!(entity.inertia_kg_m2.value, None);
        assert_eq!(entity.inertia_kg_m2.metadata.status, FieldStatus::Unavailable);
        assert_eq!(entity.contact.status, ContactStatus::NoContact);
    }

    #[test]
    fn angular_bivector_dual_matches_native_apply_to_vector_convention() {
        let cases = [
            (Bivector::<3>::unit_plane(0, 1), [0.0, 0.0, -1.0]),
            (Bivector::<3>::unit_plane(0, 2), [0.0, 1.0, 0.0]),
            (Bivector::<3>::unit_plane(1, 2), [-1.0, 0.0, 0.0]),
        ];
        for (bivector, expected) in cases {
            let matrix = bivector.to_matrix();
            let mapped = [
                -matrix[(1, 2)],
                matrix[(0, 2)],
                -matrix[(0, 1)],
            ];
            assert_eq!(mapped, expected);
        }
    }

    #[test]
    fn stale_contact_cache_is_unavailable_not_no_contact() {
        let mut world = world();
        let handle = world.add_sphere(Point::origin(), 0.25, 1.0);
        let adapter = adapter(&world, &[handle], false);
        let snapshot = adapter.snapshot().unwrap();
        assert!(matches!(
            snapshot.entities[0].contact.status,
            ContactStatus::Unavailable { .. }
        ));
    }

    #[test]
    fn sensor_body_does_not_claim_force_bearing_contact() {
        let mut world = world();
        let handle = world.add_sphere(Point::origin(), 0.25, 1.0);
        world.bodies[0].is_sensor = true;
        let adapter = adapter(&world, &[handle], true);
        let snapshot = adapter.snapshot().unwrap();
        assert!(matches!(
            snapshot.entities[0].contact.status,
            ContactStatus::Unavailable { .. }
        ));
    }

    #[test]
    fn missing_or_duplicate_stable_ids_are_rejected() {
        let mut world = world();
        let first = world.add_sphere(Point::origin(), 0.25, 1.0);
        let second = world.add_sphere(Point::new([2.0, 0.0, 0.0]), 0.25, 1.0);
        let missing = BTreeMap::from([(
            first.0,
            StableEntityIdentity {
                entity_id: "one".to_owned(),
                generation: 0,
            },
        )]);
        let context = AdapterContext {
            identity: identity(),
            stable_entities: missing,
            contacts_fresh: true,
        };
        assert!(SymtropyPhysicsAdapter::new(&world, context).is_err());

        let duplicate = BTreeMap::from([
            (
                first.0,
                StableEntityIdentity {
                    entity_id: "same".to_owned(),
                    generation: 0,
                },
            ),
            (
                second.0,
                StableEntityIdentity {
                    entity_id: "same".to_owned(),
                    generation: 0,
                },
            ),
        ]);
        let context = AdapterContext {
            identity: identity(),
            stable_entities: duplicate,
            contacts_fresh: true,
        };
        assert!(SymtropyPhysicsAdapter::new(&world, context).is_err());
    }

    #[test]
    fn contact_manifold_maps_partners_but_not_ambiguous_impulse_vector() {
        let mut world = world();
        let a = world.add_sphere(Point::origin(), 0.5, 1.0);
        let b = world.add_sphere(Point::new([0.5, 0.0, 0.0]), 0.5, 1.0);
        world.contacts.push(ContactManifold::single(
            a,
            b,
            SVector::<f64, 3>::new(1.0, 0.0, 0.0),
            SVector::<f64, 3>::new(0.25, 0.0, 0.0),
            0.1,
        ));
        let adapter = adapter(&world, &[a, b], true);
        let snapshot = adapter.snapshot().unwrap();
        assert_eq!(snapshot.entities[0].contact.status, ContactStatus::InContact);
        assert_eq!(
            snapshot.entities[0].contact.partner_entity_ids,
            vec!["entity-1".to_owned()]
        );
        assert!(snapshot.entities[0].contact.impulse_ns.is_none());
    }

    #[test]
    fn read_only_query_returns_requested_entity_and_contact_context_only() {
        let mut world = world();
        let a = world.add_sphere(Point::origin(), 0.5, 1.0);
        let b = world.add_sphere(Point::new([0.5, 0.0, 0.0]), 0.5, 1.0);
        let c = world.add_sphere(Point::new([5.0, 0.0, 0.0]), 0.5, 1.0);
        world.contacts.push(ContactManifold::single(
            a,
            b,
            SVector::<f64, 3>::new(1.0, 0.0, 0.0),
            SVector::<f64, 3>::new(0.25, 0.0, 0.0),
            0.1,
        ));
        let adapter = adapter(&world, &[a, b, c], true);
        let query = StateQuery {
            snapshot_sha256: identity().snapshot_sha256,
            entity_ids: vec!["entity-0".to_owned()],
            fields: vec![physical_state_feedback_contract::StateField::Pose],
        };
        let response = adapter.query(&query).unwrap();
        assert_eq!(
            response
                .entities
                .iter()
                .map(|entity| entity.entity_id.as_str())
                .collect::<Vec<_>>(),
            vec!["entity-0", "entity-1"]
        );

        let mut stale = query;
        stale.snapshot_sha256 = "f".repeat(64);
        assert!(adapter.query(&stale).is_err());
    }

    #[test]
    fn quaternion_conversion_handles_identity_and_half_turns() {
        assert_eq!(
            matrix_to_quaternion_xyzw([
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ]),
            Some([0.0, 0.0, 0.0, 1.0])
        );
        let half_turn_x = matrix_to_quaternion_xyzw([
            [1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, -1.0],
        ])
        .unwrap();
        assert!((half_turn_x[0].abs() - 1.0).abs() < 1e-12);
        assert!(half_turn_x[1].abs() < 1e-12);
        assert!(half_turn_x[2].abs() < 1e-12);
        assert!(half_turn_x[3].abs() < 1e-12);
    }
}
