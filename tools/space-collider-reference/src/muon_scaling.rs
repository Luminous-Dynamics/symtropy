//! First-order species / circumference scaling for circular collider screening.
//!
//! This module is deliberately not a lattice, beam-dynamics, RF or radiation-transport
//! solver. It keeps the geometric assumptions explicit and refuses to label a
//! fixed-energy orbit sustainable when its isomagnetic synchrotron loss exceeds
//! the beam energy on each turn.

const C_M_S: f64 = 299_792_458.0;
const ELEMENTARY_CHARGE_C: f64 = 1.602_176_634e-19;
const HBAR_C_EV_M: f64 = 1.973_269_804e-7;
const MUON_MASS_GEV: f64 = 0.105_658_375_5;
const MUON_MEAN_LIFETIME_S: f64 = 2.196_981_1e-6;
const ELECTRON_MASS_GEV: f64 = 0.000_510_998_95;
const PROTON_MASS_GEV: f64 = 0.938_272_088_16;
const ELECTRON_CRITICAL_FIELD_T: f64 = 4.414_005_218e9;
const ELECTRON_U0_COEFFICIENT_MEV: f64 = 0.088_5;
const GEV_TO_EV: f64 = 1.0e9;
const MEV_TO_EV: f64 = 1.0e6;
const JOULES_PER_GEV: f64 = GEV_TO_EV * ELEMENTARY_CHARGE_C;

#[derive(Debug, Clone, Copy)]
struct MuonScenario {
    circumference_m: f64,
    dipole_field_t: f64,
    dipole_fill_fraction: f64,
    bending_radius_m: f64,
    beam_energy_gev: f64,
    gamma: f64,
    revolution_frequency_hz: f64,
    muon_lifetime_s: f64,
    particle_decay_turns: f64,
    luminosity_decay_turns: f64,
    bunch_energy_j: f64,
    average_circulating_bunches: f64,
    average_beam_current_a: f64,
    average_stored_energy_per_beam_j: f64,
    injection_power_per_beam_w: f64,
    muon_decay_rate_per_beam_s: f64,
    decay_energy_flow_per_beam_w: f64,
    decay_energy_flow_per_ring_m_w: f64,
    synchrotron_loss_per_turn_mev: f64,
    synchrotron_loss_fraction_per_turn: f64,
    synchrotron_power_per_beam_w: f64,
    synchrotron_to_injection_power_ratio: f64,
    critical_photon_energy_ev: f64,
    quantum_chi: f64,
}

fn synchrotron_loss_per_turn_mev(
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
    let loss =
        ELECTRON_U0_COEFFICIENT_MEV * beam_energy_gev.powi(4) / bending_radius_m * mass_scale;
    if !loss.is_finite() {
        return Err("synchrotron loss is non-finite".to_owned());
    }
    Ok(loss)
}

fn classical_quantum_parameter(
    beam_energy_gev: f64,
    dipole_field_t: f64,
    particle_mass_gev: f64,
) -> Result<f64, String> {
    if !beam_energy_gev.is_finite()
        || beam_energy_gev <= 0.0
        || !dipole_field_t.is_finite()
        || dipole_field_t <= 0.0
        || !particle_mass_gev.is_finite()
        || particle_mass_gev <= 0.0
    {
        return Err("quantum synchrotron inputs must be finite and positive".to_owned());
    }
    let gamma = beam_energy_gev / particle_mass_gev;
    let critical_field_t =
        ELECTRON_CRITICAL_FIELD_T * (particle_mass_gev / ELECTRON_MASS_GEV).powi(2);
    let chi = gamma * dipole_field_t / critical_field_t;
    if !chi.is_finite() {
        return Err("quantum synchrotron parameter is non-finite".to_owned());
    }
    Ok(chi)
}

