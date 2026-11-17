//! Local engineered-ring geometry and species scaling screen.
//!
//! This extends the existing space-collider reference. It is not a lattice,
//! beam-dynamics, luminosity, RF, radiation-transport, civil/site or thermal
//! model. The host body's mean radius is deliberately not used: circumference
//! is an independent machine-design input.

const C_M_S: f64 = 299_792_458.0;
const GEV_PER_T_M: f64 = 0.299_792_458;
const ELEMENTARY_CHARGE_C: f64 = 1.602_176_634e-19;
const ELECTRON_MASS_GEV: f64 = 0.000_510_998_95;
const PROTON_MASS_GEV: f64 = 0.938_272_088_16;
const MUON_MASS_GEV: f64 = 0.105_658_375_5;
const MUON_MEAN_LIFETIME_S: f64 = 2.196_981_1e-6;
const ELECTRON_CRITICAL_FIELD_T: f64 = 4.414_005_218e9;
const ELECTRON_U0_COEFFICIENT_MEV: f64 = 0.088_5;
const JOULES_PER_GEV: f64 = 1.0e9 * ELEMENTARY_CHARGE_C;
const MUONS_PER_BUNCH: f64 = 1.8e12;
const MUON_BUNCH_REPETITION_HZ: f64 = 5.0;
const PROTON_REFERENCE_CURRENT_A: f64 = 0.5;

#[derive(Debug, Clone, Copy)]
struct LocalRingCase {
    circumference_km: f64,
    field_t: f64,
    fill_fraction: f64,
    bending_radius_km: f64,
    proton_beam_energy_tev: f64,
    proton_cm_energy_tev: f64,
    revolution_frequency_hz: f64,
    proton_sr_loss_mev_per_turn: f64,
    proton_sr_loss_fraction_per_turn: f64,
    proton_sr_power_mw_at_reference_current: f64,
    proton_quantum_chi: f64,
    muon_gamma: f64,
    muon_lab_lifetime_s: f64,
    muon_decay_turns: f64,
    two_beam_muon_luminosity_decay_turns: f64,
    muon_sr_loss_mev_per_turn: f64,
    muon_sr_loss_fraction_per_turn: f64,
    muon_injection_power_mw_per_beam: f64,
    muon_sr_power_mw_per_beam_formal: f64,
    muon_sr_to_injection_power_ratio: f64,
    muon_quantum_chi: f64,
}

fn synchrotron_loss_mev_per_turn(
    beam_energy_gev: f64,
    bending_radius_m: f64,
    particle_mass_gev: f64,
) -> Result<f64, String> {
    if !beam_energy_gev.is_finite()
        || beam_energy_gev <= 0.0
        || !bending_radius_m.is_finite()
        || bending_radius_m <= 0.0
        || !particle_mass_gev.is_finite()
        || particle_mass_gev <= 0.0
    {
        return Err("synchrotron inputs must be finite and positive".to_owned());
    }
    let mass_scale = (ELECTRON_MASS_GEV / particle_mass_gev).powi(4);
    let loss = ELECTRON_U0_COEFFICIENT_MEV
        * beam_energy_gev.powi(4)
        / bending_radius_m
        * mass_scale;
    if !loss.is_finite() || loss < 0.0 {
        return Err("synchrotron loss is non-finite or negative".to_owned());
    }
    Ok(loss)
}

fn quantum_parameter(
    beam_energy_gev: f64,
    field_t: f64,
    particle_mass_gev: f64,
) -> Result<f64, String> {
    if !beam_energy_gev.is_finite()
        || beam_energy_gev <= 0.0
        || !field_t.is_finite()
        || field_t <= 0.0
        || !particle_mass_gev.is_finite()
        || particle_mass_gev <= 0.0
    {
        return Err("quantum-parameter inputs must be finite and positive".to_owned());
    }
    let gamma = beam_energy_gev / particle_mass_gev;
    let critical_field_t =
        ELECTRON_CRITICAL_FIELD_T * (particle_mass_gev / ELECTRON_MASS_GEV).powi(2);
    let chi = gamma * field_t / critical_field_t;
    if !chi.is_finite() || chi < 0.0 {
        return Err("quantum parameter is invalid".to_owned());
    }
    Ok(chi)
}

