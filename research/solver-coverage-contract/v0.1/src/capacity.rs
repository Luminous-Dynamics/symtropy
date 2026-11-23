// SPDX-License-Identifier: AGPL-3.0-or-later
//! Contact-manifold lineage and temporally scoped capacity diagnostics.
//!
//! Geometric contact candidates, reduced manifold points, solver contacts and
//! constraint rows are distinct populations. This module records their lineage
//! without treating point counts as solver-row counts or physical proof.

use std::collections::{BTreeMap, BTreeSet};

use super::{ConstraintRowFamily, ValidationError, checked_sum, is_sha256, non_empty};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum GeometricPointDisposition {
    Retained = 1,
    ReducedByPolicy = 2,
    CandidateBufferCapacityDropped = 3,
    ManifoldCapacityDropped = 4,
    Invalid = 5,
    Unsupported = 6,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PointIdentityMode {
    StablePointIds = 1,
    AggregateOnly = 2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregateManifoldCounts {
    pub generated_points: u64,
    pub retained_points: u64,
    pub reduced_by_policy: u64,
    pub candidate_buffer_capacity_dropped: u64,
    pub manifold_capacity_dropped: u64,
    pub invalid_points: u64,
    pub unsupported_points: u64,
    pub solver_contact_groups: u64,
    pub solver_solved_points: u64,
    pub solver_not_converged_points: u64,
    pub solver_sleeping_skipped_points: u64,
    pub solver_capacity_dropped_points: u64,
    pub solver_unsupported_points: u64,
    pub solver_not_submitted_points: u64,
    pub solver_aborted_points: u64,
}

impl AggregateManifoldCounts {
    fn validate(&self) -> Result<(), ValidationError> {
        let generated_partition = checked_sum(
            &[
                self.retained_points,
                self.reduced_by_policy,
                self.candidate_buffer_capacity_dropped,
                self.manifold_capacity_dropped,
                self.invalid_points,
                self.unsupported_points,
            ],
            "manifold.aggregate_counts",
        )?;
        if generated_partition != self.generated_points {
            return Err(ValidationError::new(
                "manifold.aggregate_counts",
                "generated_points must equal retained + reduced + capacity_dropped + invalid + unsupported",
            ));
        }

        let solver_partition = checked_sum(
            &[
                self.solver_solved_points,
                self.solver_not_converged_points,
                self.solver_sleeping_skipped_points,
                self.solver_capacity_dropped_points,
                self.solver_unsupported_points,
                self.solver_not_submitted_points,
                self.solver_aborted_points,
            ],
            "manifold.aggregate_counts",
        )?;
        if solver_partition != self.retained_points {
            return Err(ValidationError::new(
                "manifold.aggregate_counts",
                "retained_points must equal solver-contact disposition point counts",
            ));
        }
        if (self.solver_contact_groups == 0) != (solver_partition == 0)
            || self.solver_contact_groups > solver_partition
        {
            return Err(ValidationError::new(
                "manifold.aggregate_counts.solver_contact_groups",
                "group count must be zero iff no points are represented, and cannot exceed represented points",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactManifoldLineage {
    pub pair_id: String,
    pub identity_mode: PointIdentityMode,
    /// Present when any generated point is intentionally reduced by policy.
    pub reduction_policy_id: Option<String>,
    /// Capacity of the temporary candidate buffer before deliberate reduction.
    pub candidate_buffer_capacity: Option<u64>,
    /// Capacity of the final stored manifold points.
    pub manifold_point_capacity: Option<u64>,
    /// Exactly one disposition for each generated geometric candidate in StablePointIds mode.
    pub generated_points: Vec<GeometricPointRecord>,
    /// Solver contacts may represent one point or a cluster of geometric points.
    pub solver_contacts: Vec<SolverContactGroup>,
    /// Aggregate stage counts are mandatory when stable point identifiers are unavailable.
    pub aggregate_counts: Option<AggregateManifoldCounts>,
    /// True only when every retained point is mapped to exactly one solver contact group.
    pub solver_mapping_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactManifoldReport {
    pub pair_id: String,
    pub generated_points: u64,
    pub retained_points: u64,
    pub reduced_by_policy: u64,
    pub candidate_buffer_capacity_dropped: u64,
    pub manifold_capacity_dropped: u64,
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
    pub candidate_buffer_capacity_respected: bool,
    pub manifold_point_capacity_respected: bool,
}

impl ContactManifoldLineage {
    pub fn validate(&self) -> Result<ContactManifoldReport, ValidationError> {
        if !non_empty(&self.pair_id) {
            return Err(ValidationError::new(
                "manifold.pair_id",
                "must not be empty",
            ));
        }
        if let Some(policy) = &self.reduction_policy_id
            && !non_empty(policy)
        {
            return Err(ValidationError::new(
                "manifold.reduction_policy_id",
                "must not be empty when present",
            ));
        }

        if self.identity_mode == PointIdentityMode::AggregateOnly {
            if !self.generated_points.is_empty()
                || !self.solver_contacts.is_empty()
                || self.solver_mapping_complete
            {
                return Err(ValidationError::new(
                    "manifold.identity_mode",
                    "AggregateOnly mode cannot contain point IDs or claim complete point mapping",
                ));
            }
            let counts = self.aggregate_counts.as_ref().ok_or_else(|| {
                ValidationError::new(
                    "manifold.aggregate_counts",
                    "AggregateOnly mode requires aggregate stage counts",
                )
            })?;
            counts.validate()?;
            if counts.reduced_by_policy > 0 && self.reduction_policy_id.is_none() {
                return Err(ValidationError::new(
                    "manifold.reduction_policy_id",
                    "aggregate reductions require a reduction policy identity",
                ));
            }
            let candidate_buffer_accepted = checked_sum(
                &[
                    counts.retained_points,
                    counts.reduced_by_policy,
                    counts.manifold_capacity_dropped,
                ],
                "manifold.aggregate_counts",
            )?;
            let candidate_buffer_capacity_respected = self
                .candidate_buffer_capacity
                .is_none_or(|capacity| candidate_buffer_accepted <= capacity);
            let manifold_point_capacity_respected = self
                .manifold_point_capacity
                .is_none_or(|capacity| counts.retained_points <= capacity);
            if !candidate_buffer_capacity_respected || !manifold_point_capacity_respected {
                return Err(ValidationError::new(
                    "manifold.capacity",
                    "aggregate candidate or final-manifold population exceeds its declared capacity",
                ));
            }
            return Ok(ContactManifoldReport {
                pair_id: self.pair_id.clone(),
                generated_points: counts.generated_points,
                retained_points: counts.retained_points,
                reduced_by_policy: counts.reduced_by_policy,
                candidate_buffer_capacity_dropped: counts.candidate_buffer_capacity_dropped,
                manifold_capacity_dropped: counts.manifold_capacity_dropped,
                invalid_points: counts.invalid_points,
                unsupported_points: counts.unsupported_points,
                solver_groups: counts.solver_contact_groups,
                solver_mapped_points: 0,
                solver_solved_points: counts.solver_solved_points,
                solver_not_converged_points: counts.solver_not_converged_points,
                solver_sleeping_skipped_points: counts.solver_sleeping_skipped_points,
                solver_capacity_dropped_points: counts.solver_capacity_dropped_points,
                solver_unsupported_points: counts.solver_unsupported_points,
                solver_not_submitted_points: counts.solver_not_submitted_points,
                solver_aborted_points: counts.solver_aborted_points,
                solver_mapping_complete: false,
                candidate_buffer_capacity_respected,
                manifold_point_capacity_respected,
            });
        }
        if self.aggregate_counts.is_some() {
            return Err(ValidationError::new(
                "manifold.aggregate_counts",
                "StablePointIds mode cannot also provide aggregate-only counts",
            ));
        }

        let mut point_records = BTreeMap::new();
        let mut generated_points = 0_u64;
        let mut retained_points = 0_u64;
        let mut reduced_by_policy = 0_u64;
        let mut candidate_buffer_capacity_dropped = 0_u64;
        let mut manifold_capacity_dropped = 0_u64;
        let mut invalid_points = 0_u64;
        let mut unsupported_points = 0_u64;

        for (index, point) in self.generated_points.iter().enumerate() {
            let path = format!("manifold.generated_points[{index}]");
            point.validate(&path)?;
            if point_records
                .insert(point.point_id.as_str(), point)
                .is_some()
            {
                return Err(ValidationError::new(&path, "duplicate geometric point ID"));
            }
            generated_points = generated_points.checked_add(1).ok_or_else(|| {
                ValidationError::new("manifold.generated_points", "point count overflow")
            })?;
            match point.disposition {
                GeometricPointDisposition::Retained => retained_points += 1,
                GeometricPointDisposition::ReducedByPolicy => reduced_by_policy += 1,
                GeometricPointDisposition::CandidateBufferCapacityDropped => {
                    candidate_buffer_capacity_dropped += 1
                }
                GeometricPointDisposition::ManifoldCapacityDropped => {
                    manifold_capacity_dropped += 1
                }
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

        let candidate_buffer_accepted = checked_sum(
            &[
                retained_points,
                reduced_by_policy,
                manifold_capacity_dropped,
            ],
            "manifold.candidate_buffer",
        )?;
        let candidate_buffer_capacity_respected = self
            .candidate_buffer_capacity
            .is_none_or(|capacity| candidate_buffer_accepted <= capacity);
        let manifold_point_capacity_respected = self
            .manifold_point_capacity
            .is_none_or(|capacity| retained_points <= capacity);
        if !candidate_buffer_capacity_respected || !manifold_point_capacity_respected {
            return Err(ValidationError::new(
                "manifold.capacity",
                "candidate-buffer or final-manifold population exceeds its declared capacity",
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
                    SolverContactDisposition::SleepingSkipped => {
                        solver_sleeping_skipped_points += 1
                    }
                    SolverContactDisposition::CapacityDropped => {
                        solver_capacity_dropped_points += 1
                    }
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
            candidate_buffer_capacity_dropped,
            manifold_capacity_dropped,
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
            candidate_buffer_capacity_respected,
            manifold_point_capacity_respected,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
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
    /// Backend-provided overflow-count delta across this exact observed step.
    pub overflow_count_delta: Option<u64>,
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

        if let Some(count) = self.overflow_count_delta {
            return Ok(if count > 0 {
                StepOverflowAttribution::OccurredThisStep
            } else {
                StepOverflowAttribution::DidNotOccurThisStep
            });
        }

        if matches!(
            self.flag_lifetime,
            OverflowFlagLifetime::StickyUntilReset | OverflowFlagLifetime::Cumulative
        ) && (self.reset_epoch.is_none() || self.last_confirmed_reset_step.is_none())
        {
            return Ok(StepOverflowAttribution::Unattributable);
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapacityEvidence {
    pub manifold_lineages: Vec<ContactManifoldLineage>,
    pub overflow_observations: Vec<OverflowObservation>,
    pub diagnostic_resets: Vec<DiagnosticResetReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityEvidenceReport {
    pub manifold_pairs: u64,
    pub geometric_points_generated: u64,
    pub geometric_points_retained: u64,
    pub geometric_points_reduced: u64,
    pub geometric_points_candidate_buffer_dropped: u64,
    pub geometric_points_manifold_capacity_dropped: u64,
    pub geometric_points_capacity_dropped: u64,
    pub solver_contact_groups: u64,
    pub solver_mapping_incomplete_pairs: u64,
    pub overflow_observations: u64,
    pub overflow_occurred_this_step: u64,
    pub overflow_not_attributable_to_step: u64,
    pub diagnostic_resets: u64,
}

impl CapacityEvidence {
    pub fn validate(&self) -> Result<CapacityEvidenceReport, ValidationError> {
        let mut pair_ids = BTreeSet::new();
        let mut manifold_pairs = 0_u64;
        let mut geometric_points_generated = 0_u64;
        let mut geometric_points_retained = 0_u64;
        let mut geometric_points_reduced = 0_u64;
        let mut geometric_points_candidate_buffer_dropped = 0_u64;
        let mut geometric_points_manifold_capacity_dropped = 0_u64;
        let mut geometric_points_capacity_dropped = 0_u64;
        let mut solver_contact_groups = 0_u64;
        let mut solver_mapping_incomplete_pairs = 0_u64;

        for lineage in &self.manifold_lineages {
            if !pair_ids.insert(lineage.pair_id.as_str()) {
                return Err(ValidationError::new(
                    "capacity.manifold_lineages",
                    "duplicate contact-pair lineage identity",
                ));
            }
            let report = lineage.validate()?;
            manifold_pairs += 1;
            geometric_points_generated += report.generated_points;
            geometric_points_retained += report.retained_points;
            geometric_points_reduced += report.reduced_by_policy;
            geometric_points_candidate_buffer_dropped += report.candidate_buffer_capacity_dropped;
            geometric_points_manifold_capacity_dropped += report.manifold_capacity_dropped;
            geometric_points_capacity_dropped = checked_sum(
                &[
                    geometric_points_capacity_dropped,
                    report.candidate_buffer_capacity_dropped,
                    report.manifold_capacity_dropped,
                ],
                "capacity.geometric_points_capacity_dropped",
            )?;
            solver_contact_groups += report.solver_groups;
            if !report.solver_mapping_complete {
                solver_mapping_incomplete_pairs += 1;
            }
        }

        let mut overflow_keys = BTreeSet::new();
        let mut overflow_occurred_this_step = 0_u64;
        let mut overflow_not_attributable_to_step = 0_u64;
        for observation in &self.overflow_observations {
            let key = (
                observation.scope_kind,
                observation.scope_id.as_str(),
                observation.row_family,
                observation.step_index,
            );
            if !overflow_keys.insert(key) {
                return Err(ValidationError::new(
                    "capacity.overflow_observations",
                    "duplicate scope/row-family/step observation",
                ));
            }
            match observation.validate()? {
                StepOverflowAttribution::OccurredThisStep => overflow_occurred_this_step += 1,
                StepOverflowAttribution::Unattributable => overflow_not_attributable_to_step += 1,
                StepOverflowAttribution::DidNotOccurThisStep => {}
            }
        }

        let mut reset_ids = BTreeSet::new();
        for reset in &self.diagnostic_resets {
            reset.validate()?;
            if !reset_ids.insert((
                reset.scope_kind,
                reset.scope_id.as_str(),
                reset.reset_id.as_str(),
            )) {
                return Err(ValidationError::new(
                    "capacity.diagnostic_resets",
                    "duplicate reset identity within diagnostic scope",
                ));
            }
        }

        Ok(CapacityEvidenceReport {
            manifold_pairs,
            geometric_points_generated,
            geometric_points_retained,
            geometric_points_reduced,
            geometric_points_candidate_buffer_dropped,
            geometric_points_manifold_capacity_dropped,
            geometric_points_capacity_dropped,
            solver_contact_groups,
            solver_mapping_incomplete_pairs,
            overflow_observations: self.overflow_observations.len() as u64,
            overflow_occurred_this_step,
            overflow_not_attributable_to_step,
            diagnostic_resets: self.diagnostic_resets.len() as u64,
        })
    }

    /// Deterministic preimage for contact-lineage and temporal capacity evidence.
    pub fn canonical_preimage(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"SYMCOV-CAPACITY\0");

        let mut lineages = self.manifold_lineages.clone();
        lineages.sort_by(|a, b| a.pair_id.cmp(&b.pair_id));
        push_u64(&mut out, lineages.len() as u64);
        for lineage in lineages {
            push_string(&mut out, &lineage.pair_id);
            push_optional_string(&mut out, lineage.reduction_policy_id.as_deref());
            push_optional_u64(&mut out, lineage.candidate_buffer_capacity);
            push_optional_u64(&mut out, lineage.manifold_point_capacity);
            out.push(lineage.identity_mode as u8);
            match lineage.aggregate_counts {
                Some(counts) => {
                    out.push(1);
                    for value in [
                        counts.generated_points,
                        counts.retained_points,
                        counts.reduced_by_policy,
                        counts.candidate_buffer_capacity_dropped,
                        counts.manifold_capacity_dropped,
                        counts.invalid_points,
                        counts.unsupported_points,
                        counts.solver_contact_groups,
                        counts.solver_solved_points,
                        counts.solver_not_converged_points,
                        counts.solver_sleeping_skipped_points,
                        counts.solver_capacity_dropped_points,
                        counts.solver_unsupported_points,
                        counts.solver_not_submitted_points,
                        counts.solver_aborted_points,
                    ] {
                        push_u64(&mut out, value);
                    }
                }
                None => out.push(0),
            }
            out.push(u8::from(lineage.solver_mapping_complete));

            let mut points = lineage.generated_points.clone();
            points.sort_by(|a, b| a.point_id.cmp(&b.point_id));
            push_u64(&mut out, points.len() as u64);
            for point in points {
                push_string(&mut out, &point.point_id);
                out.push(point.disposition as u8);
                push_optional_string(&mut out, point.reason.as_deref());
            }

            let mut groups = lineage.solver_contacts.clone();
            groups.sort_by(|a, b| a.solver_contact_id.cmp(&b.solver_contact_id));
            push_u64(&mut out, groups.len() as u64);
            for mut group in groups {
                push_string(&mut out, &group.solver_contact_id);
                group.source_point_ids.sort();
                push_u64(&mut out, group.source_point_ids.len() as u64);
                for source_id in group.source_point_ids {
                    push_string(&mut out, &source_id);
                }
                out.push(group.disposition as u8);
                push_optional_string(&mut out, group.reason.as_deref());
            }
        }

        let mut observations = self.overflow_observations.clone();
        observations.sort_by(|a, b| {
            (a.scope_kind, &a.scope_id, a.row_family, a.step_index).cmp(&(
                b.scope_kind,
                &b.scope_id,
                b.row_family,
                b.step_index,
            ))
        });
        push_u64(&mut out, observations.len() as u64);
        for item in observations {
            push_string(&mut out, &item.backend_profile_sha256.to_ascii_lowercase());
            out.push(item.scope_kind as u8);
            push_string(&mut out, &item.scope_id);
            out.push(item.row_family.map_or(0, |family| family as u8));
            push_u64(&mut out, item.step_index);
            push_optional_u64(&mut out, item.reset_epoch);
            push_optional_u64(&mut out, item.last_confirmed_reset_step);
            out.push(item.flag_lifetime as u8);
            out.push(u8::from(item.observed_before_step));
            out.push(u8::from(item.observed_after_step));
            push_optional_u64(&mut out, item.overflow_count_delta);
            push_string(&mut out, &item.source_semantics_id);
        }

        let mut resets = self.diagnostic_resets.clone();
        resets.sort_by(|a, b| {
            (a.scope_kind, &a.scope_id, &a.reset_id).cmp(&(b.scope_kind, &b.scope_id, &b.reset_id))
        });
        push_u64(&mut out, resets.len() as u64);
        for item in resets {
            push_string(&mut out, &item.backend_profile_sha256.to_ascii_lowercase());
            out.push(item.scope_kind as u8);
            push_string(&mut out, &item.scope_id);
            push_string(&mut out, &item.reset_id);
            push_u64(&mut out, item.step_index);
            push_u64(&mut out, item.epoch_before);
            push_u64(&mut out, item.epoch_after);
            push_string(&mut out, &item.source_semantics_id);
        }
        out
    }
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_optional_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            push_u64(out, value);
        }
        None => out.push(0),
    }
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    push_u64(out, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

fn push_optional_string(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            out.push(1);
            push_string(out, value);
        }
        None => out.push(0),
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
            overflow_count_delta: None,
            source_semantics_id: "steel-v1-constraint-overflow".into(),
        }
    }

    #[test]
    fn geometric_reduction_is_separate_from_solver_contact_groups() {
        let receipt = ContactManifoldLineage {
            pair_id: "body-a/body-b".into(),
            identity_mode: PointIdentityMode::StablePointIds,
            reduction_policy_id: Some("area-preserving-v1".into()),
            candidate_buffer_capacity: Some(8),
            manifold_point_capacity: Some(8),
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
            aggregate_counts: None,
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
            identity_mode: PointIdentityMode::StablePointIds,
            reduction_policy_id: Some("policy-0".into()),
            candidate_buffer_capacity: None,
            manifold_point_capacity: Some(4),
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
            aggregate_counts: None,
            solver_mapping_complete: false,
        };
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn reduced_points_require_policy_identity() {
        let receipt = ContactManifoldLineage {
            pair_id: "pair-0".into(),
            identity_mode: PointIdentityMode::StablePointIds,
            reduction_policy_id: None,
            candidate_buffer_capacity: None,
            manifold_point_capacity: Some(4),
            generated_points: vec![GeometricPointRecord {
                point_id: "p0".into(),
                disposition: GeometricPointDisposition::ReducedByPolicy,
                reason: Some("reduced".into()),
            }],
            solver_contacts: vec![],
            aggregate_counts: None,
            solver_mapping_complete: false,
        };
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn retained_points_cannot_exceed_declared_capacity() {
        let receipt = ContactManifoldLineage {
            pair_id: "pair-0".into(),
            identity_mode: PointIdentityMode::StablePointIds,
            reduction_policy_id: None,
            candidate_buffer_capacity: None,
            manifold_point_capacity: Some(1),
            generated_points: vec![retained("p0"), retained("p1")],
            solver_contacts: vec![],
            aggregate_counts: None,
            solver_mapping_complete: false,
        };
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn aggregate_only_lineage_preserves_counts_without_fabricating_point_ids() {
        let lineage = ContactManifoldLineage {
            pair_id: "aggregate-pair".into(),
            identity_mode: PointIdentityMode::AggregateOnly,
            reduction_policy_id: Some("area-policy-v2".into()),
            candidate_buffer_capacity: None,
            manifold_point_capacity: Some(4),
            generated_points: vec![],
            solver_contacts: vec![],
            aggregate_counts: Some(AggregateManifoldCounts {
                generated_points: 5,
                retained_points: 3,
                reduced_by_policy: 1,
                candidate_buffer_capacity_dropped: 1,
                    manifold_capacity_dropped: 0,
                invalid_points: 0,
                unsupported_points: 0,
                solver_contact_groups: 2,
                solver_solved_points: 1,
                solver_not_converged_points: 0,
                solver_sleeping_skipped_points: 0,
                solver_capacity_dropped_points: 2,
                solver_unsupported_points: 0,
                solver_not_submitted_points: 0,
                solver_aborted_points: 0,
            }),
            solver_mapping_complete: false,
        };
        let report = lineage.validate().expect("aggregate-only lineage");
        assert_eq!(report.generated_points, 5);
        assert_eq!(report.retained_points, 3);
        assert_eq!(report.reduced_by_policy, 1);
        assert_eq!(report.candidate_buffer_capacity_dropped, 1);
        assert_eq!(report.solver_groups, 2);
        assert_eq!(report.solver_solved_points, 1);
        assert_eq!(report.solver_capacity_dropped_points, 2);
        assert!(!report.solver_mapping_complete);
    }

    #[test]
    fn aggregate_only_stage_counts_must_partition_exactly() {
        let lineage = ContactManifoldLineage {
            pair_id: "aggregate-invalid".into(),
            identity_mode: PointIdentityMode::AggregateOnly,
            reduction_policy_id: None,
            candidate_buffer_capacity: Some(8),
            manifold_point_capacity: Some(8),
            generated_points: vec![],
            solver_contacts: vec![],
            aggregate_counts: Some(AggregateManifoldCounts {
                generated_points: 4,
                retained_points: 2,
                reduced_by_policy: 0,
                candidate_buffer_capacity_dropped: 0,
                manifold_capacity_dropped: 0,
                invalid_points: 0,
                unsupported_points: 0,
                solver_contact_groups: 1,
                solver_solved_points: 1,
                solver_not_converged_points: 0,
                solver_sleeping_skipped_points: 0,
                solver_capacity_dropped_points: 0,
                solver_unsupported_points: 0,
                solver_not_submitted_points: 0,
                solver_aborted_points: 0,
            }),
            solver_mapping_complete: false,
        };
        assert!(lineage.validate().is_err());
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
        item.overflow_count_delta = Some(0);
        assert_eq!(
            item.validate().unwrap(),
            StepOverflowAttribution::DidNotOccurThisStep
        );
        item.overflow_count_delta = Some(2);
        assert_eq!(
            item.validate().unwrap(),
            StepOverflowAttribution::OccurredThisStep
        );
    }

    #[test]
    fn sticky_flag_without_reset_epoch_is_unattributable() {
        let mut item = overflow(OverflowFlagLifetime::StickyUntilReset, false, true);
        item.reset_epoch = None;
        assert_eq!(
            item.validate().unwrap(),
            StepOverflowAttribution::Unattributable
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
