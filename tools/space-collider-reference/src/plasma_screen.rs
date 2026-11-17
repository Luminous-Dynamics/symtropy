//! Source-bound screening of the 2026 muon plasma-wakefield result.
//!
//! This is not a WarpX reproduction or a plasma solver. It exposes the
//! reported output and the unresolved comparison gaps as machine-readable
//! evidence so downstream experiment selection cannot mistake a PIC study
//! for a collider-ready, finite-source beam.

#[derive(Debug, Clone, Copy)]
struct PlasmaReference {
    driver_energy_gev: f64,
    muon_injection_energy_gev: f64,
    muon_final_energy_gev: f64,
    reported_energy_spread_percent: f64,
    muon_normalized_emittance_mm_mrad: f64,
    collider_energy_spread_reference_percent_rms: f64,
    collider_normalized_transverse_emittance_reference_um: f64,
}

fn published_reference() -> PlasmaReference {
    PlasmaReference {
        driver_energy_gev: 23.0,
        muon_injection_energy_gev: 0.300,
        muon_final_energy_gev: 40.0,
        reported_energy_spread_percent: 18.85,
        muon_normalized_emittance_mm_mrad: 8.86,
        collider_energy_spread_reference_percent_rms: 0.1,
        collider_normalized_transverse_emittance_reference_um: 25.0,
    }
}

fn evaluate(reference: PlasmaReference) -> Result<String, String> {
    let values = [
        reference.driver_energy_gev,
        reference.muon_injection_energy_gev,
        reference.muon_final_energy_gev,
        reference.reported_energy_spread_percent,
        reference.muon_normalized_emittance_mm_mrad,
        reference.collider_energy_spread_reference_percent_rms,
        reference.collider_normalized_transverse_emittance_reference_um,
    ];
    if values
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("plasma screen inputs must be finite and positive".to_owned());
    }
    if reference.muon_final_energy_gev <= reference.muon_injection_energy_gev {
        return Err("plasma output energy must exceed injection energy".to_owned());
    }

    // 1 mm mrad = 1000 um mrad. This numerical ratio is conditional on
    // the reported emittance and comparator using equivalent definitions.
    let emittance_um_mrad = reference.muon_normalized_emittance_mm_mrad * 1000.0;
    let conditional_emittance_gap_ratio =
        emittance_um_mrad / reference.collider_normalized_transverse_emittance_reference_um;
    let energy_spread_gap_ratio = reference.reported_energy_spread_percent
        / reference.collider_energy_spread_reference_percent_rms;
    let energy_gain_factor = reference.muon_final_energy_gev / reference.muon_injection_energy_gev;
    let energy_gain_gev = reference.muon_final_energy_gev - reference.muon_injection_energy_gev;

    let derived = [
        emittance_um_mrad,
        conditional_emittance_gap_ratio,
        energy_spread_gap_ratio,
        energy_gain_factor,
        energy_gain_gev,
    ];
    if derived.iter().any(|value| !value.is_finite()) {
        return Err("plasma screen produced a non-finite derived value".to_owned());
    }

    Ok(format!(
        concat!(
            "{{",
            "\"evidence_class\":\"source_bound_plasma_acceleration_screen\",",
            "\"source\":\"Wang et al., Phys. Rev. Accel. Beams 29, 021302 (2026), DOI 10.1103/wq5g-mtjq\",",
            "\"solver\":\"WarpX 3D PIC (paper-reported)\",",
            "\"driver_energy_gev\":{:.6},",
            "\"muon_injection_energy_gev\":{:.6},",
            "\"muon_final_energy_gev\":{:.6},",
            "\"muon_energy_gain_gev\":{:.6},",
            "\"muon_energy_gain_factor\":{:.6},",
            "\"reported_final_energy_spread_percent\":{:.6},",
            "\"collider_reference_energy_spread_percent_rms\":{:.6},",
            "\"energy_spread_gap_ratio_if_definitions_match\":{:.6},",
            "\"reported_normalized_emittance_mm_mrad\":{:.6},",
            "\"reported_normalized_emittance_um_mrad\":{:.6},",
            "\"collider_reference_normalized_transverse_emittance_um_mrad\":{:.6},",
            "\"conditional_emittance_gap_ratio_if_definitions_match\":{:.6},",
            "\"acceleration_result_initial_distribution\":\"zero initial energy spread and zero initial emittance\",",
            "\"separate_finite_emittance_injection_study\":{{",
            "\"initial_transverse_position_range_um\":[-500.0,500.0],",
            "\"initial_divergence_range_deg\":[-15.0,15.0],",
            "\"reported_injectable_divergence_range_deg\":[-10.0,10.0],",
            "\"large_divergence_particles\":\"some are lost when oscillation amplitude exceeds plasma bubble radius\"",
            "}},",
            "\"pic_setup\":{{",
            "\"driver_peak_density_per_m3\":7.0e25,",
            "\"driver_transverse_sigma_um\":8.0,",
            "\"driver_longitudinal_sigma_um\":15.0,",
            "\"initial_muon_density_per_m3\":1.0e21,",
            "\"plasma_density_initial_per_m3\":1.0e23,",
            "\"plasma_density_final_per_m3\":7.0e23,",
            "\"density_up_ramp_start_mm\":2.25,",
            "\"density_up_ramp_end_mm\":2.35,",
            "\"simulation_box_um\":[800.0,800.0,750.0],",
            "\"grid_cells\":[256,256,512],",
            "\"maxwell_solver\":\"FDTD with numerical-Cherenkov corrector\"",
            "}},",
            "\"data_availability\":\"simulation data not publicly available at publication; available from authors on reasonable request\",",
            "\"comparison_caveat\":\"Energy spread and emittance ratios are screening indicators only; definitions, planes, averaging and input-distribution differences must be harmonized before treating the ratios as formal performance comparisons.\",",
            "\"claim_ceiling\":\"paper-reported PIC output and conditional source-to-design gap screen only; no independent WarpX reproduction, realistic finite-source end-to-end output, beam transport handoff, collider readiness or experiment qualification\"",
            "}}"
        ),
        reference.driver_energy_gev,
        reference.muon_injection_energy_gev,
        reference.muon_final_energy_gev,
        energy_gain_gev,
        energy_gain_factor,
        reference.reported_energy_spread_percent,
        reference.collider_energy_spread_reference_percent_rms,
        energy_spread_gap_ratio,
        reference.muon_normalized_emittance_mm_mrad,
        emittance_um_mrad,
        reference.collider_normalized_transverse_emittance_reference_um,
        conditional_emittance_gap_ratio,
    ))
}

