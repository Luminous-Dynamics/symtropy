// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Thermodynamic semantic overlay for exact Symtropy physics state.
//!
//! This module deliberately composes with [`crate::PhysicsFrameEncoder`] rather
//! than changing its v1 semantic contract in place. Existing HDC research can
//! therefore remain reproducible while thermal-aware studies opt into a new,
//! versioned overlay and exact-state digest.
//!
//! The overlay is non-authoritative. Temperatures, material properties and
//! energy remain owned by `symtropy-physics`; this module only derives a
//! deterministic associative representation from validated authoritative state.

use serde::{Deserialize, Serialize};
use symtropy_hdc_core::{
    BipolarBundle, BipolarHV, EncoderSpec, ItemMemory, LevelEncoder, StableHash64,
};
use symtropy_physics::PhysicsWorld;

use crate::{
    EncodedPhysicsFrame, ExactStateDigest, IdentityPolicy, PhysicsFrameEncoder, PhysicsHdcError,
};

/// Versioned thermodynamic semantic ranges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalSemanticConfig {
    pub hdc: EncoderSpec,
    /// Maximum represented absolute temperature in kelvin.
    pub temperature_max_kelvin: f64,
    /// Maximum represented effective thermal mass in kilograms.
    pub thermal_mass_extent_kg: f64,
    /// Maximum represented specific heat capacity in J/(kg*K).
    pub specific_heat_capacity_extent: f64,
    /// Maximum represented conductivity in W/(m*K).
    pub thermal_conductivity_extent: f64,
    /// Maximum represented modeled energy magnitude in joules.
    pub energy_extent_joules: f64,
}

impl ThermalSemanticConfig {
    pub fn from_base(encoder: &PhysicsFrameEncoder) -> Self {
        Self {
            hdc: encoder.config().hdc.clone(),
            temperature_max_kelvin: 10_000.0,
            thermal_mass_extent_kg: encoder.config().mass_extent.max(1.0),
            specific_heat_capacity_extent: 10_000.0,
            thermal_conductivity_extent: 10_000.0,
            energy_extent_joules: encoder.config().energy_extent.max(1.0),
        }
    }

    pub fn validate(&self) -> Result<(), PhysicsHdcError> {
        self.hdc.validate()?;
        for (name, value) in [
            ("temperature_max_kelvin", self.temperature_max_kelvin),
            ("thermal_mass_extent_kg", self.thermal_mass_extent_kg),
            (
                "specific_heat_capacity_extent",
                self.specific_heat_capacity_extent,
            ),
            (
                "thermal_conductivity_extent",
                self.thermal_conductivity_extent,
            ),
            ("energy_extent_joules", self.energy_extent_joules),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(PhysicsHdcError::InvalidConfig(format!(
                    "{name} must be finite and positive"
                )));
            }
        }
        Ok(())
    }

    /// Fingerprint only for this overlay. The composite encoder fingerprint
    /// additionally binds the base physics encoder fingerprint.
    pub fn fingerprint(&self) -> u64 {
        let mut hash = StableHash64::with_seed(self.hdc.fingerprint());
        hash.write(b"symtropy-thermal-semantics-v1");
        for value in [
            self.temperature_max_kelvin,
            self.thermal_mass_extent_kg,
            self.specific_heat_capacity_extent,
            self.thermal_conductivity_extent,
            self.energy_extent_joules,
        ] {
            hash.write_u64(value.to_bits());
        }
        hash.finish()
    }
}

/// A base physics frame augmented with thermodynamic semantics.
#[derive(Debug, Clone)]
pub struct ThermalEncodedPhysicsFrame<const D: usize> {
    pub tick: u64,
    pub encoder_fingerprint: u64,
    /// Digest v2: every state covered by v1 plus exact thermal state.
    ///
    /// This is a provenance identity, not proof that the state is physically
    /// valid and not a receipt for the lifecycle transition that produced it.
    pub exact_digest: ExactStateDigest,
    pub reference_origin: [f64; D],
    pub body_count: usize,
    pub thermal_body_count: usize,
    pub collision_count: usize,
    pub vector: BipolarHV,
}

/// Deterministic thermodynamic overlay around an existing physics encoder.
#[derive(Debug, Clone)]
pub struct ThermalSemanticEncoder {
    config: ThermalSemanticConfig,
    memory: ItemMemory,
    temperature: LevelEncoder,
    thermal_mass: LevelEncoder,
    specific_heat: LevelEncoder,
    conductivity: LevelEncoder,
    energy: LevelEncoder,
    unit: LevelEncoder,
}

