//! Dependency-aware solution-separation envelope for UWB navigation integrity.
//!
//! This module does not estimate a position or declare a fault hypothesis true.
//! It consumes externally produced, qualified position sets for a declared
//! hypothesis inventory and constructs a conservative ball centred at the
//! nominal solution that contains every supplied hypothesis-specific set.
//!
//! For nominal centre x0 and hypothesis set X_h = Ball(x_h, r_h), the
//! hypothesis envelope radius is:
//!
//!     PL_candidate = max_h ( norm(x_h - x0) + r_h )
//!
//! This is a deterministic outer bound conditional on:
//! - the declared hypothesis inventory being complete for its profile;
//! - every supplied position-set radius being valid for that hypothesis;
//! - source, clock, calibration and dependency evidence being qualified;
//! - the represented fault model covering the actual fault.
//!
//! It is not a statistical integrity-risk estimate, a calibrated protection
//! level, a fault detector, a cryptographic verifier or a safety certificate.
//! An incomplete inventory or any unbounded hypothesis suppresses the
//! protection-radius output and returns IntegrityUncertain.
//!
//! The dependency graph and SHA-256 values are caller-supplied commitments.
//! This module checks their shape and preserves them; it does not authenticate
//! source bytes or prove physical independence.

use std::collections::BTreeSet;
use std::fmt;

