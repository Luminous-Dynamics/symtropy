// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence ladder for common fungal / mycorrhizal network claims.
//!
//! The five CMN criteria are evaluated independently. Missing criteria never
//! count as passes. Hyphal continuity is not proof of continuous cytoplasmic
//! flow, so a separate flow-evidence field is required.
//!
//! This module qualifies experimental claim scope; it does not establish that
//! a biological mechanism is true beyond the linked source evidence.

use std::collections::BTreeSet;
use std::fmt;

use crate::observation_evidence::{
    EvidenceReceiptError, ObservationEvidenceId, ObservationEvidenceReceipt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FungalNetworkIdentity {
    ArbuscularMycorrhizal,
    OtherFilamentousFungus,
    MixedOrUnresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCriterion {
    AtLeastTwoPlantUnits,
    AllLinkedPlantsAmColonized,
    RootSystemsSpatiallySeparated,
    HyphalLinkingTreatment,
    HyphalContinuityTested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceOutcome {
    Pass,
    Fail,
    Unreported,
    NotApplicable,
}

/// One criterion result with an explicit source observation reference.
///
/// PASS and FAIL both require a linked receipt. UNREPORTED and NOT_APPLICABLE
/// must not carry a fabricated receipt. reason_code is a stable machine code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriterionEvidence {
    pub criterion: NetworkCriterion,
    pub outcome: EvidenceOutcome,
    pub observation: Option<ObservationEvidenceId>,
    pub reason_code: String,
}

impl CriterionEvidence {
    pub fn validate(&self) -> Result<(), NetworkEvidenceError> {
        let reason_empty = self.reason_code.trim().is_empty();
        match self.outcome {
            EvidenceOutcome::Pass | EvidenceOutcome::Fail => {
                if self.observation.is_none() {
                    return Err(NetworkEvidenceError::MissingReceipt(self.criterion));
                }
                if reason_empty {
                    return Err(NetworkEvidenceError::MissingReasonCode(self.criterion));
                }
            }
            EvidenceOutcome::Unreported => {
                if self.observation.is_some() {
                    return Err(NetworkEvidenceError::UnexpectedReceipt(self.criterion));
                }
                if reason_empty {
                    return Err(NetworkEvidenceError::MissingReasonCode(self.criterion));
                }
            }
            EvidenceOutcome::NotApplicable => {
                if self.observation.is_some() {
                    return Err(NetworkEvidenceError::UnexpectedReceipt(self.criterion));
                }
                if reason_empty {
                    return Err(NetworkEvidenceError::MissingReasonCode(self.criterion));
                }
            }
        }
        Ok(())
    }
}

/// Evidence that one fungal network links at least two plant units.
/// This is deliberately distinct from criterion 1 (the count of plant units).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedFungalLinkEvidence {
    pub outcome: EvidenceOutcome,
    pub observation: Option<ObservationEvidenceId>,
    pub reason_code: String,
}

impl SharedFungalLinkEvidence {
    pub fn validate(&self) -> Result<(), NetworkEvidenceError> {
        if self.reason_code.trim().is_empty() {
            return Err(NetworkEvidenceError::MissingSharedLinkReasonCode);
        }
        match self.outcome {
            EvidenceOutcome::Pass | EvidenceOutcome::Fail => {
                if self.observation.is_none() {
                    return Err(NetworkEvidenceError::MissingSharedLinkReceipt);
                }
            }
            EvidenceOutcome::Unreported | EvidenceOutcome::NotApplicable => {
                if self.observation.is_some() {
                    return Err(NetworkEvidenceError::UnexpectedSharedLinkReceipt);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkClaimLevel {
    NoNetworkClaimSupported,
    NetworkCandidateOnly,
    CommonFungalNetworkSupported,
    CommonMycorrhizalNetworkObserved,
    CommonMycorrhizalNetworkEffectSupported,
    CommonMycorrhizalNetworkHyphalContinuitySupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CytoplasmicFlowEvidence {
    NotEvaluated,
    NotSupported,
    CytoplasmicFlowStreamObserved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CytoplasmicFlowReceipt {
    pub outcome: CytoplasmicFlowEvidence,
    pub observation: Option<ObservationEvidenceId>,
    pub reason_code: String,
}

impl CytoplasmicFlowReceipt {
    pub fn validate(&self) -> Result<(), NetworkEvidenceError> {
        if self.reason_code.trim().is_empty() {
            return Err(NetworkEvidenceError::MissingFlowReasonCode);
        }
        match self.outcome {
            CytoplasmicFlowEvidence::CytoplasmicFlowStreamObserved
            | CytoplasmicFlowEvidence::NotSupported => {
                if self.observation.is_none() {
                    return Err(NetworkEvidenceError::MissingFlowReceipt);
                }
            }
            CytoplasmicFlowEvidence::NotEvaluated => {
                if self.observation.is_some() {
                    return Err(NetworkEvidenceError::UnexpectedFlowReceipt);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkEvidenceAssessment {
    pub fungal_identity: FungalNetworkIdentity,
    /// Evidence that the same fungal network links the plant units.
    pub shared_fungal_link: SharedFungalLinkEvidence,
    /// Fixed order: criteria 1 through 5 from the CMN evidence framework.
    pub criteria: [CriterionEvidence; 5],
    /// Independent of criterion 5: continuity testing alone does not prove flow.
    pub cytoplasmic_flow: CytoplasmicFlowReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkEvidenceClassification {
    pub claim_level: NetworkClaimLevel,
    pub cytoplasmic_flow: CytoplasmicFlowEvidence,
    pub criteria_1_to_4_pass: bool,
    pub all_five_criteria_pass: bool,
}

impl NetworkEvidenceAssessment {
    pub fn validate(&self) -> Result<(), NetworkEvidenceError> {
        self.shared_fungal_link.validate()?;
        self.cytoplasmic_flow.validate()?;
        for (index, criterion) in self.criteria.iter().enumerate() {
            let expected = match index {
                0 => NetworkCriterion::AtLeastTwoPlantUnits,
                1 => NetworkCriterion::AllLinkedPlantsAmColonized,
                2 => NetworkCriterion::RootSystemsSpatiallySeparated,
                3 => NetworkCriterion::HyphalLinkingTreatment,
                4 => NetworkCriterion::HyphalContinuityTested,
                _ => unreachable!("fixed five-element criterion array"),
            };
            if criterion.criterion != expected {
                return Err(NetworkEvidenceError::CriterionOrderMismatch {
                    index,
                    expected,
                    actual: criterion.criterion,
                });
            }
            criterion.validate()?;
        }
        Ok(())
    }

    /// Validate every referenced observation ID against the supplied receipt set.
    ///
    /// This prevents a syntactically valid but unresolved caller-provided ID
    /// from being accepted as source evidence. Each resolved receipt is itself
    /// validated before it can support a criterion or flow claim.
    pub fn validate_against_receipts(
        &self,
        receipts: &[ObservationEvidenceReceipt],
    ) -> Result<(), NetworkEvidenceError> {
        self.validate()?;

        let mut seen_receipt_ids = BTreeSet::new();
        for receipt in receipts {
            if !seen_receipt_ids.insert(receipt.id) {
                return Err(NetworkEvidenceError::DuplicateObservationReceipt(
                    receipt.id,
                ));
            }
        }

        let mut referenced = Vec::new();
        if let Some(id) = self.shared_fungal_link.observation {
            referenced.push(id);
        }
        for criterion in &self.criteria {
            if let Some(id) = criterion.observation {
                referenced.push(id);
            }
        }
        if let Some(id) = self.cytoplasmic_flow.observation {
            referenced.push(id);
        }

        for id in referenced {
            let receipt = receipts
                .iter()
                .find(|receipt| receipt.id == id)
                .ok_or(NetworkEvidenceError::MissingObservationReceipt(id))?;
            receipt
                .validate()
                .map_err(|source| NetworkEvidenceError::InvalidObservationReceipt { id, source })?;
        }

        if let Some(flow_id) = self.cytoplasmic_flow.observation {
            let flow_receipt = receipts
                .iter()
                .find(|receipt| receipt.id == flow_id)
                .expect("referenced flow receipt was resolved above");
            if self.cytoplasmic_flow.outcome
                == CytoplasmicFlowEvidence::CytoplasmicFlowStreamObserved
                && flow_receipt.subject
                    != crate::observation_evidence::ObservationSubject::CytoplasmicFlowStream
            {
                return Err(NetworkEvidenceError::FlowReceiptSubjectMismatch(flow_id));
            }
        }
        Ok(())
    }

    pub fn classify(&self) -> Result<NetworkEvidenceClassification, NetworkEvidenceError> {
        self.validate()?;

        let criterion = |item: NetworkCriterion| {
            self.criteria
                .iter()
                .find(|entry| entry.criterion == item)
                .map(|entry| entry.outcome)
                .unwrap_or(EvidenceOutcome::Unreported)
        };

        let link_is_supported = self.shared_fungal_link.outcome == EvidenceOutcome::Pass
            && criterion(NetworkCriterion::AtLeastTwoPlantUnits) == EvidenceOutcome::Pass;

        if self.cytoplasmic_flow.outcome == CytoplasmicFlowEvidence::CytoplasmicFlowStreamObserved
            && !link_is_supported
        {
            return Err(NetworkEvidenceError::FlowWithoutSharedNetwork);
        }

        let claim_level = if !link_is_supported {
            NetworkClaimLevel::NoNetworkClaimSupported
        } else {
            match self.fungal_identity {
                FungalNetworkIdentity::OtherFilamentousFungus => {
                    NetworkClaimLevel::CommonFungalNetworkSupported
                }
                FungalNetworkIdentity::MixedOrUnresolved => NetworkClaimLevel::NetworkCandidateOnly,
                FungalNetworkIdentity::ArbuscularMycorrhizal => {
                    let am_colonization = criterion(NetworkCriterion::AllLinkedPlantsAmColonized)
                        == EvidenceOutcome::Pass;
                    if !am_colonization {
                        NetworkClaimLevel::NetworkCandidateOnly
                    } else {
                        let first_four_pass = [
                            NetworkCriterion::AtLeastTwoPlantUnits,
                            NetworkCriterion::AllLinkedPlantsAmColonized,
                            NetworkCriterion::RootSystemsSpatiallySeparated,
                            NetworkCriterion::HyphalLinkingTreatment,
                        ]
                        .iter()
                        .all(|item| criterion(*item) == EvidenceOutcome::Pass);

                        if !first_four_pass {
                            NetworkClaimLevel::CommonMycorrhizalNetworkObserved
                        } else if criterion(NetworkCriterion::HyphalContinuityTested)
                            == EvidenceOutcome::Pass
                        {
                            NetworkClaimLevel::CommonMycorrhizalNetworkHyphalContinuitySupported
                        } else {
                            NetworkClaimLevel::CommonMycorrhizalNetworkEffectSupported
                        }
                    }
                }
            }
        };

        let criteria_1_to_4_pass = [
            NetworkCriterion::AtLeastTwoPlantUnits,
            NetworkCriterion::AllLinkedPlantsAmColonized,
            NetworkCriterion::RootSystemsSpatiallySeparated,
            NetworkCriterion::HyphalLinkingTreatment,
        ]
        .iter()
        .all(|item| criterion(*item) == EvidenceOutcome::Pass);

        let all_five_criteria_pass = criteria_1_to_4_pass
            && criterion(NetworkCriterion::HyphalContinuityTested) == EvidenceOutcome::Pass;

        Ok(NetworkEvidenceClassification {
            claim_level,
            cytoplasmic_flow: self.cytoplasmic_flow.outcome,
            criteria_1_to_4_pass,
            all_five_criteria_pass,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkEvidenceError {
    MissingReceipt(NetworkCriterion),
    UnexpectedReceipt(NetworkCriterion),
    MissingReasonCode(NetworkCriterion),
    CriterionOrderMismatch {
        index: usize,
        expected: NetworkCriterion,
        actual: NetworkCriterion,
    },
    MissingSharedLinkReceipt,
    UnexpectedSharedLinkReceipt,
    MissingObservationReceipt(ObservationEvidenceId),
    DuplicateObservationReceipt(ObservationEvidenceId),
    InvalidObservationReceipt {
        id: ObservationEvidenceId,
        source: EvidenceReceiptError,
    },
    MissingSharedLinkReasonCode,
    MissingFlowReceipt,
    FlowReceiptSubjectMismatch(ObservationEvidenceId),
    FlowWithoutSharedNetwork,
    UnexpectedFlowReceipt,
    MissingFlowReasonCode,
}

impl fmt::Display for NetworkEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingReceipt(criterion) => {
                write!(
                    f,
                    "criterion {:?} requires a source-linked receipt",
                    criterion
                )
            }
            Self::UnexpectedReceipt(criterion) => {
                write!(
                    f,
                    "criterion {:?} must not carry a receipt for this status",
                    criterion
                )
            }
            Self::MissingReasonCode(criterion) => {
                write!(f, "criterion {:?} requires a stable reason code", criterion)
            }
            Self::CriterionOrderMismatch {
                index,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "criterion slot {} expected {:?}, found {:?}",
                    index, expected, actual
                )
            }
            Self::MissingSharedLinkReceipt => {
                write!(f, "shared fungal link requires a source receipt")
            }
            Self::MissingObservationReceipt(id) => {
                write!(
                    f,
                    "observation {:?} does not resolve to a source receipt",
                    id
                )
            }
            Self::DuplicateObservationReceipt(id) => {
                write!(f, "source receipt ID {:?} is duplicated", id)
            }
            Self::InvalidObservationReceipt { id, source } => {
                write!(
                    f,
                    "observation {:?} has an invalid source receipt: {}",
                    id, source
                )
            }
            Self::UnexpectedSharedLinkReceipt => {
                write!(f, "unreported shared fungal link must not carry a receipt")
            }
            Self::MissingSharedLinkReasonCode => {
                write!(f, "shared fungal link requires a stable reason code")
            }
            Self::MissingFlowReceipt => {
                write!(
                    f,
                    "cytoplasmic-flow stream result requires a source receipt"
                )
            }
            Self::FlowReceiptSubjectMismatch(id) => {
                write!(
                    f,
                    "flow observation {:?} is not typed as a cytoplasmic flow stream",
                    id
                )
            }
            Self::FlowWithoutSharedNetwork => write!(
                f,
                "cytoplasmic-flow observation requires a supported shared fungal link"
            ),
            Self::UnexpectedFlowReceipt => {
                write!(f, "unevaluated flow result must not carry a receipt")
            }
            Self::MissingFlowReasonCode => write!(f, "flow result requires a stable reason code"),
        }
    }
}

impl std::error::Error for NetworkEvidenceError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(criterion: NetworkCriterion, outcome: EvidenceOutcome) -> CriterionEvidence {
        CriterionEvidence {
            criterion,
            outcome,
            observation: match outcome {
                EvidenceOutcome::Pass | EvidenceOutcome::Fail => {
                    Some(ObservationEvidenceId(criterion as u64 + 1))
                }
                EvidenceOutcome::Unreported | EvidenceOutcome::NotApplicable => None,
            },
            reason_code: "fixture_evidence".to_owned(),
        }
    }

    fn amf_assessment(outcomes: [EvidenceOutcome; 5]) -> NetworkEvidenceAssessment {
        NetworkEvidenceAssessment {
            fungal_identity: FungalNetworkIdentity::ArbuscularMycorrhizal,
            shared_fungal_link: SharedFungalLinkEvidence {
                outcome: EvidenceOutcome::Pass,
                observation: Some(ObservationEvidenceId(90)),
                reason_code: "shared_genet_link_observed".to_owned(),
            },
            criteria: [
                item(NetworkCriterion::AtLeastTwoPlantUnits, outcomes[0]),
                item(NetworkCriterion::AllLinkedPlantsAmColonized, outcomes[1]),
                item(NetworkCriterion::RootSystemsSpatiallySeparated, outcomes[2]),
                item(NetworkCriterion::HyphalLinkingTreatment, outcomes[3]),
                item(NetworkCriterion::HyphalContinuityTested, outcomes[4]),
            ],
            cytoplasmic_flow: CytoplasmicFlowReceipt {
                outcome: CytoplasmicFlowEvidence::NotEvaluated,
                observation: None,
                reason_code: "not_measured".to_owned(),
            },
        }
    }

    #[test]
    fn four_criteria_support_cmn_effect_but_not_hyphal_continuity() {
        let assessment = amf_assessment([
            EvidenceOutcome::Pass,
            EvidenceOutcome::Pass,
            EvidenceOutcome::Pass,
            EvidenceOutcome::Pass,
            EvidenceOutcome::Unreported,
        ]);
        let result = assessment.classify().unwrap();
        assert_eq!(
            result.claim_level,
            NetworkClaimLevel::CommonMycorrhizalNetworkEffectSupported
        );
        assert!(result.criteria_1_to_4_pass);
        assert!(!result.all_five_criteria_pass);
        assert_eq!(
            result.cytoplasmic_flow,
            CytoplasmicFlowEvidence::NotEvaluated
        );
    }

    #[test]
    fn all_five_criteria_support_cmn_hc_but_not_automatic_flow_claim() {
        let assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        let result = assessment.classify().unwrap();
        assert_eq!(
            result.claim_level,
            NetworkClaimLevel::CommonMycorrhizalNetworkHyphalContinuitySupported
        );
        assert!(result.all_five_criteria_pass);
        assert_eq!(
            result.cytoplasmic_flow,
            CytoplasmicFlowEvidence::NotEvaluated
        );
    }

    #[test]
    fn root_or_hyphal_linking_control_failure_prevents_causal_cmn_attribution() {
        let assessment = amf_assessment([
            EvidenceOutcome::Pass,
            EvidenceOutcome::Pass,
            EvidenceOutcome::Fail,
            EvidenceOutcome::Pass,
            EvidenceOutcome::Pass,
        ]);
        let result = assessment.classify().unwrap();
        assert_eq!(
            result.claim_level,
            NetworkClaimLevel::CommonMycorrhizalNetworkObserved
        );
        assert!(!result.criteria_1_to_4_pass);
    }

    #[test]
    fn unreported_am_colonization_never_promotes_to_cmn() {
        let assessment = amf_assessment([
            EvidenceOutcome::Pass,
            EvidenceOutcome::Unreported,
            EvidenceOutcome::Pass,
            EvidenceOutcome::Pass,
            EvidenceOutcome::Pass,
        ]);
        assert_eq!(
            assessment.classify().unwrap().claim_level,
            NetworkClaimLevel::NetworkCandidateOnly
        );
    }

    #[test]
    fn other_filamentous_fungus_is_classified_as_cfn_not_cmn() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.fungal_identity = FungalNetworkIdentity::OtherFilamentousFungus;
        assessment.criteria[1] = item(
            NetworkCriterion::AllLinkedPlantsAmColonized,
            EvidenceOutcome::NotApplicable,
        );
        assert_eq!(
            assessment.classify().unwrap().claim_level,
            NetworkClaimLevel::CommonFungalNetworkSupported
        );
    }

    #[test]
    fn mixed_or_unresolved_identity_is_only_a_network_candidate() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.fungal_identity = FungalNetworkIdentity::MixedOrUnresolved;
        assert_eq!(
            assessment.classify().unwrap().claim_level,
            NetworkClaimLevel::NetworkCandidateOnly
        );
    }

    #[test]
    fn hyphal_continuity_does_not_imply_plant_to_plant_cytoplasmic_flow() {
        let assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        let result = assessment.classify().unwrap();
        assert_eq!(
            result.claim_level,
            NetworkClaimLevel::CommonMycorrhizalNetworkHyphalContinuitySupported
        );
        assert_eq!(
            result.cytoplasmic_flow,
            CytoplasmicFlowEvidence::NotEvaluated
        );
    }

    #[test]
    fn observed_flow_stream_is_retained_as_a_distinct_axis() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.cytoplasmic_flow = CytoplasmicFlowReceipt {
            outcome: CytoplasmicFlowEvidence::CytoplasmicFlowStreamObserved,
            observation: Some(ObservationEvidenceId(101)),
            reason_code: "cytoplasmic_stream_observed_on_segment".to_owned(),
        };
        let result = assessment.classify().unwrap();
        assert_eq!(
            result.cytoplasmic_flow,
            CytoplasmicFlowEvidence::CytoplasmicFlowStreamObserved
        );
    }

    #[test]
    fn flow_stream_cannot_be_claimed_without_a_shared_fungal_link() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.shared_fungal_link.outcome = EvidenceOutcome::Fail;
        assessment.cytoplasmic_flow = CytoplasmicFlowReceipt {
            outcome: CytoplasmicFlowEvidence::CytoplasmicFlowStreamObserved,
            observation: Some(ObservationEvidenceId(101)),
            reason_code: "flow_measurement_without_link".to_owned(),
        };
        assert_eq!(
            assessment.classify(),
            Err(NetworkEvidenceError::FlowWithoutSharedNetwork)
        );
    }

    fn receipt(id: u64) -> ObservationEvidenceReceipt {
        ObservationEvidenceReceipt {
            id: ObservationEvidenceId(id),
            source_file_sha256: crate::observation_evidence::Sha256Digest([1; 32]),
            frame: crate::observation_evidence::ImageFrameReference {
                frame: crate::observation_evidence::ImageFrameIndex(0),
                slice: None,
                timestamp_s: 0.0,
            },
            roi: crate::observation_evidence::ImageRoiBoundsPx {
                id: crate::observation_evidence::ObservationRoiId(id),
                x_min: 0,
                y_min: 0,
                width_px: 10,
                height_px: 10,
            },
            calibration: crate::observation_evidence::PhysicalSamplingCalibration {
                pixel_width_um: 0.25,
                pixel_height_um: 0.25,
                voxel_depth_um: None,
                frame_interval_s: None,
            },
            processor_name: "fixture".to_owned(),
            processor_revision: "v1".to_owned(),
            processor_parameters_sha256: crate::observation_evidence::Sha256Digest([2; 32]),
            subject: crate::observation_evidence::ObservationSubject::HyphalSegmentTrace,
            graph_target: None,
            graph_event: None,
            uncertainty: None,
            disposition: crate::observation_evidence::EvidenceDisposition::MachineGenerated,
        }
    }

    #[test]
    fn all_criterion_receipts_must_resolve_and_validate() {
        let assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        let receipts = vec![
            receipt(90),
            receipt(1),
            receipt(2),
            receipt(3),
            receipt(4),
            receipt(5),
        ];
        assert_eq!(assessment.validate_against_receipts(&receipts), Ok(()));
    }

    #[test]
    fn flow_receipt_must_be_typed_as_flow_stream() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.cytoplasmic_flow = CytoplasmicFlowReceipt {
            outcome: CytoplasmicFlowEvidence::CytoplasmicFlowStreamObserved,
            observation: Some(ObservationEvidenceId(101)),
            reason_code: "local_flow_stream_observation".to_owned(),
        };
        let mut receipts = vec![
            receipt(90),
            receipt(1),
            receipt(2),
            receipt(3),
            receipt(4),
            receipt(5),
            receipt(101),
        ];
        assert_eq!(
            assessment.validate_against_receipts(&receipts),
            Err(NetworkEvidenceError::FlowReceiptSubjectMismatch(
                ObservationEvidenceId(101)
            ))
        );
        receipts.last_mut().unwrap().subject =
            crate::observation_evidence::ObservationSubject::CytoplasmicFlowStream;
        assert_eq!(assessment.validate_against_receipts(&receipts), Ok(()));
    }

    #[test]
    fn duplicate_source_receipt_ids_are_rejected() {
        let assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        let receipts = vec![
            receipt(90),
            receipt(1),
            receipt(2),
            receipt(3),
            receipt(4),
            receipt(5),
            receipt(5),
        ];
        assert_eq!(
            assessment.validate_against_receipts(&receipts),
            Err(NetworkEvidenceError::DuplicateObservationReceipt(
                ObservationEvidenceId(5)
            ))
        );
    }

    #[test]
    fn invalid_source_receipt_is_rejected() {
        let assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        let mut invalid = receipt(3);
        invalid.source_file_sha256 = crate::observation_evidence::Sha256Digest([0; 32]);
        let receipts = vec![
            receipt(90),
            receipt(1),
            receipt(2),
            invalid,
            receipt(4),
            receipt(5),
        ];
        assert!(matches!(
            assessment.validate_against_receipts(&receipts),
            Err(NetworkEvidenceError::InvalidObservationReceipt {
                id: ObservationEvidenceId(3),
                source: EvidenceReceiptError::InvalidDigest
            })
        ));
    }

    #[test]
    fn unresolved_criterion_receipt_id_is_rejected() {
        let assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        // Include receipts 1-3 so the first unresolved identity is 4.
        let receipts = vec![receipt(90), receipt(1), receipt(2), receipt(3), receipt(5)];
        assert_eq!(
            assessment.validate_against_receipts(&receipts),
            Err(NetworkEvidenceError::MissingObservationReceipt(
                ObservationEvidenceId(4)
            ))
        );
    }

    #[test]
    fn shared_link_pass_without_source_receipt_is_rejected() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.shared_fungal_link.observation = None;
        assert_eq!(
            assessment.classify(),
            Err(NetworkEvidenceError::MissingSharedLinkReceipt)
        );
    }

    #[test]
    fn pass_or_fail_without_receipt_is_rejected() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.criteria[2].observation = None;
        assert_eq!(
            assessment.classify(),
            Err(NetworkEvidenceError::MissingReceipt(
                NetworkCriterion::RootSystemsSpatiallySeparated
            ))
        );
    }

    #[test]
    fn unreported_criterion_with_receipt_is_rejected() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.criteria[4].outcome = EvidenceOutcome::Unreported;
        assert_eq!(
            assessment.classify(),
            Err(NetworkEvidenceError::UnexpectedReceipt(
                NetworkCriterion::HyphalContinuityTested
            ))
        );
    }

    #[test]
    fn criterion_order_is_fixed_and_validated() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.criteria.swap(0, 1);
        assert!(matches!(
            assessment.classify(),
            Err(NetworkEvidenceError::CriterionOrderMismatch { index: 0, .. })
        ));
    }

    #[test]
    fn shared_link_and_two_plant_requirement_are_both_required() {
        let mut assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.shared_fungal_link.outcome = EvidenceOutcome::Fail;
        assessment.shared_fungal_link.observation = Some(ObservationEvidenceId(90));
        assert_eq!(
            assessment.classify().unwrap().claim_level,
            NetworkClaimLevel::NoNetworkClaimSupported
        );

        assessment = amf_assessment([EvidenceOutcome::Pass; 5]);
        assessment.criteria[0] = item(
            NetworkCriterion::AtLeastTwoPlantUnits,
            EvidenceOutcome::Fail,
        );
        assert_eq!(
            assessment.classify().unwrap().claim_level,
            NetworkClaimLevel::NoNetworkClaimSupported
        );
    }
}