impl ThermalSemanticEncoder {
    pub fn new(config: ThermalSemanticConfig) -> Result<Self, PhysicsHdcError> {
        config.validate()?;
        let memory = ItemMemory::new(config.hdc.clone())?;
        Ok(Self {
            temperature: memory.scalar_encoder("physics.temperature-kelvin.v1"),
            thermal_mass: memory.scalar_encoder("physics.thermal-mass.v1"),
            specific_heat: memory.scalar_encoder("physics.specific-heat.v1"),
            conductivity: memory.scalar_encoder("physics.thermal-conductivity.v1"),
            energy: memory.scalar_encoder("physics.thermal-energy.v1"),
            unit: memory.scalar_encoder("physics.unit-interval.v1"),
            memory,
            config,
        })
    }

    pub fn from_base(encoder: &PhysicsFrameEncoder) -> Result<Self, PhysicsHdcError> {
        Self::new(ThermalSemanticConfig::from_base(encoder))
    }

    pub fn config(&self) -> &ThermalSemanticConfig {
        &self.config
    }

    pub fn composite_fingerprint(&self, base: &PhysicsFrameEncoder) -> u64 {
        let mut hash = StableHash64::with_seed(base.config().fingerprint());
        hash.write(b"symtropy-thermal-composite-encoder-v1");
        hash.write_u64(self.config.fingerprint());
        hash.finish()
    }

    /// Encode exact physics state plus authoritative thermodynamic state.
    ///
    /// Semantic encoding fails closed when the source world contains non-finite
    /// body state, an invalid thermal reservoir, incomplete modeled thermal
    /// accounting, or a non-finite modeled energy total. Invalid state is never
    /// silently mapped onto a benign semantic value such as zero.
    pub fn encode_world<const D: usize>(
        &self,
        tick: u64,
        world: &PhysicsWorld<D>,
        base: &PhysicsFrameEncoder,
    ) -> Result<ThermalEncodedPhysicsFrame<D>, PhysicsHdcError> {
        if self.config.hdc != base.config().hdc {
            return Err(PhysicsHdcError::InvalidConfig(
                "thermal overlay and base encoder must share the same HDC spec".to_owned(),
            ));
        }
        self.validate_source_world(world)?;

        let base_frame = base.encode_world(tick, world)?;
        self.combine_base_frame(world, base, base_frame)
    }

    fn validate_source_world<const D: usize>(
        &self,
        world: &PhysicsWorld<D>,
    ) -> Result<(), PhysicsHdcError> {
        for body in &world.bodies {
            if let Some(thermal) = body.thermal {
                thermal.validate().map_err(|error| {
                    PhysicsHdcError::InvalidConfig(format!(
                        "thermal semantic source body {:?} is invalid: {error:?}",
                        body.handle
                    ))
                })?;
            }
        }

        let invariants = world.invariant_snapshot();
        if invariants.non_finite_body_count != 0 {
            return Err(PhysicsHdcError::InvalidConfig(format!(
                "thermal semantic source contains {} non-finite bodies",
                invariants.non_finite_body_count
            )));
        }
        if !invariants.has_complete_modeled_energy_accounting() {
            return Err(PhysicsHdcError::InvalidConfig(format!(
                "thermal semantic source has incomplete modeled energy accounting: {} invalid thermal reservoirs",
                invariants.invalid_thermal_body_count
            )));
        }
        if !invariants.modeled_thermal_energy.is_finite()
            || !invariants.modeled_total_energy.is_finite()
        {
            return Err(PhysicsHdcError::InvalidConfig(
                "thermal semantic source has non-finite modeled energy totals".to_owned(),
            ));
        }
        Ok(())
    }

