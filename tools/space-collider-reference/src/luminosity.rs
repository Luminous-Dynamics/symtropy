//! Arithmetic closure for the published lunar-collider parameter set.
//!
//! This is not a beam-dynamics model. It checks dimensional and accounting
//! consistency between source-published beam parameters and derived rates.

const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;
const ELEMENTARY_CHARGE_C: f64 = 1.602_176_634e-19;
const PROTON_REST_ENERGY_GEV: f64 = 0.938_272_088_16;
const PROTON_CLASSICAL_RADIUS_M: f64 = 1.534_7e-18;
const CM2_PER_M2: f64 = 1.0e-4;
const CM2_PER_MB: f64 = 1.0e-27;
const SECONDS_PER_DAY: f64 = 86_400.0;
const CM2_PER_AB_INV: f64 = 1.0e42;

#[derive(Debug, Clone, Copy)]
struct CcmInputs {
    circumference_m: f64,
    beam_energy_tev: f64,
    beam_current_reported_a: f64,
    total_bunches_per_beam: f64,
    colliding_bunch_pairs_per_ip: f64,
    protons_per_bunch: f64,
    nominal_bunch_spacing_s: f64,
    beta_star_m: f64,
    beam_beam_tune_shift_per_ip: f64,
    geometric_factor: f64,
    interaction_points: f64,
    total_cross_section_mb: f64,
    reported_luminosity_per_ip_cm2_s: f64,
    synchrotron_power_per_beam_w: f64,
    scheduled_physics_days_per_year: f64,
    availability: f64,
    effective_luminosity_fraction: f64,
}

impl CcmInputs {
    fn published_reference() -> Self {
        Self {
            circumference_m: 11_000_000.0,
            beam_energy_tev: 7_000.0,
            beam_current_reported_a: 0.5,
            total_bunches_per_beam: 1_200_000.0,
            colliding_bunch_pairs_per_ip: 1_200_000.0,
            protons_per_bunch: 1.0e11,
            nominal_bunch_spacing_s: 25.0e-9,
            beta_star_m: 0.5,
            beam_beam_tune_shift_per_ip: 0.06,
            geometric_factor: 1.0,
            interaction_points: 2.0,
            total_cross_section_mb: 283.0,
            reported_luminosity_per_ip_cm2_s: 2.0e38,
            synchrotron_power_per_beam_w: 8.5e12,
            scheduled_physics_days_per_year: 160.0,
            availability: 0.70,
            effective_luminosity_fraction: 1.0,
        }
    }

