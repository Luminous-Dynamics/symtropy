//! Fail-closed admission gates for the GAUGE rigid-body physical dataset.
//!
//! This reference checks whether required provenance and qualification
//! evidence is present. It does not inspect or certify that evidence, fetch
//! the dataset, or execute a physics benchmark.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionGate {
    ImmutableRevision,
    ReviewedLicenseScope,
    ArtifactDigestCoverage,
    TrialSchemaReviewed,
    CoordinateFramesAndUnitsReviewed,
    CalibrationUncertaintyReviewed,
    RepeatAndSplitStructureReviewed,
    ProvenanceAndIntegrityReviewed,
    DataUseAuthorized,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionEvidence {
    pub immutable_revision: Option<String>,
    pub license_expression: Option<String>,
    pub license_scope_reviewed: bool,
    pub artifact_sha256: Vec<String>,
    pub trial_schema_reviewed: bool,
    pub coordinate_frames_and_units_reviewed: bool,
    pub calibration_uncertainty_reviewed: bool,
    pub repeat_and_split_structure_reviewed: bool,
    pub provenance_and_integrity_reviewed: bool,
    pub data_use_authorized: bool,
}

impl AdmissionEvidence {
    /// Return every missing or malformed evidence gate.
    pub fn blockers(&self) -> Vec<AdmissionGate> {
        let mut blockers = Vec::new();

        if !self
            .immutable_revision
            .as_deref()
            .is_some_and(is_full_git_revision)
        {
            blockers.push(AdmissionGate::ImmutableRevision);
        }
        if self
            .license_expression
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
            || !self.license_scope_reviewed
        {
            blockers.push(AdmissionGate::ReviewedLicenseScope);
        }
        if self.artifact_sha256.is_empty()
            || self.artifact_sha256.iter().any(|digest| !is_sha256(digest))
        {
            blockers.push(AdmissionGate::ArtifactDigestCoverage);
        }
        if !self.trial_schema_reviewed {
            blockers.push(AdmissionGate::TrialSchemaReviewed);
        }
        if !self.coordinate_frames_and_units_reviewed {
            blockers.push(AdmissionGate::CoordinateFramesAndUnitsReviewed);
        }
        if !self.calibration_uncertainty_reviewed {
            blockers.push(AdmissionGate::CalibrationUncertaintyReviewed);
        }
        if !self.repeat_and_split_structure_reviewed {
            blockers.push(AdmissionGate::RepeatAndSplitStructureReviewed);
        }
        if !self.provenance_and_integrity_reviewed {
            blockers.push(AdmissionGate::ProvenanceAndIntegrityReviewed);
        }
        if !self.data_use_authorized {
            blockers.push(AdmissionGate::DataUseAuthorized);
        }

        blockers
    }

    /// Whether the supplied record is complete enough for human review.
    /// This does not itself authorize use of the dataset.
    pub fn is_eligible_for_review(&self) -> bool {
        self.blockers().is_empty()
    }

    /// Reject an externally declared admission if any evidence gate is open.
    pub fn validate_admitted_claim(
        &self,
        declared_admitted: bool,
    ) -> Result<(), Vec<AdmissionGate>> {
        let blockers = self.blockers();
        if declared_admitted && !blockers.is_empty() {
            return Err(blockers);
        }
        Ok(())
    }
}

fn is_full_git_revision(value: &str) -> bool {
    value.len() == 40 && is_lower_hex(value)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && is_lower_hex(value)
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_evidence() -> AdmissionEvidence {
        AdmissionEvidence {
            immutable_revision: Some("a".repeat(40)),
            license_expression: Some("MIT".to_owned()),
            license_scope_reviewed: true,
            artifact_sha256: vec!["b".repeat(64)],
            trial_schema_reviewed: true,
            coordinate_frames_and_units_reviewed: true,
            calibration_uncertainty_reviewed: true,
            repeat_and_split_structure_reviewed: true,
            provenance_and_integrity_reviewed: true,
            data_use_authorized: true,
        }
    }

    fn observed_gauge_dataset_evidence() -> AdmissionEvidence {
        AdmissionEvidence {
            immutable_revision: None,
            license_expression: Some("MIT".to_owned()),
            license_scope_reviewed: false,
            artifact_sha256: Vec::new(),
            trial_schema_reviewed: false,
            coordinate_frames_and_units_reviewed: false,
            calibration_uncertainty_reviewed: false,
            repeat_and_split_structure_reviewed: false,
            provenance_and_integrity_reviewed: false,
            data_use_authorized: false,
        }
    }

    #[test]
    fn current_observed_dataset_state_is_not_admissible() {
        let evidence = observed_gauge_dataset_evidence();
        assert!(!evidence.is_eligible_for_review());
        assert_eq!(
            evidence.blockers(),
            vec![
                AdmissionGate::ImmutableRevision,
                AdmissionGate::ReviewedLicenseScope,
                AdmissionGate::ArtifactDigestCoverage,
                AdmissionGate::TrialSchemaReviewed,
                AdmissionGate::CoordinateFramesAndUnitsReviewed,
                AdmissionGate::CalibrationUncertaintyReviewed,
                AdmissionGate::RepeatAndSplitStructureReviewed,
                AdmissionGate::ProvenanceAndIntegrityReviewed,
                AdmissionGate::DataUseAuthorized,
            ]
        );
    }

    #[test]
    fn abbreviated_hub_revision_cannot_satisfy_immutable_pin() {
        let mut evidence = complete_evidence();
        evidence.immutable_revision = Some("f95bb06".to_owned());
        assert!(
            evidence
                .blockers()
                .contains(&AdmissionGate::ImmutableRevision)
        );
        assert!(!evidence.is_eligible_for_review());
    }

    #[test]
    fn malformed_artifact_digest_blocks_admission() {
        let mut evidence = complete_evidence();
        evidence.artifact_sha256 = vec!["not-a-sha256".to_owned()];
        assert!(
            evidence
                .blockers()
                .contains(&AdmissionGate::ArtifactDigestCoverage)
        );
    }

    #[test]
    fn missing_license_scope_review_blocks_even_with_mit_label() {
        let mut evidence = complete_evidence();
        evidence.license_scope_reviewed = false;
        assert!(
            evidence
                .blockers()
                .contains(&AdmissionGate::ReviewedLicenseScope)
        );
    }

    #[test]
    fn each_data_quality_gate_is_independent() {
        let mut evidence = complete_evidence();
        evidence.coordinate_frames_and_units_reviewed = false;
        assert_eq!(
            evidence.blockers(),
            vec![AdmissionGate::CoordinateFramesAndUnitsReviewed]
        );

        evidence = complete_evidence();
        evidence.repeat_and_split_structure_reviewed = false;
        assert_eq!(
            evidence.blockers(),
            vec![AdmissionGate::RepeatAndSplitStructureReviewed]
        );
    }

    #[test]
    fn complete_evidence_is_eligible_but_not_automatically_authorized() {
        let evidence = complete_evidence();
        assert!(evidence.is_eligible_for_review());
        assert!(evidence.validate_admitted_claim(false).is_ok());
    }

    #[test]
    fn declared_admission_with_open_gate_is_rejected() {
        let evidence = observed_gauge_dataset_evidence();
        let blockers = evidence.validate_admitted_claim(true).unwrap_err();
        assert_eq!(blockers.len(), 9);
    }

    #[test]
    fn blocked_manifest_is_not_promoted_by_software_validation() {
        let evidence = observed_gauge_dataset_evidence();
        assert!(evidence.validate_admitted_claim(false).is_ok());
        assert!(!evidence.is_eligible_for_review());
    }
}
