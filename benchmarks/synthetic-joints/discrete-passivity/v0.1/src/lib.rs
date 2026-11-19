#![forbid(unsafe_code)]

//! Dependency-free scalar mechanical-port passivity and energy-tank reference.
//!
//! This M1 module is a set of accounting/control primitives, not a general
//! port-Hamiltonian controller or a hardware safety certification. Callers
//! must declare port signs, sampling convention, measurement boundaries and
//! the physical path that captures negative mechanical work.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortSample {
    pub time_s: f64,
    /// Generalized effort (e.g. N m).
    pub effort: f64,
    /// Conjugate flow (e.g. rad/s).
    pub flow: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassivityAssessment {
    pub storage_change_j: f64,
    pub port_work_j: f64,
    pub tolerance_j: f64,
    /// max(0, delta_storage - port_work - tolerance).
    pub deficit_j: f64,
    /// Signed residual delta_storage - port_work.
    pub residual_j: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TankReceipt {
    pub energy_before_j: f64,
    pub energy_after_j: f64,
    pub mechanical_port_work_j: f64,
    pub energy_debited_j: f64,
    pub energy_captured_j: f64,
    pub energy_spilled_j: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TankLimitedEffort {
    pub desired_effort: f64,
    pub actuator_saturated_effort: f64,
    pub applied_effort: f64,
    pub requested_port_work_j: f64,
    pub applied_port_work_j: f64,
    /// Positive energy injection prevented by the tank.
    pub curtailed_injection_j: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassivityCorrection {
    pub requested_effort: f64,
    pub bounded_requested_effort: f64,
    pub corrected_effort: f64,
    /// Correction effort added by the passivity controller.
    pub damping_effort: f64,
    pub energy_dissipated_j: f64,
    pub remaining_deficit_j: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorqueAngleSample {
    pub generalized_coordinate_rad: f64,
    pub generalized_torque_nm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyTank {
    energy_j: f64,
    minimum_energy_j: f64,
    maximum_energy_j: f64,
    /// Fraction of negative mechanical-port work physically routed into the tank.
    capture_efficiency: f64,
}

fn finite(value: f64) -> bool {
    value.is_finite()
}

impl EnergyTank {
    pub fn new(
        initial_energy_j: f64,
        minimum_energy_j: f64,
        maximum_energy_j: f64,
        capture_efficiency: f64,
    ) -> Option<Self> {
        if !finite(initial_energy_j)
            || !finite(minimum_energy_j)
            || !finite(maximum_energy_j)
            || !finite(capture_efficiency)
            || minimum_energy_j < 0.0
            || maximum_energy_j < minimum_energy_j
            || initial_energy_j < minimum_energy_j
            || initial_energy_j > maximum_energy_j
            || !(0.0..=1.0).contains(&capture_efficiency)
        {
            return None;
        }

        Some(Self {
            energy_j: initial_energy_j,
            minimum_energy_j,
            maximum_energy_j,
            capture_efficiency,
        })
    }

    pub fn energy_j(&self) -> f64 {
        self.energy_j
    }

    pub fn available_injection_energy_j(&self) -> f64 {
        (self.energy_j - self.minimum_energy_j).max(0.0)
    }

    pub fn remaining_capacity_j(&self) -> f64 {
        (self.maximum_energy_j - self.energy_j).max(0.0)
    }

    /// Books measured signed mechanical-port work.
    ///
    /// Positive work is energy injected into the plant and debited from the
    /// tank. Negative work replenishes the tank only by the explicit capture
    /// efficiency. Energy that cannot be accepted at the upper bound is
    /// returned as spilled energy; it is not silently treated as stored.
    pub fn book_mechanical_work(&mut self, work_j: f64) -> Result<TankReceipt, String> {
        if !finite(work_j) {
            return Err("mechanical-port work must be finite".to_owned());
        }

        let before = self.energy_j;
        let mut debited = 0.0;
        let mut captured = 0.0;
        let mut spilled = 0.0;

        if work_j > 0.0 {
            let available = self.available_injection_energy_j();
            if work_j > available + 1.0e-12 * available.max(1.0) {
                return Err(format!(
                    "requested injection {work_j} J exceeds tank budget {available} J"
                ));
            }
            debited = work_j.min(available);
            self.energy_j -= debited;
        } else if work_j < 0.0 {
            let routed_energy = -work_j * self.capture_efficiency;
            let capacity = self.remaining_capacity_j();
            captured = routed_energy.min(capacity);
            spilled = routed_energy - captured;
            self.energy_j += captured;
        }

        if !finite(self.energy_j)
            || self.energy_j < self.minimum_energy_j - 1.0e-10
            || self.energy_j > self.maximum_energy_j + 1.0e-10
        {
            self.energy_j = before;
            return Err("tank state escaped declared minimum/maximum bounds".to_owned());
        }

        Ok(TankReceipt {
            energy_before_j: before,
            energy_after_j: self.energy_j,
            mechanical_port_work_j: work_j,
            energy_debited_j: debited,
            energy_captured_j: captured,
            energy_spilled_j: spilled,
        })
    }
}

/// Integrates scalar effort×flow power at the declared mechanical port using
/// the composite trapezoidal rule. All samples must share one effort/flow unit
/// convention and strictly increasing timestamps.
pub fn integrate_port_work_trapezoid(samples: &[PortSample]) -> Result<f64, String> {
    if samples.len() < 2 {
        return Err("at least two port samples are required".to_owned());
    }

    let mut work_j = 0.0;
    for (index, sample) in samples.iter().enumerate() {
        if !finite(sample.time_s) || !finite(sample.effort) || !finite(sample.flow) {
            return Err(format!("non-finite port sample at index {index}"));
        }
        if index + 1 == samples.len() {
            continue;
        }

        let next = samples[index + 1];
        if !finite(next.time_s) || !finite(next.effort) || !finite(next.flow) {
            return Err(format!("non-finite port sample at index {}", index + 1));
        }
        let dt = next.time_s - sample.time_s;
        if dt <= 0.0 || !finite(dt) {
            return Err(format!("port sample timestamps are not increasing at index {index}"));
        }

        let power_before = sample.effort * sample.flow;
        let power_after = next.effort * next.flow;
        work_j += 0.5 * (power_before + power_after) * dt;
        if !finite(work_j) {
            return Err("integrated port work is non-finite".to_owned());
        }
    }

    Ok(work_j)
}

/// Checks the sampled storage inequality: delta_H <= W_port + tolerance.
///
/// A positive deficit indicates unaccounted energy generation relative to the
/// declared storage and measured port work. Dissipation is not a free balancing
/// term; include it in the declared storage/control-volume model before calling.
pub fn assess_passivity(
    storage_change_j: f64,
    port_work_j: f64,
    tolerance_j: f64,
) -> Option<PassivityAssessment> {
    if !finite(storage_change_j)
        || !finite(port_work_j)
        || !finite(tolerance_j)
        || tolerance_j < 0.0
    {
        return None;
    }

    let residual = storage_change_j - port_work_j;
    let deficit = (residual - tolerance_j).max(0.0);
    if !finite(residual) || !finite(deficit) {
        return None;
    }

    Some(PassivityAssessment {
        storage_change_j,
        port_work_j,
        tolerance_j,
        deficit_j: deficit,
        residual_j: residual,
    })
}

/// Scales a bounded scalar effort command so its positive port work cannot
/// exceed the tank's currently available injection energy over one interval.
///
/// Negative mechanical work is not automatically credited here. Credit is a
/// separate operation performed by EnergyTank::book_mechanical_work using an
/// explicit physical capture efficiency.
pub fn limit_effort_by_tank(
    desired_effort: f64,
    flow: f64,
    dt_s: f64,
    maximum_abs_effort: f64,
    available_injection_energy_j: f64,
) -> Option<TankLimitedEffort> {
    if !finite(desired_effort)
        || !finite(flow)
        || !finite(dt_s)
        || !finite(maximum_abs_effort)
        || !finite(available_injection_energy_j)
        || dt_s <= 0.0
        || maximum_abs_effort < 0.0
        || available_injection_energy_j < 0.0
    {
        return None;
    }

    let actuator_saturated_effort =
        desired_effort.clamp(-maximum_abs_effort, maximum_abs_effort);
    let requested_work = actuator_saturated_effort * flow * dt_s;
    if !finite(requested_work) {
        return None;
    }

    let applied_effort = if requested_work > available_injection_energy_j
        && requested_work > 0.0
    {
        actuator_saturated_effort * (available_injection_energy_j / requested_work)
    } else {
        actuator_saturated_effort
    };
    let applied_work = applied_effort * flow * dt_s;
    if !finite(applied_effort) || !finite(applied_work) {
        return None;
    }

    Some(TankLimitedEffort {
        desired_effort,
        actuator_saturated_effort,
        applied_effort,
        requested_port_work_j: requested_work,
        applied_port_work_j: applied_work,
        curtailed_injection_j: (requested_work - applied_work).max(0.0),
    })
}

/// Adds a dissipative correction to a bounded scalar effort command to reduce
/// an observed passivity deficit. The correction always has non-positive power
/// at the current measured flow; insufficient torque authority is reported
/// through remaining_deficit_j.
pub fn apply_passivity_correction(
    desired_effort: f64,
    flow: f64,
    dt_s: f64,
    maximum_abs_effort: f64,
    deficit_j: f64,
) -> Option<PassivityCorrection> {
    if !finite(desired_effort)
        || !finite(flow)
        || !finite(dt_s)
        || !finite(maximum_abs_effort)
        || !finite(deficit_j)
        || dt_s <= 0.0
        || maximum_abs_effort < 0.0
        || deficit_j < 0.0
    {
        return None;
    }

    let bounded = desired_effort.clamp(-maximum_abs_effort, maximum_abs_effort);
    let corrected = if deficit_j == 0.0 || flow.abs() <= f64::MIN_POSITIVE {
        bounded
    } else {
        let required_damping_effort = deficit_j / (flow.abs() * dt_s);
        let direction = flow.signum();
        (bounded - direction * required_damping_effort)
            .clamp(-maximum_abs_effort, maximum_abs_effort)
    };

    let damping_effort = corrected - bounded;
    let correction_power = damping_effort * flow;
    if !finite(correction_power) || correction_power > 1.0e-12 {
        return None;
    }

    let dissipated = (-correction_power * dt_s).max(0.0);
    let remaining = (deficit_j - dissipated).max(0.0);

    Some(PassivityCorrection {
        requested_effort: desired_effort,
        bounded_requested_effort: bounded,
        corrected_effort: corrected,
        damping_effort,
        energy_dissipated_j: dissipated,
        remaining_deficit_j: remaining,
    })
}

/// Computes work along a sampled generalized-coordinate path:
/// sum 0.5*(tau_i + tau_(i+1))*(q_(i+1)-q_i).
///
/// The generalized coordinate is in radians and torque in N m, so the result
/// is joules. Coordinate reversals are allowed and contribute signed work.
pub fn integrate_torque_angle_work(samples: &[TorqueAngleSample]) -> Result<f64, String> {
    if samples.len() < 2 {
        return Err("at least two torque-angle samples are required".to_owned());
    }

    let mut work_j = 0.0;
    for (index, sample) in samples.iter().enumerate() {
        if !finite(sample.generalized_coordinate_rad) || !finite(sample.generalized_torque_nm) {
            return Err(format!("non-finite torque-angle sample at index {index}"));
        }
        if index + 1 == samples.len() {
            continue;
        }

        let next = samples[index + 1];
        let delta_q = next.generalized_coordinate_rad - sample.generalized_coordinate_rad;
        let average_torque = 0.5 * (sample.generalized_torque_nm + next.generalized_torque_nm);
        work_j += average_torque * delta_q;
        if !finite(work_j) {
            return Err("integrated torque-angle work is non-finite".to_owned());
        }
    }
    Ok(work_j)
}

/// Upper bound on work for a declared maximum torque and angular travel.
/// This returns joules, unlike the torque bound itself (N m).
pub fn bounded_torque_travel_work_j(
    maximum_abs_torque_nm: f64,
    maximum_absolute_travel_rad: f64,
) -> Option<f64> {
    if !finite(maximum_abs_torque_nm)
        || !finite(maximum_absolute_travel_rad)
        || maximum_abs_torque_nm < 0.0
        || maximum_absolute_travel_rad < 0.0
    {
        return None;
    }
    let work = maximum_abs_torque_nm * maximum_absolute_travel_rad;
    finite(work).then_some(work)
}

/// Stored energy in a linear torsional spring, 0.5*k*theta^2.
pub fn torsional_spring_energy_j(stiffness_nm_per_rad: f64, deflection_rad: f64) -> Option<f64> {
    if !finite(stiffness_nm_per_rad)
        || !finite(deflection_rad)
        || stiffness_nm_per_rad < 0.0
    {
        return None;
    }
    let energy = 0.5 * stiffness_nm_per_rad * deflection_rad.powi(2);
    finite(energy).then_some(energy)
}

/// Dissipated energy in a linear rotational damper over a constant-speed step.
pub fn torsional_damper_dissipation_j(
    damping_nm_s_per_rad: f64,
    angular_velocity_rad_s: f64,
    dt_s: f64,
) -> Option<f64> {
    if !finite(damping_nm_s_per_rad)
        || !finite(angular_velocity_rad_s)
        || !finite(dt_s)
        || damping_nm_s_per_rad < 0.0
        || dt_s < 0.0
    {
        return None;
    }
    let dissipation = damping_nm_s_per_rad * angular_velocity_rad_s.powi(2) * dt_s;
    finite(dissipation).then_some(dissipation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64, label: &str) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{label}: expected {expected:.12e}, got {actual:.12e}, tolerance {tolerance:.3e}"
        );
    }

    #[test]
    fn passivity_001_port_work_known_answer_and_sign() {
        let injection = [
            PortSample { time_s: 0.0, effort: 2.0, flow: 3.0 },
            PortSample { time_s: 1.0, effort: 2.0, flow: 3.0 },
        ];
        close(integrate_port_work_trapezoid(&injection).unwrap(), 6.0, 1.0e-12, "positive port work");

        let absorption = [
            PortSample { time_s: 0.0, effort: -2.0, flow: 3.0 },
            PortSample { time_s: 1.0, effort: -2.0, flow: 3.0 },
        ];
        close(integrate_port_work_trapezoid(&absorption).unwrap(), -6.0, 1.0e-12, "negative port work");
    }

    #[test]
    fn passivity_002_port_work_is_invariant_to_linear_sample_subdivision() {
        let coarse = [
            PortSample { time_s: 0.0, effort: 2.0, flow: 0.0 },
            PortSample { time_s: 1.0, effort: 2.0, flow: 4.0 },
        ];
        let subdivided = [
            PortSample { time_s: 0.0, effort: 2.0, flow: 0.0 },
            PortSample { time_s: 0.25, effort: 2.0, flow: 1.0 },
            PortSample { time_s: 0.5, effort: 2.0, flow: 2.0 },
            PortSample { time_s: 0.75, effort: 2.0, flow: 3.0 },
            PortSample { time_s: 1.0, effort: 2.0, flow: 4.0 },
        ];
        close(
            integrate_port_work_trapezoid(&coarse).unwrap(),
            integrate_port_work_trapezoid(&subdivided).unwrap(),
            1.0e-12,
            "linear interpolation sample subdivision",
        );
    }

    #[test]
    fn passivity_003_invalid_port_samples_reject() {
        assert!(integrate_port_work_trapezoid(&[]).is_err());
        let duplicate_time = [
            PortSample { time_s: 0.0, effort: 1.0, flow: 1.0 },
            PortSample { time_s: 0.0, effort: 1.0, flow: 1.0 },
        ];
        assert!(integrate_port_work_trapezoid(&duplicate_time).is_err());
        let nonfinite = [
            PortSample { time_s: 0.0, effort: 1.0, flow: 1.0 },
            PortSample { time_s: 1.0, effort: f64::NAN, flow: 1.0 },
        ];
        assert!(integrate_port_work_trapezoid(&nonfinite).is_err());
    }

    #[test]
    fn passivity_004_storage_inequality_detects_energy_deficit_and_tolerance() {
        let active = assess_passivity(5.0, 3.0, 0.5).unwrap();
        close(active.residual_j, 2.0, 1.0e-12, "residual");
        close(active.deficit_j, 1.5, 1.0e-12, "deficit");

        let passive = assess_passivity(2.0, 3.0, 0.1).unwrap();
        close(passive.deficit_j, 0.0, 1.0e-12, "passive deficit");
    }

    #[test]
    fn passivity_005_energy_tank_debits_injected_work_and_captures_only_declared_fraction() {
        let mut tank = EnergyTank::new(10.0, 2.0, 12.0, 0.5).unwrap();
        let debit = tank.book_mechanical_work(3.0).unwrap();
        close(debit.energy_after_j, 7.0, 1.0e-12, "tank after debit");
        close(debit.energy_debited_j, 3.0, 1.0e-12, "debit");

        let capture = tank.book_mechanical_work(-4.0).unwrap();
        close(capture.energy_captured_j, 2.0, 1.0e-12, "captured fraction");
        close(capture.energy_after_j, 9.0, 1.0e-12, "tank after capture");
        close(capture.energy_spilled_j, 0.0, 1.0e-12, "spilled");
    }

    #[test]
    fn passivity_006_energy_tank_clamps_capacity_and_rejects_overdraw_without_mutation() {
        let mut tank = EnergyTank::new(11.0, 1.0, 12.0, 1.0).unwrap();
        let capture = tank.book_mechanical_work(-5.0).unwrap();
        close(capture.energy_captured_j, 1.0, 1.0e-12, "capacity limited capture");
        close(capture.energy_spilled_j, 4.0, 1.0e-12, "spilled energy");

        let before = tank.energy_j();
        assert!(tank.book_mechanical_work(20.0).is_err());
        close(tank.energy_j(), before, 1.0e-12, "state unchanged after overdraw");
    }

    #[test]
    fn passivity_007_tank_limited_effort_respects_energy_budget() {
        let limited = limit_effort_by_tank(10.0, 2.0, 1.0, 20.0, 6.0).unwrap();
        close(limited.applied_effort, 3.0, 1.0e-12, "applied effort");
        close(limited.applied_port_work_j, 6.0, 1.0e-12, "applied work");
        close(limited.curtailed_injection_j, 14.0, 1.0e-12, "curtailed work");
    }

    #[test]
    fn passivity_008_energy_tank_does_not_clip_negative_mechanical_work() {
        let limited = limit_effort_by_tank(-10.0, 2.0, 1.0, 20.0, 0.0).unwrap();
        close(limited.applied_effort, -10.0, 1.0e-12, "negative work effort");
        close(limited.applied_port_work_j, -20.0, 1.0e-12, "negative work");
    }

    #[test]
    fn passivity_009_controller_correction_is_dissipative_and_reports_residual() {
        let correction = apply_passivity_correction(2.0, 1.0, 1.0, 5.0, 3.0).unwrap();
        close(correction.corrected_effort, -1.0, 1.0e-12, "corrected effort");
        close(correction.damping_effort, -3.0, 1.0e-12, "damping effort");
        close(correction.energy_dissipated_j, 3.0, 1.0e-12, "dissipated");
        close(correction.remaining_deficit_j, 0.0, 1.0e-12, "remaining");

        let bounded = apply_passivity_correction(5.0, 1.0, 1.0, 5.0, 20.0).unwrap();
        assert!(bounded.damping_effort * 1.0 <= 0.0);
        assert!(bounded.remaining_deficit_j > 0.0);
    }

    #[test]
    fn passivity_010_zero_flow_cannot_dissipate_energy_deficit() {
        let correction = apply_passivity_correction(2.0, 0.0, 0.1, 5.0, 1.0).unwrap();
        close(correction.energy_dissipated_j, 0.0, 1.0e-12, "zero-flow dissipation");
        close(correction.remaining_deficit_j, 1.0, 1.0e-12, "unpaid deficit");
    }

    #[test]
    fn passivity_011_gravity_torque_alone_is_not_energy() {
        // 70.632 N m is a torque, not a tank capacity in joules.
        let torque_nm = 70.632;
        let work = bounded_torque_travel_work_j(
            torque_nm,
            std::f64::consts::FRAC_PI_2,
        ).unwrap();
        close(
            work,
            torque_nm * std::f64::consts::FRAC_PI_2,
            1.0e-12,
            "torque bound times angular travel",
        );
    }

    #[test]
    fn passivity_012_torque_angle_integral_tracks_direction_reversal() {
        let samples = [
            TorqueAngleSample { generalized_coordinate_rad: 0.0, generalized_torque_nm: 10.0 },
            TorqueAngleSample { generalized_coordinate_rad: 0.5, generalized_torque_nm: 10.0 },
            TorqueAngleSample { generalized_coordinate_rad: 0.0, generalized_torque_nm: 10.0 },
        ];
        close(integrate_torque_angle_work(&samples).unwrap(), 0.0, 1.0e-12, "forward and reverse work");
    }

    #[test]
    fn passivity_013_linear_spring_and_damper_have_known_energy() {
        close(torsional_spring_energy_j(100.0, 0.2).unwrap(), 2.0, 1.0e-12, "spring energy");
        close(torsional_damper_dissipation_j(10.0, 2.0, 0.5).unwrap(), 20.0, 1.0e-12, "damper dissipation");
    }

    #[test]
    fn passivity_014_invalid_tank_and_dimensional_inputs_fail_closed() {
        assert!(EnergyTank::new(1.0, 2.0, 3.0, 0.5).is_none());
        assert!(EnergyTank::new(2.0, 1.0, 3.0, 1.1).is_none());
        assert!(bounded_torque_travel_work_j(-1.0, 0.5).is_none());
        assert!(torsional_spring_energy_j(-1.0, 0.2).is_none());
        assert!(torsional_damper_dissipation_j(1.0, 2.0, -1.0).is_none());
        assert!(assess_passivity(1.0, 1.0, -0.1).is_none());
    }
}