    fn combine_base_frame<const D: usize>(
        &self,
        world: &PhysicsWorld<D>,
        base: &PhysicsFrameEncoder,
        base_frame: EncodedPhysicsFrame<D>,
    ) -> Result<ThermalEncodedPhysicsFrame<D>, PhysicsHdcError> {
        let mut bundle = BipolarBundle::new(self.config.hdc.dimension);
        bundle.add(&base_frame.vector)?;
        bundle.add(&self.memory.item("record-kind", "thermal-overlay"))?;

        let mut thermal_body_count = 0usize;
        let mut bodies: Vec<_> = world.bodies.iter().collect();
        bodies.sort_by_key(|body| body.handle);
        for body in bodies {
            let Some(thermal) = body.thermal else {
                continue;
            };
            thermal_body_count += 1;
            bundle.add(&self.encode_thermal_body(
                base.config().identity_policy,
                body.handle.0 as u64,
                body.net_id.map(|net_id| net_id.0),
                thermal,
            )?)?;
        }

        let invariants = world.invariant_snapshot();
        // `encode_world` already established completeness, but re-check the
        // totals here so this helper never silently encodes non-finite values if
        // its call structure changes later.
        bundle.add(&self.bind_scalar(
            "modeled-thermal-energy",
            &self.energy,
            invariants.modeled_thermal_energy,
            0.0,
            self.config.energy_extent_joules,
        )?)?;
        bundle.add(&self.bind_scalar(
            "modeled-total-energy",
            &self.energy,
            invariants.modeled_total_energy,
            -self.config.energy_extent_joules,
            self.config.energy_extent_joules,
        )?)?;

        let vector = bundle.finish(&self.memory.tie_breaker("thermal-composite-frame"))?;
        Ok(ThermalEncodedPhysicsFrame {
            tick: base_frame.tick,
            encoder_fingerprint: self.composite_fingerprint(base),
            exact_digest: exact_world_digest_v2(world),
            reference_origin: base_frame.reference_origin,
            body_count: base_frame.body_count,
            thermal_body_count,
            collision_count: base_frame.collision_count,
            vector,
        })
    }

    fn encode_thermal_body(
        &self,
        identity_policy: IdentityPolicy,
        body_handle: u64,
        net_id: Option<u64>,
        thermal: symtropy_physics::ThermalBody,
    ) -> Result<BipolarHV, PhysicsHdcError> {
        thermal.validate().map_err(|error| {
            PhysicsHdcError::InvalidConfig(format!(
                "thermal semantic source body {body_handle} is invalid: {error:?}"
            ))
        })?;

        let mut bundle = BipolarBundle::new(self.config.hdc.dimension);
        bundle.add(&self.memory.item("record-kind", "thermal-body"))?;

        if let Some(identity) = thermal_identity(identity_policy, body_handle, net_id) {
            bundle.add(&self.bind_item("identity", "body-identity", &identity)?)?;
        }

        bundle.add(&self.bind_scalar(
            "temperature-kelvin",
            &self.temperature,
            thermal.state.temperature_kelvin,
            0.0,
            self.config.temperature_max_kelvin,
        )?)?;
        bundle.add(&self.bind_scalar(
            "thermal-mass-kg",
            &self.thermal_mass,
            thermal.thermal_mass_kg,
            0.0,
            self.config.thermal_mass_extent_kg,
        )?)?;
        bundle.add(&self.bind_scalar(
            "specific-heat-capacity",
            &self.specific_heat,
            thermal.material.specific_heat_capacity,
            0.0,
            self.config.specific_heat_capacity_extent,
        )?)?;
        bundle.add(&self.bind_scalar(
            "thermal-conductivity",
            &self.conductivity,
            thermal.material.thermal_conductivity,
            0.0,
            self.config.thermal_conductivity_extent,
        )?)?;
        bundle.add(&self.bind_scalar(
            "emissivity",
            &self.unit,
            thermal.material.emissivity,
            0.0,
            1.0,
        )?)?;
        bundle
            .finish(&self.memory.tie_breaker("thermal-body-record"))
            .map_err(Into::into)
    }

    fn bind_item(
        &self,
        role: &str,
        namespace: &str,
        value: &str,
    ) -> Result<BipolarHV, PhysicsHdcError> {
        self.memory
            .role(role)
            .bind(&self.memory.item(namespace, value))
            .map_err(Into::into)
    }

    fn bind_scalar(
        &self,
        role: &str,
        encoder: &LevelEncoder,
        value: f64,
        min: f64,
        max: f64,
    ) -> Result<BipolarHV, PhysicsHdcError> {
        if !value.is_finite() {
            return Err(PhysicsHdcError::InvalidConfig(format!(
                "thermal semantic source `{role}` must be finite"
            )));
        }
        self.memory
            .role(role)
            .bind(&encoder.encode(value, min, max)?)
            .map_err(Into::into)
    }
}