fn calculate_case(
    circumference_km: f64,
    field_t: f64,
    fill_fraction: f64,
) -> Result<LocalRingCase, String> {
    let inputs = [circumference_km, field_t, fill_fraction];
    if inputs.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return Err("local-ring inputs must be finite and positive".to_owned());
    }
    if fill_fraction > 1.0 {
        return Err("dipole fill fraction must be in (0, 1]".to_owned());
    }

    let circumference_m = circumference_km * 1_000.0;
    let bending_radius_m = fill_fraction * circumference_m / (2.0 * std::f64::consts::PI);
    let beam_energy_gev = GEV_PER_T_M * field_t * bending_radius_m;
    if !beam_energy_gev.is_finite() || beam_energy_gev <= PROTON_MASS_GEV {
        return Err("rigidity-derived beam energy is invalid".to_owned());
    }

    let revolution_frequency_hz = C_M_S / circumference_m;
    let revolution_period_s = 1.0 / revolution_frequency_hz;

    let proton_sr_loss_mev_per_turn =
        synchrotron_loss_mev_per_turn(beam_energy_gev, bending_radius_m, PROTON_MASS_GEV)?;
    let proton_beam_energy_mev = beam_energy_gev * 1.0e3;
    let proton_sr_loss_fraction_per_turn =
        proton_sr_loss_mev_per_turn / proton_beam_energy_mev;
    let proton_sr_power_mw_at_reference_current =
        PROTON_REFERENCE_CURRENT_A * proton_sr_loss_mev_per_turn;

    let proton_gamma = beam_energy_gev / PROTON_MASS_GEV;
    let proton_quantum_chi = quantum_parameter(beam_energy_gev, field_t, PROTON_MASS_GEV)?;

    let muon_gamma = beam_energy_gev / MUON_MASS_GEV;
    let muon_lab_lifetime_s = muon_gamma * MUON_MEAN_LIFETIME_S;
    let muon_decay_turns = muon_lab_lifetime_s / revolution_period_s;
    let two_beam_muon_luminosity_decay_turns = muon_decay_turns / 2.0;

    // Retain the NLASG reference bunch/repetition inputs solely as a common
    // scaling profile. This is not a bunch-pattern or luminosity prediction.
    let average_circulating_bunch_equivalents =
        MUON_BUNCH_REPETITION_HZ * muon_lab_lifetime_s;
    let average_muon_beam_current_a = ELEMENTARY_CHARGE_C
        * MUONS_PER_BUNCH
        * average_circulating_bunch_equivalents
        * revolution_frequency_hz;
    let muon_bunch_energy_j = MUONS_PER_BUNCH * beam_energy_gev * JOULES_PER_GEV;
    let muon_injection_power_mw_per_beam =
        muon_bunch_energy_j * MUON_BUNCH_REPETITION_HZ / 1.0e6;

    let muon_sr_loss_mev_per_turn =
        synchrotron_loss_mev_per_turn(beam_energy_gev, bending_radius_m, MUON_MASS_GEV)?;
    let muon_beam_energy_mev = beam_energy_gev * 1.0e3;
    let muon_sr_loss_fraction_per_turn = muon_sr_loss_mev_per_turn / muon_beam_energy_mev;
    let muon_sr_power_mw_per_beam_formal =
        average_muon_beam_current_a * muon_sr_loss_mev_per_turn;
    let muon_sr_to_injection_power_ratio =
        muon_sr_power_mw_per_beam_formal / muon_injection_power_mw_per_beam;
    let muon_quantum_chi = quantum_parameter(beam_energy_gev, field_t, MUON_MASS_GEV)?;

    let derived = [
        bending_radius_m,
        beam_energy_gev,
        revolution_frequency_hz,
        proton_sr_loss_mev_per_turn,
        proton_sr_loss_fraction_per_turn,
        proton_sr_power_mw_at_reference_current,
        proton_gamma,
        proton_quantum_chi,
        muon_gamma,
        muon_lab_lifetime_s,
        muon_decay_turns,
        two_beam_muon_luminosity_decay_turns,
        average_circulating_bunch_equivalents,
        average_muon_beam_current_a,
        muon_bunch_energy_j,
        muon_injection_power_mw_per_beam,
        muon_sr_loss_per_turn,
        muon_sr_loss_fraction_per_turn,
        muon_sr_power_mw_per_beam_formal,
        muon_sr_to_injection_power_ratio,
        muon_quantum_chi,
    ];
    if derived.iter().any(|x| !x.is_finite() || *x < 0.0) {
        return Err("local-ring derived quantity is non-finite or negative".to_owned());
    }

    Ok(LocalRingCase {
        circumference_km,
        field_t,
        fill_fraction,
        bending_radius_km: bending_radius_m / 1_000.0,
        proton_beam_energy_tev: beam_energy_gev / 1_000.0,
        proton_cm_energy_tev: 2.0 * beam_energy_gev / 1_000.0,
        revolution_frequency_hz,
        proton_sr_loss_mev_per_turn,
        proton_sr_loss_fraction_per_turn,
        proton_sr_power_mw_at_reference_current,
        proton_quantum_chi,
        muon_gamma,
        muon_lab_lifetime_s,
        muon_decay_turns,
        two_beam_muon_luminosity_decay_turns,
        muon_sr_loss_mev_per_turn,
        muon_sr_loss_fraction_per_turn,
        muon_injection_power_mw_per_beam,
        muon_sr_power_mw_per_beam_formal,
        muon_sr_to_injection_power_ratio,
        muon_quantum_chi,
    })
}