fn calculate_scenario(
    circumference_m: f64,
    dipole_field_t: f64,
    dipole_fill_fraction: f64,
    muons_per_bunch: f64,
    bunch_repetition_hz: f64,
) -> Result<MuonScenario, String> {
    let positive = [
        circumference_m,
        dipole_field_t,
        dipole_fill_fraction,
        muons_per_bunch,
        bunch_repetition_hz,
    ];
    if positive.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return Err("muon scenario inputs must be finite and positive".to_owned());
    }
    if dipole_fill_fraction > 1.0 {
        return Err("dipole fill fraction must be in (0, 1]".to_owned());
    }
    if muons_per_bunch.fract() != 0.0 {
        return Err("muons per bunch must be an integer count".to_owned());
    }

    // The isomagnetic, constant-field approximation:
    // rho = f_dip * C / (2 pi), p[GeV/c] = 0.299792458 B[T] rho[m].
    let bending_radius_m = dipole_fill_fraction * circumference_m / (2.0 * std::f64::consts::PI);
    let beam_energy_gev = 0.299_792_458 * dipole_field_t * bending_radius_m;
    let gamma = beam_energy_gev / MUON_MASS_GEV;
    if !beam_energy_gev.is_finite() || gamma < 1.0 {
        return Err("rigidity-derived muon energy is invalid".to_owned());
    }

    let revolution_frequency_hz = C_M_S / circumference_m;
    let muon_lifetime_s = gamma * MUON_MEAN_LIFETIME_S;
    let revolution_period_s = 1.0 / revolution_frequency_hz;
    let particle_decay_turns = muon_lifetime_s / revolution_period_s;
    let luminosity_decay_turns = particle_decay_turns / 2.0;

    // A continuous-repetition, exponential-decay approximation to the
    // time-averaged bunch population. It is not a bunch-by-bunch fill pattern.
    let average_circulating_bunches = bunch_repetition_hz * muon_lifetime_s;
    let bunch_energy_j = muons_per_bunch * beam_energy_gev * JOULES_PER_GEV;
    let average_stored_energy_per_beam_j = bunch_energy_j * average_circulating_bunches;
    let average_beam_current_a = ELEMENTARY_CHARGE_C
        * muons_per_bunch
        * average_circulating_bunches
        * revolution_frequency_hz;
    let injection_power_per_beam_w = bunch_energy_j * bunch_repetition_hz;
    // In steady state, one bunch's worth of muons per repetition ultimately
    // decays out of the circulating beam. This is total energy carried into
    // decay products, not deposited heat or neutrino dose.
    let muon_decay_rate_per_beam_s = muons_per_bunch * bunch_repetition_hz;
    let decay_energy_flow_per_beam_w =
        muon_decay_rate_per_beam_s * beam_energy_gev * JOULES_PER_GEV;
    let decay_energy_flow_per_ring_m_w = decay_energy_flow_per_beam_w / circumference_m;

    let synchrotron_loss_per_turn_mev =
        synchrotron_loss_per_turn_mev(beam_energy_gev, bending_radius_m, MUON_MASS_GEV)?;
    let beam_energy_mev = beam_energy_gev * 1.0e3;
    let synchrotron_loss_fraction_per_turn = synchrotron_loss_per_turn_mev / beam_energy_mev;
    let synchrotron_power_per_beam_w =
        average_beam_current_a * synchrotron_loss_per_turn_mev * MEV_TO_EV;
    let synchrotron_to_injection_power_ratio =
        synchrotron_power_per_beam_w / injection_power_per_beam_w;

    let critical_photon_energy_ev = 1.5 * HBAR_C_EV_M * gamma.powi(3) / bending_radius_m;
    let quantum_chi = classical_quantum_parameter(beam_energy_gev, dipole_field_t, MUON_MASS_GEV)?;

    let derived = [
        bending_radius_m,
        beam_energy_gev,
        gamma,
        revolution_frequency_hz,
        muon_lifetime_s,
        particle_decay_turns,
        luminosity_decay_turns,
        bunch_energy_j,
        average_circulating_bunches,
        average_beam_current_a,
        average_stored_energy_per_beam_j,
        injection_power_per_beam_w,
        muon_decay_rate_per_beam_s,
        decay_energy_flow_per_beam_w,
        decay_energy_flow_per_ring_m_w,
        synchrotron_loss_per_turn_mev,
        synchrotron_loss_fraction_per_turn,
        synchrotron_power_per_beam_w,
        synchrotron_to_injection_power_ratio,
        critical_photon_energy_ev,
        quantum_chi,
    ];
    if derived.iter().any(|x| !x.is_finite()) {
        return Err("muon scaling produced a non-finite derived quantity".to_owned());
    }

    Ok(MuonScenario {
        circumference_m,
        dipole_field_t,
        dipole_fill_fraction,
        bending_radius_m,
        beam_energy_gev,
        gamma,
        revolution_frequency_hz,
        muon_lifetime_s,
        particle_decay_turns,
        luminosity_decay_turns,
        bunch_energy_j,
        average_circulating_bunches,
        average_beam_current_a,
        average_stored_energy_per_beam_j,
        injection_power_per_beam_w,
        muon_decay_rate_per_beam_s,
        decay_energy_flow_per_beam_w,
        decay_energy_flow_per_ring_m_w,
        synchrotron_loss_per_turn_mev,
        synchrotron_loss_fraction_per_turn,
        synchrotron_power_per_beam_w,
        synchrotron_to_injection_power_ratio,
        critical_photon_energy_ev,
        quantum_chi,
    })
}

