// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericCoordinates {
    pub froude_number: Option<f64>,
    pub cohesion_to_particle_weight: Option<f64>,
    pub wall_adhesion_to_particle_weight: Option<f64>,
    pub particle_to_drum_ratio: Option<f64>,
    pub wall_roughness_to_particle_ratio: Option<f64>,
    pub fill_fraction: Option<f64>,
    pub gravity_m_s2: Option<f64>,
    pub chamber_pressure_pa: Option<f64>,
    pub inertial_number: Option<f64>,
    pub electrostatic_number: Option<f64>,
    pub vibration_gamma: Option<f64>,
    pub vibration_frequency_gravity_scale: Option<f64>,
    pub vibration_amplitude_to_particle_size: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoricalCoordinates {
    pub atmosphere: Option<String>,
    pub wall_material: Option<String>,
    pub wall_grounding: Option<String>,
    pub flow_regime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowProfile {
    pub profile_id: String,
    pub evidence_class: String,
    pub numeric: NumericCoordinates,
    pub categorical: CategoricalCoordinates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityTolerances {
    pub froude_factor: f64,
    pub cohesion_factor: f64,
    pub wall_adhesion_factor: f64,
    pub particle_to_drum_factor: f64,
    pub wall_roughness_factor: f64,
    pub fill_fraction_absolute: f64,
    pub gravity_factor: f64,
    pub pressure_factor: f64,
    pub inertial_number_factor: f64,
    pub electrostatic_number_factor: f64,
    pub vibration_gamma_factor: f64,
    pub vibration_frequency_factor: f64,
    pub vibration_amplitude_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonInput {
    pub target: FlowProfile,
    pub candidate: FlowProfile,
    pub tolerances: SimilarityTolerances,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateStatus {
    WithinTolerance,
    OutsideTolerance,
    UnknownMissingValue,
    Match,
    Mismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateResult {
    pub coordinate: String,
    pub target_value: Option<f64>,
    pub candidate_value: Option<f64>,
    pub deviation: Option<f64>,
    pub declared_tolerance: Option<f64>,
    pub status: CoordinateStatus,
    pub interpretation: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeStatus {
    WithinDeclaredSimilarityEnvelope,
    OutsideDeclaredSimilarityEnvelope,
    IndeterminateMissingCoordinate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityReport {
    pub report_class: String,
    pub target_profile_id: String,
    pub candidate_profile_id: String,
    pub target_evidence_class: String,
    pub candidate_evidence_class: String,
    pub overall_status: EnvelopeStatus,
    pub coordinates: Vec<CoordinateResult>,
    pub claim_ceiling: String,
}

fn require_nonnegative(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{name} must be finite and non-negative"));
    }
    Ok(())
}

fn require_strictly_positive(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{name} must be finite and strictly positive"));
    }
    Ok(())
}

fn require_factor(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() || value < 1.0 {
        return Err(format!("{name} must be finite and at least 1.0"));
    }
    Ok(())
}

fn require_fill(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) || value == 0.0 {
        return Err(format!("{name} must be finite and in (0, 1]"));
    }
    Ok(())
}

fn positive_ratio_result(
    coordinate: &str,
    target: Option<f64>,
    candidate: Option<f64>,
    factor_tolerance: f64,
    zero_is_valid: bool,
) -> Result<CoordinateResult, String> {
    require_factor(factor_tolerance, &format!("{coordinate} tolerance"))?;

    let (Some(target), Some(candidate)) = (target, candidate) else {
        return Ok(CoordinateResult {
            coordinate: coordinate.to_owned(),
            target_value: target,
            candidate_value: candidate,
            deviation: None,
            declared_tolerance: Some(factor_tolerance),
            status: CoordinateStatus::UnknownMissingValue,
            interpretation: "required coordinate missing; similarity cannot be established".to_owned(),
        });
    };

    require_nonnegative(target, &format!("target {coordinate}"))?;
    require_nonnegative(candidate, &format!("candidate {coordinate}"))?;

    if (target == 0.0 || candidate == 0.0) && !zero_is_valid {
        return Err(format!("{coordinate} must be strictly positive"));
    }

    if target == 0.0 || candidate == 0.0 {
        let both_zero = target == 0.0 && candidate == 0.0;
        return Ok(CoordinateResult {
            coordinate: coordinate.to_owned(),
            target_value: Some(target),
            candidate_value: Some(candidate),
            deviation: if both_zero { Some(1.0) } else { None },
            declared_tolerance: Some(factor_tolerance),
            status: if both_zero {
                CoordinateStatus::WithinTolerance
            } else {
                CoordinateStatus::OutsideTolerance
            },
            interpretation: if both_zero {
                "both profiles report zero for this coordinate".to_owned()
            } else {
                "one profile reports zero while the other is non-zero".to_owned()
            },
        });
    }

    let ratio = candidate / target;
    if !ratio.is_finite() || ratio <= 0.0 {
        return Err(format!("{coordinate} ratio is outside finite numeric range"));
    }
    let symmetric_factor = ratio.max(1.0 / ratio);
    if !symmetric_factor.is_finite() {
        return Err(format!("{coordinate} symmetric ratio is outside finite numeric range"));
    }
    let status = if symmetric_factor <= factor_tolerance {
        CoordinateStatus::WithinTolerance
    } else {
        CoordinateStatus::OutsideTolerance
    };

    Ok(CoordinateResult {
        coordinate: coordinate.to_owned(),
        target_value: Some(target),
        candidate_value: Some(candidate),
        deviation: Some(symmetric_factor),
        declared_tolerance: Some(factor_tolerance),
        interpretation: match &status {
            CoordinateStatus::WithinTolerance => {
                "symmetric multiplicative deviation is within the declared factor".to_owned()
            }
            _ => "symmetric multiplicative deviation exceeds the declared factor".to_owned(),
        },
        status,
    })
}

fn signed_ratio_result(
    coordinate: &str,
    target: Option<f64>,
    candidate: Option<f64>,
    factor_tolerance: f64,
) -> Result<CoordinateResult, String> {
    require_factor(factor_tolerance, &format!("{coordinate} tolerance"))?;

    let (Some(target), Some(candidate)) = (target, candidate) else {
        return Ok(CoordinateResult {
            coordinate: coordinate.to_owned(),
            target_value: target,
            candidate_value: candidate,
            deviation: None,
            declared_tolerance: Some(factor_tolerance),
            status: CoordinateStatus::UnknownMissingValue,
            interpretation: "required signed coordinate missing; similarity cannot be established".to_owned(),
        });
    };

    if !target.is_finite() || !candidate.is_finite() {
        return Err(format!("{coordinate} values must be finite"));
    }

    if target == 0.0 || candidate == 0.0 {
        let both_zero = target == 0.0 && candidate == 0.0;
        return Ok(CoordinateResult {
            coordinate: coordinate.to_owned(),
            target_value: Some(target),
            candidate_value: Some(candidate),
            deviation: if both_zero { Some(1.0) } else { None },
            declared_tolerance: Some(factor_tolerance),
            status: if both_zero {
                CoordinateStatus::WithinTolerance
            } else {
                CoordinateStatus::OutsideTolerance
            },
            interpretation: if both_zero {
                "both profiles report zero electrostatic drive".to_owned()
            } else {
                "one profile reports zero electrostatic drive while the other is non-zero".to_owned()
            },
        });
    }

    if target.is_sign_positive() != candidate.is_sign_positive() {
        return Ok(CoordinateResult {
            coordinate: coordinate.to_owned(),
            target_value: Some(target),
            candidate_value: Some(candidate),
            deviation: None,
            declared_tolerance: Some(factor_tolerance),
            status: CoordinateStatus::OutsideTolerance,
            interpretation: "electrostatic coordinate sign differs; force direction is not similar".to_owned(),
        });
    }

    let ratio = candidate.abs() / target.abs();
    if !ratio.is_finite() || ratio <= 0.0 {
        return Err(format!("{coordinate} magnitude ratio is outside finite numeric range"));
    }
    let symmetric_factor = ratio.max(1.0 / ratio);
    if !symmetric_factor.is_finite() {
        return Err(format!("{coordinate} symmetric magnitude ratio is outside finite numeric range"));
    }
    let status = if symmetric_factor <= factor_tolerance {
        CoordinateStatus::WithinTolerance
    } else {
        CoordinateStatus::OutsideTolerance
    };

    Ok(CoordinateResult {
        coordinate: coordinate.to_owned(),
        target_value: Some(target),
        candidate_value: Some(candidate),
        deviation: Some(symmetric_factor),
        declared_tolerance: Some(factor_tolerance),
        status,
        interpretation: "electrostatic drive polarity matches; magnitude compared with a symmetric factor tolerance".to_owned(),
    })
}

fn fill_result(
    target: Option<f64>,
    candidate: Option<f64>,
    absolute_tolerance: f64,
) -> Result<CoordinateResult, String> {
    if !absolute_tolerance.is_finite() || !(0.0..=1.0).contains(&absolute_tolerance) {
        return Err("fill_fraction_absolute tolerance must be finite and in [0, 1]".to_owned());
    }

    let (Some(target), Some(candidate)) = (target, candidate) else {
        return Ok(CoordinateResult {
            coordinate: "fill_fraction".to_owned(),
            target_value: target,
            candidate_value: candidate,
            deviation: None,
            declared_tolerance: Some(absolute_tolerance),
            status: CoordinateStatus::UnknownMissingValue,
            interpretation: "required coordinate missing; similarity cannot be established".to_owned(),
        });
    };
    require_fill(target, "target fill_fraction")?;
    require_fill(candidate, "candidate fill_fraction")?;

    let delta = (candidate - target).abs();
    let status = if delta <= absolute_tolerance {
        CoordinateStatus::WithinTolerance
    } else {
        CoordinateStatus::OutsideTolerance
    };

    Ok(CoordinateResult {
        coordinate: "fill_fraction".to_owned(),
        target_value: Some(target),
        candidate_value: Some(candidate),
        deviation: Some(delta),
        declared_tolerance: Some(absolute_tolerance),
        status,
        interpretation: match status {
            CoordinateStatus::WithinTolerance => {
                "absolute fill-fraction difference is within the declared tolerance".to_owned()
            }
            _ => "absolute fill-fraction difference exceeds the declared tolerance".to_owned(),
        },
    })
}

fn categorical_result(
    coordinate: &str,
    target: &Option<String>,
    candidate: &Option<String>,
) -> CoordinateResult {
    match (target, candidate) {
        (Some(target), Some(candidate)) if target.trim().is_empty() || candidate.trim().is_empty() => {
            CoordinateResult {
                coordinate: coordinate.to_owned(),
                target_value: None,
                candidate_value: None,
                deviation: None,
                declared_tolerance: None,
                status: CoordinateStatus::UnknownMissingValue,
                interpretation: "categorical value is blank".to_owned(),
            }
        }
        (Some(target), Some(candidate)) if target == candidate => CoordinateResult {
            coordinate: coordinate.to_owned(),
            target_value: None,
            candidate_value: None,
            deviation: None,
            declared_tolerance: None,
            status: CoordinateStatus::Match,
            interpretation: "categorical boundary matches exactly".to_owned(),
        },
        (Some(_), Some(_)) => CoordinateResult {
            coordinate: coordinate.to_owned(),
            target_value: None,
            candidate_value: None,
            deviation: None,
            declared_tolerance: None,
            status: CoordinateStatus::Mismatch,
            interpretation: "categorical boundary differs; do not claim direct similitude".to_owned(),
        },
        _ => CoordinateResult {
            coordinate: coordinate.to_owned(),
            target_value: None,
            candidate_value: None,
            deviation: None,
            declared_tolerance: None,
            status: CoordinateStatus::UnknownMissingValue,
            interpretation: "categorical boundary missing; similarity cannot be established".to_owned(),
        },
    }
}

pub fn compare(input: &ComparisonInput) -> Result<SimilarityReport, String> {
    if input.target.profile_id.trim().is_empty() || input.candidate.profile_id.trim().is_empty() {
        return Err("profile IDs must not be empty".to_owned());
    }
    if input.target.evidence_class.trim().is_empty()
        || input.candidate.evidence_class.trim().is_empty()
    {
        return Err("both evidence classes must be explicit and non-empty".to_owned());
    }

    let t = &input.tolerances;
    let a = &input.target.numeric;
    let b = &input.candidate.numeric;
    let ca = &input.target.categorical;
    let cb = &input.candidate.categorical;

    for (value, name) in [
        (a.gravity_m_s2, "target gravity"),
        (b.gravity_m_s2, "candidate gravity"),
        (a.chamber_pressure_pa, "target chamber pressure"),
        (b.chamber_pressure_pa, "candidate chamber pressure"),
    ] {
        if let Some(value) = value {
            require_strictly_positive(value, name)?;
        }
    }

    let coordinates = vec![
        positive_ratio_result("froude_number", a.froude_number, b.froude_number, t.froude_factor, false)?,
        positive_ratio_result(
            "cohesion_to_particle_weight",
            a.cohesion_to_particle_weight,
            b.cohesion_to_particle_weight,
            t.cohesion_factor,
            true,
        )?,
        positive_ratio_result(
            "wall_adhesion_to_particle_weight",
            a.wall_adhesion_to_particle_weight,
            b.wall_adhesion_to_particle_weight,
            t.wall_adhesion_factor,
            true,
        )?,
        positive_ratio_result(
            "particle_to_drum_ratio",
            a.particle_to_drum_ratio,
            b.particle_to_drum_ratio,
            t.particle_to_drum_factor,
            false,
        )?,
        positive_ratio_result(
            "wall_roughness_to_particle_ratio",
            a.wall_roughness_to_particle_ratio,
            b.wall_roughness_to_particle_ratio,
            t.wall_roughness_factor,
            false,
        )?,
        fill_result(a.fill_fraction, b.fill_fraction, t.fill_fraction_absolute)?,
        positive_ratio_result("gravity_m_s2", a.gravity_m_s2, b.gravity_m_s2, t.gravity_factor, false)?,
        positive_ratio_result(
            "chamber_pressure_pa",
            a.chamber_pressure_pa,
            b.chamber_pressure_pa,
            t.pressure_factor,
            false,
        )?,
        positive_ratio_result(
            "inertial_number",
            a.inertial_number,
            b.inertial_number,
            t.inertial_number_factor,
            true,
        )?,
        signed_ratio_result(
            "electrostatic_number",
            a.electrostatic_number,
            b.electrostatic_number,
            t.electrostatic_number_factor,
        )?,
        positive_ratio_result(
            "vibration_gamma",
            a.vibration_gamma,
            b.vibration_gamma,
            t.vibration_gamma_factor,
            true,
        )?,
        positive_ratio_result(
            "vibration_frequency_gravity_scale",
            a.vibration_frequency_gravity_scale,
            b.vibration_frequency_gravity_scale,
            t.vibration_frequency_factor,
            true,
        )?,
        positive_ratio_result(
            "vibration_amplitude_to_particle_size",
            a.vibration_amplitude_to_particle_size,
            b.vibration_amplitude_to_particle_size,
            t.vibration_amplitude_factor,
            true,
        )?,
        categorical_result("atmosphere", &ca.atmosphere, &cb.atmosphere),
        categorical_result("wall_material", &ca.wall_material, &cb.wall_material),
        categorical_result("wall_grounding", &ca.wall_grounding, &cb.wall_grounding),
        categorical_result("flow_regime", &ca.flow_regime, &cb.flow_regime),
    ];

    let has_outside = coordinates.iter().any(|item| {
        matches!(
            item.status,
            CoordinateStatus::OutsideTolerance | CoordinateStatus::Mismatch
        )
    });
    let has_unknown = coordinates
        .iter()
        .any(|item| item.status == CoordinateStatus::UnknownMissingValue);
    let overall_status = if has_outside {
        EnvelopeStatus::OutsideDeclaredSimilarityEnvelope
    } else if has_unknown {
        EnvelopeStatus::IndeterminateMissingCoordinate
    } else {
        EnvelopeStatus::WithinDeclaredSimilarityEnvelope
    };

    Ok(SimilarityReport {
        report_class: "granular_flow_similarity_envelope_comparison".to_owned(),
        target_profile_id: input.target.profile_id.clone(),
        candidate_profile_id: input.candidate.profile_id.clone(),
        target_evidence_class: input.target.evidence_class.clone(),
        candidate_evidence_class: input.candidate.evidence_class.clone(),
        overall_status,
        coordinates,
        claim_ceiling: "A within-envelope result means only that all supplied coordinates satisfy caller-declared tolerances. It does not prove physical equivalence, lunar-gravity similitude, model validity, hardware qualification or mission performance.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str) -> FlowProfile {
        FlowProfile {
            profile_id: id.to_owned(),
            evidence_class: "synthetic_fixture".to_owned(),
            numeric: NumericCoordinates {
                froude_number: Some(1.0e-3),
                cohesion_to_particle_weight: Some(0.20),
                wall_adhesion_to_particle_weight: Some(0.10),
                particle_to_drum_ratio: Some(9.0e-4),
                wall_roughness_to_particle_ratio: Some(0.10),
                fill_fraction: Some(0.25),
                gravity_m_s2: Some(9.81),
                chamber_pressure_pa: Some(0.08),
                inertial_number: Some(0.01),
                electrostatic_number: Some(-0.5),
                vibration_gamma: Some(0.816),
                vibration_frequency_gravity_scale: Some(0.0857),
                vibration_amplitude_to_particle_size: Some(2.22),
            },
            categorical: CategoricalCoordinates {
                atmosphere: Some("high_vacuum".to_owned()),
                wall_material: Some("PTFE".to_owned()),
                wall_grounding: Some("floating".to_owned()),
                flow_regime: Some("slumping".to_owned()),
            },
        }
    }

    fn input() -> ComparisonInput {
        let mut target = profile("target");
        target.evidence_class = "synthetic_target_fixture".to_owned();
        let mut candidate = profile("candidate");
        candidate.evidence_class = "synthetic_candidate_fixture".to_owned();
        candidate.numeric.froude_number = Some(1.1e-3);
        candidate.numeric.cohesion_to_particle_weight = Some(0.22);
        candidate.numeric.wall_adhesion_to_particle_weight = Some(0.09);
        candidate.numeric.particle_to_drum_ratio = Some(9.5e-4);
        candidate.numeric.wall_roughness_to_particle_ratio = Some(0.11);
        candidate.numeric.fill_fraction = Some(0.27);
        candidate.numeric.inertial_number = Some(0.011);
        candidate.numeric.electrostatic_number = Some(-0.55);
        candidate.numeric.vibration_gamma = Some(0.84);
        candidate.numeric.vibration_frequency_gravity_scale = Some(0.088);
        candidate.numeric.vibration_amplitude_to_particle_size = Some(2.3);

        ComparisonInput {
            target,
            candidate,
            tolerances: SimilarityTolerances {
                froude_factor: 1.25,
                cohesion_factor: 1.25,
                wall_adhesion_factor: 1.25,
                particle_to_drum_factor: 1.20,
                wall_roughness_factor: 1.20,
                fill_fraction_absolute: 0.05,
                gravity_factor: 1.10,
                pressure_factor: 1.25,
                inertial_number_factor: 1.25,
                electrostatic_number_factor: 1.25,
                vibration_gamma_factor: 1.25,
                vibration_frequency_factor: 1.25,
                vibration_amplitude_factor: 1.25,
            },
        }
    }

    #[test]
    fn all_required_coordinates_within_tolerance_return_within() {
        let report = compare(&input()).expect("valid similarity profiles");
        assert_eq!(
            report.overall_status,
            EnvelopeStatus::WithinDeclaredSimilarityEnvelope
        );
        assert_eq!(report.coordinates.len(), 17);
    }

    #[test]
    fn matching_froude_does_not_hide_cohesion_mismatch() {
        let mut value = input();
        value.candidate.numeric.froude_number = value.target.numeric.froude_number;
        value.candidate.numeric.cohesion_to_particle_weight = Some(2.0);
        let report = compare(&value).expect("valid profile values");
        assert_eq!(
            report.overall_status,
            EnvelopeStatus::OutsideDeclaredSimilarityEnvelope
        );
        assert_eq!(
            report.coordinates[1].status,
            CoordinateStatus::OutsideTolerance
        );
    }

    #[test]
    fn zero_cohesion_matches_only_zero_cohesion() {
        let mut value = input();
        value.target.numeric.cohesion_to_particle_weight = Some(0.0);
        value.candidate.numeric.cohesion_to_particle_weight = Some(0.0);
        let same = compare(&value).expect("zero is a valid measured limit");
        assert_eq!(
            same.coordinates[1].status,
            CoordinateStatus::WithinTolerance
        );

        value.candidate.numeric.cohesion_to_particle_weight = Some(0.01);
        let mismatch = compare(&value).expect("zero/non-zero mismatch is classifiable");
        assert_eq!(
            mismatch.coordinates[1].status,
            CoordinateStatus::OutsideTolerance
        );
    }

    #[test]
    fn zero_geometric_ratio_is_rejected_but_zero_contact_force_is_allowed() {
        let mut value = input();
        value.target.numeric.cohesion_to_particle_weight = Some(0.0);
        value.candidate.numeric.cohesion_to_particle_weight = Some(0.0);
        assert!(compare(&value).is_ok());

        value.candidate.numeric.particle_to_drum_ratio = Some(0.0);
        assert!(compare(&value).is_err());

        value = input();
        value.candidate.numeric.wall_roughness_to_particle_ratio = Some(0.0);
        assert!(compare(&value).is_err());
    }

    #[test]
    fn unknown_cohesion_returns_indeterminate_not_a_pass() {
        let mut value = input();
        value.candidate.numeric.cohesion_to_particle_weight = None;
        let report = compare(&value).expect("missing values are represented as unknown");
        assert_eq!(
            report.overall_status,
            EnvelopeStatus::IndeterminateMissingCoordinate
        );
        assert_eq!(
            report.coordinates[1].status,
            CoordinateStatus::UnknownMissingValue
        );
    }

    #[test]
    fn opposite_electrostatic_polarity_is_outside_even_at_equal_magnitude() {
        let mut value = input();
        value.target.numeric.electrostatic_number = Some(0.5);
        value.candidate.numeric.electrostatic_number = Some(-0.5);
        let report = compare(&value).expect("signed electrostatic coordinate is valid");
        assert_eq!(
            report.overall_status,
            EnvelopeStatus::OutsideDeclaredSimilarityEnvelope
        );
        assert_eq!(
            report.coordinates[9].status,
            CoordinateStatus::OutsideTolerance
        );
    }

    #[test]
    fn missing_inertial_number_is_indeterminate() {
        let mut value = input();
        value.candidate.numeric.inertial_number = None;
        let report = compare(&value).expect("missing inertial number is represented as unknown");
        assert_eq!(
            report.overall_status,
            EnvelopeStatus::IndeterminateMissingCoordinate
        );
        assert_eq!(
            report.coordinates[8].status,
            CoordinateStatus::UnknownMissingValue
        );
    }

    #[test]
    fn wall_material_mismatch_is_a_hard_boundary() {
        let mut value = input();
        value.candidate.categorical.wall_material = Some("copper".to_owned());
        let report = compare(&value).expect("valid categorical values");
        assert_eq!(
            report.overall_status,
            EnvelopeStatus::OutsideDeclaredSimilarityEnvelope
        );
        assert_eq!(
            report.coordinates[14].status,
            CoordinateStatus::Mismatch
        );
    }

    #[test]
    fn rejects_invalid_tolerances_and_nonpositive_coordinates() {
        let mut value = input();
        value.tolerances.froude_factor = 0.9;
        assert!(compare(&value).is_err());

        let mut value = input();
        value.candidate.numeric.chamber_pressure_pa = Some(0.0);
        assert!(compare(&value).is_err());
    }

    #[test]
    fn a_missing_categorical_boundary_is_indeterminate() {
        let mut value = input();
        value.candidate.categorical.wall_grounding = None;
        let report = compare(&value).expect("missing category is unknown");
        assert_eq!(
            report.overall_status,
            EnvelopeStatus::IndeterminateMissingCoordinate
        );
    }
}
