// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Dimensional-leakage gameplay model with fail-closed numerical boundaries.
//!
//! This module models an explicit operational boundary through which budget-like
//! energy may appear or disappear. The mechanism is speculative/gameplay physics,
//! not evidence that real Randall-Sundrum leakage has been measured in Symtropy.
//! Callers that need validation evidence should use the checked APIs.
//!
//! The important accounting rule is structural: source/sink direction is explicit,
//! all geometry/rates must be representable, aggregate field effects must remain
//! finite, and a tick either commits all cumulative counters or none of them.

use nalgebra::SVector;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionalLeakageError {
    NonFinitePosition,
    NonFiniteWDepth,
    NonFiniteRate,
    InvalidRadius,
    InvalidAccumulatedState,
    NonFiniteDistance,
    NonFiniteEffect,
    NonFiniteAggregate,
    InvalidThreshold,
}

/// A point at which the gameplay model exchanges operational energy with an
/// explicit extra-dimensional boundary.
#[derive(Debug, Clone)]
pub struct LeakagePoint<const D: usize> {
    pub position: SVector<f64, D>,
    pub w_depth: f64,
    /// Positive = source into the modeled domain; negative = sink out.
    pub flow_rate: f64,
    pub radius: f64,
    pub active: bool,
    /// Cumulative absolute throughput associated with this point.
    pub total_transferred: f64,
}

impl<const D: usize> LeakagePoint<D> {
    pub fn sink(position: SVector<f64, D>, w_depth: f64, rate: f64, radius: f64) -> Self {
        Self {
            position,
            w_depth,
            flow_rate: -rate.abs(),
            radius,
            active: true,
            total_transferred: 0.0,
        }
    }

    pub fn source(position: SVector<f64, D>, w_depth: f64, rate: f64, radius: f64) -> Self {
        Self {
            position,
            w_depth,
            flow_rate: rate.abs(),
            radius,
            active: true,
            total_transferred: 0.0,
        }
    }

    pub fn sink_checked(
        position: SVector<f64, D>,
        w_depth: f64,
        rate: f64,
        radius: f64,
    ) -> Result<Self, DimensionalLeakageError> {
        let point = Self::sink(position, w_depth, rate, radius);
        point.validate()?;
        Ok(point)
    }

    pub fn source_checked(
        position: SVector<f64, D>,
        w_depth: f64,
        rate: f64,
        radius: f64,
    ) -> Result<Self, DimensionalLeakageError> {
        let point = Self::source(position, w_depth, rate, radius);
        point.validate()?;
        Ok(point)
    }

    pub fn validate(&self) -> Result<(), DimensionalLeakageError> {
        if !self.position.iter().all(|value| value.is_finite()) {
            return Err(DimensionalLeakageError::NonFinitePosition);
        }
        if !self.w_depth.is_finite() {
            return Err(DimensionalLeakageError::NonFiniteWDepth);
        }
        if !self.flow_rate.is_finite() {
            return Err(DimensionalLeakageError::NonFiniteRate);
        }
        if !self.radius.is_finite() || self.radius <= 0.0 {
            return Err(DimensionalLeakageError::InvalidRadius);
        }
        if !self.total_transferred.is_finite() || self.total_transferred < 0.0 {
            return Err(DimensionalLeakageError::InvalidAccumulatedState);
        }
        Ok(())
    }

    pub fn distance_checked(
        &self,
        pos: &SVector<f64, D>,
    ) -> Result<f64, DimensionalLeakageError> {
        self.validate()?;
        if !pos.iter().all(|value| value.is_finite()) {
            return Err(DimensionalLeakageError::NonFinitePosition);
        }
        let distance = (pos - self.position).norm();
        if !distance.is_finite() {
            return Err(DimensionalLeakageError::NonFiniteDistance);
        }
        Ok(distance)
    }

    /// Compatibility query. Invalid geometry fails closed to infinity.
    pub fn distance(&self, pos: &SVector<f64, D>) -> f64 {
        self.distance_checked(pos).unwrap_or(f64::INFINITY)
    }

