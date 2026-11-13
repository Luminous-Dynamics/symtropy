#![forbid(unsafe_code)]

//! Contract checks for literature baselines whose public code/data packages
//! are not sufficient to reproduce the headline claim.
//!
//! This module does not reproduce a paper result. It ensures that incomplete
//! public releases stay explicitly marked as incomplete and that reported
//! scores are never promoted into Symtropy measurements.

use serde_json::Value;
use std::fmt;

pub const MARINE_ENGINE_BASELINE_AUDIT_SCHEMA: &str =
    "symtropy-published-marine-engine-baseline-audit-v0.1";
pub const MULTISOURCE_TRANSFER_SCHEMA: &str =
    "symtropy-marine-multisource-virtual-real-transfer-v0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteratureAuditError {
    InvalidMarineBaseline(String),
    InvalidMultisourceTransfer(String),
}

impl fmt::Display for LiteratureAuditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMarineBaseline(reason) => {
                write!(formatter, "marine-engine literature audit contract is invalid: {reason}")
            }
            Self::InvalidMultisourceTransfer(reason) => {
                write!(formatter, "multi-source transfer contract is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for LiteratureAuditError {}

/// Validate the published iScience result against the observed public GitHub
/// code/data snapshot. The test deliberately refuses to convert a one-class
/// public CSV into evidence for an eleven-state reproduction.
pub fn validate_marine_engine_baseline_audit(
    audit: &Value,
) -> Result<(), LiteratureAuditError> {
    let invalid = |reason: &str| {
        LiteratureAuditError::InvalidMarineBaseline(reason.to_owned())
    };
    if audit["schema_version"] != MARINE_ENGINE_BASELINE_AUDIT_SCHEMA {
        return Err(invalid("unexpected schema version"));
    }
    if audit["status"] != "public_release_preflight_incomplete_not_reproduced" {
        return Err(invalid("incomplete public release must remain explicitly unreproduced"));
    }
    if audit["publication"]["doi"] != "10.1016/j.isci.2025.114345"
        || audit["publication"]["article_reported_accuracy_percent"] != 99.45
    {
        return Err(invalid("publication identity or reported reference score changed"));
    }
    let sample = &audit["public_code_repository"]["sample_csv"];
    if sample["parsed_rows"] != 1025
        || sample["columns"] != 25
        || sample["observed_unique_labels"] != 1
        || sample["observed_label_counts"]["10"] != 1025
        || sample["paper_described_state_count"] != 11
        || sample["label_coverage_complete"] != false
    {
        return Err(invalid("public CSV no longer matches the recorded one-class snapshot"));
    }
    if audit["public_code_repository"]["observed_commit_sha"]
        != "aa9b0e41598f17f89750f9cf68aefad95cbbd9ab"
        || audit["public_code_repository"]["sample_csv"]["git_blob_sha"]
            != "80eb557243fcf982cfa9d84aa4d1bb590e8b7eda"
    {
        return Err(invalid("public source snapshot identity changed; audit must be refreshed"));
    }
    if audit["implementation_review"]["repository_script_split"]["test_fraction"] != 0.2
        || audit["implementation_review"]["repository_script_split"]["shuffle"] != true
        || audit["implementation_review"]["repository_script_split"]["stratified"] != true
    {
        return Err(invalid("public source split description changed"));
    }
    if audit["audit_disposition"]["reproduced"] != false
        || audit["audit_disposition"]["public_data_reproduced"] != false
        || audit["audit_disposition"]["model_score_claim_allowed"] != false
        || audit["audit_disposition"]["independent_vessel_transfer_claim_allowed"] != false
    {
        return Err(invalid("incomplete public release must not permit a reproduced-score claim"));
    }
    Ok(())
}

/// Validate that the 2026 multi-source marine-engine result remains a
/// literature-only transfer reference until the dataset, license and splits
/// have been independently retrieved and audited.
pub fn validate_multisource_transfer_reference(
    reference: &Value,
) -> Result<(), LiteratureAuditError> {
    let invalid = |reason: &str| {
        LiteratureAuditError::InvalidMultisourceTransfer(reason.to_owned())
    };
    if reference["schema_version"] != MULTISOURCE_TRANSFER_SCHEMA {
        return Err(invalid("unexpected schema version"));
    }
    if reference["status"] != "published_dataset_reference_access_not_yet_audited" {
        return Err(invalid("dataset access status must remain unaudited"));
    }
    if reference["publication"]["doi"] != "10.1038/s41597-026-07924-7"
        || reference["publication"]["dataset_license"] != "not_verified_from_accessible_record"
        || reference["publication"]["data_access_url"] != Value::Null
    {
        return Err(invalid("dataset DOI, license or access boundary changed"));
    }
    let dataset = &reference["reported_dataset"];
    if dataset["load_range_percent"][0] != 25
        || dataset["load_range_percent"][1] != 100
        || dataset["health_states"]["healthy"] != 1
        || dataset["health_states"]["representative_thermal_fault_modes"] != 10
        || dataset["source_domains"].as_array().map(Vec::len) != Some(4)
    {
        return Err(invalid("published dataset scope or source-domain inventory changed"));
    }
    let transfer = &dataset["published_transfer_result"];
    if transfer["value_percent"] != 70.18
        || transfer["status"] != "published_reference_not_reproduced"
    {
        return Err(invalid("zero-shot virtual-to-real reference must remain unreproduced"));
    }
    if reference["hard_gates"]
        .as_array()
        .is_none_or(|gates| gates.len() < 10)
        || reference["claim_ceiling"]
            .as_str()
            .is_none_or(|claim| !claim.contains("not been independently audited"))
    {
        return Err(invalid("hard evidence gates or claim ceiling are missing"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_iscience_baseline_is_not_misreported_as_reproduced() {
        let audit: Value = serde_json::from_str(include_str!(
            "../published-marine-engine-baseline-audit-v0.1.json"
        ))
        .unwrap();
        assert!(validate_marine_engine_baseline_audit(&audit).is_ok());
        assert_eq!(audit["publication"]["article_reported_accuracy_percent"], 99.45);
        assert_eq!(audit["public_code_repository"]["sample_csv"]["parsed_rows"], 1025);
        assert_eq!(audit["public_code_repository"]["sample_csv"]["observed_unique_labels"], 1);
        assert_eq!(audit["audit_disposition"]["model_score_claim_allowed"], false);
    }

    #[test]
    fn one_class_public_snapshot_cannot_be_promoted_to_an_eleven_state_reproduction() {
        let mut audit: Value = serde_json::from_str(include_str!(
            "../published-marine-engine-baseline-audit-v0.1.json"
        ))
        .unwrap();
        audit["public_code_repository"]["sample_csv"]["observed_unique_labels"] = 11.into();
        assert!(matches!(
            validate_marine_engine_baseline_audit(&audit),
            Err(LiteratureAuditError::InvalidMarineBaseline(_))
        ));
    }

    #[test]
    fn multi_source_virtual_real_result_remains_a_literature_reference() {
        let reference: Value = serde_json::from_str(include_str!(
            "../multi-source-virtual-real-transfer-v0.1.json"
        ))
        .unwrap();
        assert!(validate_multisource_transfer_reference(&reference).is_ok());
        assert_eq!(
            reference["reported_dataset"]["published_transfer_result"]["value_percent"],
            70.18
        );
        assert_eq!(reference["publication"]["dataset_license"], "not_verified_from_accessible_record");
    }

    #[test]
    fn virtual_real_fault_fusion_is_not_treated_as_observed_field_fault_ground_truth() {
        let reference: Value = serde_json::from_str(include_str!(
            "../multi-source-virtual-real-transfer-v0.1.json"
        ))
        .unwrap();
        let track = reference["benchmark_tracks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "virtual_real_fault_fusion")
            .unwrap();
        assert_eq!(
            track["status"],
            "not_qualified_until_real_vessel_fault_ground_truth_exists"
        );
    }
}
