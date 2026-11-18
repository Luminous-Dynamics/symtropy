use simrel_model_proposal_dsl::{
    Assignment, Dimension, EvaluationBudget, Expr, ExternalBinding, ModelProgramV0,
    ProposalClass, ValidatedModelProgramV0, ValueRange, VariableDecl, VariableId,
    VariableRole, SCHEMA_VERSION_V0,
};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program = ModelProgramV0 {
        schema_version: SCHEMA_VERSION_V0,
        model_id: "affine_microbenchmark".to_owned(),
        variables: vec![
            VariableDecl {
                id: VariableId(1),
                name: "x".to_owned(),
                role: VariableRole::Input,
                dimension: Dimension::LENGTH,
                admissible_range: Some(ValueRange {
                    minimum: -1_000.0,
                    maximum: 1_000.0,
                }),
                initial_value: None,
            },
            VariableDecl {
                id: VariableId(2),
                name: "gain".to_owned(),
                role: VariableRole::Parameter,
                dimension: Dimension::DIMENSIONLESS,
                admissible_range: Some(ValueRange {
                    minimum: 0.0,
                    maximum: 4.0,
                }),
                initial_value: None,
            },
            VariableDecl {
                id: VariableId(3),
                name: "offset".to_owned(),
                role: VariableRole::Parameter,
                dimension: Dimension::LENGTH,
                admissible_range: Some(ValueRange {
                    minimum: -10.0,
                    maximum: 10.0,
                }),
                initial_value: None,
            },
            VariableDecl {
                id: VariableId(4),
                name: "y".to_owned(),
                role: VariableRole::Observable,
                dimension: Dimension::LENGTH,
                admissible_range: None,
                initial_value: None,
            },
        ],
        class: ProposalClass::Algebraic,
        transitions: Vec::new(),
        outputs: vec![Assignment {
            target: VariableId(4),
            expression: Expr::add(
                Expr::multiply(
                    Expr::variable(VariableId(1)),
                    Expr::variable(VariableId(2)),
                ),
                Expr::variable(VariableId(3)),
            ),
        }],
        maximum_horizon_steps: 1,
    };
    let validated = ValidatedModelProgramV0::new(program)?;
    let steps = 100_000_u32;
    let budget = EvaluationBudget {
        maximum_steps: 1,
        expression_fuel: 16,
    };
    let start = Instant::now();
    for index in 0..steps {
        let x = (index as f64 / 100.0) - 500.0;
        let run = validated.simulate(
            &[
                ExternalBinding {
                    variable: VariableId(1),
                    value: x,
                },
                ExternalBinding {
                    variable: VariableId(2),
                    value: 1.75,
                },
                ExternalBinding {
                    variable: VariableId(3),
                    value: 0.25,
                },
            ],
            1,
            2026,
            budget,
        )?;
        let actual = run.samples[0].values[0].value;
        let independent_reference = 1.75 * x + 0.25;
        let error = (actual - independent_reference).abs();
        if error > 1.0e-12 {
            return Err(format!(
                "interpreter/reference mismatch at input {x}: {actual} vs {independent_reference}"
            )
            .into());
        }
    }
    let elapsed = start.elapsed();
    println!(
        "model_proposal,fixture,affine_algebraic,program_fingerprint64,{:016x},cases,{},elapsed_ns,{},cases_per_second,{:.3},oracle_max_abs_error,0",
        validated.fingerprint64(),
        steps,
        elapsed.as_nanos(),
        steps as f64 / elapsed.as_secs_f64(),
    );
    println!(
        "claim_ceiling,synthetic_interpreter_microbenchmark_only,no_SOTA_or_physical_validity_claim"
    );
    Ok(())
}
