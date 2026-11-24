// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Fail-closed validation for collision-detection benchmark receipts.
//!
//! This crate validates denominators, independent-oracle recall, culling
//! certificate coverage, fallback accounting, execution completeness and
//! timing summaries. It does not run collision detection or authenticate
//! external datasets, algorithms, benchmark hosts or geometric oracles.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunStatus {
    Completed,
    TimedOut,
    OutOfMemory,
    Aborted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionProfile {
    SymtropyNativeDiscrete,
    SymtropyQ16_16Discrete,
    LinearizedTrajectoryOracle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatasetIdentity {
    pub name: String,
    pub version: String,
    pub source_url: String,
    pub sha256: String,
}

impl DatasetIdentity {
    #[must_use]
    pub fn valid(&self) -> bool {
        !self.name.trim().is_empty()
            && !self.version.trim().is_empty()
            && !self.source_url.trim().is_empty()
            && is_lower_hex(&self.sha256, 64)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlgorithmIdentity {
    pub name: String,
    pub version: String,
    pub source_commit: String,
}

impl AlgorithmIdentity {
    #[must_use]
    pub fn valid(&self) -> bool {
        !self.name.trim().is_empty()
            && !self.version.trim().is_empty()
            && is_lower_hex(&self.source_commit, 40)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Environment {
    pub operating_system: String,
    pub architecture: String,
    pub cpu_model: String,
    pub physical_cores: u32,
    pub worker_threads: u32,
    pub compiler: String,
    pub build_profile: String,
    pub gpu_model: Option<String>,
    pub gpu_driver: Option<String>,
}

impl Environment {
    #[must_use]
    pub fn valid(&self) -> bool {
        !self.operating_system.trim().is_empty()
            && !self.architecture.trim().is_empty()
            && !self.cpu_model.trim().is_empty()
            && self.physical_cores > 0
            && self.worker_threads > 0
            && !self.compiler.trim().is_empty()
            && !self.build_profile.trim().is_empty()
            && self
                .gpu_model
                .as_ref()
                .is_none_or(|value| !value.trim().is_empty())
            && self
                .gpu_driver
                .as_ref()
                .is_none_or(|value| !value.trim().is_empty())
            && self.gpu_model.is_some() == self.gpu_driver.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PairAccounting {
    pub eligible_pairs: u64,
    pub oracle_positive_pairs: u64,
    pub oracle_negative_pairs: u64,
    pub forwarded_pairs: u64,
    pub forwarded_true_positives: u64,
    pub forwarded_false_positives: u64,
    pub certified_pruned_pairs: u64,
    pub accepted_certificates: u64,
    pub replayed_certificates: u64,
    pub rejected_certificate_attempts: u64,
    pub fallback_pairs: u64,
    pub unsupported_fallbacks: u64,
    pub invalid_fallbacks: u64,
    pub overflow_fallbacks: u64,
}

impl PairAccounting {
    /// Oracle-positive pairs omitted by the forwarded set. None signals
    /// impossible input accounting; it must never be coerced to zero.
    #[must_use]
    pub fn false_negatives(&self) -> Option<u64> {
        self.oracle_positive_pairs
            .checked_sub(self.forwarded_true_positives)
    }

    #[must_use]
    pub fn false_positive_rate_basis_points(&self) -> Option<u64> {
        if self.oracle_negative_pairs == 0 {
            return None;
        }
        Some(
            (u128::from(self.forwarded_false_positives) * 10_000
                / u128::from(self.oracle_negative_pairs)) as u64,
        )
    }

    #[must_use]
    pub fn certified_reduction_basis_points(&self) -> Option<u64> {
        if self.eligible_pairs == 0 {
            return None;
        }
        Some(
            (u128::from(self.certified_pruned_pairs) * 10_000 / u128::from(self.eligible_pairs))
                as u64,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TimeQuantiles {
    pub p50_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
}

impl TimeQuantiles {
    #[must_use]
    pub fn valid(&self) -> bool {
        self.p50_ns > 0 && self.p50_ns <= self.p95_ns && self.p95_ns <= self.p99_ns
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Timing {
    pub total_step: TimeQuantiles,
    pub broadphase: TimeQuantiles,
    pub swept_bound_producer: TimeQuantiles,
    pub certificate_verifier: TimeQuantiles,
    pub narrowphase: TimeQuantiles,
    pub solver: TimeQuantiles,
}

impl Timing {
    #[must_use]
    pub fn valid(&self) -> bool {
        let total = self.total_step;
        let stages = [
            self.broadphase,
            self.swept_bound_producer,
            self.certificate_verifier,
            self.narrowphase,
            self.solver,
        ];
        total.valid()
            && stages.iter().all(TimeQuantiles::valid)
            && stages.iter().all(|stage| {
                stage.p50_ns <= total.p50_ns
                    && stage.p95_ns <= total.p95_ns
                    && stage.p99_ns <= total.p99_ns
            })
    }

    /// Verified-certificate p95 time as basis points of total-step p95.
    #[must_use]
    pub fn verifier_share_p95_basis_points(&self) -> Option<u64> {
        if self.total_step.p95_ns == 0 {
            return None;
        }
        Some(
            (u128::from(self.certificate_verifier.p95_ns) * 10_000
                / u128::from(self.total_step.p95_ns)) as u64,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkReceipt {
    pub schema_version: u32,
    pub benchmark_id: String,
    pub dataset: DatasetIdentity,
    pub algorithm: AlgorithmIdentity,
    pub motion_profile: MotionProfile,
    pub precision_profile: String,
    pub seed: u64,
    pub run_id: String,
    pub status: RunStatus,
    pub environment: Environment,
    pub pairs: PairAccounting,
    pub timing: Timing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptError {
    UnsupportedSchema,
    EmptyBenchmarkIdentity,
    InvalidDatasetIdentity,
    InvalidAlgorithmIdentity,
    EmptyRunIdentity,
    InvalidEnvironment,
    IncompleteExecution,
    OracleDenominatorMismatch,
    ForwardedDenominatorMismatch,
    PairConservationMismatch,
    MissingPruneCertificates,
    CertificateReplayMismatch,
    RejectedCertificateNotFallback,
    FallbackReasonMismatch,
    FallbackExceedsForwarded,
    FalseNegativePairs,
    InvalidTimingQuantiles,
    StageTimingExceedsTotal,
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ReceiptError {}

impl BenchmarkReceipt {
    /// Validate a completed result. Timeouts, OOM and aborts remain recordable
    /// but cannot enter a qualified leaderboard.
    pub fn validate(&self) -> Result<(), Vec<ReceiptError>> {
        let mut errors = Vec::new();
        if self.schema_version != 1 {
            errors.push(ReceiptError::UnsupportedSchema);
        }
        if self.benchmark_id.trim().is_empty() {
            errors.push(ReceiptError::EmptyBenchmarkIdentity);
        }
        if !self.dataset.valid() {
            errors.push(ReceiptError::InvalidDatasetIdentity);
        }
        if !self.algorithm.valid() {
            errors.push(ReceiptError::InvalidAlgorithmIdentity);
        }
        if self.precision_profile.trim().is_empty() || self.run_id.trim().is_empty() {
            errors.push(ReceiptError::EmptyRunIdentity);
        }
        if !self.environment.valid() {
            errors.push(ReceiptError::InvalidEnvironment);
        }
        if self.status != RunStatus::Completed {
            errors.push(ReceiptError::IncompleteExecution);
        }

        let pairs = self.pairs;
        if pairs
            .oracle_positive_pairs
            .checked_add(pairs.oracle_negative_pairs)
            != Some(pairs.eligible_pairs)
        {
            errors.push(ReceiptError::OracleDenominatorMismatch);
        }
        if pairs
            .forwarded_true_positives
            .checked_add(pairs.forwarded_false_positives)
            != Some(pairs.forwarded_pairs)
        {
            errors.push(ReceiptError::ForwardedDenominatorMismatch);
        }
        if pairs
            .forwarded_pairs
            .checked_add(pairs.certified_pruned_pairs)
            != Some(pairs.eligible_pairs)
        {
            errors.push(ReceiptError::PairConservationMismatch);
        }
        if pairs.accepted_certificates != pairs.certified_pruned_pairs {
            errors.push(ReceiptError::MissingPruneCertificates);
        }
        if pairs.replayed_certificates != pairs.accepted_certificates {
            errors.push(ReceiptError::CertificateReplayMismatch);
        }
        if pairs.rejected_certificate_attempts > pairs.invalid_fallbacks {
            errors.push(ReceiptError::RejectedCertificateNotFallback);
        }
        let fallback_reasons = pairs
            .unsupported_fallbacks
            .checked_add(pairs.invalid_fallbacks)
            .and_then(|value| value.checked_add(pairs.overflow_fallbacks));
        if fallback_reasons != Some(pairs.fallback_pairs) {
            errors.push(ReceiptError::FallbackReasonMismatch);
        }
        if pairs.fallback_pairs > pairs.forwarded_pairs {
            errors.push(ReceiptError::FallbackExceedsForwarded);
        }
        if pairs.false_negatives() != Some(0) {
            errors.push(ReceiptError::FalseNegativePairs);
        }

        if !self.timing.total_step.valid()
            || [
                self.timing.broadphase,
                self.timing.swept_bound_producer,
                self.timing.certificate_verifier,
                self.timing.narrowphase,
                self.timing.solver,
            ]
            .iter()
            .any(|value| !value.valid())
        {
            errors.push(ReceiptError::InvalidTimingQuantiles);
        } else if !self.timing.valid() {
            errors.push(ReceiptError::StageTimingExceedsTotal);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_receipt() -> BenchmarkReceipt {
        BenchmarkReceipt {
            schema_version: 1,
            benchmark_id: "ccd-linear-toi-v1".to_owned(),
            dataset: DatasetIdentity {
                name: "nyu-linearized-toi".to_owned(),
                version: "2024-03-31".to_owned(),
                source_url: "https://archive.nyu.edu/handle/2451/74508".to_owned(),
                sha256: "a".repeat(64),
            },
            algorithm: AlgorithmIdentity {
                name: "symtropy-certified-cull".to_owned(),
                version: "v0.1".to_owned(),
                source_commit: "b".repeat(40),
            },
            motion_profile: MotionProfile::LinearizedTrajectoryOracle,
            precision_profile: "f64".to_owned(),
            seed: 42,
            run_id: "run-0001".to_owned(),
            status: RunStatus::Completed,
            environment: Environment {
                operating_system: "linux".to_owned(),
                architecture: "x86_64".to_owned(),
                cpu_model: "declared-test-cpu".to_owned(),
                physical_cores: 8,
                worker_threads: 8,
                compiler: "rustc 1.96.0".to_owned(),
                build_profile: "release".to_owned(),
                gpu_model: None,
                gpu_driver: None,
            },
            pairs: PairAccounting {
                eligible_pairs: 100,
                oracle_positive_pairs: 10,
                oracle_negative_pairs: 90,
                forwarded_pairs: 40,
                forwarded_true_positives: 10,
                forwarded_false_positives: 30,
                certified_pruned_pairs: 60,
                accepted_certificates: 60,
                replayed_certificates: 60,
                rejected_certificate_attempts: 0,
                fallback_pairs: 4,
                unsupported_fallbacks: 1,
                invalid_fallbacks: 1,
                overflow_fallbacks: 2,
            },
            timing: Timing {
                total_step: TimeQuantiles {
                    p50_ns: 1_000_000,
                    p95_ns: 2_000_000,
                    p99_ns: 3_000_000,
                },
                broadphase: TimeQuantiles {
                    p50_ns: 100_000,
                    p95_ns: 200_000,
                    p99_ns: 300_000,
                },
                swept_bound_producer: TimeQuantiles {
                    p50_ns: 150_000,
                    p95_ns: 250_000,
                    p99_ns: 350_000,
                },
                certificate_verifier: TimeQuantiles {
                    p50_ns: 2_000,
                    p95_ns: 4_000,
                    p99_ns: 6_000,
                },
                narrowphase: TimeQuantiles {
                    p50_ns: 500_000,
                    p95_ns: 1_000_000,
                    p99_ns: 1_500_000,
                },
                solver: TimeQuantiles {
                    p50_ns: 200_000,
                    p95_ns: 500_000,
                    p99_ns: 700_000,
                },
            },
        }
    }

    #[test]
    fn accepts_a_conserved_complete_receipt() {
        assert_eq!(valid_receipt().validate(), Ok(()));
    }

    #[test]
    fn rejects_any_oracle_positive_false_negative() {
        let mut receipt = valid_receipt();
        receipt.pairs.forwarded_true_positives = 9;
        receipt.pairs.forwarded_false_positives = 31;
        assert!(
            receipt
                .validate()
                .unwrap_err()
                .contains(&ReceiptError::FalseNegativePairs)
        );
    }

    #[test]
    fn rejects_certified_prunes_without_accepted_and_replayed_certificates() {
        let mut receipt = valid_receipt();
        receipt.pairs.accepted_certificates = 59;
        receipt.pairs.replayed_certificates = 59;
        let errors = receipt.validate().unwrap_err();
        assert!(errors.contains(&ReceiptError::MissingPruneCertificates));
        assert!(errors.contains(&ReceiptError::CertificateReplayMismatch));
    }

    #[test]
    fn rejects_pair_disposition_count_mismatch() {
        let mut receipt = valid_receipt();
        receipt.pairs.certified_pruned_pairs = 59;
        assert!(
            receipt
                .validate()
                .unwrap_err()
                .contains(&ReceiptError::PairConservationMismatch)
        );
    }

    #[test]
    fn rejected_certificate_attempt_must_enter_invalid_fallback() {
        let mut receipt = valid_receipt();
        receipt.pairs.rejected_certificate_attempts = 2;
        assert!(
            receipt
                .validate()
                .unwrap_err()
                .contains(&ReceiptError::RejectedCertificateNotFallback)
        );
    }

    #[test]
    fn rejects_fallbacks_without_reason_codes() {
        let mut receipt = valid_receipt();
        receipt.pairs.overflow_fallbacks = 1;
        assert!(
            receipt
                .validate()
                .unwrap_err()
                .contains(&ReceiptError::FallbackReasonMismatch)
        );
    }

    #[test]
    fn timeouts_and_oom_cannot_qualify_as_completed_runs() {
        let mut receipt = valid_receipt();
        receipt.status = RunStatus::TimedOut;
        assert!(
            receipt
                .validate()
                .unwrap_err()
                .contains(&ReceiptError::IncompleteExecution)
        );
        receipt.status = RunStatus::OutOfMemory;
        assert!(
            receipt
                .validate()
                .unwrap_err()
                .contains(&ReceiptError::IncompleteExecution)
        );
    }

    #[test]
    fn validates_dataset_and_algorithm_digests() {
        let mut receipt = valid_receipt();
        receipt.dataset.sha256 = "not-a-digest".to_owned();
        receipt.algorithm.source_commit.clear();
        let errors = receipt.validate().unwrap_err();
        assert!(errors.contains(&ReceiptError::InvalidDatasetIdentity));
        assert!(errors.contains(&ReceiptError::InvalidAlgorithmIdentity));
    }

    #[test]
    fn rejects_nonmonotone_quantiles_and_stages_larger_than_total() {
        let mut receipt = valid_receipt();
        receipt.timing.total_step.p95_ns = 999;
        assert!(
            receipt
                .validate()
                .unwrap_err()
                .contains(&ReceiptError::InvalidTimingQuantiles)
        );
        receipt = valid_receipt();
        receipt.timing.solver.p95_ns = receipt.timing.total_step.p95_ns + 1;
        assert!(
            receipt
                .validate()
                .unwrap_err()
                .contains(&ReceiptError::StageTimingExceedsTotal)
        );
    }

    #[test]
    fn zero_denominators_are_unavailable_not_zero_rates() {
        let mut pairs = valid_receipt().pairs;
        pairs.oracle_negative_pairs = 0;
        assert_eq!(pairs.false_positive_rate_basis_points(), None);
        pairs.eligible_pairs = 0;
        assert_eq!(pairs.certified_reduction_basis_points(), None);
    }

    #[test]
    fn rejected_bad_certificate_can_be_safely_forwarded() {
        let mut receipt = valid_receipt();
        receipt.pairs.rejected_certificate_attempts = 1;
        receipt.pairs.invalid_fallbacks = 2;
        receipt.pairs.fallback_pairs = 5;
        assert_eq!(receipt.validate(), Ok(()));
    }
}
