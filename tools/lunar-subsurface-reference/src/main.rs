use std::process::ExitCode;

const CIRCUMFERENCE_CASES_KM: [f64; 3] = [50.0, 100.0, 150.0];
const EXCAVATED_SECTION_CASES_M2: [f64; 4] = [10.0, 20.0, 30.0, 40.0];
const BORE_COUNT_CASES: [u32; 2] = [1, 2];
const OVERBREAK_CASES: [f64; 3] = [0.0, 0.10, 0.20];

#[derive(Debug, Clone, Copy, Default)]
struct AncillaryVolumes {
    shafts_m3: f64,
    caverns_m3: f64,
    cross_passages_m3: f64,
}

#[derive(Debug, Clone, Copy)]
struct ExcavationCase {
    circumference_km: f64,
    excavated_section_m2: f64,
    bore_count: u32,
    overbreak_fraction: f64,
    ancillary: AncillaryVolumes,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ExcavationEstimate {
    design_bore_volume_m3: f64,
    overbreak_volume_m3: f64,
    shaft_volume_m3: f64,
    cavern_volume_m3: f64,
    cross_passage_volume_m3: f64,
    ancillary_volume_m3: f64,
    total_excavation_volume_m3: f64,
}

fn calculate_excavation(case: ExcavationCase) -> Result<ExcavationEstimate, String> {
    if !case.circumference_km.is_finite() || case.circumference_km <= 0.0 {
        return Err("circumference must be finite and positive".to_owned());
    }
    if !case.excavated_section_m2.is_finite() || case.excavated_section_m2 <= 0.0 {
        return Err("excavated cross-section must be finite and positive".to_owned());
    }
    if case.bore_count == 0 {
        return Err("bore count must be at least one".to_owned());
    }
    if !case.overbreak_fraction.is_finite() || !(0.0..=1.0).contains(&case.overbreak_fraction) {
        return Err("overbreak fraction must be finite and in [0, 1]".to_owned());
    }

    let ancillary = [
        case.ancillary.shafts_m3,
        case.ancillary.caverns_m3,
        case.ancillary.cross_passages_m3,
    ];
    if ancillary
        .iter()
        .any(|volume| !volume.is_finite() || *volume < 0.0)
    {
        return Err("ancillary volumes must be finite and non-negative".to_owned());
    }

    let circumference_m = case.circumference_km * 1_000.0;
    let design_bore_volume_m3 =
        circumference_m * case.excavated_section_m2 * f64::from(case.bore_count);
    let overbreak_volume_m3 = design_bore_volume_m3 * case.overbreak_fraction;
    let ancillary_volume_m3 =
        case.ancillary.shafts_m3 + case.ancillary.caverns_m3 + case.ancillary.cross_passages_m3;
    let total_excavation_volume_m3 =
        design_bore_volume_m3 + overbreak_volume_m3 + ancillary_volume_m3;

    let derived = [
        design_bore_volume_m3,
        overbreak_volume_m3,
        ancillary_volume_m3,
        total_excavation_volume_m3,
    ];
    if derived
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("derived excavation volume is non-finite or negative".to_owned());
    }

    Ok(ExcavationEstimate {
        design_bore_volume_m3,
        overbreak_volume_m3,
        shaft_volume_m3: case.ancillary.shafts_m3,
        cavern_volume_m3: case.ancillary.caverns_m3,
        cross_passage_volume_m3: case.ancillary.cross_passages_m3,
        ancillary_volume_m3,
        total_excavation_volume_m3,
    })
}

fn render_report_json() -> Result<String, String> {
    let scenario_count = CIRCUMFERENCE_CASES_KM.len()
        * EXCAVATED_SECTION_CASES_M2.len()
        * BORE_COUNT_CASES.len()
        * OVERBREAK_CASES.len();

    let mut report = String::new();
    report.push_str(
        concat!(
            "{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"evidence_class\": \"geometry_only_excavation_volume_screen\",\n",
            "  \"units\": {\n",
            "    \"circumference\": \"km (converted to m internally)\",\n",
            "    \"excavated_cross_section\": \"m^2\",\n",
            "    \"volume\": \"m^3\"\n",
            "  },\n",
            "  \"assumptions\": [\n",
            "    \"Design bore volume = circumference_m * excavated_cross_section_m2 * bore_count.\",\n",
            "    \"Overbreak is an explicit fraction of design bore volume.\",\n",
            "    \"Shaft, cavern and cross-passage volumes are separate additive inputs.\",\n",
            "    \"The baseline matrix sets ancillary volumes to zero because no layout is selected.\",\n",
            "    \"No spoil density, swell, excavation energy, advance rate, cost, support or geotechnical performance is inferred.\",\n",
            "    \"Cross-section and overbreak values are analyst-selected sensitivity inputs, not lunar design standards.\"\n",
            "  ],\n"
        ),
    );
    report.push_str(&format!("  \"scenario_count\": {scenario_count},\n"));
    report.push_str("  \"scenarios\": [\n");

    let mut emitted = 0usize;
    for circumference_km in CIRCUMFERENCE_CASES_KM {
        for excavated_section_m2 in EXCAVATED_SECTION_CASES_M2 {
            for bore_count in BORE_COUNT_CASES {
                for overbreak_fraction in OVERBREAK_CASES {
                    let case = ExcavationCase {
                        circumference_km,
                        excavated_section_m2,
                        bore_count,
                        overbreak_fraction,
                        ancillary: AncillaryVolumes::default(),
                    };
                    let estimate = calculate_excavation(case)?;
                    if emitted > 0 {
                        report.push_str(",\n");
                    }
                    report.push_str(&format!(
                        concat!(
                            "    {{\"circumference_km\": {:.1}, ",
                            "\"excavated_cross_section_m2\": {:.1}, ",
                            "\"bore_count\": {}, ",
                            "\"overbreak_fraction\": {:.2}, ",
                            "\"design_bore_volume_m3\": {:.3}, ",
                            "\"overbreak_volume_m3\": {:.3}, ",
                            "\"shaft_volume_m3\": {:.3}, ",
                            "\"cavern_volume_m3\": {:.3}, ",
                            "\"cross_passage_volume_m3\": {:.3}, ",
                            "\"ancillary_volume_m3\": {:.3}, ",
                            "\"total_excavation_volume_m3\": {:.3}}}"
                        ),
                        case.circumference_km,
                        case.excavated_section_m2,
                        case.bore_count,
                        case.overbreak_fraction,
                        estimate.design_bore_volume_m3,
                        estimate.overbreak_volume_m3,
                        estimate.shaft_volume_m3,
                        estimate.cavern_volume_m3,
                        estimate.cross_passage_volume_m3,
                        estimate.ancillary_volume_m3,
                        estimate.total_excavation_volume_m3,
                    ));
                    emitted += 1;
                }
            }
        }
    }

    report.push_str("\n  ]\n}\n");
    if emitted != scenario_count {
        return Err("emitted scenario count does not match declared matrix size".to_owned());
    }
    Ok(report)
}

