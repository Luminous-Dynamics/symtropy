//! AIACHNE-style grouped Gaussian energy-bias basis construction.
//!
//! This module implements the published basis geometry only: three Gaussian
//! width scales in log10 energy, with short / medium / long groups of 100 / 50 / 10 centres respectively.
//! It does not implement the horseshoe prior or the Bayesian Monte Carlo
//! inference used by the reference paper.

use crate::ModelError;

pub const ENERGY_BIAS_LENGTH_SCALES_LOG10: [f64; 3] = [0.05, 0.15, 0.3];
pub const ENERGY_BIAS_CENTRES_PER_SCALE: [usize; 3] = [100, 50, 10];
pub const ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP: usize = 160;
pub const AIACHNE_BIAS_DOMAIN_LOWER_SUBTRACTION_MEV: f64 = 1.0e-4;
pub const AIACHNE_BIAS_DOMAIN_UPPER_ADDITION_MEV: f64 = 5.0;

/// Reproduce the reference-grid domain extension in AIACHNE/ExecuteStudy.ipynb.
/// Map each coefficient in the flattened, group-major AIACHNE basis to
/// its separate global horseshoe scale. Within each feature group the order is
/// short (B_s), medium (B_m), then long (B_l).
pub fn aiachne_global_scale_index_by_coefficient(
    feature_group_count: usize,
) -> Result<Vec<usize>, ModelError> {
    if feature_group_count == 0 {
        return Err(ModelError::InvalidParameter(
            "aiachne_feature_group_count_must_be_positive",
        ));
    }
    let coefficient_count = feature_group_count
        .checked_mul(ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP)
        .ok_or(ModelError::InvalidParameter(
            "aiachne_horseshoe_coefficient_count_overflow",
        ))?;
    let mut indices = Vec::with_capacity(coefficient_count);
    for group in 0..feature_group_count {
        let base = group.checked_mul(3).ok_or(ModelError::InvalidParameter(
            "aiachne_horseshoe_group_index_overflow",
        ))?;
        indices.extend(std::iter::repeat_n(base, ENERGY_BIAS_CENTRES_PER_SCALE[0]));
        indices.extend(std::iter::repeat_n(
            base + 1,
            ENERGY_BIAS_CENTRES_PER_SCALE[1],
        ));
        indices.extend(std::iter::repeat_n(
            base + 2,
            ENERGY_BIAS_CENTRES_PER_SCALE[2],
        ));
    }
    if indices.len() != coefficient_count {
        return Err(ModelError::InvalidParameter(
            "aiachne_horseshoe_scale_mapping_length_invalid",
        ));
    }
    Ok(indices)
}

pub fn aiachne_bias_basis_energy_domain(
    reference_energies_mev: &[f64],
) -> Result<(f64, f64), ModelError> {
    if reference_energies_mev.len() < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    if reference_energies_mev
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("aiachne_reference_energy_grid"));
    }
    if reference_energies_mev
        .windows(2)
        .any(|pair| pair[1] <= pair[0])
    {
        return Err(ModelError::InvalidParameter(
            "aiachne_reference_energy_grid_must_be_strictly_increasing",
        ));
    }
    let minimum = reference_energies_mev[0] - AIACHNE_BIAS_DOMAIN_LOWER_SUBTRACTION_MEV;
    let maximum = reference_energies_mev[reference_energies_mev.len() - 1]
        + AIACHNE_BIAS_DOMAIN_UPPER_ADDITION_MEV;
    if !minimum.is_finite() || !maximum.is_finite() || minimum <= 0.0 || maximum <= minimum {
        return Err(ModelError::InvalidParameter(
            "aiachne_bias_basis_energy_domain_invalid",
        ));
    }
    Ok((minimum, maximum))
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedEnergyBiasDesign {
    pub energy_centres_mev: Vec<f64>,
    pub length_scales_log10: Vec<f64>,
    pub columns_per_group: usize,
    pub group_count: usize,
    /// One row per observation and one column per Gaussian energy basis.
    pub basis_by_observation: Vec<Vec<f64>>,
    /// One row per observation; each feature-level group owns 160 columns.
    pub grouped_design_matrix: Vec<Vec<f64>>,
}

