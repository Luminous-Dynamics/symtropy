// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Dependency-light planetary forcing and assumption identity primitives.
//!
//! PLANET-01 deliberately owns model/forcing semantics only. It does not own
//! climate, geology, hydrology, biosphere, habitability, life, or civilization
//! truth. Missing inputs remain explicit; Earth defaults are never invented.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const PLANET_SCHEMA_VERSION: u32 = 1;
const AUTHORITY_DOMAIN: &[u8] = b"symtropy:planet-forcing-authority:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EvidenceClass {
    EmpiricalEarth,
    EmpiricalPlanetary,
    ValidatedModel,
    GroundedExtrapolation,
    SpeculativeHypothesis,
    FictionalExtension,
}

impl EvidenceClass {
    pub fn rank(self) -> u8 {
        match self {
            Self::EmpiricalEarth => 0,
            Self::EmpiricalPlanetary => 1,
            Self::ValidatedModel => 2,
            Self::GroundedExtrapolation => 3,
            Self::SpeculativeHypothesis => 4,
            Self::FictionalExtension => 5,
        }
    }

    pub fn weakest(a: Self, b: Self) -> Self {
        if a.rank() >= b.rank() { a } else { b }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ModelRef {
    pub namespace: String,
    pub model_id: String,
    pub version: String,
    pub evidence_class: EvidenceClass,
}

impl ModelRef {
    pub fn new(
        namespace: impl Into<String>,
        model_id: impl Into<String>,
        version: impl Into<String>,
        evidence_class: EvidenceClass,
    ) -> Result<Self, PlanetError> {
        let value = Self {
            namespace: namespace.into(),
            model_id: model_id.into(),
            version: version.into(),
            evidence_class,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), PlanetError> {
        validate_text("namespace", &self.namespace)?;
        validate_text("model_id", &self.model_id)?;
        validate_text("version", &self.version)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BulkBodyParameters {
    pub mass_kg: f64,
    pub mean_radius_m: f64,
}

impl BulkBodyParameters {
    pub fn new(mass_kg: f64, mean_radius_m: f64) -> Result<Self, PlanetError> {
        let value = Self {
            mass_kg,
            mean_radius_m,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), PlanetError> {
        validate_positive_finite("mass_kg", self.mass_kg)?;
        validate_positive_finite("mean_radius_m", self.mean_radius_m)?;
        Ok(())
    }

    /// Surface gravity from GM/r^2 using the fixed SI 2018 CODATA value of G.
    /// This is a deterministic derived convenience value, not an orbital solver.
    pub fn surface_gravity_m_s2(&self) -> f64 {
        const G: f64 = 6.674_30e-11;
        G * self.mass_kg / (self.mean_radius_m * self.mean_radius_m)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotationProfile {
    /// Sidereal rotation period in SI seconds. Negative values are not used to
    /// encode retrograde rotation; orientation belongs to an orbital provider.
    pub period_s: f64,
    /// Axial obliquity in degrees [0, 180].
    pub obliquity_deg: f64,
    /// Explicitly records synchronous/tidally locked treatment when modeled.
    pub synchronous: bool,
}

impl RotationProfile {
    pub fn validate(&self) -> Result<(), PlanetError> {
        validate_positive_finite("rotation.period_s", self.period_s)?;
        validate_finite("rotation.obliquity_deg", self.obliquity_deg)?;
        if !(0.0..=180.0).contains(&self.obliquity_deg) {
            return Err(PlanetError::OutOfRange {
                field: "rotation.obliquity_deg",
                min: 0.0,
                max: 180.0,
                observed: self.obliquity_deg,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbitalProfile {
    /// Semimajor axis in metres for a Keplerian-style reference profile.
    pub semimajor_axis_m: f64,
    /// Eccentricity in [0, 1). Hyperbolic trajectories are outside PLANET-01.
    pub eccentricity: f64,
    pub rotation: RotationProfile,
    /// Exact model/provider identity that interprets the orbital parameters.
    pub model: ModelRef,
}

impl OrbitalProfile {
    pub fn validate(&self) -> Result<(), PlanetError> {
        validate_positive_finite("orbit.semimajor_axis_m", self.semimajor_axis_m)?;
        validate_finite("orbit.eccentricity", self.eccentricity)?;
        if !(0.0..1.0).contains(&self.eccentricity) {
            return Err(PlanetError::OutOfRange {
                field: "orbit.eccentricity",
                min: 0.0,
                max: 1.0,
                observed: self.eccentricity,
            });
        }
        self.rotation.validate()?;
        self.model.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StellarForcingProfile {
    /// Bolometric luminosity in watts when explicitly modeled.
    pub luminosity_w: Option<f64>,
    /// Optional effective temperature in kelvin; no value is inferred when absent.
    pub effective_temperature_k: Option<f64>,
    /// Model/provider identity for stellar spectrum/activity interpretation.
    pub model: ModelRef,
}

impl StellarForcingProfile {
    pub fn validate(&self) -> Result<(), PlanetError> {
        if let Some(value) = self.luminosity_w {
            validate_positive_finite("stellar.luminosity_w", value)?;
        }
        if let Some(value) = self.effective_temperature_k {
            validate_positive_finite("stellar.effective_temperature_k", value)?;
        }
        self.model.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetAssumptionManifest {
    pub schema_version: u32,
    /// Stable world/body identifier supplied by a higher-level body identity owner.
    pub body_ref: String,
    pub bulk: BulkBodyParameters,
    pub stellar: StellarForcingProfile,
    pub orbit: OrbitalProfile,
    /// Optional domain model refs. Absence means unknown/unmodeled, not Earth-like.
    pub atmosphere_model: Option<ModelRef>,
    pub ocean_model: Option<ModelRef>,
    pub interior_model: Option<ModelRef>,
    /// Extensible exact assumptions, canonicalized by key.
    pub assumptions: BTreeMap<String, String>,
}

impl PlanetAssumptionManifest {
    pub fn new(
        body_ref: impl Into<String>,
        bulk: BulkBodyParameters,
        stellar: StellarForcingProfile,
        orbit: OrbitalProfile,
    ) -> Result<Self, PlanetError> {
        let value = Self {
            schema_version: PLANET_SCHEMA_VERSION,
            body_ref: body_ref.into(),
            bulk,
            stellar,
            orbit,
            atmosphere_model: None,
            ocean_model: None,
            interior_model: None,
            assumptions: BTreeMap::new(),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), PlanetError> {
        if self.schema_version != PLANET_SCHEMA_VERSION {
            return Err(PlanetError::UnsupportedSchema(self.schema_version));
        }
        validate_text("body_ref", &self.body_ref)?;
        self.bulk.validate()?;
        self.stellar.validate()?;
        self.orbit.validate()?;
        for model in [
            self.atmosphere_model.as_ref(),
            self.ocean_model.as_ref(),
            self.interior_model.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            model.validate()?;
        }
        for (key, value) in &self.assumptions {
            validate_text("assumption.key", key)?;
            validate_text("assumption.value", value)?;
        }
        Ok(())
    }

    pub fn weakest_evidence_class(&self) -> EvidenceClass {
        let mut weakest = EvidenceClass::EmpiricalEarth;
        for model in std::iter::once(&self.stellar.model)
            .chain(std::iter::once(&self.orbit.model))
            .chain(self.atmosphere_model.iter())
            .chain(self.ocean_model.iter())
            .chain(self.interior_model.iter())
        {
            weakest = EvidenceClass::weakest(weakest, model.evidence_class);
        }
        weakest
    }

    pub fn authority_stamp(&self) -> Result<PlanetForcingAuthorityStamp, PlanetError> {
        self.validate()?;
        let mut digest = Sha256::new();
        digest.update(AUTHORITY_DOMAIN);
        put_u32(&mut digest, self.schema_version);
        put_text(&mut digest, &self.body_ref);
        put_f64(&mut digest, self.bulk.mass_kg);
        put_f64(&mut digest, self.bulk.mean_radius_m);
        put_optional_f64(&mut digest, self.stellar.luminosity_w);
        put_optional_f64(&mut digest, self.stellar.effective_temperature_k);
        put_model(&mut digest, &self.stellar.model);
        put_f64(&mut digest, self.orbit.semimajor_axis_m);
        put_f64(&mut digest, self.orbit.eccentricity);
        put_f64(&mut digest, self.orbit.rotation.period_s);
        put_f64(&mut digest, self.orbit.rotation.obliquity_deg);
        digest.update([u8::from(self.orbit.rotation.synchronous)]);
        put_model(&mut digest, &self.orbit.model);
        put_optional_model(&mut digest, self.atmosphere_model.as_ref());
        put_optional_model(&mut digest, self.ocean_model.as_ref());
        put_optional_model(&mut digest, self.interior_model.as_ref());
        put_u64(&mut digest, self.assumptions.len() as u64);
        for (key, value) in &self.assumptions {
            put_text(&mut digest, key);
            put_text(&mut digest, value);
        }
        Ok(PlanetForcingAuthorityStamp(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanetForcingAuthorityStamp(pub [u8; 32]);

impl fmt::Debug for PlanetForcingAuthorityStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PlanetForcingAuthorityStamp({})", self)
    }
}

impl fmt::Display for PlanetForcingAuthorityStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlanetError {
    UnsupportedSchema(u32),
    EmptyText { field: &'static str },
    NonFinite { field: &'static str, observed: f64 },
    NonPositive { field: &'static str, observed: f64 },
    OutOfRange {
        field: &'static str,
        min: f64,
        max: f64,
        observed: f64,
    },
}

impl fmt::Display for PlanetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => write!(f, "unsupported planet schema {version}"),
            Self::EmptyText { field } => write!(f, "{field} must not be empty"),
            Self::NonFinite { field, observed } => {
                write!(f, "{field} must be finite, observed {observed}")
            }
            Self::NonPositive { field, observed } => {
                write!(f, "{field} must be positive, observed {observed}")
            }
            Self::OutOfRange {
                field,
                min,
                max,
                observed,
            } => write!(f, "{field} must be in [{min}, {max}], observed {observed}"),
        }
    }
}

impl Error for PlanetError {}

fn validate_text(field: &'static str, value: &str) -> Result<(), PlanetError> {
    if value.trim().is_empty() {
        Err(PlanetError::EmptyText { field })
    } else {
        Ok(())
    }
}

fn validate_finite(field: &'static str, value: f64) -> Result<(), PlanetError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(PlanetError::NonFinite {
            field,
            observed: value,
        })
    }
}

fn validate_positive_finite(field: &'static str, value: f64) -> Result<(), PlanetError> {
    validate_finite(field, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(PlanetError::NonPositive {
            field,
            observed: value,
        })
    }
}

fn put_u32(digest: &mut Sha256, value: u32) {
    digest.update(value.to_le_bytes());
}

fn put_u64(digest: &mut Sha256, value: u64) {
    digest.update(value.to_le_bytes());
}

fn put_f64(digest: &mut Sha256, value: f64) {
    digest.update(value.to_bits().to_le_bytes());
}

fn put_text(digest: &mut Sha256, value: &str) {
    put_u64(digest, value.len() as u64);
    digest.update(value.as_bytes());
}

fn put_optional_f64(digest: &mut Sha256, value: Option<f64>) {
    match value {
        Some(value) => {
            digest.update([1]);
            put_f64(digest, value);
        }
        None => digest.update([0]),
    }
}

fn put_model(digest: &mut Sha256, model: &ModelRef) {
    put_text(digest, &model.namespace);
    put_text(digest, &model.model_id);
    put_text(digest, &model.version);
    digest.update([model.evidence_class.rank()]);
}

fn put_optional_model(digest: &mut Sha256, model: Option<&ModelRef>) {
    match model {
        Some(model) => {
            digest.update([1]);
            put_model(digest, model);
        }
        None => digest.update([0]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_manifest() -> PlanetAssumptionManifest {
        let bulk = BulkBodyParameters::new(5.9722e24, 6_371_000.0).expect("valid body");
        let stellar = StellarForcingProfile {
            luminosity_w: Some(3.828e26),
            effective_temperature_k: Some(5772.0),
            model: ModelRef::new(
                "stellar",
                "reference-star",
                "v1",
                EvidenceClass::EmpiricalPlanetary,
            )
            .expect("valid stellar model"),
        };
        let orbit = OrbitalProfile {
            semimajor_axis_m: 149_597_870_700.0,
            eccentricity: 0.0167,
            rotation: RotationProfile {
                period_s: 86_164.0905,
                obliquity_deg: 23.4393,
                synchronous: false,
            },
            model: ModelRef::new(
                "orbit",
                "kepler-reference",
                "v1",
                EvidenceClass::ValidatedModel,
            )
            .expect("valid orbit model"),
        };
        PlanetAssumptionManifest::new("sol:earth", bulk, stellar, orbit).expect("valid manifest")
    }

    #[test]
    fn earth_reference_gravity_is_reasonable() {
        let gravity = reference_manifest().bulk.surface_gravity_m_s2();
        assert!((gravity - 9.82).abs() < 0.05);
    }

    #[test]
    fn invalid_orbit_rejects() {
        let mut manifest = reference_manifest();
        manifest.orbit.eccentricity = 1.0;
        assert!(matches!(
            manifest.validate(),
            Err(PlanetError::OutOfRange {
                field: "orbit.eccentricity",
                ..
            })
        ));
    }

    #[test]
    fn missing_optional_domains_remain_missing() {
        let manifest = reference_manifest();
        assert!(manifest.atmosphere_model.is_none());
        assert!(manifest.ocean_model.is_none());
        assert!(manifest.interior_model.is_none());
    }

    #[test]
    fn assumption_order_does_not_change_authority() {
        let mut a = reference_manifest();
        a.assumptions.insert("alpha".into(), "1".into());
        a.assumptions.insert("beta".into(), "2".into());

        let mut b = reference_manifest();
        b.assumptions.insert("beta".into(), "2".into());
        b.assumptions.insert("alpha".into(), "1".into());

        assert_eq!(
            a.authority_stamp().expect("authority"),
            b.authority_stamp().expect("authority")
        );
    }

    #[test]
    fn model_version_changes_authority() {
        let a = reference_manifest();
        let mut b = reference_manifest();
        b.orbit.model.version = "v2".into();
        assert_ne!(
            a.authority_stamp().expect("authority"),
            b.authority_stamp().expect("authority")
        );
    }

    #[test]
    fn weaker_model_propagates_to_manifest() {
        let mut manifest = reference_manifest();
        manifest.atmosphere_model = Some(
            ModelRef::new(
                "atmosphere",
                "speculative-profile",
                "v0",
                EvidenceClass::SpeculativeHypothesis,
            )
            .expect("model"),
        );
        assert_eq!(
            manifest.weakest_evidence_class(),
            EvidenceClass::SpeculativeHypothesis
        );
    }

    #[test]
    fn fictional_extension_cannot_be_hidden_by_stronger_models() {
        assert_eq!(
            EvidenceClass::weakest(
                EvidenceClass::ValidatedModel,
                EvidenceClass::FictionalExtension
            ),
            EvidenceClass::FictionalExtension
        );
    }
}
