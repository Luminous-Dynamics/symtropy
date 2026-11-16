//! Synthetic per-taxel calibration model-form comparison.
//!
//! The TouchWGNN-style rational curve is a published baseline for distributed
//! piezoresistive arrays. This harness compares per-taxel and pooled forms on a
//! deterministic 113-taxel synthetic array. No physical sensor is represented.

#![forbid(unsafe_code)]

use super::*;

const TAXEL_COUNT: usize = 113;
const FULL_FORCE_GRID_N: [f64; 11] =
    [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0];
const SPARSE_SIX_INDICES: [usize; 6] = [0, 2, 4, 6, 8, 10];
const SPARSE_FOUR_INDICES: [usize; 4] = [0, 3, 7, 10];
const HELDOUT_FORCE_N: [f64; 10] =
    [0.25, 0.75, 1.25, 1.75, 2.25, 2.75, 3.25, 3.75, 4.25, 4.75];

#[derive(Clone, Copy, Debug, PartialEq)]
struct TaxelProfile {
    a: f64,
    b: f64,
    c: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CalibrationObservation {
    taxel_id: usize,
    force_setpoint_n: f64,
    voltage_v: f64,
    reference_force_n: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RationalFit {
    a: f64,
    b: f64,
    c: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct AffineFit {
    gain: f64,
    offset: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelFormSummary {
    pub model_id: &'static str,
    pub reference_contacts_per_taxel: usize,
    pub total_reference_contacts: usize,
    pub pooled_rmse_n: f64,
    pub mean_taxel_rmse_n: f64,
    pub p95_taxel_rmse_n: f64,
    pub worst_taxel_rmse_n: f64,
    pub worst_taxel_id: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelFormCampaignReport {
    pub taxel_count: usize,
    pub full_grid_contacts_per_taxel: usize,
    pub heldout_force_count_per_taxel: usize,
    pub results: Vec<ModelFormSummary>,
}

fn taxel_profile(taxel_id: usize) -> TaxelProfile {
    // Deliberate article-to-article/taxel transfer-function variation.
    TaxelProfile {
        a: 1.80 + (taxel_id as f64) * 0.0008,
        b: 0.15 + ((taxel_id % 17) as f64) * 0.0004,
        c: -0.10 + ((taxel_id % 13) as f64) * 0.0003,
    }
}

/// Forward sensor model for the rational inverse calibration form:
/// F(V) = b * V / (a - V) + c.
fn voltage_for_force(profile: TaxelProfile, force_n: f64) -> f64 {
    let shifted_force = force_n - profile.c;
    profile.a * shifted_force / (profile.b + shifted_force)
}

fn force_from_rational(fit: RationalFit, voltage_v: f64) -> f64 {
    fit.b * voltage_v / (fit.a - voltage_v) + fit.c
}

fn deterministic_unit(seed: u64, taxel_id: usize, force_index: usize) -> f64 {
    let mut state = seed
        ^ (taxel_id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (force_index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
    if state == 0 {
        state = 1;
    }
    let value = unit_interval(&mut state);
    (value - 0.5) * 2.0
}

fn observations_for_taxel(
    taxel_id: usize,
    profile: TaxelProfile,
    force_setpoints_n: &[f64],
    seed: u64,
) -> Vec<CalibrationObservation> {
    force_setpoints_n
        .iter()
        .enumerate()
        .map(|(force_index, force_n)| {
            let voltage_v = voltage_for_force(profile, *force_n)
                + deterministic_unit(seed, taxel_id, force_index) * 0.00015;
            let reference_force_n = *force_n
                + deterministic_unit(seed ^ 0xA076_1D64_78BD_642F, taxel_id, force_index)
                    * 0.0015;
            CalibrationObservation {
                taxel_id,
                force_setpoint_n: *force_n,
                voltage_v,
                reference_force_n,
            }
        })
        .collect()
}

/// Conditional linear least squares over a bounded grid of the asymptote a.
/// For fixed a, y = b * (V / (a - V)) + c is linear in b and c.
/// The grid search makes the fit deterministic and avoids an unchecked
/// unconstrained three-parameter optimizer near the saturation singularity.
fn fit_rational(observations: &[CalibrationObservation]) -> Result<RationalFit, String> {
    if observations.len() < 4
        || observations.iter().any(|observation| {
            !observation.voltage_v.is_finite()
                || !observation.reference_force_n.is_finite()
                || observation.voltage_v < 0.0
        })
    {
        return Err("rational calibration requires at least four finite observations".to_owned());
    }

    let max_voltage = observations
        .iter()
        .map(|observation| observation.voltage_v)
        .fold(f64::NEG_INFINITY, f64::max);
    let start_a = max_voltage + 0.0005;
    let steps = 750_usize;
    let step = 0.0002_f64;
    let mut best: Option<(f64, RationalFit)> = None;

    for grid_index in 0..=steps {
        let a = start_a + grid_index as f64 * step;
        let mut sum_z = 0.0;
        let mut sum_y = 0.0;
        let mut sum_zz = 0.0;
        let mut sum_zy = 0.0;
        let count = observations.len() as f64;
        let mut valid = true;

        for observation in observations {
            let denominator = a - observation.voltage_v;
            if denominator <= 0.0 {
                valid = false;
                break;
            }
            let z = observation.voltage_v / denominator;
            let y = observation.reference_force_n;
            sum_z += z;
            sum_y += y;
            sum_zz += z * z;
            sum_zy += z * y;
        }
        if !valid {
            continue;
        }

        let determinant = count * sum_zz - sum_z * sum_z;
        if determinant <= 1.0e-14 {
            continue;
        }
        let b = (count * sum_zy - sum_z * sum_y) / determinant;
        let c = (sum_y - b * sum_z) / count;
        if !b.is_finite() || !c.is_finite() || b <= 0.0 {
            continue;
        }

        let fit = RationalFit { a, b, c };
        let squared_error = observations
            .iter()
            .map(|observation| {
                let error =
                    observation.reference_force_n - force_from_rational(fit, observation.voltage_v);
                error * error
            })
            .sum::<f64>();
        if best.is_none_or(|(best_error, _)| squared_error < best_error) {
            best = Some((squared_error, fit));
        }
    }

    best.map(|(_, fit)| fit)
        .ok_or_else(|| "rational calibration fit is not identifiable on this sample set".to_owned())
}

fn fit_affine(observations: &[CalibrationObservation]) -> Result<AffineFit, String> {
    if observations.len() < 2 {
        return Err("affine calibration requires at least two observations".to_owned());
    }
    let count = observations.len() as f64;
    let sum_x = observations.iter().map(|item| item.voltage_v).sum::<f64>();
    let sum_y = observations
        .iter()
        .map(|item| item.reference_force_n)
        .sum::<f64>();
    let sum_xx = observations
        .iter()
        .map(|item| item.voltage_v * item.voltage_v)
        .sum::<f64>();
    let sum_xy = observations
        .iter()
        .map(|item| item.voltage_v * item.reference_force_n)
        .sum::<f64>();
    let determinant = count * sum_xx - sum_x * sum_x;
    if determinant.abs() <= 1.0e-14 {
        return Err("affine calibration design is singular".to_owned());
    }
    let gain = (count * sum_xy - sum_x * sum_y) / determinant;
    let offset = (sum_y - gain * sum_x) / count;
    if !gain.is_finite() || !offset.is_finite() {
        return Err("affine calibration coefficients are non-finite".to_owned());
    }
    Ok(AffineFit { gain, offset })
}

fn selected_force_setpoints(indices: &[usize]) -> Result<Vec<f64>, String> {
    indices
        .iter()
        .map(|index| {
            FULL_FORCE_GRID_N
                .get(*index)
                .copied()
                .ok_or_else(|| "force-grid index is out of bounds".to_owned())
        })
        .collect()
}

fn heldout_predictions(
    taxel_id: usize,
    profile: TaxelProfile,
    seed: u64,
) -> Vec<(f64, f64)> {
    HELDOUT_FORCE_N
        .iter()
        .enumerate()
        .map(|(index, force_n)| {
            let voltage_v = voltage_for_force(profile, *force_n)
                + deterministic_unit(seed ^ 0xE703_7ED1_A0B4_28DB, taxel_id, index)
                    * 0.00015;
            (*force_n, voltage_v)
        })
        .collect()
}

fn summarize_errors(
    model_id: &'static str,
    contacts_per_taxel: usize,
    taxel_errors: &[(usize, Vec<f64>)],
) -> Result<ModelFormSummary, String> {
    if taxel_errors.len() != TAXEL_COUNT || taxel_errors.iter().any(|(_, errors)| errors.is_empty()) {
        return Err("model-form scorecard requires one nonempty held-out error vector per taxel".to_owned());
    }
    let mut taxel_rmse = Vec::with_capacity(TAXEL_COUNT);
    let mut all_squared = 0.0;
    let mut all_count = 0_usize;
    for (taxel_id, errors) in taxel_errors {
        let squared = errors.iter().map(|error| error * error).sum::<f64>();
        all_squared += squared;
        all_count += errors.len();
        taxel_rmse.push((*taxel_id, (squared / errors.len() as f64).sqrt()));
    }
    let pooled_rmse_n = (all_squared / all_count as f64).sqrt();
    let mut ordered: Vec<_> = taxel_rmse.iter().map(|(_, rmse)| *rmse).collect();
    ordered.sort_by(f64::total_cmp);
    let p95_index = ((ordered.len() - 1) as f64 * 0.95).ceil() as usize;
    let worst = taxel_rmse
        .iter()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .ok_or_else(|| "taxel RMSE summary is empty".to_owned())?;
    let mean = ordered.iter().sum::<f64>() / ordered.len() as f64;

    Ok(ModelFormSummary {
        model_id,
        reference_contacts_per_taxel: contacts_per_taxel,
        total_reference_contacts: contacts_per_taxel * TAXEL_COUNT,
        pooled_rmse_n,
        mean_taxel_rmse_n: mean,
        p95_taxel_rmse_n: ordered[p95_index],
        worst_taxel_rmse_n: worst.1,
        worst_taxel_id: worst.0,
    })
}

fn score_per_taxel_rational(
    seed: u64,
    force_indices: &[usize],
    model_id: &'static str,
) -> Result<ModelFormSummary, String> {
    let setpoints = selected_force_setpoints(force_indices)?;
    let mut taxel_errors = Vec::with_capacity(TAXEL_COUNT);
    for taxel_id in 0..TAXEL_COUNT {
        let profile = taxel_profile(taxel_id);
        let training =
            observations_for_taxel(taxel_id, profile, &setpoints, seed ^ 0x1357_9BDF_2468_ACE0);
        let fit = fit_rational(&training)?;
        let errors = heldout_predictions(taxel_id, profile, seed)
            .into_iter()
            .map(|(truth, voltage)| truth - force_from_rational(fit, voltage))
            .collect();
        taxel_errors.push((taxel_id, errors));
    }
    summarize_errors(model_id, setpoints.len(), &taxel_errors)
}

fn score_shared_rational(
    seed: u64,
    force_indices: &[usize],
) -> Result<ModelFormSummary, String> {
    let setpoints = selected_force_setpoints(force_indices)?;
    let mut pooled_training = Vec::with_capacity(TAXEL_COUNT * setpoints.len());
    for taxel_id in 0..TAXEL_COUNT {
        pooled_training.extend(observations_for_taxel(
            taxel_id,
            taxel_profile(taxel_id),
            &setpoints,
            seed ^ 0x1357_9BDF_2468_ACE0,
        ));
    }
    let fit = fit_rational(&pooled_training)?;
    let mut taxel_errors = Vec::with_capacity(TAXEL_COUNT);
    for taxel_id in 0..TAXEL_COUNT {
        let profile = taxel_profile(taxel_id);
        let errors = heldout_predictions(taxel_id, profile, seed)
            .into_iter()
            .map(|(truth, voltage)| truth - force_from_rational(fit, voltage))
            .collect();
        taxel_errors.push((taxel_id, errors));
    }
    summarize_errors("shared_rational_6_per_taxel", setpoints.len(), &taxel_errors)
}

fn score_per_taxel_affine(
    seed: u64,
    force_indices: &[usize],
) -> Result<ModelFormSummary, String> {
    let setpoints = selected_force_setpoints(force_indices)?;
    let mut taxel_errors = Vec::with_capacity(TAXEL_COUNT);
    for taxel_id in 0..TAXEL_COUNT {
        let profile = taxel_profile(taxel_id);
        let training =
            observations_for_taxel(taxel_id, profile, &setpoints, seed ^ 0x1357_9BDF_2468_ACE0);
        let fit = fit_affine(&training)?;
        let errors = heldout_predictions(taxel_id, profile, seed)
            .into_iter()
            .map(|(truth, voltage)| truth - (fit.gain * voltage + fit.offset))
            .collect();
        taxel_errors.push((taxel_id, errors));
    }
    summarize_errors("per_taxel_affine_6", setpoints.len(), &taxel_errors)
}

/// Compare the published TouchWGNN-style per-taxel rational curve with sparse
/// per-taxel fitting, a pooled/global curve and per-taxel affine fitting.
/// All results are synthetic; the TouchWGNN paper values remain a separate
/// paper-reported comparator and are not treated as reproduced here.
pub fn run_model_form_campaign(seed: u64) -> Result<ModelFormCampaignReport, String> {
    if seed == 0 {
        return Err("model-form campaign seed must be nonzero".to_owned());
    }
    let full_grid: Vec<_> = (0..FULL_FORCE_GRID_N.len()).collect();
    let six = SPARSE_SIX_INDICES.to_vec();
    let four = SPARSE_FOUR_INDICES.to_vec();

    let results = vec![
        score_per_taxel_rational(seed, &full_grid, "per_taxel_rational_11")?,
        score_per_taxel_rational(seed, &six, "per_taxel_rational_6")?,
        score_per_taxel_rational(seed, &four, "per_taxel_rational_4")?,
        score_shared_rational(seed, &six)?,
        score_per_taxel_affine(seed, &six)?,
    ];

    Ok(ModelFormCampaignReport {
        taxel_count: TAXEL_COUNT,
        full_grid_contacts_per_taxel: FULL_FORCE_GRID_N.len(),
        heldout_force_count_per_taxel: HELDOUT_FORCE_N.len(),
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_form_recovers_a_known_taxel_curve() {
        let expected = TaxelProfile {
            a: 1.92,
            b: 0.17,
            c: -0.11,
        };
        let observations: Vec<_> = FULL_FORCE_GRID_N
            .iter()
            .enumerate()
            .map(|(_index, force_n)| CalibrationObservation {
                taxel_id: 0,
                force_setpoint_n: *force_n,
                voltage_v: voltage_for_force(expected, *force_n),
                reference_force_n: *force_n,
            })
            .collect();
        let fit = fit_rational(&observations).expect("rational fit");
        assert!((fit.a - expected.a).abs() < 0.001);
        assert!((fit.b - expected.b).abs() < 0.01);
        assert!((fit.c - expected.c).abs() < 0.01);
    }

    #[test]
    fn model_form_campaign_is_deterministic_and_reports_all_taxels() {
        let first = run_model_form_campaign(20261116).expect("campaign");
        let replay = run_model_form_campaign(20261116).expect("replay");
        assert_eq!(first, replay);
        assert_eq!(first.taxel_count, TAXEL_COUNT);
        assert_eq!(first.results.len(), 5);
        assert_eq!(first.results[0].total_reference_contacts, TAXEL_COUNT * 11);
        assert_eq!(first.results[1].total_reference_contacts, TAXEL_COUNT * 6);
        assert_eq!(first.results[2].total_reference_contacts, TAXEL_COUNT * 4);
        for result in &first.results {
            assert!(result.pooled_rmse_n.is_finite());
            assert!(result.worst_taxel_rmse_n.is_finite());
            assert!(result.p95_taxel_rmse_n.is_finite());
        }
    }

    #[test]
    fn per_taxel_rational_fit_beats_a_single_shared_curve_on_heterogeneous_taxels() {
        let report = run_model_form_campaign(20261116).expect("campaign");
        let per_taxel = report
            .results
            .iter()
            .find(|result| result.model_id == "per_taxel_rational_6")
            .expect("per-taxel rational result");
        let shared = report
            .results
            .iter()
            .find(|result| result.model_id == "shared_rational_6_per_taxel")
            .expect("shared rational result");
        assert!(per_taxel.pooled_rmse_n < shared.pooled_rmse_n);
        assert!(per_taxel.worst_taxel_rmse_n < shared.worst_taxel_rmse_n);
    }
}
