// SPDX-License-Identifier: AGPL-3.0-or-later
//! Passive accounting contract for contact-manifold generation stages.

use super::OracleError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FinalStorageMode {
    FixedCapacity,
    InlineWithSpill,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceDisposition {
    Complete,
    DiagnosticBufferTruncated,
    InstrumentationUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateGenerationCounts {
    pub perturbations_requested: u64,
    pub perturbations_valid: u64,
    pub perturbations_invalid_or_degenerate: u64,

    pub support_queries_completed: u64,
    pub support_queries_failed: u64,
    pub shallow_separation_rejected: u64,
    pub duplicate_geometry_rejected: u64,
    pub candidate_admission_capacity_rejected: u64,
    pub support_candidates_admitted: u64,

    pub candidate_buffer_storage_capacity: u64,
    pub candidate_admission_limit: u64,
    pub primary_candidate_admitted: bool,
    pub candidate_pool_count_before_reduction: u64,

    pub reduction_policy_id: String,
    pub best_n_limit: u64,
    pub best_n_output_count: u64,
    pub best_n_reduced_count: u64,

    pub final_storage_mode: FinalStorageMode,
    pub final_storage_inline_capacity: u64,
    pub final_storage_fixed_capacity: Option<u64>,
    pub final_storage_attempted_count: u64,
    pub final_storage_stored_count: u64,
    pub final_storage_dropped_count: u64,
    pub final_storage_spill_used: bool,

    pub trace_disposition: TraceDisposition,
    pub diagnostic_events_expected: u64,
    pub diagnostic_events_emitted: u64,
    pub diagnostic_events_dropped: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateGenerationReport {
    pub support_queries_attempted: u64,
    pub support_queries_completed: u64,
    pub support_queries_failed: u64,
    pub support_candidates_admitted: u64,
    pub candidate_pool_count_before_reduction: u64,
    pub best_n_reduced_count: u64,
    pub final_storage_stored_count: u64,
    pub final_storage_dropped_count: u64,
    pub final_storage_spill_used: bool,
    pub trace_complete: bool,
    pub evidence_complete: bool,
}

fn checked_sum(values: &[u64], field: &str) -> Result<u64, OracleError> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| OracleError::new(format!("{field} count overflow")))
    })
}

