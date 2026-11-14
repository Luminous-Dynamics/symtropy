//! Industrial Evidence Matrix profile bound to a benchmark run manifest.
use serde::{Deserialize, Serialize};

#[derive(Clone,Copy,Debug,Deserialize,Serialize,PartialEq,Eq)]
pub enum PhysicalIndependenceLevel {
    #[serde(rename="NOT_EVALUATED")] NotEvaluated,
    #[serde(rename="A0")] A0, #[serde(rename="A1")] A1,
    #[serde(rename="A2")] A2, #[serde(rename="A3")] A3, #[serde(rename="A4")] A4,
}
impl PhysicalIndependenceLevel {
    fn supports_score(self)->bool { matches!(self,Self::A1|Self::A2|Self::A3|Self::A4) }
}
#[derive(Clone,Copy,Debug,Deserialize,Serialize,PartialEq,Eq)]
pub enum DomainShiftLevel {
    #[serde(rename="NOT_EVALUATED")] NotEvaluated,
    #[serde(rename="B0")] B0, #[serde(rename="B1")] B1,
    #[serde(rename="B2")] B2, #[serde(rename="B3")] B3, #[serde(rename="B4")] B4,
}
#[derive(Clone,Copy,Debug,Deserialize,Serialize,PartialEq,Eq)]
pub enum TargetDomainAccess {
    #[serde(rename="NOT_EVALUATED")] NotEvaluated,
    #[serde(rename="NONE")] None,
    #[serde(rename="UNLABELED_ADAPTATION")] UnlabeledAdaptation,
    #[serde(rename="LABELED_ADAPTATION")] LabeledAdaptation,
    #[serde(rename="SELF_SUPERVISED_PRETRAINING")] SelfSupervisedPretraining,
    #[serde(rename="UNCLEAR")] Unclear,
}
#[derive(Clone,Copy,Debug,Deserialize,Serialize,PartialEq,Eq)]
pub enum LeakageRisk {
    #[serde(rename="NOT_EVALUATED")] NotEvaluated,
    #[serde(rename="LOW")] Low, #[serde(rename="SOME_CONCERNS")] SomeConcerns,
    #[serde(rename="HIGH")] High, #[serde(rename="UNCLEAR")] Unclear,
}
#[derive(Clone,Copy,Debug,Deserialize,Serialize,PartialEq,Eq)]
pub enum ProfileStatus {
    #[serde(rename="NOT_EVALUATED")] NotEvaluated,
    #[serde(rename="STRUCTURALLY_ASSESSED")] StructurallyAssessed,
    #[serde(rename="INDEPENDENTLY_REVIEWED")] IndependentlyReviewed,
}
#[derive(Clone,Copy,Debug,Deserialize,Serialize,PartialEq,Eq)]
pub enum PartitionUnit {
    #[serde(rename="NOT_EVALUATED")] NotEvaluated,
    #[serde(rename="WINDOW")] Window, #[serde(rename="RECORDING")] Recording,
    #[serde(rename="COMPONENT")] Component, #[serde(rename="MACHINE")] Machine,
    #[serde(rename="SITE")] Site,
}
#[derive(Clone,Copy,Debug,Deserialize,Serialize,PartialEq,Eq)]
pub enum IndependentUnitKind {
    #[serde(rename="NOT_EVALUATED")] NotEvaluated,
    #[serde(rename="WINDOW")] Window, #[serde(rename="RECORDING")] Recording,
    #[serde(rename="ENGINE_RUN")] EngineRun, #[serde(rename="COMPONENT")] Component,
    #[serde(rename="MACHINE")] Machine, #[serde(rename="SITE")] Site, #[serde(rename="FLEET")] Fleet,
}
#[derive(Clone,Copy,Debug,Deserialize,Serialize,PartialEq,Eq)]
pub enum ReviewerScope {
    #[serde(rename="NOT_REVIEWED")] NotReviewed,
    #[serde(rename="STRUCTURAL_METADATA_ONLY")] StructuralMetadataOnly,
    #[serde(rename="INDEPENDENT_PHYSICAL_AUDIT")] IndependentPhysicalAudit,
}
#[derive(Clone,Debug,Deserialize,Serialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct ValidationEvidenceProfile {
    pub profile_schema_version:String,
    pub profile_receipt_sha256:Option<String>,
    pub profile_status:ProfileStatus,
    pub physical_independence_level:PhysicalIndependenceLevel,
    pub domain_shift_level:DomainShiftLevel,
    pub partition_unit:PartitionUnit,
    pub independent_unit_kind:IndependentUnitKind,
    pub independent_unit_count:u64,
    pub target_domain_access:TargetDomainAccess,
    pub leakage_risk:LeakageRisk,
    pub physical_identity_verified:bool,
    pub source_artifact_payloads_verified:bool,
    pub reviewer_scope:ReviewerScope,
    pub known_limitations:Vec<String>,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ValidationEvidenceError {
    SchemaVersionMismatch, InvalidNotEvaluatedProfile, InvalidStructuralProfile,
    InvalidIndependentReview, ScoreClaimWithoutIndependentEvidence, PhysicalTransferClaimForbidden,
}
impl ValidationEvidenceProfile {
    fn digest_valid(&self)->bool {
        self.profile_receipt_sha256.as_ref().is_some_and(|s|s.len()==64&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)))
    }
    /// Validate declared fields. The caller must still verify receipt bytes,
    /// cryptographic signatures and reviewer authority before publication.
    pub fn validate_for_claim(&self,run_status:&str,evidence_level:&str,score_claim_allowed:bool,physical_transfer_claim_allowed:bool)->Result<(),ValidationEvidenceError>{
        if self.profile_schema_version!="industrial-evidence-matrix-v0.1"{return Err(ValidationEvidenceError::SchemaVersionMismatch);}
        if physical_transfer_claim_allowed{return Err(ValidationEvidenceError::PhysicalTransferClaimForbidden);}
        match self.profile_status {
            ProfileStatus::NotEvaluated=>{
                if self.profile_receipt_sha256.is_some()
                    ||self.physical_independence_level!=PhysicalIndependenceLevel::NotEvaluated
                    ||self.domain_shift_level!=DomainShiftLevel::NotEvaluated
                    ||self.partition_unit!=PartitionUnit::NotEvaluated
                    ||self.independent_unit_kind!=IndependentUnitKind::NotEvaluated
                    ||self.independent_unit_count!=0
                    ||self.target_domain_access!=TargetDomainAccess::NotEvaluated
                    ||self.leakage_risk!=LeakageRisk::NotEvaluated
                    ||self.physical_identity_verified||self.source_artifact_payloads_verified
                    ||self.reviewer_scope!=ReviewerScope::NotReviewed||score_claim_allowed{
                    return Err(ValidationEvidenceError::InvalidNotEvaluatedProfile);
                }
                if matches!(run_status,"planned_not_executed"|"data_not_audited"|"protocol_reproduction_pending"){return Ok(());}
                return Err(ValidationEvidenceError::InvalidNotEvaluatedProfile);
            }
            ProfileStatus::StructurallyAssessed=>{
                if !self.digest_valid()
                    ||self.physical_independence_level==PhysicalIndependenceLevel::NotEvaluated
                    ||self.domain_shift_level==DomainShiftLevel::NotEvaluated
                    ||self.partition_unit==PartitionUnit::NotEvaluated
                    ||self.independent_unit_kind==IndependentUnitKind::NotEvaluated
                    ||self.independent_unit_count==0
                    ||self.target_domain_access==TargetDomainAccess::NotEvaluated
                    ||self.leakage_risk==LeakageRisk::NotEvaluated
                    ||self.physical_identity_verified||self.source_artifact_payloads_verified
                    ||self.reviewer_scope!=ReviewerScope::StructuralMetadataOnly||score_claim_allowed{
                    return Err(ValidationEvidenceError::InvalidStructuralProfile);
                }
            }
            ProfileStatus::IndependentlyReviewed=>{
                if !self.digest_valid()||!self.physical_independence_level.supports_score()
                    ||self.domain_shift_level==DomainShiftLevel::NotEvaluated
                    ||self.partition_unit==PartitionUnit::NotEvaluated
                    ||self.independent_unit_kind==IndependentUnitKind::NotEvaluated
                    ||self.independent_unit_count==0
                    ||self.target_domain_access==TargetDomainAccess::NotEvaluated
                    ||!matches!(self.leakage_risk,LeakageRisk::Low|LeakageRisk::SomeConcerns)
                    ||!self.physical_identity_verified||!self.source_artifact_payloads_verified
                    ||self.reviewer_scope!=ReviewerScope::IndependentPhysicalAudit{
                    return Err(ValidationEvidenceError::InvalidIndependentReview);
                }
            }
        }
        if score_claim_allowed{
            if run_status!="qualified"||evidence_level=="P0_synthetic_contract"
                ||self.profile_status!=ProfileStatus::IndependentlyReviewed
                ||!self.physical_independence_level.supports_score()
                ||matches!(self.target_domain_access,TargetDomainAccess::Unclear|TargetDomainAccess::NotEvaluated)
                ||!matches!(self.leakage_risk,LeakageRisk::Low|LeakageRisk::SomeConcerns){
                return Err(ValidationEvidenceError::ScoreClaimWithoutIndependentEvidence);
            }
        }
        if run_status=="qualified"&&!score_claim_allowed{return Err(ValidationEvidenceError::ScoreClaimWithoutIndependentEvidence);}
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn planned()->ValidationEvidenceProfile {
        ValidationEvidenceProfile {
            profile_schema_version:"industrial-evidence-matrix-v0.1".into(),profile_receipt_sha256:None,
            profile_status:ProfileStatus::NotEvaluated,physical_independence_level:PhysicalIndependenceLevel::NotEvaluated,
            domain_shift_level:DomainShiftLevel::NotEvaluated,partition_unit:PartitionUnit::NotEvaluated,
            independent_unit_kind:IndependentUnitKind::NotEvaluated,independent_unit_count:0,
            target_domain_access:TargetDomainAccess::NotEvaluated,leakage_risk:LeakageRisk::NotEvaluated,
            physical_identity_verified:false,source_artifact_payloads_verified:false,
            reviewer_scope:ReviewerScope::NotReviewed,known_limitations:vec!["No source payload audit".into()],
        }
    }
    fn reviewed()->ValidationEvidenceProfile {
        ValidationEvidenceProfile {
            profile_schema_version:"industrial-evidence-matrix-v0.1".into(),profile_receipt_sha256:Some("a".repeat(64)),
            profile_status:ProfileStatus::IndependentlyReviewed,physical_independence_level:PhysicalIndependenceLevel::A1,
            domain_shift_level:DomainShiftLevel::B1,partition_unit:PartitionUnit::Recording,
            independent_unit_kind:IndependentUnitKind::Recording,independent_unit_count:9,
            target_domain_access:TargetDomainAccess::UnlabeledAdaptation,leakage_risk:LeakageRisk::Low,
            physical_identity_verified:true,source_artifact_payloads_verified:true,
            reviewer_scope:ReviewerScope::IndependentPhysicalAudit,known_limitations:vec!["Single physical engine".into()],
        }
    }
    #[test] fn planned_profile_cannot_claim_score(){assert!(planned().validate_for_claim("planned_not_executed","P0_synthetic_contract",false,false).is_ok());}
    #[test] fn structural_metadata_profile_cannot_publish_score(){
        let mut p=reviewed();p.profile_status=ProfileStatus::StructurallyAssessed;p.physical_identity_verified=false;p.source_artifact_payloads_verified=false;p.reviewer_scope=ReviewerScope::StructuralMetadataOnly;
        assert!(p.validate_for_claim("executed_unqualified","P1_public_benchmark_reproduction",false,false).is_ok());
        assert_eq!(p.validate_for_claim("qualified","P1_public_benchmark_reproduction",true,false),Err(ValidationEvidenceError::InvalidStructuralProfile));
    }
    #[test] fn independently_reviewed_profile_supports_public_benchmark_score(){assert!(reviewed().validate_for_claim("qualified","P1_public_benchmark_reproduction",true,false).is_ok());}
    #[test] fn high_or_unclear_leakage_blocks_score(){
        for risk in [LeakageRisk::High,LeakageRisk::Unclear]{let mut p=reviewed();p.leakage_risk=risk;assert_eq!(p.validate_for_claim("qualified","P1_public_benchmark_reproduction",true,false),Err(ValidationEvidenceError::InvalidIndependentReview));}
    }
    #[test] fn sample_windows_cannot_support_unique_score(){let mut p=reviewed();p.physical_independence_level=PhysicalIndependenceLevel::A0;assert_eq!(p.validate_for_claim("qualified","P1_public_benchmark_reproduction",true,false),Err(ValidationEvidenceError::InvalidIndependentReview));}
    #[test] fn physical_transfer_claim_is_forbidden(){assert_eq!(reviewed().validate_for_claim("qualified","P1_public_benchmark_reproduction",true,true),Err(ValidationEvidenceError::PhysicalTransferClaimForbidden));}
}