    fn validate(self) -> Result<(), String> {
        let positive = [
            self.circumference_m,
            self.beam_energy_tev,
            self.beam_current_reported_a,
            self.total_bunches_per_beam,
            self.colliding_bunch_pairs_per_ip,
            self.protons_per_bunch,
            self.nominal_bunch_spacing_s,
            self.beta_star_m,
            self.beam_beam_tune_shift_per_ip,
            self.geometric_factor,
            self.interaction_points,
            self.total_cross_section_mb,
            self.reported_luminosity_per_ip_cm2_s,
            self.synchrotron_power_per_beam_w,
            self.scheduled_physics_days_per_year,
        ];
        if positive
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        {
            return Err("all dimensional inputs must be finite and positive".to_owned());
        }
        if !self.availability.is_finite() || !(0.0..=1.0).contains(&self.availability) {
            return Err("availability must be finite and in [0, 1]".to_owned());
        }
        if !self.effective_luminosity_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.effective_luminosity_fraction)
        {
            return Err("effective luminosity fraction must be finite and in [0, 1]".to_owned());
        }
        let integer_counts = [
            self.total_bunches_per_beam,
            self.colliding_bunch_pairs_per_ip,
            self.protons_per_bunch,
            self.interaction_points,
        ];
        if integer_counts.iter().any(|value| value.fract() != 0.0) {
            return Err("bunch, particle and interaction-point counts must be integers".to_owned());
        }
        if self.colliding_bunch_pairs_per_ip > self.total_bunches_per_beam {
            return Err(
                "colliding bunch pairs per IP cannot exceed total stored bunches".to_owned(),
            );
        }
        if self.beam_beam_tune_shift_per_ip >= 1.0 {
            return Err("beam-beam tune shift must be less than one".to_owned());
        }
        if self.geometric_factor > 1.0 {
            return Err("the declared geometric reduction factor must be in (0, 1]".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct CcmClosure {
    revolution_frequency_hz: f64,
    beam_current_from_bunches_a: f64,
    beam_current_relative_residual: f64,
    stored_energy_per_beam_from_bunches_j: f64,
    stored_energy_per_beam_from_reported_current_j: f64,
    stored_energy_current_vs_bunch_relative_residual: f64,
    stored_energy_both_beams_j: f64,
    ccm_to_fcc_stored_energy_ratio: f64,
    collision_crossing_rate_per_ip_hz: f64,
    nominal_full_fill_slot_rate_hz: f64,
    nominal_fill_fraction: f64,
    gamma: f64,
    luminosity_from_reported_current_cm2_s: f64,
    luminosity_from_bunch_derived_current_cm2_s: f64,
    reported_luminosity_cm2_s: f64,
    total_cross_section_interactions_per_crossing: f64,
    implied_cross_section_for_one_million_per_crossing_mb: f64,
    burnoff_lifetime_s: f64,
    synchrotron_energy_loss_per_turn_tev: f64,
    synchrotron_loss_fraction_per_turn: f64,
    annual_integrated_luminosity_ab_inv: f64,
}

fn calculate(inputs: CcmInputs) -> Result<CcmClosure, String> {
    inputs.validate()?;

    let revolution_frequency_hz = SPEED_OF_LIGHT_M_S / inputs.circumference_m;
    let beam_current_from_bunches_a = ELEMENTARY_CHARGE_C
        * inputs.protons_per_bunch
        * inputs.total_bunches_per_beam
        * revolution_frequency_hz;

    let collision_crossing_rate_per_ip_hz =
        inputs.colliding_bunch_pairs_per_ip * revolution_frequency_hz;
    let nominal_full_fill_slot_rate_hz = 1.0 / inputs.nominal_bunch_spacing_s;
    let slots_per_turn = nominal_full_fill_slot_rate_hz / revolution_frequency_hz;
    let nominal_fill_fraction = inputs.total_bunches_per_beam / slots_per_turn;
    if nominal_fill_fraction > 1.0 {
        return Err(
            "stored bunch count exceeds the declared nominal bunch-spacing slots".to_owned(),
        );
    }

    let gamma = inputs.beam_energy_tev * 1_000.0 / PROTON_REST_ENERGY_GEV;
    let luminosity_scale_m2_s = gamma / (ELEMENTARY_CHARGE_C * PROTON_CLASSICAL_RADIUS_M)
        * inputs.beam_beam_tune_shift_per_ip
        * inputs.geometric_factor
        / inputs.beta_star_m;
    let luminosity_from_reported_current_cm2_s =
        luminosity_scale_m2_s * inputs.beam_current_reported_a * CM2_PER_M2;
    let luminosity_from_bunch_derived_current_cm2_s =
        luminosity_scale_m2_s * beam_current_from_bunches_a * CM2_PER_M2;

    let total_cross_section_cm2 = inputs.total_cross_section_mb * CM2_PER_MB;
    let total_cross_section_interactions_per_crossing = inputs.reported_luminosity_per_ip_cm2_s
        * total_cross_section_cm2
        / collision_crossing_rate_per_ip_hz;
    let implied_cross_section_for_one_million_per_crossing_mb = 1.0e6
        * collision_crossing_rate_per_ip_hz
        / inputs.reported_luminosity_per_ip_cm2_s
        / CM2_PER_MB;

    let total_protons_per_beam = inputs.total_bunches_per_beam * inputs.protons_per_bunch;
    let stored_energy_per_beam_from_bunches_j =
        total_protons_per_beam * inputs.beam_energy_tev * 1.0e12 * ELEMENTARY_CHARGE_C;
    let stored_energy_per_beam_from_reported_current_j =
        inputs.beam_current_reported_a * inputs.beam_energy_tev * 1.0e12 / revolution_frequency_hz;
    let stored_energy_current_vs_bunch_relative_residual =
        (stored_energy_per_beam_from_reported_current_j - stored_energy_per_beam_from_bunches_j)
            / stored_energy_per_beam_from_bunches_j;
    let stored_energy_both_beams_j = 2.0 * stored_energy_per_beam_from_bunches_j;
    let ccm_to_fcc_stored_energy_ratio = stored_energy_per_beam_from_bunches_j / 8.4e9;
    let burnoff_lifetime_s = total_protons_per_beam
        / (inputs.reported_luminosity_per_ip_cm2_s
            * inputs.interaction_points
            * total_cross_section_cm2);

    // P / I has units volts. Dividing by 1e12 converts eV/turn to TeV/turn
    // for a singly charged proton.
    let synchrotron_energy_loss_per_turn_tev =
        (inputs.synchrotron_power_per_beam_w / inputs.beam_current_reported_a) / 1.0e12;
    let synchrotron_loss_fraction_per_turn =
        synchrotron_energy_loss_per_turn_tev / inputs.beam_energy_tev;

    let scheduled_seconds = inputs.scheduled_physics_days_per_year * SECONDS_PER_DAY;
    let annual_integrated_luminosity_ab_inv = inputs.reported_luminosity_per_ip_cm2_s
        * scheduled_seconds
        * inputs.availability
        * inputs.effective_luminosity_fraction
        / CM2_PER_AB_INV;

    let beam_current_relative_residual = (beam_current_from_bunches_a
        - inputs.beam_current_reported_a)
        / inputs.beam_current_reported_a;
    let values = [
        revolution_frequency_hz,
        beam_current_from_bunches_a,
        beam_current_relative_residual,
        stored_energy_per_beam_from_bunches_j,
        stored_energy_per_beam_from_reported_current_j,
        stored_energy_current_vs_bunch_relative_residual,
        stored_energy_both_beams_j,
        ccm_to_fcc_stored_energy_ratio,
        collision_crossing_rate_per_ip_hz,
        nominal_full_fill_slot_rate_hz,
        nominal_fill_fraction,
        gamma,
        luminosity_from_reported_current_cm2_s,
        luminosity_from_bunch_derived_current_cm2_s,
        total_cross_section_interactions_per_crossing,
        implied_cross_section_for_one_million_per_crossing_mb,
        burnoff_lifetime_s,
        synchrotron_energy_loss_per_turn_tev,
        synchrotron_loss_fraction_per_turn,
        annual_integrated_luminosity_ab_inv,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err("derived collider performance quantity is non-finite".to_owned());
    }

    Ok(CcmClosure {
        revolution_frequency_hz,
        beam_current_from_bunches_a,
        beam_current_relative_residual,
        stored_energy_per_beam_from_bunches_j,
        stored_energy_per_beam_from_reported_current_j,
        stored_energy_current_vs_bunch_relative_residual,
        stored_energy_both_beams_j,
        ccm_to_fcc_stored_energy_ratio,
        collision_crossing_rate_per_ip_hz,
        nominal_full_fill_slot_rate_hz,
        nominal_fill_fraction,
        gamma,
        luminosity_from_reported_current_cm2_s,
        luminosity_from_bunch_derived_current_cm2_s,
        reported_luminosity_cm2_s: inputs.reported_luminosity_per_ip_cm2_s,
        total_cross_section_interactions_per_crossing,
        implied_cross_section_for_one_million_per_crossing_mb,
        burnoff_lifetime_s,
        synchrotron_energy_loss_per_turn_tev,
        synchrotron_loss_fraction_per_turn,
        annual_integrated_luminosity_ab_inv,
    })
}

fn luminosity_for_tune_shift(
    base: CcmInputs,
    beam_current_a: f64,
    beta_star_m: f64,
    tune_shift: f64,
) -> Result<f64, String> {
    if !beam_current_a.is_finite()
        || beam_current_a <= 0.0
        || !beta_star_m.is_finite()
        || beta_star_m <= 0.0
        || !tune_shift.is_finite()
        || tune_shift <= 0.0
        || tune_shift >= 1.0
    {
        return Err("sensitivity inputs must be finite and physically positive".to_owned());
    }
    let gamma = base.beam_energy_tev * 1_000.0 / PROTON_REST_ENERGY_GEV;
    let result_m2_s = gamma / (ELEMENTARY_CHARGE_C * PROTON_CLASSICAL_RADIUS_M)
        * (beam_current_a / beta_star_m)
        * tune_shift
        * base.geometric_factor;
    let result_cm2_s = result_m2_s * CM2_PER_M2;
    if !result_cm2_s.is_finite() {
        return Err("sensitivity result is non-finite".to_owned());
    }
    Ok(result_cm2_s)
}

fn render_sensitivity_rows(base: CcmInputs, xis: &[f64], betas: &[f64]) -> Result<String, String> {
    let mut output = String::from("{\n    \"beam_beam_tune_shift\": [");
    for (index, xi) in xis.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let luminosity =
            luminosity_for_tune_shift(base, base.beam_current_reported_a, base.beta_star_m, *xi)?;
        output.push_str(&format!(
            "{{\"xi\":{xi:.6},\"luminosity_cm2_s\":{luminosity:.6e}}}"
        ));
    }
    output.push_str("],\n    \"beta_star\": [");
    for (index, beta) in betas.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        let luminosity = luminosity_for_tune_shift(
            base,
            base.beam_current_reported_a,
            *beta,
            base.beam_beam_tune_shift_per_ip,
        )?;
        output.push_str(&format!(
            "{{\"beta_star_m\":{beta:.6},\"luminosity_cm2_s\":{luminosity:.6e}}}"
        ));
    }
    output.push_str("]\n  }");
    Ok(output)
}

pub fn render_report_json() -> Result<String, String> {
    let inputs = CcmInputs::published_reference();
    let result = calculate(inputs)?;

    let fcc_circumference_m = 97_750.0;
    let fcc_revolution_frequency_hz = SPEED_OF_LIGHT_M_S / fcc_circumference_m;
    let fcc_baseline_bunches = 10_600.0;
    let fcc_baseline_crossing_rate_hz = fcc_baseline_bunches * fcc_revolution_frequency_hz;
    let fcc_synchrotron_loss_per_turn_tev = (2.4e6 / 0.5) / 1.0e12;

    let sensitivities =
        render_sensitivity_rows(inputs, &[0.006, 0.01, 0.03, 0.06, 0.08], &[0.25, 0.5, 1.0])?;

    Ok(format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"evidence_class\": \"analytical_performance_closure\",\n",
            "  \"source_case\": \"Beacham-Zimmermann-CCM-2022-tentative-parameters\",\n",
            "  \"inputs\": {{\n",
            "    \"circumference_km\": 11000.0,\n",
            "    \"beam_energy_tev_per_beam\": 7000.0,\n",
            "    \"beam_current_reported_a_per_beam\": 0.5,\n",
            "    \"total_bunches_per_beam\": 1200000,\n",
            "    \"colliding_bunch_pairs_per_ip\": 1200000,\n",
            "    \"protons_per_bunch\": 1.0e11,\n",
            "    \"nominal_bunch_spacing_ns\": 25.0,\n",
            "    \"beta_star_m\": 0.5,\n",
            "    \"beam_beam_tune_shift_per_ip\": 0.06,\n",
            "    \"geometric_factor\": 1.0,\n",
            "    \"interaction_points\": 2,\n",
            "    \"total_cross_section_mb\": 283.0,\n",
            "    \"reported_luminosity_per_ip_cm2_s\": 2.0e38,\n",
            "    \"synchrotron_power_per_beam_w\": 8.5e12,\n",
            "    \"scheduled_physics_days_per_year\": 160.0,\n",
            "    \"availability\": 0.70,\n",
            "    \"effective_luminosity_fraction\": 1.0\n",
            "  }},\n",
            "  \"derived\": {{\n",
            "    \"revolution_frequency_hz\": {:.6},\n",
            "    \"beam_current_from_bunch_population_a\": {:.6},\n",
            "    \"beam_current_relative_residual_vs_reported\": {:.6},\n",
            "    \"collision_crossing_rate_per_ip_hz\": {:.6},\n",
            "    \"full_fill_slot_rate_from_25ns_hz\": {:.6},\n",
            "    \"nominal_fill_fraction\": {:.6},\n",
            "    \"lorentz_gamma\": {:.6e},\n",
            "    \"luminosity_from_reported_current_cm2_s\": {:.6e},\n",
            "    \"luminosity_from_bunch_derived_current_cm2_s\": {:.6e},\n",
            "    \"paper_reported_luminosity_cm2_s\": {:.6e},\n",
            "    \"total_cross_section_interactions_per_crossing\": {:.6e},\n",
            "    \"implied_cross_section_for_1e6_interactions_per_crossing_mb\": {:.6},\n",
            "    \"burnoff_lifetime_s\": {:.6},\n",
            "    \"burnoff_lifetime_min\": {:.6},\n",
            "    \"synchrotron_energy_loss_per_turn_tev\": {:.6},\n",
            "    \"synchrotron_loss_fraction_of_beam_energy_per_turn\": {:.6e},\n",
            "    \"annual_integrated_luminosity_ab_inv\": {:.6},\n",
            "    \"stored_energy_per_beam_from_bunch_population_j\": {:.6e},\n",
            "    \"stored_energy_per_beam_from_reported_current_j\": {:.6e},\n",
            "    \"stored_energy_current_vs_bunch_relative_residual\": {:.6},\n",
            "    \"stored_energy_both_beams_j\": {:.6e},\n",
            "    \"fcc_hh_cdr_stored_energy_per_beam_gj\": 8.4,\n",
            "    \"ccm_to_fcc_stored_energy_ratio\": {:.6}\n",
            "  }},\n",
            "  \"fcc_hh_cdr_baseline_reference\": {{\n",
            "    \"circumference_km\": 97.75,\n",
            "    \"centre_of_mass_energy_tev\": 100.0,\n",
            "    \"arc_dipole_field_t\": 16.0,\n",
            "    \"beam_current_a\": 0.5,\n",
            "    \"bunches_per_beam_baseline\": 10600,\n",
            "    \"revolution_frequency_hz\": {:.6},\n",
            "    \"collision_crossing_rate_per_ip_hz\": {:.6},\n",
            "    \"synchrotron_power_per_beam_mw\": 2.4,\n",
            "    \"synchrotron_energy_loss_per_turn_tev_from_P_over_I\": {:.6e},\n",
            "    \"peak_luminosity_baseline_cm2_s\": 5.0e34,\n",
            "    \"peak_luminosity_ultimate_cm2_s\": 3.0e35,\n",
            "    \"reported_events_per_crossing_baseline\": 170,\n",
            "    \"reported_events_per_crossing_ultimate\": 1020,\n",
            "    \"source\": \"CERN-2025-006 Table 2.1; baseline and ultimate values kept separate\"\n",
            "  }},\n",
            "  \"sensitivity\": {},\n",
            "  \"interpretation\": [\n",
            "    \"The 25 ns nominal slot rate is not the actual collision rate when the ring is not fully filled; this source case yields a fill fraction below one.\",\n",
            "    \"The total-cross-section interaction count is a rate proxy, not detector-accepted pile-up. The paper reports pile-up only as approximately 1e6; the independently recomputed 1.73e6 total-cross-section proxy is consistent at that order of magnitude. Do not infer a separate inelastic cross section or detector-accepted event count from the rounded table entry.\",\n",
            "    \"The beam-beam tune shift xi=0.06 is an assumed parameter extrapolated in the paper, not independently qualified proton-beam stability. Sensitivity rows are arithmetic only.\",\n",
            "    \"Burn-off lifetime excludes other loss mechanisms. Annual integrated luminosity assumes the stated scheduled time, availability and effective luminosity fraction.\",\n",
            "    \"The synchrotron-radiation loss per turn is computed from the paper's stated per-beam power and beam current.\"\n",
            "  ],\n",
            "  \"claim_ceiling\": \"arithmetic closure of the declared literature reference parameters only; no beam dynamics, tune-shift validation, detector, magnet, RF, orbital, physical or construction qualification\"\n",
            "}}"
        ),
        result.revolution_frequency_hz,
        result.beam_current_from_bunches_a,
        result.beam_current_relative_residual,
        result.collision_crossing_rate_per_ip_hz,
        result.nominal_full_fill_slot_rate_hz,
        result.nominal_fill_fraction,
        result.gamma,
        result.luminosity_from_reported_current_cm2_s,
        result.luminosity_from_bunch_derived_current_cm2_s,
        result.reported_luminosity_cm2_s,
        result.total_cross_section_interactions_per_crossing,
        result.implied_cross_section_for_one_million_per_crossing_mb,
        result.burnoff_lifetime_s,
        result.burnoff_lifetime_s / 60.0,
        result.synchrotron_energy_loss_per_turn_tev,
        result.synchrotron_loss_fraction_per_turn,
        result.annual_integrated_luminosity_ab_inv,
        result.stored_energy_per_beam_from_bunches_j,
        result.stored_energy_per_beam_from_reported_current_j,
        result.stored_energy_current_vs_bunch_relative_residual,
        result.stored_energy_both_beams_j,
        result.ccm_to_fcc_stored_energy_ratio,
        fcc_revolution_frequency_hz,
        fcc_baseline_crossing_rate_hz,
        fcc_synchrotron_loss_per_turn_tev,
        sensitivities,
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
    fn lunar_revolution_and_collision_rates_reproduce_the_source_case() {
        let value = calculate(CcmInputs::published_reference()).expect("valid CCM");
        close(value.revolution_frequency_hz, 27.2538598, 1.0e-6);
        close(
            value.collision_crossing_rate_per_ip_hz / 1.0e6,
            32.704632,
            1.0e-5,
        );
        assert!(value.nominal_fill_fraction > 0.80 && value.nominal_fill_fraction < 0.83);
    }

    #[test]
    fn bunch_population_reconstructs_the_rounded_half_ampere_beam_current() {
        let value = calculate(CcmInputs::published_reference()).expect("valid CCM");
        close(value.beam_current_from_bunches_a, 0.5238, 0.001);
        assert!(value.beam_current_relative_residual.abs() < 0.05);
    }

    #[test]
    fn luminosity_tune_shift_relation_is_close_to_the_rounded_paper_value() {
        let value = calculate(CcmInputs::published_reference()).expect("valid CCM");
        assert!(value.luminosity_from_reported_current_cm2_s > 1.7e38);
        assert!(value.luminosity_from_reported_current_cm2_s < 2.0e38);
        assert!(value.luminosity_from_bunch_derived_current_cm2_s > 1.8e38);
        assert!(value.luminosity_from_bunch_derived_current_cm2_s < 2.1e38);
    }

    #[test]
    fn total_cross_section_burnoff_and_interaction_proxy_are_reproduced() {
        let value = calculate(CcmInputs::published_reference()).expect("valid CCM");
        assert!(value.total_cross_section_interactions_per_crossing > 1.6e6);
        assert!(value.total_cross_section_interactions_per_crossing < 1.9e6);
        close(value.burnoff_lifetime_s / 60.0, 17.7, 0.2);
        assert!(value.implied_cross_section_for_one_million_per_crossing_mb > 150.0);
        assert!(value.implied_cross_section_for_one_million_per_crossing_mb < 180.0);
    }

    #[test]
    fn synchrotron_power_closes_to_about_seventeen_tev_per_turn() {
        let value = calculate(CcmInputs::published_reference()).expect("valid CCM");
        close(value.synchrotron_energy_loss_per_turn_tev, 17.0, 1.0e-9);
        close(
            value.synchrotron_loss_fraction_per_turn,
            17.0 / 7000.0,
            1.0e-12,
        );
    }

    #[test]
    fn scheduled_time_and_availability_reproduce_the_reported_annual_luminosity() {
        let value = calculate(CcmInputs::published_reference()).expect("valid CCM");
        close(value.annual_integrated_luminosity_ab_inv, 1935.36, 1.0e-6);
    }

    #[test]
    fn lunar_stored_beam_energy_closes_against_current_and_fcc_reference() {
        let value = calculate(CcmInputs::published_reference()).expect("valid CCM");
        close(
            value.stored_energy_per_beam_from_bunches_j / 1.0e12,
            134.582_837_256,
            1.0e-6,
        );
        close(
            value.stored_energy_both_beams_j / 1.0e12,
            269.165_674_512,
            1.0e-6,
        );
        assert!(value.stored_energy_current_vs_bunch_relative_residual.abs() < 0.05);
        close(value.ccm_to_fcc_stored_energy_ratio, 16_021.766_34, 0.01);
    }

    #[test]
    fn fcc_reference_reproduces_thousand_hertz_revolution_and_million_hertz_crossing_rates() {
        let revolution_hz = SPEED_OF_LIGHT_M_S / 97_750.0;
        close(revolution_hz, 3_066.930_516_624_041, 1.0e-9);
        close(
            revolution_hz * 10_600.0 / 1.0e6,
            32.509_463_476_214_83,
            1.0e-9,
        );
    }

    #[test]
    fn sensitivity_sweeps_are_monotone_and_preserve_the_declared_relation() {
        let base = CcmInputs::published_reference();
        let low = luminosity_for_tune_shift(base, 0.5, 0.5, 0.006).expect("valid xi");
        let nominal = luminosity_for_tune_shift(base, 0.5, 0.5, 0.06).expect("valid xi");
        let high = luminosity_for_tune_shift(base, 0.5, 0.5, 0.08).expect("valid xi");
        assert!(low < nominal && nominal < high);
        close(nominal / low, 10.0, 1.0e-12);

        let short_beta = luminosity_for_tune_shift(base, 0.5, 0.25, 0.06).expect("valid beta");
        let long_beta = luminosity_for_tune_shift(base, 0.5, 1.0, 0.06).expect("valid beta");
        close(short_beta / nominal, 2.0, 1.0e-12);
        close(long_beta / nominal, 0.5, 1.0e-12);
    }

    #[test]
    fn rejects_invalid_units_or_physical_parameters_before_reporting() {
        let mut invalid = CcmInputs::published_reference();
        invalid.circumference_m = 0.0;
        assert!(calculate(invalid).is_err());

        let mut invalid = CcmInputs::published_reference();
        invalid.geometric_factor = 1.1;
        assert!(calculate(invalid).is_err());

        let mut invalid = CcmInputs::published_reference();
        invalid.availability = 1.01;
        assert!(calculate(invalid).is_err());

        let mut invalid = CcmInputs::published_reference();
        invalid.interaction_points = f64::NAN;
        assert!(calculate(invalid).is_err());

        let mut invalid = CcmInputs::published_reference();
        invalid.colliding_bunch_pairs_per_ip = 1_200_000.5;
        assert!(calculate(invalid).is_err());

        let mut invalid = CcmInputs::published_reference();
        invalid.colliding_bunch_pairs_per_ip = 1_200_001.0;
        assert!(calculate(invalid).is_err());
    }

    #[test]
    fn report_keeps_total_interaction_proxy_and_detector_event_claim_separate() {
        let report = render_report_json().expect("valid report");
        assert!(report.contains("total_cross_section_interactions_per_crossing"));
        assert!(report.contains("not detector-accepted pile-up"));
        assert!(report.contains("implied_cross_section_for_1e6_interactions_per_crossing_mb"));
    }
}
