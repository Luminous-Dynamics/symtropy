//! Joint tactile operating-envelope qualification without silent extrapolation.
//!
//! A calibration profile is represented as a union of explicitly qualified,
//! non-overlapping operating-domain cells. Every point must match one exact
//! cell across all required numeric and categorical dimensions. Being inside
//! each independent axis range is insufficient when the joint combination
//! was never validated.
//!
//! This is a synthetic evidence/admission contract. Receipt identifiers are
//! references only and must be resolved and independently verified by the
//! consuming evidence system. This module does not fit calibration curves,
//! estimate uncertainty, or establish physical traceability.

#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct NumericRange {
    pub minimum: f64,
    pub maximum: f64,
}

impl NumericRange {
    fn validate(self, name: &str) -> Result<(), String> {
        if !self.minimum.is_finite() || !self.maximum.is_finite() || self.minimum > self.maximum {
            return Err(format!("{name} range must have finite ordered bounds"));
        }
        Ok(())
    }

    fn contains(self, value: f64) -> bool {
        value.is_finite() && value >= self.minimum && value <= self.maximum
    }

    fn overlaps(self, other: Self) -> bool {
        self.minimum <= other.maximum && other.minimum <= self.maximum
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum CellQualificationStatus {
    Qualified,
    Failed,
    NotValidated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ReferenceIndependenceDisposition {
    IndependentlyCharacterized,
    SharedCommonCause,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct DomainCellEvidence {
    pub qualification_status: CellQualificationStatus,
    pub heldout_validation_receipt_id: String,
    pub uncertainty_characterization_receipt_id: String,
    pub model_form_adequacy_receipt_id: String,
    pub observability_qualification_receipt_id: String,
    pub independent_reference_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub reference_independence: ReferenceIndependenceDisposition,
    pub model_form_adequate: bool,
    pub observability_qualified: bool,
    pub uncertainty_qualified: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct OperatingDomainCell {
    pub cell_id: String,
    pub normal_force_n: NumericRange,
    pub tangential_force_n: NumericRange,
    pub temperature_c: NumericRange,
    pub humidity_percent: NumericRange,
    pub loading_rate_n_per_s: NumericRange,
    pub contact_velocity_mm_per_s: NumericRange,
    pub contact_radius_mm: NumericRange,
    pub sensor_identity_digest: String,
    pub sensorized_body_context_digest: String,
    pub readout_chain_digest: String,
    pub material_profile_digest: String,
    pub contact_geometry_digest: String,
    pub loading_history_state: String,
    pub evidence: DomainCellEvidence,
}

impl OperatingDomainCell {
    fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.cell_id.trim().is_empty() {
            errors.push("operating-domain cell ID is missing".to_owned());
        }
        for (name, range) in [
            ("normal_force_n", self.normal_force_n),
            ("tangential_force_n", self.tangential_force_n),
            ("temperature_c", self.temperature_c),
            ("humidity_percent", self.humidity_percent),
            ("loading_rate_n_per_s", self.loading_rate_n_per_s),
            ("contact_velocity_mm_per_s", self.contact_velocity_mm_per_s),
            ("contact_radius_mm", self.contact_radius_mm),
        ] {
            if let Err(error) = range.validate(name) {
                errors.push(error);
            }
        }
        if self.normal_force_n.minimum < 0.0 || self.tangential_force_n.minimum < 0.0 {
            errors.push("normal/tangential force magnitude ranges must be nonnegative".to_owned());
        }
        if self.humidity_percent.minimum < 0.0 || self.humidity_percent.maximum > 100.0 {
            errors.push("relative humidity range must lie within 0–100 percent RH".to_owned());
        }
        if self.contact_velocity_mm_per_s.minimum < 0.0 || self.contact_radius_mm.minimum < 0.0 {
            errors
                .push("contact velocity and contact radius ranges must be nonnegative".to_owned());
        }
        for (name, value) in [
            (
                "sensor_identity_digest",
                self.sensor_identity_digest.as_str(),
            ),
            (
                "sensorized_body_context_digest",
                self.sensorized_body_context_digest.as_str(),
            ),
            ("readout_chain_digest", self.readout_chain_digest.as_str()),
            (
                "material_profile_digest",
                self.material_profile_digest.as_str(),
            ),
            (
                "contact_geometry_digest",
                self.contact_geometry_digest.as_str(),
            ),
            ("loading_history_state", self.loading_history_state.as_str()),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("operating-domain cell field {name} is missing"));
            }
        }
        if self.evidence.qualification_status == CellQualificationStatus::Qualified {
            for (name, value) in [
                (
                    "heldout_validation_receipt_id",
                    self.evidence.heldout_validation_receipt_id.as_str(),
                ),
                (
                    "uncertainty_characterization_receipt_id",
                    self.evidence
                        .uncertainty_characterization_receipt_id
                        .as_str(),
                ),
                (
                    "model_form_adequacy_receipt_id",
                    self.evidence.model_form_adequacy_receipt_id.as_str(),
                ),
                (
                    "observability_qualification_receipt_id",
                    self.evidence
                        .observability_qualification_receipt_id
                        .as_str(),
                ),
                (
                    "independent_reference_receipt_id",
                    self.evidence.independent_reference_receipt_id.as_str(),
                ),
                (
                    "reference_ancestry_receipt_id",
                    self.evidence.reference_ancestry_receipt_id.as_str(),
                ),
                (
                    "common_cause_analysis_receipt_id",
                    self.evidence.common_cause_analysis_receipt_id.as_str(),
                ),
            ] {
                if value.trim().is_empty() {
                    errors.push(format!("qualified domain cell is missing {name}"));
                }
            }
        }
        errors
    }

    fn numeric_match(&self, point: &OperatingPoint) -> bool {
        self.normal_force_n
            .contains(point.normal_force_n.unwrap_or(f64::NAN))
            && self
                .tangential_force_n
                .contains(point.tangential_force_n.unwrap_or(f64::NAN))
            && self
                .temperature_c
                .contains(point.temperature_c.unwrap_or(f64::NAN))
            && self
                .humidity_percent
                .contains(point.humidity_percent.unwrap_or(f64::NAN))
            && self
                .loading_rate_n_per_s
                .contains(point.loading_rate_n_per_s.unwrap_or(f64::NAN))
            && self
                .contact_velocity_mm_per_s
                .contains(point.contact_velocity_mm_per_s.unwrap_or(f64::NAN))
            && self
                .contact_radius_mm
                .contains(point.contact_radius_mm.unwrap_or(f64::NAN))
    }

    fn context_match(&self, point: &OperatingPoint) -> bool {
        self.sensor_identity_digest == point.sensor_identity_digest
            && self.sensorized_body_context_digest == point.sensorized_body_context_digest
            && self.readout_chain_digest == point.readout_chain_digest
            && self.material_profile_digest == point.material_profile_digest
            && self.contact_geometry_digest == point.contact_geometry_digest
            && self.loading_history_state == point.loading_history_state
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.sensor_identity_digest == other.sensor_identity_digest
            && self.sensorized_body_context_digest == other.sensorized_body_context_digest
            && self.readout_chain_digest == other.readout_chain_digest
            && self.material_profile_digest == other.material_profile_digest
            && self.contact_geometry_digest == other.contact_geometry_digest
            && self.loading_history_state == other.loading_history_state
            && self.normal_force_n.overlaps(other.normal_force_n)
            && self.tangential_force_n.overlaps(other.tangential_force_n)
            && self.temperature_c.overlaps(other.temperature_c)
            && self.humidity_percent.overlaps(other.humidity_percent)
            && self
                .loading_rate_n_per_s
                .overlaps(other.loading_rate_n_per_s)
            && self
                .contact_velocity_mm_per_s
                .overlaps(other.contact_velocity_mm_per_s)
            && self.contact_radius_mm.overlaps(other.contact_radius_mm)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct QualifiedOperatingEnvelope {
    pub envelope_id: String,
    pub calibration_record_id: String,
    pub sensor_identity_digest: String,
    pub model_form_digest: String,
    pub reference_chain_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub cells: Vec<OperatingDomainCell>,
}

impl QualifiedOperatingEnvelope {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (name, value) in [
            ("envelope_id", self.envelope_id.as_str()),
            ("calibration_record_id", self.calibration_record_id.as_str()),
            (
                "sensor_identity_digest",
                self.sensor_identity_digest.as_str(),
            ),
            ("model_form_digest", self.model_form_digest.as_str()),
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
                errors.push(format!("operating-envelope field {name} is missing"));
            }
        }
        if self.cells.is_empty() {
            errors.push("operating envelope contains no domain cells".to_owned());
        }

        for cell in &self.cells {
            errors.extend(
                cell.validate()
                    .into_iter()
                    .map(|reason| format!("cell {}: {reason}", cell.cell_id)),
            );
            if cell.sensor_identity_digest != self.sensor_identity_digest {
                errors.push(format!(
                    "cell {} sensor identity differs from envelope sensor identity",
                    cell.cell_id
                ));
            }
        }

        for left in 0..self.cells.len() {
            for right in (left + 1)..self.cells.len() {
                if self.cells[left].cell_id == self.cells[right].cell_id {
                    errors.push(format!(
                        "duplicate operating-domain cell ID {}",
                        self.cells[left].cell_id
                    ));
                }
                if self.cells[left].overlaps(&self.cells[right]) {
                    errors.push(format!(
                        "operating-domain cells {} and {} overlap for the same physical context",
                        self.cells[left].cell_id, self.cells[right].cell_id
                    ));
                }
            }
        }
        errors
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperatingPoint {
    pub normal_force_n: Option<f64>,
    pub tangential_force_n: Option<f64>,
    pub temperature_c: Option<f64>,
    pub humidity_percent: Option<f64>,
    pub loading_rate_n_per_s: Option<f64>,
    pub contact_velocity_mm_per_s: Option<f64>,
    pub contact_radius_mm: Option<f64>,
    pub sensor_identity_digest: String,
    pub sensorized_body_context_digest: String,
    pub readout_chain_digest: String,
    pub material_profile_digest: String,
    pub contact_geometry_digest: String,
    pub loading_history_state: String,
}

impl OperatingPoint {
    fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (name, value) in [
            ("normal_force_n", self.normal_force_n),
            ("tangential_force_n", self.tangential_force_n),
            ("temperature_c", self.temperature_c),
            ("humidity_percent", self.humidity_percent),
            ("loading_rate_n_per_s", self.loading_rate_n_per_s),
            ("contact_velocity_mm_per_s", self.contact_velocity_mm_per_s),
            ("contact_radius_mm", self.contact_radius_mm),
        ] {
            match value {
                None => errors.push(format!("operating point is missing required state {name}")),
                Some(value) if !value.is_finite() => {
                    errors.push(format!("operating point state {name} is non-finite"));
                }
                Some(_) => {}
            }
        }
        for (name, value) in [
            (
                "sensor_identity_digest",
                self.sensor_identity_digest.as_str(),
            ),
            (
                "sensorized_body_context_digest",
                self.sensorized_body_context_digest.as_str(),
            ),
            ("readout_chain_digest", self.readout_chain_digest.as_str()),
            (
                "material_profile_digest",
                self.material_profile_digest.as_str(),
            ),
            (
                "contact_geometry_digest",
                self.contact_geometry_digest.as_str(),
            ),
            ("loading_history_state", self.loading_history_state.as_str()),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("operating point is missing context field {name}"));
            }
        }
        errors
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum OperatingEnvelopeDisposition {
    QualifiedInsideCell,
    EnvelopeInvalid,
    OutsideJointDomain,
    MissingRequiredState,
    PhysicalContextMismatch,
    OverlappingOrAmbiguousCells,
    CellNotValidated,
    ReferenceChainUnresolved,
    ModelFormUnresolved,
    ObservabilityNotQualified,
    UncertaintyNotQualified,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct OperatingEnvelopeAssessment {
    pub disposition: OperatingEnvelopeDisposition,
    pub matched_cell_id: Option<String>,
    pub reasons: Vec<String>,
    pub envelope_id: String,
    pub calibration_record_id: String,
    pub claim_class: &'static str,
}

/// Assess one operating point against a union of exact, jointly qualified cells.
/// No extrapolation or interpolation is performed. A point is qualified only
/// when exactly one cell matches every required numeric and context dimension.
pub fn assess_operating_point(
    envelope: &QualifiedOperatingEnvelope,
    point: &OperatingPoint,
) -> OperatingEnvelopeAssessment {
    let invalid_envelope = envelope.validate();
    if !invalid_envelope.is_empty() {
        let overlap_only = invalid_envelope
            .iter()
            .all(|reason| reason.contains("operating-domain cells") && reason.contains("overlap"));
        return OperatingEnvelopeAssessment {
            disposition: if overlap_only {
                OperatingEnvelopeDisposition::OverlappingOrAmbiguousCells
            } else {
                OperatingEnvelopeDisposition::EnvelopeInvalid
            },
            matched_cell_id: None,
            reasons: invalid_envelope,
            envelope_id: envelope.envelope_id.clone(),
            calibration_record_id: envelope.calibration_record_id.clone(),
            claim_class: "synthetic_joint_operating_envelope_contract_only",
        };
    }

    let point_errors = point.validate();
    if !point_errors.is_empty() {
        let missing = point_errors.iter().any(|reason| {
            reason.contains("missing required state") || reason.contains("missing context field")
        });
        return OperatingEnvelopeAssessment {
            disposition: if missing {
                OperatingEnvelopeDisposition::MissingRequiredState
            } else {
                OperatingEnvelopeDisposition::EnvelopeInvalid
            },
            matched_cell_id: None,
            reasons: point_errors,
            envelope_id: envelope.envelope_id.clone(),
            calibration_record_id: envelope.calibration_record_id.clone(),
            claim_class: "synthetic_joint_operating_envelope_contract_only",
        };
    }

    let numeric_matches: Vec<_> = envelope
        .cells
        .iter()
        .filter(|cell| cell.numeric_match(point))
        .collect();
    if numeric_matches.is_empty() {
        return OperatingEnvelopeAssessment {
            disposition: OperatingEnvelopeDisposition::OutsideJointDomain,
            matched_cell_id: None,
            reasons: vec![
                "operating point is outside every jointly validated numeric domain cell".to_owned(),
            ],
            envelope_id: envelope.envelope_id.clone(),
            calibration_record_id: envelope.calibration_record_id.clone(),
            claim_class: "synthetic_joint_operating_envelope_contract_only",
        };
    }

    let context_matches: Vec<_> = numeric_matches
        .into_iter()
        .filter(|cell| cell.context_match(point))
        .collect();
    if context_matches.is_empty() {
        return OperatingEnvelopeAssessment {
            disposition: OperatingEnvelopeDisposition::PhysicalContextMismatch,
            matched_cell_id: None,
            reasons: vec![
                "numeric state is covered, but the sensorized-body/readout/material/geometry/history context is not".to_owned(),
            ],
            envelope_id: envelope.envelope_id.clone(),
            calibration_record_id: envelope.calibration_record_id.clone(),
            claim_class: "synthetic_joint_operating_envelope_contract_only",
        };
    }
    if context_matches.len() > 1 {
        return OperatingEnvelopeAssessment {
            disposition: OperatingEnvelopeDisposition::OverlappingOrAmbiguousCells,
            matched_cell_id: None,
            reasons: vec!["more than one operating-domain cell matches this point".to_owned()],
            envelope_id: envelope.envelope_id.clone(),
            calibration_record_id: envelope.calibration_record_id.clone(),
            claim_class: "synthetic_joint_operating_envelope_contract_only",
        };
    }

    let cell = context_matches[0];
    let mut reasons = Vec::new();
    let disposition = match cell.evidence.qualification_status {
        CellQualificationStatus::Failed | CellQualificationStatus::NotValidated => {
            reasons.push(format!("domain cell {} is not qualified", cell.cell_id));
            OperatingEnvelopeDisposition::CellNotValidated
        }
        CellQualificationStatus::Qualified => {
            if cell.evidence.reference_independence
                != ReferenceIndependenceDisposition::IndependentlyCharacterized
            {
                reasons
                    .push("cell reference ancestry/common-cause evidence is unresolved".to_owned());
                OperatingEnvelopeDisposition::ReferenceChainUnresolved
            } else if !cell.evidence.model_form_adequate {
                reasons.push("cell model-form adequacy is unresolved".to_owned());
                OperatingEnvelopeDisposition::ModelFormUnresolved
            } else if !cell.evidence.observability_qualified {
                reasons.push("cell operating range/observability is not qualified".to_owned());
                OperatingEnvelopeDisposition::ObservabilityNotQualified
            } else if !cell.evidence.uncertainty_qualified {
                reasons.push("cell measurand uncertainty is not qualified".to_owned());
                OperatingEnvelopeDisposition::UncertaintyNotQualified
            } else {
                OperatingEnvelopeDisposition::QualifiedInsideCell
            }
        }
    };

    OperatingEnvelopeAssessment {
        disposition,
        matched_cell_id: Some(cell.cell_id.clone()),
        reasons,
        envelope_id: envelope.envelope_id.clone(),
        calibration_record_id: envelope.calibration_record_id.clone(),
        claim_class: "synthetic_joint_operating_envelope_contract_only",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_evidence() -> DomainCellEvidence {
        DomainCellEvidence {
            qualification_status: CellQualificationStatus::Qualified,
            heldout_validation_receipt_id: "receipt:heldout".to_owned(),
            uncertainty_characterization_receipt_id: "receipt:uncertainty".to_owned(),
            model_form_adequacy_receipt_id: "receipt:model-form".to_owned(),
            observability_qualification_receipt_id: "receipt:observability".to_owned(),
            independent_reference_receipt_id: "receipt:reference".to_owned(),
            reference_ancestry_receipt_id: "receipt:ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:common-cause".to_owned(),
            reference_independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
            model_form_adequate: true,
            observability_qualified: true,
            uncertainty_qualified: true,
        }
    }

    fn cell(
        id: &str,
        normal_force_n: NumericRange,
        temperature_c: NumericRange,
    ) -> OperatingDomainCell {
        OperatingDomainCell {
            cell_id: id.to_owned(),
            normal_force_n,
            tangential_force_n: NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            temperature_c,
            humidity_percent: NumericRange {
                minimum: 10.0,
                maximum: 90.0,
            },
            loading_rate_n_per_s: NumericRange {
                minimum: 0.0,
                maximum: 20.0,
            },
            contact_velocity_mm_per_s: NumericRange {
                minimum: 0.0,
                maximum: 100.0,
            },
            contact_radius_mm: NumericRange {
                minimum: 1.0,
                maximum: 20.0,
            },
            sensor_identity_digest: "sha256:sensor-article".to_owned(),
            sensorized_body_context_digest: "sha256:body".to_owned(),
            readout_chain_digest: "sha256:readout".to_owned(),
            material_profile_digest: "sha256:material".to_owned(),
            contact_geometry_digest: "sha256:geometry".to_owned(),
            loading_history_state: "preconditioned-cycle-10".to_owned(),
            evidence: valid_evidence(),
        }
    }

    fn envelope(cells: Vec<OperatingDomainCell>) -> QualifiedOperatingEnvelope {
        QualifiedOperatingEnvelope {
            envelope_id: "envelope:fixture-01".to_owned(),
            calibration_record_id: "calibration:fixture-01".to_owned(),
            sensor_identity_digest: "sha256:sensor-article".to_owned(),
            model_form_digest: "sha256:model-form".to_owned(),
            reference_chain_receipt_id: "receipt:chain".to_owned(),
            reference_ancestry_receipt_id: "receipt:ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:cca".to_owned(),
            cells,
        }
    }

    fn point() -> OperatingPoint {
        OperatingPoint {
            normal_force_n: Some(4.0),
            tangential_force_n: Some(2.0),
            temperature_c: Some(25.0),
            humidity_percent: Some(50.0),
            loading_rate_n_per_s: Some(4.0),
            contact_velocity_mm_per_s: Some(20.0),
            contact_radius_mm: Some(5.0),
            sensor_identity_digest: "sha256:sensor-article".to_owned(),
            sensorized_body_context_digest: "sha256:body".to_owned(),
            readout_chain_digest: "sha256:readout".to_owned(),
            material_profile_digest: "sha256:material".to_owned(),
            contact_geometry_digest: "sha256:geometry".to_owned(),
            loading_history_state: "preconditioned-cycle-10".to_owned(),
        }
    }

    #[test]
    fn point_inside_exact_qualified_joint_cell_is_accepted() {
        let cell = cell(
            "cell-a",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        let assessment = assess_operating_point(&envelope(vec![cell]), &point());
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::QualifiedInsideCell
        );
        assert_eq!(assessment.matched_cell_id.as_deref(), Some("cell-a"));
    }

    #[test]
    fn independently_in_range_axes_do_not_qualify_an_untested_joint_combination() {
        let lower_force_high_temp = cell(
            "low-force-hot",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 30.0,
                maximum: 40.0,
            },
        );
        let high_force_low_temp = cell(
            "high-force-cool",
            NumericRange {
                minimum: 6.0,
                maximum: 10.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 29.0,
            },
        );
        let mut candidate = point();
        candidate.normal_force_n = Some(8.0);
        candidate.temperature_c = Some(35.0);
        let assessment = assess_operating_point(
            &envelope(vec![lower_force_high_temp, high_force_low_temp]),
            &candidate,
        );
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::OutsideJointDomain
        );
    }

    #[test]
    fn exact_numeric_bounds_are_inclusive_and_context_mismatch_is_separate() {
        let cell = cell(
            "cell-boundary",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        let mut boundary = point();
        boundary.normal_force_n = Some(5.0);
        boundary.temperature_c = Some(30.0);
        let assessment = assess_operating_point(&envelope(vec![cell.clone()]), &boundary);
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::QualifiedInsideCell
        );

        let mut changed_context = point();
        changed_context.sensorized_body_context_digest = "sha256:new-host".to_owned();
        let assessment = assess_operating_point(&envelope(vec![cell]), &changed_context);
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::PhysicalContextMismatch
        );
    }

    #[test]
    fn missing_dimension_and_overlapping_cells_fail_closed() {
        let cell_a = cell(
            "cell-a",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        let mut missing = point();
        missing.humidity_percent = None;
        let assessment = assess_operating_point(&envelope(vec![cell_a.clone()]), &missing);
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::MissingRequiredState
        );

        let cell_b = cell(
            "cell-b",
            NumericRange {
                minimum: 4.0,
                maximum: 7.0,
            },
            NumericRange {
                minimum: 25.0,
                maximum: 35.0,
            },
        );
        let assessment = assess_operating_point(&envelope(vec![cell_a, cell_b]), &point());
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::OverlappingOrAmbiguousCells
        );
    }

    #[test]
    fn cells_cannot_cross_sensor_article_identity() {
        let mut changed_sensor = cell(
            "cell-new-sensor",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        changed_sensor.sensor_identity_digest = "sha256:replacement-sensor".to_owned();
        let assessment = assess_operating_point(&envelope(vec![changed_sensor]), &point());
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::EnvelopeInvalid
        );
        assert!(assessment.reasons.iter().any(|reason| {
            reason.contains("sensor identity differs from envelope sensor identity")
        }));
    }

    #[test]
    fn physically_invalid_humidity_and_force_ranges_fail_closed() {
        let mut invalid_humidity = cell(
            "bad-humidity",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        invalid_humidity.humidity_percent = NumericRange {
            minimum: 10.0,
            maximum: 120.0,
        };
        let assessment = assess_operating_point(&envelope(vec![invalid_humidity]), &point());
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::EnvelopeInvalid
        );

        let mut negative_force = cell(
            "negative-force",
            NumericRange {
                minimum: -1.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        negative_force.evidence.qualification_status = CellQualificationStatus::NotValidated;
        let assessment = assess_operating_point(&envelope(vec![negative_force]), &point());
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::EnvelopeInvalid
        );
    }

    #[test]
    fn reference_model_observability_and_uncertainty_gates_remain_independent() {
        let mut unresolved_reference = cell(
            "cell-a",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        unresolved_reference.evidence.reference_independence =
            ReferenceIndependenceDisposition::SharedCommonCause;
        assert_eq!(
            assess_operating_point(&envelope(vec![unresolved_reference]), &point()).disposition,
            OperatingEnvelopeDisposition::ReferenceChainUnresolved
        );

        let mut model_unresolved = cell(
            "cell-a",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        model_unresolved.evidence.model_form_adequate = false;
        assert_eq!(
            assess_operating_point(&envelope(vec![model_unresolved]), &point()).disposition,
            OperatingEnvelopeDisposition::ModelFormUnresolved
        );

        let mut unobservable = cell(
            "cell-a",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        unobservable.evidence.observability_qualified = false;
        assert_eq!(
            assess_operating_point(&envelope(vec![unobservable]), &point()).disposition,
            OperatingEnvelopeDisposition::ObservabilityNotQualified
        );

        let mut uncertainty_missing = cell(
            "cell-a",
            NumericRange {
                minimum: 0.0,
                maximum: 5.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        uncertainty_missing.evidence.uncertainty_qualified = false;
        assert_eq!(
            assess_operating_point(&envelope(vec![uncertainty_missing]), &point()).disposition,
            OperatingEnvelopeDisposition::UncertaintyNotQualified
        );
    }

    #[test]
    fn nonfinite_or_invalid_cells_cannot_emit_qualified_measurements() {
        let mut invalid = cell(
            "cell-a",
            NumericRange {
                minimum: 5.0,
                maximum: 0.0,
            },
            NumericRange {
                minimum: 20.0,
                maximum: 30.0,
            },
        );
        invalid.evidence.heldout_validation_receipt_id.clear();
        let assessment = assess_operating_point(&envelope(vec![invalid]), &point());
        assert_eq!(
            assessment.disposition,
            OperatingEnvelopeDisposition::EnvelopeInvalid
        );
    }
}