fn scenario_status(s: MuonScenario) -> &'static str {
    if s.synchrotron_loss_fraction_per_turn >= 1.0 {
        "no_fixed_energy_circular_equilibrium_under_declared_model"
    } else if s.synchrotron_loss_fraction_per_turn >= 0.01 {
        "high_rf_replacement_burden_requires_system_level_review"
    } else if s.quantum_chi >= 1.0e-3 {
        "classical_radiation_approximation_requires_quantum_review"
    } else {
        "analytical_screen_only"
    }
}

fn render_scenario_json(s: MuonScenario) -> String {
    format!(
        concat!(
            "{{",
            "\"circumference_km\":{:.6},",
            "\"dipole_field_t\":{:.6},",
            "\"dipole_fill_fraction\":{:.9},",
            "\"bending_radius_km\":{:.6},",
            "\"beam_energy_tev_per_beam\":{:.9},",
            "\"centre_of_mass_energy_tev\":{:.9},",
            "\"lorentz_gamma\":{:.6e},",
            "\"revolution_frequency_hz\":{:.9},",
            "\"muon_mean_lifetime_s\":{:.9},",
            "\"muon_decay_turns\":{:.6},",
            "\"two_beam_luminosity_decay_turns\":{:.6},",
            "\"muons_per_bunch\":1.8e12,",
            "\"bunch_repetition_hz\":5.0,",
            "\"average_circulating_bunch_equivalents\":{:.9},",
            "\"bunch_energy_j_per_beam\":{:.6e},",
            "\"average_stored_energy_j_per_beam\":{:.6e},",
            "\"average_beam_current_a\":{:.9e},",
            "\"injection_power_w_per_beam\":{:.6e},",
            "\"muon_decay_rate_per_beam_per_s\":{:.6e},",
            "\"decay_energy_flow_w_per_beam_not_deposited_heat\":{:.6e},",
            "\"decay_energy_flow_w_per_ring_m_per_beam_not_deposited_heat\":{:.6e},",
            "\"synchrotron_loss_mev_per_turn\":{:.9e},",
            "\"synchrotron_loss_fraction_per_turn\":{:.9e},",
            "\"synchrotron_power_w_per_beam_formal\":{:.6e},",
            "\"synchrotron_to_injection_power_ratio\":{:.9e},",
            "\"critical_photon_energy_ev\":{:.9e},",
            "\"classical_quantum_parameter_chi\":{:.9e},",
            "\"status\":\"{}\"",
            "}}"
        ),
        s.circumference_m / 1000.0,
        s.dipole_field_t,
        s.dipole_fill_fraction,
        s.bending_radius_m / 1000.0,
        s.beam_energy_gev / 1000.0,
        2.0 * s.beam_energy_gev / 1000.0,
        s.gamma,
        s.revolution_frequency_hz,
        s.muon_lifetime_s,
        s.particle_decay_turns,
        s.luminosity_decay_turns,
        s.average_circulating_bunches,
        s.bunch_energy_j,
        s.average_stored_energy_per_beam_j,
        s.average_beam_current_a,
        s.injection_power_per_beam_w,
        s.muon_decay_rate_per_beam_s,
        s.decay_energy_flow_per_beam_w,
        s.decay_energy_flow_per_ring_m_w,
        s.synchrotron_loss_per_turn_mev,
        s.synchrotron_loss_fraction_per_turn,
        s.synchrotron_power_per_beam_w,
        s.synchrotron_to_injection_power_ratio,
        s.critical_photon_energy_ev,
        s.quantum_chi,
        scenario_status(s),
    )
}

#[derive(Debug, Clone, Copy)]
struct RcsStageInput {
    name: &'static str,
    circumference_m: f64,
    injection_energy_gev: f64,
    extraction_energy_gev: f64,
    acceleration_time_s: f64,
    turns: u32,
}

#[derive(Debug, Clone, Copy)]
struct RcsStageResult {
    input: RcsStageInput,
    decay_exponent: f64,
    survival_fraction: f64,
    cumulative_survival_fraction: f64,
}

