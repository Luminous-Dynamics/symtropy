// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic CSV demonstration of a perturbation-aware row-rank certificate.

use inertial_contact_rank_stability::{
    RankStabilityInput, StabilityConfig, certify_rank_stability,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = RankStabilityInput {
        candidate_set_digest: "synthetic-candidate-set-uncertainty-001".to_owned(),
        mass_properties_digest: "synthetic-mass-properties-uncertainty-001".to_owned(),
        uncertainty_profile_digest: "mass-and-jacobian-envelope-001".to_owned(),
        contact_ids: vec![
            "support-left".to_owned(),
            "support-right".to_owned(),
            "inactive-rear".to_owned(),
        ],
        whitened_rows: vec![
            vec![3.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.2],
        ],
        relative_mass_perturbation_bound: 0.01,
        whitened_jacobian_perturbation_bound: 0.005,
        nominal_operator_norm_upper: 3.0,
        absolute_rank_floor: 0.5,
    };

    let report = certify_rank_stability(&input, StabilityConfig::default())?;

    println!(
        "schema,candidate_set,uncertainty_profile,status,nominal_rank,robust_rank,epsilon_s,rank_floor,basis_id_stability,physical_omission_certified"
    );
    println!(
        "{},{},{},{:?},{},{},{:.9},{:.9},{:?},{}",
        report.schema_version,
        report.candidate_set_digest,
        report.uncertainty_profile_digest,
        report.status,
        report.nominal_spectral_rank,
        report
            .robust_spectral_rank
            .map(|rank| rank.to_string())
            .unwrap_or_else(|| "INCONCLUSIVE".to_owned()),
        report.whitened_operator_perturbation_bound,
        report.absolute_rank_floor,
        report.basis_id_stability,
        report.physical_omission_certified,
    );

    println!("index,nominal_singular,lower,upper,disposition");
    for interval in report.singular_value_intervals {
        println!(
            "{},{:.9},{:.9},{:.9},{:?}",
            interval.index,
            interval.nominal_value,
            interval.lower_bound,
            interval.upper_bound,
            interval.disposition,
        );
    }
    Ok(())
}