fn field_evidence_label(field_t: f64) -> &'static str {
    if field_t >= 20.0 {
        "hts_stretch_case_not_production_ready"
    } else if field_t >= 16.0 {
        "fcc_hh_conceptual_target_not_operational"
    } else if field_t >= 14.0 {
        "high_field_development_reference_not_production_qualified"
    } else {
        "operating_lhc_dipole_reference_scale"
    }
}

fn render_case_json(case: LocalRingCase) -> String {
    format!(
        concat!(
            "{{",
            "\"circumference_km\":{:.6},",
            "\"dipole_field_t\":{:.6},",
            "\"field_evidence_label\":\"{}\",",
            "\"dipole_fill_fraction\":{:.6},",
            "\"effective_bending_radius_km\":{:.6},",
            "\"proton_per_beam_energy_tev_rigidity_only\":{:.9},",
            "\"proton_centre_of_mass_energy_tev_rigidity_only\":{:.9},",
            "\"revolution_frequency_hz\":{:.9},",
            "\"proton_synchrotron_loss_mev_per_turn\":{:.9e},",
            "\"proton_synchrotron_loss_fraction_per_turn\":{:.9e},",
            "\"proton_synchrotron_power_mw_per_beam_at_0p5_a_reference_current\":{:.9},",
            "\"proton_classical_quantum_parameter_chi\":{:.9e},",
            "\"muon_gamma\":{:.9e},",
            "\"muon_lab_mean_lifetime_s\":{:.9e},",
            "\"muon_decay_efold_turns\":{:.6},",
            "\"two_beam_muon_luminosity_decay_efold_turns\":{:.6},",
            "\"muon_synchrotron_loss_mev_per_turn\":{:.9e},",
            "\"muon_synchrotron_loss_fraction_per_turn\":{:.9e},",
            "\"muon_injection_power_mw_per_beam_nlasg_bunch_profile\":{:.9},",
            "\"muon_synchrotron_power_mw_per_beam_formal\":{:.9},",
            "\"muon_synchrotron_to_injection_power_ratio_formal\":{:.9},",
            "\"muon_classical_quantum_parameter_chi\":{:.9e},",
            "\"muon_radiation_status\":\"{}\"",
            "}}"
        ),
        case.circumference_km,
        case.field_t,
        field_evidence_label(case.field_t),
        case.fill_fraction,
        case.bending_radius_km,
        case.proton_beam_energy_tev,
        case.proton_cm_energy_tev,
        case.revolution_frequency_hz,
        case.proton_sr_loss_mev_per_turn,
        case.proton_sr_loss_fraction_per_turn,
        case.proton_sr_power_mw_at_reference_current,
        case.proton_quantum_chi,
        case.muon_gamma,
        case.muon_lab_lifetime_s,
        case.muon_decay_turns,
        case.two_beam_muon_luminosity_decay_turns,
        case.muon_sr_loss_mev_per_turn,
        case.muon_sr_loss_fraction_per_turn,
        case.muon_injection_power_mw_per_beam,
        case.muon_sr_power_mw_per_beam_formal,
        case.muon_sr_to_injection_power_ratio,
        case.muon_quantum_chi,
        if case.muon_sr_loss_fraction_per_turn >= 1.0 {
            "no_fixed_energy_circular_equilibrium_under_declared_classical_screen"
        } else if case.muon_quantum_chi >= 1.0e-3 {
            "quantum_synchrotron_review_required"
        } else {
            "analytical_screen_only"
        },
    )
}

