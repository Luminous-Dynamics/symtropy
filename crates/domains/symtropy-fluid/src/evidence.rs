// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Generic source/profile binding for continuum verification evidence.
//!
//! This envelope is deliberately independent of any one benchmark. External
//! theorem provenance belongs to `validation::ExternalBenchmarkManifest`; this
//! module binds an executable result to the exact Symtropy source and declared
//! numerical/case profiles that produced it.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const CONTINUUM_EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const MAX_EVIDENCE_EXECUTION_PROFILES: usize = 16;
pub const MAX_EVIDENCE_PROFILE_BYTES: usize = 256;
pub const MAX_EVIDENCE_CASE_PROFILE_BYTES: usize = 256;
pub const MAX_EVIDENCE_BENCHMARK_ID_BYTES: usize = 128;
pub const MAX_EVIDENCE_FIXTURE_DIGEST_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuumEvidenceSubject {
    pub schema_version: u32,
    /// Exact Symtropy commit that produced the executable evidence.
    pub source_revision: String,
    /// Exact numerical backend/configuration identities represented by the
    /// payload. A temporal ladder may have one; a spatial ladder normally has
    /// several because grid resolution is part of the solver profile.
    pub execution_profiles: Vec<String>,
    /// Comparator/manufactured/fixture identity for this particular campaign.
    pub case_profile: String,
    /// Optional external benchmark manifest identifier when the run is tied to
    /// a captured external reference. Smooth internal controls normally omit it.
    pub benchmark_id: Option<String>,
    /// Optional immutable executable-fixture digest. This is distinct from the
    /// external source/formalization commit and must not be invented when no
    /// reviewed executable fixture exists.
    pub fixture_digest: Option<String>,
}

