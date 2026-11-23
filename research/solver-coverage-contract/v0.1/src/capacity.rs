// SPDX-License-Identifier: AGPL-3.0-or-later
//! Contact-manifold lineage and temporally scoped capacity diagnostics.
//!
//! Geometric contact candidates, reduced manifold points, solver contacts and
//! constraint rows are distinct populations. This module records their lineage
//! without treating point counts as solver-row counts or physical proof.

use std::collections::{BTreeMap, BTreeSet};

use super::{ConstraintRowFamily, ValidationError, is_sha256, non_empty};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum GeometricPointDisposition {
    Retained = 1,
    ReducedByPolicy = 2,
    CapacityDropped = 3,
    Invalid = 4,
    Unsupported = 5,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometricPointRecord {
    pub point_id: String,
    pub disposition: GeometricPointDisposition,
    pub reason: Option<String>,
}

impl GeometricPointRecord {
    fn validate(&self, path: &str) -> Result<(), ValidationError> {
        if !non_empty(&self.point_id) {
            return Err(ValidationError::new(path, "point_id must not be empty"));
        }
        match self.disposition {
            GeometricPointDisposition::Retained => {
                if self.reason.is_some() {
                    return Err(ValidationError::new(
                        path,
                        "Retained point cannot carry an omission reason",
                    ));
                }
            }
            _ => {
                if !self.reason.as_deref().is_some_and(non_empty) {
                    return Err(ValidationError::new(
                        path,
                        "non-retained point requires an explicit reason",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SolverContactDisposition {
    Solved = 1,
    NotConverged = 2,
    SleepingSkipped = 3,
    CapacityDropped = 4,
    Unsupported = 5,
    NotSubmitted = 6,
    ExecutionAborted = 7,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverContactGroup {
    /// Stable identity for a submitted solver contact or cluster.
    pub solver_contact_id: String,
    /// Geometry points represented by this solver contact/cluster.
    pub source_point_ids: Vec<String>,
    pub disposition: SolverContactDisposition,
    pub reason: Option<String>,
}

impl SolverContactGroup {
    fn validate(&self, path: &str) -> Result<(), ValidationError> {
        if !non_empty(&self.solver_contact_id) {
            return Err(ValidationError::new(
                format!("{path}.solver_contact_id"),
                "must not be empty",
            ));
        }
        if self.source_point_ids.is_empty() {
            return Err(ValidationError::new(
                format!("{path}.source_point_ids"),
                "a solver contact must identify at least one source geometry point",
            ));
        }
        let mut ids = BTreeSet::new();
        for id in &self.source_point_ids {
            if !non_empty(id) || !ids.insert(id) {
                return Err(ValidationError::new(
                    format!("{path}.source_point_ids"),
                    "source point IDs must be non-empty and unique within the group",
                ));
            }
        }
        match self.disposition {
            SolverContactDisposition::Solved => {
                if self.reason.is_some() {
                    return Err(ValidationError::new(
                        path,
                        "Solved contact cannot carry a failure reason",
                    ));
                }
            }
            _ => {
                if !self.reason.as_deref().is_some_and(non_empty) {
                    return Err(ValidationError::new(
                        path,
                        "non-solved contact requires an explicit reason",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactManifoldLineage {
    pub pair_id: String,
    /// Present when any generated point is intentionally reduced by policy.
    pub reduction_policy_id: Option<String>,
    /// Inline manifold point capacity; None means the adapter declares no fixed cap.
    pub point_capacity: Option<u64>,
    /// Exactly one disposition for each generated geometric candidate.
    pub generated_points: Vec<GeometricPointRecord>,
    /// Solver contacts may represent one point or a cluster of geometric points.
    pub solver_contacts: Vec<SolverContactGroup>,
    /// True only when every retained point is mapped to exactly one solver contact group.
    pub solver_mapping_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactManifoldReport {
    pub pair_id: String,
    pub generated_points: u64,
    pub retained_points: u64,
    pub reduced_by_policy: u64,
    pub capacity_dropped: u64,
    pub invalid_points: u64,
    pub unsupported_points: u64,
    pub solver_groups: u64,
    pub solver_mapped_points: u64,
    pub solver_solved_points: u64,
    pub solver_not_converged_points: u64,
    pub solver_sleeping_skipped_points: u64,
    pub solver_capacity_dropped_points: u64,
    pub solver_unsupported_points: u64,
    pub solver_not_submitted_points: u64,
    pub solver_aborted_points: u64,
    pub solver_mapping_complete: bool,
    pub point_capacity_respected: bool,
}

impl ContactManifoldLineage {
    pub fn validate(&self) -> Result<ContactManifoldReport, ValidationError> {
        if !non_empty(&self.pair_id) {
            return Err(ValidationError::new("manifold.pair_id", "must not be empty"));
        }
        if let Some(policy) = &self.reduction_policy_id {
            if !non_empty(policy) {
                return Err(ValidationError::new(
                    "manifold.reduction_policy_id",
                    "must not be empty when present",
                ));
            }
        }

        let mut point_records = BTreeMap::new();
        let mut generated_points = 0_u64;
        let mut retained_points = 0_u64;
        let mut reduced_by_policy = 0_u64;
        let mut capacity_dropped = 0_u64;
        let mut invalid_points = 0_u64;
        let mut unsupported_points = 0_u64;

        for (index, point) in self.generated_points.iter().enumerate() {
            let path = format!("manifold.generated_points[{index}]");
            point.validate(&path)?;
            if point_records.insert(point.point_id.as_str(), point).is_some() {
                return Err(ValidationError::new(&path, "duplicate geometric point ID"));
            }
            generated_points = generated_points.checked_add(1).ok_or_else(|| {
                ValidationError::new("manifold.generated_points", "point count overflow")
            })?;
            match point.disposition {
                GeometricPointDisposition::Retained => retained_points += 1,
                GeometricPointDisposition::ReducedByPolicy => reduced_by_policy += 1,
                GeometricPointDisposition::CapacityDropped => capacity_dropped += 1,
                GeometricPointDisposition::Invalid => invalid_points += 1,
                GeometricPointDisposition::Unsupported => unsupported_points += 1,
            }
        }

        if reduced_by_policy > 0 && self.reduction_policy_id.is_none() {
            return Err(ValidationError::new(
                "manifold.reduction_policy_id",
                "a reduction policy identity is required when geometric points are reduced",
            ));
        }

        let point_capacity_respected = self
            .point_capacity
            .is_none_or(|capacity| retained_points <= capacity);
        if !point_capacity_respected {
            return Err(ValidationError::new(
                "manifold.point_capacity",
                "retained points exceed the declared inline point capacity",
            ));
        }

        let mut solver_ids = BTreeSet::new();
        let mut mapped_source_ids = BTreeSet::new();
        let mut solver_mapped_points = 0_u64;
        let mut solver_solved_points = 0_u64;
        let mut solver_not_converged_points = 0_u64;
        let mut solver_sleeping_skipped_points = 0_u64;
        let mut solver_capacity_dropped_points = 0_u64;
        let mut solver_unsupported_points = 0_u64;
        let mut solver_not_submitted_points = 0_u64;
        let mut solver_aborted_points = 0_u64;

        for (index, group) in self.solver_contacts.iter().enumerate() {
            let path = format!("manifold.solver_contacts[{index}]");
            group.validate(&path)?;
            if !solver_ids.insert(&group.solver_contact_id) {
                return Err(ValidationError::new(&path, "duplicate solver contact ID"));
            }
            for point_id in &group.source_point_ids {
                let source = point_records.get(point_id.as_str()).ok_or_else(|| {
                    ValidationError::new(
                        &path,
                        "solver contact references an unknown geometric point",
                    )
                })?;
                if source.disposition != GeometricPointDisposition::Retained {
                    return Err(ValidationError::new(
                        &path,
                        "solver contact may reference only retained geometric points",
                    ));
                }
                if !mapped_source_ids.insert(point_id.as_str()) {
                    return Err(ValidationError::new(
                        &path,
                        "a retained geometric point maps to multiple solver contact groups",
                    ));
                }
                solver_mapped_points += 1;
                match group.disposition {
                    SolverContactDisposition::Solved => solver_solved_points += 1,
                    SolverContactDisposition::NotConverged => solver_not_converged_points += 1,
                    SolverContactDisposition::SleepingSkipped => solver_sleeping_skipped_points += 1,
                    SolverContactDisposition::CapacityDropped => solver_capacity_dropped_points += 1,
                    SolverContactDisposition::Unsupported => solver_unsupported_points += 1,
                    SolverContactDisposition::NotSubmitted => solver_not_submitted_points += 1,
                    SolverContactDisposition::ExecutionAborted => solver_aborted_points += 1,
                }
            }
        }

        let solver_mapping_complete = self.solver_mapping_complete
            && solver_mapped_points == retained_points
            && mapped_source_ids.len() == retained_points as usize;
        if self.solver_mapping_complete && !solver_mapping_complete {
            return Err(ValidationError::new(
                "manifold.solver_mapping_complete",
                "declared complete mapping does not cover every retained point exactly once",
            ));
        }

        Ok(ContactManifoldReport {
            pair_id: self.pair_id.clone(),
            generated_points,
            retained_points,
            reduced_by_policy,
            capacity_dropped,
            invalid_points,
            unsupported_points,
            solver_groups: self.solver_contacts.len() as u64,
            solver_mapped_points,
            solver_solved_points,
            solver_not_converged_points,
            solver_sleeping_skipped_points,
            solver_capacity_dropped_points,
            solver_unsupported_points,
            solver_not_submitted_points,
            solver_aborted_points,
            solver_mapping_complete,
            point_capacity_respected,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum OverflowFlagLifetime {
    StepLocal = 1,
    StickyUntilReset = 2,
    Cumulative = 3,
    Unknown = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CapacityDiagnosticScope {
    World = 1,
    GlobalArticulation = 2,
    Device = 3,
    BackendWide = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum StepOverflowAttribution {
    OccurredThisStep = 1,
    DidNotOccurThisStep = 2,
    Unattributable = 3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverflowObservation {
    pub backend_profile_sha256: String,
    pub scope_kind: CapacityDiagnosticScope,
    pub scope_id: String,
    pub row_family: Option<ConstraintRowFamily>,
    pub step_index: u64,
    pub reset_epoch: Option<u64>,
    pub last_confirmed_reset_step: Option<u64>,
    pub flag_lifetime: OverflowFlagLifetime,
    pub observed_before_step: bool,
    pub observed_after_step: bool,
    /// A backend-provided per-step or reset-epoch increment, when available.
    pub new_overflow_count: Option<u64>,
    pub source_semantics_id: String,
}

impl OverflowObservation {
    pub fn validate(&self) -> Result<StepOverflowAttribution, ValidationError> {
        if !is_sha256(&self.backend_profile_sha256) {
            return Err(ValidationError::new(
                "overflow.backend_profile_sha256",
                "must be exactly 64 hexadecimal characters",
            ));
        }
        if !non_empty(&self.scope_id) || !non_empty(&self.source_semantics_id) {
            return Err(ValidationError::new(
                "overflow",
                "scope_id and source_semantics_id must not be empty",
            ));
        }
        if self
            .last_confirmed_reset_step
            .is_some_and(|reset_step| reset_step > self.step_index)
        {
            return Err(ValidationError::new(
                "overflow.last_confirmed_reset_step",
                "cannot be later than the observed step",
            ));
        }

        if let Some(count) = self.new_overflow_count {
            return Ok(if count > 0 {
                StepOverflowAttribution::OccurredThisStep
            } else {
                StepOverflowAttribution::DidNotOccurThisStep
            });
        }

        let transition = match (self.observed_before_step, self.observed_after_step) {
            (false, true) => Some(StepOverflowAttribution::OccurredThisStep),
            (false, false) => Some(StepOverflowAttribution::DidNotOccurThisStep),
            (true, true) | (true, false) => None,
        };

        Ok(match self.flag_lifetime {
            OverflowFlagLifetime::StepLocal => {
                if self.observed_after_step {
                    StepOverflowAttribution::OccurredThisStep
                } else {
                    StepOverflowAttribution::DidNotOccurThisStep
                }
            }
            OverflowFlagLifetime::StickyUntilReset | OverflowFlagLifetime::Cumulative => {
                transition.unwrap_or(StepOverflowAttribution::Unattributable)
            }
            OverflowFlagLifetime::Unknown => StepOverflowAttribution::Unattributable,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticResetReceipt {
    pub backend_profile_sha256: String,
    pub scope_kind: CapacityDiagnosticScope,
    pub scope_id: String,
    pub reset_id: String,
    pub step_index: u64,
    pub epoch_before: u64,
    pub epoch_after: u64,
    pub source_semantics_id: String,
}

impl DiagnosticResetReceipt {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if !is_sha256(&self.backend_profile_sha256) {
            return Err(ValidationError::new(
                "reset.backend_profile_sha256",
                "must be exactly 64 hexadecimal characters",
            ));
        }
        if !non_empty(&self.scope_id)
            || !non_empty(&self.reset_id)
            || !non_empty(&self.source_semantics_id)
        {
            return Err(ValidationError::new(
                "reset",
                "scope_id, reset_id and source_semantics_id must not be empty",
            ));
        }
        if self.epoch_before.checked_add(1) != Some(self.epoch_after) {
            return Err(ValidationError::new(
                "reset.epoch_after",
                "must equal epoch_before + 1 without overflow",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn retained(id: &str) -> GeometricPointRecord {
        GeometricPointRecord {
            point_id: id.into(),
            disposition: GeometricPointDisposition::Retained,
            reason: None,
        }
    }

    fn overflow(lifetime: OverflowFlagLifetime, before: bool, after: bool) -> OverflowObservation {
        OverflowObservation {
            backend_profile_sha256: "a".repeat(64),
            scope_kind: CapacityDiagnosticScope::World,
            scope_id: "world-0".into(),
            row_family: Some(ConstraintRowFamily::Contact),
            step_index: 12,
            reset_epoch: Some(3),
            last_confirmed_reset_step: Some(8),
            flag_lifetime: lifetime,
            observed_before_step: before,
            observed_after_step: after,
            new_overflow_count: None,
            source_semantics_id: "steel-v1-constraint-overflow".into(),
        }
    }

    #[test]
    fn geometric_reduction_is_separate_from_solver_contact_groups() {
        let receipt = ContactManifoldLineage {
            pair_id: "body-a/body-b".into(),
            reduction_policy_id: Some("area-preserving-v1".into()),
            point_capacity: Some(8),
            generated_points: vec![
                retained("p0"),
                retained("p1"),
                GeometricPointRecord {
                    point_id: "p2".into(),
                    disposition: GeometricPointDisposition::ReducedByPolicy,
                    reason: Some("lower wrench-support score under area-preserving-v1".into()),
                },
            ],
            solver_contacts: vec![SolverContactGroup {
                solver_contact_id: "cluster-0".into(),
                source_point_ids: vec!["p0".into(), "p1".into()],
                disposition: SolverContactDisposition::Solved,
                reason: None,
            }],
            solver_mapping_complete: true,
        };
        let report = receipt.validate().expect("valid contact lineage");
        assert_eq!(report.generated_points, 3);
        assert_eq!(report.retained_points, 2);
        assert_eq!(report.reduced_by_policy, 1);
        assert_eq!(report.solver_groups, 1);
        assert_eq!(report.solver_solved_points, 2);
        assert!(report.solver_mapping_complete);
    }

    #[test]
    fn solver_group_cannot_reference_reduced_geometry() {
        let receipt = ContactManifoldLineage {
            pair_id: "pair-0".into(),
            reduction_policy_id: Some("policy-0".into()),
            point_capacity: Some(4),
            generated_points: vec![GeometricPointRecord {
                point_id: "p0".into(),
                disposition: GeometricPointDisposition::ReducedByPolicy,
                reason: Some("reduced".into()),
            }],
            solver_contacts: vec![SolverContactGroup {
                solver_contact_id: "s0".into(),
                source_point_ids: vec!["p0".into()],
                disposition: SolverContactDisposition::Solved,
                reason: None,
            }],
            solver_mapping_complete: false,
        };
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn reduced_points_require_policy_identity() {
        let receipt = ContactManifoldLineage {
            pair_id: "pair-0".into(),
            reduction_policy_id: None,
            point_capacity: Some(4),
            generated_points: vec![GeometricPointRecord {
                point_id: "p0".into(),
                disposition: GeometricPointDisposition::ReducedByPolicy,
                reason: Some("reduced".into()),
            }],
            solver_contacts: vec![],
            solver_mapping_complete: false,
        };
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn retained_points_cannot_exceed_declared_capacity() {
        let receipt = ContactManifoldLineage {
            pair_id: "pair-0".into(),
            reduction_policy_id: None,
            point_capacity: Some(1),
            generated_points: vec![retained("p0"), retained("p1")],
            solver_contacts: vec![],
            solver_mapping_complete: false,
        };
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn sticky_flag_transition_can_be_attributed_to_current_step() {
        assert_eq!(
            overflow(OverflowFlagLifetime::StickyUntilReset, false, true)
                .validate()
                .unwrap(),
            StepOverflowAttribution::OccurredThisStep
        );
    }

    #[test]
    fn sticky_flag_that_was_already_set_is_unattributable_without_counter() {
        assert_eq!(
            overflow(OverflowFlagLifetime::StickyUntilReset, true, true)
                .validate()
                .unwrap(),
            StepOverflowAttribution::Unattributable
        );
    }

    #[test]
    fn step_local_false_flag_is_not_current_step_overflow() {
        assert_eq!(
            overflow(OverflowFlagLifetime::StepLocal, false, false)
                .validate()
                .unwrap(),
            StepOverflowAttribution::DidNotOccurThisStep
        );
    }

    #[test]
    fn backend_increment_overrides_sticky_flag_ambiguity() {
        let mut item = overflow(OverflowFlagLifetime::StickyUntilReset, true, true);
        item.new_overflow_count = Some(0);
        assert_eq!(
            item.validate().unwrap(),
            StepOverflowAttribution::DidNotOccurThisStep
        );
        item.new_overflow_count = Some(2);
        assert_eq!(
            item.validate().unwrap(),
            StepOverflowAttribution::OccurredThisStep
        );
    }

    #[test]
    fn unknown_flag_lifetime_is_never_interpreted_as_success() {
        assert_eq!(
            overflow(OverflowFlagLifetime::Unknown, false, false)
                .validate()
                .unwrap(),
            StepOverflowAttribution::Unattributable
        );
    }

    #[test]
    fn reset_receipt_requires_monotonic_epoch_transition() {
        let mut reset = DiagnosticResetReceipt {
            backend_profile_sha256: "b".repeat(64),
            scope_kind: CapacityDiagnosticScope::World,
            scope_id: "world-0".into(),
            reset_id: "reset-3".into(),
            step_index: 13,
            epoch_before: 3,
            epoch_after: 4,
            source_semantics_id: "steel-reset-v1".into(),
        };
        assert!(reset.validate().is_ok());
        reset.epoch_after = 3;
        assert!(reset.validate().is_err());
    }
}
