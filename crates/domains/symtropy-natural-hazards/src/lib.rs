// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-aware contracts for multi-hazard model qualification.
//!
//! This crate deliberately does not implement a universal hazard solver. It
//! provides shared metadata and claim gates while each physical domain keeps
//! its own equations, observables, and applicable conservation laws.

#![forbid(unsafe_code)]

pub mod cascade;
pub mod scoring;

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Broad physical or environmental family used for cataloguing hazard models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardFamily {
    Geophysical,
    Hydrological,
    Meteorological,
    Climatological,
    Extraterrestrial,
    SpaceWeather,
    Biological,
    Environmental,
}

/// A named hazard process; a profile must explicitly declare which kinds it supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardKind {
    Earthquake,
    Tsunami,
    VolcanicEruption,
    Landslide,
    DebrisFlow,
    Avalanche,
    Sinkhole,
    Flood,
    StormSurge,
    Drought,
    CoastalErosion,
    TropicalCyclone,
    SevereConvectiveStorm,
    Tornado,
    Hailstorm,
    Lightning,
    WinterStorm,
    Heatwave,
    ColdWave,
    Wildfire,
    DustStorm,
    AsteroidCometImpact,
    SolarFlare,
    GeomagneticStorm,
    Epidemic,
    HarmfulAlgalBloom,
    CropPestOutbreak,
}

impl HazardKind {
    /// Return the catalogue family. This classification does not assert model support.
    pub const fn family(self) -> HazardFamily {
        match self {
            Self::Earthquake
            | Self::VolcanicEruption
            | Self::Landslide
            | Self::DebrisFlow
            | Self::Avalanche
            | Self::Sinkhole => HazardFamily::Geophysical,
            Self::Tsunami | Self::Flood | Self::StormSurge => HazardFamily::Hydrological,
            Self::TropicalCyclone
            | Self::SevereConvectiveStorm
            | Self::Tornado
            | Self::Hailstorm
            | Self::Lightning
            | Self::WinterStorm
            | Self::DustStorm => HazardFamily::Meteorological,
            Self::Drought
            | Self::CoastalErosion
            | Self::Heatwave
            | Self::ColdWave
            | Self::Wildfire => HazardFamily::Climatological,
            Self::AsteroidCometImpact => HazardFamily::Extraterrestrial,
            Self::SolarFlare | Self::GeomagneticStorm => HazardFamily::SpaceWeather,
            Self::Epidemic => HazardFamily::Biological,
            Self::HarmfulAlgalBloom | Self::CropPestOutbreak => HazardFamily::Environmental,
        }
    }
}

/// Independent evidence classes. They are intentionally not ordered into one score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    StructuralInvariant,
    AnalyticalOrManufactured,
    PublishedBenchmark,
    ReferenceDifferential,
    HeldOutObservation,
    OperationalSkill,
    PresentationQuality,
}

/// State of one explicitly identified evidence suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Pass,
    Fail,
    Unavailable,
    NotApplicable,
}

/// One evidence result for a model profile and a named suite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub class: EvidenceClass,
    pub status: EvidenceStatus,
    pub suite_id: String,
    pub source_uri: Option<String>,
    pub detail: String,
}

/// A model card with an explicit validity envelope and input/output contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardModelProfile {
    pub profile_id: String,
    pub model_name: String,
    pub model_revision: String,
    pub hazards: BTreeSet<HazardKind>,
    /// Declared characteristic spatial scale in metres, not display pixel size.
    pub min_spatial_scale_m: f64,
    pub max_spatial_scale_m: f64,
    /// Declared model timestep in seconds.
    pub min_timestep_s: f64,
    pub max_timestep_s: f64,
    pub required_inputs: BTreeSet<String>,
    pub declared_outputs: BTreeSet<String>,
    pub exclusions: Vec<String>,
    pub evidence: Vec<EvidenceRecord>,
}

/// Errors that make a model profile malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileValidationError {
    EmptyProfileId,
    EmptyModelName,
    EmptyModelRevision,
    EmptyHazardSet,
    InvalidSpatialEnvelope,
    InvalidTimestepEnvelope,
    EmptyRequiredInput,
    EmptyDeclaredOutput,
    EmptyEvidenceSuite,
    DuplicateEvidenceSuite {
        class: EvidenceClass,
        suite_id: String,
    },
}

