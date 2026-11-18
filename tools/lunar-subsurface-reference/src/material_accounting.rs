#[derive(Debug, Clone, Copy, PartialEq)]
struct MassRange {
    minimum_kg: f64,
    maximum_kg: f64,
}

#[derive(Debug, Clone, Copy)]
struct DensityRange {
    minimum_kg_m3: f64,
    maximum_kg_m3: f64,
}

fn calculate_mass_range(volume_m3: f64, density: DensityRange) -> Result<MassRange, String> {
    if !volume_m3.is_finite() || volume_m3 <= 0.0 {
        return Err("volume must be finite and positive".to_owned());
    }
    if !density.minimum_kg_m3.is_finite()
        || !density.maximum_kg_m3.is_finite()
        || density.minimum_kg_m3 <= 0.0
        || density.maximum_kg_m3 < density.minimum_kg_m3
    {
        return Err("density bounds must be finite, positive, and ordered".to_owned());
    }

    let minimum_kg = volume_m3 * density.minimum_kg_m3;
    let maximum_kg = volume_m3 * density.maximum_kg_m3;
    if !minimum_kg.is_finite() || !maximum_kg.is_finite() {
        return Err("derived mass is non-finite".to_owned());
    }
    Ok(MassRange {
        minimum_kg,
        maximum_kg,
    })
}

