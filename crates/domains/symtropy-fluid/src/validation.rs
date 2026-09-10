// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Solver-independent continuum-fluid validation vocabulary.
//!
//! This module is deliberately separate from the current SPH scaffold. It does
//! not implement incompressible Navier-Stokes, reproduce any external blowup
//! construction, or grant gameplay authority to a research result. Its job is
//! narrower: preserve exact external provenance, represent diagnostics without
//! sentinel values, and distinguish ordinary numerical failure from loss of a
//! solver profile's declared validity.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Schema version for [`ExternalBenchmarkManifest`].
pub const EXTERNAL_BENCHMARK_SCHEMA_VERSION: u32 = 1;
/// Schema version for [`ContinuumDiagnosticSample`].
pub const CONTINUUM_DIAGNOSTIC_SCHEMA_VERSION: u32 = 1;

/// Public OpenAI formalization commit captured on 2026-09-10.
///
/// The commit identifies an external theorem/formalization artifact. It is not
/// a Symtropy qualification certificate and does not imply Clay/community
/// acceptance.
pub const OPENAI_NAVIER_STOKES_FORMALIZATION_COMMIT: &str =
    "8937a8f4cbc7abaab5e9e97d1cc7f5d2319d9538";

/// Domain variant associated with a theorem/reference benchmark.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceDomain {
    WholeSpaceR3,
    PeriodicTorus3,
}

/// Dated review state of an external reference captured by Symtropy.
///
/// This is evidence metadata, not a permanent judgement about mathematical
/// truth. A later review/source revision should produce successor provenance
/// rather than rewriting the earlier capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalReferenceStatus {
    /// Primary announcement/paper metadata and formal artifact were captured,
    /// but independent technical/community review is still pending.
    PrimaryAndFormalArtifactsCapturedReviewPending,
    /// A separately defined review gate has been satisfied for the declared
    /// benchmark use. V0 does not assign this state automatically.
    IndependentlyQualifiedForDeclaredUse,
    /// The captured source/reference has been superseded or materially
    /// corrected and should not be silently treated as current.
    SupersededOrCorrected,
}

/// Qualitative theorem-level property captured from a primary reference.
///
/// These values intentionally carry no invented numeric threshold or scaling
/// exponent. Numeric benchmark expectations belong to a later executable
/// fixture extracted directly from the primary construction.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceClaimKind {
    PositiveViscosity,
    SmoothForcing,
    StartsFromRest,
    FiniteTimeBreakdown,
    BoundedKineticEnergy,
    UnboundedVelocityApproachingTerminalTime,
}

/// One qualitative claim with an optional domain restriction.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ReferenceClaim {
    pub domain: Option<ReferenceDomain>,
    pub kind: ReferenceClaimKind,
}

/// Immutable-by-convention metadata for an external mathematical benchmark.
///
/// `executable_profile` is intentionally separate from theorem provenance: a
/// theorem can be captured before Symtropy possesses a faithful numerical
/// initial/forcing-state generator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalBenchmarkManifest {
    pub schema_version: u32,
    pub benchmark_id: String,
    pub title: String,
    /// ISO-8601 calendar date of the captured release.
    pub released_on: String,
    /// ISO-8601 calendar date on which this provenance snapshot was captured.
    pub captured_on: String,
    pub primary_source: String,
    pub formalization_repository: Option<String>,
    pub formalization_commit: Option<String>,
    pub domains: Vec<ReferenceDomain>,
    pub status: ExternalReferenceStatus,
    pub claims: Vec<ReferenceClaim>,
    /// True only after a separately reviewed numerical fixture exists.
    pub executable_profile: bool,
    /// Digest of that executable fixture when one exists. V0 does not define
    /// the digest preimage/algorithm; therefore the OpenAI capture leaves it
    /// absent and `executable_profile = false`.
    pub executable_fixture_digest: Option<String>,
}

