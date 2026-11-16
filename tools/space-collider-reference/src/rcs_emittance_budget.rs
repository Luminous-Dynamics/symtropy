// SPDX-License-Identifier: AGPL-3.0-or-later

#[derive(Debug, Clone, Copy, PartialEq)]
struct StageGrowth {
    id: &'static str,
    horizontal_pct: f64,
    vertical_pct: f64,
    longitudinal_pct: f64,
    max_horizontal_orbit_offset_mm: f64,
    max_horizontal_beta_beating_pct: f64,
    max_vertical_beta_beating_pct: f64,
}

const STAGES: [StageGrowth; 4] = [
    StageGrowth {
        id: "RCS1",
        horizontal_pct: 6.8,
        vertical_pct: -0.2,
        longitudinal_pct: 3.4,
        max_horizontal_orbit_offset_mm: 4.1,
        max_horizontal_beta_beating_pct: 3.3,
        max_vertical_beta_beating_pct: 1.4,
    },
    StageGrowth {
        id: "RCS2",
        horizontal_pct: 0.4,
        vertical_pct: -0.3,
        longitudinal_pct: 2.8,
        max_horizontal_orbit_offset_mm: 0.5,
        max_horizontal_beta_beating_pct: 0.4,
        max_vertical_beta_beating_pct: 0.5,
    },
    StageGrowth {
        id: "RCS3",
        horizontal_pct: 1.0,
        vertical_pct: -0.1,
        longitudinal_pct: -0.5,
        max_horizontal_orbit_offset_mm: 0.4,
        max_horizontal_beta_beating_pct: 0.3,
        max_vertical_beta_beating_pct: 0.1,
    },
    StageGrowth {
        id: "RCS4",
        horizontal_pct: 1.5,
        vertical_pct: 0.5,
        longitudinal_pct: 1.5,
        max_horizontal_orbit_offset_mm: 1.0,
        max_horizontal_beta_beating_pct: 1.0,
        max_vertical_beta_beating_pct: 3.0,
    },
];

