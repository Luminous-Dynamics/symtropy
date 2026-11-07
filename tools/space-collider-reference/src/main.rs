use std::process::ExitCode;

const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;
const PROTON_REST_ENERGY_GEV: f64 = 0.938_272_088_16;
const GEV_PER_PEV: f64 = 1_000_000.0;
const GEV_PER_TEV: f64 = 1_000.0;
const GEV_PER_T_M: f64 = 0.299_792_458;

/// Centre-of-mass energy for a symmetric, head-on collider.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CenterOfMassEnergyGeV(f64);

/// Energy carried by one beam particle.
#[derive(Debug, Clone, Copy, PartialEq)]
struct BeamEnergyGeV(f64);

#[derive(Debug, Clone, Copy, PartialEq)]
struct LorentzFactor(f64);

fn beam_energy_from_center_of_mass(
    center_of_mass: CenterOfMassEnergyGeV,
) -> Result<BeamEnergyGeV, String> {
    if !center_of_mass.0.is_finite() || center_of_mass.0 <= 0.0 {
        return Err("centre-of-mass energy must be finite and positive".to_owned());
    }
    Ok(BeamEnergyGeV(center_of_mass.0 / 2.0))
}

fn lorentz_factor_from_beam(beam: BeamEnergyGeV) -> Result<LorentzFactor, String> {
    if !beam.0.is_finite() || beam.0 <= 0.0 {
        return Err("per-beam energy must be finite and positive".to_owned());
    }
    let gamma = beam.0 / PROTON_REST_ENERGY_GEV;
    if !gamma.is_finite() || gamma < 1.0 {
        return Err(
            "per-beam energy is below proton rest energy or numerically invalid".to_owned(),
        );
    }
    Ok(LorentzFactor(gamma))
}

fn gamma_ratio_from_cm(
    candidate: CenterOfMassEnergyGeV,
    reference: CenterOfMassEnergyGeV,
) -> Result<f64, String> {
    let candidate_beam = beam_energy_from_center_of_mass(candidate)?;
    let reference_beam = beam_energy_from_center_of_mass(reference)?;
    let candidate_gamma = lorentz_factor_from_beam(candidate_beam)?.0;
    let reference_gamma = lorentz_factor_from_beam(reference_beam)?.0;
    Ok(candidate_gamma / reference_gamma)
}

/// Magnetic-rigidity approximation used in the cited preprint:
/// E_beam [GeV] = (0.299792458 GeV/(T m)) * B[T] * f_dip * r[m].
fn required_ring_radius_m(
    center_of_mass: CenterOfMassEnergyGeV,
    field_t: f64,
    dipole_fill: f64,
) -> Result<f64, String> {
    if !field_t.is_finite() || field_t <= 0.0 {
        return Err("dipole field must be finite and positive".to_owned());
    }
    if !dipole_fill.is_finite() || dipole_fill <= 0.0 || dipole_fill > 1.0 {
        return Err("dipole filling factor must be in (0, 1]".to_owned());
    }
    let beam = beam_energy_from_center_of_mass(center_of_mass)?;
    Ok(beam.0 / (GEV_PER_T_M * field_t * dipole_fill))
}

fn max_center_of_mass_energy_gev(
    ring_radius_m: f64,
    field_t: f64,
    dipole_fill: f64,
) -> Result<CenterOfMassEnergyGeV, String> {
    if !ring_radius_m.is_finite() || ring_radius_m <= 0.0 {
        return Err("ring radius must be finite and positive".to_owned());
    }
    if !field_t.is_finite() || field_t <= 0.0 {
        return Err("dipole field must be finite and positive".to_owned());
    }
    if !dipole_fill.is_finite() || dipole_fill <= 0.0 || dipole_fill > 1.0 {
        return Err("dipole filling factor must be in (0, 1]".to_owned());
    }
    let beam_gev = GEV_PER_T_M * field_t * dipole_fill * ring_radius_m;
    let cm_gev = 2.0 * beam_gev;
    if !cm_gev.is_finite() {
        return Err("computed centre-of-mass energy is not finite".to_owned());
    }
    Ok(CenterOfMassEnergyGeV(cm_gev))
}

