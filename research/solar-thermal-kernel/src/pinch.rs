// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Static sensible-heat pinch targeting for interval-integrated stream duties.
//!
//! Version 0.1 supports non-isothermal streams with constant integrated heat
//! capacity over one analysis interval. It computes thermodynamic utility
//! targets only: it does not synthesize an exchanger network, model phase
//! change, establish dynamic feasibility, or include storage/utility equipment.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetEnergyJ(f64);

impl NetEnergyJ {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        finite(value, "net_energy_j").map(Self)
    }
    pub fn value(self) -> f64 { self.0 }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HotProcessStream {
    pub supply_temperature: Kelvin,
    pub target_temperature: Kelvin,
    pub available_heat: EnergyJ,
}

impl HotProcessStream {
    pub fn new(
        supply_temperature: Kelvin,
        target_temperature: Kelvin,
        available_heat: EnergyJ,
    ) -> Result<Self, ModelError> {
        if supply_temperature.value() <= target_temperature.value() {
            return Err(ModelError::InvalidHotStreamOrder);
        }
        positive(available_heat.value(), "hot_stream_available_heat_j")?;
        Ok(Self { supply_temperature, target_temperature, available_heat })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColdProcessStream {
    pub supply_temperature: Kelvin,
    pub target_temperature: Kelvin,
    pub required_heat: EnergyJ,
}

impl ColdProcessStream {
    pub fn new(
        supply_temperature: Kelvin,
        target_temperature: Kelvin,
        required_heat: EnergyJ,
    ) -> Result<Self, ModelError> {
        if target_temperature.value() <= supply_temperature.value() {
            return Err(ModelError::InvalidColdStreamOrder);
        }
        positive(required_heat.value(), "cold_stream_required_heat_j")?;
        Ok(Self { supply_temperature, target_temperature, required_heat })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinchProblem {
    pub hot_streams: Vec<HotProcessStream>,
    pub cold_streams: Vec<ColdProcessStream>,
    pub minimum_approach: TemperatureDifferenceK,
}

impl PinchProblem {
    pub fn new(
        hot_streams: Vec<HotProcessStream>,
        cold_streams: Vec<ColdProcessStream>,
        minimum_approach: TemperatureDifferenceK,
    ) -> Result<Self, ModelError> {
        if hot_streams.is_empty() || cold_streams.is_empty() {
            return Err(ModelError::NoProcessStreams);
        }
        // Validate that shifted endpoints remain representable as absolute
        // temperatures. This rejects impossible ΔTmin shifts near absolute zero.
        let half = minimum_approach.value() / 2.0;
        for stream in &hot_streams {
            if stream.supply_temperature.value() <= stream.target_temperature.value() {
                return Err(ModelError::InvalidHotStreamOrder);
            }
            positive(stream.available_heat.value(), "hot_stream_available_heat_j")?;
            Kelvin::new(stream.supply_temperature.value() - half)?;
            Kelvin::new(stream.target_temperature.value() - half)?;
        }
        for stream in &cold_streams {
            if stream.target_temperature.value() <= stream.supply_temperature.value() {
                return Err(ModelError::InvalidColdStreamOrder);
            }
            positive(stream.required_heat.value(), "cold_stream_required_heat_j")?;
            Kelvin::new(stream.supply_temperature.value() + half)?;
            Kelvin::new(stream.target_temperature.value() + half)?;
        }
        Ok(Self { hot_streams, cold_streams, minimum_approach })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinchInterval {
    pub shifted_high_temperature: Kelvin,
    pub shifted_low_temperature: Kelvin,
    pub net_hot_surplus: NetEnergyJ,
    /// Cumulative process surplus before adding minimum hot utility.
    pub unconstrained_cascade_after: NetEnergyJ,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinchPoint {
    pub shifted_temperature: Kelvin,
    pub hot_side_temperature: Kelvin,
    pub cold_side_temperature: Kelvin,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinchTargets {
    pub minimum_hot_utility: EnergyJ,
    pub minimum_cold_utility: EnergyJ,
    pub maximum_process_heat_recovery: EnergyJ,
    pub intervals: Vec<PinchInterval>,
    pub pinch_points: Vec<PinchPoint>,
}

#[derive(Clone, Copy, Debug)]
struct ShiftedStream {
    high: f64,
    low: f64,
    integrated_heat_capacity_j_per_k: f64,
}

struct PreparedPinchStreams {
    hot: Vec<ShiftedStream>,
    cold: Vec<ShiftedStream>,
    endpoints: Vec<f64>,
}

fn shifted_streams(problem: &PinchProblem) -> Result<PreparedPinchStreams, ModelError> {
    let half = problem.minimum_approach.value() / 2.0;
    let mut hot = Vec::with_capacity(problem.hot_streams.len());
    let mut cold = Vec::with_capacity(problem.cold_streams.len());
    let mut endpoints = Vec::with_capacity(2 * (problem.hot_streams.len() + problem.cold_streams.len()));

    for stream in &problem.hot_streams {
        let supply = stream.supply_temperature.value() - half;
        let target = stream.target_temperature.value() - half;
        if supply <= target {
            return Err(ModelError::InvalidHotStreamOrder);
        }
        let cp_integrated = stream.available_heat.value()
            / (stream.supply_temperature.value() - stream.target_temperature.value());
        hot.push(ShiftedStream {
            high: supply,
            low: target,
            integrated_heat_capacity_j_per_k: cp_integrated,
        });
        endpoints.push(supply);
        endpoints.push(target);
    }

    for stream in &problem.cold_streams {
        let supply = stream.supply_temperature.value() + half;
        let target = stream.target_temperature.value() + half;
        if target <= supply {
            return Err(ModelError::InvalidColdStreamOrder);
        }
        let cp_integrated = stream.required_heat.value()
            / (stream.target_temperature.value() - stream.supply_temperature.value());
        cold.push(ShiftedStream {
            high: target,
            low: supply,
            integrated_heat_capacity_j_per_k: cp_integrated,
        });
        endpoints.push(target);
        endpoints.push(supply);
    }

    endpoints.sort_by(f64::total_cmp);
    endpoints.dedup_by(|a, b| close(*a, *b));
    endpoints.reverse();
    if endpoints.len() < 2 {
        return Err(ModelError::NoProcessStreams);
    }
    Ok(PreparedPinchStreams { hot, cold, endpoints })
}

fn build_intervals(
    hot: &[ShiftedStream],
    cold: &[ShiftedStream],
    descending_endpoints: &[f64],
) -> Result<(Vec<PinchInterval>, f64, f64), ModelError> {
    let mut intervals = Vec::with_capacity(descending_endpoints.len().saturating_sub(1));
    let mut cumulative = 0.0_f64;
    let mut minimum_cumulative = 0.0_f64;

    for pair in descending_endpoints.windows(2) {
        let upper = pair[0];
        let lower = pair[1];
        let span = upper - lower;
        if span <= 0.0 || close(upper, lower) {
            continue;
        }
        let hot_capacity: f64 = hot.iter()
            .filter(|s| s.high >= upper && s.low <= lower)
            .map(|s| s.integrated_heat_capacity_j_per_k)
            .sum();
        let cold_capacity: f64 = cold.iter()
            .filter(|s| s.high >= upper && s.low <= lower)
            .map(|s| s.integrated_heat_capacity_j_per_k)
            .sum();
        let net = (hot_capacity - cold_capacity) * span;
        cumulative += net;
        minimum_cumulative = minimum_cumulative.min(cumulative);
        intervals.push(PinchInterval {
            shifted_high_temperature: Kelvin::new(upper)?,
            shifted_low_temperature: Kelvin::new(lower)?,
            net_hot_surplus: NetEnergyJ::new(net)?,
            unconstrained_cascade_after: NetEnergyJ::new(cumulative)?,
        });
    }

    if intervals.is_empty() {
        return Err(ModelError::NoProcessStreams);
    }
    Ok((intervals, cumulative, minimum_cumulative))
}

fn target_from_problem(problem: &PinchProblem) -> Result<PinchTargets, ModelError> {
    let prepared = shifted_streams(problem)?;
    let (intervals, total_net, minimum_cumulative) =
        build_intervals(&prepared.hot, &prepared.cold, &prepared.endpoints)?;
    let hot_utility = (-minimum_cumulative).max(0.0);
    let cold_utility = (hot_utility + total_net).max(0.0);

    let total_hot: f64 = problem.hot_streams.iter().map(|s| s.available_heat.value()).sum();
    let total_cold: f64 = problem.cold_streams.iter().map(|s| s.required_heat.value()).sum();
    let recovered = (total_hot - cold_utility)
        .min(total_cold - hot_utility)
        .max(0.0);

    let half = problem.minimum_approach.value() / 2.0;
    let mut pinch_points = Vec::new();
    if hot_utility > 0.0 {
        for interval in &intervals {
            if close(interval.unconstrained_cascade_after.value() + hot_utility, 0.0) {
                let shifted = interval.shifted_low_temperature.value();
                pinch_points.push(PinchPoint {
                    shifted_temperature: interval.shifted_low_temperature,
                    hot_side_temperature: Kelvin::new(shifted + half)?,
                    cold_side_temperature: Kelvin::new(shifted - half)?,
                });
            }
        }
    }

    Ok(PinchTargets {
        minimum_hot_utility: EnergyJ::new(hot_utility)?,
        minimum_cold_utility: EnergyJ::new(cold_utility)?,
        maximum_process_heat_recovery: EnergyJ::new(recovered)?,
        intervals,
        pinch_points,
    })
}

/// Calculate minimum hot/cold utilities and the maximum process heat-recovery target.
///
/// This is static pinch targeting for constant integrated heat-capacity streams.
/// Stream heat duties are interval energies (J), not power (W). Temperatures and
/// minimum approach use kelvin. These are thermodynamic targets, not an exchanger
/// design or an operating schedule.
pub fn target_pinch_utilities(problem: &PinchProblem) -> Result<PinchTargets, ModelError> {
    target_from_problem(problem)
}

/// Verify a claim by independently rebuilding the temperature intervals and
/// heat cascade from the original stream definitions.
pub fn verify_pinch_targets(
    problem: &PinchProblem,
    claimed: &PinchTargets,
) -> Result<(), ModelError> {
    let prepared = shifted_streams(problem)?;
    let mut recomputed_cascade = 0.0_f64;
    let mut minimum_cascade = 0.0_f64;
    let mut recomputed_intervals = Vec::with_capacity(prepared.endpoints.len().saturating_sub(1));

    // Deliberately re-express the interval loop here rather than calling the
    // production target calculator or build_intervals().
    for bounds in prepared.endpoints.windows(2) {
        let top = bounds[0];
        let bottom = bounds[1];
        if top - bottom <= 0.0 || close(top, bottom) {
            continue;
        }
        let mut hot_sum = 0.0;
        for stream in &prepared.hot {
            if stream.high >= top && stream.low <= bottom {
                hot_sum += stream.integrated_heat_capacity_j_per_k;
            }
        }
        let mut cold_sum = 0.0;
        for stream in &prepared.cold {
            if stream.high >= top && stream.low <= bottom {
                cold_sum += stream.integrated_heat_capacity_j_per_k;
            }
        }
        let surplus = (hot_sum - cold_sum) * (top - bottom);
        recomputed_cascade += surplus;
        minimum_cascade = minimum_cascade.min(recomputed_cascade);
        recomputed_intervals.push((top, bottom, surplus, recomputed_cascade));
    }

    let expected_hot = (-minimum_cascade).max(0.0);
    let expected_cold = (expected_hot + recomputed_cascade).max(0.0);
    let total_hot: f64 = problem.hot_streams.iter().map(|s| s.available_heat.value()).sum();
    let total_cold: f64 = problem.cold_streams.iter().map(|s| s.required_heat.value()).sum();
    let expected_recovery = (total_hot - expected_cold)
        .min(total_cold - expected_hot)
        .max(0.0);

    check("pinch.minimum_hot_utility", claimed.minimum_hot_utility.value(), expected_hot)?;
    check("pinch.minimum_cold_utility", claimed.minimum_cold_utility.value(), expected_cold)?;
    check("pinch.maximum_heat_recovery", claimed.maximum_process_heat_recovery.value(), expected_recovery)?;
    if claimed.intervals.len() != recomputed_intervals.len() {
        return Err(ModelError::ReportMismatch("pinch.interval_count"));
    }
    for (interval, expected) in claimed.intervals.iter().zip(recomputed_intervals.iter().copied()) {
        check("pinch.interval_high", interval.shifted_high_temperature.value(), expected.0)?;
        check("pinch.interval_low", interval.shifted_low_temperature.value(), expected.1)?;
        check("pinch.interval_net", interval.net_hot_surplus.value(), expected.2)?;
        check("pinch.interval_cascade", interval.unconstrained_cascade_after.value(), expected.3)?;
    }

    let half = problem.minimum_approach.value() / 2.0;
    let expected_pinches: Vec<f64> = if expected_hot > 0.0 {
        recomputed_intervals.iter()
            .filter(|item| close(item.3 + expected_hot, 0.0))
            .map(|item| item.1)
            .collect()
    } else {
        Vec::new()
    };
    if claimed.pinch_points.len() != expected_pinches.len() {
        return Err(ModelError::ReportMismatch("pinch.point_count"));
    }
    for (point, shifted) in claimed.pinch_points.iter().zip(expected_pinches) {
        check("pinch.shifted_temperature", point.shifted_temperature.value(), shifted)?;
        check("pinch.hot_side_temperature", point.hot_side_temperature.value(), shifted + half)?;
        check("pinch.cold_side_temperature", point.cold_side_temperature.value(), shifted - half)?;
    }

    // Independent whole-system first-law check.
    check(
        "pinch.global_energy_balance",
        total_hot + claimed.minimum_hot_utility.value(),
        total_cold + claimed.minimum_cold_utility.value(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(value: f64) -> Kelvin { Kelvin::new(value).unwrap() }
    fn e(value: f64) -> EnergyJ { EnergyJ::new(value).unwrap() }
    fn hot(supply: f64, target: f64, heat: f64) -> HotProcessStream {
        HotProcessStream::new(k(supply), k(target), e(heat)).unwrap()
    }
    fn cold(supply: f64, target: f64, heat: f64) -> ColdProcessStream {
        ColdProcessStream::new(k(supply), k(target), e(heat)).unwrap()
    }
    fn canonical_problem() -> PinchProblem {
        PinchProblem::new(
            vec![hot(423.15, 323.15, 100_000.0)],
            vec![cold(393.15, 493.15, 120_000.0)],
            TemperatureDifferenceK::new(20.0).unwrap(),
        ).unwrap()
    }

    #[test]
    fn canonical_pinch_problem_matches_hand_calculated_utility_targets() {
        let result = target_pinch_utilities(&canonical_problem()).unwrap();
        assert!((result.minimum_hot_utility.value() - 110_000.0).abs() < 1.0e-7);
        assert!((result.minimum_cold_utility.value() - 90_000.0).abs() < 1.0e-7);
        assert!((result.maximum_process_heat_recovery.value() - 10_000.0).abs() < 1.0e-7);
        assert_eq!(result.pinch_points.len(), 1);
        assert!((result.pinch_points[0].shifted_temperature.value() - 403.15).abs() < 1.0e-8);
        assert!((result.pinch_points[0].hot_side_temperature.value() - 413.15).abs() < 1.0e-8);
        assert!((result.pinch_points[0].cold_side_temperature.value() - 393.15).abs() < 1.0e-8);
        verify_pinch_targets(&canonical_problem(), &result).unwrap();
    }

    #[test]
    fn verifier_rejects_tampered_utility_target() {
        let problem = canonical_problem();
        let result = target_pinch_utilities(&problem).unwrap();
        let tampered = PinchTargets {
            minimum_hot_utility: e(109_000.0),
            ..result
        };
        assert!(matches!(
            verify_pinch_targets(&problem, &tampered),
            Err(ModelError::ReportMismatch(_))
        ));
    }

    #[test]
    fn rejects_hot_stream_with_reversed_temperature_order() {
        assert_eq!(
            HotProcessStream::new(k(300.0), k(400.0), e(1_000.0)),
            Err(ModelError::InvalidHotStreamOrder)
        );
    }

    #[test]
    fn rejects_cold_stream_with_reversed_temperature_order() {
        assert_eq!(
            ColdProcessStream::new(k(400.0), k(300.0), e(1_000.0)),
            Err(ModelError::InvalidColdStreamOrder)
        );
    }

    #[test]
    fn requires_hot_and_cold_streams() {
        assert_eq!(
            PinchProblem::new(vec![], vec![cold(300.0, 350.0, 1_000.0)],
                TemperatureDifferenceK::new(10.0).unwrap()),
            Err(ModelError::NoProcessStreams)
        );
    }

    #[test]
    fn rejects_zero_stream_duty() {
        assert_eq!(
            HotProcessStream::new(k(400.0), k(300.0), e(0.0)),
            Err(ModelError::MustBePositive("hot_stream_available_heat_j"))
        );
    }

    #[test]
    fn zero_approach_preserves_energy_balance_for_identical_stream_ranges() {
        let problem = PinchProblem::new(
            vec![hot(400.0, 300.0, 100_000.0)],
            vec![cold(300.0, 400.0, 100_000.0)],
            TemperatureDifferenceK::new(0.0).unwrap(),
        ).unwrap();
        let result = target_pinch_utilities(&problem).unwrap();
        assert!((result.minimum_hot_utility.value()).abs() < 1.0e-7);
        assert!((result.minimum_cold_utility.value()).abs() < 1.0e-7);
        assert!((result.maximum_process_heat_recovery.value() - 100_000.0).abs() < 1.0e-7);
        verify_pinch_targets(&problem, &result).unwrap();
    }
}