const HORIZONTAL_EMITTANCE_BUDGET_PCT: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct StageComposition {
    stage: StageGrowth,
    factor_after: f64,
    incremental_growth_from_initial_pct: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BudgetClosure {
    additive_growth_pct: f64,
    multiplicative_growth_pct: f64,
    budget_pct: f64,
    budget_consumption_pct: f64,
    remaining_margin_percentage_points: f64,
    status: &'static str,
    stages: [StageComposition; 4],
}

fn validate_stage_set(stages: &[StageGrowth]) -> Result<(), String> {
    if stages.len() != 4 {
        return Err(format!(
            "the Table III reference requires exactly four ordered RCS stages; found {}",
            stages.len()
        ));
    }

    for (index, (stage, expected_id)) in stages
        .iter()
        .zip(["RCS1", "RCS2", "RCS3", "RCS4"])
        .enumerate()
    {
        if stage.id != expected_id {
            return Err(format!(
                "stage {index} must be {expected_id}, found {}",
                stage.id
            ));
        }

        for (name, value) in [
            ("horizontal emittance growth", stage.horizontal_pct),
            ("vertical emittance growth", stage.vertical_pct),
            ("longitudinal emittance growth", stage.longitudinal_pct),
            (
                "horizontal orbit offset",
                stage.max_horizontal_orbit_offset_mm,
            ),
            (
                "horizontal beta beating",
                stage.max_horizontal_beta_beating_pct,
            ),
            ("vertical beta beating", stage.max_vertical_beta_beating_pct),
        ] {
            if !value.is_finite() {
                return Err(format!("{} {name} must be finite", stage.id));
            }
        }

        if stage.horizontal_pct <= -100.0 {
            return Err(format!(
                "{} horizontal emittance change must be greater than -100%",
                stage.id
            ));
        }
        if stage.vertical_pct <= -100.0 || stage.longitudinal_pct <= -100.0 {
            return Err(format!(
                "{} transverse/longitudinal emittance change must be greater than -100%",
                stage.id
            ));
        }
        if stage.max_horizontal_orbit_offset_mm < 0.0
            || stage.max_horizontal_beta_beating_pct < 0.0
            || stage.max_vertical_beta_beating_pct < 0.0
        {
            return Err(format!(
                "{} orbit and beta-beating magnitudes must be non-negative",
                stage.id
            ));
        }
    }

    Ok(())
}

fn compute_budget_closure(
    stages: &[StageGrowth],
    budget_pct: f64,
) -> Result<BudgetClosure, String> {
    validate_stage_set(stages)?;

    if !budget_pct.is_finite() || budget_pct <= 0.0 {
        return Err("horizontal emittance budget must be finite and positive".to_owned());
    }

    let mut additive_growth_pct = 0.0;
    let mut multiplicative_factor = 1.0;
    let mut composed_stages = [StageComposition {
        stage: STAGES[0],
        factor_after: 1.0,
        incremental_growth_from_initial_pct: 0.0,
    }; 4];

    for (index, stage) in stages.iter().copied().enumerate() {
        let factor_before = multiplicative_factor;
        let stage_factor = 1.0 + stage.horizontal_pct / 100.0;
        if !stage_factor.is_finite() || stage_factor <= 0.0 {
            return Err(format!(
                "{} produces an invalid multiplicative factor",
                stage.id
            ));
        }

        multiplicative_factor *= stage_factor;
        if !multiplicative_factor.is_finite() {
            return Err("cumulative multiplicative factor is not finite".to_owned());
        }

        additive_growth_pct += stage.horizontal_pct;
        composed_stages[index] = StageComposition {
            stage,
            factor_after: multiplicative_factor,
            incremental_growth_from_initial_pct: (multiplicative_factor - factor_before) * 100.0,
        };
    }

    let multiplicative_growth_pct = (multiplicative_factor - 1.0) * 100.0;
    let budget_consumption_pct = multiplicative_growth_pct / budget_pct * 100.0;
    let remaining_margin_percentage_points = budget_pct - multiplicative_growth_pct;
    let status = if remaining_margin_percentage_points >= 0.0 {
        "within_conditional_arithmetic_budget"
    } else {
        "exceeds_conditional_arithmetic_budget"
    };

    Ok(BudgetClosure {
        additive_growth_pct,
        multiplicative_growth_pct,
        budget_pct,
        budget_consumption_pct,
        remaining_margin_percentage_points,
        status,
        stages: composed_stages,
    })
}

pub(super) fn render_report_json() -> Result<String, String> {
    let closure = compute_budget_closure(&STAGES, HORIZONTAL_EMITTANCE_BUDGET_PCT)?;

    let stage_rows = closure
        .stages
        .iter()
        .map(|row| {
            format!(
                concat!(
                    "{{",
                    "\"stage\":\"{}\",",
                    "\"horizontal_emittance_growth_pct\":{:.6},",
                    "\"vertical_emittance_growth_pct\":{:.6},",
                    "\"longitudinal_emittance_growth_pct\":{:.6},",
                    "\"max_horizontal_orbit_offset_mm\":{:.6},",
                    "\"max_horizontal_beta_beating_pct\":{:.6},",
                    "\"max_vertical_beta_beating_pct\":{:.6},",
                    "\"sequential_factor_after_stage\":{:.9},",
                    "\"incremental_compound_growth_from_initial_pct\":{:.9}",
                    "}}"
                ),
                row.stage.id,
                row.stage.horizontal_pct,
                row.stage.vertical_pct,
                row.stage.longitudinal_pct,
                row.stage.max_horizontal_orbit_offset_mm,
                row.stage.max_horizontal_beta_beating_pct,
                row.stage.max_vertical_beta_beating_pct,
                row.factor_after,
                row.incremental_growth_from_initial_pct,
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    Ok(format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"evidence_class\": \"source_table_arithmetic_screen\",\n",
            "  \"source\": {{\n",
            "    \"title\": \"Impact of Rapid Acceleration on Beam Dynamics in the Rapid-Cycling Synchrotrons of a Muon Collider\",\n",
            "    \"authors\": \"Lisa Soubirou; Antoine Chancé\",\n",
            "    \"arxiv_id\": \"2611.02865v1\",\n",
            "    \"table\": \"Table III\",\n",
            "    \"url\": \"https://arxiv.org/abs/2611.02865\"\n",
            "  }},\n",
            "  \"case_separation\": {{\n",
            "    \"energy_field_mismatch\": \"Table III 32-arc stage-local results used for the conditional budget arithmetic below\",\n",
            "    \"rf_distribution_comparison\": {{\n",
            "      \"eight_arc_horizontal_emittance_growth_gt_pct\": 500.0,\n",
            "      \"thirty_two_arc_horizontal_emittance_growth_pct\": 13.0,\n",
            "      \"second_order_matching\": \"source reports substantial suppression of the residual in the 32-arc case; it is not combined with Table III values\"\n",
            "    }}\n",
            "  }},\n",
            "  \"conditional_chain_budget\": {{\n",
            "    \"composition_status\": \"conditional_sequential_composition\",\n",
            "    \"physical_qualification_status\": \"not_evaluated\",\n",
            "    \"budget_pct\": {:.6},\n",
            "    \"stages\": [\n",
            "{}\n",
            "    ],\n",
            "    \"derived\": {{\n",
            "      \"additive_first_order_growth_pct\": {:.9},\n",
            "      \"multiplicative_compound_growth_pct\": {:.9},\n",
            "      \"budget_consumption_pct\": {:.9},\n",
            "      \"remaining_margin_percentage_points\": {:.9},\n",
            "      \"budget_status\": \"{}\"\n",
            "    }},\n",
            "    \"formula\": \"G_chain = product_i(1 + g_i / 100) - 1\",\n",
            "    \"assumptions\": [\n",
            "      \"Each reported Table III horizontal relative growth is applied sequentially to the same propagated emittance definition.\",\n",
            "      \"Inter-stage rematching, correction, distribution change and cross-stage correlations are not represented.\",\n",
            "      \"The stage-local Table III cases are not asserted to be one integrated start-to-end tracking run.\"\n",
            "    ],\n",
            "    \"limitations\": [\n",
            "      \"Conditional arithmetic only; not full-chain beam tracking.\",\n",
            "      \"Negative vertical/longitudinal emittance changes are preserved and are not interpreted as genuine cooling.\",\n",
            "      \"Particle losses, acceptance and rematching can change the interpretation of reported emittance evolution.\"\n",
            "    ]\n",
            "  }},\n",
            "  \"ramp_jitter_source_observations\": {{\n",
            "    \"nominal_jitter_ppm\": 100.0,\n",
            "    \"additional_emittance_growth_at_100_ppm\": \"reported negligible relative to nominal fast-ramp case\",\n",
            "    \"orbit_distortion_increase_at_100_ppm_mm\": 0.5,\n",
            "    \"orbit_distortion_increase_at_500_ppm_mm\": 3.0,\n",
            "    \"beta_beating_relative_increase_at_100_ppm_pct\": 20.0,\n",
            "    \"beta_beating_relative_increase_at_500_ppm_pct\": 100.0,\n",
            "    \"claim_ceiling\": \"source-reported sensitivity summaries; not a new jitter simulation\"\n",
            "  }},\n",
            "  \"claim_ceiling\": \"source transcription and conditional arithmetic only; no RCS tracking reproduction, inter-stage propagation, beam transmission or physical acceleration qualification\"\n",
            "}}\n"
        ),
        closure.budget_pct,
        stage_rows,
        closure.additive_growth_pct,
        closure.multiplicative_growth_pct,
        closure.budget_consumption_pct,
        closure.remaining_margin_percentage_points,
        closure.status,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual}, expected {expected}, tolerance {tolerance}"
        );
    }

    #[test]
    fn table_iii_stage_values_match_source_transcription() {
        assert_eq!(
            STAGES.map(|stage| stage.id),
            ["RCS1", "RCS2", "RCS3", "RCS4"]
        );
        assert_eq!(
            STAGES.map(|stage| stage.horizontal_pct),
            [6.8, 0.4, 1.0, 1.5]
        );
        assert_eq!(
            STAGES.map(|stage| stage.max_horizontal_orbit_offset_mm),
            [4.1, 0.5, 0.4, 1.0]
        );
    }

    #[test]
    fn conditional_multiplicative_composition_matches_independent_oracle() {
        let closure = compute_budget_closure(&STAGES, 10.0).expect("valid source stages");
        close(closure.additive_growth_pct, 9.7, 1e-12);
        close(closure.multiplicative_growth_pct, 9.923_964_08, 1e-10);
        close(closure.budget_consumption_pct, 99.239_640_8, 1e-9);
        close(
            closure.remaining_margin_percentage_points,
            0.076_035_92,
            1e-10,
        );
        assert_eq!(closure.status, "within_conditional_arithmetic_budget");
    }

    #[test]
    fn negative_vertical_and_longitudinal_changes_are_preserved() {
        assert_eq!(STAGES[0].vertical_pct, -0.2);
        assert_eq!(STAGES[0].longitudinal_pct, 3.4);
        assert_eq!(STAGES[2].longitudinal_pct, -0.5);
        let report = render_report_json().expect("valid report");
        assert!(report.contains("\"vertical_emittance_growth_pct\":-0.200000"));
        assert!(report.contains("\"longitudinal_emittance_growth_pct\":-0.500000"));
    }

    #[test]
    fn missing_or_duplicate_stage_is_rejected() {
        assert!(validate_stage_set(&STAGES[..3]).is_err());
        let mut duplicated = STAGES;
        duplicated[3].id = "RCS3";
        assert!(validate_stage_set(&duplicated).is_err());
    }

    #[test]
    fn invalid_growth_and_budget_inputs_are_rejected() {
        let mut invalid = STAGES;
        invalid[0].horizontal_pct = -100.0;
        assert!(compute_budget_closure(&invalid, 10.0).is_err());
        assert!(compute_budget_closure(&STAGES, 0.0).is_err());
        assert!(compute_budget_closure(&STAGES, f64::NAN).is_err());
    }

    #[test]
    fn report_is_explicitly_conditional_not_physically_qualified() {
        let report = render_report_json().expect("valid report");
        assert!(report.contains("\"composition_status\": \"conditional_sequential_composition\""));
        assert!(report.contains("\"physical_qualification_status\": \"not_evaluated\""));
        assert!(report.contains("\"energy_field_mismatch\""));
        assert!(report.contains("\"rf_distribution_comparison\""));
    }
}
