// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Emit candidate singular-spectrum receipts for the frozen exact-rational oracle cases.

use std::error::Error;

use metric_induced_error_envelope_reference::{
    CartesianQ, DenseY, Matrix, Metric, MetricProfileV1,
    operator::{OperatorConditionProfileV1, OperatorConditionReceiptV1, TangentToTangentOperator},
    rectangular_operator::RectangularTangentToTangentOperator,
};

fn identity_matrix<const N: usize>() -> Matrix<N> {
    std::array::from_fn(|row| std::array::from_fn(|column| f64::from(row == column)))
}

fn identity_metric<Frame: metric_induced_error_envelope_reference::CoordinateFrame, const N: usize>()
    -> Metric<Frame, N>
{
    Metric::admit(identity_matrix(), MetricProfileV1::default())
        .expect("identity metric is SPD")
}

fn profile() -> OperatorConditionProfileV1 {
    OperatorConditionProfileV1 {
        relative_jacobi_tolerance: 1.0e-13,
        relative_rank_tolerance: 1.0e-14,
        maximum_condition_estimate: 1.0e15,
        maximum_jacobi_sweeps: 64,
    }
}

fn json_number(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.17e}")
    } else {
        "null".to_owned()
    }
}

fn json_optional(value: Option<f64>) -> String {
    value.map(json_number).unwrap_or_else(|| "null".to_owned())
}

fn json_singular_values(values: &[f64]) -> String {
    format!(
        "[{}]",
        values.iter().map(|value| json_number(*value)).collect::<Vec<_>>().join(",")
    )
}

fn square_row<const N: usize>(
    fixture_id: &str,
    receipt: &OperatorConditionReceiptV1,
) -> String {
    let rank = receipt.numerical_rank;
    let nonzero_condition = if rank > 0 {
        Some(receipt.singular_values[0] / receipt.singular_values[rank - 1])
    } else {
        None
    };
    format!(
        concat!(
            "{{\"fixture_id\":\"{}\",\"singular_values\":{},",
            "\"numerical_rank\":{},\"domain_nullity\":{},\"codomain_nullity\":{},",
            "\"condition_estimate\":{},\"nonzero_spectrum_condition_estimate\":{},",
            "\"status\":\"{:?}\"}}"
        ),
        fixture_id,
        json_singular_values(&receipt.singular_values),
        rank,
        N - rank,
        N - rank,
        json_optional(receipt.condition_estimate),
        json_optional(nonzero_condition),
        receipt.status
    )
}

fn rectangular_row(
    fixture_id: &str,
    receipt: &metric_induced_error_envelope_reference::rectangular_operator::RectangularOperatorConditionReceiptV1,
) -> String {
    format!(
        concat!(
            "{{\"fixture_id\":\"{}\",\"singular_values\":{},",
            "\"numerical_rank\":{},\"domain_nullity\":{},\"codomain_nullity\":{},",
            "\"condition_estimate\":{},\"nonzero_spectrum_condition_estimate\":{},",
            "\"status\":\"{:?}\"}}"
        ),
        fixture_id,
        json_singular_values(&receipt.singular_values),
        receipt.numerical_rank,
        receipt.domain_nullity,
        receipt.codomain_nullity,
        json_optional(receipt.condition_estimate),
        json_optional(receipt.nonzero_spectrum_condition_estimate),
        receipt.status
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let profile = profile();
    let mut rows = Vec::new();

    let d2 = identity_metric::<CartesianQ, 2>();
    let c2 = identity_metric::<DenseY, 2>();
    let weighted = TangentToTangentOperator::<CartesianQ, DenseY, 2>::new([
        [3.0, 0.0],
        [0.0, 4.0],
    ])?;
    rows.push(square_row::<2>(
        "weighted_diagonal_2x2",
        &weighted.assess(&d2, &c2, profile)?,
    ));

    let dense = TangentToTangentOperator::<CartesianQ, DenseY, 2>::new([
        [1.0, 2.0],
        [3.0, 4.0],
    ])?;
    rows.push(square_row::<2>(
        "dense_exact_2x2",
        &dense.assess(&d2, &c2, profile)?,
    ));

    let d3 = identity_metric::<CartesianQ, 3>();
    let c3 = identity_metric::<DenseY, 3>();
    let repeated = TangentToTangentOperator::<CartesianQ, DenseY, 3>::new([
        [5.0, 0.0, 0.0],
        [0.0, 5.0, 0.0],
        [0.0, 0.0, 1.0],
    ])?;
    rows.push(square_row::<3>(
        "repeated_spectrum_3x3",
        &repeated.assess(&d3, &c3, profile)?,
    ));

    let rank_deficient = TangentToTangentOperator::<CartesianQ, DenseY, 2>::new([
        [1.0, 2.0],
        [2.0, 4.0],
    ])?;
    rows.push(square_row::<2>(
        "rank_deficient_square_2x2",
        &rank_deficient.assess(&d2, &c2, profile)?,
    ));

    let scaled = TangentToTangentOperator::<CartesianQ, DenseY, 3>::new([
        [1.0e12, 0.0, 0.0],
        [0.0, 1.0e6, 0.0],
        [0.0, 0.0, 1.0],
    ])?;
    rows.push(square_row::<3>(
        "scaled_full_rank_3x3",
        &scaled.assess(&d3, &c3, profile)?,
    ));

    let d3_rect = identity_metric::<CartesianQ, 3>();
    let c2_rect = identity_metric::<DenseY, 2>();
    let wide = RectangularTangentToTangentOperator::<CartesianQ, DenseY, 2, 3>::new([
        [1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
    ])?;
    rows.push(rectangular_row(
        "full_row_rank_wide_2x3",
        &wide.assess(&d3_rect, &c2_rect, profile)?,
    ));

    let d2_rect = identity_metric::<CartesianQ, 2>();
    let c3_rect = identity_metric::<DenseY, 3>();
    let tall = RectangularTangentToTangentOperator::<CartesianQ, DenseY, 3, 2>::new([
        [1.0, 0.0],
        [0.0, 2.0],
        [0.0, 0.0],
    ])?;
    rows.push(rectangular_row(
        "full_column_rank_tall_3x2",
        &tall.assess(&d2_rect, &c3_rect, profile)?,
    ));

    let rank_deficient_wide =
        RectangularTangentToTangentOperator::<CartesianQ, DenseY, 2, 3>::new([
            [1.0, 2.0, 3.0],
            [2.0, 4.0, 6.0],
        ])?;
    rows.push(rectangular_row(
        "rank_deficient_wide_2x3",
        &rank_deficient_wide.assess(&d3_rect, &c2_rect, profile)?,
    ));

    let source_commit = std::env::var("OPERATOR_ORACLE_SOURCE_SHA")
        .or_else(|_| std::env::var("GITHUB_SHA"))
        .unwrap_or_else(|_| "local".to_owned());
    let report = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"luminous.operator_spectrum_candidate.v0.1\",\n",
            "  \"source_commit\": \"{}\",\n",
            "  \"profile\": {{\"relative_jacobi_tolerance\":1.0e-13,",
            "\"relative_rank_tolerance\":1.0e-14,",
            "\"maximum_condition_estimate\":1.0e15,\"maximum_jacobi_sweeps\":64}},\n",
            "  \"measurements\": [\n    {}\n  ]\n",
            "}}\n"
        ),
        source_commit,
        rows.join(",\n    ")
    );

    if let Some(path) = std::env::args().nth(1) {
        std::fs::write(&path, &report)?;
    }
    print!("{report}");
    Ok(())
}
