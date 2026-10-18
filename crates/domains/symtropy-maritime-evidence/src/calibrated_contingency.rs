// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Policy adapter from a calibrated ETA interval to a bounded service-window choice.
//!
//! This joins the split-conformal estimate and the synthetic decision cost model,
//! but it still does not have access to a real port schedule or schedule authority.

use super::arrival_calibration::{
    CalibratedArrivalEnvelope, CalibrationError, PredictionInterval,
};
use super::evidence_action::{DecisionWindow, score_window};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalibratedAbstentionReason {
    NoCandidateWindows,
    UnboundedCalibrationInterval,
    NotBetterThanAbstentionCost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibratedContingencyDecision {
    pub chosen_window_id: Option<String>,
    pub forecast_interval: Option<PredictionInterval>,
    pub worst_case_cost_minutes: Option<u64>,
    pub abstention_cost_minutes: u64,
    pub windows_considered: usize,
    pub abstention_reason: Option<CalibratedAbstentionReason>,
}

/// Select the candidate window minimizing worst-case cost across the complete
/// calibrated interval. Because score_window is monotone on either side of a
/// window's start, the interval endpoints give its worst-case cost.
///
/// Abstain if the calibration radius is infinite, no windows are supplied, or
/// the best finite interval cost is not strictly lower than the declared
/// abstention cost. The output is a research decision only, not a schedule write.
pub fn choose_calibrated_window(
    calibration: &CalibratedArrivalEnvelope,
    predicted_arrival_utc_nanos: i128,
    windows: &[DecisionWindow],
    missed_window_penalty_minutes: u64,
    abstention_cost_minutes: u64,
) -> Result<CalibratedContingencyDecision, CalibrationError> {
    if windows.is_empty() {
        return Ok(CalibratedContingencyDecision {
            chosen_window_id: None,
            forecast_interval: None,
            worst_case_cost_minutes: None,
            abstention_cost_minutes,
            windows_considered: 0,
            abstention_reason: Some(CalibratedAbstentionReason::NoCandidateWindows),
        });
    }

    let Some(interval) = calibration.interval_for(predicted_arrival_utc_nanos)? else {
        return Ok(CalibratedContingencyDecision {
            chosen_window_id: None,
            forecast_interval: None,
            worst_case_cost_minutes: None,
            abstention_cost_minutes,
            windows_considered: windows.len(),
            abstention_reason: Some(CalibratedAbstentionReason::UnboundedCalibrationInterval),
        });
    };

    let best = windows
        .iter()
        .map(|window| {
            let lower_cost = score_window(
                interval.lower_utc_nanos,
                window,
                missed_window_penalty_minutes,
            );
            let upper_cost = score_window(
                interval.upper_utc_nanos,
                window,
                missed_window_penalty_minutes,
            );
            (
                lower_cost.max(upper_cost),
                window.starts_at_utc_nanos,
                window.id.clone(),
            )
        })
        .min();

    let Some((worst_case_cost_minutes, _, chosen_window_id)) = best else {
        return Ok(CalibratedContingencyDecision {
            chosen_window_id: None,
            forecast_interval: Some(interval),
            worst_case_cost_minutes: None,
            abstention_cost_minutes,
            windows_considered: 0,
            abstention_reason: Some(CalibratedAbstentionReason::NoCandidateWindows),
        });
    };

    if worst_case_cost_minutes >= abstention_cost_minutes {
        return Ok(CalibratedContingencyDecision {
            chosen_window_id: None,
            forecast_interval: Some(interval),
            worst_case_cost_minutes: Some(worst_case_cost_minutes),
            abstention_cost_minutes,
            windows_considered: windows.len(),
            abstention_reason: Some(CalibratedAbstentionReason::NotBetterThanAbstentionCost),
        });
    }

    Ok(CalibratedContingencyDecision {
        chosen_window_id: Some(chosen_window_id),
        forecast_interval: Some(interval),
        worst_case_cost_minutes: Some(worst_case_cost_minutes),
        abstention_cost_minutes,
        windows_considered: windows.len(),
        abstention_reason: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::arrival_calibration::{
        ArrivalResidualSample, NANOS_PER_SECOND, fit_split_conformal,
    };

    fn utc(value: &str) -> i128 {
        super::super::parse_rfc3339_timestamp(value)
            .expect("frozen synthetic timestamp parses")
            .utc_unix_nanos()
    }

    fn sample(index: usize, error_seconds: i64) -> ArrivalResidualSample {
        let base = 10_000 * NANOS_PER_SECOND;
        let predicted = base + index as i128 * 10 * NANOS_PER_SECOND;
        let actual = predicted + i128::from(error_seconds) * NANOS_PER_SECOND;
        ArrivalResidualSample {
            forecast_id: format!("CALIBRATION-{index:03}"),
            forecast_issued_at_utc_nanos: base - 3_600 * NANOS_PER_SECOND,
            predicted_arrival_utc_nanos: predicted,
            actual_arrival_utc_nanos: actual,
            actual_observed_at_utc_nanos: actual + NANOS_PER_SECOND,
        }
    }

    fn calibration(count: usize) -> CalibratedArrivalEnvelope {
        let samples = (1..=count)
            .map(|index| sample(index, i64::try_from(index).unwrap()))
            .collect::<Vec<_>>();
        super::super::arrival_calibration::fit_split_conformal(
            &samples,
            20_000 * NANOS_PER_SECOND,
            1_000,
        )
        .unwrap()
    }

    fn windows() -> Vec<DecisionWindow> {
        vec![
            DecisionWindow {
                id: "WINDOW-12".to_owned(),
                starts_at_utc_nanos: utc("2026-10-18T12:00:00Z"),
                late_grace_minutes: 15,
            },
            DecisionWindow {
                id: "WINDOW-13".to_owned(),
                starts_at_utc_nanos: utc("2026-10-18T13:00:00Z"),
                late_grace_minutes: 15,
            },
        ]
    }

    #[test]
    fn calibrated_interval_changes_point_decision_to_lower_worst_case_window() {
        let fitted = calibration(9);
        let predicted = utc("2026-10-18T12:55:00Z");
        let decision =
            choose_calibrated_window(&fitted, predicted, &windows(), 100, 130).unwrap();
        assert_eq!(decision.chosen_window_id.as_deref(), Some("WINDOW-13"));
        assert_eq!(decision.worst_case_cost_minutes, Some(5));
        assert_eq!(decision.abstention_reason, None);
        assert_eq!(
            decision.forecast_interval.unwrap().lower_utc_nanos,
            predicted - 9 * NANOS_PER_SECOND
        );
    }

    #[test]
    fn unbounded_calibration_interval_forces_explicit_abstention() {
        let fitted = calibration(8);
        let decision = choose_calibrated_window(
            &fitted,
            utc("2026-10-18T12:55:00Z"),
            &windows(),
            100,
            130,
        )
        .unwrap();
        assert_eq!(decision.chosen_window_id, None);
        assert_eq!(
            decision.abstention_reason,
            Some(CalibratedAbstentionReason::UnboundedCalibrationInterval)
        );
    }

    #[test]
    fn abstains_when_worst_case_cost_is_not_better_than_abstention() {
        let fitted = calibration(9);
        let decision = choose_calibrated_window(
            &fitted,
            utc("2026-10-18T12:55:00Z"),
            &windows(),
            100,
            5,
        )
        .unwrap();
        assert_eq!(decision.chosen_window_id, None);
        assert_eq!(decision.worst_case_cost_minutes, Some(5));
        assert_eq!(
            decision.abstention_reason,
            Some(CalibratedAbstentionReason::NotBetterThanAbstentionCost)
        );
    }

    #[test]
    fn no_windows_is_reported_separately_from_calibration_failure() {
        let fitted = calibration(9);
        let decision = choose_calibrated_window(
            &fitted,
            utc("2026-10-18T12:55:00Z"),
            &[],
            100,
            130,
        )
        .unwrap();
        assert_eq!(decision.windows_considered, 0);
        assert_eq!(
            decision.abstention_reason,
            Some(CalibratedAbstentionReason::NoCandidateWindows)
        );
    }

    #[test]
    fn window_permutations_keep_choice_identical() {
        let fitted = calibration(9);
        let mut reversed = windows();
        reversed.reverse();
        let predicted = utc("2026-10-18T12:55:00Z");
        let first = choose_calibrated_window(&fitted, predicted, &windows(), 100, 130).unwrap();
        let second = choose_calibrated_window(&fitted, predicted, &reversed, 100, 130).unwrap();
        assert_eq!(first, second);
    }

    fn oracle_score_window(
        arrival_utc_nanos: i128,
        window: &DecisionWindow,
        missed_penalty_minutes: u64,
    ) -> u64 {
        let absolute_delta_nanos = arrival_utc_nanos.abs_diff(window.starts_at_utc_nanos);
        let whole_minutes = absolute_delta_nanos / 60_000_000_000;
        let remainder = absolute_delta_nanos % 60_000_000_000;
        let rounded_minutes = if remainder == 0 {
            whole_minutes
        } else {
            whole_minutes.saturating_add(1)
        };
        let cost = u64::try_from(rounded_minutes).unwrap_or(u64::MAX);
        if arrival_utc_nanos <= window.starts_at_utc_nanos {
            cost
        } else if cost <= u64::from(window.late_grace_minutes) {
            cost
        } else {
            missed_penalty_minutes.saturating_add(cost)
        }
    }

    fn calibration_with_radius_minutes(radius_minutes: u32) -> CalibratedArrivalEnvelope {
        let samples = (1_usize..=20)
            .map(|index| {
                let error = match index {
                    1..=18 => 0,
                    19 => i64::from(radius_minutes) * 60,
                    _ => i64::from(radius_minutes) * 60 + 1,
                };
                sample(index, error)
            })
            .collect::<Vec<_>>();
        super::super::arrival_calibration::fit_split_conformal(
            &samples,
            20_000 * NANOS_PER_SECOND,
            1_000,
        )
        .unwrap()
    }

    fn windows_with_grace(grace: u32) -> Vec<DecisionWindow> {
        [12 * 60, 13 * 60, 14 * 60]
            .into_iter()
            .map(|minute| DecisionWindow {
                id: format!("WINDOW-{minute}"),
                starts_at_utc_nanos: utc(&format!(
                    "2026-10-18T{:02}:{:02}:00Z",
                    minute / 60,
                    minute % 60
                )),
                late_grace_minutes: grace,
            })
            .collect()
    }

    #[test]
    fn interval_choice_and_abstention_match_independent_exhaustive_oracle() {
        let radii = [0_u32, 1, 5, 15, 30, 60]
            .into_iter()
            .map(calibration_with_radius_minutes)
            .collect::<Vec<_>>();
        let mut configurations = 0usize;

        for forecast_minute in (11 * 60..=15 * 60).step_by(15) {
            let predicted = utc(&format!(
                "2026-10-18T{:02}:{:02}:00Z",
                forecast_minute / 60,
                forecast_minute % 60
            ));
            for calibration in &radii {
                let interval = calibration.interval_for(predicted).unwrap().unwrap();
                for grace in [0_u32, 15, 30] {
                    let windows = windows_with_grace(grace);
                    for missed_penalty in [30_u64, 100, 200] {
                        let scored = windows
                            .iter()
                            .map(|window| {
                                let lower = oracle_score_window(
                                    interval.lower_utc_nanos,
                                    window,
                                    missed_penalty,
                                );
                                let upper = oracle_score_window(
                                    interval.upper_utc_nanos,
                                    window,
                                    missed_penalty,
                                );
                                (lower.max(upper), window.starts_at_utc_nanos, window.id.clone())
                            })
                            .min();
                        for abstention_cost in [0_u64, 5, 15, 30, 60, 130, 300] {
                            let expected = scored.as_ref().and_then(|(cost, _, id)| {
                                (*cost < abstention_cost).then(|| id.clone())
                            });
                            let actual = choose_calibrated_window(
                                calibration,
                                predicted,
                                &windows,
                                missed_penalty,
                                abstention_cost,
                            )
                            .unwrap()
                            .chosen_window_id;
                            assert_eq!(
                                actual,
                                expected,
                                "oracle mismatch {forecast_minute}/{grace}/{missed_penalty}/{abstention_cost}"
                            );
                            configurations += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(configurations, 6_426);
    }

    #[test]
    fn interval_overflow_fails_closed_as_an_error() {
        let fitted = calibration(9);
        assert_eq!(
            choose_calibrated_window(&fitted, i128::MAX, &windows(), 100, 130),
            Err(CalibrationError::IntervalOverflow)
        );
    }
}
