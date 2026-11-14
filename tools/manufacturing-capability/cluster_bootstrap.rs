//! Lot-cluster bootstrap reference for manufacturing capability intervals.
//!
//! Measurements are nested in windows, windows are nested in independently
//! sampled lots. The primary bootstrap resamples whole lots. Resampling windows
//! is included only as a deliberately naive pseudo-replication comparator.
//!
//! Build:
//!   rustc --edition=2024 -D warnings --test tools/manufacturing-capability/cluster_bootstrap.rs -o /tmp/cluster-bootstrap-tests
//!   /tmp/cluster-bootstrap-tests
//!   rustc --edition=2024 -D warnings tools/manufacturing-capability/cluster_bootstrap.rs -o /tmp/cluster-bootstrap
//!   /tmp/cluster-bootstrap --verify
//!   /tmp/cluster-bootstrap --csv
//!   /tmp/cluster-bootstrap --coverage

const LSL: f64 = 0.0;
const USL: f64 = 1.0;
const LOT_MEAN_SD: f64 = 0.05;
const WITHIN_SD: f64 = 0.04;
const WINDOWS_PER_LOT: usize = 4;
const SAMPLES_PER_WINDOW: usize = 8;
const MIN_INDEPENDENT_LOTS: usize = 8;
const DEFAULT_BOOTSTRAPS: usize = 1_000;
const VERIFY_OUTER_CAMPAIGNS: usize = 24;
const VERIFY_BOOTSTRAPS: usize = 120;
const COVERAGE_OUTER_CAMPAIGNS: usize = 500;
const COVERAGE_BOOTSTRAPS: usize = 250;
const SEED: u64 = 0x6d61_6e75_6661_6374;

#[derive(Clone, Debug)]
struct Window {
    id: String,
    values: Vec<f64>,
}

#[derive(Clone, Debug)]
struct Lot {
    id: String,
    windows: Vec<Window>,
}

#[derive(Clone, Copy, Debug)]
struct Statistics {
    cpk: f64,
    ppk: f64,
}

#[derive(Clone, Copy, Debug)]
struct Interval {
    lower: f64,
    upper: f64,
}

impl Interval {
    fn width(self) -> f64 { self.upper - self.lower }
    fn contains(self, value: f64) -> bool { self.lower <= value && value <= self.upper }
}

#[derive(Clone, Copy, Debug)]
struct BootstrapIntervals {
    cpk: Interval,
    ppk: Interval,
    valid_replicates: usize,
    invalid_replicates: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResamplingUnit {
    WholeLotCluster,
    NaiveWindow,
}

#[derive(Clone, Copy, Debug)]
struct Comparison {
    point: Statistics,
    cluster: BootstrapIntervals,
    naive_window: BootstrapIntervals,
    independent_lots: usize,
    windows: usize,
    observations: usize,
}

#[derive(Clone, Copy, Debug)]
struct Coverage {
    outer_campaigns: usize,
    lots_per_campaign: usize,
    bootstrap_replicates: usize,
    cluster_cpk_coverage: f64,
    naive_cpk_coverage: f64,
    cluster_ppk_coverage: f64,
    naive_ppk_coverage: f64,
    cluster_ppk_mean_width: f64,
    naive_ppk_mean_width: f64,
}

#[derive(Clone, Debug)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn unit_open(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64 + 0.5) / 9_007_199_254_740_992.0
    }

    fn normal(&mut self) -> f64 {
        let u1 = self.unit_open();
        let u2 = self.unit_open();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    fn index(&mut self, length: usize) -> usize {
        // Rejection-free modulo is adequate for these benchmark resampling
        // indices; the slight modulo bias is documented in the RNG ID.
        (self.next_u64() % length as u64) as usize
    }
}

fn sample_variance(values: &[f64]) -> Result<f64, String> {
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return Err("SAMPLE_INVALID".into());
    }
    let average = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|value| (value - average).powi(2)).sum::<f64>()
        / (values.len() - 1) as f64;
    if variance.is_finite() && variance > 0.0 {
        Ok(variance)
    } else {
        Err("SAMPLE_VARIANCE_INVALID".into())
    }
}