pub fn render_report_json() -> Result<String, String> {
    evaluate(published_reference())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plasma_to_collider_gap_ratios_are_unit_explicit() {
        let report = render_report_json().expect("valid plasma report");
        assert!(report.contains("\"energy_spread_gap_ratio_if_definitions_match\":188.500000"));
        assert!(report.contains("\"reported_normalized_emittance_um_mrad\":8860.000000"));
        assert!(
            report.contains("\"conditional_emittance_gap_ratio_if_definitions_match\":354.400000")
        );
        assert!(report.contains("\"muon_energy_gain_factor\":133.333333"));
    }

    #[test]
    fn zero_emittance_acceleration_run_is_not_conflated_with_finite_emittance_injection() {
        let report = render_report_json().expect("valid plasma report");
        assert!(report.contains("zero initial energy spread and zero initial emittance"));
        assert!(report.contains("\"separate_finite_emittance_injection_study\""));
        assert!(report.contains("\"initial_divergence_range_deg\":[-15.0,15.0]"));
        assert!(report.contains("\"reported_injectable_divergence_range_deg\":[-10.0,10.0]"));
    }

    #[test]
    fn non_public_source_data_is_a_machine_readable_reproduction_blocker() {
        let report = render_report_json().expect("valid plasma report");
        assert!(report.contains("not publicly available at publication"));
    }

    #[test]
    fn rejects_inconsistent_or_non_finite_source_inputs() {
        let mut invalid = published_reference();
        invalid.muon_final_energy_gev = invalid.muon_injection_energy_gev;
        assert!(evaluate(invalid).is_err());

        let mut invalid = published_reference();
        invalid.muon_normalized_emittance_mm_mrad = f64::NAN;
        assert!(evaluate(invalid).is_err());
    }
}
