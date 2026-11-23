// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

const SCHEDULE_SIZE: usize = 25;
const CALIBRATION_SCHEDULES: usize = 120;
const TEST_SCHEDULES_PER_WINDOW: usize = 120;
const MIN_CALIBRATION_SCHEDULES: usize = 40;
const TARGET_ALPHA_MILLI: u16 = 100;
const BONFERRONI_ALPHA_PER_CALL_MILLI: u16 = TARGET_ALPHA_MILLI / SCHEDULE_SIZE as u16;
const ARRIVAL_SCALE_HOURS: u32 = 5;
const HANDLING_SCALE_HOURS: u32 = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VesselResidual {
    arrival_abs_hours: u32,
    handling_abs_hours: u32,
}

impl VesselResidual {
    fn joint_score(self) -> u32 {
        let arrival = self.arrival_abs_hours.div_ceil(ARRIVAL_SCALE_HOURS);
        let handling = self.handling_abs_hours.div_ceil(HANDLING_SCALE_HOURS);
        arrival.max(handling)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Schedule {
    calls: [VesselResidual; SCHEDULE_SIZE],
}

impl Schedule {
    fn maximum_joint_score(self) -> u32 {
        self.calls
            .iter()
            .map(|call| call.joint_score())
            .max()
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Window {
    Stationary,
    Drift,
    Burst,
}

impl Window {
    const ALL: [Self; 3] = [Self::Stationary, Self::Drift, Self::Burst];

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
    PerVesselJoint90,
    BonferroniSchedule90,
    ScheduleMaximumJoint90,
}

impl Method {
    const ALL: [Self; 3] = [
        Self::PerVesselJoint90,
        Self::BonferroniSchedule90,
        Self::ScheduleMaximumJoint90,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::PerVesselJoint90 => "PER_VESSEL_JOINT_90",
            Self::BonferroniSchedule90 => "BONFERRONI_SCHEDULE_90",
            Self::ScheduleMaximumJoint90 => "SCHEDULE_MAX_JOINT_90",
        }
    }

    fn calibration_unit(self) -> &'static str {
        match self {
            Self::PerVesselJoint90 | Self::BonferroniSchedule90 => "VESSEL_CALL",
            Self::ScheduleMaximumJoint90 => "WHOLE_SCHEDULE",
        }
    }

    fn target_alpha_milli(self) -> u16 {
        match self {
            Self::PerVesselJoint90 | Self::ScheduleMaximumJoint90 => TARGET_ALPHA_MILLI,
            Self::BonferroniSchedule90 => BONFERRONI_ALPHA_PER_CALL_MILLI,
        }
    }
}

fn split_conformal_quantile(scores: &[u32], alpha_milli: u16) -> Option<u32> {
    if scores.len() < MIN_CALIBRATION_SCHEDULES || alpha_milli == 0 || alpha_milli >= 1_000 {
        return None;
    }

    let mut ordered = scores.to_vec();
    ordered.sort_unstable();

    // Finite-sample order statistic k = ceil((n + 1) * (1 - alpha)).
    // If k > n, no finite distribution-free split-conformal quantile exists.
    let n = ordered.len() as u64;
    let numerator = (n + 1) * u64::from(1_000 - alpha_milli);
    let rank = numerator.div_ceil(1_000);
    if rank == 0 || rank > n {
        return None;
    }

    ordered.get((rank - 1) as usize).copied()
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

fn generate_schedule(rng: &mut SplitMix64, window: Window, schedule_index: usize) -> Schedule {
    Schedule {
        calls: std::array::from_fn(|call_index| {
            let heterogeneity = (call_index % 5) as u32;
            let mut arrival = rng.below(12) + heterogeneity * 2;
            let mut handling = rng.below(60) + heterogeneity * 8;

            match window {
                Window::Stationary => {}
                Window::Drift => {
                    arrival += 12;
                    handling += 40;
                }
                Window::Burst => {
                    arrival += 8;
                    handling += 30;
                    if schedule_index % 10 == 0 && call_index == schedule_index % SCHEDULE_SIZE {
                        arrival += 60;
                        handling += 240;
                    }
                }
            }

            VesselResidual {
                arrival_abs_hours: arrival,
                handling_abs_hours: handling,
            }
        }),
    }
}

fn generate_calibration(seed: u64) -> Vec<Schedule> {
    let mut rng = SplitMix64::new(seed);
    (0..CALIBRATION_SCHEDULES)
        .map(|index| generate_schedule(&mut rng, Window::Stationary, index))
        .collect()
}

fn generate_test_windows(seed: u64) -> [(Window, Vec<Schedule>); 3] {
    let mut rng = SplitMix64::new(seed);
    std::array::from_fn(|index| {
        let window = Window::ALL[index];
        let schedules = (0..TEST_SCHEDULES_PER_WINDOW)
            .map(|schedule_index| generate_schedule(&mut rng, window, schedule_index))
            .collect();
        (window, schedules)
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CallBands {
    quantiles: [u32; SCHEDULE_SIZE],
}

fn calibrate(method: Method, calibration: &[Schedule]) -> Option<CallBands> {
    match method {
        Method::PerVesselJoint90 | Method::BonferroniSchedule90 => {
            let alpha = method.alpha_per_call_milli();
            let mut quantiles = [0; SCHEDULE_SIZE];

            for call_index in 0..SCHEDULE_SIZE {
                let scores = calibration
                    .iter()
                    .map(|schedule| schedule.calls[call_index].joint_score())
                    .collect::<Vec<_>>();
                quantiles[call_index] = split_conformal_quantile(&scores, alpha)?;
            }

            Some(CallBands { quantiles })
        }
        Method::ScheduleMaximumJoint90 => {
            let scores = calibration
                .iter()
                .copied()
                .map(Schedule::maximum_joint_score)
                .collect::<Vec<_>>();
            let quantile = split_conformal_quantile(&scores, TARGET_ALPHA_MILLI)?;
            Some(CallBands {
                quantiles: [quantile; SCHEDULE_SIZE],
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ScheduleMetrics {
    total_schedules: usize,
    covered_schedules: usize,
    total_calls: usize,
    covered_calls: usize,
    arrival_width_sum_hours: u64,
    handling_width_sum_hours: u64,
}

impl ScheduleMetrics {
    fn whole_schedule_coverage_pct(self) -> f64 {
        percentage(self.covered_schedules, self.total_schedules)
    }

    fn per_call_coverage_pct(self) -> f64 {
        percentage(self.covered_calls, self.total_calls)
    }

    fn mean_arrival_width_hours(self) -> f64 {
        mean(self.arrival_width_sum_hours, self.total_calls)
    }

    fn mean_handling_width_hours(self) -> f64 {
        mean(self.handling_width_sum_hours, self.total_calls)
    }
}

fn percentage(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 * 100.0 / denominator as f64
    }
}

fn mean(numerator: u64, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn evaluate(bands: Option<CallBands>, schedules: &[Schedule]) -> ScheduleMetrics {
    let mut metrics = ScheduleMetrics::default();

    for schedule in schedules {
        metrics.total_schedules += 1;
        metrics.total_calls += SCHEDULE_SIZE;

        let Some(bands) = bands else {
            continue;
        };

        let mut all_calls_covered = true;
        for (call_index, call) in schedule.calls.iter().enumerate() {
            let quantile = bands.quantiles[call_index];
            let covered = call.joint_score() <= quantile;
            metrics.covered_calls += usize::from(covered);
            all_calls_covered &= covered;

            metrics.arrival_width_sum_hours += u64::from(
                quantile
                    .saturating_mul(ARRIVAL_SCALE_HOURS)
                    .saturating_mul(2),
            );
            metrics.handling_width_sum_hours += u64::from(
                quantile
                    .saturating_mul(HANDLING_SCALE_HOURS)
                    .saturating_mul(2),
            );
        }

        metrics.covered_schedules += usize::from(all_calls_covered);
    }

    metrics
}

#[derive(Clone, Debug)]
struct MethodReport {
    method: Method,
    bands: Option<CallBands>,
    windows: [ScheduleMetrics; 3],
}

impl MethodReport {
    fn status(&self) -> &'static str {
        if self.bands.is_none() {
            "NO_FINITE_SAMPLE_QUANTILE"
        } else {
            "EVALUATED_SYNTHETIC"
        }
    }

    fn quantile_min(&self) -> Option<u32> {
        self.bands
            .as_ref()
            .map(|bands| *bands.quantiles.iter().min().unwrap_or(&0))
    }

    fn quantile_max(&self) -> Option<u32> {
        self.bands
            .as_ref()
            .map(|bands| *bands.quantiles.iter().max().unwrap_or(&0))
    }
}

fn evaluate_suite() -> Vec<MethodReport> {
    let calibration = generate_calibration(0x25C_A11_2026);
    let windows = generate_test_windows(0x25C_7E57_2026);

    Method::ALL
        .iter()
        .copied()
        .map(|method| {
            let bands = calibrate(method, &calibration);
            let metrics = std::array::from_fn(|index| evaluate(bands, &windows[index].1));
            MethodReport {
                method,
                bands,
                windows: metrics,
            }
        })
        .collect()
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "method,calibration_unit,window,calibration_schedules,alpha_per_call_milli,test_schedules,covered_schedules,whole_schedule_coverage_pct,covered_calls,total_calls,per_call_coverage_pct,mean_arrival_interval_width_hours,mean_handling_interval_width_hours,quantile_min,quantile_max,status\n",
    );

    for report in evaluate_suite() {
        for (index, window) in Window::ALL.iter().copied().enumerate() {
            let metrics = report.windows[index];
            let calibrated = report.bands.is_some();
            let (
                covered_schedules,
                whole_schedule_coverage,
                covered_calls,
                per_call_coverage,
                arrival_width,
                handling_width,
                quantile_min,
                quantile_max,
            ) = if calibrated {
                (
                    metrics.covered_schedules.to_string(),
                    format!("{:.1}", metrics.whole_schedule_coverage_pct()),
                    metrics.covered_calls.to_string(),
                    format!("{:.1}", metrics.per_call_coverage_pct()),
                    format!("{:.1}", metrics.mean_arrival_width_hours()),
                    format!("{:.1}", metrics.mean_handling_width_hours()),
                    report
                        .quantile_min()
                        .map_or_else(|| "NA".to_owned(), |value| value.to_string()),
                    report
                        .quantile_max()
                        .map_or_else(|| "NA".to_owned(), |value| value.to_string()),
                )
            } else {
                (
                    "NA".to_owned(),
                    "NA".to_owned(),
                    "NA".to_owned(),
                    "NA".to_owned(),
                    "NA".to_owned(),
                    "NA".to_owned(),
                    "NA".to_owned(),
                    "NA".to_owned(),
                )
            };

            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                report.method.id(),
                report.method.calibration_unit(),
                window.id(),
                CALIBRATION_SCHEDULES,
                report.method.alpha_per_call_milli(),
                metrics.total_schedules,
                covered_schedules,
                whole_schedule_coverage,
                covered_calls,
                metrics.total_calls,
                per_call_coverage,
                arrival_width,
                handling_width,
                quantile_min,
                quantile_max,
                report.status()
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_max_score_is_at_least_every_component_call_score() {
        let schedule = Schedule {
            calls: std::array::from_fn(|index| VesselResidual {
                arrival_abs_hours: index as u32 * 3,
                handling_abs_hours: index as u32 * 9,
            }),
        };
        let max_score = schedule.maximum_joint_score();

        assert!(
            schedule
                .calls
                .iter()
                .all(|call| max_score >= call.joint_score())
        );
    }

    #[test]
    fn per_call_90_percent_does_not_imply_90_percent_whole_schedule_coverage() {
        let independent_schedule_coverage = 0.9_f64.powi(SCHEDULE_SIZE as i32);
        assert!(independent_schedule_coverage < 0.1);
    }

    #[test]
    fn bonferroni_alpha_has_no_finite_quantile_with_120_calibration_schedules() {
        let scores = (0..CALIBRATION_SCHEDULES as u32).collect::<Vec<_>>();
        assert_eq!(BONFERRONI_ALPHA_PER_CALL_MILLI, 4);
        assert!(split_conformal_quantile(&scores, BONFERRONI_ALPHA_PER_CALL_MILLI).is_none());
    }

    #[test]
    fn bonferroni_alpha_becomes_finite_at_249_calibration_schedules() {
        let scores_248 = (0..248_u32).collect::<Vec<_>>();
        assert!(split_conformal_quantile(&scores_248, BONFERRONI_ALPHA_PER_CALL_MILLI).is_none());

        let scores_249 = (0..249_u32).collect::<Vec<_>>();
        assert_eq!(
            split_conformal_quantile(&scores_249, BONFERRONI_ALPHA_PER_CALL_MILLI),
            Some(248)
        );
    }

    #[test]
    fn schedule_maximum_quantile_is_finite_with_120_calibration_schedules() {
        let calibration = generate_calibration(42);
        let bands = calibrate(Method::ScheduleMaximumJoint90, &calibration);
        assert!(bands.is_some());
    }

    #[test]
    fn schedule_maximum_quantile_is_never_smaller_than_any_per_call_quantile() {
        let calibration = generate_calibration(7);
        let per_call =
            calibrate(Method::PerVesselJoint90, &calibration).expect("per-call quantiles");
        let schedule = calibrate(Method::ScheduleMaximumJoint90, &calibration)
            .expect("schedule-level quantile");

        assert!(
            schedule
                .quantiles
                .iter()
                .zip(per_call.quantiles.iter())
                .all(|(schedule_q, call_q)| schedule_q >= call_q)
        );
    }

    #[test]
    fn schedule_level_method_evaluates_all_calls_as_one_joint_event() {
        let calibration = generate_calibration(123);
        let schedules = generate_test_windows(456);
        let bands =
            calibrate(Method::ScheduleMaximumJoint90, &calibration).expect("schedule-level bands");
        let metrics = evaluate(bands.into(), &schedules[0].1);

        assert_eq!(metrics.total_schedules, TEST_SCHEDULES_PER_WINDOW);
        assert_eq!(
            metrics.total_calls,
            TEST_SCHEDULES_PER_WINDOW * SCHEDULE_SIZE
        );
        assert!(metrics.covered_calls >= metrics.covered_schedules * SCHEDULE_SIZE);
    }

    #[test]
    fn deterministic_suite_contains_all_methods_and_windows() {
        let reports = evaluate_suite();
        assert_eq!(reports.len(), Method::ALL.len());
        for report in reports {
            assert_eq!(report.windows.len(), Window::ALL.len());
            assert_eq!(
                report
                    .windows
                    .iter()
                    .map(|window| window.total_schedules)
                    .sum::<usize>(),
                TEST_SCHEDULES_PER_WINDOW * Window::ALL.len()
            );
        }
    }

    #[test]
    fn bonferroni_method_is_not_misreported_as_a_feasible_schedule_certificate() {
        let reports = evaluate_suite();
        let bonferroni = reports
            .iter()
            .find(|report| report.method == Method::BonferroniSchedule90)
            .expect("Bonferroni baseline");
        assert_eq!(bonferroni.status(), "NO_FINITE_SAMPLE_QUANTILE");
        assert!(bonferroni.bands.is_none());
    }

    #[test]
    fn deterministic_csv_replay_is_byte_identical() {
        assert_eq!(render_csv(), render_csv());
    }

    #[test]
    fn csv_schema_has_16_columns_and_nine_method_window_rows() {
        let output = render_csv();
        let mut lines = output.lines();
        let header = lines.next().expect("CSV header");
        let width = header.split(',').count();
        assert_eq!(width, 16);

        let rows = lines.collect::<Vec<_>>();
        assert_eq!(rows.len(), 9);
        for row in &rows {
            assert_eq!(row.split(',').count(), width, "{row}");
        }
    }
}
