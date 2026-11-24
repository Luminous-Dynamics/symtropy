// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic demonstration of inertia-whitened contact-row rank selection.
//! The selected rows are a numerical seed only, not physically certified drops.

use inertial_contact_rank_oracle::{
    InertialRankInput, RankSelectionConfig, select_inertial_rank_basis,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = InertialRankInput {
        candidate_set_digest: "synthetic-candidate-set-001".to_owned(),
        mass_properties_digest: "synthetic-mass-properties-001".to_owned(),
        contact_ids: vec![
            "base-contact".to_owned(),
            "tip-contact".to_owned(),
            "side-contact".to_owned(),
            "duplicate-base".to_owned(),
        ],
        mass_matrix: vec![vec![4.0, 0.0], vec![0.0, 1.0]],
        normal_jacobian: vec![
            vec![2.0, 0.0],
            vec![4.0, 0.0],
            vec![0.0, 1.0],
            vec![-2.0, 0.0],
        ],
    };
    let report = select_inertial_rank_basis(&input, RankSelectionConfig::default())?;

    println!(
        "schema,candidate_set,rank,candidates,first_pivot,rank_threshold,max_projection_residual,physical_omission_certified,selected_ids,excluded_ids"
    );
    println!(
        "{},{},{},{},{:.9},{:.9},{:.9},{},{},{}",
        report.schema_version,
        report.candidate_set_digest,
        report.selected_rank,
        report.candidate_count,
        report.first_pivot_norm,
        report.rank_threshold,
        report.maximum_projection_residual,
        report.physical_omission_certified,
        report.selected_basis_contact_ids.join("|"),
        report.excluded_contact_ids.join("|"),
    );

    println!("contact_id,inertial_row_norm,projection_residual,relative_residual,disposition");
    for row in report.row_diagnostics {
        println!(
            "{},{:.9},{:.9},{}, {:?}",
            row.contact_id,
            row.inertial_row_norm,
            row.final_projection_residual,
            row.relative_projection_residual
                .map(|value| format!("{value:.9}"))
                .unwrap_or_else(|| "none".to_owned()),
            row.disposition,
        );
    }

    Ok(())
}
