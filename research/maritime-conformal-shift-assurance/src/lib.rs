// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::VecDeque;

const TARGET_ALPHA_MILLI: u16 = 100;
const INITIAL_CALIBRATION_SIZE: usize = 160;
const ROLLING_WINDOW_SIZE: usize = 160;
const MINIMUM_CALIBRATION_SIZE: usize = 40;
const ARRIVAL_SCALE_HOURS: u32 = 5;
const HANDLING_SCALE_HOURS: u32 = 20;
const ACI_STEP_MILLI: i32 = 50;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResidualPair {
    arrival_abs_hours: u32,
    handling_abs_hours: u32,
}

impl ResidualPair {
    fn joint_score(self) -> u32 {
        let arrival = self.arrival_abs_hours.div_ceil(ARRIVAL_SCALE_HOURS);
        let handling = self.handling_abs_hours.div_ceil(HANDLING_SCALE_HOURS);
        arrival.max(handling)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Window {
    Stationary,
    Drift,
    Burst,
}

impl Window {
    fn id(self) -> &'static str {
        match self {
            Self::Stationary => "STATIONARY",
            Self::Drift => "TEMPORAL_DRIFT",
            Self::Burst => "BURST_TAIL",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Method {
    FixedMarginal,
    FixedJoint,
    RollingJoint,
    AdaptiveJoint,
}

impl Method {
    const ALL: [Self; 4] = [
        Self::FixedMarginal,
        Self::FixedJoint,
        Self::RollingJoint,
        Self::AdaptiveJoint,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::FixedMarginal => "FIXED_MARGINAL_SPLIT",
            Self::FixedJoint => "FIXED_JOINT_SPLIT",
            Self::RollingJoint => "ROLLING_JOINT",
            Self::AdaptiveJoint => "ADAPTIVE_JOINT_ACI",
        }
    }

    fn uses_rolling_history(self) -> bool {
        matches!(self, Self::RollingJoint | Self::AdaptiveJoint)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PredictionBand {
    arrival_half_width_hours: u32,
    handling_half_width_hours: u32,
}

impl PredictionBand {
    fn joint(quantile: u32) -> Self {
        Self {
            arrival_half_width_hours: quantile.saturating_mul(ARRIVAL_SCALE_HOURS),
            handling_half_width_hours: quantile.saturating_mul(HANDLING_SCALE_HOURS),
        }
    }

    fn marginal(arrival_quantile: u32, handling_quantile: u32) -> Self {
        Self {
            arrival_half_width_hours: arrival_quantile.saturating_mul(ARRIVAL_SCALE_HOURS),
            handling_half_width_hours: handling_quantile.saturating_mul(HANDLING_SCALE_HOURS),
        }
    }

    fn covers(self, residual: ResidualPair) -> (bool, bool, bool) {
        let arrival = residual.arrival_abs_hours <= self.arrival_half_width_hours;
        let handling = residual.handling_abs_hours <= self.handling_half_width_hours;
        (arrival && handling, arrival, handling)
    }
}

fn conformal_quantile(scores: &[u32], alpha_milli: u16) -> Option<u32> {
    if scores.len() < MINIMUM_CALIBRATION_SIZE || alpha_milli == 0 || alpha_milli >= 1_000 {
        return None;
    }

    let mut ordered = scores.to_vec();
    ordered.sort_unstable();

    // Split-conformal finite-sample order statistic:
    // k = ceil((n + 1) * (1 - alpha)).
    let n = ordered.len() as u64;
    let numerator = (n + 1) * u64::from(1_000 - alpha_milli);
    let rank = numerator.div_ceil(1_000);

    if rank == 0 || rank > n {
        return None;
    }
    ordered.get((rank - 1) as usize).copied()
}

fn marginal_quantile(
    residuals: &[ResidualPair],
    arrival_target: bool,
    alpha_milli: u16,
) -> Option<u32> {
    let scores = residuals
        .iter()
        .map(|sample| {
            if arrival_target {
                sample.arrival_abs_hours.div_ceil(ARRIVAL_SCALE_HOURS)
            } else {
                sample.handling_abs_hours.div_ceil(HANDLING_SCALE_HOURS)
            }
        })
        .collect::<Vec<_>>();
    conformal_quantile(&scores, alpha_milli)
}

fn update_aci_alpha(current_alpha_milli: u16, missed: bool) -> u16 {
    let target = i32::from(TARGET_ALPHA_MILLI);
    let observed_error = if missed { 1_000 } else { 0 };
    let delta = ACI_STEP_MILLI * (target - observed_error) / 1_000;
    (i32::from(current_alpha_milli) + delta).clamp(10, 500) as u16
}

#[derive(Clone, Debug)]
struct PolicyState {
    method: Method,
    calibration: Vec<ResidualPair>,
    history: VecDeque<ResidualPair>,
    alpha_milli: u16,
}

impl PolicyState {
    fn new(method: Method, calibration: &[ResidualPair]) -> Self {
        Self {
            method,
            calibration: calibration.to_vec(),
            history: calibration.iter().copied().collect(),
            alpha_milli: TARGET_ALPHA_MILLI,
        }
    }

    fn prediction(&self) -> Option<PredictionBand> {
        match self.method {
            Method::FixedMarginal => Some(PredictionBand::marginal(
                marginal_quantile(&self.calibration, true, TARGET_ALPHA_MILLI)?,
                marginal_quantile(&self.calibration, false, TARGET_ALPHA_MILLI)?,
            )),
            Method::FixedJoint => {
                let scores = self
                    .calibration
                    .iter()
                    .copied()
                    .map(ResidualPair::joint_score)
                    .collect::<Vec<_>>();
                Some(PredictionBand::joint(conformal_quantile(
                    &scores,
                    TARGET_ALPHA_MILLI,
                )?))
            }
            Method::RollingJoint | Method::AdaptiveJoint => {
                let scores = self
                    .history
                    .iter()
                    .copied()
                    .map(ResidualPair::joint_score)
                    .collect::<Vec<_>>();
                Some(PredictionBand::joint(conformal_quantile(
                    &scores,
                    self.alpha_milli,
                )?))
            }
        }
    }

    fn observe(&mut self, residual: ResidualPair, observed: bool, missed: bool) {
        // The online policy cannot update from an outcome that was not observed.
        // The evaluator retains hidden outcomes for latent-coverage accounting;
        // those outcomes are never passed into this method.
        if !observed {
            return;
        }

        if self.method.uses_rolling_history() {
            self.history.push_back(residual);
            while self.history.len() > ROLLING_WINDOW_SIZE {
                self.history.pop_front();
            }
        }

        if self.method == Method::AdaptiveJoint {
            self.alpha_milli = update_aci_alpha(self.alpha_milli, missed);
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct EvaluatedSample {
    window: Window,
    residual: ResidualPair,
    observed: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct WindowMetrics {
    total: usize,
    observed: usize,
    missing: usize,
    observed_joint_covered: usize,
    latent_joint_covered: usize,
    observed_arrival_covered: usize,
    observed_handling_covered: usize,
    arrival_width_sum: u64,
    handling_width_sum: u64,
    final_alpha_milli: u16,
}

impl WindowMetrics {
    fn observed_joint_coverage_pct(self) -> f64 {
        percentage(self.observed_joint_covered, self.observed)
    }

    fn latent_joint_coverage_pct(self) -> f64 {
        percentage(self.latent_joint_covered, self.total)
    }

    fn observed_arrival_coverage_pct(self) -> f64 {
        percentage(self.observed_arrival_covered, self.observed)
    }

    fn observed_handling_coverage_pct(self) -> f64 {
        percentage(self.observed_handling_covered, self.observed)
    }

    fn mean_arrival_width(self) -> f64 {
        mean(self.arrival_width_sum, self.total)
    }

    fn mean_handling_width(self) -> f64 {
        mean(self.handling_width_sum, self.total)
    }

    fn claim_status(self) -> &'static str {
        if self.missing > 0 {
            "NOT_CERTIFIED_MISSINGNESS"
        } else if self.observed_joint_coverage_pct() < 90.0 {
            "OBSERVED_UNDER_TARGET"
        } else {
            "OBSERVED_AT_OR_ABOVE_TARGET"
        }
    }
}

fn percentage(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        (numerator as f64) * 100.0 / (denominator as f64)
    }
}

fn mean(total: u64, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        total as f64 / denominator as f64
    }
}

#[derive(Clone, Debug)]
struct PolicyReport {
    method: Method,
    windows: [WindowMetrics; 3],
}

impl PolicyReport {
    fn new(method: Method) -> Self {
        Self {
            method,
            windows: [WindowMetrics::default(); 3],
        }
    }
}

fn window_index(window: Window) -> usize {
    match window {
        Window::Stationary => 0,
        Window::Drift => 1,
        Window::Burst => 2,
    }
}

#[derive(Clone, Copy, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, upper: u32) -> u32 {
        (self.next_u64() % u64::from(upper)) as u32
    }
}

fn generate_calibration(rng: &mut SplitMix64) -> Vec<ResidualPair> {
    (0..INITIAL_CALIBRATION_SIZE)
        .map(|_| ResidualPair {
            arrival_abs_hours: rng.below(36),
            handling_abs_hours: rng.below(144),
        })
        .collect()
}

fn generate_test_samples(rng: &mut SplitMix64) -> Vec<EvaluatedSample> {
    let mut output = Vec::with_capacity(360);

    for window in [Window::Stationary, Window::Drift, Window::Burst] {
        for index in 0..120 {
            let (arrival_shift, handling_shift) = match window {
                Window::Stationary => (0, 0),
                Window::Drift => (18, 60),
                Window::Burst => {
                    if index % 12 == 0 {
                        (80, 320)
                    } else {
                        (12, 40)
                    }
                }
            };

            let residual = ResidualPair {
                arrival_abs_hours: rng.below(36) + arrival_shift,
                handling_abs_hours: rng.below(144) + handling_shift,
            };
            let score = residual.joint_score();

            // Deliberately outcome-dependent missingness: large residuals are
            // much more likely to be absent from the deployed observer.
            // This is a structural MNAR stress fixture, not a fitted field model.
            let missing = if score >= 9 {
                rng.below(100) < 80
            } else {
                rng.below(100) < 2
            };

            output.push(EvaluatedSample {
                window,
                residual,
                observed: !missing,
            });
        }
    }

    output
}

fn evaluate_method(
    method: Method,
    calibration: &[ResidualPair],
    samples: &[EvaluatedSample],
) -> PolicyReport {
    let mut state = PolicyState::new(method, calibration);
    let mut report = PolicyReport::new(method);

    for sample in samples {
        let Some(prediction) = state.prediction() else {
            continue;
        };
        let (joint_covered, arrival_covered, handling_covered) =
            prediction.covers(sample.residual);
        let index = window_index(sample.window);
        let metrics = &mut report.windows[index];

        metrics.total += 1;
        metrics.arrival_width_sum +=
            u64::from(prediction.arrival_half_width_hours.saturating_mul(2));
        metrics.handling_width_sum +=
            u64::from(prediction.handling_half_width_hours.saturating_mul(2));

        if joint_covered {
            metrics.latent_joint_covered += 1;
        }

        if sample.observed {
            metrics.observed += 1;
            metrics.observed_joint_covered += usize::from(joint_covered);
            metrics.observed_arrival_covered += usize::from(arrival_covered);
            metrics.observed_handling_covered += usize::from(handling_covered);
        } else {
            metrics.missing += 1;
        }

        // The policy receives feedback only when an outcome is observed.
        state.observe(sample.residual, sample.observed, !joint_covered);
        metrics.final_alpha_milli = state.alpha_milli;
    }

    report
}

fn evaluate_suite() -> Vec<PolicyReport> {
    let mut rng = SplitMix64::new(0xDCA7_2026_0241);
    let calibration = generate_calibration(&mut rng);
    let samples = generate_test_samples(&mut rng);

    Method::ALL
        .iter()
        .copied()
        .map(|method| evaluate_method(method, &calibration, &samples))
        .collect()
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "policy,window,total_samples,observed_samples,missing_samples,observed_joint_coverage_pct,latent_joint_coverage_pct,observed_arrival_coverage_pct,observed_handling_coverage_pct,mean_arrival_interval_width_hours,mean_handling_interval_width_hours,final_alpha_milli,claim_status\n",
    );

    for report in evaluate_suite() {
        for (index, window) in [
            Window::Stationary,
            Window::Drift,
            Window::Burst,
        ]
        .iter()
        .copied()
        .enumerate()
        {
            let metrics = report.windows[index];
            output.push_str(&format!(
                "{},{},{},{},{},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{},{}\n",
                report.method.id(),
                window.id(),
                metrics.total,
                metrics.observed,
                metrics.missing,
                metrics.observed_joint_coverage_pct(),
                metrics.latent_joint_coverage_pct(),
                metrics.observed_arrival_coverage_pct(),
                metrics.observed_handling_coverage_pct(),
                metrics.mean_arrival_width(),
                metrics.mean_handling_width(),
                metrics.final_alpha_milli,
                metrics.claim_status()
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joint_score_covers_the_worst_normalised_target() {
        let residual = ResidualPair {
            arrival_abs_hours: 26,
            handling_abs_hours: 41,
        };
        assert_eq!(residual.joint_score(), 6);
    }

    #[test]
    fn lower_miscoverage_target_never_shrinks_the_conformal_quantile() {
        let scores = (1..=160).collect::<Vec<_>>();
        let q95 = conformal_quantile(&scores, 50).expect("95% quantile");
        let q90 = conformal_quantile(&scores, 100).expect("90% quantile");
        let q85 = conformal_quantile(&scores, 150).expect("85% quantile");
        assert!(q95 >= q90);
        assert!(q90 >= q85);
    }

    #[test]
    fn joint_band_covers_both_targets_or_neither() {
        let band = PredictionBand::joint(4);
        let inside = ResidualPair {
            arrival_abs_hours: 20,
            handling_abs_hours: 80,
        };
        let outside = ResidualPair {
            arrival_abs_hours: 21,
            handling_abs_hours: 80,
        };
        assert_eq!(band.covers(inside), (true, true, true));
        assert_eq!(band.covers(outside), (false, false, true));
    }

    #[test]
    fn adaptive_alpha_becomes_more_conservative_after_an_observed_miss() {
        let after_miss = update_aci_alpha(TARGET_ALPHA_MILLI, true);
        let after_success = update_aci_alpha(TARGET_ALPHA_MILLI, false);
        assert!(after_miss < TARGET_ALPHA_MILLI);
        assert!(after_success > TARGET_ALPHA_MILLI);
    }

    #[test]
    fn missing_high_residuals_can_make_observed_coverage_overstate_latent_coverage() {
        let mut metrics = WindowMetrics {
            total: 10,
            observed: 8,
            missing: 2,
            observed_joint_covered: 8,
            latent_joint_covered: 8,
            ..WindowMetrics::default()
        };
        assert_eq!(metrics.observed_joint_coverage_pct(), 100.0);
        assert_eq!(metrics.latent_joint_coverage_pct(), 80.0);
        assert_eq!(metrics.claim_status(), "NOT_CERTIFIED_MISSINGNESS");

        metrics.missing = 0;
        metrics.observed = 10;
        metrics.observed_joint_covered = 8;
        assert_eq!(metrics.claim_status(), "OBSERVED_UNDER_TARGET");
    }

    #[test]
    fn rolling_calibration_does_not_update_from_missing_outcomes() {
        let calibration = vec![
            ResidualPair {
                arrival_abs_hours: 10,
                handling_abs_hours: 20,
            };
            INITIAL_CALIBRATION_SIZE
        ];
        let mut state = PolicyState::new(Method::RollingJoint, &calibration);
        let before = state.history.len();
        state.observe(
            ResidualPair {
                arrival_abs_hours: 500,
                handling_abs_hours: 1_000,
            },
            false,
            true,
        );
        assert_eq!(state.history.len(), before);
    }

    #[test]
    fn prequential_policy_updates_history_only_after_prediction_feedback() {
        let calibration = vec![
            ResidualPair {
                arrival_abs_hours: 5,
                handling_abs_hours: 20,
            };
            INITIAL_CALIBRATION_SIZE
        ];
        let mut state = PolicyState::new(Method::RollingJoint, &calibration);
        let before = state.prediction().expect("initial prediction");
        state.observe(
            ResidualPair {
                arrival_abs_hours: 100,
                handling_abs_hours: 400,
            },
            true,
            false,
        );
        let after = state.prediction().expect("updated prediction");
        assert_eq!(state.history.len(), ROLLING_WINDOW_SIZE);
        assert!(after.arrival_half_width_hours >= before.arrival_half_width_hours);
        assert!(after.handling_half_width_hours >= before.handling_half_width_hours);
    }

    #[test]
    fn deterministic_stress_suite_contains_all_methods_and_windows() {
        let reports = evaluate_suite();
        assert_eq!(reports.len(), Method::ALL.len());
        for report in reports {
            assert_eq!(report.windows.len(), 3);
            assert_eq!(
                report.windows.iter().map(|window| window.total).sum::<usize>(),
                360
            );
        }
    }

    #[test]
    fn deterministic_csv_replay_is_byte_identical() {
        assert_eq!(render_csv(), render_csv());
    }

    #[test]
    fn csv_rows_match_the_declared_schema() {
        let output = render_csv();
        let mut lines = output.lines();
        let header = lines.next().expect("CSV header");
        let width = header.split(',').count();
        assert_eq!(width, 13);

        let rows = lines.collect::<Vec<_>>();
        assert_eq!(rows.len(), 12);
        for row in &rows {
            assert_eq!(row.split(',').count(), width, "{row}");
        }
    }

    #[test]
    fn mnar_windows_are_never_promoted_to_a_coverage_certificate() {
        let reports = evaluate_suite();
        for report in reports {
            for index in [1, 2] {
                assert!(report.windows[index].missing > 0);
                assert_eq!(
                    report.windows[index].claim_status(),
                    "NOT_CERTIFIED_MISSINGNESS"
                );
            }
        }
    }

    #[test]
    fn joint_fixed_calibration_uses_both_targets() {
        let calibration = vec![
            ResidualPair {
                arrival_abs_hours: 10,
                handling_abs_hours: 20,
            };
            INITIAL_CALIBRATION_SIZE
        ];
        let joint = PolicyState::new(Method::FixedJoint, &calibration)
            .prediction()
            .expect("joint band");
        let marginal = PolicyState::new(Method::FixedMarginal, &calibration)
            .prediction()
            .expect("marginal bands");
        assert_eq!(joint.arrival_half_width_hours, marginal.arrival_half_width_hours);
        assert_eq!(joint.handling_half_width_hours, marginal.handling_half_width_hours);
    }
}