/// Build the Gaussian basis and overlapping feature-level design.
///
/// Each observation can belong to zero, one, or multiple feature-level groups.
/// This allows the same data set to appear in several groupings (for example,
/// detector type and response-correction method), intentionally exposing
/// redundant explanations. A group block is populated only for observations
/// assigned to that group.
///
/// Basis centres are equally spaced in log10 energy over the explicitly frozen
/// energy domain. Basis values are:
/// exp(-((log10(E) - log10(zeta)) / ell)^2).
pub fn build_grouped_energy_bias_design(
    energies_mev: &[f64],
    group_memberships_by_observation: &[Vec<usize>],
    group_count: usize,
    energy_domain_min_mev: f64,
    energy_domain_max_mev: f64,
) -> Result<GroupedEnergyBiasDesign, ModelError> {
    if energies_mev.is_empty()
        || energies_mev.len() != group_memberships_by_observation.len()
        || group_count == 0
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    for (value, name) in [
        (energy_domain_min_mev, "energy_domain_min_mev"),
        (energy_domain_max_mev, "energy_domain_max_mev"),
    ] {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue(name));
        }
    }
    if energy_domain_min_mev <= 0.0 || energy_domain_max_mev <= energy_domain_min_mev {
        return Err(ModelError::InvalidParameter(
            "energy_bias_domain_must_be_positive_and_ordered",
        ));
    }
    for energy in energies_mev {
        if !energy.is_finite() {
            return Err(ModelError::NonFiniteValue("observation_energy_mev"));
        }
        if *energy <= 0.0 {
            return Err(ModelError::NonPositiveValue("observation_energy_mev"));
        }
        if *energy < energy_domain_min_mev || *energy > energy_domain_max_mev {
            return Err(ModelError::InvalidParameter(
                "observation_energy_outside_frozen_bias_domain",
            ));
        }
    }
    for memberships in group_memberships_by_observation {
        let mut seen = vec![false; group_count];
        for group in memberships {
            if *group >= group_count {
                return Err(ModelError::InvalidParameter(
                    "feature_group_index_out_of_range",
                ));
            }
            if seen[*group] {
                return Err(ModelError::InvalidParameter(
                    "duplicate_feature_group_membership",
                ));
            }
            seen[*group] = true;
        }
    }

    let log_min = energy_domain_min_mev.log10();
    let log_max = energy_domain_max_mev.log10();
    let mut energy_centres_mev = Vec::with_capacity(ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP);
    let mut length_scales_log10 = Vec::with_capacity(ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP);
    for (length_scale, centre_count) in ENERGY_BIAS_LENGTH_SCALES_LOG10
        .iter()
        .zip(ENERGY_BIAS_CENTRES_PER_SCALE.iter())
    {
        for index in 0..*centre_count {
            let fraction = index as f64 / (*centre_count - 1) as f64;
            let centre = 10.0_f64.powf(log_min + fraction * (log_max - log_min));
            energy_centres_mev.push(centre);
            length_scales_log10.push(*length_scale);
        }
    }

    let mut basis_by_observation =
        vec![vec![0.0; ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP]; energies_mev.len()];
    for (row, energy) in energies_mev.iter().enumerate() {
        let log_energy = energy.log10();
        for column in 0..ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP {
            let standardized =
                (log_energy - energy_centres_mev[column].log10()) / length_scales_log10[column];
            basis_by_observation[row][column] = (-(standardized * standardized)).exp();
        }
    }

    let total_columns = group_count
        .checked_mul(ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP)
        .ok_or(ModelError::InvalidParameter(
            "energy_bias_design_dimension_overflow",
        ))?;
    let mut grouped_design_matrix = vec![vec![0.0; total_columns]; energies_mev.len()];
    for row in 0..energies_mev.len() {
        for group in &group_memberships_by_observation[row] {
            let offset = group * ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP;
            for basis_column in 0..ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP {
                grouped_design_matrix[row][offset + basis_column] =
                    basis_by_observation[row][basis_column];
            }
        }
    }

    Ok(GroupedEnergyBiasDesign {
        energy_centres_mev,
        length_scales_log10,
        columns_per_group: ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP,
        group_count,
        basis_by_observation,
        grouped_design_matrix,
    })
}