fn thermal_identity(
    policy: IdentityPolicy,
    body_handle: u64,
    net_id: Option<u64>,
) -> Option<String> {
    match policy {
        IdentityPolicy::None => None,
        IdentityPolicy::Handle => Some(format!("handle:{body_handle}")),
        IdentityPolicy::NetIdPreferred => Some(match net_id {
            Some(net_id) => format!("net:{net_id}"),
            None => format!("handle:{body_handle}"),
        }),
    }
}

/// Exact-state digest v2.
///
/// Version 1 already covers mechanics, contacts, events, solver parameters and
/// collision shapes. Version 2 binds that digest to every body's optional
/// thermal state, so a temperature/material/presence change is an exact bitwise
/// provenance change without redefining the established v1 contract.
///
/// The digest intentionally remains defined for invalid state so failures can
/// still be identified reproducibly. It is **not** a validity certificate and
/// does not explain the lifecycle transition between two digests.
pub fn exact_world_digest_v2<const D: usize>(world: &PhysicsWorld<D>) -> ExactStateDigest {
    let v1 = crate::exact_world_digest(world);
    let mut low = StableHash64::with_seed(0x5448_4552_4d41_4c32);
    let mut high = StableHash64::with_seed(0x5359_4d54_524f_5032);
    hash_v2_thermal(world, v1, &mut low);
    hash_v2_thermal(world, v1, &mut high);
    ExactStateDigest {
        algorithm_version: 2,
        low: low.finish(),
        high: high.finish(),
    }
}

