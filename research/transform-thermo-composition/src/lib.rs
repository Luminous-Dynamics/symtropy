//! Composition verifier for physical transformation, energy and entropy receipts.
//!
//! This crate does not calculate physical transformations, energy balances,
//! thermal entropy, exergy or material mass. It validates cross-owner receipt
//! identity and the internal consistency of the supplied summaries. The
//! upstream owners remain authoritative for their own calculations.
//!
//! A valid digest string is only a reference token here. This crate does not
//! hash source bytes, authenticate evidence, establish measurements or prove
//! the physical completeness of a declared boundary.

use std::collections::BTreeSet;
use std::fmt;

const NUMERIC_CLOSURE_ABS_TOLERANCE: f64 = 1.0e-12;

/// Evidence class copied from an owner receipt; composition never promotes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    Synthetic,
    Derived,
    ModelEstimate,
    Measured,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRef {
    pub evidence_id: String,
    pub sha256: String,
    pub class: EvidenceClass,
}

/// Requirements describe which ledgers are applicable to this event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialRequirement {
    Required,
    Optional,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntropyRequirement {
    Required,
    Optional,
    NotApplicable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransformationEvent {
    pub event_id: String,
    pub world_id: String,
    pub predecessor_revision: u64,
    pub successor_revision: u64,
    pub boundary_id: String,
    pub interval_id: String,
    pub interval_start_ns: u64,
    pub interval_end_ns: u64,
    pub material_requirement: MaterialRequirement,
    pub entropy_requirement: EntropyRequirement,
}

/// Summary of the material owner result. Mass is not recomputed here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialDisposition {
    Balanced,
    Imbalanced,
    Incomplete,
    NotApplicable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialReceipt {
    pub event_id: String,
    pub world_id: String,
    pub predecessor_revision: u64,
    pub successor_revision: u64,
    pub disposition: MaterialDisposition,
    /// Exact integer-milligram residual from the upstream mass ledger.
    /// Some(0) is an explicitly reconciled zero; None is unknown/incomplete.
    pub residual_mass_mg: Option<i128>,
    pub artifact: EvidenceRef,
}

/// Status vocabulary mirrors the open-system balance owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnergyBalanceStatus {
    Balanced,
    Imbalanced,
    Inconclusive,
    Incomplete,
}

/// Summary of an upstream first-law report. No energy equation is recomputed.
#[derive(Clone, Debug, PartialEq)]
pub struct EnergyBalanceReceipt {
    pub event_id: String,
    pub world_id: String,
    pub predecessor_revision: u64,
    pub successor_revision: u64,
    pub boundary_id: String,
    pub interval_id: String,
    pub status: EnergyBalanceStatus,
    /// Conservative residual interval in joules.
    pub residual_interval_j: Option<(f64, f64)>,
    /// Allowed absolute residual in joules.
    pub allowed_residual_j: Option<f64>,
    pub flow_count: Option<usize>,
    pub reservoir_ids: Option<Vec<String>>,
    pub artifact: EvidenceRef,
}

/// Supported thermal calculation profile in the current reference owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntropyProfile {
    ConstantCapacityBodyFixedTemperatureBath,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntropyDisposition {
    Satisfied,
    Violated,
    Incomplete,
    NotEvaluable,
    NotApplicable,
}

/// Summary of a supported upstream entropy calculation.
#[derive(Clone, Debug, PartialEq)]
pub struct EntropyReceipt {
    pub event_id: String,
    pub world_id: String,
    pub predecessor_revision: u64,
    pub successor_revision: u64,
    pub boundary_id: String,
    pub interval_id: String,
    pub profile: Option<EntropyProfile>,
    pub disposition: EntropyDisposition,
    pub body_delta_s_j_per_k: Option<f64>,
    pub bath_delta_s_j_per_k: Option<f64>,
    pub total_delta_s_j_per_k: Option<f64>,
    pub absolute_tolerance_j_per_k: Option<f64>,
    pub reason: Option<String>,
    pub artifact: Option<EvidenceRef>,
}