fn main() -> ExitCode {
    match render_report_json() {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_case(
        circumference_km: f64,
        excavated_section_m2: f64,
        bore_count: u32,
        overbreak_fraction: f64,
    ) -> ExcavationCase {
        ExcavationCase {
            circumference_km,
            excavated_section_m2,
            bore_count,
            overbreak_fraction,
            ancillary: AncillaryVolumes::default(),
        }
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-9,
            "actual {actual}, expected {expected}"
        );
    }

    #[test]
    fn one_hundred_km_by_thirty_square_metres_is_three_million_cubic_metres() {
        let estimate =
            calculate_excavation(base_case(100.0, 30.0, 1, 0.0)).expect("valid geometry");
        close(estimate.design_bore_volume_m3, 3_000_000.0);
        close(estimate.total_excavation_volume_m3, 3_000_000.0);
    }

    #[test]
    fn two_bores_double_the_design_volume() {
        let one = calculate_excavation(base_case(100.0, 30.0, 1, 0.0)).expect("one bore");
        let two = calculate_excavation(base_case(100.0, 30.0, 2, 0.0)).expect("two bores");
        close(two.design_bore_volume_m3, 2.0 * one.design_bore_volume_m3);
    }

    #[test]
    fn overbreak_is_separately_reported_and_additive() {
        let estimate =
            calculate_excavation(base_case(100.0, 30.0, 1, 0.10)).expect("valid geometry");
        close(estimate.design_bore_volume_m3, 3_000_000.0);
        close(estimate.overbreak_volume_m3, 300_000.0);
        close(estimate.total_excavation_volume_m3, 3_300_000.0);
    }

    #[test]
    fn ancillary_volumes_are_separately_accounted() {
        let mut case = base_case(100.0, 30.0, 1, 0.10);
        case.ancillary = AncillaryVolumes {
            shafts_m3: 100.0,
            caverns_m3: 200.0,
            cross_passages_m3: 50.0,
        };
        let estimate = calculate_excavation(case).expect("valid ancillary volumes");
        close(estimate.shaft_volume_m3, 100.0);
        close(estimate.cavern_volume_m3, 200.0);
        close(estimate.cross_passage_volume_m3, 50.0);
        close(estimate.ancillary_volume_m3, 350.0);
        close(estimate.total_excavation_volume_m3, 3_300_350.0);
    }

    #[test]
    fn rejects_invalid_geometry_and_ancillary_inputs() {
        assert!(calculate_excavation(base_case(0.0, 30.0, 1, 0.0)).is_err());
        assert!(calculate_excavation(base_case(f64::NAN, 30.0, 1, 0.0)).is_err());
        assert!(calculate_excavation(base_case(100.0, 0.0, 1, 0.0)).is_err());
        assert!(calculate_excavation(base_case(100.0, f64::INFINITY, 1, 0.0)).is_err());
        assert!(calculate_excavation(base_case(100.0, 30.0, 0, 0.0)).is_err());
        assert!(calculate_excavation(base_case(100.0, 30.0, 1, -0.01)).is_err());
        assert!(calculate_excavation(base_case(100.0, 30.0, 1, 1.01)).is_err());
        assert!(calculate_excavation(base_case(100.0, 30.0, 1, f64::NAN)).is_err());
        assert!(calculate_excavation(base_case(f64::MAX, 30.0, 1, 0.0)).is_err());

        let mut negative_ancillary = base_case(100.0, 30.0, 1, 0.0);
        negative_ancillary.ancillary.shafts_m3 = -1.0;
        assert!(calculate_excavation(negative_ancillary).is_err());
    }

    #[test]
    fn renders_seventy_two_scenarios_deterministically() {
        let first = render_report_json().expect("valid report");
        let second = render_report_json().expect("valid report");
        assert_eq!(first, second);
        assert!(first.contains("\"scenario_count\": 72"));
        assert!(first.contains("\"evidence_class\": \"geometry_only_excavation_volume_screen\""));
        assert!(first.contains("\"total_excavation_volume_m3\": 3000000.000"));
    }
}