fn render_report_json() -> Result<String, String> {
    let mut rows = Vec::new();
    for circumference_km in [50.0, 100.0, 150.0] {
        for field_t in [8.33, 14.0, 16.0, 20.0] {
            rows.push(render_case_json(calculate_case(circumference_km, field_t, 0.67)?));
        }
    }
    let fcc_hh_comparator = calculate_case(97.75, 16.0, 0.67)?;
    let fcc_energy_relative_difference =
        (fcc_hh_comparator.proton_cm_energy_tev - 100.0) / 100.0;
    let fcc_power_relative_difference =
        (fcc_hh_comparator.proton_sr_power_mw_at_reference_current - 2.4) / 2.4;

    Ok(format!(
        concat!(
            "{{",
            "\"evidence_class\":\"local_ring_rigidity_and_species_scaling_screen\",",
            "\"comparison_matrix_count\":{},",
            "\"dipole_fill_fraction\":0.67,",
            "\"circumference_cases_km\":[50.0,100.0,150.0],",
            "\"field_cases_t\":[8.33,14.0,16.0,20.0],",
            "\"fcc_hh_geometry_comparator\":{{",
            "\"source_circumference_km\":97.75,",
            "\"source_centre_of_mass_energy_tev\":100.0,",
            "\"source_dipole_field_t\":16.0,",
            "\"source_synchrotron_power_mw_per_beam\":2.4,",
            "\"screened_rigidity_only_centre_of_mass_energy_tev\":{:.9},",
            "\"screened_proton_synchrotron_power_mw_per_beam_at_0p5_a\":{:.9},",
            "\"energy_relative_difference_vs_source\":{:.9e},",
            "\"synchrotron_power_relative_difference_vs_source\":{:.9e},",
            "\"comparison_status\":\"unadjusted_screening_residual_retained_not_calibrated_away\"",
            "}},",
            "\"nlasg_muon_profile\":{{",
            "\"source_reference\":\"FERMILAB-PUB-26-0741-TD / arXiv:2611.06680, 5 November 2026\",",
            "\"source_centre_of_mass_energy_tev\":10.0,",
            "\"source_circumference_km\":11.4,",
            "\"source_arc_peak_field_t\":14.0,",
            "\"source_two_beam_injection_power_mw\":14.4,",
            "\"source_luminosity_decay_efold_turns\":1363,",
            "\"reference_profile_bunches_per_beam\":1.8e12,",
            "\"reference_profile_repetition_hz\":5.0,",
            "\"claim\":\"Existing muon scaling source profile reused for comparative injection and decay arithmetic; this is not a matched lattice or luminosity design.\"",
            "}},",
            "\"cases\":[{}],",
            "\"assumptions\":[",
            "\"Effective bending radius rho = dipole_fill_fraction * circumference / (2 pi); host-body mean radius is not used.\",",
            "\"Symmetric head-on beams; proton and muon rigidity are compared at equal charge magnitude; energies are ultra-relativistic approximations.\",",
            "\"Proton synchrotron power uses a declared 0.5 A reference current and the classical isomagnetic U0 relation; it is not a lattice radiation-integral result.\",",
            "\"Muon injection and synchrotron power use 1.8e12 muons per bunch and 5 Hz as a common NLASG source profile; continuous exponential-decay average only.\",",
            "\"Muon decay-product energy flow is not converted to deposited heat or neutrino dose.\",",
            "\"14 T / 16 T / 20 T rows are development or stretch assumptions; field_evidence_label does not imply production availability.\",",
            "\"No beam optics, nonlinear beam-beam dynamics, luminosity prediction, emittance evolution, detector acceptance, RF layout, orbit control, site, civil, thermal or lifecycle model.\"",
            "],",
            "\"claim_ceiling\":\"First-order local-ring rigidity, classical synchrotron and muon lifetime scaling only; no achievable collider, site selection or engineering feasibility claim.\"",
            "}}"
        ),
        rows.len(),
        fcc_hh_comparator.proton_cm_energy_tev,
        fcc_hh_comparator.proton_sr_power_mw_at_reference_current,
        fcc_energy_relative_difference,
        fcc_power_relative_difference,
        rows.join(","),
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
    fn one_hundred_km_sixteen_tesla_is_near_one_hundred_tev_cm() {
        let case = calculate_case(100.0, 16.0, 0.67).expect("valid local ring");
        close(case.proton_cm_energy_tev, 102.297_640_214, 1.0e-9);
    }

    #[test]
    fn fcc_hh_geometry_screen_reproduces_energy_and_power_scale_without_fitting() {
        let case = calculate_case(97.75, 16.0, 0.67).expect("valid FCC comparator geometry");
        close(case.proton_cm_energy_tev, 99.995_943_309, 1.0e-9);
        close(case.proton_sr_power_mw_at_reference_current, 2.333_866_237, 1.0e-9);
        assert!((case.proton_sr_power_mw_at_reference_current - 2.4).abs() / 2.4 < 0.04);
    }

    #[test]
    fn fixed_field_and_fill_preserve_muon_decay_turns_across_local_circumferences() {
        let c50 = calculate_case(50.0, 14.0, 0.67).expect("50 km");
        let c100 = calculate_case(100.0, 14.0, 0.67).expect("100 km");
        let c150 = calculate_case(150.0, 14.0, 0.67).expect("150 km");
        close(c50.muon_decay_turns, c100.muon_decay_turns, 1.0e-9);
        close(c100.muon_decay_turns, c150.muon_decay_turns, 1.0e-9);
        close(c50.two_beam_muon_luminosity_decay_turns, c50.muon_decay_turns / 2.0, 1.0e-12);
    }

    #[test]
    fn muon_radiation_burden_is_reported_separately_from_injection_power() {
        let c50 = calculate_case(50.0, 14.0, 0.67).expect("50 km");
        let c100 = calculate_case(100.0, 16.0, 0.67).expect("100 km");
        let c150 = calculate_case(150.0, 16.0, 0.67).expect("150 km");
        assert!(c50.muon_sr_to_injection_power_ratio < 1.0);
        assert!(c100.muon_sr_to_injection_power_ratio > 1.9);
        assert!(c150.muon_sr_to_injection_power_ratio > 4.0);
        assert!(c150.muon_sr_loss_fraction_per_turn < 1.0);
    }

    #[test]
    fn rejects_invalid_local_ring_inputs() {
        assert!(calculate_case(0.0, 14.0, 0.67).is_err());
        assert!(calculate_case(100.0, 0.0, 0.67).is_err());
        assert!(calculate_case(100.0, 14.0, 0.0).is_err());
        assert!(calculate_case(100.0, 14.0, 1.01).is_err());
        assert!(calculate_case(f64::NAN, 14.0, 0.67).is_err());
    }

    #[test]
    fn render_contains_all_twelve_cases_and_preserves_claim_ceiling() {
        let report = render_report_json().expect("valid local-ring report");
        assert!(report.contains("\"comparison_matrix_count\":12"));
        assert!(report.contains("\"circumference_km\":100.000000"));
        assert!(report.contains(
            "\"field_evidence_label\":\"hts_stretch_case_not_production_ready\""
        ));
        assert!(report.contains("unadjusted_screening_residual_retained_not_calibrated_away"));
        assert!(report.contains("no achievable collider"));
    }
}