impl HazardModelProfile {
    /// Validate structure only. A structurally valid profile is not thereby scientifically qualified.
    pub fn validate(&self) -> Result<(), ProfileValidationError> {
        if !nonblank(&self.profile_id) {
            return Err(ProfileValidationError::EmptyProfileId);
        }
        if !nonblank(&self.model_name) {
            return Err(ProfileValidationError::EmptyModelName);
        }
        if !nonblank(&self.model_revision) {
            return Err(ProfileValidationError::EmptyModelRevision);
        }
        if self.hazards.is_empty() {
            return Err(ProfileValidationError::EmptyHazardSet);
        }
        if !self.min_spatial_scale_m.is_finite()
            || !self.max_spatial_scale_m.is_finite()
            || self.min_spatial_scale_m <= 0.0
            || self.max_spatial_scale_m < self.min_spatial_scale_m
        {
            return Err(ProfileValidationError::InvalidSpatialEnvelope);
        }
        if !self.min_timestep_s.is_finite()
            || !self.max_timestep_s.is_finite()
            || self.min_timestep_s <= 0.0
            || self.max_timestep_s < self.min_timestep_s
        {
            return Err(ProfileValidationError::InvalidTimestepEnvelope);
        }
        if self.required_inputs.iter().any(|name| !nonblank(name)) {
            return Err(ProfileValidationError::EmptyRequiredInput);
        }
        if self.declared_outputs.iter().any(|name| !nonblank(name)) {
            return Err(ProfileValidationError::EmptyDeclaredOutput);
        }

        let mut evidence_suites = BTreeSet::new();
        for record in &self.evidence {
            if !nonblank(&record.suite_id) {
                return Err(ProfileValidationError::EmptyEvidenceSuite);
            }
            let suite_key = (record.class, record.suite_id.clone());
            if !evidence_suites.insert(suite_key) {
                return Err(ProfileValidationError::DuplicateEvidenceSuite {
                    class: record.class,
                    suite_id: record.suite_id.clone(),
                });
            }
        }
        Ok(())
    }

    /// Return the first reason a scenario is outside this profile's declared scope.
    pub fn check_scope(&self, scope: &HazardScope) -> Result<(), ScopeFailure> {
        self.validate().map_err(ScopeFailure::InvalidProfile)?;
        if !self.hazards.contains(&scope.hazard) {
            return Err(ScopeFailure::UnsupportedHazard(scope.hazard));
        }
        if !scope.spatial_scale_m.is_finite() || scope.spatial_scale_m <= 0.0 {
            return Err(ScopeFailure::InvalidSpatialScale);
        }
        if scope.spatial_scale_m < self.min_spatial_scale_m
            || scope.spatial_scale_m > self.max_spatial_scale_m
        {
            return Err(ScopeFailure::OutOfSpatialEnvelope);
        }
        if !scope.timestep_s.is_finite() || scope.timestep_s <= 0.0 {
            return Err(ScopeFailure::InvalidTimestep);
        }
        if scope.timestep_s < self.min_timestep_s || scope.timestep_s > self.max_timestep_s {
            return Err(ScopeFailure::OutOfTimestepEnvelope);
        }
        if let Some(missing) = self
            .required_inputs
            .iter()
            .find(|required| !scope.available_inputs.contains(*required))
        {
            return Err(ScopeFailure::MissingRequiredInput(missing.clone()));
        }
        Ok(())
    }

    /// Require an exact evidence-class PASS. One class never implies another.
    pub fn qualification_decision(
        &self,
        scope: &HazardScope,
        required_class: EvidenceClass,
    ) -> QualificationDecision {
        if let Err(reason) = self.check_scope(scope) {
            return QualificationDecision::OutOfScope(reason);
        }
        let matching: Vec<_> = self
            .evidence
            .iter()
            .filter(|record| record.class == required_class)
            .collect();
        if matching.is_empty() {
            return QualificationDecision::MissingEvidence(required_class);
        }
        if matching
            .iter()
            .any(|record| record.status == EvidenceStatus::Fail)
        {
            return QualificationDecision::FailedEvidence(required_class);
        }
        if matching
            .iter()
            .any(|record| record.status == EvidenceStatus::Unavailable)
        {
            return QualificationDecision::UnavailableEvidence(required_class);
        }
        if matching
            .iter()
            .any(|record| record.status == EvidenceStatus::NotApplicable)
        {
            return QualificationDecision::NotApplicable(required_class);
        }
        QualificationDecision::Qualified
    }
}