/// Optional exergy summary. It is admissible only when bound to the exact
/// entropy receipt and its supported body-plus-bath entropy-generation result.
#[derive(Clone, Debug, PartialEq)]
pub struct ExergyReceipt {
    pub event_id: String,
    pub world_id: String,
    pub predecessor_revision: u64,
    pub successor_revision: u64,
    pub boundary_id: String,
    pub interval_id: String,
    pub environment_profile_id: String,
    pub reference_temperature_k: f64,
    pub entropy_generation_j_per_k: f64,
    pub exergy_destroyed_j: f64,
    pub entropy_artifact_id: String,
    pub artifact: EvidenceRef,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompositionManifest {
    pub event: TransformationEvent,
    pub material: MaterialReceipt,
    pub energy: EnergyBalanceReceipt,
    pub entropy: EntropyReceipt,
    pub exergy: Option<ExergyReceipt>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExergyDisposition {
    NotRequested,
    ValidatedForDeclaredReference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverallDisposition {
    QualifiedForDeclaredProfile,
    Rejected,
    Incomplete,
    Inconclusive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionReport {
    pub event_id: String,
    pub world_id: String,
    pub predecessor_revision: u64,
    pub successor_revision: u64,
    pub material: MaterialDisposition,
    pub energy: EnergyBalanceStatus,
    pub entropy: EntropyDisposition,
    pub exergy: ExergyDisposition,
    pub overall: OverallDisposition,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionError {
    EmptyIdentity(&'static str),
    InvalidRevisionSequence,
    InvalidInterval,
    ReceiptIdentityMismatch(&'static str),
    BoundaryMismatch(&'static str),
    IntervalMismatch(&'static str),
    InvalidDigest(&'static str),
    DuplicateEvidenceId,
    MaterialStatusMismatch,
    MaterialResidualMismatch,
    EnergyStatusMismatch,
    InvalidEnergySummary,
    EnergyReservoirMismatch,
    EntropyRequirementMismatch,
    InvalidEntropySummary,
    EntropyClosureMismatch,
    EntropyStatusMismatch,
    InvalidExergySummary,
    ExergySourceMismatch,
    NumericOverflow,
}

impl fmt::Display for CompositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CompositionError {}

impl CompositionManifest {
    /// Validate owner receipt composition without rerunning any owner model.
    pub fn qualify(&self) -> Result<CompositionReport, CompositionError> {
        validate_event(&self.event)?;
        validate_receipt_identity(
            &self.event,
            &self.material.event_id,
            &self.material.world_id,
            self.material.predecessor_revision,
            self.material.successor_revision,
            "material",
        )?;
        validate_receipt_identity(
            &self.event,
            &self.energy.event_id,
            &self.energy.world_id,
            self.energy.predecessor_revision,
            self.energy.successor_revision,
            "energy",
        )?;
        validate_receipt_identity(
            &self.event,
            &self.entropy.event_id,
            &self.entropy.world_id,
            self.entropy.predecessor_revision,
            self.entropy.successor_revision,
            "entropy",
        )?;
        if self.energy.boundary_id != self.event.boundary_id {
            return Err(CompositionError::BoundaryMismatch("energy"));
        }
        if self.entropy.boundary_id != self.event.boundary_id {
            return Err(CompositionError::BoundaryMismatch("entropy"));
        }
        if self.energy.interval_id != self.event.interval_id {
            return Err(CompositionError::IntervalMismatch("energy"));
        }
        if self.entropy.interval_id != self.event.interval_id {
            return Err(CompositionError::IntervalMismatch("entropy"));
        }

        validate_material(&self.event, &self.material)?;
        validate_energy(&self.energy)?;
        validate_entropy(&self.event, &self.entropy)?;

        let mut evidence = vec![self.material.artifact.clone(), self.energy.artifact.clone()];
        if let Some(entropy_artifact) = &self.entropy.artifact {
            evidence.push(entropy_artifact.clone());
        }
        if let Some(exergy) = &self.exergy {
            validate_receipt_identity(
                &self.event,
                &exergy.event_id,
                &exergy.world_id,
                exergy.predecessor_revision,
                exergy.successor_revision,
                "exergy",
            )?;
            if exergy.boundary_id != self.event.boundary_id {
                return Err(CompositionError::BoundaryMismatch("exergy"));
            }
            if exergy.interval_id != self.event.interval_id {
                return Err(CompositionError::IntervalMismatch("exergy"));
            }
            validate_exergy(exergy, &self.entropy)?;
            evidence.push(exergy.artifact.clone());
        }
        let evidence_ids = validate_unique_evidence(&evidence)?;

        let overall = overall_disposition(
            self.event.material_requirement,
            self.event.entropy_requirement,
            self.material.disposition,
            self.energy.status,
            self.entropy.disposition,
        );

        Ok(CompositionReport {
            event_id: self.event.event_id.clone(),
            world_id: self.event.world_id.clone(),
            predecessor_revision: self.event.predecessor_revision,
            successor_revision: self.event.successor_revision,
            material: self.material.disposition,
            energy: self.energy.status,
            entropy: self.entropy.disposition,
            exergy: if self.exergy.is_some() {
                ExergyDisposition::ValidatedForDeclaredReference
            } else {
                ExergyDisposition::NotRequested
            },
            overall,
            evidence_ids,
        })
    }
}

fn validate_event(event: &TransformationEvent) -> Result<(), CompositionError> {
    for (value, field) in [
        (event.event_id.as_str(), "event_id"),
        (event.world_id.as_str(), "world_id"),
        (event.boundary_id.as_str(), "boundary_id"),
        (event.interval_id.as_str(), "interval_id"),
    ] {
        if value.trim().is_empty() {
            return Err(CompositionError::EmptyIdentity(field));
        }
    }
    let expected = event
        .predecessor_revision
        .checked_add(1)
        .ok_or(CompositionError::InvalidRevisionSequence)?;
    if event.successor_revision != expected {
        return Err(CompositionError::InvalidRevisionSequence);
    }
    if event.interval_start_ns >= event.interval_end_ns {
        return Err(CompositionError::InvalidInterval);
    }
    Ok(())
}

fn validate_receipt_identity(
    event: &TransformationEvent,
    event_id: &str,
    world_id: &str,
    predecessor_revision: u64,
    successor_revision: u64,
    role: &'static str,
) -> Result<(), CompositionError> {
    if event.event_id != event_id
        || event.world_id != world_id
        || event.predecessor_revision != predecessor_revision
        || event.successor_revision != successor_revision
    {
        return Err(CompositionError::ReceiptIdentityMismatch(role));
    }
    Ok(())
}

fn validate_material(
    event: &TransformationEvent,
    receipt: &MaterialReceipt,
) -> Result<(), CompositionError> {
    validate_evidence(&receipt.artifact, "material")?;
    match (
        event.material_requirement,
        receipt.disposition,
        receipt.residual_mass_mg,
    ) {
        (MaterialRequirement::NotApplicable, MaterialDisposition::NotApplicable, None) => Ok(()),
        (MaterialRequirement::NotApplicable, _, _) => Err(CompositionError::MaterialStatusMismatch),
        (_, MaterialDisposition::Balanced, Some(0)) => Ok(()),
        (_, MaterialDisposition::Imbalanced, Some(value)) if value != 0 => Ok(()),
        (_, MaterialDisposition::Incomplete, None) => Ok(()),
        (_, MaterialDisposition::NotApplicable, None)
            if event.material_requirement == MaterialRequirement::Optional =>
        {
            Ok(())
        }
        _ => Err(CompositionError::MaterialResidualMismatch),
    }
}

fn validate_energy(receipt: &EnergyBalanceReceipt) -> Result<(), CompositionError> {
    validate_evidence(&receipt.artifact, "energy")?;
    match (
        receipt.status,
        receipt.residual_interval_j,
        receipt.allowed_residual_j,
    ) {
        (EnergyBalanceStatus::Incomplete, None, None) => {
            if receipt.flow_count.is_some() || receipt.reservoir_ids.is_some() {
                return Err(CompositionError::InvalidEnergySummary);
            }
            Ok(())
        }
        (status, Some((lower, upper)), Some(allowed)) => {
            if !lower.is_finite()
                || !upper.is_finite()
                || lower > upper
                || !allowed.is_finite()
                || allowed < 0.0
            {
                return Err(CompositionError::InvalidEnergySummary);
            }
            let inside = lower >= -allowed && upper <= allowed;
            let outside = lower > allowed || upper < -allowed;
            let consistent = match status {
                EnergyBalanceStatus::Balanced => inside,
                EnergyBalanceStatus::Imbalanced => outside,
                EnergyBalanceStatus::Inconclusive => !inside && !outside,
                EnergyBalanceStatus::Incomplete => false,
            };
            if !consistent {
                return Err(CompositionError::EnergyStatusMismatch);
            }
            let count = receipt
                .flow_count
                .ok_or(CompositionError::InvalidEnergySummary)?;
            let reservoirs = receipt
                .reservoir_ids
                .as_ref()
                .ok_or(CompositionError::InvalidEnergySummary)?;
            if reservoirs.iter().any(|r| r.trim().is_empty()) {
                return Err(CompositionError::EnergyReservoirMismatch);
            }
            let unique: BTreeSet<_> = reservoirs.iter().collect();
            if unique.len() != reservoirs.len() || reservoirs.len() > count {
                return Err(CompositionError::EnergyReservoirMismatch);
            }
            Ok(())
        }
        _ => Err(CompositionError::InvalidEnergySummary),
    }
}

fn validate_entropy(
    event: &TransformationEvent,
    receipt: &EntropyReceipt,
) -> Result<(), CompositionError> {
    match receipt.disposition {
        EntropyDisposition::Satisfied | EntropyDisposition::Violated => {
            let artifact = receipt
                .artifact
                .as_ref()
                .ok_or(CompositionError::InvalidEntropySummary)?;
            validate_evidence(artifact, "entropy")?;
            if receipt.profile.is_none() {
                return Err(CompositionError::InvalidEntropySummary);
            }
            let body = receipt
                .body_delta_s_j_per_k
                .ok_or(CompositionError::InvalidEntropySummary)?;
            let bath = receipt
                .bath_delta_s_j_per_k
                .ok_or(CompositionError::InvalidEntropySummary)?;
            let total = receipt
                .total_delta_s_j_per_k
                .ok_or(CompositionError::InvalidEntropySummary)?;
            let tolerance = receipt
                .absolute_tolerance_j_per_k
                .ok_or(CompositionError::InvalidEntropySummary)?;
            if !body.is_finite()
                || !bath.is_finite()
                || !total.is_finite()
                || !tolerance.is_finite()
                || tolerance < 0.0
            {
                return Err(CompositionError::InvalidEntropySummary);
            }
            if (body + bath - total).abs() > tolerance.max(NUMERIC_CLOSURE_ABS_TOLERANCE) {
                return Err(CompositionError::EntropyClosureMismatch);
            }
            let status_matches = match receipt.disposition {
                EntropyDisposition::Satisfied => total >= -tolerance,
                EntropyDisposition::Violated => total < -tolerance,
                _ => unreachable!(),
            };
            if !status_matches {
                return Err(CompositionError::EntropyStatusMismatch);
            }
            if event.entropy_requirement == EntropyRequirement::NotApplicable {
                return Err(CompositionError::EntropyRequirementMismatch);
            }
            Ok(())
        }
        EntropyDisposition::Incomplete | EntropyDisposition::NotEvaluable => {
            if receipt.body_delta_s_j_per_k.is_some()
                || receipt.bath_delta_s_j_per_k.is_some()
                || receipt.total_delta_s_j_per_k.is_some()
                || receipt.absolute_tolerance_j_per_k.is_some()
            {
                return Err(CompositionError::InvalidEntropySummary);
            }
            if event.entropy_requirement == EntropyRequirement::NotApplicable {
                return Err(CompositionError::EntropyRequirementMismatch);
            }
            if let Some(artifact) = &receipt.artifact {
                validate_evidence(artifact, "entropy")?;
            }
            if receipt.reason.as_deref().is_none_or(str::is_empty) {
                return Err(CompositionError::InvalidEntropySummary);
            }
            Ok(())
        }
        EntropyDisposition::NotApplicable => {
            if event.entropy_requirement == EntropyRequirement::Required
                || receipt.profile.is_some()
                || receipt.body_delta_s_j_per_k.is_some()
                || receipt.bath_delta_s_j_per_k.is_some()
                || receipt.total_delta_s_j_per_k.is_some()
                || receipt.absolute_tolerance_j_per_k.is_some()
                || receipt.artifact.is_some()
            {
                return Err(CompositionError::EntropyRequirementMismatch);
            }
            Ok(())
        }
    }
}

fn validate_exergy(
    receipt: &ExergyReceipt,
    entropy: &EntropyReceipt,
) -> Result<(), CompositionError> {
    validate_evidence(&receipt.artifact, "exergy")?;
    if receipt.environment_profile_id.trim().is_empty()
        || !receipt.reference_temperature_k.is_finite()
        || receipt.reference_temperature_k <= 0.0
        || !receipt.entropy_generation_j_per_k.is_finite()
        || receipt.entropy_generation_j_per_k < 0.0
        || !receipt.exergy_destroyed_j.is_finite()
        || receipt.exergy_destroyed_j < 0.0
    {
        return Err(CompositionError::InvalidExergySummary);
    }
    if entropy.disposition != EntropyDisposition::Satisfied {
        return Err(CompositionError::ExergySourceMismatch);
    }
    let entropy_artifact = entropy
        .artifact
        .as_ref()
        .ok_or(CompositionError::ExergySourceMismatch)?;
    if receipt.entropy_artifact_id != entropy_artifact.evidence_id {
        return Err(CompositionError::ExergySourceMismatch);
    }
    let total_entropy = entropy
        .total_delta_s_j_per_k
        .ok_or(CompositionError::ExergySourceMismatch)?;
    if !values_close(total_entropy, receipt.entropy_generation_j_per_k)
        || !values_close(
            receipt.reference_temperature_k * receipt.entropy_generation_j_per_k,
            receipt.exergy_destroyed_j,
        )
    {
        return Err(CompositionError::InvalidExergySummary);
    }
    Ok(())
}

fn validate_evidence(evidence: &EvidenceRef, role: &'static str) -> Result<(), CompositionError> {
    if evidence.evidence_id.trim().is_empty() {
        return Err(CompositionError::EmptyIdentity(role));
    }
    if evidence.sha256.len() != 64 || !evidence.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(CompositionError::InvalidDigest(role));
    }
    Ok(())
}

fn validate_unique_evidence(evidence: &[EvidenceRef]) -> Result<Vec<String>, CompositionError> {
    let mut ids = BTreeSet::new();
    for item in evidence {
        if !ids.insert(item.evidence_id.as_str()) {
            return Err(CompositionError::DuplicateEvidenceId);
        }
    }
    Ok(ids.into_iter().map(str::to_owned).collect())
}

fn values_close(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs()
            <= NUMERIC_CLOSURE_ABS_TOLERANCE
                .max(NUMERIC_CLOSURE_ABS_TOLERANCE * left.abs().max(right.abs()))
}

fn overall_disposition(
    material_requirement: MaterialRequirement,
    entropy_requirement: EntropyRequirement,
    material: MaterialDisposition,
    energy: EnergyBalanceStatus,
    entropy: EntropyDisposition,
) -> OverallDisposition {
    match (material_requirement, material) {
        (MaterialRequirement::NotApplicable, MaterialDisposition::NotApplicable)
        | (MaterialRequirement::Optional, MaterialDisposition::NotApplicable)
        | (_, MaterialDisposition::Balanced) => {}
        (_, MaterialDisposition::Imbalanced) => return OverallDisposition::Rejected,
        (_, MaterialDisposition::Incomplete)
        | (MaterialRequirement::Required, MaterialDisposition::NotApplicable) => {
            return OverallDisposition::Incomplete;
        }
    }

    match energy {
        EnergyBalanceStatus::Imbalanced => return OverallDisposition::Rejected,
        EnergyBalanceStatus::Incomplete => return OverallDisposition::Incomplete,
        EnergyBalanceStatus::Inconclusive => return OverallDisposition::Inconclusive,
        EnergyBalanceStatus::Balanced => {}
    }

    match entropy {
        EntropyDisposition::Violated => return OverallDisposition::Rejected,
        EntropyDisposition::Incomplete | EntropyDisposition::NotEvaluable => {
            return OverallDisposition::Incomplete;
        }
        EntropyDisposition::Satisfied => {}
        EntropyDisposition::NotApplicable => {
            if entropy_requirement == EntropyRequirement::Required {
                return OverallDisposition::Incomplete;
            }
        }
    }

    OverallDisposition::QualifiedForDeclaredProfile
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(id: &str, marker: char, class: EvidenceClass) -> EvidenceRef {
        EvidenceRef {
            evidence_id: id.into(),
            sha256: std::iter::repeat_n(marker, 64).collect(),
            class,
        }
    }

    fn event(
        material_requirement: MaterialRequirement,
        entropy_requirement: EntropyRequirement,
    ) -> TransformationEvent {
        TransformationEvent {
            event_id: "transform-001".into(),
            world_id: "world-001".into(),
            predecessor_revision: 4,
            successor_revision: 5,
            boundary_id: "assembly-cell-001".into(),
            interval_id: "interval-001".into(),
            interval_start_ns: 10,
            interval_end_ns: 20,
            material_requirement,
            entropy_requirement,
        }
    }

    fn balanced_material() -> MaterialReceipt {
        MaterialReceipt {
            event_id: "transform-001".into(),
            world_id: "world-001".into(),
            predecessor_revision: 4,
            successor_revision: 5,
            disposition: MaterialDisposition::Balanced,
            residual_mass_mg: Some(0),
            artifact: artifact("material-ledger-001", 'a', EvidenceClass::Derived),
        }
    }

    fn balanced_energy() -> EnergyBalanceReceipt {
        EnergyBalanceReceipt {
            event_id: "transform-001".into(),
            world_id: "world-001".into(),
            predecessor_revision: 4,
            successor_revision: 5,
            boundary_id: "assembly-cell-001".into(),
            interval_id: "interval-001".into(),
            status: EnergyBalanceStatus::Balanced,
            residual_interval_j: Some((-0.1, 0.1)),
            allowed_residual_j: Some(0.2),
            flow_count: Some(2),
            reservoir_ids: Some(vec!["grid".into(), "thermal-bath".into()]),
            artifact: artifact("energy-balance-001", 'b', EvidenceClass::Derived),
        }
    }

    fn satisfied_entropy() -> EntropyReceipt {
        EntropyReceipt {
            event_id: "transform-001".into(),
            world_id: "world-001".into(),
            predecessor_revision: 4,
            successor_revision: 5,
            boundary_id: "assembly-cell-001".into(),
            interval_id: "interval-001".into(),
            profile: Some(EntropyProfile::ConstantCapacityBodyFixedTemperatureBath),
            disposition: EntropyDisposition::Satisfied,
            body_delta_s_j_per_k: Some(0.4),
            bath_delta_s_j_per_k: Some(-0.3),
            total_delta_s_j_per_k: Some(0.1),
            absolute_tolerance_j_per_k: Some(1.0e-9),
            reason: None,
            artifact: Some(artifact("entropy-balance-001", 'c', EvidenceClass::Derived)),
        }
    }

    fn manifest() -> CompositionManifest {
        CompositionManifest {
            event: event(MaterialRequirement::Required, EntropyRequirement::Required),
            material: balanced_material(),
            energy: balanced_energy(),
            entropy: satisfied_entropy(),
            exergy: None,
        }
    }

    #[test]
    fn all_matching_owner_receipts_qualify_for_the_declared_profile() {
        let report = manifest().qualify().unwrap();
        assert_eq!(
            report.overall,
            OverallDisposition::QualifiedForDeclaredProfile
        );
        assert_eq!(report.material, MaterialDisposition::Balanced);
        assert_eq!(report.energy, EnergyBalanceStatus::Balanced);
        assert_eq!(report.entropy, EntropyDisposition::Satisfied);
        assert_eq!(report.evidence_ids.len(), 3);
    }

    #[test]
    fn wrong_material_revision_is_rejected() {
        let mut m = manifest();
        m.material.successor_revision = 6;
        assert_eq!(
            m.qualify(),
            Err(CompositionError::ReceiptIdentityMismatch("material"))
        );
    }

    #[test]
    fn wrong_energy_boundary_is_rejected() {
        let mut m = manifest();
        m.energy.boundary_id = "other-cell".into();
        assert_eq!(
            m.qualify(),
            Err(CompositionError::BoundaryMismatch("energy"))
        );
    }

    #[test]
    fn wrong_entropy_interval_is_rejected() {
        let mut m = manifest();
        m.entropy.interval_id = "other-interval".into();
        assert_eq!(
            m.qualify(),
            Err(CompositionError::IntervalMismatch("entropy"))
        );
    }

    #[test]
    fn balanced_energy_status_must_match_its_residual_interval() {
        let mut m = manifest();
        m.energy.residual_interval_j = Some((0.5, 0.6));
        assert_eq!(m.qualify(), Err(CompositionError::EnergyStatusMismatch));
    }

    #[test]
    fn energy_incomplete_cannot_carry_a_residual_interval() {
        let mut m = manifest();
        m.energy.status = EnergyBalanceStatus::Incomplete;
        assert_eq!(m.qualify(), Err(CompositionError::InvalidEnergySummary));
    }

    #[test]
    fn entropy_body_bath_terms_must_close_to_total() {
        let mut m = manifest();
        m.entropy.total_delta_s_j_per_k = Some(0.2);
        assert_eq!(m.qualify(), Err(CompositionError::EntropyClosureMismatch));
    }

    #[test]
    fn entropy_satisfied_status_must_match_the_total_and_tolerance() {
        let mut m = manifest();
        m.entropy.total_delta_s_j_per_k = Some(-0.1);
        m.entropy.body_delta_s_j_per_k = Some(0.2);
        m.entropy.bath_delta_s_j_per_k = Some(-0.3);
        assert_eq!(m.qualify(), Err(CompositionError::EntropyStatusMismatch));
    }

    #[test]
    fn entropy_violation_rejects_the_composition() {
        let mut m = manifest();
        m.entropy.disposition = EntropyDisposition::Violated;
        m.entropy.body_delta_s_j_per_k = Some(0.1);
        m.entropy.bath_delta_s_j_per_k = Some(-0.2);
        m.entropy.total_delta_s_j_per_k = Some(-0.1);
        let report = m.qualify().unwrap();
        assert_eq!(report.overall, OverallDisposition::Rejected);
    }

    #[test]
    fn required_but_not_evaluable_entropy_is_incomplete_not_a_pass() {
        let mut m = manifest();
        m.entropy.disposition = EntropyDisposition::NotEvaluable;
        m.entropy.profile = None;
        m.entropy.body_delta_s_j_per_k = None;
        m.entropy.bath_delta_s_j_per_k = None;
        m.entropy.total_delta_s_j_per_k = None;
        m.entropy.absolute_tolerance_j_per_k = None;
        m.entropy.reason = Some("phase transition outside constant-C profile".into());
        let report = m.qualify().unwrap();
        assert_eq!(report.overall, OverallDisposition::Incomplete);
    }

    #[test]
    fn explicitly_not_applicable_thermal_entropy_can_be_qualified_as_first_law_only() {
        let mut m = manifest();
        m.event.entropy_requirement = EntropyRequirement::NotApplicable;
        m.entropy.disposition = EntropyDisposition::NotApplicable;
        m.entropy.profile = None;
        m.entropy.body_delta_s_j_per_k = None;
        m.entropy.bath_delta_s_j_per_k = None;
        m.entropy.total_delta_s_j_per_k = None;
        m.entropy.absolute_tolerance_j_per_k = None;
        m.entropy.reason = None;
        m.entropy.artifact = None;
        let report = m.qualify().unwrap();
        assert_eq!(
            report.overall,
            OverallDisposition::QualifiedForDeclaredProfile
        );
        assert_eq!(report.exergy, ExergyDisposition::NotRequested);
    }

    #[test]
    fn material_imbalance_rejects_even_when_energy_is_balanced() {
        let mut m = manifest();
        m.material.disposition = MaterialDisposition::Imbalanced;
        m.material.residual_mass_mg = Some(-25);
        let report = m.qualify().unwrap();
        assert_eq!(report.overall, OverallDisposition::Rejected);
    }

    #[test]
    fn energy_imbalance_rejects_even_when_material_closes() {
        let mut m = manifest();
        m.energy.status = EnergyBalanceStatus::Imbalanced;
        m.energy.residual_interval_j = Some((1.0, 2.0));
        let report = m.qualify().unwrap();
        assert_eq!(report.overall, OverallDisposition::Rejected);
    }

    #[test]
    fn inconclusive_energy_remains_inconclusive() {
        let mut m = manifest();
        m.energy.status = EnergyBalanceStatus::Inconclusive;
        m.energy.residual_interval_j = Some((0.1, 0.4));
        let report = m.qualify().unwrap();
        assert_eq!(report.overall, OverallDisposition::Inconclusive);
    }

    #[test]
    fn exergy_must_match_the_same_entropy_artifact_and_temperature_reference() {
        let mut m = manifest();
        m.exergy = Some(ExergyReceipt {
            event_id: "transform-001".into(),
            world_id: "world-001".into(),
            predecessor_revision: 4,
            successor_revision: 5,
            boundary_id: "assembly-cell-001".into(),
            interval_id: "interval-001".into(),
            environment_profile_id: "reference-environment-300k".into(),
            reference_temperature_k: 300.0,
            entropy_generation_j_per_k: 0.1,
            exergy_destroyed_j: 30.0,
            entropy_artifact_id: "entropy-balance-001".into(),
            artifact: artifact("exergy-output-001", 'd', EvidenceClass::Derived),
        });
        let report = m.qualify().unwrap();
        assert_eq!(
            report.exergy,
            ExergyDisposition::ValidatedForDeclaredReference
        );

        m.exergy.as_mut().unwrap().entropy_artifact_id = "unrelated-entropy".into();
        assert_eq!(m.qualify(), Err(CompositionError::ExergySourceMismatch));
    }

    #[test]
    fn exergy_formula_closure_is_checked() {
        let mut m = manifest();
        m.exergy = Some(ExergyReceipt {
            event_id: "transform-001".into(),
            world_id: "world-001".into(),
            predecessor_revision: 4,
            successor_revision: 5,
            boundary_id: "assembly-cell-001".into(),
            interval_id: "interval-001".into(),
            environment_profile_id: "reference-environment-300k".into(),
            reference_temperature_k: 300.0,
            entropy_generation_j_per_k: 0.1,
            exergy_destroyed_j: 31.0,
            entropy_artifact_id: "entropy-balance-001".into(),
            artifact: artifact("exergy-output-001", 'd', EvidenceClass::Derived),
        });
        assert_eq!(m.qualify(), Err(CompositionError::InvalidExergySummary));
    }

    #[test]
    fn duplicate_artifact_identity_is_rejected() {
        let mut m = manifest();
        m.energy.artifact.evidence_id = m.material.artifact.evidence_id.clone();
        assert_eq!(m.qualify(), Err(CompositionError::DuplicateEvidenceId));
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let mut m = manifest();
        m.entropy.artifact.as_mut().unwrap().sha256 = "bad".into();
        assert_eq!(m.qualify(), Err(CompositionError::InvalidDigest("entropy")));
    }

    #[test]
    fn non_applicable_material_must_be_declared_by_the_event() {
        let mut m = manifest();
        m.material.disposition = MaterialDisposition::NotApplicable;
        m.material.residual_mass_mg = None;
        assert_eq!(m.qualify(), Err(CompositionError::MaterialStatusMismatch));
    }

    #[test]
    fn event_revision_must_advance_exactly_once() {
        let mut m = manifest();
        m.event.successor_revision = 7;
        assert_eq!(m.qualify(), Err(CompositionError::InvalidRevisionSequence));
    }

    #[test]
    fn missing_reservoir_inventory_prevents_energy_receipt_admission() {
        let mut m = manifest();
        m.energy.reservoir_ids = None;
        assert_eq!(m.qualify(), Err(CompositionError::InvalidEnergySummary));
    }

    #[test]
    fn identical_manifests_produce_identical_reports() {
        let a = manifest().qualify().unwrap();
        let b = manifest().qualify().unwrap();
        assert_eq!(a, b);
    }
}