fn statistics(windows: &[&Window]) -> Result<Statistics, String> {
    if windows.is_empty() {
        return Err("WINDOWS_EMPTY".into());
    }
    let mut values = Vec::new();
    let mut within_ss = 0.0;
    let mut within_df = 0_usize;
    for window in windows {
        let variance = sample_variance(&window.values)?;
        within_ss += (window.values.len() - 1) as f64 * variance;
        within_df += window.values.len() - 1;
        values.extend_from_slice(&window.values);
    }
    if values.len() < 2 || within_df == 0 {
        return Err("OBSERVATION_COUNT_INVALID".into());
    }
    let average = values.iter().sum::<f64>() / values.len() as f64;
    let overall_variance = sample_variance(&values)?;
    let within_variance = within_ss / within_df as f64;
    if !average.is_finite() || !within_variance.is_finite() || within_variance <= 0.0 {
        return Err("CAPABILITY_VARIANCE_INVALID".into());
    }
    let overall_sd = overall_variance.sqrt();
    let within_sd = within_variance.sqrt();
    let cpk = (USL - average).min(average - LSL) / (3.0 * within_sd);
    let ppk = (USL - average).min(average - LSL) / (3.0 * overall_sd);
    if !cpk.is_finite() || !ppk.is_finite() {
        return Err("CAPABILITY_INDEX_NONFINITE".into());
    }
    Ok(Statistics { cpk, ppk })
}