fn orbital_revolution_frequency_hz(circumference_m: f64) -> Result<f64, String> {
    if !circumference_m.is_finite() || circumference_m <= 0.0 {
        return Err("circumference must be finite and positive".to_owned());
    }
    Ok(SPEED_OF_LIGHT_M_S / circumference_m)
}

fn render_reference_report() -> Result<String, String> {
    // Reproduce the paper's stated convention: candidate E_cm = 1 PeV;
    // LHC design reference E_cm = 14 TeV.
    let candidate_cm = CenterOfMassEnergyGeV(GEV_PER_PEV);
    let lhc_cm = CenterOfMassEnergyGeV(14.0 * GEV_PER_TEV);

    let candidate_beam = beam_energy_from_center_of_mass(candidate_cm)?;
    let lhc_beam = beam_energy_from_center_of_mass(lhc_cm)?;
    let candidate_gamma = lorentz_factor_from_beam(candidate_beam)?;
    let lhc_gamma = lorentz_factor_from_beam(lhc_beam)?;
    let gamma_ratio = gamma_ratio_from_cm(candidate_cm, lhc_cm)?;

    // A 47,000 km circumference is the worked orbital-scale example.
    let circumference_m = 47_000_000.0;
    let revolution_hz = orbital_revolution_frequency_hz(circumference_m)?;
    let radius_m = circumference_m / (2.0 * std::f64::consts::PI);

    // The preprint's Table 1 gives ~1,190 km radius for 1 PeV E_cm,
    // 2 T dipoles and a 0.7 dipole filling factor.
    let table_radius_m = required_ring_radius_m(candidate_cm, 2.0, 0.7)?;
    let ring_max_cm = max_center_of_mass_energy_gev(radius_m, 2.0, 0.7)?;

    // Show the alternative interpretation which would produce the paper's
    // approximately 140x gamma ratio: treating 1 PeV as per-beam energy.
    let alternative_beam = BeamEnergyGeV(GEV_PER_PEV);
    let alternative_gamma = lorentz_factor_from_beam(alternative_beam)?;
    let alternative_ratio = alternative_gamma.0 / lhc_gamma.0;

    Ok(format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"evidence_class\": \"analytical_reference_check\",\n",
            "  \"candidate_energy_convention\": \"1 PeV centre-of-mass; symmetric head-on beams\",\n",
            "  \"candidate_center_of_mass_energy_gev\": {:.6},\n",
            "  \"candidate_per_beam_energy_gev\": {:.6},\n",
            "  \"lhc_design_center_of_mass_energy_gev\": {:.6},\n",
            "  \"lhc_per_beam_energy_gev\": {:.6},\n",
            "  \"candidate_gamma\": {:.6},\n",
            "  \"lhc_gamma\": {:.6},\n",
            "  \"gamma_ratio_under_center_of_mass_convention\": {:.6},\n",
            "  \"gamma_ratio_if_1_pev_means_per_beam\": {:.6},\n",
            "  \"orbital_circumference_km\": {:.6},\n",
            "  \"orbital_radius_km\": {:.6},\n",
            "  \"revolution_frequency_hz\": {:.6},\n",
            "  \"required_radius_for_1_pev_cm_at_2_t_fill_0p7_km\": {:.6},\n",
            "  \"maximum_cm_energy_for_47000_km_circumference_2_t_fill_0p7_pev\": {:.6},\n",
            "  \"paper_claim_review\": \"A 140x Lorentz-factor ratio is inconsistent with 1 PeV as centre-of-mass energy and 14 TeV LHC centre-of-mass energy; it corresponds to 1 PeV per beam. Clarify convention before using the compensation factor.\",\n",
            "  \"claim_ceiling\": \"analytic energy-convention, rigidity and revolution-frequency checks only; no beam-optics, luminosity, magnet, orbit-control, thermal or flight qualification\"\n",
            "}}"
        ),
        candidate_cm.0,
        candidate_beam.0,
        lhc_cm.0,
        lhc_beam.0,
        candidate_gamma.0,
        lhc_gamma.0,
        gamma_ratio,
        alternative_ratio,
        circumference_m / 1000.0,
        radius_m / 1000.0,
        revolution_hz,
        table_radius_m / 1000.0,
        ring_max_cm.0 / GEV_PER_PEV,
    ))
}

