#![forbid(unsafe_code)]

//! Ordered calibration-chain structural audit and conservative uncertainty ledger.
//!
//! This M1 reference validates declared calibration-link continuity, sensor and
//! terminal-reference identity, validity dates, and before/after measurement
//! assurance checks. It aggregates only validated absolute-error bounds using
//! the triangle inequality. Standard/expanded uncertainties are preserved as
//! statistical-only evidence and are never promoted to hard bounds.
//!
//! The code cannot authenticate certificates, establish laboratory competence,
//! verify SI traceability, or prove that a supplied uncertainty contribution is
//! correct. Those claims require external evidence and independent review.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkUncertaintyKind {
    ValidatedAbsoluteBound,
    StandardUncertainty,
    ExpandedUncertainty,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssuranceStatus {
    WithinLimit,
    OutOfTolerance,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainDisposition {
    BoundQualified,
    StatisticalUncertaintyOnly,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationLink {
    pub link_id: String,
    /// Item whose calibration is established by this link.
    pub calibrated_item_id: String,
    /// Reference standard used for the calibrated item.
    /// This must equal the next link's calibrated_item_id, or the chain's
    /// declared terminal_reference_id for the final link.
    pub reference_standard_id: String,
    pub calibration_certificate_reference: String,
    pub reference_standard_reference: String,
    pub calibration_procedure_reference: String,
    pub competence_evidence_reference: String,
    pub calibration_day: i64,
    pub valid_from_day: i64,
    pub valid_until_day: i64,
    /// Every link contribution is converted to the final measurement measurand
    /// unit by the referenced conversion/measurement model.
    pub uncertainty_unit: String,
    pub uncertainty_kind: LinkUncertaintyKind,
    pub uncertainty_value: Option<f64>,
    /// Required for expanded uncertainty; retained for the report.
    pub coverage_factor: Option<f64>,
    /// Required for standard/expanded uncertainty so the later GUM path can
    /// recover the declared distribution/model assumptions.
    pub distribution_reference: Option<String>,
    /// Describes how this link's uncertainty contribution was mapped into the
    /// final measurand unit. This is not verified by the M1 structural auditor.
    pub conversion_model_reference: String,
    pub certificate_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementAssuranceCheck {
    pub status: AssuranceStatus,
    pub check_day: i64,
    pub method_reference: String,
    pub reference_artifact_id: String,
    pub raw_check_digest: String,
    pub observed_drift: Option<f64>,
    pub declared_drift_limit: Option<f64>,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationChainRecord {
    pub sensor_serial: String,
    pub measurement_day: i64,
    pub measurand_unit: String,
    pub terminal_reference_id: String,
    pub terminal_reference_source: String,
    pub chain_digest: String,
    pub links: Vec<CalibrationLink>,
    pub pre_use_check: MeasurementAssuranceCheck,
    pub post_use_check: MeasurementAssuranceCheck,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkAuditReceipt {
    pub link_id: String,
    pub calibrated_item_id: String,
    pub reference_standard_id: String,
    pub calibration_certificate_reference: String,
    pub reference_standard_reference: String,
    pub calibration_procedure_reference: String,
    pub competence_evidence_reference: String,
    pub calibration_day: i64,
    pub valid_from_day: i64,
    pub valid_until_day: i64,
    pub uncertainty_kind: LinkUncertaintyKind,
    pub uncertainty_value: Option<f64>,
    pub uncertainty_unit: String,
    pub coverage_factor: Option<f64>,
    pub distribution_reference: Option<String>,
    pub conversion_model_reference: String,
    pub certificate_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationChainReport {
    pub disposition: ChainDisposition,
    pub sensor_serial: String,
    pub measurement_day: i64,
    pub measurand_unit: String,
    pub terminal_reference_id: String,
    pub chain_digest: String,
    pub link_count: usize,
    /// Present only when every calibration link contributes a validated
    /// absolute-error bound in the final measurand unit.
    pub total_absolute_error_bound: Option<f64>,
    pub statistical_only_link_count: usize,
    pub unknown_uncertainty_link_count: usize,
    pub pre_use_check_status: AssuranceStatus,
    pub post_use_check_status: AssuranceStatus,
    pub link_receipts: Vec<LinkAuditReceipt>,
    pub reasons: Vec<String>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn required(value: &str, label: &str) -> Result<(), String> {
    if nonempty(value) {
        Ok(())
    } else {
        Err(format!("{label} must be nonempty"))
    }
}

fn finite_nonnegative(value: f64, label: &str) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("{label} must be finite and nonnegative"))
    }
}

fn validate_assurance_check(
    check: &MeasurementAssuranceCheck,
    label: &str,
    measurement_day: i64,
    pre_use: bool,
    expected_unit: &str,
) -> Result<Option<String>, String> {
    required(&check.method_reference, &format!("{label} method reference"))?;
    required(&check.reference_artifact_id, &format!("{label} reference artifact ID"))?;
    required(&check.raw_check_digest, &format!("{label} raw-check digest"))?;

    if check.unit != expected_unit {
        return Err(format!(
            "{label} drift unit {:?} does not match measurand unit {:?}",
            check.unit, expected_unit
        ));
    }

    match (check.observed_drift, check.declared_drift_limit) {
        (Some(drift), Some(limit)) => {
            finite_nonnegative(drift, &format!("{label} observed drift"))?;
            finite_nonnegative(limit, &format!("{label} drift limit"))?;
            if drift > limit {
                return Ok(Some(format!(
                    "{label} observed drift {drift} exceeds declared limit {limit}"
                )));
            }
        }
        _ => return Ok(Some(format!("{label} drift or limit is unknown"))),
    }

    if pre_use && check.check_day > measurement_day {
        return Err(format!("{label} check occurs after the measurement day"));
    }
    if !pre_use && check.check_day < measurement_day {
        return Err(format!("{label} check occurs before the measurement day"));
    }

    match check.status {
        AssuranceStatus::WithinLimit => Ok(None),
        AssuranceStatus::OutOfTolerance => {
            Ok(Some(format!("{label} measurement-assurance check is out of tolerance")))
        }
        AssuranceStatus::Unknown => {
            Ok(Some(format!("{label} measurement-assurance status is unknown")))
        }
    }
}

fn validate_link(
    link: &CalibrationLink,
    measurement_day: i64,
    expected_unit: &str,
) -> Result<Option<String>, String> {
    required(&link.link_id, "calibration link ID")?;
    required(&link.calibrated_item_id, "calibrated item ID")?;
    required(&link.reference_standard_id, "reference standard ID")?;
    required(
        &link.calibration_certificate_reference,
        "calibration certificate reference",
    )?;
    required(
        &link.reference_standard_reference,
        "reference standard reference",
    )?;
    required(
        &link.calibration_procedure_reference,
        "calibration procedure reference",
    )?;
    required(
        &link.competence_evidence_reference,
        "technical-competence evidence reference",
    )?;
    required(
        &link.conversion_model_reference,
        "uncertainty conversion/measurement-model reference",
    )?;
    required(&link.certificate_digest, "calibration certificate digest")?;

    if link.calibration_day > link.valid_from_day
        || link.valid_from_day > link.valid_until_day
    {
        return Err(format!(
            "calibration link {} has an invalid calibration/validity date order",
            link.link_id
        ));
    }
    if measurement_day < link.valid_from_day || measurement_day > link.valid_until_day {
        return Ok(Some(format!(
            "calibration link {} is not valid on measurement day {}",
            link.link_id, measurement_day
        )));
    }

    if link.uncertainty_unit != expected_unit {
        return Err(format!(
            "calibration link {} uncertainty unit {:?} does not match final measurand unit {:?}",
            link.link_id, link.uncertainty_unit, expected_unit
        ));
    }

    match link.uncertainty_kind {
        LinkUncertaintyKind::ValidatedAbsoluteBound => {
            let Some(value) = link.uncertainty_value else {
                return Err(format!(
                    "calibration link {} is marked as a validated bound but has no bound value",
                    link.link_id
                ));
            };
            finite_nonnegative(value, &format!("link {} absolute-error bound", link.link_id))?;
            if link.coverage_factor.is_some() || link.distribution_reference.is_some() {
                return Err(format!(
                    "calibration link {} mixes a deterministic bound with statistical uncertainty fields",
                    link.link_id
                ));
            }
            Ok(None)
        }
        LinkUncertaintyKind::StandardUncertainty => {
            let Some(value) = link.uncertainty_value else {
                return Err(format!(
                    "calibration link {} standard uncertainty has no value",
                    link.link_id
                ));
            };
            finite_nonnegative(value, &format!("link {} standard uncertainty", link.link_id))?;
            required(
                link.distribution_reference.as_deref().unwrap_or(""),
                &format!("link {} uncertainty distribution/model reference", link.link_id),
            )?;
            if link.coverage_factor.is_some() {
                return Err(format!(
                    "calibration link {} standard uncertainty must not claim an expanded-uncertainty coverage factor",
                    link.link_id
                ));
            }
            Ok(Some(format!(
                "calibration link {} supplies standard uncertainty, not a deterministic hard bound",
                link.link_id
            )))
        }
        LinkUncertaintyKind::ExpandedUncertainty => {
            let Some(value) = link.uncertainty_value else {
                return Err(format!(
                    "calibration link {} expanded uncertainty has no value",
                    link.link_id
                ));
            };
            finite_nonnegative(value, &format!("link {} expanded uncertainty", link.link_id))?;
            let Some(coverage_factor) = link.coverage_factor else {
                return Err(format!(
                    "calibration link {} expanded uncertainty lacks a coverage factor",
                    link.link_id
                ));
            };
            if !coverage_factor.is_finite() || coverage_factor <= 0.0 {
                return Err(format!(
                    "calibration link {} coverage factor must be finite and positive",
                    link.link_id
                ));
            }
            required(
                link.distribution_reference.as_deref().unwrap_or(""),
                &format!("link {} uncertainty distribution/model reference", link.link_id),
            )?;
            Ok(Some(format!(
                "calibration link {} supplies expanded uncertainty, not a deterministic hard bound",
                link.link_id
            )))
        }
        LinkUncertaintyKind::Unknown => {
            if link.uncertainty_value.is_some() {
                return Err(format!(
                    "calibration link {} marks uncertainty Unknown but supplies a numeric value",
                    link.link_id
                ));
            }
            Ok(Some(format!(
                "calibration link {} uncertainty is Unknown",
                link.link_id
            )))
        }
    }
}

/// Audits one ordered calibration chain.
///
/// A structurally broken chain is rejected as invalid input. A complete chain
/// with out-of-date calibration or failed/unknown before/after assurance checks
/// returns an Unknown disposition. A complete statistical-uncertainty chain
/// returns StatisticalUncertaintyOnly and no deterministic aggregate bound.
pub fn audit_calibration_chain(
    record: &CalibrationChainRecord,
) -> Result<CalibrationChainReport, String> {
    required(&record.sensor_serial, "sensor serial")?;
    required(&record.measurand_unit, "final measurand unit")?;
    required(&record.terminal_reference_id, "terminal reference ID")?;
    required(&record.terminal_reference_source, "terminal reference source")?;
    required(&record.chain_digest, "calibration-chain digest")?;

    if record.links.is_empty() {
        return Err("calibration chain must contain at least one link".to_owned());
    }

    let mut link_ids = BTreeSet::new();
    let mut calibrated_items = BTreeSet::new();
    let mut reasons = Vec::new();
    let mut total_absolute_error_bound = 0.0_f64;
    let mut statistical_only_link_count = 0usize;
    let mut unknown_uncertainty_link_count = 0usize;
    let mut link_receipts = Vec::with_capacity(record.links.len());

    if record.links[0].calibrated_item_id != record.sensor_serial {
        return Err(
            "first calibration link does not calibrate the declared sensor serial".to_owned(),
        );
    }

    for (index, link) in record.links.iter().enumerate() {
        if !link_ids.insert(link.link_id.as_str()) {
            return Err(format!("duplicate calibration link ID {:?}", link.link_id));
        }
        if !calibrated_items.insert(link.calibrated_item_id.as_str()) {
            return Err(format!(
                "duplicate calibrated item ID {:?}; cyclic or repeated chain node",
                link.calibrated_item_id
            ));
        }
        if link.calibrated_item_id == link.reference_standard_id {
            return Err(format!(
                "calibration link {} references itself",
                link.link_id
            ));
        }

        if index + 1 < record.links.len() {
            let next = &record.links[index + 1];
            if link.reference_standard_id != next.calibrated_item_id {
                return Err(format!(
                    "broken calibration chain between link {} reference {} and next link calibrated item {}",
                    link.link_id, link.reference_standard_id, next.calibrated_item_id
                ));
            }
        } else if link.reference_standard_id != record.terminal_reference_id {
            return Err(format!(
                "last calibration link terminates at {}, not declared terminal reference {}",
                link.reference_standard_id, record.terminal_reference_id
            ));
        }

        if let Some(reason) = validate_link(link, record.measurement_day, &record.measurand_unit)? {
            if link.uncertainty_kind == LinkUncertaintyKind::Unknown {
                unknown_uncertainty_link_count += 1;
            } else if matches!(
                link.uncertainty_kind,
                LinkUncertaintyKind::StandardUncertainty | LinkUncertaintyKind::ExpandedUncertainty
            ) {
                statistical_only_link_count += 1;
            }
            reasons.push(reason);
        }

        if link.uncertainty_kind == LinkUncertaintyKind::ValidatedAbsoluteBound {
            total_absolute_error_bound += link.uncertainty_value.unwrap_or(0.0);
            if !total_absolute_error_bound.is_finite() {
                return Err("aggregate calibration absolute-error bound overflowed".to_owned());
            }
        }

        link_receipts.push(LinkAuditReceipt {
            link_id: link.link_id.clone(),
            calibrated_item_id: link.calibrated_item_id.clone(),
            reference_standard_id: link.reference_standard_id.clone(),
            calibration_certificate_reference: link.calibration_certificate_reference.clone(),
            reference_standard_reference: link.reference_standard_reference.clone(),
            calibration_procedure_reference: link.calibration_procedure_reference.clone(),
            competence_evidence_reference: link.competence_evidence_reference.clone(),
            calibration_day: link.calibration_day,
            valid_from_day: link.valid_from_day,
            valid_until_day: link.valid_until_day,
            uncertainty_kind: link.uncertainty_kind,
            uncertainty_value: link.uncertainty_value,
            uncertainty_unit: link.uncertainty_unit.clone(),
            coverage_factor: link.coverage_factor,
            distribution_reference: link.distribution_reference.clone(),
            conversion_model_reference: link.conversion_model_reference.clone(),
            certificate_digest: link.certificate_digest.clone(),
        });
    }

    for (label, check) in [
        ("pre-use", &record.pre_use_check),
        ("post-use", &record.post_use_check),
    ] {
        if let Some(reason) = validate_assurance_check(
            check,
            label,
            record.measurement_day,
            label == "pre-use",
            &record.measurand_unit,
        )? {
            reasons.push(reason);
        }
    }

    let disposition = if !reasons.is_empty() || unknown_uncertainty_link_count > 0 {
        ChainDisposition::Unknown
    } else if statistical_only_link_count > 0 {
        ChainDisposition::StatisticalUncertaintyOnly
    } else {
        ChainDisposition::BoundQualified
    };

    let reported_bound = if disposition == ChainDisposition::BoundQualified {
        Some(total_absolute_error_bound)
    } else {
        None
    };

    Ok(CalibrationChainReport {
        disposition,
        sensor_serial: record.sensor_serial.clone(),
        measurement_day: record.measurement_day,
        measurand_unit: record.measurand_unit.clone(),
        terminal_reference_id: record.terminal_reference_id.clone(),
        chain_digest: record.chain_digest.clone(),
        link_count: record.links.len(),
        total_absolute_error_bound: reported_bound,
        statistical_only_link_count,
        unknown_uncertainty_link_count,
        pre_use_check_status: record.pre_use_check.status,
        post_use_check_status: record.post_use_check.status,
        link_receipts,
        reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assurance(day: i64) -> MeasurementAssuranceCheck {
        MeasurementAssuranceCheck {
            status: AssuranceStatus::WithinLimit,
            check_day: day,
            method_reference: "https://example.invalid/assurance/method".to_owned(),
            reference_artifact_id: "transfer-standard-01".to_owned(),
            raw_check_digest: format!("assurance-check-{day}"),
            observed_drift: Some(0.01),
            declared_drift_limit: Some(0.05),
            unit: "N m".to_owned(),
        }
    }

    fn link(
        id: &str,
        calibrated: &str,
        reference: &str,
        bound: f64,
    ) -> CalibrationLink {
        CalibrationLink {
            link_id: id.to_owned(),
            calibrated_item_id: calibrated.to_owned(),
            reference_standard_id: reference.to_owned(),
            calibration_certificate_reference: format!("https://example.invalid/certificate/{id}"),
            reference_standard_reference: format!("https://example.invalid/standard/{reference}"),
            calibration_procedure_reference: format!("https://example.invalid/procedure/{id}"),
            competence_evidence_reference: format!("https://example.invalid/competence/{id}"),
            calibration_day: 100,
            valid_from_day: 101,
            valid_until_day: 200,
            uncertainty_unit: "N m".to_owned(),
            uncertainty_kind: LinkUncertaintyKind::ValidatedAbsoluteBound,
            uncertainty_value: Some(bound),
            coverage_factor: None,
            distribution_reference: None,
            conversion_model_reference: format!("https://example.invalid/model/{id}"),
            certificate_digest: format!("certificate-digest-{id}"),
        }
    }

    fn record() -> CalibrationChainRecord {
        CalibrationChainRecord {
            sensor_serial: "torque-sensor-01".to_owned(),
            measurement_day: 150,
            measurand_unit: "N m".to_owned(),
            terminal_reference_id: "national-torque-reference".to_owned(),
            terminal_reference_source: "https://example.invalid/national-torque-reference".to_owned(),
            chain_digest: "ordered-calibration-chain-digest-v1".to_owned(),
            links: vec![
                link("link-1", "torque-sensor-01", "transfer-standard-01", 0.02),
                link("link-2", "transfer-standard-01", "national-torque-reference", 0.03),
            ],
            pre_use_check: assurance(149),
            post_use_check: assurance(151),
        }
    }

    #[test]
    fn chain_001_valid_ordered_chain_aggregates_absolute_bounds() {
        let report = audit_calibration_chain(&record()).unwrap();
        assert_eq!(report.disposition, ChainDisposition::BoundQualified);
        assert_eq!(report.link_count, 2);
        assert_eq!(report.total_absolute_error_bound, Some(0.05));
        assert_eq!(report.statistical_only_link_count, 0);
        assert_eq!(report.unknown_uncertainty_link_count, 0);
        assert_eq!(report.link_receipts[0].calibrated_item_id, "torque-sensor-01");
        assert_eq!(report.link_receipts[1].calibrated_item_id, "transfer-standard-01");
    }

    #[test]
    fn chain_002_broken_intermediate_reference_rejects() {
        let mut value = record();
        value.links[0].reference_standard_id = "missing-standard".to_owned();
        assert!(audit_calibration_chain(&value).is_err());
    }

    #[test]
    fn chain_003_sensor_serial_must_match_first_calibrated_item() {
        let mut value = record();
        value.links[0].calibrated_item_id = "different-sensor".to_owned();
        assert!(audit_calibration_chain(&value).is_err());
    }

    #[test]
    fn chain_004_terminal_reference_must_match_last_link() {
        let mut value = record();
        value.terminal_reference_id = "different-terminal".to_owned();
        assert!(audit_calibration_chain(&value).is_err());
    }

    #[test]
    fn chain_005_duplicate_or_self_referential_links_reject() {
        let mut duplicate = record();
        duplicate.links[1].link_id = duplicate.links[0].link_id.clone();
        assert!(audit_calibration_chain(&duplicate).is_err());

        let mut self_link = record();
        self_link.links[0].reference_standard_id = "torque-sensor-01".to_owned();
        assert!(audit_calibration_chain(&self_link).is_err());
    }

    #[test]
    fn chain_006_invalid_or_expired_calibration_window_is_unknown() {
        let mut expired = record();
        expired.links[0].valid_until_day = 149;
        let report = audit_calibration_chain(&expired).unwrap();
        assert_eq!(report.disposition, ChainDisposition::Unknown);
        assert!(report.total_absolute_error_bound.is_none());
    }

    #[test]
    fn chain_007_standard_uncertainty_is_not_promoted_to_a_hard_bound() {
        let mut value = record();
        value.links[0].uncertainty_kind = LinkUncertaintyKind::StandardUncertainty;
        value.links[0].uncertainty_value = Some(0.01);
        value.links[0].distribution_reference =
            Some("https://example.invalid/distribution/normal-model".to_owned());
        let report = audit_calibration_chain(&value).unwrap();
        assert_eq!(report.disposition, ChainDisposition::StatisticalUncertaintyOnly);
        assert_eq!(report.total_absolute_error_bound, None);
        assert_eq!(report.statistical_only_link_count, 1);
    }

    #[test]
    fn chain_008_expanded_uncertainty_requires_coverage_factor_and_distribution() {
        let mut missing_factor = record();
        missing_factor.links[0].uncertainty_kind = LinkUncertaintyKind::ExpandedUncertainty;
        missing_factor.links[0].uncertainty_value = Some(0.02);
        missing_factor.links[0].distribution_reference =
            Some("https://example.invalid/distribution/model".to_owned());
        assert!(audit_calibration_chain(&missing_factor).is_err());

        let mut valid_expanded = record();
        valid_expanded.links[0].uncertainty_kind = LinkUncertaintyKind::ExpandedUncertainty;
        valid_expanded.links[0].uncertainty_value = Some(0.02);
        valid_expanded.links[0].coverage_factor = Some(2.0);
        valid_expanded.links[0].distribution_reference =
            Some("https://example.invalid/distribution/model".to_owned());
        let report = audit_calibration_chain(&valid_expanded).unwrap();
        assert_eq!(report.disposition, ChainDisposition::StatisticalUncertaintyOnly);
    }

    #[test]
    fn chain_009_unknown_uncertainty_makes_chain_unknown() {
        let mut value = record();
        value.links[1].uncertainty_kind = LinkUncertaintyKind::Unknown;
        value.links[1].uncertainty_value = None;
        let report = audit_calibration_chain(&value).unwrap();
        assert_eq!(report.disposition, ChainDisposition::Unknown);
        assert_eq!(report.unknown_uncertainty_link_count, 1);
    }

    #[test]
    fn chain_010_unit_mismatch_in_link_bound_rejects() {
        let mut value = record();
        value.links[0].uncertainty_unit = "N".to_owned();
        assert!(audit_calibration_chain(&value).is_err());
    }

    #[test]
    fn chain_011_pre_use_assurance_must_precede_measurement() {
        let mut value = record();
        value.pre_use_check.check_day = 151;
        assert!(audit_calibration_chain(&value).is_err());
    }

    #[test]
    fn chain_012_post_use_assurance_must_follow_measurement() {
        let mut value = record();
        value.post_use_check.check_day = 149;
        assert!(audit_calibration_chain(&value).is_err());
    }

    #[test]
    fn chain_013_out_of_tolerance_assurance_is_unknown() {
        let mut value = record();
        value.post_use_check.status = AssuranceStatus::OutOfTolerance;
        value.post_use_check.observed_drift = Some(0.1);
        let report = audit_calibration_chain(&value).unwrap();
        assert_eq!(report.disposition, ChainDisposition::Unknown);
        assert!(report.reasons.iter().any(|reason| reason.contains("out of tolerance")));
    }

    #[test]
    fn chain_014_before_after_assurance_requires_drift_and_limit() {
        let mut value = record();
        value.pre_use_check.declared_drift_limit = None;
        let report = audit_calibration_chain(&value).unwrap();
        assert_eq!(report.disposition, ChainDisposition::Unknown);
    }

    #[test]
    fn chain_015_missing_reference_or_method_evidence_rejects() {
        let mut value = record();
        value.links[0].calibration_procedure_reference.clear();
        assert!(audit_calibration_chain(&value).is_err());

        let mut other = record();
        other.links[1].competence_evidence_reference.clear();
        assert!(audit_calibration_chain(&other).is_err());
    }

    #[test]
    fn chain_016_empty_chain_rejects() {
        let mut value = record();
        value.links.clear();
        assert!(audit_calibration_chain(&value).is_err());
    }

    #[test]
    fn chain_017_bound_aggregation_overflow_rejects() {
        let mut value = record();
        value.links[0].uncertainty_value = Some(f64::MAX);
        value.links[1].uncertainty_value = Some(f64::MAX);
        assert!(audit_calibration_chain(&value).is_err());
    }

    #[test]
    fn chain_018_receipt_preserves_each_link_reference_and_uncertainty_kind() {
        let mut value = record();
        value.links[0].uncertainty_kind = LinkUncertaintyKind::StandardUncertainty;
        value.links[0].uncertainty_value = Some(0.01);
        value.links[0].distribution_reference =
            Some("https://example.invalid/distribution/model".to_owned());
        let report = audit_calibration_chain(&value).unwrap();
        assert_eq!(report.link_receipts.len(), 2);
        assert_eq!(
            report.link_receipts[0].uncertainty_kind,
            LinkUncertaintyKind::StandardUncertainty
        );
        assert!(report.link_receipts[0].calibration_certificate_reference.contains("link-1"));
        assert!(report.link_receipts[0].conversion_model_reference.contains("link-1"));
    }
}
