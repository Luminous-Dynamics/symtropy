//! Two-stage tactile characterization: sensor object versus sensorized body.
//!
//! This module prevents a benchtop characterization record from silently
//! qualifying a mounted/embodied sensor whose host, mounting, geometry,
//! compliance, preload or interface has changed. Embodiment transfer requires
//! exact context bindings and independently referenced validation evidence.
//!
//! Receipt identifiers are references, not cryptographic verification. This
//! module does not validate signatures, metrological traceability, reference
//! independence, uncertainty, or physical efficacy. It is a typed admission
//! contract, not a calibration registry or hardware authority.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

pub const REQUIRED_CHARACTERIZATION_TESTS: [CharacterizationTestFamily; 9] = [
    CharacterizationTestFamily::QuasistaticIndentation,
    CharacterizationTestFamily::Creep,
    CharacterizationTestFamily::StressRelaxation,
    CharacterizationTestFamily::MultirateLoading,
    CharacterizationTestFamily::FrequencySweep,
    CharacterizationTestFamily::MultiaxisCharacterization,
    CharacterizationTestFamily::EnvironmentalSensitivity,
    CharacterizationTestFamily::Drift,
    CharacterizationTestFamily::Fatigue,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum CharacterizationStage {
    SensorObject,
    SensorizedBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum CharacterizationTestFamily {
    QuasistaticIndentation,
    Creep,
    StressRelaxation,
    MultirateLoading,
    FrequencySweep,
    MultiaxisCharacterization,
    EnvironmentalSensitivity,
    Drift,
    Fatigue,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SensorObjectIdentity {
    pub sensor_article_id: String,
    pub electronics_revision: String,
    pub firmware_revision: String,
    pub skin_article_id: String,
    pub skin_revision: String,
    pub material_lot_id: String,
    pub transducer_profile_digest: String,
}

impl SensorObjectIdentity {
    fn validate(&self) -> Vec<String> {
        let fields = [
            ("sensor_article_id", self.sensor_article_id.as_str()),
            ("electronics_revision", self.electronics_revision.as_str()),
            ("firmware_revision", self.firmware_revision.as_str()),
            ("skin_article_id", self.skin_article_id.as_str()),
            ("skin_revision", self.skin_revision.as_str()),
            ("material_lot_id", self.material_lot_id.as_str()),
            (
                "transducer_profile_digest",
                self.transducer_profile_digest.as_str(),
            ),
        ];
        fields
            .into_iter()
            .filter(|(_, value)| value.trim().is_empty())
            .map(|(name, _)| format!("sensor identity field {name} is missing"))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SensorObjectContext {
    pub sensor: SensorObjectIdentity,
    /// Rigid/controlled Stage-1 test fixture identity.
    pub fixture_article_id: String,
    pub fixture_revision: String,
    pub fixture_geometry_digest: String,
    pub boundary_condition_digest: String,
    /// Canonical digest over the complete Stage-1 context, including fixture,
    /// sensor object, preconditioning and the declared test configuration.
    pub context_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SensorizedBodyContext {
    /// The exact sensor object characterized in Stage 1.
    pub sensor: SensorObjectIdentity,
    pub host_article_id: String,
    pub host_revision: String,
    pub mounting_revision: String,
    pub mount_interface_digest: String,
    pub host_geometry_digest: String,
    pub host_compliance_profile_digest: String,
    pub preload_profile_digest: String,
    /// Canonical digest over the complete Stage-2 sensorized-body context.
    pub context_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CharacterizationContext {
    SensorObject(SensorObjectContext),
    SensorizedBody(SensorizedBodyContext),
}

impl CharacterizationContext {
    pub fn stage(&self) -> CharacterizationStage {
        match self {
            Self::SensorObject(_) => CharacterizationStage::SensorObject,
            Self::SensorizedBody(_) => CharacterizationStage::SensorizedBody,
        }
    }

    pub fn context_digest(&self) -> &str {
        match self {
            Self::SensorObject(context) => &context.context_digest,
            Self::SensorizedBody(context) => &context.context_digest,
        }
    }

    fn sensor_identity(&self) -> &SensorObjectIdentity {
        match self {
            Self::SensorObject(context) => &context.sensor,
            Self::SensorizedBody(context) => &context.sensor,
        }
    }

    fn validate(&self) -> Vec<String> {
        let mut errors = self.sensor_identity().validate();
        match self {
            Self::SensorObject(context) => {
                for (name, value) in [
                    ("fixture_article_id", context.fixture_article_id.as_str()),
                    ("fixture_revision", context.fixture_revision.as_str()),
                    (
                        "fixture_geometry_digest",
                        context.fixture_geometry_digest.as_str(),
                    ),
                    (
                        "boundary_condition_digest",
                        context.boundary_condition_digest.as_str(),
                    ),
                    ("context_digest", context.context_digest.as_str()),
                ] {
                    if value.trim().is_empty() {
                        errors.push(format!("Stage-1 context field {name} is missing"));
                    }
                }
            }
            Self::SensorizedBody(context) => {
                for (name, value) in [
                    ("host_article_id", context.host_article_id.as_str()),
                    ("host_revision", context.host_revision.as_str()),
                    ("mounting_revision", context.mounting_revision.as_str()),
                    (
                        "mount_interface_digest",
                        context.mount_interface_digest.as_str(),
                    ),
                    (
                        "host_geometry_digest",
                        context.host_geometry_digest.as_str(),
                    ),
                    (
                        "host_compliance_profile_digest",
                        context.host_compliance_profile_digest.as_str(),
                    ),
                    (
                        "preload_profile_digest",
                        context.preload_profile_digest.as_str(),
                    ),
                    ("context_digest", context.context_digest.as_str()),
                ] {
                    if value.trim().is_empty() {
                        errors.push(format!("Stage-2 context field {name} is missing"));
                    }
                }
            }
        }
        errors
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TestDisposition {
    Pass,
    Fail,
    NotApplicable,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CharacterizationTestResult {
    pub family: CharacterizationTestFamily,
    pub disposition: TestDisposition,
    /// Required for Pass/Fail. Identifies the immutable raw synchronized data.
    pub raw_observation_digest: String,
    /// Required for Pass/Fail. Identifies the test result/artifact.
    pub result_digest: String,
    /// Required for Pass/Fail. Receipt for the reference used in this test.
    pub reference_receipt_id: String,
    /// Required for NotApplicable and ignored otherwise.
    pub not_applicable_rationale: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CharacterizationRecord {
    pub record_id: String,
    pub stage: CharacterizationStage,
    pub context_digest: String,
    pub protocol_revision: String,
    pub raw_campaign_digest: String,
    pub reference_chain_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub tests: Vec<CharacterizationTestResult>,
}

impl CharacterizationRecord {
    pub fn validate(&self, context: &CharacterizationContext) -> Vec<String> {
        let mut errors = context.validate();
        if self.record_id.trim().is_empty() {
            errors.push("characterization record ID is missing".to_owned());
        }
        if self.stage != context.stage() {
            errors.push("record stage does not match physical context stage".to_owned());
        }
        if self.context_digest.trim().is_empty() || self.context_digest != context.context_digest()
        {
            errors.push("record context digest does not match its physical context".to_owned());
        }
        for (name, value) in [
            ("protocol_revision", self.protocol_revision.as_str()),
            ("raw_campaign_digest", self.raw_campaign_digest.as_str()),
            (
                "reference_chain_receipt_id",
                self.reference_chain_receipt_id.as_str(),
            ),
            (
                "reference_ancestry_receipt_id",
                self.reference_ancestry_receipt_id.as_str(),
            ),
            (
                "common_cause_analysis_receipt_id",
                self.common_cause_analysis_receipt_id.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("characterization record field {name} is missing"));
            }
        }

        let mut seen = BTreeSet::new();
        for test in &self.tests {
            if !seen.insert(test.family) {
                errors.push(format!("duplicate characterization test {:?}", test.family));
            }
            match test.disposition {
                TestDisposition::Pass | TestDisposition::Fail => {
                    for (name, value) in [
                        (
                            "raw_observation_digest",
                            test.raw_observation_digest.as_str(),
                        ),
                        ("result_digest", test.result_digest.as_str()),
                        ("reference_receipt_id", test.reference_receipt_id.as_str()),
                    ] {
                        if value.trim().is_empty() {
                            errors.push(format!("test {:?} is missing {name}", test.family));
                        }
                    }
                    if test.disposition == TestDisposition::Fail {
                        errors.push(format!("test {:?} failed", test.family));
                    }
                }
                TestDisposition::NotApplicable => {
                    if test.not_applicable_rationale.trim().is_empty() {
                        errors.push(format!(
                            "test {:?} is NotApplicable without a rationale",
                            test.family
                        ));
                    }
                }
            }
        }

        for family in REQUIRED_CHARACTERIZATION_TESTS {
            if !seen.contains(&family) {
                errors.push(format!(
                    "required characterization test {:?} is missing",
                    family
                ));
            }
        }
        errors
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EmbodimentTransferReceipt {
    pub receipt_id: String,
    pub source_record_id: String,
    pub source_context_digest: String,
    pub target_record_id: String,
    pub target_context_digest: String,
    pub transfer_model_digest: String,
    pub independent_reference_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub heldout_validation_receipt_id: String,
}

impl EmbodimentTransferReceipt {
    fn validate(&self) -> Vec<String> {
        let fields = [
            ("receipt_id", self.receipt_id.as_str()),
            ("source_record_id", self.source_record_id.as_str()),
            ("source_context_digest", self.source_context_digest.as_str()),
            ("target_record_id", self.target_record_id.as_str()),
            ("target_context_digest", self.target_context_digest.as_str()),
            ("transfer_model_digest", self.transfer_model_digest.as_str()),
            (
                "independent_reference_receipt_id",
                self.independent_reference_receipt_id.as_str(),
            ),
            (
                "common_cause_analysis_receipt_id",
                self.common_cause_analysis_receipt_id.as_str(),
            ),
            (
                "heldout_validation_receipt_id",
                self.heldout_validation_receipt_id.as_str(),
            ),
        ];
        fields
            .into_iter()
            .filter(|(_, value)| value.trim().is_empty())
            .map(|(name, _)| format!("embodiment transfer field {name} is missing"))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum EmbodimentDisposition {
    QualifiedForSensorizedBody,
    SensorObjectBaselineOnly,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EmbodimentAssessment {
    pub disposition: EmbodimentDisposition,
    pub reasons: Vec<String>,
}

/// Qualify a Stage-1 sensor-object characterization for an exact Stage-2
/// sensorized-body context. This is deliberately fail-closed: matching sensor
/// identity does not imply matching host, mount, geometry, preload or system
/// transfer function.
pub fn assess_embodiment_transfer(
    source_context: &CharacterizationContext,
    source_record: &CharacterizationRecord,
    target_context: &CharacterizationContext,
    target_record: &CharacterizationRecord,
    receipt: &EmbodimentTransferReceipt,
) -> EmbodimentAssessment {
    let mut reasons = Vec::new();

    if source_context.stage() != CharacterizationStage::SensorObject {
        reasons.push("source context is not a Stage-1 sensor object".to_owned());
    }
    if target_context.stage() != CharacterizationStage::SensorizedBody {
        reasons.push("target context is not a Stage-2 sensorized body".to_owned());
    }
    if source_context.sensor_identity() != target_context.sensor_identity() {
        reasons.push("Stage-1 and Stage-2 sensor object identities differ".to_owned());
    }

    reasons.extend(
        source_record
            .validate(source_context)
            .into_iter()
            .map(|reason| format!("source record: {reason}")),
    );
    reasons.extend(
        target_record
            .validate(target_context)
            .into_iter()
            .map(|reason| format!("target record: {reason}")),
    );
    reasons.extend(receipt.validate());

    if receipt.source_record_id != source_record.record_id {
        reasons.push("transfer receipt source record ID mismatch".to_owned());
    }
    if receipt.source_context_digest != source_context.context_digest() {
        reasons.push("transfer receipt source context digest mismatch".to_owned());
    }
    if receipt.target_record_id != target_record.record_id {
        reasons.push("transfer receipt target record ID mismatch".to_owned());
    }
    if receipt.target_context_digest != target_context.context_digest() {
        reasons.push("transfer receipt target context digest mismatch".to_owned());
    }

    let disposition = if reasons.is_empty() {
        EmbodimentDisposition::QualifiedForSensorizedBody
    } else if source_context.stage() == CharacterizationStage::SensorObject
        && source_record.validate(source_context).is_empty()
    {
        EmbodimentDisposition::SensorObjectBaselineOnly
    } else {
        EmbodimentDisposition::Rejected
    };

    EmbodimentAssessment {
        disposition,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sensor_identity() -> SensorObjectIdentity {
        SensorObjectIdentity {
            sensor_article_id: "sensor-article-01".to_owned(),
            electronics_revision: "pcb-r2".to_owned(),
            firmware_revision: "fw-r3".to_owned(),
            skin_article_id: "skin-article-01".to_owned(),
            skin_revision: "skin-r4".to_owned(),
            material_lot_id: "elastomer-lot-17".to_owned(),
            transducer_profile_digest: "sha256:transducer-profile-01".to_owned(),
        }
    }

    fn source_context() -> CharacterizationContext {
        CharacterizationContext::SensorObject(SensorObjectContext {
            sensor: sensor_identity(),
            fixture_article_id: "metrology-fixture-01".to_owned(),
            fixture_revision: "fixture-r1".to_owned(),
            fixture_geometry_digest: "sha256:fixture-geometry".to_owned(),
            boundary_condition_digest: "sha256:stage1-boundary".to_owned(),
            context_digest: "sha256:stage1-context".to_owned(),
        })
    }

    fn target_context() -> CharacterizationContext {
        CharacterizationContext::SensorizedBody(SensorizedBodyContext {
            sensor: sensor_identity(),
            host_article_id: "robot-finger-01".to_owned(),
            host_revision: "finger-r7".to_owned(),
            mounting_revision: "mount-r2".to_owned(),
            mount_interface_digest: "sha256:mount-interface".to_owned(),
            host_geometry_digest: "sha256:host-geometry".to_owned(),
            host_compliance_profile_digest: "sha256:host-compliance".to_owned(),
            preload_profile_digest: "sha256:preload".to_owned(),
            context_digest: "sha256:stage2-context".to_owned(),
        })
    }

    fn complete_tests() -> Vec<CharacterizationTestResult> {
        REQUIRED_CHARACTERIZATION_TESTS
            .into_iter()
            .map(|family| CharacterizationTestResult {
                family,
                disposition: TestDisposition::Pass,
                raw_observation_digest: format!("sha256:raw-{family:?}"),
                result_digest: format!("sha256:result-{family:?}"),
                reference_receipt_id: format!("receipt:reference-{family:?}"),
                not_applicable_rationale: String::new(),
            })
            .collect()
    }

    fn source_record(context: &CharacterizationContext) -> CharacterizationRecord {
        CharacterizationRecord {
            record_id: "characterization-stage1-01".to_owned(),
            stage: CharacterizationStage::SensorObject,
            context_digest: context.context_digest().to_owned(),
            protocol_revision: "phri-tac-characterization-v0.1".to_owned(),
            raw_campaign_digest: "sha256:stage1-raw-campaign".to_owned(),
            reference_chain_receipt_id: "receipt:stage1-reference-chain".to_owned(),
            reference_ancestry_receipt_id: "receipt:stage1-reference-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:stage1-common-cause".to_owned(),
            tests: complete_tests(),
        }
    }

    fn target_record(context: &CharacterizationContext) -> CharacterizationRecord {
        CharacterizationRecord {
            record_id: "characterization-stage2-01".to_owned(),
            stage: CharacterizationStage::SensorizedBody,
            context_digest: context.context_digest().to_owned(),
            protocol_revision: "phri-tac-characterization-v0.1".to_owned(),
            raw_campaign_digest: "sha256:stage2-raw-campaign".to_owned(),
            reference_chain_receipt_id: "receipt:stage2-reference-chain".to_owned(),
            reference_ancestry_receipt_id: "receipt:stage2-reference-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:stage2-common-cause".to_owned(),
            tests: complete_tests(),
        }
    }

    fn valid_receipt() -> EmbodimentTransferReceipt {
        EmbodimentTransferReceipt {
            receipt_id: "receipt:embodiment-transfer-01".to_owned(),
            source_record_id: "characterization-stage1-01".to_owned(),
            source_context_digest: "sha256:stage1-context".to_owned(),
            target_record_id: "characterization-stage2-01".to_owned(),
            target_context_digest: "sha256:stage2-context".to_owned(),
            transfer_model_digest: "sha256:embodiment-transfer-model".to_owned(),
            independent_reference_receipt_id: "receipt:independent-stage2-reference".to_owned(),
            common_cause_analysis_receipt_id: "receipt:transfer-common-cause".to_owned(),
            heldout_validation_receipt_id: "receipt:heldout-stage2-validation".to_owned(),
        }
    }

    #[test]
    fn exact_stage1_to_stage2_transfer_requires_matching_contexts_and_evidence() {
        let source_context = source_context();
        let target_context = target_context();
        let source = source_record(&source_context);
        let target = target_record(&target_context);
        let assessment = assess_embodiment_transfer(
            &source_context,
            &source,
            &target_context,
            &target,
            &valid_receipt(),
        );
        assert_eq!(
            assessment.disposition,
            EmbodimentDisposition::QualifiedForSensorizedBody
        );
        assert!(assessment.reasons.is_empty());
    }

    #[test]
    fn stage1_characterization_alone_never_qualifies_a_sensorized_body() {
        let source_context = source_context();
        let source = source_record(&source_context);
        let target_context = target_context();
        let mut target = target_record(&target_context);
        target.raw_campaign_digest.clear();
        let assessment = assess_embodiment_transfer(
            &source_context,
            &source,
            &target_context,
            &target,
            &valid_receipt(),
        );
        assert_eq!(
            assessment.disposition,
            EmbodimentDisposition::SensorObjectBaselineOnly
        );
        assert!(!assessment.reasons.is_empty());
    }

    #[test]
    fn changed_host_or_mounting_context_invalidates_old_transfer_receipt() {
        let source_context = source_context();
        let source = source_record(&source_context);
        let mut target_context = match target_context() {
            CharacterizationContext::SensorizedBody(context) => context,
            CharacterizationContext::SensorObject(_) => unreachable!(),
        };
        target_context.mounting_revision = "mount-r3".to_owned();
        target_context.context_digest = "sha256:changed-stage2-context".to_owned();
        let target_context = CharacterizationContext::SensorizedBody(target_context);
        let target = target_record(&target_context);
        let assessment = assess_embodiment_transfer(
            &source_context,
            &source,
            &target_context,
            &target,
            &valid_receipt(),
        );
        assert_eq!(
            assessment.disposition,
            EmbodimentDisposition::SensorObjectBaselineOnly
        );
        assert!(
            assessment
                .reasons
                .iter()
                .any(|reason| reason.contains("target context digest mismatch"))
        );
    }

    #[test]
    fn changing_the_sensor_object_between_stages_is_rejected() {
        let source_context = source_context();
        let source = source_record(&source_context);
        let mut target = match target_context() {
            CharacterizationContext::SensorizedBody(context) => context,
            CharacterizationContext::SensorObject(_) => unreachable!(),
        };
        target.sensor.skin_revision = "skin-r5".to_owned();
        target.context_digest = "sha256:changed-sensorized-body".to_owned();
        let target_context = CharacterizationContext::SensorizedBody(target);
        let target_record = target_record(&target_context);
        let assessment = assess_embodiment_transfer(
            &source_context,
            &source,
            &target_context,
            &target_record,
            &valid_receipt(),
        );
        assert_eq!(
            assessment.disposition,
            EmbodimentDisposition::SensorObjectBaselineOnly
        );
        assert!(
            assessment
                .reasons
                .iter()
                .any(|reason| reason.contains("sensor object identities differ"))
        );
    }

    #[test]
    fn missing_or_duplicate_test_families_fail_characterization() {
        let context = source_context();
        let mut record = source_record(&context);
        record.tests.pop();
        assert!(!record.validate(&context).is_empty());

        let mut duplicate = source_record(&context);
        duplicate.tests.push(duplicate.tests[0].clone());
        assert!(
            duplicate
                .validate(&context)
                .iter()
                .any(|reason| reason.contains("duplicate characterization test"))
        );
    }

    #[test]
    fn failed_test_or_not_applicable_without_rationale_blocks_transfer() {
        let source_context = source_context();
        let target_context = target_context();
        let mut source = source_record(&source_context);
        source.tests[0].disposition = TestDisposition::Fail;
        let assessment = assess_embodiment_transfer(
            &source_context,
            &source,
            &target_context,
            &target_record(&target_context),
            &valid_receipt(),
        );
        assert_eq!(assessment.disposition, EmbodimentDisposition::Rejected);

        let mut not_applicable = source_record(&source_context);
        not_applicable.tests[0].disposition = TestDisposition::NotApplicable;
        not_applicable.tests[0].raw_observation_digest.clear();
        not_applicable.tests[0].result_digest.clear();
        not_applicable.tests[0].reference_receipt_id.clear();
        let errors = not_applicable.validate(&source_context);
        assert!(
            errors
                .iter()
                .any(|reason| reason.contains("NotApplicable without a rationale"))
        );
    }

    #[test]
    fn justified_not_applicable_test_is_explicitly_accepted() {
        let context = source_context();
        let mut record = source_record(&context);
        record.tests[0].disposition = TestDisposition::NotApplicable;
        record.tests[0].raw_observation_digest.clear();
        record.tests[0].result_digest.clear();
        record.tests[0].reference_receipt_id.clear();
        record.tests[0].not_applicable_rationale =
            "The test stimulus is not physically meaningful for this modality.".to_owned();
        assert!(record.validate(&context).is_empty());
    }

    #[test]
    fn missing_reference_ancestry_or_common_cause_evidence_blocks_transfer() {
        let source_context = source_context();
        let target_context = target_context();
        let mut source = source_record(&source_context);
        source.reference_ancestry_receipt_id.clear();
        let assessment = assess_embodiment_transfer(
            &source_context,
            &source,
            &target_context,
            &target_record(&target_context),
            &valid_receipt(),
        );
        assert_eq!(assessment.disposition, EmbodimentDisposition::Rejected);
    }

    #[test]
    fn receipt_must_bind_exact_source_and_target_records() {
        let source_context = source_context();
        let target_context = target_context();
        let source = source_record(&source_context);
        let target = target_record(&target_context);
        let mut receipt = valid_receipt();
        receipt.target_record_id = "different-record".to_owned();
        let assessment = assess_embodiment_transfer(
            &source_context,
            &source,
            &target_context,
            &target,
            &receipt,
        );
        assert_eq!(
            assessment.disposition,
            EmbodimentDisposition::SensorObjectBaselineOnly
        );
        assert!(
            assessment
                .reasons
                .iter()
                .any(|reason| reason.contains("target record ID mismatch"))
        );
    }
}