/// Project a flattened vector of group-specific Gaussian-basis coefficients
/// into the observation-level energy-dependent log-bias field.
///
/// Coefficient ordering follows the grouped design matrix: all 160 columns for
/// group 0, followed by all 160 columns for group 1, and so on. This output can
/// be passed to the raw-scale correlated Gaussian likelihood as delta(E).
pub fn project_grouped_energy_bias_field(
    design: &GroupedEnergyBiasDesign,
    coefficients_by_group: &[f64],
) -> Result<Vec<f64>, ModelError> {
    let expected_columns = design
        .group_count
        .checked_mul(design.columns_per_group)
        .ok_or(ModelError::InvalidParameter(
            "energy_bias_design_dimension_overflow",
        ))?;
    if design.columns_per_group != ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP
        || coefficients_by_group.len() != expected_columns
        || design.basis_by_observation.len() != design.grouped_design_matrix.len()
        || design
            .grouped_design_matrix
            .iter()
            .any(|row| row.len() != expected_columns)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if coefficients_by_group.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue("energy_bias_coefficients"));
    }
    let mut bias = Vec::with_capacity(design.grouped_design_matrix.len());
    for row in &design.grouped_design_matrix {
        let value = row
            .iter()
            .zip(coefficients_by_group.iter())
            .map(|(basis, coefficient)| basis * coefficient)
            .sum::<f64>();
        if !value.is_finite() {
            return Err(ModelError::InvalidParameter(
                "energy_bias_projection_nonfinite",
            ));
        }
        bias.push(value);
    }
    Ok(bias)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_scale_and_centre_counts_produce_160_columns() {
        let design = build_grouped_energy_bias_design(
            &[0.1, 1.0, 10.0],
            &[vec![0], vec![0], vec![0]],
            1,
            0.1,
            10.0,
        )
        .expect("basis");
        assert_eq!(design.columns_per_group, 160);
        assert_eq!(design.energy_centres_mev.len(), 160);
        assert_eq!(design.length_scales_log10.len(), 160);
        assert_eq!(design.grouped_design_matrix[0].len(), 160);
        assert_eq!(&design.length_scales_log10[..100], &[0.05; 100]);
        assert_eq!(&design.length_scales_log10[100..150], &[0.15; 50]);
        assert_eq!(&design.length_scales_log10[150..160], &[0.3; 10]);
    }

    #[test]
    fn aiachne_bias_basis_matches_reference_domain_and_short_medium_long_centres() {
        let reference_grid = [0.0005, 0.01, 25.01];
        let (domain_min, domain_max) =
            aiachne_bias_basis_energy_domain(&reference_grid).expect("AIACHNE domain");
        let design = build_grouped_energy_bias_design(
            &[0.0005, 1.0, 25.01],
            &[vec![0], vec![0], vec![0]],
            1,
            domain_min,
            domain_max,
        )
        .expect("AIACHNE grouped bias basis");
        assert!((design.energy_centres_mev[0] - 0.0004).abs() < 1.0e-15);
        assert!((design.energy_centres_mev[99] - 30.01).abs() < 1.0e-12);
        assert!((design.energy_centres_mev[100] - 0.0004).abs() < 1.0e-15);
        assert!((design.energy_centres_mev[150] - 0.0004).abs() < 1.0e-15);
    }

    #[test]
    fn grouped_coefficients_map_to_short_medium_long_global_scales() {
        let indices = aiachne_global_scale_index_by_coefficient(2).expect("scale map");
        assert_eq!(indices.len(), 320);
        assert!(indices[..100].iter().all(|index| *index == 0));
        assert!(indices[100..150].iter().all(|index| *index == 1));
        assert!(indices[150..160].iter().all(|index| *index == 2));
        assert!(indices[160..260].iter().all(|index| *index == 3));
        assert!(indices[260..310].iter().all(|index| *index == 4));
        assert!(indices[310..320].iter().all(|index| *index == 5));
    }

    #[test]
    fn notebook_reference_grid_extension_is_reproduced() {
        let domain = aiachne_bias_basis_energy_domain(&[0.0005, 0.01, 25.01])
            .expect("AIACHNE basis energy domain");
        assert!((domain.0 - 0.0004).abs() < 1.0e-15);
        assert!((domain.1 - 30.01).abs() < 1.0e-12);
    }

    #[test]
    fn basis_peak_is_one_at_each_width_scale_domain_minimum() {
        let design =
            build_grouped_energy_bias_design(&[0.1], &[vec![0]], 1, 0.1, 10.0).expect("basis");
        assert!((design.basis_by_observation[0][0] - 1.0).abs() < 1.0e-12);
        assert!((design.basis_by_observation[0][100] - 1.0).abs() < 1.0e-12);
        assert!((design.basis_by_observation[0][150] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn grouped_design_activates_only_the_assigned_group_block() {
        let design =
            build_grouped_energy_bias_design(&[0.1, 1.0], &[vec![0], vec![1]], 2, 0.1, 10.0)
                .expect("grouped design");
        assert_eq!(design.grouped_design_matrix[0].len(), 320);
        assert!(design.grouped_design_matrix[0][..160]
            .iter()
            .any(|value| *value > 0.0));
        assert!(design.grouped_design_matrix[0][160..]
            .iter()
            .all(|value| *value == 0.0));
        assert!(design.grouped_design_matrix[1][..160]
            .iter()
            .all(|value| *value == 0.0));
        assert!(design.grouped_design_matrix[1][160..]
            .iter()
            .any(|value| *value > 0.0));
    }

    #[test]
    fn overlapping_feature_levels_share_observations_without_collapsing_groups() {
        let design =
            build_grouped_energy_bias_design(&[0.1, 1.0], &[vec![0, 1], vec![1]], 2, 0.1, 10.0)
                .expect("overlapping groups");
        assert_eq!(
            design.grouped_design_matrix[0][..160],
            design.grouped_design_matrix[0][160..320]
        );
        assert!(design.grouped_design_matrix[1][..160]
            .iter()
            .all(|value| *value == 0.0));
        assert!(design.grouped_design_matrix[1][160..]
            .iter()
            .any(|value| *value > 0.0));
    }

    #[test]
    fn observations_with_no_membership_have_zero_grouped_bias_row() {
        let design = build_grouped_energy_bias_design(&[0.1], &[vec![]], 2, 0.1, 10.0)
            .expect("no membership");
        assert!(design.grouped_design_matrix[0]
            .iter()
            .all(|value| *value == 0.0));
        assert!(design.basis_by_observation[0]
            .iter()
            .any(|value| *value > 0.0));
    }

    #[test]
    fn rejects_out_of_domain_energies_and_invalid_memberships() {
        assert_eq!(
            build_grouped_energy_bias_design(&[20.0], &[vec![0]], 1, 0.1, 10.0),
            Err(ModelError::InvalidParameter(
                "observation_energy_outside_frozen_bias_domain"
            ))
        );
        assert_eq!(
            build_grouped_energy_bias_design(&[1.0], &[vec![1]], 1, 0.1, 10.0),
            Err(ModelError::InvalidParameter(
                "feature_group_index_out_of_range"
            ))
        );
        assert_eq!(
            build_grouped_energy_bias_design(&[1.0], &[vec![0, 0]], 1, 0.1, 10.0),
            Err(ModelError::InvalidParameter(
                "duplicate_feature_group_membership"
            ))
        );
    }

    #[test]
    fn rejects_invalid_energy_domain_and_nonpositive_energy() {
        assert_eq!(
            build_grouped_energy_bias_design(&[1.0], &[vec![0]], 1, 0.1, 0.1),
            Err(ModelError::InvalidParameter(
                "energy_bias_domain_must_be_positive_and_ordered"
            ))
        );
        assert_eq!(
            build_grouped_energy_bias_design(&[0.0], &[vec![0]], 1, 0.1, 10.0),
            Err(ModelError::NonPositiveValue("observation_energy_mev"))
        );
    }

    #[test]
    fn projects_grouped_coefficients_to_observation_log_bias() {
        let design =
            build_grouped_energy_bias_design(&[0.1, 10.0], &[vec![0], vec![1]], 2, 0.1, 10.0)
                .expect("design");
        let mut coefficients = vec![0.0; 320];
        coefficients[0] = 0.5;
        coefficients[160 + 159] = -0.25;
        let bias =
            project_grouped_energy_bias_field(&design, &coefficients).expect("projected bias");
        assert!((bias[0] - 0.5).abs() < 1.0e-12);
        assert!((bias[1] + 0.25).abs() < 1.0e-12);
    }

    #[test]
    fn empty_feature_membership_projects_to_zero_bias() {
        let design = build_grouped_energy_bias_design(&[0.1, 1.0], &[vec![], vec![]], 1, 0.1, 10.0)
            .expect("design");
        let bias = project_grouped_energy_bias_field(
            &design,
            &vec![1.0; ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP],
        )
        .expect("zero projection");
        assert_eq!(bias, vec![0.0, 0.0]);
    }

    #[test]
    fn projection_rejects_wrong_coefficient_size_and_nonfinite_values() {
        let design =
            build_grouped_energy_bias_design(&[1.0], &[vec![0]], 1, 0.1, 10.0).expect("design");
        assert_eq!(
            project_grouped_energy_bias_field(&design, &[0.0]),
            Err(ModelError::MismatchedDataLengths)
        );
        let mut coefficients = vec![0.0; ENERGY_BIAS_BASIS_COLUMNS_PER_GROUP];
        coefficients[0] = f64::NAN;
        assert_eq!(
            project_grouped_energy_bias_field(&design, &coefficients),
            Err(ModelError::NonFiniteValue("energy_bias_coefficients"))
        );
    }
}
