// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Append-only continuum validity evidence lineage.
//!
//! A later finer-profile result may supersede the *current interpretation* of a
//! flow, but it must not delete earlier under-resolution or solver-failure
//! evidence. This module preserves that history structurally. It does not decide
//! validity, request refinement, authorize semantic transitions, or select an
//! execution backend.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::problem_revision_envelope::MAX_REVISION_ID_BYTES;
use crate::refinement_policy_contract::MAX_PROFILE_ID_BYTES;
use crate::validation::ContinuumValidityState;

pub const CONTINUUM_VALIDITY_EVIDENCE_LINEAGE_SCHEMA_ID: &str =
    "continuum-validity-evidence-lineage-v0.1";
pub const MAX_EVIDENCE_ID_BYTES: usize = 256;
pub const MAX_SUBJECT_ID_BYTES: usize = 256;
pub const MAX_LINEAGE_ENTRIES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuumValidityEvidenceEntry {
    /// Zero-based canonical append position.
    pub sequence: u64,
    pub evidence_id: String,
    /// Exact qualification/source revision for this evidence record.
    pub evidence_revision: String,
    pub semantic_profile_id: String,
    pub diagnostic_profile_id: String,
    pub state_revision_id: String,
    pub logical_snapshot_id: String,
    /// Exact IEEE-754 bits of the sampled physical time.
    pub sample_time_bits: u64,
    pub validity: ContinuumValidityState,
    /// Exact predecessor evidence ID. Only the first entry may omit it.
    pub predecessor_evidence_id: Option<String>,
}

impl ContinuumValidityEvidenceEntry {
    pub fn sample_time_s(&self) -> f64 {
        f64::from_bits(self.sample_time_bits)
    }