/// A requested scenario point inside or outside a model profile's declared envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardScope {
    pub hazard: HazardKind,
    pub spatial_scale_m: f64,
    pub timestep_s: f64,
    pub available_inputs: BTreeSet<String>,
}

/// Why a proposed scenario cannot be claimed within a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeFailure {
    InvalidProfile(ProfileValidationError),
    UnsupportedHazard(HazardKind),
    InvalidSpatialScale,
    OutOfSpatialEnvelope,
    InvalidTimestep,
    OutOfTimestepEnvelope,
    MissingRequiredInput(String),
}

/// Qualification is one decision for one profile, scope and evidence class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualificationDecision {
    Qualified,
    OutOfScope(ScopeFailure),
    MissingEvidence(EvidenceClass),
    FailedEvidence(EvidenceClass),
    UnavailableEvidence(EvidenceClass),
    NotApplicable(EvidenceClass),
}

/// A declared residual for a domain-specific ledger, such as water, salt, mass or energy.
/// It is optional because not every hazard has the same conserved quantities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetResidual {
    pub quantity: String,
    pub residual: f64,
    pub tolerance: f64,
    pub units: String,
}

impl BudgetResidual {
    /// A budget closes only when its name/units and finite residual/tolerance are valid.
    pub fn closes(&self) -> bool {
        nonblank(&self.quantity)
            && nonblank(&self.units)
            && self.residual.is_finite()
            && self.tolerance.is_finite()
            && self.tolerance >= 0.0
            && self.residual.abs() <= self.tolerance
    }
}

/// Execution outcome. A PASS does not by itself establish empirical predictive skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pass,
    Fail,
    Unavailable,
}

/// Immutable receipt metadata for one execution. Domain-specific outputs remain in model-owned schemas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HazardRunReceipt {
    pub run_id: String,
    pub scenario_digest_sha256: String,
    pub source_revision: String,
    pub model_profile_id: String,
    pub input_digest_sha256: String,
    pub output_digest_sha256: Option<String>,
    pub backend: String,
    pub status: RunStatus,
    pub budget_residuals: Vec<BudgetResidual>,
    pub warnings: Vec<String>,
    pub unavailable_outputs: Vec<String>,
}