const DIM: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UwbFaultHypothesisKind {
    Nominal,
    AnchorExcluded,
    RadioExcluded,
    SharedClockFault,
    SharedCalibrationFault,
    BoundedRangeBias,
    MultiDomainFault,
    UnknownFault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnboundedPositionReason {
    MissingQualifiedRangeBound,
    UnresolvedGeometry,
    UnsupportedFaultModel,
    StaleOrUntrustedEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QualifiedPositionSet3d {
    Bounded {
        center_m: [f64; DIM],
        absolute_error_radius_m: f64,
    },
    Unbounded {
        reason: UnboundedPositionReason,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct UwbFaultHypothesisSolution3d {
    /// Stable nonzero identity within this inventory.
    pub hypothesis_id: u64,
    pub kind: UwbFaultHypothesisKind,
    /// Evaluator-declared common-cause/dependency groups affected by this
    /// hypothesis. These IDs are metadata, not proof of actual independence.
    pub dependency_group_ids: Vec<u64>,
    /// Exact source observation IDs affected by this hypothesis.
    pub affected_observation_ids: Vec<u64>,
    /// Externally qualified position set under this hypothesis.
    pub position_set: QualifiedPositionSet3d,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UwbFaultHypothesisInventory3d {
    pub inventory_id: u64,
    pub profile_id: u64,
    /// Commitment to the controller-visible observation/configuration inputs.
    pub input_manifest_sha256: [u8; 32],
    /// Commitment to the canonical, predeclared hypothesis inventory.
    pub inventory_sha256: [u8; 32],
    /// Number of hypotheses expected by the frozen inventory manifest.
    pub expected_hypothesis_count: usize,
    /// False when enumeration was truncated, unsupported or not completed.
    pub enumeration_complete: bool,
    /// Maximum number of simultaneous dependency groups represented by any
    /// declared non-nominal hypothesis.
    pub declared_max_fault_cardinality: usize,
    pub hypotheses: Vec<UwbFaultHypothesisSolution3d>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UwbFaultHypothesisIntegrityConfig {
    pub maximum_hypothesis_count: usize,
    pub maximum_fault_cardinality: usize,
    /// Profile-owned alert limit in metres. Not a universal navigation limit.
    pub alert_limit_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UwbFaultHypothesisIntegrityDisposition {
    QualifiedWithinAlertLimit,
    ExceedsAlertLimit,
    IntegrityUncertain,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UwbHypothesisSeparation3d {
    pub hypothesis_id: u64,
    pub kind: UwbFaultHypothesisKind,
    pub dependency_group_ids: Vec<u64>,
    pub affected_observation_ids: Vec<u64>,
    pub position_set_bounded: bool,
    pub separation_from_nominal_m: Option<f64>,
    pub hypothesis_uncertainty_radius_m: Option<f64>,
    /// Distance from the nominal centre to the farthest point in this
    /// hypothesis-specific ball.
    pub outer_radius_from_nominal_m: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UwbFaultHypothesisIntegrityReport3d {
    pub inventory_id: u64,
    pub profile_id: u64,
    pub input_manifest_sha256: [u8; 32],
    pub inventory_sha256: [u8; 32],
    pub expected_hypothesis_count: usize,
    pub evaluated_hypothesis_count: usize,
    pub hypothesis_inventory_complete: bool,
    pub every_hypothesis_position_set_bounded: bool,
    pub alert_limit_m: f64,
    /// Present only when the inventory is complete and every hypothesis has
    /// a qualified bounded position set. Never interpret a partial maximum as
    /// a protection level.
    pub protection_radius_m: Option<f64>,
    pub disposition: UwbFaultHypothesisIntegrityDisposition,
    /// Canonically sorted by hypothesis_id.
    pub hypotheses: Vec<UwbHypothesisSeparation3d>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UwbFaultHypothesisIntegrityError {
    InvalidConfiguration,
    InvalidInventoryIdentity,
    MissingInputManifestCommitment,
    MissingInventoryCommitment,
    EmptyHypothesisInventory,
    ExpectedHypothesisCountExceedsBudget,
    ObservedHypothesisCountExceedsBudget,
    FaultCardinalityExceedsBudget,
    InvalidHypothesisIdentity,
    DuplicateHypothesisIdentity(u64),
    MissingNominalHypothesis,
    MultipleNominalHypotheses,
    InvalidNominalHypothesisShape,
    InvalidFaultHypothesisShape(u64),
    InvalidDependencyGroupIdentity(u64),
    DuplicateDependencyGroupIdentity { hypothesis_id: u64, group_id: u64 },
    InvalidObservationIdentity(u64),
    DuplicateObservationIdentity { hypothesis_id: u64, observation_id: u64 },
    InvalidPositionSet(u64),
    ArithmeticOverflow(u64),
}

impl fmt::Display for UwbFaultHypothesisIntegrityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration => write!(f, "hypothesis and alert-limit budgets must be finite and positive"),
            Self::InvalidInventoryIdentity => write!(f, "inventory and profile identities must be nonzero"),
            Self::MissingInputManifestCommitment => write!(f, "input manifest SHA-256 commitment must be nonzero"),
            Self::MissingInventoryCommitment => write!(f, "hypothesis inventory SHA-256 commitment must be nonzero"),
            Self::EmptyHypothesisInventory => write!(f, "at least one hypothesis is required"),
            Self::ExpectedHypothesisCountExceedsBudget => write!(f, "expected hypothesis count exceeds the configured enumeration budget"),
            Self::ObservedHypothesisCountExceedsBudget => write!(f, "observed hypothesis count exceeds the configured enumeration budget"),
            Self::FaultCardinalityExceedsBudget => write!(f, "declared or observed fault cardinality exceeds the configured budget"),
            Self::InvalidHypothesisIdentity => write!(f, "hypothesis identity must be nonzero"),
            Self::DuplicateHypothesisIdentity(id) => write!(f, "duplicate hypothesis identity: {id}"),
            Self::MissingNominalHypothesis => write!(f, "exactly one nominal hypothesis is required"),
            Self::MultipleNominalHypotheses => write!(f, "more than one nominal hypothesis was supplied"),
            Self::InvalidNominalHypothesisShape => write!(f, "nominal hypothesis cannot name fault domains or affected observations"),
            Self::InvalidFaultHypothesisShape(id) => write!(f, "non-nominal hypothesis has no affected domain/observation or an invalid cardinality: {id}"),
            Self::InvalidDependencyGroupIdentity(id) => write!(f, "dependency group identity must be nonzero: {id}"),
            Self::DuplicateDependencyGroupIdentity { hypothesis_id, group_id } => write!(f, "duplicate dependency group {group_id} in hypothesis {hypothesis_id}"),
            Self::InvalidObservationIdentity(id) => write!(f, "affected observation identity must be nonzero: {id}"),
            Self::DuplicateObservationIdentity { hypothesis_id, observation_id } => write!(f, "duplicate observation {observation_id} in hypothesis {hypothesis_id}"),
            Self::InvalidPositionSet(id) => write!(f, "position set is invalid for hypothesis {id}"),
            Self::ArithmeticOverflow(id) => write!(f, "solution-separation arithmetic overflowed for hypothesis {id}"),
        }
    }
}

impl std::error::Error for UwbFaultHypothesisIntegrityError {}

/// Evaluate a complete or incomplete set of externally qualified
/// hypothesis-specific position sets.
///
/// A protection radius is returned only if the inventory is complete and
/// every hypothesis position set is bounded. An incomplete hypothesis set is
/// a valid research result with IntegrityUncertain disposition, not an error.
pub fn evaluate_uwb_fault_hypothesis_envelope_3d(
    inventory: &UwbFaultHypothesisInventory3d,
    config: UwbFaultHypothesisIntegrityConfig,
) -> Result<UwbFaultHypothesisIntegrityReport3d, UwbFaultHypothesisIntegrityError> {
    validate_config(config)?;

    if inventory.inventory_id == 0 || inventory.profile_id == 0 {
        return Err(UwbFaultHypothesisIntegrityError::InvalidInventoryIdentity);
    }
    if inventory.input_manifest_sha256.iter().all(|byte| *byte == 0) {
        return Err(UwbFaultHypothesisIntegrityError::MissingInputManifestCommitment);
    }
    if inventory.inventory_sha256.iter().all(|byte| *byte == 0) {
        return Err(UwbFaultHypothesisIntegrityError::MissingInventoryCommitment);
    }
    if inventory.expected_hypothesis_count == 0 || inventory.hypotheses.is_empty() {
        return Err(UwbFaultHypothesisIntegrityError::EmptyHypothesisInventory);
    }
    if inventory.expected_hypothesis_count > config.maximum_hypothesis_count {
        return Err(UwbFaultHypothesisIntegrityError::ExpectedHypothesisCountExceedsBudget);
    }
    if inventory.hypotheses.len() > config.maximum_hypothesis_count {
        return Err(UwbFaultHypothesisIntegrityError::ObservedHypothesisCountExceedsBudget);
    }
    if inventory.declared_max_fault_cardinality == 0
        || inventory.declared_max_fault_cardinality > config.maximum_fault_cardinality
    {
        return Err(UwbFaultHypothesisIntegrityError::FaultCardinalityExceedsBudget);
    }

    let mut hypothesis_ids = BTreeSet::new();
    let mut nominal_count = 0_usize;
    let mut nominal_center = None;
    let mut every_position_set_bounded = true;

    for hypothesis in &inventory.hypotheses {
        if hypothesis.hypothesis_id == 0 {
            return Err(UwbFaultHypothesisIntegrityError::InvalidHypothesisIdentity);
        }
        if !hypothesis_ids.insert(hypothesis.hypothesis_id) {
            return Err(UwbFaultHypothesisIntegrityError::DuplicateHypothesisIdentity(
                hypothesis.hypothesis_id,
            ));
        }

        if hypothesis.kind == UwbFaultHypothesisKind::Nominal {
            nominal_count += 1;
            if !hypothesis.dependency_group_ids.is_empty()
                || !hypothesis.affected_observation_ids.is_empty()
            {
                return Err(UwbFaultHypothesisIntegrityError::InvalidNominalHypothesisShape);
            }
            if let QualifiedPositionSet3d::Bounded { center_m, .. } = hypothesis.position_set {
                nominal_center = Some(center_m);
            }
        } else {
            if hypothesis.dependency_group_ids.is_empty()
                || hypothesis.affected_observation_ids.is_empty()
                || hypothesis.dependency_group_ids.len() > inventory.declared_max_fault_cardinality
                || hypothesis.dependency_group_ids.len() > config.maximum_fault_cardinality
            {
                return Err(UwbFaultHypothesisIntegrityError::InvalidFaultHypothesisShape(
                    hypothesis.hypothesis_id,
                ));
            }
            validate_unique_nonzero(
                &hypothesis.dependency_group_ids,
                hypothesis.hypothesis_id,
            )?;
            validate_unique_observations(
                &hypothesis.affected_observation_ids,
                hypothesis.hypothesis_id,
            )?;
        }

        match hypothesis.position_set {
            QualifiedPositionSet3d::Bounded {
                center_m,
                absolute_error_radius_m,
            } => {
                if !center_m.iter().all(|coordinate| coordinate.is_finite())
                    || !absolute_error_radius_m.is_finite()
                    || absolute_error_radius_m < 0.0
                {
                    return Err(UwbFaultHypothesisIntegrityError::InvalidPositionSet(
                        hypothesis.hypothesis_id,
                    ));
                }
            }
            QualifiedPositionSet3d::Unbounded { .. } => {
                every_position_set_bounded = false;
            }
        }
    }

    match nominal_count {
        0 => return Err(UwbFaultHypothesisIntegrityError::MissingNominalHypothesis),
        1 => {}
        _ => return Err(UwbFaultHypothesisIntegrityError::MultipleNominalHypotheses),
    }

    let hypothesis_inventory_complete = inventory.enumeration_complete
        && inventory.expected_hypothesis_count == inventory.hypotheses.len();
    let mut hypotheses = inventory.hypotheses.clone();
    hypotheses.sort_by_key(|hypothesis| hypothesis.hypothesis_id);

    let mut reports = Vec::with_capacity(hypotheses.len());
    let mut maximum_outer_radius = 0.0_f64;
    let mut arithmetic_valid = true;

    for hypothesis in &hypotheses {
        let report = match (nominal_center, hypothesis.position_set) {
            (
                Some(nominal),
                QualifiedPositionSet3d::Bounded {
                    center_m,
                    absolute_error_radius_m,
                },
            ) => {
                let separation = euclidean_distance_3d(nominal, center_m).ok_or(
                    UwbFaultHypothesisIntegrityError::ArithmeticOverflow(
                        hypothesis.hypothesis_id,
                    ),
                )?;
                let outer_radius = separation + absolute_error_radius_m;
                if !outer_radius.is_finite() {
                    return Err(UwbFaultHypothesisIntegrityError::ArithmeticOverflow(
                        hypothesis.hypothesis_id,
                    ));
                }
                maximum_outer_radius = maximum_outer_radius.max(outer_radius);
                UwbHypothesisSeparation3d {
                    hypothesis_id: hypothesis.hypothesis_id,
                    kind: hypothesis.kind,
                    dependency_group_ids: hypothesis.dependency_group_ids.clone(),
                    affected_observation_ids: hypothesis.affected_observation_ids.clone(),
                    position_set_bounded: true,
                    separation_from_nominal_m: Some(separation),
                    hypothesis_uncertainty_radius_m: Some(absolute_error_radius_m),
                    outer_radius_from_nominal_m: Some(outer_radius),
                }
            }
            (_, QualifiedPositionSet3d::Unbounded { .. }) => {
                arithmetic_valid = false;
                UwbHypothesisSeparation3d {
                    hypothesis_id: hypothesis.hypothesis_id,
                    kind: hypothesis.kind,
                    dependency_group_ids: hypothesis.dependency_group_ids.clone(),
                    affected_observation_ids: hypothesis.affected_observation_ids.clone(),
                    position_set_bounded: false,
                    separation_from_nominal_m: None,
                    hypothesis_uncertainty_radius_m: None,
                    outer_radius_from_nominal_m: None,
                }
            }
            (None, QualifiedPositionSet3d::Bounded { .. }) => {
                arithmetic_valid = false;
                UwbHypothesisSeparation3d {
                    hypothesis_id: hypothesis.hypothesis_id,
                    kind: hypothesis.kind,
                    dependency_group_ids: hypothesis.dependency_group_ids.clone(),
                    affected_observation_ids: hypothesis.affected_observation_ids.clone(),
                    position_set_bounded: true,
                    separation_from_nominal_m: None,
                    hypothesis_uncertainty_radius_m: None,
                    outer_radius_from_nominal_m: None,
                }
            }
        };
        reports.push(report);
    }

    let may_issue_protection_radius = hypothesis_inventory_complete
        && every_position_set_bounded
        && arithmetic_valid
        && nominal_center.is_some();

    let protection_radius_m = may_issue_protection_radius.then_some(maximum_outer_radius);
    let disposition = match protection_radius_m {
        Some(radius) if radius <= config.alert_limit_m => {
            UwbFaultHypothesisIntegrityDisposition::QualifiedWithinAlertLimit
        }
        Some(_) => UwbFaultHypothesisIntegrityDisposition::ExceedsAlertLimit,
        None => UwbFaultHypothesisIntegrityDisposition::IntegrityUncertain,
    };

    Ok(UwbFaultHypothesisIntegrityReport3d {
        inventory_id: inventory.inventory_id,
        profile_id: inventory.profile_id,
        input_manifest_sha256: inventory.input_manifest_sha256,
        inventory_sha256: inventory.inventory_sha256,
        expected_hypothesis_count: inventory.expected_hypothesis_count,
        evaluated_hypothesis_count: inventory.hypotheses.len(),
        hypothesis_inventory_complete,
        every_hypothesis_position_set_bounded: every_position_set_bounded,
        alert_limit_m: config.alert_limit_m,
        protection_radius_m,
        disposition,
        hypotheses: reports,
    })
}

fn validate_config(
    config: UwbFaultHypothesisIntegrityConfig,
) -> Result<(), UwbFaultHypothesisIntegrityError> {
    if config.maximum_hypothesis_count == 0
        || config.maximum_fault_cardinality == 0
        || !config.alert_limit_m.is_finite()
        || config.alert_limit_m <= 0.0
    {
        return Err(UwbFaultHypothesisIntegrityError::InvalidConfiguration);
    }
    Ok(())
}

fn validate_unique_nonzero(
    ids: &[u64],
    hypothesis_id: u64,
) -> Result<(), UwbFaultHypothesisIntegrityError> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if *id == 0 {
            return Err(UwbFaultHypothesisIntegrityError::InvalidDependencyGroupIdentity(
                *id,
            ));
        }
        if !unique.insert(*id) {
            return Err(
                UwbFaultHypothesisIntegrityError::DuplicateDependencyGroupIdentity {
                    hypothesis_id,
                    group_id: *id,
                },
            );
        }
    }
    Ok(())
}

fn validate_unique_observations(
    ids: &[u64],
    hypothesis_id: u64,
) -> Result<(), UwbFaultHypothesisIntegrityError> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if *id == 0 {
            return Err(UwbFaultHypothesisIntegrityError::InvalidObservationIdentity(
                *id,
            ));
        }
        if !unique.insert(*id) {
            return Err(
                UwbFaultHypothesisIntegrityError::DuplicateObservationIdentity {
                    hypothesis_id,
                    observation_id: *id,
                },
            );
        }
    }
    Ok(())
}

fn euclidean_distance_3d(left: [f64; DIM], right: [f64; DIM]) -> Option<f64> {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    if !dx.is_finite() || !dy.is_finite() || !dz.is_finite() {
        return None;
    }
    let distance = dx.hypot(dy).hypot(dz);
    distance.is_finite().then_some(distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(alert_limit_m: f64) -> UwbFaultHypothesisIntegrityConfig {
        UwbFaultHypothesisIntegrityConfig {
            maximum_hypothesis_count: 32,
            maximum_fault_cardinality: 3,
            alert_limit_m,
        }
    }

    fn nominal(center_m: [f64; 3], radius_m: f64) -> UwbFaultHypothesisSolution3d {
        UwbFaultHypothesisSolution3d {
            hypothesis_id: 1,
            kind: UwbFaultHypothesisKind::Nominal,
            dependency_group_ids: Vec::new(),
            affected_observation_ids: Vec::new(),
            position_set: QualifiedPositionSet3d::Bounded {
                center_m,
                absolute_error_radius_m: radius_m,
            },
        }
    }

    fn fault(
        hypothesis_id: u64,
        center_m: [f64; 3],
        radius_m: f64,
        groups: &[u64],
    ) -> UwbFaultHypothesisSolution3d {
        UwbFaultHypothesisSolution3d {
            hypothesis_id,
            kind: UwbFaultHypothesisKind::AnchorExcluded,
            dependency_group_ids: groups.to_vec(),
            affected_observation_ids: vec![100 + hypothesis_id],
            position_set: QualifiedPositionSet3d::Bounded {
                center_m,
                absolute_error_radius_m: radius_m,
            },
        }
    }

    fn inventory(
        hypotheses: Vec<UwbFaultHypothesisSolution3d>,
    ) -> UwbFaultHypothesisInventory3d {
        UwbFaultHypothesisInventory3d {
            inventory_id: 7,
            profile_id: 9,
            input_manifest_sha256: [0x11; 32],
            inventory_sha256: [0x22; 32],
            expected_hypothesis_count: hypotheses.len(),
            enumeration_complete: true,
            declared_max_fault_cardinality: 3,
            hypotheses,
        }
    }

    #[test]
    fn bounds_union_of_hypothesis_sets_without_rss_combination() {
        let input = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.5),
            fault(2, [2.0, 0.0, 0.0], 0.25, &[10]),
        ]);
        let report = evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)).unwrap();
        assert_eq!(
            report.disposition,
            UwbFaultHypothesisIntegrityDisposition::QualifiedWithinAlertLimit
        );
        assert_eq!(report.protection_radius_m, Some(2.25));
        assert_eq!(report.hypotheses[1].separation_from_nominal_m, Some(2.0));
        assert_eq!(report.hypotheses[1].outer_radius_from_nominal_m, Some(2.25));
    }

    #[test]
    fn nominal_uncertainty_is_part_of_the_outer_envelope() {
        let input = inventory(vec![nominal([0.0, 0.0, 0.0], 0.75)]);
        let report = evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(1.0)).unwrap();
        assert_eq!(report.protection_radius_m, Some(0.75));
    }

    #[test]
    fn protection_radius_above_profile_limit_is_not_qualified() {
        let input = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [2.0, 0.0, 0.0], 0.25, &[10]),
        ]);
        let report = evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(2.0)).unwrap();
        assert_eq!(
            report.disposition,
            UwbFaultHypothesisIntegrityDisposition::ExceedsAlertLimit
        );
        assert_eq!(report.protection_radius_m, Some(2.25));
    }

    #[test]
    fn incomplete_enumeration_never_emits_a_protection_radius() {
        let mut input = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [0.2, 0.0, 0.0], 0.1, &[10]),
        ]);
        input.enumeration_complete = false;
        let report = evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)).unwrap();
        assert!(!report.hypothesis_inventory_complete);
        assert_eq!(report.protection_radius_m, None);
        assert_eq!(
            report.disposition,
            UwbFaultHypothesisIntegrityDisposition::IntegrityUncertain
        );
    }

    #[test]
    fn expected_count_mismatch_is_treated_as_incomplete() {
        let mut input = inventory(vec![nominal([0.0, 0.0, 0.0], 0.2)]);
        input.expected_hypothesis_count = 2;
        let report = evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)).unwrap();
        assert!(!report.hypothesis_inventory_complete);
        assert_eq!(report.protection_radius_m, None);
    }

    #[test]
    fn unbounded_hypothesis_suppresses_the_entire_protection_radius() {
        let mut input = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [0.0, 0.0, 0.0], 0.0, &[10]),
        ]);
        input.hypotheses[1].position_set = QualifiedPositionSet3d::Unbounded {
            reason: UnboundedPositionReason::UnresolvedGeometry,
        };
        let report = evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)).unwrap();
        assert!(!report.every_hypothesis_position_set_bounded);
        assert_eq!(report.protection_radius_m, None);
        assert_eq!(
            report.disposition,
            UwbFaultHypothesisIntegrityDisposition::IntegrityUncertain
        );
    }

    #[test]
    fn a_new_hypothesis_cannot_shrink_the_outer_envelope() {
        let first = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [1.0, 0.0, 0.0], 0.1, &[10]),
        ]);
        let first_report =
            evaluate_uwb_fault_hypothesis_envelope_3d(&first, config(3.0)).unwrap();

        let expanded = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [1.0, 0.0, 0.0], 0.1, &[10]),
            fault(3, [2.0, 0.0, 0.0], 0.2, &[11]),
        ]);
        let expanded_report =
            evaluate_uwb_fault_hypothesis_envelope_3d(&expanded, config(3.0)).unwrap();
        assert!(expanded_report.protection_radius_m.unwrap()
            >= first_report.protection_radius_m.unwrap());
    }

    #[test]
    fn widening_a_hypothesis_set_cannot_shrink_the_envelope() {
        let narrow = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [1.0, 0.0, 0.0], 0.1, &[10]),
        ]);
        let wide = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [1.0, 0.0, 0.0], 0.5, &[10]),
        ]);
        let narrow_radius =
            evaluate_uwb_fault_hypothesis_envelope_3d(&narrow, config(3.0))
                .unwrap()
                .protection_radius_m
                .unwrap();
        let wide_radius =
            evaluate_uwb_fault_hypothesis_envelope_3d(&wide, config(3.0))
                .unwrap()
                .protection_radius_m
                .unwrap();
        assert!(wide_radius >= narrow_radius);
    }

    #[test]
    fn rejects_duplicate_hypothesis_dependency_and_observation_identities() {
        let mut duplicated_hypothesis = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [1.0, 0.0, 0.0], 0.1, &[10]),
            fault(2, [2.0, 0.0, 0.0], 0.1, &[11]),
        ]);
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(
                &duplicated_hypothesis,
                config(3.0)
            ),
            Err(UwbFaultHypothesisIntegrityError::DuplicateHypothesisIdentity(2))
        );

        duplicated_hypothesis = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [1.0, 0.0, 0.0], 0.1, &[10, 10]),
        ]);
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(
                &duplicated_hypothesis,
                config(3.0)
            ),
            Err(UwbFaultHypothesisIntegrityError::DuplicateDependencyGroupIdentity {
                hypothesis_id: 2,
                group_id: 10
            })
        );

        let mut duplicate_observation = fault(2, [1.0, 0.0, 0.0], 0.1, &[10]);
        duplicate_observation.affected_observation_ids = vec![7, 7];
        let input = inventory(vec![nominal([0.0, 0.0, 0.0], 0.2), duplicate_observation]);
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)),
            Err(UwbFaultHypothesisIntegrityError::DuplicateObservationIdentity {
                hypothesis_id: 2,
                observation_id: 7
            })
        );
    }

    #[test]
    fn requires_one_nominal_and_nonempty_fault_scope() {
        let missing_nominal = inventory(vec![fault(2, [1.0, 0.0, 0.0], 0.1, &[10])]);
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&missing_nominal, config(3.0)),
            Err(UwbFaultHypothesisIntegrityError::MissingNominalHypothesis)
        );

        let mut no_fault_scope = fault(2, [1.0, 0.0, 0.0], 0.1, &[10]);
        no_fault_scope.affected_observation_ids.clear();
        let input = inventory(vec![nominal([0.0, 0.0, 0.0], 0.2), no_fault_scope]);
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)),
            Err(UwbFaultHypothesisIntegrityError::InvalidFaultHypothesisShape(2))
        );
    }

    #[test]
    fn enforces_hypothesis_and_fault_cardinality_budgets() {
        let input = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [1.0, 0.0, 0.0], 0.1, &[10]),
        ]);
        let mut low_hypothesis_budget = config(3.0);
        low_hypothesis_budget.maximum_hypothesis_count = 1;
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&input, low_hypothesis_budget),
            Err(UwbFaultHypothesisIntegrityError::ExpectedHypothesisCountExceedsBudget)
        );

        let mut excessive_cardinality = input;
        excessive_cardinality.declared_max_fault_cardinality = 4;
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(
                &excessive_cardinality,
                config(3.0)
            ),
            Err(UwbFaultHypothesisIntegrityError::FaultCardinalityExceedsBudget)
        );
    }

    #[test]
    fn rejects_nonfinite_position_sets_and_missing_commitments() {
        let mut input = inventory(vec![nominal([f64::NAN, 0.0, 0.0], 0.2)]);
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)),
            Err(UwbFaultHypothesisIntegrityError::InvalidPositionSet(1))
        );

        input = inventory(vec![nominal([0.0, 0.0, 0.0], 0.2)]);
        input.inventory_sha256 = [0; 32];
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)),
            Err(UwbFaultHypothesisIntegrityError::MissingInventoryCommitment)
        );
    }

    #[test]
    fn an_unbounded_nominal_set_suppresses_the_protection_radius() {
        let mut input = inventory(vec![nominal([0.0, 0.0, 0.0], 0.2)]);
        input.hypotheses[0].position_set = QualifiedPositionSet3d::Unbounded {
            reason: UnboundedPositionReason::MissingQualifiedRangeBound,
        };
        let report = evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)).unwrap();
        assert_eq!(report.protection_radius_m, None);
        assert_eq!(
            report.disposition,
            UwbFaultHypothesisIntegrityDisposition::IntegrityUncertain
        );
    }

    #[test]
    fn rejects_multiple_nominal_hypotheses() {
        let mut second_nominal = nominal([1.0, 0.0, 0.0], 0.2);
        second_nominal.hypothesis_id = 2;
        let input = inventory(vec![nominal([0.0, 0.0, 0.0], 0.2), second_nominal]);
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)),
            Err(UwbFaultHypothesisIntegrityError::MultipleNominalHypotheses)
        );
    }

    #[test]
    fn rejects_zero_dependency_group_identity() {
        let input = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [1.0, 0.0, 0.0], 0.1, &[0]),
        ]);
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&input, config(3.0)),
            Err(UwbFaultHypothesisIntegrityError::InvalidDependencyGroupIdentity(0))
        );
    }

    #[test]
    fn invalid_alert_limit_and_distance_overflow_fail_closed() {
        let input = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(2, [1.0, 0.0, 0.0], 0.1, &[10]),
        ]);
        let mut invalid_config = config(3.0);
        invalid_config.alert_limit_m = f64::NAN;
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&input, invalid_config),
            Err(UwbFaultHypothesisIntegrityError::InvalidConfiguration)
        );

        let overflow = inventory(vec![
            nominal([-1.0e308, 0.0, 0.0], 0.0),
            fault(2, [1.0e308, 0.0, 0.0], 0.0, &[10]),
        ]);
        assert_eq!(
            evaluate_uwb_fault_hypothesis_envelope_3d(&overflow, config(3.0)),
            Err(UwbFaultHypothesisIntegrityError::ArithmeticOverflow(2))
        );
    }

    #[test]
    fn canonical_report_is_permutation_invariant() {
        let first = inventory(vec![
            nominal([0.0, 0.0, 0.0], 0.2),
            fault(3, [2.0, 0.0, 0.0], 0.1, &[11]),
            fault(2, [1.0, 0.0, 0.0], 0.1, &[10]),
        ]);
        let second = inventory(vec![
            fault(2, [1.0, 0.0, 0.0], 0.1, &[10]),
            fault(3, [2.0, 0.0, 0.0], 0.1, &[11]),
            nominal([0.0, 0.0, 0.0], 0.2),
        ]);
        let left = evaluate_uwb_fault_hypothesis_envelope_3d(&first, config(3.0)).unwrap();
        let right = evaluate_uwb_fault_hypothesis_envelope_3d(&second, config(3.0)).unwrap();
        assert_eq!(left.protection_radius_m, right.protection_radius_m);
        assert_eq!(left.hypotheses, right.hypotheses);
    }
}