    pub fn effect_at_checked(
        &self,
        pos: &SVector<f64, D>,
    ) -> Result<f64, DimensionalLeakageError> {
        self.validate()?;
        if !self.active {
            return Ok(0.0);
        }
        let distance = self.distance_checked(pos)?;
        if distance >= self.radius {
            return Ok(0.0);
        }
        let r = distance.max(1.0);
        let denominator = r * r;
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err(DimensionalLeakageError::NonFiniteEffect);
        }
        let effect = self.flow_rate / denominator;
        if !effect.is_finite() {
            return Err(DimensionalLeakageError::NonFiniteEffect);
        }
        Ok(effect)
    }

    /// Compatibility query. Invalid/unrepresentable state produces no benefit or drain.
    pub fn effect_at(&self, pos: &SVector<f64, D>) -> f64 {
        self.effect_at_checked(pos).unwrap_or(0.0)
    }
}

#[derive(Debug, Clone)]
pub struct DimensionalLeakage<const D: usize> {
    pub points: Vec<LeakagePoint<D>>,
    pub total_leaked_out: f64,
    pub total_leaked_in: f64,
    /// Signed historical difference `out - in`; diagnostic only.
    pub apparent_violation: f64,
    pub enabled: bool,
}

impl<const D: usize> DimensionalLeakage<D> {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            total_leaked_out: 0.0,
            total_leaked_in: 0.0,
            apparent_violation: 0.0,
            enabled: false,
        }
    }

    pub fn validate(&self) -> Result<(), DimensionalLeakageError> {
        if !self.total_leaked_out.is_finite()
            || self.total_leaked_out < 0.0
            || !self.total_leaked_in.is_finite()
            || self.total_leaked_in < 0.0
            || !self.apparent_violation.is_finite()
        {
            return Err(DimensionalLeakageError::InvalidAccumulatedState);
        }
        for point in &self.points {
            point.validate()?;
        }
        Ok(())
    }

    pub fn add_point_checked(
        &mut self,
        point: LeakagePoint<D>,
    ) -> Result<(), DimensionalLeakageError> {
        point.validate()?;
        self.points.push(point);
        Ok(())
    }

    /// Compatibility insertion. Invalid points are not admitted.
    pub fn add_point(&mut self, point: LeakagePoint<D>) {
        let _ = self.add_point_checked(point);
    }

    pub fn create_wormhole_checked(
        &mut self,
        entry: SVector<f64, D>,
        exit: SVector<f64, D>,
        rate: f64,
        radius: f64,
    ) -> Result<(), DimensionalLeakageError> {
        let sink = LeakagePoint::sink_checked(entry, 1.0, rate, radius)?;
        let source = LeakagePoint::source_checked(exit, 1.0, rate, radius)?;
        self.validate()?;
        self.points.push(sink);
        self.points.push(source);
        self.enabled = true;
        Ok(())
    }

    /// Compatibility constructor. Invalid input leaves the system unchanged.
    pub fn create_wormhole(
        &mut self,
        entry: SVector<f64, D>,
        exit: SVector<f64, D>,
        rate: f64,
        radius: f64,
    ) {
        let _ = self.create_wormhole_checked(entry, exit, rate, radius);
    }

    pub fn total_effect_at_checked(
        &self,
        pos: &SVector<f64, D>,
    ) -> Result<f64, DimensionalLeakageError> {
        self.validate()?;
        if !self.enabled {
            return Ok(0.0);
        }
        if !pos.iter().all(|value| value.is_finite()) {
            return Err(DimensionalLeakageError::NonFinitePosition);
        }
        let mut total = 0.0;
        for point in &self.points {
            let effect = point.effect_at_checked(pos)?;
            total += effect;
            if !total.is_finite() {
                return Err(DimensionalLeakageError::NonFiniteAggregate);
            }
        }
        Ok(total)
    }

    /// Compatibility query. Invalid aggregate state fails closed to zero effect.
    pub fn total_effect_at(&self, pos: &SVector<f64, D>) -> f64 {
        self.total_effect_at_checked(pos).unwrap_or(0.0)
    }

    /// Transactionally advance cumulative leakage telemetry by one tick.
    ///
    /// Every next point/global value is staged before mutation. If any finite
    /// input produces an unrepresentable aggregate, no point or system total is
    /// changed.
    pub fn tick_checked(&mut self) -> Result<(), DimensionalLeakageError> {
        self.validate()?;
        if !self.enabled {
            return Ok(());
        }

        let mut staged_point_totals = Vec::with_capacity(self.points.len());
        let mut next_out = self.total_leaked_out;
        let mut next_in = self.total_leaked_in;

        for point in &self.points {
            if !point.active {
                staged_point_totals.push(point.total_transferred);
                continue;
            }

            let radius_factor = point.radius.min(10.0);
            let throughput = point.flow_rate.abs() * radius_factor;
            if !throughput.is_finite() || throughput < 0.0 {
                return Err(DimensionalLeakageError::NonFiniteAggregate);
            }

            let next_point_total = point.total_transferred + throughput;
            if !next_point_total.is_finite() {
                return Err(DimensionalLeakageError::NonFiniteAggregate);
            }
            staged_point_totals.push(next_point_total);

            if point.flow_rate < 0.0 {
                next_out += throughput;
                if !next_out.is_finite() {
                    return Err(DimensionalLeakageError::NonFiniteAggregate);
                }
            } else {
                next_in += throughput;
                if !next_in.is_finite() {
                    return Err(DimensionalLeakageError::NonFiniteAggregate);
                }
            }
        }

        let next_violation = next_out - next_in;
        if !next_violation.is_finite() {
            return Err(DimensionalLeakageError::NonFiniteAggregate);
        }

        for (point, next_total) in self.points.iter_mut().zip(staged_point_totals) {
            point.total_transferred = next_total;
        }
        self.total_leaked_out = next_out;
        self.total_leaked_in = next_in;
        self.apparent_violation = next_violation;
        Ok(())
    }

    /// Compatibility tick. Invalid state produces no partial mutation.
    pub fn tick(&mut self) {
        let _ = self.tick_checked();
    }

    pub fn prediction_error_at_checked(
        &self,
        pos: &SVector<f64, D>,
    ) -> Result<f64, DimensionalLeakageError> {
        let effect = self.total_effect_at_checked(pos)?;
        let prediction_error = effect.abs();
        if prediction_error.is_finite() {
            Ok(prediction_error)
        } else {
            Err(DimensionalLeakageError::NonFiniteEffect)
        }
    }

    pub fn prediction_error_at(&self, pos: &SVector<f64, D>) -> f64 {
        self.prediction_error_at_checked(pos).unwrap_or(0.0)
    }

    pub fn near_leakage_checked(
        &self,
        pos: &SVector<f64, D>,
        threshold: f64,
    ) -> Result<bool, DimensionalLeakageError> {
        self.validate()?;
        if !threshold.is_finite() || threshold < 0.0 {
            return Err(DimensionalLeakageError::InvalidThreshold);
        }
        for point in &self.points {
            if point.distance_checked(pos)? < threshold {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn near_leakage(&self, pos: &SVector<f64, D>, threshold: f64) -> bool {
        self.near_leakage_checked(pos, threshold).unwrap_or(false)
    }

    /// Heuristic local entropy-like indicator for gameplay/semantic use.
    ///
    /// The historical `abs(flow) * 0.1` mapping is not thermodynamic entropy in
    /// J/K and must not enter the physical entropy/heat ledger.
    pub fn local_entropy_indicator_checked(
        &self,
        pos: &SVector<f64, D>,
    ) -> Result<f64, DimensionalLeakageError> {
        let effect = self.total_effect_at_checked(pos)?;
        let indicator = effect.abs() * 0.1;
        if indicator.is_finite() {
            Ok(indicator)
        } else {
            Err(DimensionalLeakageError::NonFiniteEffect)
        }
    }

    /// Compatibility name retained for callers; this is a heuristic indicator,
    /// not a physical entropy measurement.
    pub fn local_entropy_increase(&self, pos: &SVector<f64, D>) -> f64 {
        self.local_entropy_indicator_checked(pos).unwrap_or(0.0)
    }
}

impl<const D: usize> Default for DimensionalLeakage<D> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec3(x: f64, y: f64, z: f64) -> SVector<f64, 3> {
        SVector::from([x, y, z])
    }

    #[test]
    fn sink_drains_energy() {
        let sink = LeakagePoint::sink_checked(vec3(0.0, 0.0, 0.0), 1.0, 0.5, 50.0).unwrap();
        assert!(sink.effect_at_checked(&vec3(5.0, 0.0, 0.0)).unwrap() < 0.0);
    }

    #[test]
    fn source_provides_energy() {
        let source = LeakagePoint::source_checked(vec3(0.0, 0.0, 0.0), 1.0, 0.5, 50.0).unwrap();
        assert!(source.effect_at_checked(&vec3(5.0, 0.0, 0.0)).unwrap() > 0.0);
    }

    #[test]
    fn invalid_point_construction_is_explicit_and_compatibility_add_is_fail_closed() {
        assert_eq!(
            LeakagePoint::source_checked(vec3(f64::NAN, 0.0, 0.0), 1.0, 1.0, 10.0)
                .unwrap_err(),
            DimensionalLeakageError::NonFinitePosition
        );
        assert_eq!(
            LeakagePoint::sink_checked(vec3(0.0, 0.0, 0.0), f64::INFINITY, 1.0, 10.0)
                .unwrap_err(),
            DimensionalLeakageError::NonFiniteWDepth
        );
        assert_eq!(
            LeakagePoint::source_checked(vec3(0.0, 0.0, 0.0), 1.0, f64::NAN, 10.0)
                .unwrap_err(),
            DimensionalLeakageError::NonFiniteRate
        );
        assert_eq!(
            LeakagePoint::source_checked(vec3(0.0, 0.0, 0.0), 1.0, 1.0, 0.0)
                .unwrap_err(),
            DimensionalLeakageError::InvalidRadius
        );

        let mut leakage = DimensionalLeakage::<3>::new();
        leakage.add_point(LeakagePoint::source(
            vec3(f64::NAN, 0.0, 0.0),
            1.0,
            1.0,
            10.0,
        ));
        assert!(leakage.points.is_empty());
    }

    #[test]
    fn effect_falls_off_with_distance() {
        let sink = LeakagePoint::sink_checked(vec3(0.0, 0.0, 0.0), 1.0, 1.0, 100.0).unwrap();
        let near = sink.effect_at_checked(&vec3(2.0, 0.0, 0.0)).unwrap().abs();
        let far = sink.effect_at_checked(&vec3(10.0, 0.0, 0.0)).unwrap().abs();
        assert!(near > far);
    }

    #[test]
    fn invalid_effect_query_is_explicit_but_compatibility_query_is_zero() {
        let sink = LeakagePoint::sink(vec3(0.0, 0.0, 0.0), 1.0, f64::NAN, 10.0);
        assert!(sink.effect_at_checked(&vec3(1.0, 0.0, 0.0)).is_err());
        assert_eq!(sink.effect_at(&vec3(1.0, 0.0, 0.0)), 0.0);
    }

    #[test]
    fn no_effect_outside_radius() {
        let sink = LeakagePoint::sink_checked(vec3(0.0, 0.0, 0.0), 1.0, 1.0, 10.0).unwrap();
        assert_eq!(sink.effect_at_checked(&vec3(20.0, 0.0, 0.0)).unwrap(), 0.0);
    }

    #[test]
    fn wormhole_conserves_symmetric_boundary_throughput() {
        let mut leakage = DimensionalLeakage::<3>::new();
        leakage
            .create_wormhole_checked(
                vec3(0.0, 0.0, 0.0),
                vec3(50.0, 0.0, 0.0),
                1.0,
                20.0,
            )
            .unwrap();
        assert!(leakage.total_effect_at_checked(&vec3(1.0, 0.0, 0.0)).unwrap() < 0.0);
        assert!(leakage.total_effect_at_checked(&vec3(51.0, 0.0, 0.0)).unwrap() > 0.0);
        leakage.tick_checked().unwrap();
        assert_eq!(leakage.total_leaked_out, leakage.total_leaked_in);
        assert_eq!(leakage.apparent_violation, 0.0);
    }

    #[test]
    fn invalid_wormhole_is_atomic() {
        let mut leakage = DimensionalLeakage::<3>::new();
        let before = leakage.clone();
        assert!(
            leakage
                .create_wormhole_checked(
                    vec3(0.0, 0.0, 0.0),
                    vec3(f64::NAN, 0.0, 0.0),
                    1.0,
                    20.0,
                )
                .is_err()
        );
        assert_eq!(leakage.points.len(), before.points.len());
        assert_eq!(leakage.enabled, before.enabled);
    }

    #[test]
    fn total_effect_aggregate_overflow_is_explicit() {
        let mut leakage = DimensionalLeakage::<3>::new();
        leakage
            .add_point_checked(LeakagePoint::source_checked(vec3(0.0, 0.0, 0.0), 1.0, f64::MAX, 10.0).unwrap())
            .unwrap();
        leakage
            .add_point_checked(LeakagePoint::source_checked(vec3(0.0, 0.0, 0.0), 1.0, f64::MAX, 10.0).unwrap())
            .unwrap();
        leakage.enabled = true;
        assert_eq!(
            leakage.total_effect_at_checked(&vec3(0.0, 0.0, 0.0)),
            Err(DimensionalLeakageError::NonFiniteAggregate)
        );
        assert_eq!(leakage.total_effect_at(&vec3(0.0, 0.0, 0.0)), 0.0);
    }

    #[test]
    fn tick_is_transactional_on_point_or_global_overflow() {
        let mut leakage = DimensionalLeakage::<3>::new();
        let mut point = LeakagePoint::source_checked(vec3(0.0, 0.0, 0.0), 1.0, f64::MAX, 10.0).unwrap();
        point.total_transferred = f64::MAX;
        leakage.add_point_checked(point).unwrap();
        leakage.enabled = true;
        let before_point = leakage.points[0].total_transferred;
        let before_in = leakage.total_leaked_in;
        assert_eq!(
            leakage.tick_checked(),
            Err(DimensionalLeakageError::NonFiniteAggregate)
        );
        assert_eq!(leakage.points[0].total_transferred, before_point);
        assert_eq!(leakage.total_leaked_in, before_in);
    }

    #[test]
    fn prediction_error_nonzero_near_leakage() {
        let mut leakage = DimensionalLeakage::<3>::new();
        leakage
            .create_wormhole_checked(
                vec3(0.0, 0.0, 0.0),
                vec3(100.0, 0.0, 0.0),
                1.0,
                30.0,
            )
            .unwrap();
        let near = leakage.prediction_error_at_checked(&vec3(5.0, 0.0, 0.0)).unwrap();
        let far = leakage.prediction_error_at_checked(&vec3(200.0, 0.0, 0.0)).unwrap();
        assert!(near > far);
    }

    #[test]
    fn entropy_name_is_compatibility_only_and_indicator_is_checked() {
        let mut leakage = DimensionalLeakage::<3>::new();
        leakage
            .create_wormhole_checked(
                vec3(0.0, 0.0, 0.0),
                vec3(100.0, 0.0, 0.0),
                1.0,
                30.0,
            )
            .unwrap();
        let indicator = leakage
            .local_entropy_indicator_checked(&vec3(5.0, 0.0, 0.0))
            .unwrap();
        assert!(indicator >= 0.0);
        assert_eq!(indicator, leakage.local_entropy_increase(&vec3(5.0, 0.0, 0.0)));
    }

    #[test]
    fn invalid_threshold_fails_closed() {
        let leakage = DimensionalLeakage::<3>::new();
        assert_eq!(
            leakage.near_leakage_checked(&vec3(0.0, 0.0, 0.0), f64::NAN),
            Err(DimensionalLeakageError::InvalidThreshold)
        );
        assert!(!leakage.near_leakage(&vec3(0.0, 0.0, 0.0), f64::NAN));
    }

    #[test]
    fn disabled_leakage_no_effect() {
        let leakage = DimensionalLeakage::<3>::new();
        assert_eq!(
            leakage.total_effect_at_checked(&vec3(0.0, 0.0, 0.0)).unwrap(),
            0.0
        );
    }

    #[test]
    fn tick_accumulates_totals() {
        let mut leakage = DimensionalLeakage::<3>::new();
        leakage
            .create_wormhole_checked(
                vec3(0.0, 0.0, 0.0),
                vec3(100.0, 0.0, 0.0),
                1.0,
                20.0,
            )
            .unwrap();
        leakage.tick_checked().unwrap();
        let first = leakage.total_leaked_out;
        leakage.tick_checked().unwrap();
        assert!(leakage.total_leaked_out > first);
    }

    #[test]
    fn works_in_4d_and_2d() {
        let mut four = DimensionalLeakage::<4>::new();
        four
            .create_wormhole_checked(
                SVector::from([0.0, 0.0, 0.0, 0.0]),
                SVector::from([50.0, 0.0, 0.0, 0.0]),
                1.0,
                20.0,
            )
            .unwrap();
        assert!(four.total_effect_at_checked(&SVector::from([1.0, 0.0, 0.0, 0.0])).unwrap() < 0.0);

        let mut two = DimensionalLeakage::<2>::new();
        two.create_wormhole_checked(
            SVector::from([0.0, 0.0]),
            SVector::from([50.0, 0.0]),
            1.0,
            20.0,
        )
        .unwrap();
        assert!(two.total_effect_at_checked(&SVector::from([1.0, 0.0])).unwrap() < 0.0);
    }
}
