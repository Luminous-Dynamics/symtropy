//! Source-bound, deterministically encoded replay records for robust capsule
//! interval verification.
//!
//! This module records caller-supplied provenance and re-runs the existing
//! independent interval algorithm. It does not authenticate evidence
//! references or independently prove the verifier implementation.

use crate::{
    Capsule, CapsuleRelativeUncertainty, IntervalVerificationReceipt,
    IntervalVerificationStatus, IntervalVerifierConfig, IntervalVerifierError,
    NormalizedTimeWindow, Vec3, verify_capsule_interval_no_contact_with_uncertainty,
};

const CANONICAL_DOMAIN: &[u8] = b"LUMINOUS-ROBUST-CAPSULE-INTERVAL-CERTIFICATE\0";
const CANONICAL_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RobustCertificateError {
    EmptyField(&'static str),
    InvalidDigest(&'static str),
    Interval(IntervalVerifierError),
    ReceiptMismatch,
    CanonicalBytesMismatch,
}

/// Caller-supplied evidence and context identities for one robust query.
///
/// Digest validation checks lowercase SHA-256 shape only. It does not hash,
/// fetch, authenticate or assess the evidence referred to by these fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustIntervalBinding {
    pub site_epoch: String,
    pub target_frame_id: String,
    pub design_revision_sha256: String,
    pub moving_model_sha256: String,
    pub obstacle_model_sha256: String,
    pub uncertainty_profile_sha256: String,
    pub clock_domain_id: String,
    pub calibration_ref: String,
    pub localization_tracking_ref: String,
    pub controller_profile_ref: String,
    pub origin_relative_evidence_ref: String,
    pub relative_translation_evidence_ref: String,
    pub moving_axis_evidence_ref: String,
    pub obstacle_axis_evidence_ref: String,
}

impl RobustIntervalBinding {
    pub fn validate(&self) -> Result<(), RobustCertificateError> {
        for (field, value) in [
            ("site_epoch", self.site_epoch.as_str()),
            ("target_frame_id", self.target_frame_id.as_str()),
            ("clock_domain_id", self.clock_domain_id.as_str()),
            ("calibration_ref", self.calibration_ref.as_str()),
            ("localization_tracking_ref", self.localization_tracking_ref.as_str()),
            ("controller_profile_ref", self.controller_profile_ref.as_str()),
            ("origin_relative_evidence_ref", self.origin_relative_evidence_ref.as_str()),
            (
                "relative_translation_evidence_ref",
                self.relative_translation_evidence_ref.as_str(),
            ),
            ("moving_axis_evidence_ref", self.moving_axis_evidence_ref.as_str()),
            ("obstacle_axis_evidence_ref", self.obstacle_axis_evidence_ref.as_str()),
        ] {
            require_non_empty(field, value)?;
        }

        for (field, digest) in [
            ("design_revision_sha256", self.design_revision_sha256.as_str()),
            ("moving_model_sha256", self.moving_model_sha256.as_str()),
            ("obstacle_model_sha256", self.obstacle_model_sha256.as_str()),
            ("uncertainty_profile_sha256", self.uncertainty_profile_sha256.as_str()),
        ] {
            if !is_sha256(digest) {
                return Err(RobustCertificateError::InvalidDigest(field));
            }
        }
        Ok(())
    }
}

/// Full input snapshot, interval result, and deterministic canonical bytes.
///
/// The canonical representation preserves exact f64 bit patterns, including
/// signed zero. Consumers should pass the bytes to the established digest /
/// signature owner; this crate deliberately does not provide authentication.
#[derive(Clone, Debug, PartialEq)]
pub struct RobustIntervalVerificationCertificate {
    pub binding: RobustIntervalBinding,
    pub moving: Capsule,
    pub moving_translation: Vec3,
    pub obstacle: Capsule,
    pub obstacle_translation: Vec3,
    pub uncertainty: CapsuleRelativeUncertainty,
    pub window: NormalizedTimeWindow,
    pub config: IntervalVerifierConfig,
    pub receipt: IntervalVerificationReceipt,
    pub canonical_bytes: Vec<u8>,
}

pub fn create_robust_interval_certificate(
    binding: RobustIntervalBinding,
    moving: Capsule,
    moving_translation: Vec3,
    obstacle: Capsule,
    obstacle_translation: Vec3,
    uncertainty: CapsuleRelativeUncertainty,
    window: NormalizedTimeWindow,
    config: IntervalVerifierConfig,
) -> Result<RobustIntervalVerificationCertificate, RobustCertificateError> {
    binding.validate()?;
    let receipt = verify_capsule_interval_no_contact_with_uncertainty(
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
        uncertainty,
        window,
        config,
    )
    .map_err(RobustCertificateError::Interval)?;

    let mut certificate = RobustIntervalVerificationCertificate {
        binding,
        moving,
        moving_translation,
        obstacle,
        obstacle_translation,
        uncertainty,
        window,
        config,
        receipt,
        canonical_bytes: Vec::new(),
    };
    certificate.canonical_bytes = certificate.encode_canonical();
    Ok(certificate)
}

/// Recompute the interval result and the canonical input/receipt encoding.
/// Tampering with any exposed field, stored receipt or canonical byte vector
/// is rejected. This is same-algorithm replay, not independent proof checking.
pub fn replay_robust_interval_certificate(
    certificate: &RobustIntervalVerificationCertificate,
) -> Result<IntervalVerificationReceipt, RobustCertificateError> {
    certificate.binding.validate()?;
    let recomputed = verify_capsule_interval_no_contact_with_uncertainty(
        certificate.moving,
        certificate.moving_translation,
        certificate.obstacle,
        certificate.obstacle_translation,
        certificate.uncertainty,
        certificate.window,
        certificate.config,
    )
    .map_err(RobustCertificateError::Interval)?;

    if recomputed != certificate.receipt {
        return Err(RobustCertificateError::ReceiptMismatch);
    }
    if certificate.canonical_bytes != certificate.encode_canonical() {
        return Err(RobustCertificateError::CanonicalBytesMismatch);
    }
    Ok(recomputed)
}

impl RobustIntervalVerificationCertificate {
    /// Rebuild the canonical representation from the current public fields.
    pub fn encode_canonical(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(768);
        output.extend_from_slice(CANONICAL_DOMAIN);
        output.extend_from_slice(&CANONICAL_VERSION.to_be_bytes());

        for value in [
            self.binding.site_epoch.as_str(),
            self.binding.target_frame_id.as_str(),
            self.binding.design_revision_sha256.as_str(),
            self.binding.moving_model_sha256.as_str(),
            self.binding.obstacle_model_sha256.as_str(),
            self.binding.uncertainty_profile_sha256.as_str(),
            self.binding.clock_domain_id.as_str(),
            self.binding.calibration_ref.as_str(),
            self.binding.localization_tracking_ref.as_str(),
            self.binding.controller_profile_ref.as_str(),
            self.binding.origin_relative_evidence_ref.as_str(),
            self.binding.relative_translation_evidence_ref.as_str(),
            self.binding.moving_axis_evidence_ref.as_str(),
            self.binding.obstacle_axis_evidence_ref.as_str(),
        ] {
            push_string(&mut output, value);
        }

        push_capsule(&mut output, self.moving);
        push_vec3(&mut output, self.moving_translation);
        push_capsule(&mut output, self.obstacle);
        push_vec3(&mut output, self.obstacle_translation);
        push_uncertainty(&mut output, self.uncertainty);

        push_f64(&mut output, self.window.start_parameter);
        push_f64(&mut output, self.window.end_parameter);
        output.extend_from_slice(&self.config.maximum_nodes.to_be_bytes());
        output.push(self.config.maximum_depth);

        push_receipt(&mut output, self.receipt);
        output
    }
}

fn push_string(output: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    output.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    output.extend_from_slice(bytes);
}

fn push_f64(output: &mut Vec<u8>, value: f64) {
    output.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn push_vec3(output: &mut Vec<u8>, value: Vec3) {
    push_f64(output, value.x);
    push_f64(output, value.y);
    push_f64(output, value.z);
}

fn push_capsule(output: &mut Vec<u8>, capsule: Capsule) {
    push_vec3(output, capsule.start);
    push_vec3(output, capsule.end);
    push_f64(output, capsule.radius_mm);
}

fn push_uncertainty(output: &mut Vec<u8>, uncertainty: CapsuleRelativeUncertainty) {
    push_vec3(output, uncertainty.origin_relative_mm);
    push_vec3(output, uncertainty.relative_translation_mm);
    push_vec3(output, uncertainty.moving_axis_mm);
    push_vec3(output, uncertainty.obstacle_axis_mm);
}

fn push_receipt(output: &mut Vec<u8>, receipt: IntervalVerificationReceipt) {
    push_f64(output, receipt.window.start_parameter);
    push_f64(output, receipt.window.end_parameter);
    output.extend_from_slice(&receipt.config.maximum_nodes.to_be_bytes());
    output.push(receipt.config.maximum_depth);
    output.push(match receipt.status {
        IntervalVerificationStatus::Clear => 0,
        IntervalVerificationStatus::ContactWitness => 1,
        IntervalVerificationStatus::Unresolved => 2,
    });
    output.extend_from_slice(&receipt.parameter_boxes_visited.to_be_bytes());
    output.extend_from_slice(&receipt.parameter_boxes_pruned.to_be_bytes());
    output.push(receipt.maximum_depth_reached);
    match receipt.witness {
        None => output.push(0),
        Some(witness) => {
            output.push(1);
            push_f64(output, witness.time_parameter);
            push_f64(output, witness.moving_axis_parameter);
            push_f64(output, witness.obstacle_axis_parameter);
            push_f64(output, witness.distance_squared_upper_mm2);
            push_f64(output, witness.contact_squared_lower_mm2);
        }
    }
}

fn require_non_empty(
    field: &'static str,
    value: &str,
) -> Result<(), RobustCertificateError> {
    if value.trim().is_empty() {
        Err(RobustCertificateError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn capsule(x: f64, radius: f64) -> Capsule {
        Capsule::new(p(x, 0.0, 0.0), p(x, 0.0, 0.0), radius).unwrap()
    }

    fn binding() -> RobustIntervalBinding {
        RobustIntervalBinding {
            site_epoch: "site-epoch-17".into(),
            target_frame_id: "site-frame".into(),
            design_revision_sha256: "a".repeat(64),
            moving_model_sha256: "b".repeat(64),
            obstacle_model_sha256: "c".repeat(64),
            uncertainty_profile_sha256: "d".repeat(64),
            clock_domain_id: "site-monotonic-clock".into(),
            calibration_ref: "evidence://calibration".into(),
            localization_tracking_ref: "evidence://localization-tracking".into(),
            controller_profile_ref: "evidence://controller-profile".into(),
            origin_relative_evidence_ref: "evidence://origin-bound".into(),
            relative_translation_evidence_ref: "evidence://translation-bound".into(),
            moving_axis_evidence_ref: "evidence://moving-axis-bound".into(),
            obstacle_axis_evidence_ref: "evidence://obstacle-axis-bound".into(),
        }
    }

    fn create() -> RobustIntervalVerificationCertificate {
        create_robust_interval_certificate(
            binding(),
            capsule(0.0, 0.5),
            p(0.0, 0.0, 0.0),
            capsule(5.0, 0.5),
            p(0.0, 0.0, 0.0),
            CapsuleRelativeUncertainty::zero(),
            NormalizedTimeWindow {
                start_parameter: 0.0,
                end_parameter: 1.0,
            },
            IntervalVerifierConfig::default(),
        )
        .unwrap()
    }

    #[test]
    fn source_bound_certificate_replays_exactly() {
        let certificate = create();
        assert_eq!(
            replay_robust_interval_certificate(&certificate).unwrap(),
            certificate.receipt
        );
    }

    #[test]
    fn canonical_bytes_are_deterministic() {
        let first = create();
        let second = create();
        assert_eq!(first.canonical_bytes, second.canonical_bytes);
        assert_eq!(first.canonical_bytes, first.encode_canonical());
    }

    #[test]
    fn canonical_encoding_preserves_signed_zero_bits() {
        let positive_zero = create_robust_interval_certificate(
            binding(),
            capsule(0.0, 0.5),
            p(0.0, 0.0, 0.0),
            capsule(5.0, 0.5),
            p(0.0, 0.0, 0.0),
            CapsuleRelativeUncertainty::zero(),
            NormalizedTimeWindow {
                start_parameter: 0.0,
                end_parameter: 1.0,
            },
            IntervalVerifierConfig::default(),
        )
        .unwrap();
        let negative_zero = create_robust_interval_certificate(
            binding(),
            capsule(0.0, 0.5),
            p(-0.0, 0.0, 0.0),
            capsule(5.0, 0.5),
            p(0.0, 0.0, 0.0),
            CapsuleRelativeUncertainty::zero(),
            NormalizedTimeWindow {
                start_parameter: 0.0,
                end_parameter: 1.0,
            },
            IntervalVerifierConfig::default(),
        )
        .unwrap();

        assert_ne!(positive_zero.canonical_bytes, negative_zero.canonical_bytes);
        assert_eq!(positive_zero.receipt, negative_zero.receipt);
    }

    #[test]
    fn rejects_empty_identity_and_bad_digest_shape() {
        let mut empty = binding();
        empty.site_epoch.clear();
        assert_eq!(
            empty.validate(),
            Err(RobustCertificateError::EmptyField("site_epoch"))
        );

        let mut malformed = binding();
        malformed.moving_model_sha256 = "NOT-A-DIGEST".into();
        assert_eq!(
            malformed.validate(),
            Err(RobustCertificateError::InvalidDigest(
                "moving_model_sha256"
            ))
        );
    }

    #[test]
    fn tampering_with_geometry_or_uncertainty_is_rejected() {
        let mut geometry_tamper = create();
        geometry_tamper.obstacle.start.x += 0.25;
        assert!(matches!(
            replay_robust_interval_certificate(&geometry_tamper),
            Err(
                RobustCertificateError::ReceiptMismatch
                    | RobustCertificateError::CanonicalBytesMismatch
            )
        ));

        let mut uncertainty_tamper = create();
        uncertainty_tamper.uncertainty.origin_relative_mm.x = 0.1;
        assert!(matches!(
            replay_robust_interval_certificate(&uncertainty_tamper),
            Err(
                RobustCertificateError::ReceiptMismatch
                    | RobustCertificateError::CanonicalBytesMismatch
            )
        ));
    }

    #[test]
    fn tampering_with_receipt_or_canonical_bytes_is_rejected() {
        let mut receipt_tamper = create();
        receipt_tamper.receipt.parameter_boxes_visited += 1;
        assert_eq!(
            replay_robust_interval_certificate(&receipt_tamper),
            Err(RobustCertificateError::ReceiptMismatch)
        );

        let mut bytes_tamper = create();
        bytes_tamper.canonical_bytes[0] ^= 0x01;
        assert_eq!(
            replay_robust_interval_certificate(&bytes_tamper),
            Err(RobustCertificateError::CanonicalBytesMismatch)
        );
    }

    #[test]
    fn unresolved_receipts_replay_without_becoming_clear() {
        let mut certificate = create();
        certificate.config.maximum_nodes = 1;
        certificate.config.maximum_depth = 0;
        certificate.uncertainty = CapsuleRelativeUncertainty {
            origin_relative_mm: p(4.0, 0.0, 0.0),
            ..CapsuleRelativeUncertainty::zero()
        };
        certificate.receipt = verify_capsule_interval_no_contact_with_uncertainty(
            certificate.moving,
            certificate.moving_translation,
            certificate.obstacle,
            certificate.obstacle_translation,
            certificate.uncertainty,
            certificate.window,
            certificate.config,
        )
        .unwrap();
        certificate.canonical_bytes = certificate.encode_canonical();

        let replayed = replay_robust_interval_certificate(&certificate).unwrap();
        assert_eq!(replayed.status, IntervalVerificationStatus::Unresolved);
    }
}