    fn validate_standalone(&self) -> Result<(), ContinuumValidityEvidenceLineageError> {
        validate_id("evidence_id", &self.evidence_id, MAX_EVIDENCE_ID_BYTES)?;
        validate_id(
            "evidence_revision",
            &self.evidence_revision,
            MAX_REVISION_ID_BYTES,
        )?;
        validate_id(
            "semantic_profile_id",
            &self.semantic_profile_id,
            MAX_PROFILE_ID_BYTES,
        )?;
        validate_id(
            "diagnostic_profile_id",
            &self.diagnostic_profile_id,
            MAX_PROFILE_ID_BYTES,
        )?;
        validate_id(
            "state_revision_id",
            &self.state_revision_id,
            MAX_REVISION_ID_BYTES,
        )?;
        validate_id(
            "logical_snapshot_id",
            &self.logical_snapshot_id,
            MAX_REVISION_ID_BYTES,
        )?;
        if let Some(predecessor) = &self.predecessor_evidence_id {
            validate_id(
                "predecessor_evidence_id",
                predecessor,
                MAX_EVIDENCE_ID_BYTES,
            )?;
        }
        let time_s = self.sample_time_s();
        if !time_s.is_finite() || time_s < 0.0 {
            return Err(ContinuumValidityEvidenceLineageError::InvalidSampleTime);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuumValidityEvidenceLineage {
    pub schema_id: String,
    /// Stable physical/process subject identity across semantic profile changes.
    pub subject_id: String,
    /// Canonical append order. Earlier entries are never removed when a later
    /// profile reports `Resolved`.
    pub entries: Vec<ContinuumValidityEvidenceEntry>,
}

impl ContinuumValidityEvidenceLineage {
    pub fn new(
        subject_id: String,
        first: ContinuumValidityEvidenceEntry,
    ) -> Result<Self, ContinuumValidityEvidenceLineageError> {
        if first.sequence != 0 || first.predecessor_evidence_id.is_some() {
            return Err(ContinuumValidityEvidenceLineageError::InvalidFirstEntry);
        }
        let lineage = Self {
            schema_id: CONTINUUM_VALIDITY_EVIDENCE_LINEAGE_SCHEMA_ID.to_owned(),
            subject_id,
            entries: vec![first],
        };
        lineage.validate()?;
        Ok(lineage)
    }

    /// Append one evidence record without mutating or dropping prior evidence.
    pub fn append(
        &mut self,
        mut next: ContinuumValidityEvidenceEntry,
    ) -> Result<(), ContinuumValidityEvidenceLineageError> {
        self.validate()?;
        let predecessor = self
            .entries
            .last()
            .ok_or(ContinuumValidityEvidenceLineageError::EmptyLineage)?;
        next.sequence = self.entries.len() as u64;
        next.predecessor_evidence_id = Some(predecessor.evidence_id.clone());
        next.validate_standalone()?;
        if self
            .entries
            .iter()
            .any(|entry| entry.evidence_id == next.evidence_id)
        {
            return Err(ContinuumValidityEvidenceLineageError::DuplicateEvidenceId);
        }
        if self.entries.len() >= MAX_LINEAGE_ENTRIES {
            return Err(ContinuumValidityEvidenceLineageError::TooManyEntries);
        }
        self.entries.push(next);
        self.validate()?;
        Ok(())
    }

    pub fn latest(&self) -> Option<&ContinuumValidityEvidenceEntry> {
        self.entries.last()
    }

    pub fn unresolved_entries(&self) -> impl Iterator<Item = &ContinuumValidityEvidenceEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.validity != ContinuumValidityState::Resolved)
    }

    pub fn validate(&self) -> Result<(), ContinuumValidityEvidenceLineageError> {
        if self.schema_id != CONTINUUM_VALIDITY_EVIDENCE_LINEAGE_SCHEMA_ID {
            return Err(ContinuumValidityEvidenceLineageError::WrongSchemaId);
        }
        validate_id("subject_id", &self.subject_id, MAX_SUBJECT_ID_BYTES)?;
        if self.entries.is_empty() {
            return Err(ContinuumValidityEvidenceLineageError::EmptyLineage);
        }
        if self.entries.len() > MAX_LINEAGE_ENTRIES {
            return Err(ContinuumValidityEvidenceLineageError::TooManyEntries);
        }

        let mut evidence_ids = BTreeSet::new();
        for (index, entry) in self.entries.iter().enumerate() {
            entry.validate_standalone()?;
            if entry.sequence != index as u64 {
                return Err(ContinuumValidityEvidenceLineageError::NonCanonicalSequence);
            }
            if !evidence_ids.insert(entry.evidence_id.as_str()) {
                return Err(ContinuumValidityEvidenceLineageError::DuplicateEvidenceId);
            }
            match index {
                0 if entry.predecessor_evidence_id.is_some() => {
                    return Err(ContinuumValidityEvidenceLineageError::InvalidFirstEntry);
                }
                0 => {}
                _ => {
                    let expected = &self.entries[index - 1].evidence_id;
                    if entry.predecessor_evidence_id.as_deref() != Some(expected.as_str()) {
                        return Err(ContinuumValidityEvidenceLineageError::PredecessorMismatch);
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContinuumValidityEvidenceLineageError {
    WrongSchemaId,
    EmptyField(&'static str),
    FieldTooLong {
        field: &'static str,
        max_bytes: usize,
    },
    InvalidSampleTime,
    EmptyLineage,
    TooManyEntries,
    InvalidFirstEntry,
    NonCanonicalSequence,
    DuplicateEvidenceId,
    PredecessorMismatch,
}

impl fmt::Display for ContinuumValidityEvidenceLineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongSchemaId => {
                write!(f, "unsupported continuum validity evidence lineage schema")
            }
            Self::EmptyField(field) => write!(f, "{field} must not be empty"),
            Self::FieldTooLong { field, max_bytes } => {
                write!(f, "{field} must not exceed {max_bytes} bytes")
            }
            Self::InvalidSampleTime => write!(f, "sample time must be finite and non-negative"),
            Self::EmptyLineage => write!(f, "validity evidence lineage must not be empty"),
            Self::TooManyEntries => write!(f, "validity evidence lineage exceeds its entry bound"),
            Self::InvalidFirstEntry => write!(
                f,
                "first validity evidence entry must have sequence zero and no predecessor"
            ),
            Self::NonCanonicalSequence => {
                write!(
                    f,
                    "validity evidence sequence is not canonical and contiguous"
                )
            }
            Self::DuplicateEvidenceId => write!(f, "validity evidence IDs must be unique"),
            Self::PredecessorMismatch => write!(
                f,
                "validity evidence predecessor does not name the immediately preceding entry"
            ),
        }
    }
}

impl std::error::Error for ContinuumValidityEvidenceLineageError {}

fn validate_id(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), ContinuumValidityEvidenceLineageError> {
    if value.trim().is_empty() {
        return Err(ContinuumValidityEvidenceLineageError::EmptyField(field));
    }
    if value.len() > max_bytes {
        return Err(ContinuumValidityEvidenceLineageError::FieldTooLong { field, max_bytes });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        id: &str,
        profile: &str,
        validity: ContinuumValidityState,
    ) -> ContinuumValidityEvidenceEntry {
        ContinuumValidityEvidenceEntry {
            sequence: 0,
            evidence_id: id.to_owned(),
            evidence_revision: format!("exact-head:{id}"),
            semantic_profile_id: profile.to_owned(),
            diagnostic_profile_id: format!("{profile};diag=v0.1"),
            state_revision_id: "state:001".to_owned(),
            logical_snapshot_id: "snapshot:001".to_owned(),
            sample_time_bits: 1.0_f64.to_bits(),
            validity,
            predecessor_evidence_id: None,
        }
    }

    #[test]
    fn resolved_successor_cannot_erase_under_resolved_history() {
        let mut lineage = ContinuumValidityEvidenceLineage::new(
            "watershed-cell:17".to_owned(),
            entry(
                "coarse-under-resolved",
                "continuum-coarse-v0.1",
                ContinuumValidityState::UnderResolved,
            ),
        )
        .unwrap();

        lineage
            .append(entry(
                "fine-resolved",
                "continuum-fine-v0.1",
                ContinuumValidityState::Resolved,
            ))
            .unwrap();

        assert_eq!(lineage.entries.len(), 2);
        assert_eq!(
            lineage.entries[0].validity,
            ContinuumValidityState::UnderResolved
        );
        assert_eq!(
            lineage.latest().unwrap().validity,
            ContinuumValidityState::Resolved
        );
        assert_eq!(lineage.unresolved_entries().count(), 1);
        lineage.validate().unwrap();
    }

    #[test]
    fn later_coarse_result_still_retains_prior_failure_evidence() {
        let mut lineage = ContinuumValidityEvidenceLineage::new(
            "estuary:4".to_owned(),
            entry(
                "coarse-cfl-failure",
                "continuum-coarse-v0.1",
                ContinuumValidityState::CflViolation,
            ),
        )
        .unwrap();
        lineage
            .append(entry(
                "fine-resolved",
                "continuum-fine-v0.1",
                ContinuumValidityState::Resolved,
            ))
            .unwrap();
        lineage
            .append(entry(
                "coarse-later-resolved",
                "continuum-coarse-v0.1",
                ContinuumValidityState::Resolved,
            ))
            .unwrap();

        assert_eq!(lineage.entries.len(), 3);
        assert!(
            lineage
                .entries
                .iter()
                .any(|entry| entry.validity == ContinuumValidityState::CflViolation)
        );
        assert_eq!(lineage.unresolved_entries().count(), 1);
    }

    #[test]
    fn retained_receipt_rejects_deleted_middle_link_or_reordering() {
        let mut lineage = ContinuumValidityEvidenceLineage::new(
            "river-reach:9".to_owned(),
            entry(
                "first",
                "continuum-v0.1",
                ContinuumValidityState::UnderResolved,
            ),
        )
        .unwrap();
        lineage
            .append(entry(
                "second",
                "continuum-v0.2",
                ContinuumValidityState::Resolved,
            ))
            .unwrap();
        lineage
            .append(entry(
                "third",
                "continuum-v0.3",
                ContinuumValidityState::Resolved,
            ))
            .unwrap();

        let mut deleted = lineage.clone();
        deleted.entries.remove(1);
        assert!(matches!(
            deleted.validate().unwrap_err(),
            ContinuumValidityEvidenceLineageError::NonCanonicalSequence
                | ContinuumValidityEvidenceLineageError::PredecessorMismatch
        ));

        let mut reordered = lineage;
        reordered.entries.swap(1, 2);
        assert!(matches!(
            reordered.validate().unwrap_err(),
            ContinuumValidityEvidenceLineageError::NonCanonicalSequence
                | ContinuumValidityEvidenceLineageError::PredecessorMismatch
        ));
    }

    #[test]
    fn non_finite_retained_time_fails_closed() {
        let mut first = entry(
            "nan-time",
            "continuum-v0.1",
            ContinuumValidityState::UnderResolved,
        );
        first.sample_time_bits = f64::NAN.to_bits();
        assert_eq!(
            ContinuumValidityEvidenceLineage::new("subject".to_owned(), first).unwrap_err(),
            ContinuumValidityEvidenceLineageError::InvalidSampleTime
        );
    }
}