impl ExternalBenchmarkManifest {
    /// Validate V0 structural/provenance invariants.
    pub fn validate(&self) -> Result<(), BenchmarkManifestError> {
        if self.schema_version != EXTERNAL_BENCHMARK_SCHEMA_VERSION {
            return Err(BenchmarkManifestError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.benchmark_id.trim().is_empty() {
            return Err(BenchmarkManifestError::EmptyBenchmarkId);
        }
        if self.title.trim().is_empty() {
            return Err(BenchmarkManifestError::EmptyTitle);
        }
        if !looks_like_iso_date(&self.released_on) || !looks_like_iso_date(&self.captured_on) {
            return Err(BenchmarkManifestError::InvalidDate);
        }
        if self.primary_source.trim().is_empty() {
            return Err(BenchmarkManifestError::EmptyPrimarySource);
        }
        if self.domains.is_empty() {
            return Err(BenchmarkManifestError::NoDomains);
        }
        if self.claims.is_empty() {
            return Err(BenchmarkManifestError::NoClaims);
        }

        let domain_count = self.domains.iter().copied().collect::<BTreeSet<_>>().len();
        if domain_count != self.domains.len() {
            return Err(BenchmarkManifestError::DuplicateDomain);
        }
        let claim_count = self.claims.iter().copied().collect::<BTreeSet<_>>().len();
        if claim_count != self.claims.len() {
            return Err(BenchmarkManifestError::DuplicateClaim);
        }

        match (&self.formalization_repository, &self.formalization_commit) {
            (Some(repository), Some(commit)) => {
                if repository.trim().is_empty() {
                    return Err(BenchmarkManifestError::InvalidFormalizationRepository);
                }
                if !is_full_hex_commit(commit) {
                    return Err(BenchmarkManifestError::InvalidFormalizationCommit);
                }
            }
            (None, None) => {}
            _ => return Err(BenchmarkManifestError::IncompleteFormalizationIdentity),
        }

        match (self.executable_profile, &self.executable_fixture_digest) {
            (true, Some(digest)) if !digest.trim().is_empty() => {}
            (false, None) => {}
            (true, _) => return Err(BenchmarkManifestError::ExecutableProfileMissingDigest),
            (false, Some(_)) => return Err(BenchmarkManifestError::DigestWithoutExecutableProfile),
        }

        Ok(())
    }
}

/// Structural problem in an external benchmark provenance manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BenchmarkManifestError {
    UnsupportedSchemaVersion(u32),
    EmptyBenchmarkId,
    EmptyTitle,
    InvalidDate,
    EmptyPrimarySource,
    NoDomains,
    NoClaims,
    DuplicateDomain,
    DuplicateClaim,
    InvalidFormalizationRepository,
    InvalidFormalizationCommit,
    IncompleteFormalizationIdentity,
    ExecutableProfileMissingDigest,
    DigestWithoutExecutableProfile,
}

impl fmt::Display for BenchmarkManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported external benchmark schema version {version}")
            }
            Self::EmptyBenchmarkId => write!(f, "benchmark_id must not be empty"),
            Self::EmptyTitle => write!(f, "benchmark title must not be empty"),
            Self::InvalidDate => write!(f, "released_on/captured_on must use YYYY-MM-DD"),
            Self::EmptyPrimarySource => write!(f, "primary_source must not be empty"),
            Self::NoDomains => write!(f, "benchmark must declare at least one domain"),
            Self::NoClaims => write!(f, "benchmark must declare at least one qualitative claim"),
            Self::DuplicateDomain => write!(f, "benchmark domains must be unique"),
            Self::DuplicateClaim => write!(f, "benchmark qualitative claims must be unique"),
            Self::InvalidFormalizationRepository => {
                write!(f, "formalization repository must not be empty")
            }
            Self::InvalidFormalizationCommit => {
                write!(f, "formalization commit must be a full 40-character hex SHA")
            }
            Self::IncompleteFormalizationIdentity => write!(
                f,
                "formalization repository and commit must either both be present or both absent"
            ),
            Self::ExecutableProfileMissingDigest => write!(
                f,
                "an executable benchmark profile requires a non-empty fixture digest"
            ),
            Self::DigestWithoutExecutableProfile => write!(
                f,
                "fixture digest cannot be present while executable_profile is false"
            ),
        }
    }
}

impl std::error::Error for BenchmarkManifestError {}

/// Why a scalar diagnostic is not represented by a measured number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticUnavailableReason {
    BackendDoesNotExpose,
    NotApplicable,
    ComputationFailed,
    OutsideReferenceDomain,
}

/// A scalar diagnostic whose measured branch must be finite and non-negative.
///
/// Most V0 continuum diagnostics are norms, magnitudes, scales, energies,
/// residuals, or counts represented as floating point; negative values are
/// therefore invalid. Signed diagnostics should use a future explicitly signed
/// type instead of weakening this contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "availability", content = "value", rename_all = "snake_case")]
pub enum NonNegativeDiagnostic {
    Measured(f64),
    Unavailable(DiagnosticUnavailableReason),
}

impl NonNegativeDiagnostic {
    /// Construct a measured diagnostic while enforcing V0 numeric invariants.
    pub fn measured(value: f64) -> Result<Self, DiagnosticValueError> {
        validate_non_negative_finite(value)?;
        Ok(Self::Measured(value))
    }

