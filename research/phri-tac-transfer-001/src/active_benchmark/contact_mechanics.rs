//! Uncertainty-aware local Coulomb-cone falsification for tactile traction fields.
//!
//! This is a checker, not an optimizer. It preserves the input field and
//! classifies each taxel as robustly inside, definitely outside or ambiguous
//! under an independently qualified friction-coefficient interval.
//!
//! Scope is dry, non-adhesive, unilateral contact only. Adhesion, suction,
//! active fibrillar contact and other non-Coulomb regimes require separate
//! constitutive models. A friction cone is a physical assumption, not a sensor
//! observation and therefore never increases native-data identifiability.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ContactModelFamily {
    DryUnilateralCoulomb,
    AdhesiveOrOtherUnsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FrictionContactRegime {
    Static,
    Kinetic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ReferenceIndependenceDisposition {
    IndependentlyCharacterized,
    SharedCommonCause,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TaxelTractionEstimate {
    pub taxel_id: String,
    /// Compressive normal pressure, Pa; positive is compressive.
    pub normal_pressure_pa: f64,
    pub tangent_u_pa: f64,
    pub tangent_v_pa: f64,
    /// One-standard-deviation uncertainty for each local traction component, Pa.
    pub normal_standard_uncertainty_pa: f64,
    pub tangent_u_standard_uncertainty_pa: f64,
    pub tangent_v_standard_uncertainty_pa: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactFieldRequest {
    pub model_family: ContactModelFamily,
    pub contact_regime: FrictionContactRegime,
    pub surface_pair_digest: String,
    pub skin_material_digest: String,
    pub sensorized_body_context_digest: String,
    pub contact_geometry_digest: String,
    pub operating_envelope_cell_id: String,
    pub taxels: Vec<TaxelTractionEstimate>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct FrictionQualificationEvidence {
    pub friction_model_digest: String,
    pub contact_regime: FrictionContactRegime,
    /// Qualified interval for the local Coulomb coefficient; dimensionless.
    pub coefficient_lower: f64,
    pub coefficient_upper: f64,
    pub surface_pair_digest: String,
    pub skin_material_digest: String,
    pub sensorized_body_context_digest: String,
    pub contact_geometry_digest: String,
    pub operating_envelope_cell_id: String,
    pub reference_instrument_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub heldout_validation_receipt_id: String,
    pub verifier_receipt_id: String,
    pub reference_independence: ReferenceIndependenceDisposition,
    pub qualification_valid: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TaxelContactDisposition {
    RobustlyInsideFrictionCone,
    DefinitelyOutsideFrictionCone,
    NormalPressureNotCompressive,
    IndeterminateGivenUncertainty,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TaxelContactAssessment {
    pub taxel_id: String,
    pub disposition: TaxelContactDisposition,
    pub normal_pressure_lower_bound_pa: f64,
    pub normal_pressure_upper_bound_pa: f64,
    pub shear_magnitude_lower_bound_pa: f64,
    pub shear_magnitude_upper_bound_pa: f64,
    pub admissible_shear_upper_bound_pa: f64,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ContactFieldDisposition {
    AllTaxelsRobustlyAdmissible,
    ContainsDefiniteViolations,
    ContainsIndeterminateTaxels,
    FrictionEvidenceUnresolved,
    MaterialOrContextMismatch,
    UnsupportedContactModel,
    InvalidField,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ContactFieldAssessment {
    pub disposition: ContactFieldDisposition,
    pub per_taxel: Vec<TaxelContactAssessment>,
    pub robustly_inside_count: usize,
    pub definitely_outside_count: usize,
    pub noncompressive_count: usize,
    pub indeterminate_count: usize,
    pub confidence_multiplier: f64,
    pub coefficient_interval: Option<[f64; 2]>,
    pub reasons: Vec<String>,
    pub claim_class: &'static str,
}

fn nonempty_fields(fields: &[(&str, &str)]) -> Vec<String> {
    fields
        .iter()
        .filter(|(_, value)| value.trim().is_empty())
        .map(|(name, _)| format!("required friction evidence field {name} is missing"))
        .collect()
}

fn validate_evidence(
    request: &ContactFieldRequest,
    evidence: &FrictionQualificationEvidence,
) -> Result<(), (ContactFieldDisposition, Vec<String>)> {
    let missing = nonempty_fields(&[
        ("friction_model_digest", &evidence.friction_model_digest),
        ("surface_pair_digest", &evidence.surface_pair_digest),
        ("skin_material_digest", &evidence.skin_material_digest),
        ("sensorized_body_context_digest", &evidence.sensorized_body_context_digest),
        ("contact_geometry_digest", &evidence.contact_geometry_digest),
        ("operating_envelope_cell_id", &evidence.operating_envelope_cell_id),
        ("reference_instrument_receipt_id", &evidence.reference_instrument_receipt_id),
        ("reference_ancestry_receipt_id", &evidence.reference_ancestry_receipt_id),
        ("common_cause_analysis_receipt_id", &evidence.common_cause_analysis_receipt_id),
        ("heldout_validation_receipt_id", &evidence.heldout_validation_receipt_id),
        ("verifier_receipt_id", &evidence.verifier_receipt_id),
    ]);
    if !missing.is_empty()
        || !evidence.coefficient_lower.is_finite()
        || !evidence.coefficient_upper.is_finite()
        || evidence.coefficient_lower < 0.0
        || evidence.coefficient_lower > evidence.coefficient_upper
        || !evidence.qualification_valid
        || evidence.reference_independence != ReferenceIndependenceDisposition::IndependentlyCharacterized
    {
        let mut reasons = missing;
        if !evidence.coefficient_lower.is_finite()
            || !evidence.coefficient_upper.is_finite()
            || evidence.coefficient_lower < 0.0
            || evidence.coefficient_lower > evidence.coefficient_upper
        {
            reasons.push("friction coefficient interval is invalid".to_owned());
        }
        if !evidence.qualification_valid {
            reasons.push("friction coefficient evidence is not qualified".to_owned());
        }
        if evidence.reference_independence != ReferenceIndependenceDisposition::IndependentlyCharacterized {
            reasons.push("friction reference ancestry/common-cause disposition is unresolved".to_owned());
        }
        return Err((ContactFieldDisposition::FrictionEvidenceUnresolved, reasons));
    }

    if evidence.contact_regime != request.contact_regime
        || evidence.surface_pair_digest != request.surface_pair_digest
        || evidence.skin_material_digest != request.skin_material_digest
        || evidence.sensorized_body_context_digest != request.sensorized_body_context_digest
        || evidence.contact_geometry_digest != request.contact_geometry_digest
        || evidence.operating_envelope_cell_id != request.operating_envelope_cell_id
    {
        return Err((
            ContactFieldDisposition::MaterialOrContextMismatch,
            vec![
                "friction evidence does not bind the exact material pair, skin, body, geometry, regime and operating-domain cell".to_owned(),
            ],
        ));
    }
    Ok(())
}

fn classify_taxel(
    estimate: &TaxelTractionEstimate,
    mu_lower: f64,
    mu_upper: f64,
    confidence_multiplier: f64,
) -> TaxelContactAssessment {
    let normal_lower = estimate.normal_pressure_pa
        - confidence_multiplier * estimate.normal_standard_uncertainty_pa;
    let normal_upper = estimate.normal_pressure_pa
        + confidence_multiplier * estimate.normal_standard_uncertainty_pa;
    let shear_mean = estimate.tangent_u_pa.hypot(estimate.tangent_v_pa);
    let shear_uncertainty = estimate.tangent_u_standard_uncertainty_pa
        .hypot(estimate.tangent_v_standard_uncertainty_pa);
    let shear_lower = (shear_mean - confidence_multiplier * shear_uncertainty).max(0.0);
    let shear_upper = shear_mean + confidence_multiplier * shear_uncertainty;
    let admissible_upper = mu_lower * normal_lower.max(0.0);
    let (disposition, reason) = if normal_upper <= 0.0 {
        (
            TaxelContactDisposition::NormalPressureNotCompressive,
            "normal-pressure upper bound is nonpositive under the declared non-adhesive model",
        )
    } else if normal_lower > 0.0 && shear_upper <= mu_lower * normal_lower {
        (
            TaxelContactDisposition::RobustlyInsideFrictionCone,
            "upper shear bound is inside the lower qualified friction coefficient times lower pressure bound",
        )
    } else if shear_lower > mu_upper * normal_upper.max(0.0) {
        (
            TaxelContactDisposition::DefinitelyOutsideFrictionCone,
            "lower shear bound exceeds the upper qualified friction coefficient times upper pressure bound",
        )
    } else {
        (
            TaxelContactDisposition::IndeterminateGivenUncertainty,
            "uncertainty interval overlaps the friction-cone boundary or includes zero compression",
        )
    };

    TaxelContactAssessment {
        taxel_id: estimate.taxel_id.clone(),
        disposition,
        normal_pressure_lower_bound_pa: normal_lower,
        normal_pressure_upper_bound_pa: normal_upper,
        shear_magnitude_lower_bound_pa: shear_lower,
        shear_magnitude_upper_bound_pa: shear_upper,
        admissible_shear_upper_bound_pa: admissible_upper,
        reason: reason.to_owned(),
    }
}

/// Screen a local traction field against a qualified material/context-specific
/// Coulomb coefficient interval. The estimate is never modified or projected.
pub fn assess_contact_field(
    request: &ContactFieldRequest,
    evidence: &FrictionQualificationEvidence,
    confidence_multiplier: f64,
) -> ContactFieldAssessment {
    let empty = |disposition, reasons| ContactFieldAssessment {
        disposition,
        per_taxel: Vec::new(),
        robustly_inside_count: 0,
        definitely_outside_count: 0,
        noncompressive_count: 0,
        indeterminate_count: 0,
        confidence_multiplier,
        coefficient_interval: None,
        reasons,
        claim_class: "synthetic_local_coulomb_admissibility_only",
    };

    if request.model_family != ContactModelFamily::DryUnilateralCoulomb {
        return empty(
            ContactFieldDisposition::UnsupportedContactModel,
            vec!["v0.1 checker only admits dry non-adhesive unilateral Coulomb contact".to_owned()],
        );
    }
    if !confidence_multiplier.is_finite() || confidence_multiplier <= 0.0 {
        return empty(
            ContactFieldDisposition::InvalidField,
            vec!["confidence multiplier must be finite and positive".to_owned()],
        );
    }
    if let Err((disposition, reasons)) = validate_evidence(request, evidence) {
        return empty(disposition, reasons);
    }
    if request.taxels.is_empty() {
        return empty(
            ContactFieldDisposition::InvalidField,
            vec!["contact field contains no taxels".to_owned()],
        );
    }

    let mut seen = BTreeSet::new();
    for taxel in &request.taxels {
        if taxel.taxel_id.trim().is_empty()
            || !seen.insert(&taxel.taxel_id)
            || [
                taxel.normal_pressure_pa,
                taxel.tangent_u_pa,
                taxel.tangent_v_pa,
                taxel.normal_standard_uncertainty_pa,
                taxel.tangent_u_standard_uncertainty_pa,
                taxel.tangent_v_standard_uncertainty_pa,
            ]
            .iter()
            .any(|value| !value.is_finite())
            || taxel.normal_standard_uncertainty_pa < 0.0
            || taxel.tangent_u_standard_uncertainty_pa < 0.0
            || taxel.tangent_v_standard_uncertainty_pa < 0.0
        {
            return empty(
                ContactFieldDisposition::InvalidField,
                vec!["taxel IDs must be unique/nonempty and traction estimates and standard uncertainties finite/nonnegative".to_owned()],
            );
        }
    }

    let per_taxel: Vec<_> = request
        .taxels
        .iter()
        .map(|taxel| {
            classify_taxel(
                taxel,
                evidence.coefficient_lower,
                evidence.coefficient_upper,
                confidence_multiplier,
            )
        })
        .collect();

    let robustly_inside_count = per_taxel
        .iter()
        .filter(|item| item.disposition == TaxelContactDisposition::RobustlyInsideFrictionCone)
        .count();
    let definitely_outside_count = per_taxel
        .iter()
        .filter(|item| item.disposition == TaxelContactDisposition::DefinitelyOutsideFrictionCone)
        .count();
    let noncompressive_count = per_taxel
        .iter()
        .filter(|item| item.disposition == TaxelContactDisposition::NormalPressureNotCompressive)
        .count();
    let indeterminate_count = per_taxel
        .iter()
        .filter(|item| item.disposition == TaxelContactDisposition::IndeterminateGivenUncertainty)
        .count();

    let disposition = if definitely_outside_count > 0 || noncompressive_count > 0 {
        ContactFieldDisposition::ContainsDefiniteViolations
    } else if indeterminate_count > 0 {
        ContactFieldDisposition::ContainsIndeterminateTaxels
    } else {
        ContactFieldDisposition::AllTaxelsRobustlyAdmissible
    };
    ContactFieldAssessment {
        disposition,
        per_taxel,
        robustly_inside_count,
        definitely_outside_count,
        noncompressive_count,
        indeterminate_count,
        confidence_multiplier,
        coefficient_interval: Some([evidence.coefficient_lower, evidence.coefficient_upper]),
        reasons: Vec::new(),
        claim_class: "synthetic_local_coulomb_admissibility_only",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_benchmark::wrench_consistency::{
        assess_wrench_consistency, build_wrench_operator, integrate_tactile_field,
        ReferenceIndependenceDisposition as WrenchReferenceIndependence,
        TaxelGeometry, TaxelGeometrySet, Vec3, WrenchObservation, WrenchReferenceEvidence,
    };

    fn evidence() -> FrictionQualificationEvidence {
        FrictionQualificationEvidence {
            friction_model_digest: "sha256:dry-coulomb-model".to_owned(),
            contact_regime: FrictionContactRegime::Static,
            coefficient_lower: 0.4,
            coefficient_upper: 0.6,
            surface_pair_digest: "sha256:steel-rubber-pair".to_owned(),
            skin_material_digest: "sha256:skin-lot-17".to_owned(),
            sensorized_body_context_digest: "sha256:body-01".to_owned(),
            contact_geometry_digest: "sha256:geometry-01".to_owned(),
            operating_envelope_cell_id: "cell:normal-0-10n-20-30c".to_owned(),
            reference_instrument_receipt_id: "receipt:friction-reference".to_owned(),
            reference_ancestry_receipt_id: "receipt:friction-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:friction-cca".to_owned(),
            heldout_validation_receipt_id: "receipt:friction-heldout".to_owned(),
            verifier_receipt_id: "receipt:friction-verifier".to_owned(),
            reference_independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
            qualification_valid: true,
        }
    }

    fn request(taxels: Vec<TaxelTractionEstimate>) -> ContactFieldRequest {
        ContactFieldRequest {
            model_family: ContactModelFamily::DryUnilateralCoulomb,
            contact_regime: FrictionContactRegime::Static,
            surface_pair_digest: "sha256:steel-rubber-pair".to_owned(),
            skin_material_digest: "sha256:skin-lot-17".to_owned(),
            sensorized_body_context_digest: "sha256:body-01".to_owned(),
            contact_geometry_digest: "sha256:geometry-01".to_owned(),
            operating_envelope_cell_id: "cell:normal-0-10n-20-30c".to_owned(),
            taxels,
        }
    }

    fn taxel(
        id: &str,
        pressure: f64,
        tangent_u: f64,
        tangent_v: f64,
        sigma_pressure: f64,
        sigma_u: f64,
        sigma_v: f64,
    ) -> TaxelTractionEstimate {
        TaxelTractionEstimate {
            taxel_id: id.to_owned(),
            normal_pressure_pa: pressure,
            tangent_u_pa: tangent_u,
            tangent_v_pa: tangent_v,
            normal_standard_uncertainty_pa: sigma_pressure,
            tangent_u_standard_uncertainty_pa: sigma_u,
            tangent_v_standard_uncertainty_pa: sigma_v,
        }
    }

    #[test]
    fn low_shear_compressive_field_is_robustly_inside() {
        let assessment = assess_contact_field(
            &request(vec![
                taxel("t0", 100.0, 20.0, 10.0, 1.0, 1.0, 1.0),
                taxel("t1", 200.0, 30.0, 10.0, 1.0, 1.0, 1.0),
            ]),
            &evidence(),
            2.0,
        );
        assert_eq!(assessment.disposition, ContactFieldDisposition::AllTaxelsRobustlyAdmissible);
        assert_eq!(assessment.robustly_inside_count, 2);
    }

    #[test]
    fn shear_beyond_upper_friction_bound_is_definitely_outside() {
        let assessment = assess_contact_field(
            &request(vec![taxel("t0", 100.0, 100.0, 0.0, 0.1, 0.1, 0.1)]),
            &evidence(),
            2.0,
        );
        assert_eq!(assessment.disposition, ContactFieldDisposition::ContainsDefiniteViolations);
        assert_eq!(
            assessment.per_taxel[0].disposition,
            TaxelContactDisposition::DefinitelyOutsideFrictionCone
        );
    }

    #[test]
    fn uncertainty_overlap_is_indeterminate_not_silently_accepted() {
        let assessment = assess_contact_field(
            &request(vec![taxel("t0", 100.0, 45.0, 0.0, 5.0, 5.0, 0.0)]),
            &evidence(),
            2.0,
        );
        assert_eq!(assessment.disposition, ContactFieldDisposition::ContainsIndeterminateTaxels);
        assert_eq!(
            assessment.per_taxel[0].disposition,
            TaxelContactDisposition::IndeterminateGivenUncertainty
        );
    }

    #[test]
    fn tensile_pressure_and_zero_pressure_contact_are_not_accepted() {
        let assessment = assess_contact_field(
            &request(vec![
                taxel("tensile", -10.0, 0.0, 0.0, 0.1, 0.0, 0.0),
                taxel("zero", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            ]),
            &evidence(),
            2.0,
        );
        assert_eq!(assessment.disposition, ContactFieldDisposition::ContainsDefiniteViolations);
        assert_eq!(assessment.noncompressive_count, 2);
    }

    #[test]
    fn wrong_material_regime_or_shared_reference_cannot_admit_a_field() {
        let mut wrong_material = evidence();
        wrong_material.surface_pair_digest = "sha256:other-surface-pair".to_owned();
        assert_eq!(
            assess_contact_field(&request(vec![taxel("t0", 100.0, 10.0, 0.0, 0.0, 0.0, 0.0)]), &wrong_material, 2.0).disposition,
            ContactFieldDisposition::MaterialOrContextMismatch
        );

        let mut shared = evidence();
        shared.reference_independence = ReferenceIndependenceDisposition::SharedCommonCause;
        assert_eq!(
            assess_contact_field(&request(vec![taxel("t0", 100.0, 10.0, 0.0, 0.0, 0.0, 0.0)]), &shared, 2.0).disposition,
            ContactFieldDisposition::FrictionEvidenceUnresolved
        );

        let mut unsupported = request(vec![taxel("t0", 100.0, 10.0, 0.0, 0.0, 0.0, 0.0)]);
        unsupported.model_family = ContactModelFamily::AdhesiveOrOtherUnsupported;
        assert_eq!(
            assess_contact_field(&unsupported, &evidence(), 2.0).disposition,
            ContactFieldDisposition::UnsupportedContactModel
        );
    }

    #[test]
    fn global_wrench_closure_does_not_hide_a_locally_impossible_taxel() {
        let tangent_u = Vec3 { x: 1.0, y: 0.0, z: 0.0 };
        let tangent_v = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
        let normal = Vec3 { x: 0.0, y: 0.0, z: 1.0 };
        let geometry = TaxelGeometrySet {
            geometry_digest: "sha256:geometry-01".to_owned(),
            reference_frame_digest: "sha256:frame".to_owned(),
            contact_boundary_digest: "sha256:boundary".to_owned(),
            wrench_origin_m: Vec3::default(),
            taxels: [-0.01, 0.01]
                .into_iter()
                .enumerate()
                .map(|(index, x)| TaxelGeometry {
                    taxel_id: format!("t{index}"),
                    area_m2: 1.0e-4,
                    application_point_m: Vec3 { x, y: 0.0, z: 0.0 },
                    normal,
                    tangent_u,
                    tangent_v,
                })
                .collect(),
        };
        let operator = build_wrench_operator(&geometry).unwrap();
        let native_field = vec![100.0, 100.0, 0.0, 100.0, -100.0, 0.0];
        let predicted = integrate_tactile_field(&operator, &native_field).unwrap();
        let observed = WrenchObservation {
            force_n: predicted.force_n,
            moment_nm: predicted.moment_nm,
            force_standard_uncertainty_n: Vec3 { x: 0.001, y: 0.001, z: 0.001 },
            moment_standard_uncertainty_nm: Vec3 { x: 0.001, y: 0.001, z: 0.001 },
        };
        let wrench_evidence = WrenchReferenceEvidence {
            native_observation_set_digest: "sha256:native".to_owned(),
            wrench_observation_digest: "sha256:wrench".to_owned(),
            sample_identity: "sample:1".to_owned(),
            reference_frame_digest: "sha256:frame".to_owned(),
            contact_boundary_digest: "sha256:boundary".to_owned(),
            reference_instrument_receipt_id: "receipt:ati".to_owned(),
            reference_ancestry_receipt_id: "receipt:ati-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:cca".to_owned(),
            verifier_receipt_id: "receipt:verifier".to_owned(),
            reference_independence: WrenchReferenceIndependence::IndependentlyCharacterized,
            timestamp_alignment_uncertainty_ns: 0,
            maximum_permitted_alignment_uncertainty_ns: 10,
        };
        let closure = assess_wrench_consistency(
            &operator,
            &native_field,
            &observed,
            &wrench_evidence,
            "sha256:native",
            "sha256:wrench",
            3.0,
        );
        assert_eq!(closure.disposition, crate::active_benchmark::wrench_consistency::WrenchConsistencyDisposition::ConsistentWithinDeclaredUncertainty);

        let field = request(vec![
            taxel("t0", 100.0, 100.0, 0.0, 0.0, 0.0, 0.0),
            taxel("t1", 100.0, -100.0, 0.0, 0.0, 0.0, 0.0),
        ]);
        let local_check = assess_contact_field(&field, &evidence(), 2.0);
        assert_eq!(local_check.disposition, ContactFieldDisposition::ContainsDefiniteViolations);
        assert_eq!(local_check.definitely_outside_count, 2);
    }
}
