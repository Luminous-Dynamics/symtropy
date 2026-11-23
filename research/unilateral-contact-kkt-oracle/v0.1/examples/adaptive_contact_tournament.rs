// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic synthetic demonstration of omitted-contact KKT refinement.

use unilateral_contact_kkt_oracle::{
    AdaptiveConfig, UnilateralContactQp, adaptive_refine,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let problem = UnilateralContactQp {
        contact_ids: vec![
            "support-left".to_owned(),
            "support-right".to_owned(),
            "inactive-rear".to_owned(),
        ],
        delassus: vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ],
        linear_term: vec![-2.0, -1.0, 0.5],
    };
    let report = adaptive_refine(
        &problem,
        &["support-left".to_owned()],
        AdaptiveConfig::default(),
    )?;

    println!(
        "schema,status,rounds,active_set_subsets,objective,max_primal_violation,max_dual_violation,max_complementarity,min_omitted_gradient,retained_contact_ids,impulses"
    );
    let retained = report
        .refinement_rounds
        .last()
        .map(|round| round.retained_contact_ids.join("|"))
        .unwrap_or_default();
    let impulses = report
        .final_impulses
        .iter()
        .map(|item| format!("{}:{:.6}", item.contact_id, item.normal_impulse))
        .collect::<Vec<_>>()
        .join("|");
    let minimum_omitted = report
        .final_certificate
        .minimum_omitted_gradient
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "none".to_owned());

    println!(
        "{},{:?},{},{},{:.6},{:.6},{:.6},{:.6},{},{},{}",
        unilateral_contact_kkt_oracle::SCHEMA_VERSION,
        report.status,
        report.refinement_rounds.len(),
        report.active_set_subsets_evaluated,
        report
            .refinement_rounds
            .last()
            .map(|round| round.objective_value)
            .unwrap_or(0.0),
        report.final_certificate.maximum_primal_violation,
        report.final_certificate.maximum_dual_violation,
        report.final_certificate.maximum_complementarity_residual,
        minimum_omitted,
        retained,
        impulses,
    );
    Ok(())
}