    pub const fn unavailable(reason: DiagnosticUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }

    pub fn validate(&self) -> Result<(), DiagnosticValueError> {
        if let Self::Measured(value) = self {
            validate_non_negative_finite(*value)?;
        }
        Ok(())
    }

    pub fn measured_value(&self) -> Option<f64> {
        match self {
            Self::Measured(value) => Some(*value),
            Self::Unavailable(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticValueError {
    NonFinite,
    Negative,
}

impl fmt::Display for DiagnosticValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => write!(f, "measured diagnostic must be finite"),
            Self::Negative => write!(f, "measured diagnostic must be non-negative"),
        }
    }
}

impl std::error::Error for DiagnosticValueError {}

/// Solver-independent sampled diagnostics for a continuum-fluid run.
///
/// `diagnostic_profile` binds backend-specific choices such as discrete
/// divergence operator, quadrature, filtering, region of interest, and solver
/// residual normalization. Consequently a raw scalar is never expected to be
/// portable without its profile identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContinuumDiagnosticSample {
    pub schema_version: u32,
    pub diagnostic_profile: String,
    pub time_s: f64,
    /// Maximum speed represented by the discrete solver/sample set. This is not
    /// automatically a certified continuum L-infinity norm.
    pub max_resolved_speed_mps: NonNegativeDiagnostic,
    /// Physical joules only when density/domain/quadrature make that statement
    /// meaningful; otherwise use an unavailable reason.
    pub kinetic_energy_j: NonNegativeDiagnostic,
    pub divergence_rms_per_s: NonNegativeDiagnostic,
    pub max_vorticity_per_s: NonNegativeDiagnostic,
    pub max_strain_rate_per_s: NonNegativeDiagnostic,
    pub max_pressure_gradient_pa_per_m: NonNegativeDiagnostic,
    pub max_cfl: NonNegativeDiagnostic,
    pub minimum_resolved_length_m: NonNegativeDiagnostic,
    /// Optional/profile-specific estimator; its algorithm belongs in
    /// `diagnostic_profile`/benchmark evidence identity.
    pub concentration_scale_m: NonNegativeDiagnostic,
    pub solver_residual: NonNegativeDiagnostic,
    pub forcing_residual: NonNegativeDiagnostic,
    /// Number of NaN/Inf/overflow states detected by the backend. Non-finite
    /// values are not smuggled into numeric metrics.
    pub non_finite_state_count: u64,
}

impl ContinuumDiagnosticSample {
    pub fn validate(&self) -> Result<(), ContinuumSampleError> {
        if self.schema_version != CONTINUUM_DIAGNOSTIC_SCHEMA_VERSION {
            return Err(ContinuumSampleError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.diagnostic_profile.trim().is_empty() {
            return Err(ContinuumSampleError::EmptyDiagnosticProfile);
        }
        if !self.time_s.is_finite() || self.time_s < 0.0 {
            return Err(ContinuumSampleError::InvalidTime);
        }

        for (name, diagnostic) in [
            ("max_resolved_speed_mps", &self.max_resolved_speed_mps),
            ("kinetic_energy_j", &self.kinetic_energy_j),
            ("divergence_rms_per_s", &self.divergence_rms_per_s),
            ("max_vorticity_per_s", &self.max_vorticity_per_s),
            ("max_strain_rate_per_s", &self.max_strain_rate_per_s),
            (
                "max_pressure_gradient_pa_per_m",
                &self.max_pressure_gradient_pa_per_m,
            ),
            ("max_cfl", &self.max_cfl),
            ("minimum_resolved_length_m", &self.minimum_resolved_length_m),
            ("concentration_scale_m", &self.concentration_scale_m),
            ("solver_residual", &self.solver_residual),
            ("forcing_residual", &self.forcing_residual),
        ] {
            diagnostic
                .validate()
                .map_err(|source| ContinuumSampleError::InvalidDiagnostic { name, source })?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContinuumSampleError {
    UnsupportedSchemaVersion(u32),
    EmptyDiagnosticProfile,
    InvalidTime,
    InvalidDiagnostic {
        name: &'static str,
        source: DiagnosticValueError,
    },
}

impl fmt::Display for ContinuumSampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported continuum diagnostic schema version {version}")
            }
            Self::EmptyDiagnosticProfile => write!(f, "diagnostic_profile must not be empty"),
            Self::InvalidTime => write!(f, "time_s must be finite and non-negative"),
            Self::InvalidDiagnostic { name, source } => {
                write!(f, "invalid diagnostic {name}: {source}")
            }
        }
    }
}

impl std::error::Error for ContinuumSampleError {}

/// Measurement/solver validity state for one continuum profile.
///
/// None of these states is a theorem prover. In particular, numerical NaN/Inf,
/// solver divergence, CFL failure, or under-resolution is never automatically
/// evidence that a mathematical Navier-Stokes singularity occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuumValidityState {
    Resolved,
    UnderResolved,
    CflViolation,
    DivergenceFailure,
    SolverNonConvergence,
    NonFiniteState,
    ReferenceDomainViolation,
    BenchmarkProfileUnavailable,
}