fn main() -> ExitCode {
    match render_reference_report() {
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

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual}, expected {expected}, tolerance {tolerance}"
        );
    }

    #[test]
    fn one_pev_center_of_mass_means_half_pev_per_beam() {
        let beam = beam_energy_from_center_of_mass(CenterOfMassEnergyGeV(GEV_PER_PEV))
            .expect("valid CM energy");
        close(beam.0, 500_000.0, 0.0);
    }

    #[test]
    fn lorentz_factor_ratio_is_71p428_when_both_energies_are_cm() {
        let ratio = gamma_ratio_from_cm(
            CenterOfMassEnergyGeV(GEV_PER_PEV),
            CenterOfMassEnergyGeV(14.0 * GEV_PER_TEV),
        )
        .expect("valid energies");
        close(ratio, 71.428_571_428_571_43, 1e-12);
    }

    #[test]
    fn one_pev_per_beam_produces_the_approximately_140x_ratio() {
        let lhc_beam = BeamEnergyGeV(7.0 * GEV_PER_TEV);
        let lhc_gamma = lorentz_factor_from_beam(lhc_beam).expect("valid LHC beam");
        let candidate_gamma =
            lorentz_factor_from_beam(BeamEnergyGeV(GEV_PER_PEV)).expect("valid beam");
        close(
            candidate_gamma.0 / lhc_gamma.0,
            142.857_142_857_142_86,
            1e-12,
        );
    }

    #[test]
    fn forty_seven_thousand_km_orbit_has_about_six_point_four_hz_revolution() {
        let frequency = orbital_revolution_frequency_hz(47_000_000.0).expect("valid circumference");
        close(frequency, 6.378_562_936_170_213, 1e-12);
    }

    #[test]
    fn rigidity_reference_reproduces_the_preprint_table_radius() {
        let radius = required_ring_radius_m(CenterOfMassEnergyGeV(GEV_PER_PEV), 2.0, 0.7)
            .expect("valid magnet parameters");
        close(radius / 1000.0, 1_191.300_339_993_400_3, 1e-9);
    }

    #[test]
    fn forty_seven_thousand_km_at_two_tesla_reaches_about_six_point_three_pev_cm() {
        let radius_m = 47_000_000.0 / (2.0 * std::f64::consts::PI);
        let cm = max_center_of_mass_energy_gev(radius_m, 2.0, 0.7).expect("valid ring and field");
        close(cm.0 / GEV_PER_PEV, 6.279_090_229_555_815, 1e-12);
    }

    #[test]
    fn rejects_invalid_rigidity_and_orbit_inputs() {
        assert!(required_ring_radius_m(CenterOfMassEnergyGeV(0.0), 2.0, 0.7).is_err());
        assert!(required_ring_radius_m(CenterOfMassEnergyGeV(1e6), 0.0, 0.7).is_err());
        assert!(required_ring_radius_m(CenterOfMassEnergyGeV(1e6), 2.0, 1.1).is_err());
        assert!(orbital_revolution_frequency_hz(0.0).is_err());
    }

    #[test]
    fn reference_output_discloses_the_energy_convention_issue() {
        let report = render_reference_report().expect("valid reference report");
        assert!(report.contains("\"gamma_ratio_under_center_of_mass_convention\": 71.428571"));
        assert!(report.contains("\"gamma_ratio_if_1_pev_means_per_beam\": 142.857143"));
        assert!(report.contains("Clarify convention"));
    }
}
