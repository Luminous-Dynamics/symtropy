//! Source-faithful experimental observation register validation.
//!
//! This module checks transcription structure, identity, source references,
//! value/status semantics and the accepted-product claim gate. It does not
//! authenticate publications, calibrations, laboratory records or assays.

use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Numeric,
    Range,
    Qualitative,
    NotReported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStage {
    PrimaryInput,
    FeedstockCondition,
    ProcessCondition,
    ProcessMeasurement,
    ProcessSignal,
    EstimatedRelease,
    EjectedMaterial,
    CapturedRaw,
    Purified,
    AcceptedProduct,
    NotReported,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct NumericInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObservationSourceRecord {
    pub id: String,
    pub title: String,
    pub url: String,
    pub evidence_class: String,
    pub publication_date: Option<String>,
    pub source_note: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObservationRecord {
    pub id: String,
    pub source_id: String,
    pub status: ObservationStatus,
    pub metric: String,
    pub value: Option<f64>,
    pub range: Option<NumericInterval>,
    pub unit: Option<String>,
    pub statement: Option<String>,
    pub source_locator: String,
    pub method: Option<String>,
    pub stage: EvidenceStage,
    pub conditions: serde_json::Value,
    pub caveat: String,
    pub uncertainty: Option<String>,
    pub independent_assay_id: Option<String>,
    pub independent_assay_issuer_id: Option<String>,
    pub product_contract_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObservationRegister {
    pub schema_version: String,
    pub retrieved_date_utc: String,
    pub claim_ceiling: String,
    pub source_records: Vec<ObservationSourceRecord>,
    pub observations: Vec<ObservationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterError {
    UnsupportedSchema,
    MissingClaimCeiling,
    MissingRetrievalDate,
    DuplicateSourceId(String),
    DuplicateObservationId(String),
    UnknownSource(String),
    MissingSourceField(&'static str),
    MissingObservationField(&'static str),
    NonFiniteValue(&'static str),
    InvalidNumericInterval,
    StatusValueMismatch(&'static str),
    AcceptedProductEvidenceMissing,
    AcceptedProductMustBePositive,
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema => write!(f, "unsupported observation-register schema"),
            Self::MissingClaimCeiling => write!(f, "claim ceiling is required"),
            Self::MissingRetrievalDate => write!(f, "retrieval date is required"),
            Self::DuplicateSourceId(id) => write!(f, "duplicate source identity: {id}"),
            Self::DuplicateObservationId(id) => write!(f, "duplicate observation identity: {id}"),
            Self::UnknownSource(id) => write!(f, "observation references unknown source: {id}"),
            Self::MissingSourceField(field) => write!(f, "source field {field} is required"),
            Self::MissingObservationField(field) => write!(f, "observation field {field} is required"),
            Self::NonFiniteValue(field) => write!(f, "{field} must be finite"),
            Self::InvalidNumericInterval => write!(f, "numeric interval must satisfy finite lower <= upper"),
            Self::StatusValueMismatch(field) => write!(f, "observation status/value mismatch: {field}"),
            Self::AcceptedProductEvidenceMissing => write!(
                f,
                "accepted product requires independent assay, issuer and product-contract identities"
            ),
            Self::AcceptedProductMustBePositive => write!(f, "accepted product quantity must be positive"),
        }
    }
}

impl std::error::Error for RegisterError {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_value_semantics(record: &ObservationRecord) -> Result<(), RegisterError> {
    match record.status {
        ObservationStatus::Numeric => {
            let value = record.value.ok_or(RegisterError::StatusValueMismatch("numeric_value_missing"))?;
            if !value.is_finite() {
                return Err(RegisterError::NonFiniteValue("value"));
            }
            if record.range.is_some() {
                return Err(RegisterError::StatusValueMismatch("numeric_and_range_both_present"));
            }
            if !record.unit.as_deref().is_some_and(nonempty) {
                return Err(RegisterError::StatusValueMismatch("numeric_unit_missing"));
            }
        }
        ObservationStatus::Range => {
            if record.value.is_some() {
                return Err(RegisterError::StatusValueMismatch("range_has_scalar_value"));
            }
            let interval = record.range.ok_or(RegisterError::StatusValueMismatch("range_missing"))?;
            if !interval.lower.is_finite()
                || !interval.upper.is_finite()
                || interval.lower > interval.upper
            {
                return Err(RegisterError::InvalidNumericInterval);
            }
            if !record.unit.as_deref().is_some_and(nonempty) {
                return Err(RegisterError::StatusValueMismatch("range_unit_missing"));
            }
        }
        ObservationStatus::Qualitative | ObservationStatus::NotReported => {
            if record.value.is_some() || record.range.is_some() || record.unit.is_some() {
                return Err(RegisterError::StatusValueMismatch("non_numeric_record_has_numeric_value"));
            }
            if !record.statement.as_deref().is_some_and(nonempty) {
                return Err(RegisterError::StatusValueMismatch("qualitative_or_missing_statement_required"));
            }
        }
    }
    Ok(())
}

fn validate_accepted_product(record: &ObservationRecord) -> Result<(), RegisterError> {
    if record.stage != EvidenceStage::AcceptedProduct {
        return Ok(());
    }

    let conservative_quantity = match record.status {
        ObservationStatus::Numeric => record.value.unwrap_or(0.0),
        ObservationStatus::Range => record.range.map(|range| range.lower).unwrap_or(0.0),
        ObservationStatus::Qualitative | ObservationStatus::NotReported => 0.0,
    };
    if !conservative_quantity.is_finite() || conservative_quantity <= 0.0 {
        return Err(RegisterError::AcceptedProductMustBePositive);
    }

    let has_assay = record.independent_assay_id.as_deref().is_some_and(nonempty);
    let has_issuer = record.independent_assay_issuer_id.as_deref().is_some_and(nonempty);
    let has_contract = record.product_contract_id.as_deref().is_some_and(nonempty);
    if !(has_assay && has_issuer && has_contract) {
        return Err(RegisterError::AcceptedProductEvidenceMissing);
    }
    Ok(())
}

/// Validate the immutable source and observation identities, references and
/// explicit value semantics. This does not assert that any source claim is true.
pub fn validate_observation_register(
    register: &ObservationRegister,
) -> Result<(), RegisterError> {
    if register.schema_version != "0.1" {
        return Err(RegisterError::UnsupportedSchema);
    }
    if !nonempty(&register.claim_ceiling) {
        return Err(RegisterError::MissingClaimCeiling);
    }
    if !nonempty(&register.retrieved_date_utc) {
        return Err(RegisterError::MissingRetrievalDate);
    }

    let mut source_ids = BTreeSet::new();
    for source in &register.source_records {
        if !source_ids.insert(source.id.as_str()) {
            return Err(RegisterError::DuplicateSourceId(source.id.clone()));
        }
        for (field, value) in [
            ("source.id", source.id.as_str()),
            ("source.title", source.title.as_str()),
            ("source.url", source.url.as_str()),
            ("source.evidence_class", source.evidence_class.as_str()),
            ("source.source_note", source.source_note.as_str()),
        ] {
            if !nonempty(value) {
                return Err(RegisterError::MissingSourceField(field));
            }
        }
    }

    let mut observation_ids = BTreeSet::new();
    for record in &register.observations {
        if !observation_ids.insert(record.id.as_str()) {
            return Err(RegisterError::DuplicateObservationId(record.id.clone()));
        }
        for (field, value) in [
            ("observation.id", record.id.as_str()),
            ("observation.source_id", record.source_id.as_str()),
            ("observation.metric", record.metric.as_str()),
            ("observation.source_locator", record.source_locator.as_str()),
            ("observation.caveat", record.caveat.as_str()),
        ] {
            if !nonempty(value) {
                return Err(RegisterError::MissingObservationField(field));
            }
        }
        if !source_ids.contains(record.source_id.as_str()) {
            return Err(RegisterError::UnknownSource(record.source_id.clone()));
        }
        if !record.conditions.is_object() {
            return Err(RegisterError::MissingObservationField("conditions_object"));
        }

        validate_value_semantics(record)?;
        validate_accepted_product(record)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_register() -> ObservationRegister {
        serde_json::from_str(include_str!("../fixtures/experimental-observations-v0.1.json"))
            .expect("frozen observation register must parse")
    }

    fn valid_numeric_observation() -> ObservationRecord {
        ObservationRecord {
            id: "test-accepted-product".to_string(),
            source_id: "source-test".to_string(),
            status: ObservationStatus::Numeric,
            metric: "accepted_product_mass".to_string(),
            value: Some(1.0),
            range: None,
            unit: Some("kg".to_string()),
            statement: None,
            source_locator: "test fixture".to_string(),
            method: Some("independent assay".to_string()),
            stage: EvidenceStage::AcceptedProduct,
            conditions: serde_json::json!({"synthetic": true}),
            caveat: "Synthetic test only.".to_string(),
            uncertainty: None,
            independent_assay_id: Some("assay-001".to_string()),
            independent_assay_issuer_id: Some("lab-001".to_string()),
            product_contract_id: Some("product-contract-001".to_string()),
        }
    }

    #[test]
    fn frozen_register_passes_structural_validation() {
        let register = valid_register();
        assert!(validate_observation_register(&register).is_ok());
    }

    #[test]
    fn frozen_register_has_unique_identities_and_resolved_sources() {
        let register = valid_register();
        let source_ids: BTreeSet<_> = register.source_records.iter().map(|source| source.id.as_str()).collect();
        let observation_ids: BTreeSet<_> = register.observations.iter().map(|record| record.id.as_str()).collect();
        assert_eq!(source_ids.len(), register.source_records.len());
        assert_eq!(observation_ids.len(), register.observations.len());
        assert!(register.observations.iter().all(|record| source_ids.contains(record.source_id.as_str())));
    }

    #[test]
    fn explicit_not_reported_values_remain_null() {
        let register = valid_register();
        assert!(register.observations.iter().filter(|record| record.status == ObservationStatus::NotReported).all(|record| {
            record.value.is_none() && record.range.is_none() && record.unit.is_none()
        }));
    }

    #[test]
    fn duplicate_source_identity_is_rejected() {
        let mut register = valid_register();
        register.source_records.push(register.source_records[0].clone());
        assert!(matches!(
            validate_observation_register(&register),
            Err(RegisterError::DuplicateSourceId(_))
        ));
    }

    #[test]
    fn duplicate_observation_identity_is_rejected() {
        let mut register = valid_register();
        register.observations.push(register.observations[0].clone());
        assert!(matches!(
            validate_observation_register(&register),
            Err(RegisterError::DuplicateObservationId(_))
        ));
    }

    #[test]
    fn dangling_source_reference_is_rejected() {
        let mut register = valid_register();
        register.observations[0].source_id = "unknown-source".to_string();
        assert_eq!(
            validate_observation_register(&register).unwrap_err(),
            RegisterError::UnknownSource("unknown-source".to_string())
        );
    }

    #[test]
    fn not_reported_observation_cannot_be_zero_filled() {
        let mut register = valid_register();
        let record = register.observations.iter_mut().find(|record| record.status == ObservationStatus::NotReported).unwrap();
        record.value = Some(0.0);
        assert_eq!(
            validate_observation_register(&register).unwrap_err(),
            RegisterError::StatusValueMismatch("non_numeric_record_has_numeric_value")
        );
    }

    #[test]
    fn numeric_observation_requires_a_unit() {
        let mut register = valid_register();
        let record = register.observations.iter_mut().find(|record| record.status == ObservationStatus::Numeric).unwrap();
        record.unit = None;
        assert_eq!(
            validate_observation_register(&register).unwrap_err(),
            RegisterError::StatusValueMismatch("numeric_unit_missing")
        );
    }

    #[test]
    fn accepted_product_requires_independent_assay_and_contract() {
        let mut record = valid_numeric_observation();
        record.independent_assay_id = None;
        assert_eq!(
            validate_accepted_product(&record).unwrap_err(),
            RegisterError::AcceptedProductEvidenceMissing
        );
    }

    #[test]
    fn accepted_product_must_be_positive() {
        let mut record = valid_numeric_observation();
        record.value = Some(0.0);
        assert_eq!(
            validate_accepted_product(&record).unwrap_err(),
            RegisterError::AcceptedProductMustBePositive
        );
    }

    #[test]
    fn range_requires_finite_ordered_bounds() {
        let mut record = valid_numeric_observation();
        record.stage = EvidenceStage::EstimatedRelease;
        record.status = ObservationStatus::Range;
        record.value = None;
        record.range = Some(NumericInterval { lower: 2.0, upper: 1.0 });
        assert_eq!(
            validate_value_semantics(&record).unwrap_err(),
            RegisterError::InvalidNumericInterval
        );
    }

    #[test]
    fn qualitative_signal_cannot_be_promoted_to_product() {
        let mut record = valid_numeric_observation();
        record.status = ObservationStatus::Qualitative;
        record.value = None;
        record.unit = None;
        record.statement = Some("CO detected".to_string());
        record.independent_assay_id = None;
        assert_eq!(
            validate_accepted_product(&record).unwrap_err(),
            RegisterError::AcceptedProductMustBePositive
        );
    }
}