/// Return the dated V0 provenance capture for the September 2026 external
/// forced Navier-Stokes result.
///
/// The manifest intentionally has no executable fixture. Turning it into a
/// numeric benchmark requires a separate direct-paper extraction/review gate.
pub fn openai_2026_forced_navier_stokes_manifest() -> ExternalBenchmarkManifest {
    ExternalBenchmarkManifest {
        schema_version: EXTERNAL_BENCHMARK_SCHEMA_VERSION,
        benchmark_id: "openai-forced-navier-stokes-blowup-2026".to_owned(),
        title: "Finite Time Blowup for Navier-Stokes".to_owned(),
        released_on: "2026-09-08".to_owned(),
        captured_on: "2026-09-10".to_owned(),
        primary_source: "OpenAI: On the Navier-Stokes Millennium Prize Problem".to_owned(),
        formalization_repository: Some("openai/NavierStokesAndEuler".to_owned()),
        formalization_commit: Some(OPENAI_NAVIER_STOKES_FORMALIZATION_COMMIT.to_owned()),
        domains: vec![ReferenceDomain::WholeSpaceR3, ReferenceDomain::PeriodicTorus3],
        status: ExternalReferenceStatus::PrimaryAndFormalArtifactsCapturedReviewPending,
        claims: vec![
            ReferenceClaim {
                domain: None,
                kind: ReferenceClaimKind::PositiveViscosity,
            },
            ReferenceClaim {
                domain: None,
                kind: ReferenceClaimKind::SmoothForcing,
            },
            ReferenceClaim {
                domain: Some(ReferenceDomain::WholeSpaceR3),
                kind: ReferenceClaimKind::StartsFromRest,
            },
            ReferenceClaim {
                domain: None,
                kind: ReferenceClaimKind::FiniteTimeBreakdown,
            },
            ReferenceClaim {
                domain: Some(ReferenceDomain::WholeSpaceR3),
                kind: ReferenceClaimKind::BoundedKineticEnergy,
            },
            ReferenceClaim {
                domain: Some(ReferenceDomain::WholeSpaceR3),
                kind: ReferenceClaimKind::UnboundedVelocityApproachingTerminalTime,
            },
        ],
        executable_profile: false,
        executable_fixture_digest: None,
    }
}

fn validate_non_negative_finite(value: f64) -> Result<(), DiagnosticValueError> {
    if !value.is_finite() {
        return Err(DiagnosticValueError::NonFinite);
    }
    if value < 0.0 {
        return Err(DiagnosticValueError::Negative);
    }
    Ok(())
}