/// Exact integrated decay for a linearly ramped total muon energy.
///
/// For E(t) = E_in + (E_out - E_in) t/T:
/// integral(dt / gamma(t) tau_0) =
/// (m_mu / tau_0) * T/(E_out-E_in) * ln(E_out/E_in).
fn linear_energy_ramp_survival(
    injection_energy_gev: f64,
    extraction_energy_gev: f64,
    acceleration_time_s: f64,
) -> Result<(f64, f64), String> {
    let values = [
        injection_energy_gev,
        extraction_energy_gev,
        acceleration_time_s,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err("RCS decay inputs must be finite".to_owned());
    }
    if injection_energy_gev <= 0.0
        || extraction_energy_gev <= injection_energy_gev
        || acceleration_time_s <= 0.0
    {
        return Err("RCS energy must increase over a positive acceleration time".to_owned());
    }

    let exponent = (MUON_MASS_GEV / MUON_MEAN_LIFETIME_S) * acceleration_time_s
        / (extraction_energy_gev - injection_energy_gev)
        * (extraction_energy_gev / injection_energy_gev).ln();
    let survival = (-exponent).exp();
    if !exponent.is_finite() || !survival.is_finite() || !(0.0..=1.0).contains(&survival) {
        return Err("RCS decay integration produced an invalid result".to_owned());
    }
    Ok((exponent, survival))
}

/// Independent composite-Simpson reference used only by the analytical tests.
#[cfg(test)]
fn linear_energy_ramp_survival_simpson(
    injection_energy_gev: f64,
    extraction_energy_gev: f64,
    acceleration_time_s: f64,
    intervals: usize,
) -> Result<f64, String> {
    if intervals == 0 || !intervals.is_multiple_of(2) {
        return Err("Simpson integration requires a positive even interval count".to_owned());
    }
    let values = [
        injection_energy_gev,
        extraction_energy_gev,
        acceleration_time_s,
    ];
    if values.iter().any(|value| !value.is_finite())
        || injection_energy_gev <= 0.0
        || extraction_energy_gev <= injection_energy_gev
        || acceleration_time_s <= 0.0
    {
        return Err("invalid Simpson RCS decay inputs".to_owned());
    }

    let dt = acceleration_time_s / intervals as f64;
    let delta_energy = extraction_energy_gev - injection_energy_gev;
    let integrand = |time_s: f64| {
        let energy = injection_energy_gev + delta_energy * time_s / acceleration_time_s;
        MUON_MASS_GEV / (MUON_MEAN_LIFETIME_S * energy)
    };
    let mut sum = integrand(0.0) + integrand(acceleration_time_s);
    for i in 1..intervals {
        let weight = if i % 2 == 0 { 2.0 } else { 4.0 };
        sum += weight * integrand(i as f64 * dt);
    }
    let exponent = sum * dt / 3.0;
    let survival = (-exponent).exp();
    if !survival.is_finite() || !(0.0..=1.0).contains(&survival) {
        return Err("Simpson RCS decay integration produced an invalid result".to_owned());
    }
    Ok(survival)
}

fn rcs_reference_stages() -> [RcsStageInput; 4] {
    [
        RcsStageInput {
            name: "RCS1",
            circumference_m: 5_990.0,
            injection_energy_gev: 63.0,
            extraction_energy_gev: 314.0,
            acceleration_time_s: 0.34e-3,
            turns: 17,
        },
        RcsStageInput {
            name: "RCS2",
            circumference_m: 5_990.0,
            injection_energy_gev: 314.0,
            extraction_energy_gev: 750.0,
            acceleration_time_s: 1.10e-3,
            turns: 55,
        },
        RcsStageInput {
            name: "RCS3",
            circumference_m: 10_700.0,
            injection_energy_gev: 750.0,
            extraction_energy_gev: 1_500.0,
            acceleration_time_s: 2.37e-3,
            turns: 66,
        },
        RcsStageInput {
            name: "RCS4",
            circumference_m: 35_000.0,
            injection_energy_gev: 1_500.0,
            extraction_energy_gev: 5_000.0,
            acceleration_time_s: 6.37e-3,
            turns: 55,
        },
    ]
}

fn calculate_rcs_chain_survival() -> Result<([RcsStageResult; 4], f64, f64, u32), String> {
    let stages = rcs_reference_stages();
    let mut results = [RcsStageResult {
        input: stages[0],
        decay_exponent: 0.0,
        survival_fraction: 1.0,
        cumulative_survival_fraction: 1.0,
    }; 4];
    let mut cumulative_survival = 1.0;
    let mut total_time_s = 0.0;
    let mut total_turns = 0_u32;

    for (index, input) in stages.into_iter().enumerate() {
        if index > 0 {
            let previous = stages[index - 1];
            if (previous.extraction_energy_gev - input.injection_energy_gev).abs() > 1.0e-9 {
                return Err("RCS stage energy handoff is discontinuous".to_owned());
            }
        }
        let (decay_exponent, survival_fraction) = linear_energy_ramp_survival(
            input.injection_energy_gev,
            input.extraction_energy_gev,
            input.acceleration_time_s,
        )?;
        cumulative_survival *= survival_fraction;
        total_time_s += input.acceleration_time_s;
        total_turns = total_turns
            .checked_add(input.turns)
            .ok_or_else(|| "RCS turn count overflow".to_owned())?;
        results[index] = RcsStageResult {
            input,
            decay_exponent,
            survival_fraction,
            cumulative_survival_fraction: cumulative_survival,
        };
    }

    Ok((results, cumulative_survival, total_time_s, total_turns))
}

