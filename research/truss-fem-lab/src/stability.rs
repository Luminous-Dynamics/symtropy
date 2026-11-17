//! Member-level yield and ideal Euler-column screening for truss results.
//!
//! This is a preliminary member screen only. It does not model global
//! frame buckling, beam bending stiffness, joint flexibility, imperfections,
//! residual stress, plasticity, fatigue, fracture, local plate buckling,
//! load interaction, or code-prescribed design factors.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

use crate::AnalysisResult;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MemberStabilityProperties {
    pub youngs_modulus_pa: f64,
    pub cross_section_area_m2: f64,
    /// Minimum principal second moment of area about a centroidal axis,
    /// in m^4. The caller must provide the correct section orientation.
    pub weak_axis_second_moment_m4: f64,
    pub yield_strength_pa: f64,
    /// Effective-length factor K. K=1 is the ideal pinned-pinned reference.
    pub effective_length_factor: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EulerRegime {
    /// The member is not carrying a compressive axial force.
    NotInCompression,
    /// The elastic Euler critical stress does not exceed the supplied yield
    /// strength. The ideal elastic-column comparison is in its nominal range.
    ElasticEulerApplicable,
    /// Euler predicts a stress at or above yield; inelastic buckling,
    /// yielding, and code-specific column curves may govern instead.
    YieldOrInelasticRegime,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MemberStabilityResult {
    pub member_index: usize,
    pub axial_force_n: f64,
    pub axial_stress_pa: f64,
    pub yield_utilization: f64,
    pub radius_of_gyration_m: f64,
    pub slenderness_ratio: f64,
    pub elastic_euler_critical_load_n: f64,
    /// None means the member is in tension or outside the nominal elastic
    /// Euler range. It does not mean zero buckling risk.
    pub elastic_euler_utilization: Option<f64>,
    pub euler_regime: EulerRegime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StabilityScreen {
    pub members: Vec<MemberStabilityResult>,
    pub maximum_yield_utilization: f64,
    pub maximum_elastic_euler_utilization: Option<f64>,
    pub yield_exceedance_count: usize,
    pub elastic_euler_exceedance_count: usize,
    pub euler_outside_nominal_range_count: usize,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StabilityError {
    PropertyCountMismatch {
        expected: usize,
        actual: usize,
    },
    InvalidProperty {
        member_index: usize,
        property: &'static str,
    },
    InvalidMemberResult {
        member_index: usize,
    },
    NonFiniteResult {
        member_index: usize,
    },
}

impl fmt::Display for StabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PropertyCountMismatch { expected, actual } => write!(
                f,
                "stability property count {actual} does not match member count {expected}"
            ),
            Self::InvalidProperty {
                member_index,
                property,
            } => write!(
                f,
                "member {member_index} has invalid stability property {property}"
            ),
            Self::InvalidMemberResult { member_index } => {
                write!(f, "member {member_index} has invalid axial result")
            }
            Self::NonFiniteResult { member_index } => {
                write!(f, "member {member_index} stability calculation is non-finite")
            }
        }
    }
}
impl Error for StabilityError {}

/// Screen each axial member for nominal yielding and elastic Euler buckling.
///
/// Sign convention: negative axial force is compression; positive is tension.
/// Each member has its own section/material properties. The calculation
/// assumes an ideal straight prismatic member with an effective length K*L.
/// It is not a code design check or a substitute for global frame stability.
pub fn screen_member_stability(
    analysis: &AnalysisResult,
    properties: &[MemberStabilityProperties],
) -> Result<StabilityScreen, StabilityError> {
    if analysis.members.len() != properties.len() {
        return Err(StabilityError::PropertyCountMismatch {
            expected: analysis.members.len(),
            actual: properties.len(),
        });
    }

    let mut results = Vec::with_capacity(analysis.members.len());
    let mut maximum_yield_utilization = 0.0_f64;
    let mut maximum_elastic_euler_utilization = 0.0_f64;
    let mut has_euler_utilization = false;
    let mut yield_exceedance_count = 0usize;
    let mut elastic_euler_exceedance_count = 0usize;
    let mut euler_outside_nominal_range_count = 0usize;

    for (member_index, (member, property)) in
        analysis.members.iter().zip(properties.iter()).enumerate()
    {
        for (name, value) in [
            ("youngs_modulus_pa", property.youngs_modulus_pa),
            ("cross_section_area_m2", property.cross_section_area_m2),
            (
                "weak_axis_second_moment_m4",
                property.weak_axis_second_moment_m4,
            ),
            ("yield_strength_pa", property.yield_strength_pa),
            ("effective_length_factor", property.effective_length_factor),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(StabilityError::InvalidProperty {
                    member_index,
                    property: name,
                });
            }
        }
        if !member.length_m.is_finite()
            || member.length_m <= 0.0
            || !member.axial_force_n.is_finite()
            || !member.axial_stress_pa.is_finite()
        {
            return Err(StabilityError::InvalidMemberResult { member_index });
        }

        let radius_of_gyration =
            (property.weak_axis_second_moment_m4 / property.cross_section_area_m2).sqrt();
        let effective_length = property.effective_length_factor * member.length_m;
        let slenderness_ratio = effective_length / radius_of_gyration;
        let euler_numerator =
            PI * PI * property.youngs_modulus_pa * property.weak_axis_second_moment_m4;
        let euler_denominator = effective_length * effective_length;
        let elastic_euler_critical_load_n = euler_numerator / euler_denominator;
        let yield_utilization = member.axial_stress_pa.abs() / property.yield_strength_pa;
        let transition_slenderness =
            PI * (property.youngs_modulus_pa / property.yield_strength_pa).sqrt();

        if !radius_of_gyration.is_finite()
            || radius_of_gyration <= 0.0
            || !effective_length.is_finite()
            || effective_length <= 0.0
            || !slenderness_ratio.is_finite()
            || !elastic_euler_critical_load_n.is_finite()
            || elastic_euler_critical_load_n <= 0.0
            || !yield_utilization.is_finite()
            || !transition_slenderness.is_finite()
        {
            return Err(StabilityError::NonFiniteResult { member_index });
        }

        let euler_regime = if member.axial_force_n >= 0.0 {
            EulerRegime::NotInCompression
        } else if slenderness_ratio >= transition_slenderness {
            EulerRegime::ElasticEulerApplicable
        } else {
            EulerRegime::YieldOrInelasticRegime
        };

        let elastic_euler_utilization = if member.axial_force_n < 0.0
            && euler_regime == EulerRegime::ElasticEulerApplicable
        {
            Some((-member.axial_force_n) / elastic_euler_critical_load_n)
        } else {
            None
        };

        if yield_utilization > 1.0 {
            yield_exceedance_count += 1;
        }
        maximum_yield_utilization = maximum_yield_utilization.max(yield_utilization);

        match elastic_euler_utilization {
            Some(utilization) if utilization.is_finite() => {
                if utilization > 1.0 {
                    elastic_euler_exceedance_count += 1;
                }
                maximum_elastic_euler_utilization =
                    maximum_elastic_euler_utilization.max(utilization);
                has_euler_utilization = true;
            }
            Some(_) => return Err(StabilityError::NonFiniteResult { member_index }),
            None if member.axial_force_n < 0.0 => {
                euler_outside_nominal_range_count += 1;
            }
            None => {}
        }

        results.push(MemberStabilityResult {
            member_index,
            axial_force_n: member.axial_force_n,
            axial_stress_pa: member.axial_stress_pa,
            yield_utilization,
            radius_of_gyration_m: radius_of_gyration,
            slenderness_ratio,
            elastic_euler_critical_load_n,
            elastic_euler_utilization,
            euler_regime,
        });
    }

    Ok(StabilityScreen {
        members: results,
        maximum_yield_utilization,
        maximum_elastic_euler_utilization: if has_euler_utilization {
            Some(maximum_elastic_euler_utilization)
        } else {
            None
        },
        yield_exceedance_count,
        elastic_euler_exceedance_count,
        euler_outside_nominal_range_count,
        claim_ceiling: "nominal axial-yield and ideal elastic Euler member screen only; not beam/frame stability, code design, or qualification",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundaryConditions, TrussMaterial, TrussMember, TrussModel};
    use symtropy_freeflow_structure_lab::Vec3;

    fn properties() -> MemberStabilityProperties {
        MemberStabilityProperties {
            youngs_modulus_pa: 200.0e9,
            cross_section_area_m2: 0.01,
            weak_axis_second_moment_m4: 1.0e-8,
            yield_strength_pa: 250.0e6,
            effective_length_factor: 1.0,
        }
    }

    fn axial_bar(force_n: f64, length_m: f64) -> AnalysisResult {
        let model = TrussModel {
            nodes: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(length_m, 0.0, 0.0),
            ],
            members: vec![TrussMember { from: 0, to: 1 }],
        };
        let material = TrussMaterial {
            youngs_modulus_pa: 200.0e9,
            cross_section_area_m2: 0.01,
        };
        let mut boundary = BoundaryConditions::new(2);
        boundary.fix_node(0).unwrap();
        boundary.fix_dof(1, 1).unwrap();
        boundary.fix_dof(1, 2).unwrap();
        boundary.add_force(1, [force_n, 0.0, 0.0]).unwrap();
        model.analyze(material, &boundary).unwrap()
    }

    #[test]
    fn compression_uses_closed_form_euler_load_and_yield_utilization() {
        let analysis = axial_bar(-1_000.0, 2.0);
        let screen = screen_member_stability(&analysis, &[properties()]).unwrap();
        let member = screen.members[0];

        let expected_pcr = PI * PI * 200.0e9 * 1.0e-8 / 4.0;
        assert!((member.elastic_euler_critical_load_n - expected_pcr).abs() < 1e-8);
        assert!(
            (member.elastic_euler_utilization.unwrap() - 1_000.0 / expected_pcr).abs()
                < 1e-12
        );
        assert!((member.yield_utilization - 100_000.0 / 250.0e6).abs() < 1e-12);
        assert_eq!(member.euler_regime, EulerRegime::ElasticEulerApplicable);
        assert_eq!(screen.elastic_euler_exceedance_count, 0);
    }

    #[test]
    fn tension_does_not_report_euler_compression_utilization() {
        let analysis = axial_bar(1_000.0, 2.0);
        let screen = screen_member_stability(&analysis, &[properties()]).unwrap();
        assert_eq!(screen.members[0].elastic_euler_utilization, None);
        assert_eq!(screen.members[0].euler_regime, EulerRegime::NotInCompression);
        assert_eq!(screen.maximum_elastic_euler_utilization, None);
    }

    #[test]
    fn short_column_is_not_misclassified_as_elastic_euler_governed() {
        let analysis = axial_bar(-1_000.0, 0.01);
        let screen = screen_member_stability(&analysis, &[properties()]).unwrap();
        let member = screen.members[0];
        assert_eq!(member.euler_regime, EulerRegime::YieldOrInelasticRegime);
        assert_eq!(member.elastic_euler_utilization, None);
        assert_eq!(screen.euler_outside_nominal_range_count, 1);
    }

    #[test]
    fn mismatched_or_invalid_section_properties_fail_closed() {
        let analysis = axial_bar(-1_000.0, 2.0);
        assert!(matches!(
            screen_member_stability(&analysis, &[]),
            Err(StabilityError::PropertyCountMismatch { .. })
        ));
        let mut invalid = properties();
        invalid.weak_axis_second_moment_m4 = f64::NAN;
        assert!(matches!(
            screen_member_stability(&analysis, &[invalid]),
            Err(StabilityError::InvalidProperty { .. })
        ));
    }
}