fn is_full_hex_commit(commit: &str) -> bool {
    commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn looks_like_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unavailable() -> NonNegativeDiagnostic {
        NonNegativeDiagnostic::unavailable(DiagnosticUnavailableReason::BackendDoesNotExpose)
    }

    fn empty_sample() -> ContinuumDiagnosticSample {
        ContinuumDiagnosticSample {
            schema_version: CONTINUUM_DIAGNOSTIC_SCHEMA_VERSION,
            diagnostic_profile: "unit-test-v0".to_owned(),
            time_s: 0.0,
            max_resolved_speed_mps: unavailable(),
            kinetic_energy_j: unavailable(),
            divergence_rms_per_s: unavailable(),
            max_vorticity_per_s: unavailable(),
            max_strain_rate_per_s: unavailable(),
            max_pressure_gradient_pa_per_m: unavailable(),
            max_cfl: unavailable(),
            minimum_resolved_length_m: unavailable(),
            concentration_scale_m: unavailable(),
            solver_residual: unavailable(),
            forcing_residual: unavailable(),
            non_finite_state_count: 0,
        }
    }

    #[test]
    fn captured_openai_manifest_is_structurally_valid_and_non_executable() {
        let manifest = openai_2026_forced_navier_stokes_manifest();
        assert_eq!(manifest.validate(), Ok(()));
        assert_eq!(
            manifest.formalization_commit.as_deref(),
            Some(OPENAI_NAVIER_STOKES_FORMALIZATION_COMMIT)
        );
        assert_eq!(
            manifest.status,
            ExternalReferenceStatus::PrimaryAndFormalArtifactsCapturedReviewPending
        );
        assert!(!manifest.executable_profile);
        assert!(manifest.executable_fixture_digest.is_none());
        assert_eq!(
            manifest.domains,
            vec![ReferenceDomain::WholeSpaceR3, ReferenceDomain::PeriodicTorus3]
        );
    }

    #[test]
    fn executable_profile_requires_an_explicit_fixture_digest() {
        let mut manifest = openai_2026_forced_navier_stokes_manifest();
        manifest.executable_profile = true;
        assert_eq!(
            manifest.validate(),
            Err(BenchmarkManifestError::ExecutableProfileMissingDigest)
        );
    }

    #[test]
    fn provenance_rejects_partial_or_short_formalization_identity() {
        let mut manifest = openai_2026_forced_navier_stokes_manifest();
        manifest.formalization_commit = Some("8937a8f4".to_owned());
        assert_eq!(
            manifest.validate(),
            Err(BenchmarkManifestError::InvalidFormalizationCommit)
        );

        manifest.formalization_commit = None;
        assert_eq!(
            manifest.validate(),
            Err(BenchmarkManifestError::IncompleteFormalizationIdentity)
        );
    }

    #[test]
    fn provenance_rejects_duplicate_domain_or_claim() {
        let mut manifest = openai_2026_forced_navier_stokes_manifest();
        manifest.domains.push(ReferenceDomain::WholeSpaceR3);
        assert_eq!(manifest.validate(), Err(BenchmarkManifestError::DuplicateDomain));

        let mut manifest = openai_2026_forced_navier_stokes_manifest();
        manifest.claims.push(manifest.claims[0]);
        assert_eq!(manifest.validate(), Err(BenchmarkManifestError::DuplicateClaim));
    }

    #[test]
    fn measured_diagnostic_rejects_negative_and_non_finite_values() {
        assert_eq!(
            NonNegativeDiagnostic::measured(-1.0),
            Err(DiagnosticValueError::Negative)
        );
        assert_eq!(
            NonNegativeDiagnostic::measured(f64::NAN),
            Err(DiagnosticValueError::NonFinite)
        );
        assert_eq!(
            NonNegativeDiagnostic::measured(f64::INFINITY),
            Err(DiagnosticValueError::NonFinite)
        );
        assert_eq!(
            NonNegativeDiagnostic::measured(0.0)
                .unwrap()
                .measured_value(),
            Some(0.0)
        );
    }

    #[test]
    fn unavailable_reason_is_not_collapsed_to_numeric_zero() {
        let unavailable = NonNegativeDiagnostic::unavailable(
            DiagnosticUnavailableReason::BackendDoesNotExpose,
        );
        let measured_zero = NonNegativeDiagnostic::measured(0.0).unwrap();
        assert_ne!(unavailable, measured_zero);
        assert_eq!(unavailable.measured_value(), None);
        assert_eq!(measured_zero.measured_value(), Some(0.0));
    }

    #[test]
    fn sample_validation_catches_deserialized_invalid_measured_values() {
        let mut sample = empty_sample();
        // Public wire types can be constructed/deserialized without the helper;
        // aggregate validation must therefore re-check the numeric invariant.
        sample.max_cfl = NonNegativeDiagnostic::Measured(f64::NAN);
        assert_eq!(
            sample.validate(),
            Err(ContinuumSampleError::InvalidDiagnostic {
                name: "max_cfl",
                source: DiagnosticValueError::NonFinite,
            })
        );
    }

    #[test]
    fn sample_time_and_profile_are_fail_closed() {
        let mut sample = empty_sample();
        sample.time_s = f64::INFINITY;
        assert_eq!(sample.validate(), Err(ContinuumSampleError::InvalidTime));

        let mut sample = empty_sample();
        sample.diagnostic_profile.clear();
        assert_eq!(
            sample.validate(),
            Err(ContinuumSampleError::EmptyDiagnosticProfile)
        );
    }

    #[test]
    fn numerical_failure_states_do_not_encode_a_mathematical_singularity_claim() {
        let ordinary_failure_states = [
            ContinuumValidityState::UnderResolved,
            ContinuumValidityState::CflViolation,
            ContinuumValidityState::DivergenceFailure,
            ContinuumValidityState::SolverNonConvergence,
            ContinuumValidityState::NonFiniteState,
        ];
        assert_eq!(ordinary_failure_states.len(), 5);
        assert!(!ordinary_failure_states.contains(&ContinuumValidityState::Resolved));
        // There intentionally is no `MathematicalSingularityDetected` variant.
    }
}