impl CandidateGenerationCounts {
    pub fn validate(&self) -> Result<CandidateGenerationReport, OracleError> {
        if self.reduction_policy_id.trim().is_empty() {
            return Err(OracleError::new(
                "reduction policy identity must not be empty",
            ));
        }
        if self.perturbations_requested
            != checked_sum(
                &[
                    self.perturbations_valid,
                    self.perturbations_invalid_or_degenerate,
                ],
                "perturbations",
            )?
        {
            return Err(OracleError::new(
                "requested perturbations must equal valid plus invalid/degenerate",
            ));
        }

        let support_queries_attempted = self.perturbations_valid;
        if support_queries_attempted
            != checked_sum(
                &[self.support_queries_completed, self.support_queries_failed],
                "support queries",
            )?
        {
            return Err(OracleError::new(
                "valid perturbations must equal completed plus failed support queries",
            ));
        }
        if self.support_queries_completed
            != checked_sum(
                &[
                    self.shallow_separation_rejected,
                    self.duplicate_geometry_rejected,
                    self.candidate_admission_capacity_rejected,
                    self.support_candidates_admitted,
                ],
                "support query outcomes",
            )?
        {
            return Err(OracleError::new(
                "completed support queries must equal shallow + duplicate + capacity-rejected + admitted",
            ));
        }

        if self.candidate_admission_limit > self.candidate_buffer_storage_capacity {
            return Err(OracleError::new(
                "candidate admission limit exceeds temporary buffer storage capacity",
            ));
        }
        let primary_count = self.primary_candidate_admitted as u64;
        let pool_count = checked_sum(
            &[primary_count, self.support_candidates_admitted],
            "candidate pool",
        )?;
        if pool_count != self.candidate_pool_count_before_reduction {
            return Err(OracleError::new(
                "candidate pool must equal admitted primary plus admitted support candidates",
            ));
        }
        if pool_count > self.candidate_buffer_storage_capacity {
            return Err(OracleError::new(
                "candidate pool exceeds temporary buffer storage capacity",
            ));
        }
        if pool_count > self.candidate_admission_limit {
            return Err(OracleError::new(
                "candidate pool exceeds the explicit admission limit",
            ));
        }
        if self.candidate_admission_capacity_rejected > 0
            && pool_count != self.candidate_admission_limit
        {
            return Err(OracleError::new(
                "candidate admission drops require the full pool to reach the admission limit",
            ));
        }

        let expected_output = pool_count.min(self.best_n_limit);
        if self.best_n_output_count != expected_output {
            return Err(OracleError::new(
                "best-N output must equal min(candidate pool count, best-N limit)",
            ));
        }
        if self.best_n_reduced_count != pool_count - self.best_n_output_count {
            return Err(OracleError::new(
                "best-N reduction count must equal candidate pool minus output",
            ));
        }
        if self.final_storage_attempted_count != self.best_n_output_count {
            return Err(OracleError::new(
                "final storage attempted count must equal best-N output",
            ));
        }
        if self.final_storage_attempted_count
            != checked_sum(
                &[
                    self.final_storage_stored_count,
                    self.final_storage_dropped_count,
                ],
                "final storage",
            )?
        {
            return Err(OracleError::new(
                "final storage attempted must equal stored plus dropped",
            ));
        }

        match self.final_storage_mode {
            FinalStorageMode::FixedCapacity => {
                let capacity = self.final_storage_fixed_capacity.ok_or_else(|| {
                    OracleError::new("fixed-capacity storage requires a capacity")
                })?;
                if self.final_storage_stored_count > capacity || self.final_storage_spill_used {
                    return Err(OracleError::new(
                        "fixed-capacity storage overrun or invalid spill report",
                    ));
                }
            }
            FinalStorageMode::InlineWithSpill => {
                if self.final_storage_fixed_capacity.is_some()
                    || self.final_storage_dropped_count != 0
                    || self.final_storage_spill_used
                        != (self.final_storage_stored_count > self.final_storage_inline_capacity)
                {
                    return Err(OracleError::new(
                        "inline-with-spill storage must report spill accurately and cannot drop points",
                    ));
                }
            }
        }

        if self.diagnostic_events_expected
            != checked_sum(
                &[
                    self.diagnostic_events_emitted,
                    self.diagnostic_events_dropped,
                ],
                "diagnostic events",
            )?
        {
            return Err(OracleError::new(
                "expected diagnostic events must equal emitted plus dropped",
            ));
        }
        if self.trace_disposition == TraceDisposition::Complete
            && self.diagnostic_events_dropped != 0
        {
            return Err(OracleError::new(
                "complete trace cannot report dropped diagnostic events",
            ));
        }
        if self.trace_disposition == TraceDisposition::DiagnosticBufferTruncated
            && self.diagnostic_events_dropped == 0
        {
            return Err(OracleError::new(
                "truncated trace must report dropped diagnostic events",
            ));
        }

        let trace_complete = self.trace_disposition == TraceDisposition::Complete
            && self.diagnostic_events_dropped == 0;
        let evidence_complete = trace_complete
            && self.support_queries_failed == 0
            && self.candidate_admission_capacity_rejected == 0
            && self.final_storage_dropped_count == 0;

        Ok(CandidateGenerationReport {
            support_queries_attempted,
            support_queries_completed: self.support_queries_completed,
            support_queries_failed: self.support_queries_failed,
            support_candidates_admitted: self.support_candidates_admitted,
            candidate_pool_count_before_reduction: pool_count,
            best_n_reduced_count: self.best_n_reduced_count,
            final_storage_stored_count: self.final_storage_stored_count,
            final_storage_dropped_count: self.final_storage_dropped_count,
            final_storage_spill_used: self.final_storage_spill_used,
            trace_complete,
            evidence_complete,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> CandidateGenerationCounts {
        CandidateGenerationCounts {
            perturbations_requested: 8,
            perturbations_valid: 8,
            perturbations_invalid_or_degenerate: 0,
            support_queries_completed: 8,
            support_queries_failed: 0,
            shallow_separation_rejected: 1,
            duplicate_geometry_rejected: 2,
            candidate_admission_capacity_rejected: 0,
            support_candidates_admitted: 5,
            candidate_buffer_storage_capacity: 16,
            candidate_admission_limit: 8,
            primary_candidate_admitted: true,
            candidate_pool_count_before_reduction: 6,
            reduction_policy_id: "best-n-v1".into(),
            best_n_limit: 4,
            best_n_output_count: 4,
            best_n_reduced_count: 2,
            final_storage_mode: FinalStorageMode::InlineWithSpill,
            final_storage_inline_capacity: 8,
            final_storage_fixed_capacity: None,
            final_storage_attempted_count: 4,
            final_storage_stored_count: 4,
            final_storage_dropped_count: 0,
            final_storage_spill_used: false,
            trace_disposition: TraceDisposition::Complete,
            diagnostic_events_expected: 18,
            diagnostic_events_emitted: 18,
            diagnostic_events_dropped: 0,
        }
    }

    #[test]
    fn exact_stage_partitions_validate_for_complete_generation() {
        let report = baseline().validate().expect("complete generation");
        assert_eq!(report.support_queries_attempted, 8);
        assert_eq!(report.support_candidates_admitted, 5);
        assert_eq!(report.candidate_pool_count_before_reduction, 6);
        assert_eq!(report.best_n_reduced_count, 2);
        assert_eq!(report.final_storage_stored_count, 4);
        assert!(!report.final_storage_spill_used);
        assert!(report.trace_complete);
        assert!(report.evidence_complete);
    }

    #[test]
    fn candidate_admission_drop_is_not_best_n_reduction() {
        let mut counts = baseline();
        counts.perturbations_requested = 9;
        counts.perturbations_valid = 9;
        counts.support_queries_completed = 9;
        counts.candidate_admission_limit = 5;
        counts.candidate_admission_capacity_rejected = 2;
        counts.support_candidates_admitted = 4;
        counts.candidate_pool_count_before_reduction = 5;
        counts.best_n_reduced_count = 1;
        let report = counts.validate().expect("admission loss explicit");
        assert_eq!(report.support_queries_attempted, 9);
        assert_eq!(report.best_n_reduced_count, 1);
        assert!(!report.evidence_complete);
    }

    #[test]
    fn sixteen_slot_buffer_and_eight_point_admission_limit_are_distinct() {
        let mut counts = baseline();
        counts.perturbations_requested = 11;
        counts.perturbations_valid = 11;
        counts.support_queries_completed = 11;
        counts.candidate_admission_limit = 8;
        counts.candidate_admission_capacity_rejected = 3;
        counts.support_candidates_admitted = 7;
        counts.perturbations_requested = 13;
        counts.perturbations_valid = 13;
        counts.support_queries_completed = 13;
        counts.candidate_pool_count_before_reduction = 8;
        counts.best_n_reduced_count = 4;
        let report = counts.validate().expect("8 total admission, 16 backing");
        assert_eq!(report.support_candidates_admitted, 7);
        assert!(!report.evidence_complete);
    }

    #[test]
    fn best_n_policy_reduction_is_not_a_capacity_drop() {
        let mut counts = baseline();
        counts.support_candidates_admitted = 7;
        counts.candidate_pool_count_before_reduction = 8;
        counts.perturbations_requested = 10;
        counts.perturbations_valid = 10;
        counts.support_queries_completed = 10;
        counts.best_n_output_count = 4;
        counts.best_n_reduced_count = 4;
        let report = counts.validate().expect("best-N reduction");
        assert_eq!(report.best_n_reduced_count, 4);
        assert!(report.evidence_complete);
    }

    #[test]
    fn inline_storage_spill_is_not_a_dropped_point() {
        let mut counts = baseline();
        counts.support_candidates_admitted = 11;
        counts.candidate_admission_limit = 16;
        counts.candidate_pool_count_before_reduction = 12;
        counts.perturbations_requested = 14;
        counts.perturbations_valid = 14;
        counts.support_queries_completed = 14;
        counts.best_n_limit = 12;
        counts.best_n_output_count = 12;
        counts.best_n_reduced_count = 0;
        counts.final_storage_attempted_count = 12;
        counts.final_storage_stored_count = 12;
        counts.final_storage_inline_capacity = 8;
        counts.final_storage_spill_used = true;
        let report = counts.validate().expect("spill without loss");
        assert!(report.final_storage_spill_used);
        assert_eq!(report.final_storage_dropped_count, 0);
    }

    #[test]
    fn fixed_capacity_final_storage_drop_is_explicit() {
        let mut counts = baseline();
        counts.final_storage_mode = FinalStorageMode::FixedCapacity;
        counts.final_storage_fixed_capacity = Some(3);
        counts.final_storage_stored_count = 3;
        counts.final_storage_dropped_count = 1;
        let report = counts.validate().expect("fixed capacity drop");
        assert_eq!(report.final_storage_dropped_count, 1);
        assert!(!report.evidence_complete);
    }

    #[test]
    fn trace_buffer_truncation_is_separate_from_physical_candidate_drops() {
        let mut counts = baseline();
        counts.trace_disposition = TraceDisposition::DiagnosticBufferTruncated;
        counts.diagnostic_events_expected = 20;
        counts.diagnostic_events_emitted = 18;
        counts.diagnostic_events_dropped = 2;
        let report = counts.validate().expect("diagnostic truncation explicit");
        assert!(!report.trace_complete);
        assert!(!report.evidence_complete);
        assert_eq!(report.final_storage_dropped_count, 0);
    }

    #[test]
    fn support_query_failure_is_not_silently_counted_as_shallow_or_duplicate() {
        let mut counts = baseline();
        counts.support_queries_completed = 7;
        counts.support_queries_failed = 1;
        counts.support_candidates_admitted = 4;
        counts.candidate_pool_count_before_reduction = 5;
        counts.best_n_reduced_count = 1;
        let report = counts.validate().expect("query failure explicit");
        assert_eq!(report.support_queries_failed, 1);
        assert!(!report.evidence_complete);
    }

    #[test]
    fn arithmetic_overflow_fails_closed() {
        let mut counts = baseline();
        counts.perturbations_valid = u64::MAX;
        counts.perturbations_invalid_or_degenerate = 1;
        assert!(counts.validate().is_err());
    }

    #[test]
    fn admission_limit_cannot_exceed_temporary_buffer_capacity() {
        let mut counts = baseline();
        counts.candidate_admission_limit = 17;
        assert!(counts.validate().is_err());
    }

    #[test]
    fn reduction_policy_identity_is_required() {
        let mut counts = baseline();
        counts.reduction_policy_id.clear();
        assert!(counts.validate().is_err());
    }
}