pub(super) fn render_report_json() -> Result<String, String> {
    // These bulk-density ranges apply only to their declared geological classes.
    // They are not site-specific measurements and are not interchangeable.
    let highland_crust = DensityRange {
        minimum_kg_m3: 2_200.0,
        maximum_kg_m3: 2_600.0,
    };
    let basin_ejecta = DensityRange {
        minimum_kg_m3: 2_350.0,
        maximum_kg_m3: 2_600.0,
    };

    let single_bore_volume_m3 = 3_000_000.0;
    let twin_bore_overbreak_volume_m3 = 7_200_000.0;

    let highland_single = calculate_mass_range(single_bore_volume_m3, highland_crust)?;
    let highland_twin = calculate_mass_range(twin_bore_overbreak_volume_m3, highland_crust)?;
    let basin_single = calculate_mass_range(single_bore_volume_m3, basin_ejecta)?;
    let basin_twin = calculate_mass_range(twin_bore_overbreak_volume_m3, basin_ejecta)?;

    let mut report = String::new();
    report.push_str(
        concat!(
            "{\n",
            "  \"evidence_class\": \"literature_bounded_in_situ_bulk_mass_scenarios\",\n",
            "  \"units\": {\n",
            "    \"density\": \"kg/m^3\",\n",
            "    \"volume\": \"m^3\",\n",
            "    \"mass\": \"kg\"\n",
            "  },\n",
            "  \"assumptions\": [\n",
            "    \"Mass interval = declared in-situ/bank volume multiplied by a source-bounded bulk-density interval.\",\n",
            "    \"Rock mass examples apply only if the excavated corridor has the stated geological material class.\",\n",
            "    \"The excavation geometry inputs are illustrative reference cases, not a selected tunnel design.\",\n",
            "    \"Regolith and simulant density catalogues are separate and are not applied to the bedrock tunnel mass examples.\",\n",
            "    \"Grain density, bulk density, and loose-spoil density are different quantities.\",\n",
            "    \"No swell, loose-spoil volume, excavation productivity, or site-specific material property is inferred.\"\n",
            "  ],\n",
            "  \"rock_mass_examples\": [\n"
        ),
    );

    report.push_str(&format!(
        concat!(
            "    {{\"material_record_id\":\"LUNAR-HIGHLAND-CRUST-BULK-2012\",",
            "\"material_class\":\"feldspathic_highland_crust_bulk_density\",",
            "\"density_min_kg_m3\":{:.1},\"density_max_kg_m3\":{:.1},",
            "\"source\":\"Kiefer et al. 2012, DOI 10.1029/2012GL051319\",",
            "\"source_scope\":\"reported bulk density for feldspathic highland crust; not site-specific\",",
            "\"single_bore_reference_volume_m3\":{:.1},",
            "\"single_bore_mass_min_kg\":{:.1},\"single_bore_mass_max_kg\":{:.1},",
            "\"two_bore_20pct_overbreak_reference_volume_m3\":{:.1},",
            "\"two_bore_mass_min_kg\":{:.1},\"two_bore_mass_max_kg\":{:.1},",
            "\"applicability\":\"conditional scenario only if the corridor material is consistent with this class\"}},\n"
        ),
        highland_crust.minimum_kg_m3,
        highland_crust.maximum_kg_m3,
        single_bore_volume_m3,
        highland_single.minimum_kg,
        highland_single.maximum_kg,
        twin_bore_overbreak_volume_m3,
        highland_twin.minimum_kg,
        highland_twin.maximum_kg,
    ));
    report.push_str(&format!(
        concat!(
            "    {{\"material_record_id\":\"LUNAR-IMPACT-BASIN-EJECTA-BULK-2012\",",
            "\"material_class\":\"impact_basin_ejecta_bulk_density\",",
            "\"density_min_kg_m3\":{:.1},\"density_max_kg_m3\":{:.1},",
            "\"source\":\"Kiefer et al. 2012, DOI 10.1029/2012GL051319\",",
            "\"source_scope\":\"reported bulk density for impact basin ejecta; not site-specific\",",
            "\"single_bore_reference_volume_m3\":{:.1},",
            "\"single_bore_mass_min_kg\":{:.1},\"single_bore_mass_max_kg\":{:.1},",
            "\"two_bore_20pct_overbreak_reference_volume_m3\":{:.1},",
            "\"two_bore_mass_min_kg\":{:.1},\"two_bore_mass_max_kg\":{:.1},",
            "\"applicability\":\"conditional scenario only if the corridor material is consistent with this class\"}}\n"
        ),
        basin_ejecta.minimum_kg_m3,
        basin_ejecta.maximum_kg_m3,
        single_bore_volume_m3,
        basin_single.minimum_kg,
        basin_single.maximum_kg,
        twin_bore_overbreak_volume_m3,
        basin_twin.minimum_kg,
        basin_twin.maximum_kg,
    ));

    report.push_str(
        concat!(
            "  ],\n",
            "  \"regolith_density_catalog\": [\n",
            "    {\"record_id\":\"APOLLO-11-COSTES-MITCHELL-1970\",\"material_class\":\"unconsolidated_regolith\",\"density_min_kg_m3\":1540.0,\"density_max_kg_m3\":1750.0,\"source\":\"NASA/TM-20240011783 Table 15; Costes and Mitchell 1970\",\"use\":\"regolith comparator only; not tunnel-bedrock density\"},\n",
            "    {\"record_id\":\"APOLLO-12-SCOTT-1971\",\"material_class\":\"unconsolidated_regolith\",\"density_min_kg_m3\":1600.0,\"density_max_kg_m3\":2000.0,\"source\":\"NASA/TM-20240011783 Table 15; Scott et al. 1971\",\"use\":\"regolith comparator only; not tunnel-bedrock density\"},\n",
            "    {\"record_id\":\"APOLLO-17-MITCHELL-1973A\",\"material_class\":\"unconsolidated_regolith\",\"density_min_kg_m3\":1570.0,\"density_max_kg_m3\":2290.0,\"source\":\"NASA/TM-20240011783 Table 15; Mitchell et al. 1973a\",\"use\":\"regolith comparator only; preserve wide source range\"},\n",
            "    {\"record_id\":\"APOLLO-17-CARRIER-1974\",\"material_class\":\"unconsolidated_regolith\",\"density_min_kg_m3\":1740.0,\"density_max_kg_m3\":1990.0,\"source\":\"NASA/TM-20240011783 Table 15; Carrier 1974\",\"use\":\"separate source-specific regolith comparator\"}\n",
            "  ],\n",
            "  \"simulant_density_catalog\": [\n",
            "    {\"record_id\":\"CSM-LHT-1\",\"material_class\":\"terrestrial_lunar_highlands_simulant\",\"region\":\"Highlands\",\"density_min_kg_m3\":1500.0,\"density_max_kg_m3\":1900.0,\"source\":\"NASA/TM-20240011783 Table 16; SDL 2023\",\"use\":\"laboratory simulant only; not lunar in-situ material\"},\n",
            "    {\"record_id\":\"LHS-1\",\"material_class\":\"terrestrial_lunar_highlands_simulant\",\"region\":\"Highlands\",\"density_min_kg_m3\":1380.0,\"density_max_kg_m3\":1560.0,\"source\":\"NASA/TM-20240011783 Table 16; APL SAR 2022\",\"use\":\"laboratory simulant only; not lunar in-situ material\"},\n",
            "    {\"record_id\":\"JSC-1A\",\"material_class\":\"terrestrial_lunar_mare_simulant\",\"region\":\"Low-Ti Mare\",\"density_min_kg_m3\":1570.0,\"density_max_kg_m3\":2030.0,\"source\":\"NASA/TM-20240011783 Table 16; Zeng et al. 2010\",\"use\":\"laboratory simulant only; not lunar in-situ material\"}\n",
            "  ],\n",
            "  \"loose_spoil_and_swell\": {\n",
            "    \"status\": \"not_calculated\",\n",
            "    \"reason\": \"No source-bounded loose-spoil density or measured bank-to-loose volume ratio is supplied.\",\n",
            "    \"required_inputs\": [\"material-specific bank bulk density\", \"material-specific loose bulk density after a declared handling process\", \"sampled or measured bank volume\", \"loose volume or mass balance\"]\n",
            "  }\n",
            "}\n"
        ),
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-6,
            "actual {actual}, expected {expected}"
        );
    }

    #[test]
    fn feldspathic_highland_crust_mass_bounds_are_reproduced() {
        let mass = calculate_mass_range(
            3_000_000.0,
            DensityRange {
                minimum_kg_m3: 2_200.0,
                maximum_kg_m3: 2_600.0,
            },
        )
        .expect("valid bulk-density interval");
        close(mass.minimum_kg, 6_600_000_000.0);
        close(mass.maximum_kg, 7_800_000_000.0);
    }

    #[test]
    fn rejects_invalid_volume_and_density_bounds() {
        let valid = DensityRange {
            minimum_kg_m3: 2_200.0,
            maximum_kg_m3: 2_600.0,
        };
        assert!(calculate_mass_range(0.0, valid).is_err());
        assert!(calculate_mass_range(f64::NAN, valid).is_err());
        assert!(
            calculate_mass_range(
                1.0,
                DensityRange {
                    minimum_kg_m3: 0.0,
                    maximum_kg_m3: 2_600.0,
                }
            )
            .is_err()
        );
        assert!(
            calculate_mass_range(
                1.0,
                DensityRange {
                    minimum_kg_m3: 2_600.0,
                    maximum_kg_m3: 2_200.0,
                }
            )
            .is_err()
        );
    }

    #[test]
    fn density_catalogues_preserve_distinct_material_domains() {
        let report = render_report_json().expect("valid material accounting report");
        assert!(report.contains("\"feldspathic_highland_crust_bulk_density\""));
        assert!(report.contains("\"unconsolidated_regolith\""));
        assert!(report.contains("\"terrestrial_lunar_highlands_simulant\""));
        assert!(report.contains("\"not_calculated\""));
        assert!(report.contains("not tunnel-bedrock density"));
        assert!(report.contains("not lunar in-situ material"));
    }

    #[test]
    fn material_report_is_deterministic() {
        let first = render_report_json().expect("valid report");
        let second = render_report_json().expect("valid report");
        assert_eq!(first, second);
    }
}