fn render_rcs_stage_json(result: RcsStageResult) -> String {
    format!(
        concat!(
            "{{",
            "\"stage\":\"{}\",",
            "\"circumference_m\":{:.0},",
            "\"injection_energy_gev\":{:.6},",
            "\"extraction_energy_gev\":{:.6},",
            "\"acceleration_time_ms\":{:.6},",
            "\"turns\":{},",
            "\"decay_exponent\":{:.9},",
            "\"integrated_survival_fraction\":{:.9},",
            "\"cumulative_survival_fraction\":{:.9}",
            "}}"
        ),
        result.input.name,
        result.input.circumference_m,
        result.input.injection_energy_gev,
        result.input.extraction_energy_gev,
        result.input.acceleration_time_s * 1000.0,
        result.input.turns,
        result.decay_exponent,
        result.survival_fraction,
        result.cumulative_survival_fraction,
    )
}

pub fn render_muon_scaling_report_json() -> Result<String, String> {
    let source_circumference_m = 11_400.0;
    let source_beam_energy_gev = 5_000.0;
    let source_field_t = 14.0;
    let source_rigidity_radius_m = source_beam_energy_gev / (0.299_792_458 * source_field_t);
    let source_fill_fraction =
        2.0 * std::f64::consts::PI * source_rigidity_radius_m / source_circumference_m;
    let muons_per_bunch = 1.8e12;
    let repetition_hz = 5.0;

    let (rcs_stages, rcs_survival, rcs_time_s, rcs_turns) = calculate_rcs_chain_survival()?;
    let rcs_stage_json = rcs_stages
        .iter()
        .map(|stage| render_rcs_stage_json(*stage))
        .collect::<Vec<_>>()
        .join(",");

    let reference = calculate_scenario(
        source_circumference_m,
        source_field_t,
        source_fill_fraction,
        muons_per_bunch,
        repetition_hz,
    )?;

    let fixed_energy_lunar_fill =
        2.0 * std::f64::consts::PI * reference.bending_radius_m / 11_000_000.0;
    let fixed_energy_lunar = calculate_scenario(
        11_000_000.0,
        source_field_t,
        fixed_energy_lunar_fill,
        muons_per_bunch,
        repetition_hz,
    )?;

    let fixed_field_sweep = [
        calculate_scenario(
            11_400.0,
            source_field_t,
            source_fill_fraction,
            muons_per_bunch,
            repetition_hz,
        )?,
        calculate_scenario(
            100_000.0,
            source_field_t,
            source_fill_fraction,
            muons_per_bunch,
            repetition_hz,
        )?,
        calculate_scenario(
            1_000_000.0,
            source_field_t,
            source_fill_fraction,
            muons_per_bunch,
            repetition_hz,
        )?,
        calculate_scenario(
            11_000_000.0,
            source_field_t,
            source_fill_fraction,
            muons_per_bunch,
            repetition_hz,
        )?,
    ];

    let moon = fixed_field_sweep[3];
    let proton_loss_mev = synchrotron_loss_per_turn_mev(
        moon.beam_energy_gev,
        moon.bending_radius_m,
        PROTON_MASS_GEV,
    )?;
    let electron_loss_mev = synchrotron_loss_per_turn_mev(
        moon.beam_energy_gev,
        moon.bending_radius_m,
        ELECTRON_MASS_GEV,
    )?;
    let proton_chi =
        classical_quantum_parameter(moon.beam_energy_gev, source_field_t, PROTON_MASS_GEV)?;
    let electron_chi =
        classical_quantum_parameter(moon.beam_energy_gev, source_field_t, ELECTRON_MASS_GEV)?;

    let circumference_for_one_percent_loss_m =
        reference.circumference_m * (0.01 / reference.synchrotron_loss_fraction_per_turn).sqrt();
    let circumference_for_five_percent_loss_m =
        reference.circumference_m * (0.05 / reference.synchrotron_loss_fraction_per_turn).sqrt();
    let circumference_for_ten_percent_loss_m =
        reference.circumference_m * (0.10 / reference.synchrotron_loss_fraction_per_turn).sqrt();
    let circumference_for_one_hundred_percent_loss_m =
        reference.circumference_m * (1.0 / reference.synchrotron_loss_fraction_per_turn).sqrt();

    let values = [
        source_fill_fraction,
        proton_loss_mev,
        electron_loss_mev,
        circumference_for_one_percent_loss_m,
        circumference_for_five_percent_loss_m,
        circumference_for_ten_percent_loss_m,
        circumference_for_one_hundred_percent_loss_m,
    ];
    if values.iter().any(|x| !x.is_finite()) {
        return Err("muon scaling report produced a non-finite result".to_owned());
    }

    Ok(format!(
        concat!(
            "{{",
            "\"evidence_class\":\"analytical_species_scaling_screen\",",
            "\"source\":\"FERMILAB-PUB-26-0741-TD / arXiv:2611.06680, Table 1 (2026-11-05)\",",
            "\"muon_constants\":{{",
            "\"mass_gev\":0.1056583755,",
            "\"mean_lifetime_s\":2.1969811e-6,",
            "\"electron_mass_gev\":0.00051099895,",
            "\"proton_mass_gev\":0.93827208816",
            "}},",
            "\"rcs_chain_survival\":{{",
            "\"evidence_class\":\"analytic_linear_energy_ramp_decay_integration\",",
            "\"source\":\"Soubirou and Chance, arXiv:2611.02865 Table 1 (2026-11-02); MuCol Consolidated Parameters 2025\",",
            "\"stages\":[{}],",
            "\"total_acceleration_time_ms\":{:.6},",
            "\"total_turns\":{},",
            "\"integrated_survival_fraction\":{:.9},",
            "\"reference_assumed_survival_fraction_per_stage\":0.9,",
            "\"reference_cumulative_survival_fraction\":0.6561,",
            "\"absolute_difference_vs_reference_cumulative\":{:.9}",
            "}},",
            "\"source_reference\":{{",
            "\"table_energy_tev_cm\":10.0,",
            "\"beam_energy_tev\":5.0,",
            "\"circumference_km\":11.4,",
            "\"arc_peak_field_t\":14.0,",
            "\"muons_per_bunch\":1.8e12,",
            "\"bunch_repetition_hz\":5.0,",
            "\"reported_beam_power_mw_two_beams\":14.4,",
            "\"reported_luminosity_lifetime_turns\":1363.0,",
            "\"reported_beam_induced_cold_mass_power_w_per_m\":500.0,",
            "\"derived_dipole_fill_fraction_from_rigidity\":{:.9},",
            "\"derived_muon_decay_turns\":{:.6},",
            "\"derived_luminosity_decay_turns\":{:.6},",
            "\"derived_injection_power_mw_two_beams\":{:.6},",
            "\"derived_synchrotron_loss_mev_per_turn\":{:.6},",
            "\"consolidated_mucol_reference_loss_mev_per_turn\":27.2,",
            "\"relative_difference_vs_27p2_mev\":{:.6}",
            "}},",
            "\"fixed_energy_lunar_ring\":{},",
            "\"fixed_field_sweep\":[{},{},{},{}],",
            "\"fixed_field_lunar_species_comparison\":{{",
            "\"muon_loss_mev_per_turn\":{:.9e},",
            "\"proton_loss_mev_per_turn\":{:.9e},",
            "\"electron_classical_loss_mev_per_turn_formal\":{:.9e},",
            "\"muon_to_proton_loss_ratio\":{:.6e},",
            "\"muon_quantum_chi\":{:.9e},",
            "\"proton_quantum_chi\":{:.9e},",
            "\"electron_quantum_chi\":{:.9e},",
            "\"electron_classical_validity\":\"invalid_without_quantum_synchrotron_model_at_this_energy\"",
            "}},",
            "\"fixed_field_fractional_loss_thresholds\":{{",
            "\"circumference_km_for_1_percent_loss_per_turn\":{:.6},",
            "\"circumference_km_for_5_percent_loss_per_turn\":{:.6},",
            "\"circumference_km_for_10_percent_loss_per_turn\":{:.6},",
            "\"circumference_km_for_100_percent_loss_per_turn\":{:.6}",
            "}},",
            "\"assumptions\":[",
            "\"Isomagnetic circular arcs; rho = dipole_fill_fraction * circumference / (2 pi).",
            "\",",
            "\"The decay energy flow is energy carried by decay products and is not deposited heat; neutrino/electron/shower partitions require transport simulation.",
            "\",",
            "\"Classical synchrotron model; quantum validity is flagged by chi and the electron lunar-energy comparator is outside the classical regime.",
            "\",",
            "\"Peak field is used as a uniform effective arc field for this first-order screen; a real lattice requires the radiation integral I2 = integral(ds/rho(s)^2).",
            "\",",
            "\"Continuous-repetition exponential-decay average; no bunch-by-bunch fill, injection kicker, RF bucket, cooling, acceleration-chain or beam-beam model.",
            "\",",
            "\"Muon luminosity decay e-fold is half the single-beam muon population lifetime under independent equal-beam exponential decay.",
            "\",",
            "\"Formal synchrotron power is not a sustainable operating prediction when loss per turn approaches or exceeds beam energy.",
            "\"",
            "],",
            "\"claim_ceiling\":\"Relativistic lifetime, rigidity and classical synchrotron-radiation screening only; no integrated muon collider, beam stability, luminosity, detector, decay transport, neutrino dose, RF, magnet or construction qualification.\"",
            "}}"
        ),
        rcs_stage_json,
        rcs_time_s * 1000.0,
        rcs_turns,
        rcs_survival,
        (rcs_survival - 0.6561).abs(),
        source_fill_fraction,
        reference.particle_decay_turns,
        reference.luminosity_decay_turns,
        2.0 * reference.injection_power_per_beam_w / 1.0e6,
        reference.synchrotron_loss_per_turn_mev,
        (reference.synchrotron_loss_per_turn_mev - 27.2) / 27.2,
        render_scenario_json(fixed_energy_lunar),
        render_scenario_json(fixed_field_sweep[0]),
        render_scenario_json(fixed_field_sweep[1]),
        render_scenario_json(fixed_field_sweep[2]),
        render_scenario_json(fixed_field_sweep[3]),
        moon.synchrotron_loss_per_turn_mev,
        proton_loss_mev,
        electron_loss_mev,
        moon.synchrotron_loss_per_turn_mev / proton_loss_mev,
        moon.quantum_chi,
        proton_chi,
        electron_chi,
        circumference_for_one_percent_loss_m / 1000.0,
        circumference_for_five_percent_loss_m / 1000.0,
        circumference_for_ten_percent_loss_m / 1000.0,
        circumference_for_one_hundred_percent_loss_m / 1000.0,
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
    fn four_rcs_stage_integral_reproduces_reference_survival_and_turns() {
        let (stages, survival, total_time_s, turns) =
            calculate_rcs_chain_survival().expect("valid four-stage RCS chain");
        close(total_time_s * 1000.0, 10.18, 1.0e-12);
        assert_eq!(turns, 193);
        close(stages[0].survival_fraction, 0.9006486008, 1.0e-9);
        close(stages[1].survival_fraction, 0.8997454154, 1.0e-9);
        close(stages[2].survival_fraction, 0.9000191546, 1.0e-9);
        close(stages[3].survival_fraction, 0.8999808310, 1.0e-9);
        close(survival, 0.6563870932, 1.0e-10);
        close((survival - 0.6561).abs(), 0.0002870932, 1.0e-10);
    }

    #[test]
    fn analytic_rcs_survival_matches_independent_simpson_integral() {
        let (exponent, analytic_survival) =
            linear_energy_ramp_survival(63.0, 314.0, 0.34e-3).expect("valid ramp");
        let numerical_survival = linear_energy_ramp_survival_simpson(63.0, 314.0, 0.34e-3, 20_000)
            .expect("valid Simpson reference");
        assert!(exponent > 0.0);
        close(analytic_survival, numerical_survival, 1.0e-12);
        assert!(linear_energy_ramp_survival_simpson(63.0, 314.0, 0.34e-3, 0).is_err());
        assert!(linear_energy_ramp_survival_simpson(63.0, 314.0, 0.34e-3, 5).is_err());
    }

    #[test]
    fn source_reference_reproduces_muon_lifetime_and_injection_power() {
        let circumference = 11_400.0;
        let energy = 5_000.0;
        let field = 14.0;
        let radius = energy / (0.299_792_458 * field);
        let fill = 2.0 * std::f64::consts::PI * radius / circumference;
        let value = calculate_scenario(circumference, field, fill, 1.8e12, 5.0).expect("valid ref");
        close(value.particle_decay_turns, 2_734.061_3, 0.01);
        close(value.luminosity_decay_turns, 1_367.030_65, 0.01);
        close(value.injection_power_per_beam_w / 1.0e6, 7.2, 0.01);
        close(value.synchrotron_loss_per_turn_mev, 25.4, 0.1);
    }

    #[test]
    fn fixed_energy_lunar_ring_has_only_few_muon_turns() {
        let radius = 5_000.0 / (0.299_792_458 * 14.0);
        let fill = 2.0 * std::f64::consts::PI * radius / 11_000_000.0;
        let value = calculate_scenario(11_000_000.0, 14.0, fill, 1.8e12, 5.0)
            .expect("valid fixed energy lunar ring");
        close(value.beam_energy_gev, 5_000.0, 1.0e-8);
        close(value.particle_decay_turns, 2.83348, 1.0e-4);
    }

    #[test]
    fn fixed_field_lunar_ring_preserves_decay_turns_but_exceeds_one_turn_energy_loss() {
        let circumference = 11_400.0;
        let radius = 5_000.0 / (0.299_792_458 * 14.0);
        let fill = 2.0 * std::f64::consts::PI * radius / circumference;
        let value = calculate_scenario(11_000_000.0, 14.0, fill, 1.8e12, 5.0)
            .expect("valid fixed field lunar ring");
        close(value.particle_decay_turns, 2_734.061_3, 0.01);
        close(value.beam_energy_gev / 1.0e6, 4.8245614, 1.0e-5);
        close(value.synchrotron_loss_fraction_per_turn, 4.730118, 1.0e-5);
        assert_eq!(
            scenario_status(value),
            "no_fixed_energy_circular_equilibrium_under_declared_model"
        );
    }

    #[test]
    fn fixed_field_decay_energy_flow_per_ring_meter_is_invariant_in_the_simple_repetition_model() {
        let radius = 5_000.0 / (0.299_792_458 * 14.0);
        let fill = 2.0 * std::f64::consts::PI * radius / 11_400.0;
        let baseline = calculate_scenario(11_400.0, 14.0, fill, 1.8e12, 5.0).expect("baseline");
        let lunar = calculate_scenario(11_000_000.0, 14.0, fill, 1.8e12, 5.0).expect("lunar");
        close(
            lunar.decay_energy_flow_per_ring_m_w,
            baseline.decay_energy_flow_per_ring_m_w,
            1.0e-9,
        );
    }

    #[test]
    fn quantum_parameter_separates_muon_and_electron_regimes_at_lunar_energy() {
        let energy_gev = 4_824_561.404;
        let muon_chi =
            classical_quantum_parameter(energy_gev, 14.0, MUON_MASS_GEV).expect("muon chi");
        let electron_chi =
            classical_quantum_parameter(energy_gev, 14.0, ELECTRON_MASS_GEV).expect("electron chi");
        assert!(muon_chi < 1.0e-3);
        assert!(electron_chi > 1.0);
    }

    #[test]
    fn fixed_field_loss_grows_with_square_of_circumference() {
        let c0 = 11_400.0;
        let radius = 5_000.0 / (0.299_792_458 * 14.0);
        let fill = 2.0 * std::f64::consts::PI * radius / c0;
        let baseline = calculate_scenario(c0, 14.0, fill, 1.8e12, 5.0).expect("baseline");
        let larger = calculate_scenario(100_000.0, 14.0, fill, 1.8e12, 5.0).expect("larger");
        let ratio =
            larger.synchrotron_loss_fraction_per_turn / baseline.synchrotron_loss_fraction_per_turn;
        close(ratio, (100_000.0 / c0).powi(2), 1.0e-8);
    }

    #[test]
    fn muon_radiation_is_between_proton_and_electron_at_equal_energy_and_radius() {
        let radius = 1_149_500.328;
        let energy = 4_824_561.404;
        let muon = synchrotron_loss_per_turn_mev(energy, radius, MUON_MASS_GEV).expect("muon");
        let proton =
            synchrotron_loss_per_turn_mev(energy, radius, PROTON_MASS_GEV).expect("proton");
        let electron =
            synchrotron_loss_per_turn_mev(energy, radius, ELECTRON_MASS_GEV).expect("electron");
        assert!(electron > muon && muon > proton);
        close(
            muon / proton,
            (PROTON_MASS_GEV / MUON_MASS_GEV).powi(4),
            1.0e-8,
        );
    }

    #[test]
    fn rejects_invalid_species_and_geometry_inputs() {
        assert!(calculate_scenario(11_400.0, 14.0, 1.1, 1.8e12, 5.0).is_err());
        assert!(calculate_scenario(11_400.0, 0.0, 0.5, 1.8e12, 5.0).is_err());
        assert!(calculate_scenario(11_400.0, 14.0, 0.5, 1.8e12 + 0.5, 5.0).is_err());
        assert!(synchrotron_loss_per_turn_mev(5_000.0, 0.0, MUON_MASS_GEV).is_err());
    }

    #[test]
    fn report_contains_fixed_energy_fixed_field_and_power_dispositions() {
        let report = render_muon_scaling_report_json().expect("valid report");
        assert!(report.contains("fixed_energy_lunar_ring"));
        assert!(report.contains("fixed_field_sweep"));
        assert!(report.contains("no_fixed_energy_circular_equilibrium_under_declared_model"));
        assert!(report.contains("Classical synchrotron model"));
        assert!(report.contains("decay_energy_flow_w_per_beam_not_deposited_heat"));
        assert!(report.contains("electron_quantum_chi"));
        assert!(report.contains("electron_classical_validity"));
    }
}