/// Errors that make a run receipt incomplete or internally inconsistent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptValidationError {
    EmptyRunId,
    EmptySourceRevision,
    EmptyProfileId,
    EmptyBackend,
    InvalidScenarioDigest,
    InvalidInputDigest,
    InvalidOutputDigest,
    PassingRunMissingOutputDigest,
    DeclaredBudgetDoesNotClose,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl HazardRunReceipt {
    /// Validate reproducibility metadata and basic status consistency.
    pub fn validate(&self) -> Result<(), ReceiptValidationError> {
        if !nonblank(&self.run_id) {
            return Err(ReceiptValidationError::EmptyRunId);
        }
        if !is_sha256(&self.scenario_digest_sha256) {
            return Err(ReceiptValidationError::InvalidScenarioDigest);
        }
        if !nonblank(&self.source_revision) {
            return Err(ReceiptValidationError::EmptySourceRevision);
        }
        if !nonblank(&self.model_profile_id) {
            return Err(ReceiptValidationError::EmptyProfileId);
        }
        if !is_sha256(&self.input_digest_sha256) {
            return Err(ReceiptValidationError::InvalidInputDigest);
        }
        if let Some(digest) = &self.output_digest_sha256
            && !is_sha256(digest)
        {
            return Err(ReceiptValidationError::InvalidOutputDigest);
        }
        if self.status == RunStatus::Pass && self.output_digest_sha256.is_none() {
            return Err(ReceiptValidationError::PassingRunMissingOutputDigest);
        }
        if self.status == RunStatus::Pass
            && !self.budget_residuals.is_empty()
            && !self.all_declared_budgets_close()
        {
            return Err(ReceiptValidationError::DeclaredBudgetDoesNotClose);
        }
        if !nonblank(&self.backend) {
            return Err(ReceiptValidationError::EmptyBackend);
        }
        Ok(())
    }

    /// Returns true only when at least one declared domain-specific budget exists and all close.
    pub fn all_declared_budgets_close(&self) -> bool {
        !self.budget_residuals.is_empty()
            && self.budget_residuals.iter().all(BudgetResidual::closes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> HazardModelProfile {
        HazardModelProfile {
            profile_id: "flood.reference.v1".into(),
            model_name: "Synthetic shallow-water reference".into(),
            model_revision: "0123456789abcdef".into(),
            hazards: [HazardKind::Flood].into_iter().collect(),
            min_spatial_scale_m: 1.0,
            max_spatial_scale_m: 1_000.0,
            min_timestep_s: 0.001,
            max_timestep_s: 10.0,
            required_inputs: ["terrain".into(), "boundary_conditions".into()]
                .into_iter()
                .collect(),
            declared_outputs: ["water_depth".into(), "velocity".into()]
                .into_iter()
                .collect(),
            exclusions: vec!["No claim for fully 3D turbulence".into()],
            evidence: vec![EvidenceRecord {
                class: EvidenceClass::StructuralInvariant,
                status: EvidenceStatus::Pass,
                suite_id: "closed-basin-budget-v1".into(),
                source_uri: None,
                detail: "Synthetic closed-basin control only".into(),
            }],
        }
    }

    fn scope() -> HazardScope {
        HazardScope {
            hazard: HazardKind::Flood,
            spatial_scale_m: 10.0,
            timestep_s: 0.1,
            available_inputs: ["terrain".into(), "boundary_conditions".into()]
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn hazard_catalogue_classifies_families_without_claiming_support() {
        assert_eq!(HazardKind::Earthquake.family(), HazardFamily::Geophysical);
        assert_eq!(HazardKind::Flood.family(), HazardFamily::Hydrological);
        assert_eq!(HazardKind::Wildfire.family(), HazardFamily::Climatological);
        assert_eq!(
            HazardKind::AsteroidCometImpact.family(),
            HazardFamily::Extraterrestrial
        );
        assert_eq!(
            HazardKind::GeomagneticStorm.family(),
            HazardFamily::SpaceWeather
        );
        assert_eq!(HazardKind::Epidemic.family(), HazardFamily::Biological);
    }

    #[test]
    fn profile_structure_validates() {
        assert_eq!(profile().validate(), Ok(()));
    }

    #[test]
    fn malformed_or_non_finite_envelopes_are_rejected() {
        let mut invalid = profile();
        invalid.max_spatial_scale_m = f64::NAN;
        assert_eq!(
            invalid.validate(),
            Err(ProfileValidationError::InvalidSpatialEnvelope)
        );

        let mut invalid = profile();
        invalid.min_timestep_s = 0.0;
        assert_eq!(
            invalid.validate(),
            Err(ProfileValidationError::InvalidTimestepEnvelope)
        );
    }

    #[test]
    fn model_cannot_claim_an_unsupported_hazard_or_scale() {
        let model = profile();
        let mut unsupported = scope();
        unsupported.hazard = HazardKind::Tsunami;
        assert_eq!(
            model.check_scope(&unsupported),
            Err(ScopeFailure::UnsupportedHazard(HazardKind::Tsunami))
        );

        let mut out_of_range = scope();
        out_of_range.spatial_scale_m = 0.1;
        assert_eq!(
            model.check_scope(&out_of_range),
            Err(ScopeFailure::OutOfSpatialEnvelope)
        );
    }

    #[test]
    fn missing_required_inputs_fail_closed() {
        let model = profile();
        let mut incomplete = scope();
        incomplete.available_inputs.remove("terrain");
        assert_eq!(
            model.check_scope(&incomplete),
            Err(ScopeFailure::MissingRequiredInput("terrain".into()))
        );
    }

    #[test]
    fn passing_invariants_do_not_imply_held_out_observation_skill() {
        let model = profile();
        assert_eq!(
            model.qualification_decision(&scope(), EvidenceClass::HeldOutObservation),
            QualificationDecision::MissingEvidence(EvidenceClass::HeldOutObservation)
        );
        assert_eq!(
            model.qualification_decision(&scope(), EvidenceClass::StructuralInvariant),
            QualificationDecision::Qualified
        );
    }

    #[test]
    fn multiple_suites_in_one_evidence_class_are_supported_and_fail_closed() {
        let mut model = profile();
        model.evidence.push(EvidenceRecord {
            class: EvidenceClass::PublishedBenchmark,
            status: EvidenceStatus::Pass,
            suite_id: "dam-break-suite-a".into(),
            source_uri: None,
            detail: "First matched suite".into(),
        });
        model.evidence.push(EvidenceRecord {
            class: EvidenceClass::PublishedBenchmark,
            status: EvidenceStatus::Pass,
            suite_id: "dam-break-suite-b".into(),
            source_uri: None,
            detail: "Second matched suite".into(),
        });
        assert_eq!(
            model.qualification_decision(&scope(), EvidenceClass::PublishedBenchmark),
            QualificationDecision::Qualified
        );
        model.evidence[2].status = EvidenceStatus::Fail;
        assert_eq!(
            model.qualification_decision(&scope(), EvidenceClass::PublishedBenchmark),
            QualificationDecision::FailedEvidence(EvidenceClass::PublishedBenchmark)
        );
        assert_eq!(model.validate(), Ok(()));
    }

    #[test]
    fn failed_and_unavailable_evidence_are_not_passes() {
        let mut model = profile();
        model.evidence.push(EvidenceRecord {
            class: EvidenceClass::PublishedBenchmark,
            status: EvidenceStatus::Unavailable,
            suite_id: "published-dam-break-v1".into(),
            source_uri: None,
            detail: "Reference input is not available in this run".into(),
        });
        let decision = model.qualification_decision(&scope(), EvidenceClass::PublishedBenchmark);
        assert_eq!(
            decision,
            QualificationDecision::UnavailableEvidence(EvidenceClass::PublishedBenchmark)
        );
    }

    #[test]
    fn budget_residual_requires_units_and_numeric_closure() {
        let valid = BudgetResidual {
            quantity: "water_volume".into(),
            residual: 1.0e-8,
            tolerance: 1.0e-6,
            units: "m^3".into(),
        };
        assert!(valid.closes());

        let invalid = BudgetResidual {
            residual: f64::INFINITY,
            ..valid.clone()
        };
        assert!(!invalid.closes());

        let no_units = BudgetResidual {
            units: String::new(),
            ..valid
        };
        assert!(!no_units.closes());
    }

    #[test]
    fn passing_receipt_requires_a_reproducible_output_digest() {
        let receipt = HazardRunReceipt {
            run_id: "run-001".into(),
            scenario_digest_sha256: "a".repeat(64),
            source_revision: "0123456789abcdef".into(),
            model_profile_id: "flood.reference.v1".into(),
            input_digest_sha256: "b".repeat(64),
            output_digest_sha256: None,
            backend: "cpu-reference".into(),
            status: RunStatus::Pass,
            budget_residuals: vec![],
            warnings: vec![],
            unavailable_outputs: vec![],
        };
        assert_eq!(
            receipt.validate(),
            Err(ReceiptValidationError::PassingRunMissingOutputDigest)
        );
    }

    #[test]
    fn passing_receipt_rejects_a_failed_declared_budget() {
        let receipt = HazardRunReceipt {
            run_id: "run-budget-fail".into(),
            scenario_digest_sha256: "a".repeat(64),
            source_revision: "0123456789abcdef".into(),
            model_profile_id: "flood.reference.v1".into(),
            input_digest_sha256: "b".repeat(64),
            output_digest_sha256: Some("c".repeat(64)),
            backend: "cpu-reference".into(),
            status: RunStatus::Pass,
            budget_residuals: vec![BudgetResidual {
                quantity: "water_volume".into(),
                residual: 10.0,
                tolerance: 0.01,
                units: "m^3".into(),
            }],
            warnings: vec![],
            unavailable_outputs: vec![],
        };
        assert_eq!(
            receipt.validate(),
            Err(ReceiptValidationError::DeclaredBudgetDoesNotClose)
        );
    }

    #[test]
    fn no_declared_budget_is_not_misreported_as_budget_pass() {
        let receipt = HazardRunReceipt {
            run_id: "run-002".into(),
            scenario_digest_sha256: "a".repeat(64),
            source_revision: "0123456789abcdef".into(),
            model_profile_id: "flood.reference.v1".into(),
            input_digest_sha256: "b".repeat(64),
            output_digest_sha256: Some("c".repeat(64)),
            backend: "cpu-reference".into(),
            status: RunStatus::Pass,
            budget_residuals: vec![],
            warnings: vec![],
            unavailable_outputs: vec![],
        };
        assert!(!receipt.all_declared_budgets_close());
        assert_eq!(receipt.validate(), Ok(()));
    }
}