impl ContinuumEvidenceSubject {
    pub fn validate(&self) -> Result<(), ContinuumEvidenceError> {
        if self.schema_version != CONTINUUM_EVIDENCE_SCHEMA_VERSION {
            return Err(ContinuumEvidenceError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if !is_full_lower_hex_commit(&self.source_revision) {
            return Err(ContinuumEvidenceError::InvalidSourceRevision);
        }
        if self.execution_profiles.is_empty() {
            return Err(ContinuumEvidenceError::NoExecutionProfiles);
        }
        if self.execution_profiles.len() > MAX_EVIDENCE_EXECUTION_PROFILES {
            return Err(ContinuumEvidenceError::TooManyExecutionProfiles);
        }
        let mut unique_profiles = BTreeSet::new();
        for execution_profile in &self.execution_profiles {
            validate_non_empty_bounded(
                execution_profile,
                MAX_EVIDENCE_PROFILE_BYTES,
                ContinuumEvidenceError::InvalidExecutionProfile,
            )?;
            if !unique_profiles.insert(execution_profile.as_str()) {
                return Err(ContinuumEvidenceError::DuplicateExecutionProfile);
            }
        }
        validate_non_empty_bounded(
            &self.case_profile,
            MAX_EVIDENCE_CASE_PROFILE_BYTES,
            ContinuumEvidenceError::InvalidCaseProfile,
        )?;
        if let Some(benchmark_id) = &self.benchmark_id {
            validate_non_empty_bounded(
                benchmark_id,
                MAX_EVIDENCE_BENCHMARK_ID_BYTES,
                ContinuumEvidenceError::InvalidBenchmarkId,
            )?;
        }
        if let Some(fixture_digest) = &self.fixture_digest {
            validate_non_empty_bounded(
                fixture_digest,
                MAX_EVIDENCE_FIXTURE_DIGEST_BYTES,
                ContinuumEvidenceError::InvalidFixtureDigest,
            )?;
        }
        Ok(())
    }

    pub fn bind<T>(
        self,
        payload: T,
    ) -> Result<ContinuumEvidenceEnvelope<T>, ContinuumEvidenceError> {
        self.validate()?;
        Ok(ContinuumEvidenceEnvelope {
            schema_version: CONTINUUM_EVIDENCE_SCHEMA_VERSION,
            subject: self,
            payload,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContinuumEvidenceEnvelope<T> {
    pub schema_version: u32,
    pub subject: ContinuumEvidenceSubject,
    pub payload: T,
}

impl<T> ContinuumEvidenceEnvelope<T> {
    pub fn validate_subject(&self) -> Result<(), ContinuumEvidenceError> {
        if self.schema_version != CONTINUUM_EVIDENCE_SCHEMA_VERSION {
            return Err(ContinuumEvidenceError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        self.subject.validate()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuumEvidenceError {
    UnsupportedSchemaVersion(u32),
    InvalidSourceRevision,
    NoExecutionProfiles,
    TooManyExecutionProfiles,
    InvalidExecutionProfile,
    DuplicateExecutionProfile,
    InvalidCaseProfile,
    InvalidBenchmarkId,
    InvalidFixtureDigest,
}

impl fmt::Display for ContinuumEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported continuum evidence schema version {version}")
            }
            Self::InvalidSourceRevision => write!(
                f,
                "source_revision must be a canonical 40-character lowercase hex commit"
            ),
            Self::NoExecutionProfiles => {
                write!(
                    f,
                    "evidence subject requires at least one execution profile"
                )
            }
            Self::TooManyExecutionProfiles => write!(
                f,
                "evidence subject exceeds {MAX_EVIDENCE_EXECUTION_PROFILES} execution profiles"
            ),
            Self::InvalidExecutionProfile => write!(
                f,
                "execution profile must be non-empty and within the V0 evidence bound"
            ),
            Self::DuplicateExecutionProfile => {
                write!(f, "execution profiles must be unique")
            }
            Self::InvalidCaseProfile => write!(
                f,
                "case_profile must be non-empty and within the V0 evidence bound"
            ),
            Self::InvalidBenchmarkId => write!(
                f,
                "benchmark_id must be non-empty and within the V0 evidence bound"
            ),
            Self::InvalidFixtureDigest => write!(
                f,
                "fixture_digest must be non-empty and within the V0 evidence bound"
            ),
        }
    }
}

impl std::error::Error for ContinuumEvidenceError {}

fn validate_non_empty_bounded(
    value: &str,
    max_bytes: usize,
    error: ContinuumEvidenceError,
) -> Result<(), ContinuumEvidenceError> {
    if value.trim().is_empty() || value.len() > max_bytes {
        return Err(error);
    }
    Ok(())
}

fn is_full_lower_hex_commit(commit: &str) -> bool {
    commit.len() == 40
        && commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_subject() -> ContinuumEvidenceSubject {
        ContinuumEvidenceSubject {
            schema_version: CONTINUUM_EVIDENCE_SCHEMA_VERSION,
            source_revision: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            execution_profiles: vec!["periodic-mac2d-reference-v0.1;n=16x16".to_owned()],
            case_profile: "manufactured-taylor-green-periodic-2d-v0.1".to_owned(),
            benchmark_id: None,
            fixture_digest: None,
        }
    }

    #[test]
    fn smooth_control_subject_binds_without_external_benchmark_metadata() {
        let envelope = test_subject().bind(vec![1_u32, 2, 3]).unwrap();
        assert_eq!(envelope.validate_subject(), Ok(()));
        assert_eq!(envelope.payload, vec![1, 2, 3]);
    }

    #[test]
    fn source_revision_is_exact_and_fail_closed() {
        let mut subject = test_subject();
        subject.source_revision = "0123456789ABCDEF0123456789abcdef01234567".to_owned();
        assert_eq!(
            subject.validate(),
            Err(ContinuumEvidenceError::InvalidSourceRevision)
        );
    }

    #[test]
    fn execution_profile_set_is_bounded_and_unique() {
        let mut subject = test_subject();
        subject
            .execution_profiles
            .push(subject.execution_profiles[0].clone());
        assert_eq!(
            subject.validate(),
            Err(ContinuumEvidenceError::DuplicateExecutionProfile)
        );

        let mut subject = test_subject();
        subject.execution_profiles.clear();
        assert_eq!(
            subject.validate(),
            Err(ContinuumEvidenceError::NoExecutionProfiles)
        );
    }

    #[test]
    fn empty_optional_identity_is_not_treated_as_absent() {
        let mut subject = test_subject();
        subject.benchmark_id = Some(String::new());
        assert_eq!(
            subject.validate(),
            Err(ContinuumEvidenceError::InvalidBenchmarkId)
        );
    }
}