fn hash_v2_thermal<const D: usize>(
    world: &PhysicsWorld<D>,
    v1: ExactStateDigest,
    hash: &mut StableHash64,
) {
    hash.write_u32(v1.algorithm_version);
    hash.write_u64(v1.low);
    hash.write_u64(v1.high);
    let mut bodies: Vec<_> = world.bodies.iter().collect();
    bodies.sort_by_key(|body| body.handle);
    hash.write_u64(bodies.len() as u64);
    for body in bodies {
        hash.write_u64(body.handle.0 as u64);
        match body.thermal {
            None => hash.write_u32(0),
            Some(thermal) => {
                hash.write_u32(1);
                hash.write_u64(thermal.state.temperature_kelvin.to_bits());
                hash.write_u64(thermal.thermal_mass_kg.to_bits());
                hash.write_u64(thermal.material.specific_heat_capacity.to_bits());
                hash.write_u64(thermal.material.thermal_conductivity.to_bits());
                hash.write_u64(thermal.material.emissivity.to_bits());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;
    use symtropy_physics::{ThermalBody, ThermalMaterial, ThermalState};

    fn thermal_body(temp_k: f64) -> ThermalBody {
        ThermalBody::new(
            ThermalMaterial::new(900.0, 20.0, 0.7).unwrap(),
            ThermalState::new(temp_k).unwrap(),
            2.0,
        )
        .unwrap()
    }

    fn base_encoder() -> PhysicsFrameEncoder {
        let mut config = crate::PhysicsEncoderConfig::default();
        config.hdc.dimension = 4_096;
        config.hdc.scalar_levels = 129;
        PhysicsFrameEncoder::new(config).unwrap()
    }

    #[test]
    fn v2_digest_changes_when_only_temperature_changes() {
        let mut world = PhysicsWorld::<3>::default();
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world
            .body_mut(handle)
            .unwrap()
            .set_thermal(thermal_body(300.0));
        let before = exact_world_digest_v2(&world);
        world
            .body_mut(handle)
            .unwrap()
            .thermal
            .as_mut()
            .unwrap()
            .state
            .temperature_kelvin = 301.0;
        let after = exact_world_digest_v2(&world);
        assert_ne!(before, after);
    }

    #[test]
    fn v1_can_match_while_v2_detects_thermal_difference() {
        let mut cold = PhysicsWorld::<3>::default();
        let c = cold.add_sphere(Point::origin(), 0.5, 1.0);
        cold.body_mut(c).unwrap().set_thermal(thermal_body(300.0));

        let mut hot = PhysicsWorld::<3>::default();
        let h = hot.add_sphere(Point::origin(), 0.5, 1.0);
        hot.body_mut(h).unwrap().set_thermal(thermal_body(600.0));

        assert_eq!(
            crate::exact_world_digest(&cold),
            crate::exact_world_digest(&hot)
        );
        assert_ne!(exact_world_digest_v2(&cold), exact_world_digest_v2(&hot));
    }

    #[test]
    fn semantic_vector_changes_with_thermal_state() {
        let base = base_encoder();
        let overlay = ThermalSemanticEncoder::from_base(&base).unwrap();

        let mut cold = PhysicsWorld::<3>::default();
        let c = cold.add_sphere(Point::origin(), 0.5, 1.0);
        cold.body_mut(c).unwrap().set_thermal(thermal_body(300.0));

        let mut hot = PhysicsWorld::<3>::default();
        let h = hot.add_sphere(Point::origin(), 0.5, 1.0);
        hot.body_mut(h).unwrap().set_thermal(thermal_body(900.0));

        let a = overlay.encode_world(1, &cold, &base).unwrap();
        let b = overlay.encode_world(1, &hot, &base).unwrap();
        assert_ne!(a.vector, b.vector);
        assert_ne!(a.exact_digest, b.exact_digest);
        assert_eq!(a.thermal_body_count, 1);
        assert_eq!(b.thermal_body_count, 1);
    }

    #[test]
    fn identity_none_does_not_reintroduce_transient_handles() {
        let mut base_config = crate::PhysicsEncoderConfig::default();
        base_config.hdc.dimension = 4_096;
        base_config.hdc.scalar_levels = 129;
        base_config.identity_policy = IdentityPolicy::None;
        let base = PhysicsFrameEncoder::new(base_config).unwrap();
        let overlay = ThermalSemanticEncoder::from_base(&base).unwrap();

        let mut a = PhysicsWorld::<3>::default();
        let a_handle = a.add_sphere(Point::origin(), 0.5, 1.0);
        a.body_mut(a_handle).unwrap().set_thermal(thermal_body(350.0));

        let mut b = PhysicsWorld::<3>::default();
        let b_handle = b.add_sphere(Point::origin(), 0.5, 1.0);
        b.body_mut(b_handle).unwrap().set_thermal(thermal_body(350.0));
        b.body_mut(b_handle).unwrap().handle = symtropy_physics::BodyHandle(99);

        let encoded_a = overlay.encode_world(2, &a, &base).unwrap();
        let encoded_b = overlay.encode_world(2, &b, &base).unwrap();
        assert_eq!(encoded_a.vector, encoded_b.vector);
        assert_ne!(encoded_a.exact_digest, encoded_b.exact_digest);
    }

    #[test]
    fn thermal_encoding_does_not_mutate_world() {
        let base = base_encoder();
        let overlay = ThermalSemanticEncoder::from_base(&base).unwrap();
        let mut world = PhysicsWorld::<3>::default();
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world
            .body_mut(handle)
            .unwrap()
            .set_thermal(thermal_body(450.0));
        let before = exact_world_digest_v2(&world);
        let _ = overlay.encode_world(9, &world, &base).unwrap();
        let after = exact_world_digest_v2(&world);
        assert_eq!(before, after);
    }

    #[test]
    fn invalid_thermal_source_is_rejected_but_still_digestible_for_provenance() {
        let base = base_encoder();
        let overlay = ThermalSemanticEncoder::from_base(&base).unwrap();
        let mut world = PhysicsWorld::<3>::default();
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world
            .body_mut(handle)
            .unwrap()
            .set_thermal(thermal_body(300.0));
        world
            .body_mut(handle)
            .unwrap()
            .thermal
            .as_mut()
            .unwrap()
            .material
            .specific_heat_capacity = -1.0;

        let digest = exact_world_digest_v2(&world);
        assert_eq!(digest.algorithm_version, 2);
        assert!(matches!(
            overlay.encode_world(1, &world, &base),
            Err(PhysicsHdcError::InvalidConfig(_))
        ));
    }

    #[test]
    fn non_finite_thermal_source_is_not_coerced_to_zero() {
        let base = base_encoder();
        let overlay = ThermalSemanticEncoder::from_base(&base).unwrap();
        let mut world = PhysicsWorld::<3>::default();
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world
            .body_mut(handle)
            .unwrap()
            .set_thermal(thermal_body(300.0));
        world
            .body_mut(handle)
            .unwrap()
            .thermal
            .as_mut()
            .unwrap()
            .state
            .temperature_kelvin = f64::NAN;

        assert!(matches!(
            overlay.encode_world(1, &world, &base),
            Err(PhysicsHdcError::InvalidConfig(_))
        ));
    }
}
