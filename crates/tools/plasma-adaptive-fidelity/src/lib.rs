//! Evidence-gated selection among adaptive plasma model fidelities.
//!
//! This is a pure policy evaluator. It does not derive or authenticate physical
//! qualification evidence, execute a solver, or implement model transfer.

use std::collections::BTreeSet;

pub const SCHEMA_VERSION: &str = "luminous.plasma-adaptive-fidelity.v0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCompatibility {
    pub physics_system_id: String,
    /// Pins the exact governing-equation/source revision.
    pub source_revision_sha256: String,
    /// Pins the exact solver build used by the evidence.
    pub solver_build_sha256: String,
    /// Canonical compatibility-contract identity supplied by the contract owner.
    pub compatibility_sha256: String,
    pub species_scope_id: String,
    pub collision_model_id: String,
    pub normalization_id: String,
    pub conserved_units_id: String,
    pub coordinate_frame_id: String,
    pub geometry_id: String,
    pub boundary_model_id: String,
    pub solver_interface_id: String,
    pub conserved_component_layout_id: String,
    pub interface_schema_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelKind {
    Moment {
        moment_family_id: String,
        basis_id: String,
        parallel_order: u16,
        perpendicular_order: u16,
        fidelity_rank: u16,
    },
    KineticPatch {
        kinetic_method_id: String,
        velocity_grid_sha256: String,
        fidelity_rank: u16,
    },
}

impl ModelKind {
    pub fn fidelity_rank(&self) -> u16 {
        match self {
            Self::Moment { fidelity_rank, .. }
            | Self::KineticPatch { fidelity_rank, .. } => *fidelity_rank,
        }
    }