fn percentile(sorted_values: &[f64], probability: f64) -> Result<f64, String> {
    if sorted_values.is_empty() || !(0.0..=1.0).contains(&probability)
        || sorted_values.iter().any(|value| !value.is_finite()) {
        return Err("PERCENTILE_INPUT_INVALID".into());
    }
    let position = probability * (sorted_values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    Ok(sorted_values[lower] * (1.0 - fraction) + sorted_values[upper] * fraction)
}

fn interval(values: &mut [f64]) -> Result<Interval, String> {
    values.sort_by(f64::total_cmp);
    Ok(Interval {
        lower: percentile(values, 0.025)?,
        upper: percentile(values, 0.975)?,
    })
}

fn validate_lots(lots: &[Lot]) -> Result<(), String> {
    if lots.len() < MIN_INDEPENDENT_LOTS {
        return Err("INSUFFICIENT_CLUSTERS".into());
    }
    let mut lot_ids = std::collections::HashSet::new();
    let mut window_ids = std::collections::HashSet::new();
    for lot in lots {
        if lot.id.trim().is_empty() || !lot_ids.insert(lot.id.clone()) {
            return Err("LOT_ID_EMPTY_OR_DUPLICATE".into());
        }
        if lot.windows.is_empty() {
            return Err("LOT_HAS_NO_WINDOWS".into());
        }
        for window in &lot.windows {
            if window.id.trim().is_empty() || !window_ids.insert(window.id.clone()) {
                return Err("WINDOW_ID_EMPTY_OR_DUPLICATE".into());
            }
            if window.values.len() != SAMPLES_PER_WINDOW {
                return Err("WINDOW_SAMPLE_COUNT_MISMATCH".into());
            }
            sample_variance(&window.values)?;
        }
    }
    Ok(())
}

fn bootstrap_intervals(
    lots: &[Lot],
    replicates: usize,
    seed: u64,
    unit: ResamplingUnit,
) -> Result<BootstrapIntervals, String> {
    validate_lots(lots)?;
    if replicates < 100 {
        return Err("BOOTSTRAP_REPLICATES_INSUFFICIENT".into());
    }
    let mut rng = Rng::new(seed);
    let flat_windows = lots.iter().flat_map(|lot| lot.windows.iter()).collect::<Vec<_>>();
    let mut cpk = Vec::with_capacity(replicates);
    let mut ppk = Vec::with_capacity(replicates);
    let mut invalid = 0_usize;

    for _ in 0..replicates {
        let mut selected = Vec::new();
        match unit {
            ResamplingUnit::WholeLotCluster => {
                for _ in 0..lots.len() {
                    let index = rng.index(lots.len());
                    selected.extend(lots[index].windows.iter());
                }
            }
            ResamplingUnit::NaiveWindow => {
                for _ in 0..flat_windows.len() {
                    selected.push(flat_windows[rng.index(flat_windows.len())]);
                }
            }
        }

        match statistics(&selected) {
            Ok(result) => {
                cpk.push(result.cpk);
                ppk.push(result.ppk);
            }
            Err(_) => invalid += 1,
        }
    }

    if cpk.len() < replicates * 99 / 100 {
        return Err("BOOTSTRAP_INVALID_REPLICATE_RATE_EXCEEDED".into());
    }
    Ok(BootstrapIntervals {
        cpk: interval(&mut cpk)?,
        ppk: interval(&mut ppk)?,
        valid_replicates: replicates - invalid,
        invalid_replicates: invalid,
    })
}

fn generate_lots(
    lot_count: usize,
    drift: bool,
    rng: &mut Rng,
    campaign_prefix: &str,
) -> Vec<Lot> {
    let mut lots = Vec::with_capacity(lot_count);
    for lot_index in 0..lot_count {
        let lot_effect = LOT_MEAN_SD * rng.normal();
        let mut windows = Vec::with_capacity(WINDOWS_PER_LOT);
        for window_index in 0..WINDOWS_PER_LOT {
            let systematic = if drift {
                0.16 * (window_index as f64 - 1.5) / 3.0
            } else {
                0.0
            };
            let window_effect = if drift { 0.008 * rng.normal() } else { 0.0 };
            let values = (0..SAMPLES_PER_WINDOW)
                .map(|_| 0.5 + lot_effect + systematic + window_effect + WITHIN_SD * rng.normal())
                .collect();
            windows.push(Window {
                id: format!("{campaign_prefix}-LOT-{lot_index:03}-WINDOW-{window_index:02}"),
                values,
            });
        }
        lots.push(Lot {
            id: format!("{campaign_prefix}-LOT-{lot_index:03}"),
            windows,
        });
    }
    lots
}

fn compare(
    lots: &[Lot],
    replicates: usize,
    seed: u64,
) -> Result<Comparison, String> {
    validate_lots(lots)?;
    let all_windows = lots.iter().flat_map(|lot| lot.windows.iter()).collect::<Vec<_>>();
    let point = statistics(&all_windows)?;
    let cluster = bootstrap_intervals(
        lots, replicates, seed, ResamplingUnit::WholeLotCluster
    )?;
    let naive_window = bootstrap_intervals(
        lots, replicates, seed ^ 0x9e37_79b9_7f4a_7c15, ResamplingUnit::NaiveWindow
    )?;
    let windows = all_windows.len();
    let observations = all_windows.iter().map(|window| window.values.len()).sum();
    Ok(Comparison {
        point,
        cluster,
        naive_window,
        independent_lots: lots.len(),
        windows,
        observations,
    })
}

fn theoretical_truth() -> (f64, f64) {
    let marginal_sd = (LOT_MEAN_SD.powi(2) + WITHIN_SD.powi(2)).sqrt();
    let within_cpk = (USL - 0.5).min(0.5 - LSL) / (3.0 * WITHIN_SD);
    let marginal_ppk = (USL - 0.5).min(0.5 - LSL) / (3.0 * marginal_sd);
    (within_cpk, marginal_ppk)
}

fn coverage_experiment(
    outer_campaigns: usize,
    lot_count: usize,
    bootstrap_replicates: usize,
    seed: u64,
) -> Result<Coverage, String> {
    if outer_campaigns < 20 || lot_count < MIN_INDEPENDENT_LOTS || bootstrap_replicates < 100 {
        return Err("COVERAGE_EXPERIMENT_SETTINGS_INVALID".into());
    }
    let (true_cpk, true_ppk) = theoretical_truth();
    let mut rng = Rng::new(seed);
    let mut cluster_cpk_covered = 0_usize;
    let mut naive_cpk_covered = 0_usize;
    let mut cluster_ppk_covered = 0_usize;
    let mut naive_ppk_covered = 0_usize;
    let mut cluster_width_sum = 0.0;
    let mut naive_width_sum = 0.0;

    for outer in 0..outer_campaigns {
        let lots = generate_lots(lot_count, false, &mut rng, &format!("OUTER-{outer:04}"));
        let cluster = bootstrap_intervals(
            &lots, bootstrap_replicates, rng.next_u64(), ResamplingUnit::WholeLotCluster
        )?;
        let naive = bootstrap_intervals(
            &lots, bootstrap_replicates, rng.next_u64(), ResamplingUnit::NaiveWindow
        )?;
        cluster_cpk_covered += usize::from(cluster.cpk.contains(true_cpk));
        naive_cpk_covered += usize::from(naive.cpk.contains(true_cpk));
        cluster_ppk_covered += usize::from(cluster.ppk.contains(true_ppk));
        naive_ppk_covered += usize::from(naive.ppk.contains(true_ppk));
        cluster_width_sum += cluster.ppk.width();
        naive_width_sum += naive.ppk.width();
    }

    let denominator = outer_campaigns as f64;
    Ok(Coverage {
        outer_campaigns,
        lots_per_campaign: lot_count,
        bootstrap_replicates,
        cluster_cpk_coverage: cluster_cpk_covered as f64 / denominator,
        naive_cpk_coverage: naive_cpk_covered as f64 / denominator,
        cluster_ppk_coverage: cluster_ppk_covered as f64 / denominator,
        naive_ppk_coverage: naive_ppk_covered as f64 / denominator,
        cluster_ppk_mean_width: cluster_width_sum / denominator,
        naive_ppk_mean_width: naive_width_sum / denominator,
    })
}

fn verify() -> Result<(), String> {
    let mut rng = Rng::new(SEED);
    let lots = generate_lots(20, false, &mut rng, "VERIFY");
    let report = compare(&lots, 600, SEED ^ 0x1234_5678)?;
    let (true_cpk, true_ppk) = theoretical_truth();

    if report.independent_lots != 20 || report.windows != 80 || report.observations != 640 {
        return Err("INDEPENDENT_UNIT_ACCOUNTING_FAILED".into());
    }
    if report.point.cpk <= 0.0 || report.point.ppk <= 0.0
        || !report.cluster.cpk.contains(report.point.cpk)
        || !report.cluster.ppk.contains(report.point.ppk) {
        return Err("POINT_ESTIMATE_OR_CLUSTER_INTERVAL_INVALID".into());
    }
    if report.cluster.ppk.width() <= report.naive_window.ppk.width() * 1.15 {
        return Err("NAIVE_WINDOW_PSEUDOREPLICATION_NOT_EXPOSED".into());
    }
    if report.cluster.cpk.lower >= report.cluster.cpk.upper
        || report.cluster.ppk.lower >= report.cluster.ppk.upper
        || report.cluster.valid_replicates + report.cluster.invalid_replicates != 600 {
        return Err("CLUSTER_INTERVAL_OR_REPLICATE_ACCOUNTING_INVALID".into());
    }

    let too_few = generate_lots(7, false, &mut rng, "TOO-FEW");
    if bootstrap_intervals(&too_few, 200, SEED, ResamplingUnit::WholeLotCluster).is_ok() {
        return Err("INSUFFICIENT_CLUSTERS_NOT_REJECTED".into());
    }

    let stable_coverage = coverage_experiment(40, 40, 200, SEED ^ 0xdead_beef)?;
    if stable_coverage.cluster_ppk_coverage < stable_coverage.naive_ppk_coverage
        || stable_coverage.cluster_ppk_mean_width <= stable_coverage.naive_ppk_mean_width {
        return Err("CLUSTER_BOOTSTRAP_DID_NOT_REDUCE_PSEUDOREPLICATION_ERROR".into());
    }
    Ok(())
}

fn emit_csv() -> Result<(), String> {
    println!("campaign,independent_lots,windows,observations,point_cpk,cluster_cpk_lower,cluster_cpk_upper,cluster_ppk_lower,cluster_ppk_upper,cluster_cpk_width,cluster_ppk_width,naive_window_cpk_lower,naive_window_cpk_upper,naive_window_ppk_lower,naive_window_ppk_upper,naive_window_ppk_width,cluster_valid_replicates,cluster_invalid_replicates,naive_valid_replicates,naive_invalid_replicates,rng");
    let mut rng = Rng::new(SEED);
    for (name, drift) in [("stable", false), ("drift", true)] {
        let lots = generate_lots(20, drift, &mut rng, name);
        let result = compare(&lots, DEFAULT_BOOTSTRAPS, rng.next_u64())?;
        println!(
            "{},{},{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{},{},{},xorshift64star-boxmuller-v1-modulo-index",
            name, result.independent_lots, result.windows, result.observations,
            result.point.cpk, result.cluster.cpk.lower, result.cluster.cpk.upper,
            result.cluster.ppk.lower, result.cluster.ppk.upper,
            result.cluster.cpk.width(), result.cluster.ppk.width(),
            result.naive_window.cpk.lower, result.naive_window.cpk.upper,
            result.naive_window.ppk.lower, result.naive_window.ppk.upper,
            result.naive_window.ppk.width(),
            result.cluster.valid_replicates, result.cluster.invalid_replicates,
            result.naive_window.valid_replicates, result.naive_window.invalid_replicates
        );
    }
    Ok(())
}

fn emit_coverage() -> Result<(), String> {
    println!("outer_campaigns,lots_per_campaign,bootstrap_replicates,cluster_cpk_coverage,naive_window_cpk_coverage,cluster_ppk_coverage,naive_window_ppk_coverage,cluster_ppk_mean_width,naive_window_ppk_mean_width");
    for lot_count in [8_usize, 12, 20, 40, 80] {
        let result = coverage_experiment(
            COVERAGE_OUTER_CAMPAIGNS,
            lot_count,
            COVERAGE_BOOTSTRAPS,
            SEED ^ lot_count as u64
        )?;
        println!(
            "{},{},{},{:.6},{:.6},{:.6},{:.6},{:.12e},{:.12e}",
            result.outer_campaigns, result.lots_per_campaign,
            result.bootstrap_replicates, result.cluster_cpk_coverage,
            result.naive_cpk_coverage, result.cluster_ppk_coverage,
            result.naive_ppk_coverage, result.cluster_ppk_mean_width,
            result.naive_ppk_mean_width
        );
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("LOT_CLUSTER_BOOTSTRAP_OK: independent-lot capability intervals")),
        [arg] if arg == "--verify" => verify().map(|()| println!("LOT_CLUSTER_BOOTSTRAP_OK: independent-lot capability intervals")),
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--coverage" => emit_coverage(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: cluster-bootstrap [--verify|--csv|--coverage|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv, --coverage or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("LOT_CLUSTER_BOOTSTRAP_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytic_capability_truth_is_separate_for_within_and_overall_variation() {
        let (true_cpk, true_ppk) = theoretical_truth();
        assert!((true_cpk - 4.166_666_666_666_667).abs() < 1.0e-14);
        assert!((true_ppk - 2.602_896_031_476_768).abs() < 1.0e-14);
        assert!(true_ppk < true_cpk);
    }

    #[test]
    fn bootstrap_rejects_too_few_independent_lots() {
        let mut rng = Rng::new(SEED);
        let lots = generate_lots(7, false, &mut rng, "SMALL");
        assert_eq!(
            bootstrap_intervals(&lots, 200, SEED, ResamplingUnit::WholeLotCluster)
                .unwrap_err(),
            "INSUFFICIENT_CLUSTERS"
        );
    }

    #[test]
    fn deterministic_cluster_bootstrap_is_reproducible() {
        let mut rng = Rng::new(SEED);
        let lots = generate_lots(20, false, &mut rng, "REPLAY");
        let first = bootstrap_intervals(&lots, 400, SEED, ResamplingUnit::WholeLotCluster).unwrap();
        let second = bootstrap_intervals(&lots, 400, SEED, ResamplingUnit::WholeLotCluster).unwrap();
        assert_eq!(first.cpk.lower, second.cpk.lower);
        assert_eq!(first.cpk.upper, second.cpk.upper);
        assert_eq!(first.ppk.lower, second.ppk.lower);
        assert_eq!(first.ppk.upper, second.ppk.upper);
    }

    #[test]
    fn cluster_interval_is_wider_than_naive_window_bootstrap_under_shared_lot_effects() {
        let mut rng = Rng::new(SEED);
        let lots = generate_lots(20, false, &mut rng, "PSEUDOREPLICATION");
        let comparison = compare(&lots, 600, SEED ^ 0x1234_5678).unwrap();
        assert!(
            comparison.cluster.ppk.width() > comparison.naive_window.ppk.width() * 1.15,
            "cluster width={} naive width={}",
            comparison.cluster.ppk.width(),
            comparison.naive_window.ppk.width()
        );
    }

    #[test]
    fn bootstrap_intervals_are_ordered_and_keep_replicate_accounting() {
        let mut rng = Rng::new(SEED);
        let lots = generate_lots(20, false, &mut rng, "TRUTH");
        let comparison = compare(&lots, 600, SEED ^ 0x1234_5678).unwrap();
        let (cpk, ppk) = theoretical_truth();
        assert!(comparison.cluster.cpk.lower < comparison.cluster.cpk.upper);
        assert!(comparison.cluster.ppk.lower < comparison.cluster.ppk.upper);
        assert!(comparison.cluster.valid_replicates >= 594);
        assert!(comparison.cluster.cpk.contains(comparison.point.cpk));
        assert!(comparison.cluster.ppk.contains(comparison.point.ppk));
        assert!(cpk.is_finite() && ppk.is_finite());
    }

    #[test]
    fn coverage_experiment_records_group_and_observation_units_separately() {
        let result = coverage_experiment(40, 40, 150, SEED ^ 0xdead_beef).unwrap();
        assert_eq!(result.outer_campaigns, 40);
        assert_eq!(result.lots_per_campaign, 40);
        assert!(result.cluster_ppk_mean_width > 0.0);
        assert!(result.naive_ppk_mean_width > 0.0);
    }

    #[test]
    fn invalid_window_and_nonfinite_measurement_reject() {
        let mut rng = Rng::new(SEED);
        let mut lots = generate_lots(20, false, &mut rng, "INVALID");
        lots[0].windows[0].values[0] = f64::NAN;
        assert!(validate_lots(&lots).is_err());

        let mut lots = generate_lots(20, false, &mut rng, "DUPLICATE");
        lots[0].windows[1].id = lots[0].windows[0].id.clone();
        assert!(validate_lots(&lots).is_err());
    }
}