    fn matches_well_posedness(&self, evidence: &WellPosednessEvidence) -> bool {
        matches!(
            (self, evidence),
            (Self::Moment { .. }, WellPosednessEvidence::MomentHyperbolicity { .. })
                | (Self::KineticPatch { .. }, WellPosednessEvidence::KineticAdmissibility { .. })
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Evidence<T> {
    Qualified(T),
    Unknown { reason: String },
    Rejected { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum WellPosednessEvidence {
    MomentHyperbolicity {
        profile_sha256: String,
        model_sha256: String,
        compatibility_sha256: String,
        receipt_sha256: String,
        envelope_sha256: String,
        /// Model-specific, normalized distance from the admitted
        /// hyperbolicity boundary. Its definition belongs to the profile.
        minimum_normalized_margin: f64,
        maximum_eigenbasis_condition: f64,
    },
    KineticAdmissibility {
        profile_sha256: String,
        model_sha256: String,
        compatibility_sha256: String,
        receipt_sha256: String,
        envelope_sha256: String,
        minimum_realizability_margin: f64,
    },
}

impl WellPosednessEvidence {
    fn receipt_sha256(&self) -> &str {
        match self {
            Self::MomentHyperbolicity { receipt_sha256, .. }
            | Self::KineticAdmissibility { receipt_sha256, .. } => receipt_sha256,
        }
    }

    fn digests_valid(&self) -> bool {
        match self {
            Self::MomentHyperbolicity {
                profile_sha256,
                model_sha256,
                compatibility_sha256,
                receipt_sha256,
                envelope_sha256,
                ..
            }
            | Self::KineticAdmissibility {
                profile_sha256,
                model_sha256,
                compatibility_sha256,
                receipt_sha256,
                envelope_sha256,
                ..
            } => {
                is_sha256(profile_sha256)
                    && is_sha256(model_sha256)
                    && is_sha256(compatibility_sha256)
                    && is_sha256(receipt_sha256)
                    && is_sha256(envelope_sha256)
            }
        }
    }

    fn identity_matches(
        &self,
        candidate: &ModelCandidate,
        compatibility: &ModelCompatibility,
    ) -> bool {
        match self {
            Self::MomentHyperbolicity {
                profile_sha256,
                model_sha256,
                compatibility_sha256,
                ..
            }
            | Self::KineticAdmissibility {
                profile_sha256,
                model_sha256,
                compatibility_sha256,
                ..
            } => {
                profile_sha256 == &candidate.profile_sha256
                    && model_sha256 == &candidate.model_sha256
                    && compatibility_sha256 == &compatibility.compatibility_sha256
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorBoundEvidence {
    /// Declared upper bound in error_units_id. Must come from a calibrated
    /// estimator for the exact physical model / validation domain.
    pub upper_bound: f64,
    pub error_units_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub compatibility_sha256: String,
    pub estimator_sha256: String,
    pub calibration_receipt_sha256: String,
    pub validation_domain_sha256: String,
    /// Empirical calibration coverage, not a universal confidence theorem.
    pub calibration_coverage: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionEvidence {
    pub from_profile_sha256: String,
    pub to_profile_sha256: String,
    pub from_model_sha256: String,
    pub to_model_sha256: String,
    pub interface_schema_sha256: String,
    pub projection_sha256: String,
    pub reconstruction_sha256: String,
    pub conservation_receipt_sha256: String,
    pub maximum_conservation_residual: f64,
    pub state_transfer_error_upper_bound: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelCandidate {
    pub profile_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub kind: ModelKind,
    pub compatibility: ModelCompatibility,
    pub well_posedness: Evidence<WellPosednessEvidence>,
    pub error_bound: Evidence<ErrorBoundEvidence>,
    /// Same declared cost unit for every candidate in one request.
    pub estimated_cost: f64,
    pub cost_units_id: String,
    /// Required when switching from a currently active model.
    pub transition: Option<TransitionEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptivePolicyRequest {
    pub schema_version: String,
    pub request_id: String,
    pub compatibility: ModelCompatibility,
    pub candidates: Vec<ModelCandidate>,
    pub current_profile_sha256: Option<String>,
    pub current_dwell_steps: u32,
    pub minimum_dwell_steps: u32,
    /// Maximum acceptable local model-error upper bound.
    pub error_tolerance: f64,
    /// Tighter bound required before coarsening away from the current model.
    /// Must be no larger than error_tolerance.
    pub coarsen_error_tolerance: f64,
    pub error_units_id: String,
    /// All costs in a request must use this exact accounting unit.
    pub cost_units_id: String,
    pub minimum_calibration_coverage: f64,
    pub minimum_hyperbolicity_margin: f64,
    pub maximum_eigenbasis_condition: f64,
    pub minimum_kinetic_realizability_margin: f64,
    pub maximum_transition_conservation_residual: f64,
    pub maximum_state_transfer_error: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    UnsupportedSchemaVersion,
    EmptyRequestIdentity,
    InvalidCompatibilityDigest,
    EmptyCompatibilityIdentity(&'static str),
    InvalidCurrentProfileDigest,
    NoCandidates,
    EmptyCandidateIdentity,
    InvalidCandidateDigest(&'static str),
    DuplicateProfile(String),
    InvalidCost,
    InvalidPolicyThreshold(&'static str),
    InvalidEvidenceReason(String),
    InvalidModelIdentity,
    InvalidEvidenceDigest(&'static str),
    InvalidEvidenceMetric(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExclusionReason {
    CompatibilityMismatch,
    WellPosednessUnknown,
    WellPosednessRejected,
    EvidenceKindMismatch,
    EvidenceIdentityMismatch,
    InvalidWellPosednessReceipt,
    HyperbolicityMarginTooSmall,
    EigenbasisConditionTooLarge,
    KineticRealizabilityMarginTooSmall,
    ErrorEvidenceUnknown,
    ErrorEvidenceRejected,
    InvalidErrorEvidence,
    ErrorUnitsMismatch,
    CostUnitsMismatch,
    CalibrationCoverageTooLow,
    ErrorBoundTooLarge,
    MissingTransitionEvidence,
    TransitionIdentityMismatch,
    InvalidTransitionEvidence,
    TransitionConservationResidualTooLarge,
    StateTransferErrorTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcludedCandidate {
    pub profile_sha256: String,
    pub reason: ExclusionReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionReason {
    InitialLowestCostWithinTolerance,
    RetainedInsideCoarseningBand,
    RetainedDuringMinimumDwell,
    AdaptedAfterMinimumDwell,
    EscalatedForErrorOrValidity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionReceipt {
    pub profile_id: String,
    pub profile_sha256: String,
    pub model_sha256: String,
    pub kind: ModelKind,
    pub fidelity_rank: u16,
    pub error_upper_bound: f64,
    pub estimated_cost: f64,
    pub estimator_sha256: String,
    pub calibration_receipt_sha256: String,
    pub well_posedness_receipt_sha256: String,
    pub transition: Option<TransitionEvidence>,
    pub reason: SelectionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownReason {
    OnlyUnresolvedCandidatesRemain,
    CurrentModelEvidenceUnresolved,
    CurrentModelProfileNotInCandidateSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalReason {
    NoCandidateMeetsTheErrorAndValidityEnvelope,
    NoSafeRefinementAvailable,
    NoTransitionQualifiedCandidate,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Selected {
        receipt: SelectionReceipt,
        excluded: Vec<ExcludedCandidate>,
    },
    Unknown {
        reason: UnknownReason,
        excluded: Vec<ExcludedCandidate>,
    },
    Refused {
        reason: RefusalReason,
        excluded: Vec<ExcludedCandidate>,
    },
}

#[derive(Debug)]
struct EligibleCandidate<'a> {
    candidate: &'a ModelCandidate,
    error: &'a ErrorBoundEvidence,
    well_posedness: &'a WellPosednessEvidence,
}

pub fn evaluate_adaptive_policy(
    request: &AdaptivePolicyRequest,
) -> Result<PolicyDecision, InputError> {
    validate_request(request)?;

    let mut ordered: Vec<&ModelCandidate> = request.candidates.iter().collect();
    ordered.sort_by(|left, right| left.profile_sha256.cmp(&right.profile_sha256));

    let mut seen_profiles = BTreeSet::new();
    for candidate in &ordered {
        if !seen_profiles.insert(candidate.profile_sha256.as_str()) {
            return Err(InputError::DuplicateProfile(candidate.profile_id.clone()));
        }
    }

    let mut excluded = Vec::new();
    let mut eligible = Vec::new();

    for candidate in ordered {
        if candidate.compatibility != request.compatibility {
            excluded.push(excluded(candidate, ExclusionReason::CompatibilityMismatch));
            continue;
        }

        let well_posedness = match &candidate.well_posedness {
            Evidence::Qualified(evidence) => evidence,
            Evidence::Unknown { .. } => {
                excluded.push(excluded(candidate, ExclusionReason::WellPosednessUnknown));
                continue;
            }
            Evidence::Rejected { .. } => {
                excluded.push(excluded(candidate, ExclusionReason::WellPosednessRejected));
                continue;
            }
        };

        if !candidate.kind.matches_well_posedness(well_posedness) {
            excluded.push(excluded(candidate, ExclusionReason::EvidenceKindMismatch));
            continue;
        }
        if !well_posedness.digests_valid() {
            excluded.push(excluded(candidate, ExclusionReason::InvalidWellPosednessReceipt));
            continue;
        }
        if !well_posedness.identity_matches(candidate, &request.compatibility) {
            excluded.push(excluded(candidate, ExclusionReason::EvidenceIdentityMismatch));
            continue;
        }

        match (candidate.kind.clone(), well_posedness) {
            (
                ModelKind::Moment { .. },
                WellPosednessEvidence::MomentHyperbolicity {
                    minimum_normalized_margin,
                    maximum_eigenbasis_condition,
                    ..
                },
            ) => {
                if !minimum_normalized_margin.is_finite()
                    || !maximum_eigenbasis_condition.is_finite()
                    || *maximum_eigenbasis_condition < 1.0
                {
                    excluded.push(excluded(candidate, ExclusionReason::InvalidWellPosednessReceipt));
                    continue;
                }
                if *minimum_normalized_margin < request.minimum_hyperbolicity_margin {
                    excluded.push(excluded(candidate, ExclusionReason::HyperbolicityMarginTooSmall));
                    continue;
                }
                if *maximum_eigenbasis_condition > request.maximum_eigenbasis_condition {
                    excluded.push(excluded(candidate, ExclusionReason::EigenbasisConditionTooLarge));
                    continue;
                }
            }
            (
                ModelKind::KineticPatch { .. },
                WellPosednessEvidence::KineticAdmissibility {
                    minimum_realizability_margin,
                    ..
                },
            ) => {
                if !minimum_realizability_margin.is_finite() || *minimum_realizability_margin < 0.0 {
                    excluded.push(excluded(candidate, ExclusionReason::InvalidWellPosednessReceipt));
                    continue;
                }
                if *minimum_realizability_margin < request.minimum_kinetic_realizability_margin {
                    excluded.push(excluded(
                        candidate,
                        ExclusionReason::KineticRealizabilityMarginTooSmall,
                    ));
                    continue;
                }
            }
            _ => {
                excluded.push(excluded(candidate, ExclusionReason::EvidenceKindMismatch));
                continue;
            }
        }

        let error = match &candidate.error_bound {
            Evidence::Qualified(evidence) => evidence,
            Evidence::Unknown { .. } => {
                excluded.push(excluded(candidate, ExclusionReason::ErrorEvidenceUnknown));
                continue;
            }
            Evidence::Rejected { .. } => {
                excluded.push(excluded(candidate, ExclusionReason::ErrorEvidenceRejected));
                continue;
            }
        };

        if !is_sha256(&error.profile_sha256)
            || !is_sha256(&error.model_sha256)
            || !is_sha256(&error.compatibility_sha256)
            || !is_sha256(&error.estimator_sha256)
            || !is_sha256(&error.calibration_receipt_sha256)
            || !is_sha256(&error.validation_domain_sha256)
            || !error.upper_bound.is_finite()
            || error.upper_bound < 0.0
            || !error.calibration_coverage.is_finite()
            || !(0.0..=1.0).contains(&error.calibration_coverage)
            || error.error_units_id.trim().is_empty()
        {
            excluded.push(excluded(candidate, ExclusionReason::InvalidErrorEvidence));
            continue;
        }
        if error.profile_sha256 != candidate.profile_sha256
            || error.model_sha256 != candidate.model_sha256
            || error.compatibility_sha256 != request.compatibility.compatibility_sha256
        {
            excluded.push(excluded(candidate, ExclusionReason::EvidenceIdentityMismatch));
            continue;
        }
        if error.error_units_id != request.error_units_id {
            excluded.push(excluded(candidate, ExclusionReason::ErrorUnitsMismatch));
            continue;
        }
        if candidate.cost_units_id != request.cost_units_id {
            excluded.push(excluded(candidate, ExclusionReason::CostUnitsMismatch));
            continue;
        }
        if error.calibration_coverage < request.minimum_calibration_coverage {
            excluded.push(excluded(candidate, ExclusionReason::CalibrationCoverageTooLow));
            continue;
        }
        if error.upper_bound > request.error_tolerance {
            excluded.push(excluded(candidate, ExclusionReason::ErrorBoundTooLarge));
            continue;
        }

        eligible.push(EligibleCandidate {
            candidate,
            error,
            well_posedness,
        });
    }

    if eligible.is_empty() {
        let has_unknown = excluded.iter().any(|item| {
            matches!(
                item.reason,
                ExclusionReason::WellPosednessUnknown | ExclusionReason::ErrorEvidenceUnknown
            )
        });
        return Ok(if has_unknown {
            PolicyDecision::Unknown {
                reason: UnknownReason::OnlyUnresolvedCandidatesRemain,
                excluded,
            }
        } else {
            PolicyDecision::Refused {
                reason: RefusalReason::NoCandidateMeetsTheErrorAndValidityEnvelope,
                excluded,
            }
        });
    }

    let current_candidate = request.current_profile_sha256.as_ref().and_then(|current| {
        request
            .candidates
            .iter()
            .find(|candidate| &candidate.profile_sha256 == current)
    });
    if request.current_profile_sha256.is_some() && current_candidate.is_none() {
        return Ok(PolicyDecision::Unknown {
            reason: UnknownReason::CurrentModelProfileNotInCandidateSet,
            excluded,
        });
    }

    let current_eligible = request.current_profile_sha256.as_ref().and_then(|current| {
        eligible
            .iter()
            .find(|candidate| &candidate.candidate.profile_sha256 == current)
    });

    if let Some(current) = current_eligible {
        if current.error.upper_bound <= request.coarsen_error_tolerance {
            return Ok(PolicyDecision::Selected {
                receipt: selection_receipt(
                    current,
                    None,
                    SelectionReason::RetainedInsideCoarseningBand,
                ),
                excluded,
            });
        }
        if request.current_dwell_steps < request.minimum_dwell_steps {
            return Ok(PolicyDecision::Selected {
                receipt: selection_receipt(
                    current,
                    None,
                    SelectionReason::RetainedDuringMinimumDwell,
                ),
                excluded,
            });
        }
    }

    let urgent_refinement = if let Some(current) = current_candidate {
        current_eligible.is_none()
            || current_error_if_available(current)
                .is_none_or(|error| error.upper_bound > request.error_tolerance)
    } else {
        false
    };

    let mut transition_rejections = Vec::new();
    let mut selectable = Vec::new();

    for item in &eligible {
        let candidate = item.candidate;
        let is_current = request
            .current_profile_sha256
            .as_ref()
            .is_some_and(|current| current == &candidate.profile_sha256);

        if is_current {
            selectable.push(item);
            continue;
        }

        if let Some(current) = current_candidate {
            if urgent_refinement
                && candidate.kind.fidelity_rank() <= current.kind.fidelity_rank()
            {
                continue;
            }
            if !urgent_refinement
                && request.current_dwell_steps < request.minimum_dwell_steps
            {
                continue;
            }
            if !urgent_refinement
                && current_eligible.is_some()
                && candidate.kind.fidelity_rank() < current.kind.fidelity_rank()
                && item.error.upper_bound > request.coarsen_error_tolerance
            {
                continue;
            }
            match validate_transition(request, current, candidate) {
                Ok(_) => selectable.push(item),
                Err(reason) => transition_rejections.push(excluded(candidate, reason)),
            }
        } else {
            selectable.push(item);
        }
    }

    if selectable.is_empty() {
        excluded.extend(transition_rejections);
        let current_unresolved = current_candidate.is_some()
            && current_eligible.is_none()
            && excluded.iter().any(|item| {
                item.profile_sha256
                    == request.current_profile_sha256.clone().unwrap_or_default()
                    && matches!(
                        item.reason,
                        ExclusionReason::WellPosednessUnknown
                            | ExclusionReason::ErrorEvidenceUnknown
                    )
            });
        return Ok(if current_unresolved {
            PolicyDecision::Unknown {
                reason: UnknownReason::CurrentModelEvidenceUnresolved,
                excluded,
            }
        } else if current_candidate.is_some() && urgent_refinement {
            PolicyDecision::Refused {
                reason: RefusalReason::NoSafeRefinementAvailable,
                excluded,
            }
        } else if request.current_profile_sha256.is_some() {
            PolicyDecision::Refused {
                reason: RefusalReason::NoTransitionQualifiedCandidate,
                excluded,
            }
        } else {
            PolicyDecision::Refused {
                reason: RefusalReason::NoCandidateMeetsTheErrorAndValidityEnvelope,
                excluded,
            }
        });
    }

    selectable.sort_by(|left, right| {
        left.candidate
            .estimated_cost
            .total_cmp(&right.candidate.estimated_cost)
            .then_with(|| {
                left.candidate
                    .profile_sha256
                    .cmp(&right.candidate.profile_sha256)
            })
    });

    let chosen = selectable[0];
    let is_current = request
        .current_profile_sha256
        .as_ref()
        .is_some_and(|current| current == &chosen.candidate.profile_sha256);

    let reason = if is_current {
        SelectionReason::AdaptedAfterMinimumDwell
    } else if urgent_refinement {
        SelectionReason::EscalatedForErrorOrValidity
    } else if request.current_profile_sha256.is_none() {
        SelectionReason::InitialLowestCostWithinTolerance
    } else {
        SelectionReason::AdaptedAfterMinimumDwell
    };

    let transition_receipt = if is_current {
        None
    } else {
        chosen.candidate.transition.clone()
    };

    excluded.extend(transition_rejections);
    excluded.sort_by(|left, right| {
        left.profile_sha256
            .cmp(&right.profile_sha256)
            .then_with(|| format!("{:?}", left.reason).cmp(&format!("{:?}", right.reason)))
    });

    Ok(PolicyDecision::Selected {
        receipt: selection_receipt(chosen, transition_receipt, reason),
        excluded,
    })
}

fn current_error_if_available(candidate: &ModelCandidate) -> Option<&ErrorBoundEvidence> {
    match &candidate.error_bound {
        Evidence::Qualified(evidence) => Some(evidence),
        Evidence::Unknown { .. } | Evidence::Rejected { .. } => None,
    }
}

fn validate_transition(
    request: &AdaptivePolicyRequest,
    current: &ModelCandidate,
    next: &ModelCandidate,
) -> Result<(), ExclusionReason> {
    let Some(transition) = next.transition.as_ref() else {
        return Err(ExclusionReason::MissingTransitionEvidence);
    };
    if transition.from_profile_sha256 != current.profile_sha256
        || transition.to_profile_sha256 != next.profile_sha256
        || transition.from_model_sha256 != current.model_sha256
        || transition.to_model_sha256 != next.model_sha256
        || transition.interface_schema_sha256 != request.compatibility.interface_schema_sha256
    {
        return Err(ExclusionReason::TransitionIdentityMismatch);
    }
    if !is_sha256(&transition.from_profile_sha256)
        || !is_sha256(&transition.to_profile_sha256)
        || !is_sha256(&transition.from_model_sha256)
        || !is_sha256(&transition.to_model_sha256)
        || !is_sha256(&transition.interface_schema_sha256)
        || !is_sha256(&transition.projection_sha256)
        || !is_sha256(&transition.reconstruction_sha256)
        || !is_sha256(&transition.conservation_receipt_sha256)
        || !transition.maximum_conservation_residual.is_finite()
        || transition.maximum_conservation_residual < 0.0
        || !transition.state_transfer_error_upper_bound.is_finite()
        || transition.state_transfer_error_upper_bound < 0.0
    {
        return Err(ExclusionReason::InvalidTransitionEvidence);
    }
    if transition.maximum_conservation_residual
        > request.maximum_transition_conservation_residual
    {
        return Err(ExclusionReason::TransitionConservationResidualTooLarge);
    }
    if transition.state_transfer_error_upper_bound > request.maximum_state_transfer_error {
        return Err(ExclusionReason::StateTransferErrorTooLarge);
    }
    Ok(())
}

fn selection_receipt(
    item: &EligibleCandidate<'_>,
    transition: Option<TransitionEvidence>,
    reason: SelectionReason,
) -> SelectionReceipt {
    SelectionReceipt {
        profile_id: item.candidate.profile_id.clone(),
        profile_sha256: item.candidate.profile_sha256.clone(),
        model_sha256: item.candidate.model_sha256.clone(),
        kind: item.candidate.kind.clone(),
        fidelity_rank: item.candidate.kind.fidelity_rank(),
        error_upper_bound: item.error.upper_bound,
        estimated_cost: item.candidate.estimated_cost,
        estimator_sha256: item.error.estimator_sha256.clone(),
        calibration_receipt_sha256: item.error.calibration_receipt_sha256.clone(),
        well_posedness_receipt_sha256: item.well_posedness.receipt_sha256().to_owned(),
        transition,
        reason,
    }
}

fn excluded(candidate: &ModelCandidate, reason: ExclusionReason) -> ExcludedCandidate {
    ExcludedCandidate {
        profile_sha256: candidate.profile_sha256.clone(),
        reason,
    }
}

fn validate_request(request: &AdaptivePolicyRequest) -> Result<(), InputError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(InputError::UnsupportedSchemaVersion);
    }
    if request.request_id.trim().is_empty() {
        return Err(InputError::EmptyRequestIdentity);
    }
    validate_compatibility(&request.compatibility)?;
    if let Some(current) = &request.current_profile_sha256 {
        if !is_sha256(current) {
            return Err(InputError::InvalidCurrentProfileDigest);
        }
    }
    if request.candidates.is_empty() {
        return Err(InputError::NoCandidates);
    }
    for (name, value) in [
        ("error_tolerance", request.error_tolerance),
        ("coarsen_error_tolerance", request.coarsen_error_tolerance),
        ("minimum_calibration_coverage", request.minimum_calibration_coverage),
        ("minimum_hyperbolicity_margin", request.minimum_hyperbolicity_margin),
        ("maximum_eigenbasis_condition", request.maximum_eigenbasis_condition),
        ("minimum_kinetic_realizability_margin", request.minimum_kinetic_realizability_margin),
        ("maximum_transition_conservation_residual", request.maximum_transition_conservation_residual),
        ("maximum_state_transfer_error", request.maximum_state_transfer_error),
    ] {
        if !value.is_finite() {
            return Err(InputError::InvalidPolicyThreshold(name));
        }
    }
    if request.error_tolerance < 0.0
        || request.coarsen_error_tolerance < 0.0
        || request.coarsen_error_tolerance > request.error_tolerance
    {
        return Err(InputError::InvalidPolicyThreshold("coarsening threshold"));
    }
    if !(0.0..=1.0).contains(&request.minimum_calibration_coverage) {
        return Err(InputError::InvalidPolicyThreshold("minimum calibration coverage"));
    }
    if request.minimum_hyperbolicity_margin < 0.0 {
        return Err(InputError::InvalidPolicyThreshold("minimum hyperbolicity margin"));
    }
    if request.maximum_eigenbasis_condition < 1.0 {
        return Err(InputError::InvalidPolicyThreshold("maximum eigenbasis condition"));
    }
    if request.minimum_kinetic_realizability_margin < 0.0
        || request.maximum_transition_conservation_residual < 0.0
        || request.maximum_state_transfer_error < 0.0
    {
        return Err(InputError::InvalidPolicyThreshold("nonnegative physical threshold"));
    }
    if request.error_units_id.trim().is_empty() {
        return Err(InputError::EmptyCompatibilityIdentity("error_units_id"));
    }
    if request.cost_units_id.trim().is_empty() {
        return Err(InputError::EmptyCompatibilityIdentity("cost_units_id"));
    }
    for candidate in &request.candidates {
        if candidate.profile_id.trim().is_empty() {
            return Err(InputError::EmptyCandidateIdentity);
        }
        if !is_sha256(&candidate.profile_sha256) {
            return Err(InputError::InvalidCandidateDigest("profile_sha256"));
        }
        if !is_sha256(&candidate.model_sha256) {
            return Err(InputError::InvalidCandidateDigest("model_sha256"));
        }
        if !candidate.estimated_cost.is_finite() || candidate.estimated_cost <= 0.0
            || candidate.cost_units_id.trim().is_empty()
        {
            return Err(InputError::InvalidCost);
        }
        validate_compatibility(&candidate.compatibility)
            .map_err(|_| InputError::InvalidModelIdentity)?;
        match &candidate.kind {
            ModelKind::Moment { moment_family_id, basis_id, .. } => {
                if moment_family_id.trim().is_empty() || basis_id.trim().is_empty() {
                    return Err(InputError::InvalidModelIdentity);
                }
            }
            ModelKind::KineticPatch {
                kinetic_method_id,
                velocity_grid_sha256,
                ..
            } => {
                if kinetic_method_id.trim().is_empty() {
                    return Err(InputError::InvalidModelIdentity);
                }
                if !is_sha256(velocity_grid_sha256) {
                    return Err(InputError::InvalidCandidateDigest("velocity_grid_sha256"));
                }
            }
        }
        if let Evidence::Unknown { reason } | Evidence::Rejected { reason } =
            &candidate.well_posedness
        {
            if reason.trim().is_empty() {
                return Err(InputError::InvalidEvidenceReason(candidate.profile_id.clone()));
            }
        }
        if let Evidence::Unknown { reason } | Evidence::Rejected { reason } =
            &candidate.error_bound
        {
            if reason.trim().is_empty() {
                return Err(InputError::InvalidEvidenceReason(candidate.profile_id.clone()));
            }
        }
    }
    Ok(())
}

fn validate_compatibility(value: &ModelCompatibility) -> Result<(), InputError> {
    for (name, field) in [
        ("physics_system_id", value.physics_system_id.as_str()),
        ("species_scope_id", value.species_scope_id.as_str()),
        ("collision_model_id", value.collision_model_id.as_str()),
        ("normalization_id", value.normalization_id.as_str()),
        ("conserved_units_id", value.conserved_units_id.as_str()),
        ("coordinate_frame_id", value.coordinate_frame_id.as_str()),
        ("geometry_id", value.geometry_id.as_str()),
        ("boundary_model_id", value.boundary_model_id.as_str()),
        ("solver_interface_id", value.solver_interface_id.as_str()),
        ("conserved_component_layout_id", value.conserved_component_layout_id.as_str()),
    ] {
        if field.trim().is_empty() {
            return Err(InputError::EmptyCompatibilityIdentity(name));
        }
    }
    if !is_sha256(&value.interface_schema_sha256)
        || !is_sha256(&value.source_revision_sha256)
        || !is_sha256(&value.solver_build_sha256)
        || !is_sha256(&value.compatibility_sha256)
    {
        return Err(InputError::InvalidCompatibilityDigest);
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn compatibility() -> ModelCompatibility {
        ModelCompatibility {
            physics_system_id: "synthetic-1d-plasma-moment-system".into(),
            source_revision_sha256: digest('6'),
            solver_build_sha256: digest('7'),
            compatibility_sha256: digest('8'),
            species_scope_id: "electrons".into(),
            collision_model_id: "collisionless-slab".into(),
            normalization_id: "synthetic-normalization-v1".into(),
            conserved_units_id: "normalized-moment-units-v1".into(),
            coordinate_frame_id: "field-aligned".into(),
            geometry_id: "slab-1d".into(),
            boundary_model_id: "synthetic-boundary-v1".into(),
            solver_interface_id: "synthetic-fv-stage-v1".into(),
            conserved_component_layout_id: "density-momentum-energy-v1".into(),
            interface_schema_sha256: digest('a'),
        }
    }

    fn moment_kind(rank: u16) -> ModelKind {
        ModelKind::Moment {
            moment_family_id: "hermite-laguerre".into(),
            basis_id: "normalized-Hermite-Laguerre-v1".into(),
            parallel_order: rank,
            perpendicular_order: 0,
            fidelity_rank: rank,
        }
    }

    fn moment_evidence(profile_sha256: &str, model_sha256: &str) -> WellPosednessEvidence {
        WellPosednessEvidence::MomentHyperbolicity {
            profile_sha256: profile_sha256.into(),
            model_sha256: model_sha256.into(),
            compatibility_sha256: compatibility().compatibility_sha256,
            receipt_sha256: digest('b'),
            envelope_sha256: digest('c'),
            minimum_normalized_margin: 0.25,
            maximum_eigenbasis_condition: 8.0,
        }
    }

    fn kinetic_evidence(profile_sha256: &str, model_sha256: &str) -> WellPosednessEvidence {
        WellPosednessEvidence::KineticAdmissibility {
            profile_sha256: profile_sha256.into(),
            model_sha256: model_sha256.into(),
            compatibility_sha256: compatibility().compatibility_sha256,
            receipt_sha256: digest('d'),
            envelope_sha256: digest('e'),
            minimum_realizability_margin: 0.4,
        }
    }

    fn error(bound: f64, profile_sha256: &str, model_sha256: &str) -> ErrorBoundEvidence {
        ErrorBoundEvidence {
            upper_bound: bound,
            error_units_id: "normalized-moment-error".into(),
            profile_sha256: profile_sha256.into(),
            model_sha256: model_sha256.into(),
            compatibility_sha256: compatibility().compatibility_sha256,
            estimator_sha256: digest('f'),
            calibration_receipt_sha256: digest('1'),
            validation_domain_sha256: digest('2'),
            calibration_coverage: 0.99,
        }
    }

    fn transition(from: &str, to: &str) -> TransitionEvidence {
        TransitionEvidence {
            from_profile_sha256: from.into(),
            to_profile_sha256: to.into(),
            from_model_sha256: digest('9'),
            to_model_sha256: digest('9'),
            interface_schema_sha256: compatibility().interface_schema_sha256,
            projection_sha256: digest('3'),
            reconstruction_sha256: digest('4'),
            conservation_receipt_sha256: digest('5'),
            maximum_conservation_residual: 1.0e-10,
            state_transfer_error_upper_bound: 0.01,
        }
    }

    fn moment_candidate(
        profile_character: char,
        rank: u16,
        error_bound: f64,
        cost: f64,
    ) -> ModelCandidate {
        ModelCandidate {
            profile_id: format!("moment-order-{rank}"),
            profile_sha256: digest(profile_character),
            model_sha256: digest('9'),
            kind: moment_kind(rank),
            compatibility: compatibility(),
            well_posedness: Evidence::Qualified(moment_evidence(&digest(profile_character), &digest('9'))),
            error_bound: Evidence::Qualified(error(error_bound, &digest(profile_character), &digest('9'))),
            estimated_cost: cost,
            cost_units_id: "normalized-runtime-cost-v1".into(),
            transition: None,
        }
    }

    fn request(candidates: Vec<ModelCandidate>) -> AdaptivePolicyRequest {
        AdaptivePolicyRequest {
            schema_version: SCHEMA_VERSION.into(),
            request_id: "adaptive-request-1".into(),
            compatibility: compatibility(),
            candidates,
            current_profile_sha256: None,
            current_dwell_steps: 0,
            minimum_dwell_steps: 3,
            error_tolerance: 0.1,
            coarsen_error_tolerance: 0.05,
            error_units_id: "normalized-moment-error".into(),
            cost_units_id: "normalized-runtime-cost-v1".into(),
            minimum_calibration_coverage: 0.95,
            minimum_hyperbolicity_margin: 0.1,
            maximum_eigenbasis_condition: 20.0,
            minimum_kinetic_realizability_margin: 0.1,
            maximum_transition_conservation_residual: 1.0e-8,
            maximum_state_transfer_error: 0.05,
        }
    }

    fn selected(decision: PolicyDecision) -> SelectionReceipt {
        match decision {
            PolicyDecision::Selected { receipt, .. } => receipt,
            other => panic!("expected selected policy result, got {other:?}"),
        }
    }

    #[test]
    fn chooses_lowest_cost_candidate_within_error_and_validity_envelopes() {
        let decision = evaluate_adaptive_policy(&request(vec![
            moment_candidate('a', 4, 0.04, 10.0),
            moment_candidate('b', 6, 0.03, 8.0),
            moment_candidate('c', 8, 0.01, 20.0),
        ]))
        .unwrap();
        let receipt = selected(decision);
        assert_eq!(receipt.profile_sha256, digest('b'));
        assert_eq!(receipt.reason, SelectionReason::InitialLowestCostWithinTolerance);
    }

    #[test]
    fn never_selects_higher_order_just_because_it_is_higher() {
        let decision = evaluate_adaptive_policy(&request(vec![
            moment_candidate('a', 4, 0.04, 10.0),
            moment_candidate('b', 16, 0.02, 100.0),
        ]))
        .unwrap();
        assert_eq!(selected(decision).profile_sha256, digest('a'));
    }

    #[test]
    fn retains_current_profile_inside_coarsening_band() {
        let current = moment_candidate('a', 8, 0.04, 20.0);
        let mut cheaper = moment_candidate('b', 4, 0.02, 5.0);
        cheaper.transition = Some(transition(&digest('a'), &digest('b')));
        let mut req = request(vec![current.clone(), cheaper]);
        req.current_profile_sha256 = Some(digest('a'));
        req.current_dwell_steps = 10;
        let receipt = selected(evaluate_adaptive_policy(&req).unwrap());
        assert_eq!(receipt.profile_sha256, digest('a'));
        assert_eq!(receipt.reason, SelectionReason::RetainedInsideCoarseningBand);
    }

    #[test]
    fn minimum_dwell_prevents_coarsening_until_hysteresis_expires() {
        let current = moment_candidate('a', 8, 0.08, 20.0);
        let mut cheaper = moment_candidate('b', 4, 0.04, 5.0);
        cheaper.transition = Some(transition(&digest('a'), &digest('b')));
        let mut req = request(vec![current.clone(), cheaper]);
        req.current_profile_sha256 = Some(digest('a'));
        req.current_dwell_steps = 1;
        let receipt = selected(evaluate_adaptive_policy(&req).unwrap());
        assert_eq!(receipt.profile_sha256, digest('a'));
        assert_eq!(receipt.reason, SelectionReason::RetainedDuringMinimumDwell);

        req.current_dwell_steps = req.minimum_dwell_steps;
        let receipt = selected(evaluate_adaptive_policy(&req).unwrap());
        assert_eq!(receipt.profile_sha256, digest('b'));
    }

    #[test]
    fn refinement_for_error_exceedance_can_bypass_minimum_dwell() {
        let current = moment_candidate('a', 4, 0.5, 3.0);
        let mut refined = moment_candidate('b', 8, 0.08, 12.0);
        refined.transition = Some(transition(&digest('a'), &digest('b')));
        let mut req = request(vec![current.clone(), refined]);
        req.current_profile_sha256 = Some(digest('a'));
        req.current_dwell_steps = 0;
        let receipt = selected(evaluate_adaptive_policy(&req).unwrap());
        assert_eq!(receipt.profile_sha256, digest('b'));
        assert_eq!(receipt.reason, SelectionReason::EscalatedForErrorOrValidity);
    }

    #[test]
    fn model_outside_hyperbolicity_margin_is_never_selected() {
        let mut invalid = moment_candidate('a', 8, 0.01, 1.0);
        if let Evidence::Qualified(WellPosednessEvidence::MomentHyperbolicity {
            minimum_normalized_margin, ..
        }) = &mut invalid.well_posedness
        {
            *minimum_normalized_margin = 0.01;
        }
        let decision = evaluate_adaptive_policy(&request(vec![invalid])).unwrap();
        assert!(matches!(
            decision,
            PolicyDecision::Refused { reason: RefusalReason::NoCandidateMeetsTheErrorAndValidityEnvelope, .. }
        ));
    }

    #[test]
    fn ill_conditioned_eigenbasis_is_never_selected() {
        let mut invalid = moment_candidate('a', 8, 0.01, 1.0);
        if let Evidence::Qualified(WellPosednessEvidence::MomentHyperbolicity {
            maximum_eigenbasis_condition, ..
        }) = &mut invalid.well_posedness
        {
            *maximum_eigenbasis_condition = 1.0e9;
        }
        let decision = evaluate_adaptive_policy(&request(vec![invalid])).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("ill-conditioned model must be refused");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::EigenbasisConditionTooLarge);
    }

    #[test]
    fn unknown_hyperbolicity_yields_unknown_not_a_fallback_guess() {
        let mut candidate = moment_candidate('a', 8, 0.01, 1.0);
        candidate.well_posedness = Evidence::Unknown {
            reason: "hyperbolicity envelope not derived".into(),
        };
        let decision = evaluate_adaptive_policy(&request(vec![candidate])).unwrap();
        assert!(matches!(
            decision,
            PolicyDecision::Unknown {
                reason: UnknownReason::OnlyUnresolvedCandidatesRemain,
                ..
            }
        ));
    }

    #[test]
    fn wrong_model_kind_well_posedness_receipt_is_excluded() {
        let mut candidate = moment_candidate('a', 8, 0.01, 1.0);
        candidate.well_posedness = Evidence::Qualified(kinetic_evidence(&digest('a'), &digest('9')));
        let decision = evaluate_adaptive_policy(&request(vec![candidate])).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("mismatched evidence kind must be refused");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::EvidenceKindMismatch);
    }

    #[test]
    fn kinetic_patch_can_be_selected_when_its_own_admissibility_evidence_passes() {
        let kinetic = ModelCandidate {
            profile_id: "kinetic-patch-medium".into(),
            profile_sha256: digest('6'),
            model_sha256: digest('8'),
            kind: ModelKind::KineticPatch {
                kinetic_method_id: "1d1v-vlasov-grid".into(),
                velocity_grid_sha256: digest('7'),
                fidelity_rank: 20,
            },
            compatibility: compatibility(),
            well_posedness: Evidence::Qualified(kinetic_evidence(&digest('6'), &digest('8'))),
            error_bound: Evidence::Qualified(error(0.02, &digest('6'), &digest('8'))),
            estimated_cost: 40.0,
            cost_units_id: "normalized-runtime-cost-v1".into(),
            transition: None,
        };
        let decision = evaluate_adaptive_policy(&request(vec![kinetic])).unwrap();
        assert!(matches!(
            selected(decision).kind,
            ModelKind::KineticPatch { .. }
        ));
    }

    #[test]
    fn missing_transition_receipt_blocks_switching() {
        let current = moment_candidate('a', 4, 0.08, 10.0);
        let next = moment_candidate('b', 8, 0.02, 5.0);
        let mut req = request(vec![current, next]);
        req.current_profile_sha256 = Some(digest('a'));
        req.current_dwell_steps = 10;
        let decision = evaluate_adaptive_policy(&req).unwrap();
        assert!(matches!(
            decision,
            PolicyDecision::Refused {
                reason: RefusalReason::NoTransitionQualifiedCandidate,
                ..
            }
        ));
    }

    #[test]
    fn transition_must_bind_exact_profiles_and_common_interface() {
        let current = moment_candidate('a', 4, 0.08, 10.0);
        let mut next = moment_candidate('b', 8, 0.02, 5.0);
        let mut invalid = transition(&digest('a'), &digest('b'));
        invalid.to_profile_sha256 = digest('c');
        next.transition = Some(invalid);
        let mut req = request(vec![current, next]);
        req.current_profile_sha256 = Some(digest('a'));
        req.current_dwell_steps = 10;
        let decision = evaluate_adaptive_policy(&req).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("incorrect transition binding must be refused");
        };
        assert!(excluded.iter().any(|item| {
            item.profile_sha256 == digest('b')
                && item.reason == ExclusionReason::TransitionIdentityMismatch
        }));
    }

    #[test]
    fn transition_conservation_residual_must_be_below_threshold() {
        let current = moment_candidate('a', 4, 0.08, 10.0);
        let mut next = moment_candidate('b', 8, 0.02, 5.0);
        let mut transfer = transition(&digest('a'), &digest('b'));
        transfer.maximum_conservation_residual = 1.0e-3;
        next.transition = Some(transfer);
        let mut req = request(vec![current, next]);
        req.current_profile_sha256 = Some(digest('a'));
        req.current_dwell_steps = 10;
        let decision = evaluate_adaptive_policy(&req).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("bad transition balance must be refused");
        };
        assert!(excluded.iter().any(|item| {
            item.reason == ExclusionReason::TransitionConservationResidualTooLarge
        }));
    }

    #[test]
    fn insufficient_error_calibration_coverage_blocks_candidate() {
        let mut candidate = moment_candidate('a', 8, 0.01, 1.0);
        if let Evidence::Qualified(evidence) = &mut candidate.error_bound {
            evidence.calibration_coverage = 0.5;
        }
        let decision = evaluate_adaptive_policy(&request(vec![candidate])).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("under-calibrated estimator must be refused");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::CalibrationCoverageTooLow);
    }

    #[test]
    fn error_bound_units_must_match_policy_units() {
        let mut candidate = moment_candidate('a', 8, 0.01, 1.0);
        if let Evidence::Qualified(evidence) = &mut candidate.error_bound {
            evidence.error_units_id = "other-units".into();
        }
        let decision = evaluate_adaptive_policy(&request(vec![candidate])).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("incompatible error units must be refused");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::ErrorUnitsMismatch);
    }

    #[test]
    fn candidate_input_order_does_not_change_selection_or_exclusions() {
        let a = moment_candidate('a', 4, 0.04, 10.0);
        let b = moment_candidate('b', 8, 0.02, 5.0);
        let first = evaluate_adaptive_policy(&request(vec![a.clone(), b.clone()])).unwrap();
        let second = evaluate_adaptive_policy(&request(vec![b, a])).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_profiles_are_rejected_as_invalid_request() {
        let a = moment_candidate('a', 4, 0.04, 10.0);
        let duplicate = a.clone();
        let req = request(vec![a, duplicate]);
        assert_eq!(
            evaluate_adaptive_policy(&req),
            Err(InputError::DuplicateProfile("moment-order-4".into()))
        );
    }

    #[test]
    fn malformed_profile_digest_is_rejected() {
        let mut candidate = moment_candidate('a', 8, 0.01, 1.0);
        candidate.profile_sha256 = "not-a-digest".into();
        assert_eq!(
            evaluate_adaptive_policy(&request(vec![candidate])),
            Err(InputError::InvalidCandidateDigest("profile_sha256"))
        );
    }


    #[test]
    fn stale_well_posedness_or_error_receipt_cannot_be_reused_for_another_model() {
        let mut candidate = moment_candidate('a', 8, 0.01, 1.0);
        if let Evidence::Qualified(WellPosednessEvidence::MomentHyperbolicity {
            model_sha256, ..
        }) = &mut candidate.well_posedness
        {
            *model_sha256 = digest('5');
        }
        let decision = evaluate_adaptive_policy(&request(vec![candidate])).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("cross-model qualification reuse must be refused");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::EvidenceIdentityMismatch);

        let mut candidate = moment_candidate('b', 8, 0.01, 1.0);
        if let Evidence::Qualified(error) = &mut candidate.error_bound {
            error.profile_sha256 = digest('c');
        }
        let decision = evaluate_adaptive_policy(&request(vec![candidate])).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("cross-profile error evidence reuse must be refused");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::EvidenceIdentityMismatch);
    }

    #[test]
    fn costs_must_share_one_declared_unit() {
        let mut candidate = moment_candidate('a', 8, 0.01, 1.0);
        candidate.cost_units_id = "gpu-seconds".into();
        let decision = evaluate_adaptive_policy(&request(vec![candidate])).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("incommensurate cost must be refused");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::CostUnitsMismatch);
    }

    #[test]
    fn a_missing_current_profile_is_unknown_not_initial_selection() {
        let mut req = request(vec![moment_candidate('a', 8, 0.01, 1.0)]);
        req.current_profile_sha256 = Some(digest('b'));
        assert!(matches!(
            evaluate_adaptive_policy(&req).unwrap(),
            PolicyDecision::Unknown {
                reason: UnknownReason::CurrentModelProfileNotInCandidateSet,
                ..
            }
        ));
    }

    #[test]
    fn non_finite_hyperbolicity_metrics_are_rejected() {
        let mut candidate = moment_candidate('a', 8, 0.01, 1.0);
        if let Evidence::Qualified(WellPosednessEvidence::MomentHyperbolicity {
            maximum_eigenbasis_condition, ..
        }) = &mut candidate.well_posedness
        {
            *maximum_eigenbasis_condition = f64::NAN;
        }
        let decision = evaluate_adaptive_policy(&request(vec![candidate])).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("NaN condition number must be refused");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::InvalidWellPosednessReceipt);
    }

    #[test]
    fn incompatible_physical_system_is_not_coerced() {
        let mut candidate = moment_candidate('a', 8, 0.01, 1.0);
        candidate.compatibility.collision_model_id = "collisional-model".into();
        let decision = evaluate_adaptive_policy(&request(vec![candidate])).unwrap();
        let PolicyDecision::Refused { excluded, .. } = decision else {
            panic!("incompatible physical model must be refused");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::CompatibilityMismatch);
    }

    #[test]
    fn current_model_with_unresolved_evidence_does_not_trigger_silent_switch() {
        let mut current = moment_candidate('a', 4, 0.08, 10.0);
        current.well_posedness = Evidence::Unknown {
            reason: "current hyperbolicity evidence stale".into(),
        };
        let mut next = moment_candidate('b', 8, 0.02, 5.0);
        next.transition = Some(transition(&digest('a'), &digest('b')));
        let mut req = request(vec![current, next]);
        req.current_profile_sha256 = Some(digest('a'));
        req.current_dwell_steps = 10;
        let decision = evaluate_adaptive_policy(&req).unwrap();
        assert!(matches!(
            decision,
            PolicyDecision::Selected { receipt, .. }
                if receipt.profile_sha256 == digest('b')
        ));
    }
}
