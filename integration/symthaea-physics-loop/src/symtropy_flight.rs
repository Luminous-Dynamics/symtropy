//! Symtropy's PhysicsWorld as a backend for Symthaea's actual flight-control trait.
//!
//! Research integration backend, not a validated Crazyflie or MuJoCo-equivalent
//! model: spherical rigid body, mean-inertia approximation, exponential drag,
//! command-level thrust/moments, and a hard ground clamp. These limitations stay
//! explicit so a passing integration test cannot be mistaken for physical fidelity.

use nalgebra::{SMatrix, SVector};
use symthaea_multirotor::{FlightState, PhysicsSimulator, QuadrotorCommand};
use symtropy_math::{Bivector, Point, Rotor};
use symtropy_physics::{BodyHandle, PhysicsWorld};

const MASS_KG: f64 = 0.027;
const RADIUS_M: f64 = 0.02;
const DRAG_N_S_PER_M: f64 = 0.01;
const ANGULAR_DAMPING_PER_S: f64 = 5.0;
const INERTIA_KG_M2: [f64; 3] = [1.4e-5, 1.4e-5, 2.2e-5];

pub struct SymtropyFlightSimulator {
    world: PhysicsWorld<3>,
    body: BodyHandle,
    state: FlightState,
    elapsed_s: f64,
    external_force_n: [f64; 3],
    ground_contacts: u64,
    invalid_inputs: u64,
    /// Optional first-order actuator response time constant; None means direct actuation.
    actuator_lag_time_constant_s: Option<f64>,
    /// Filtered actuator outputs [thrust, roll, pitch, yaw], in physical command units.
    actuator_state: [f64; 4],
}

impl SymtropyFlightSimulator {
    pub fn new() -> Self {
        let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, 0.0, -9.81]));
        let body = world.add_sphere(Point::new([0.0, 0.0, 0.1]), RADIUS_M, MASS_KG);
        if let Some(rigid) = world.body_mut(body) {
            rigid.linear_damping = DRAG_N_S_PER_M / MASS_KG;
            rigid.angular_damping = ANGULAR_DAMPING_PER_S;
            rigid.inertia = SVector::from(INERTIA_KG_M2);
            rigid.inv_inertia = rigid.inertia.map(|i| if i > 0.0 { 1.0 / i } else { 0.0 });
        }
        let mut simulator = Self {
            world, body, state: FlightState::hover(0.1), elapsed_s: 0.0,
            external_force_n: [0.0; 3], ground_contacts: 0, invalid_inputs: 0,
            actuator_lag_time_constant_s: None,
            actuator_state: [QuadrotorCommand::HOVER_THRUST as f64, 0.0, 0.0, 0.0],
        };
        simulator.refresh_state();
        simulator
    }

    pub fn ground_contact_count(&self) -> u64 { self.ground_contacts }
    pub fn invalid_input_count(&self) -> u64 { self.invalid_inputs }
    pub fn elapsed_s(&self) -> f64 { self.elapsed_s }

    /// Enable a first-order actuator response model. `Some(tau)` applies the exact
    /// exponential discrete-time filter alpha = 1 - exp(-dt/tau) to thrust and all
    /// command moments. `None` preserves the historical direct-actuation behavior.
    ///
    /// This is a controlled stress-model parameter, not a calibrated Crazyflie motor
    /// model. Use the published actuator-speed benchmark adapter for real-flight claims.
    pub fn set_actuator_lag_time_constant_s(
        &mut self,
        tau_s: Option<f64>,
    ) -> Result<(), &'static str> {
        if tau_s.is_some_and(|tau| !tau.is_finite() || tau <= 0.0) {
            return Err("actuator lag time constant must be finite and strictly positive");
        }
        self.actuator_lag_time_constant_s = tau_s;
        self.actuator_state = [
            QuadrotorCommand::HOVER_THRUST as f64,
            0.0,
            0.0,
            0.0,
        ];
        Ok(())
    }

    pub fn actuator_lag_time_constant_s(&self) -> Option<f64> {
        self.actuator_lag_time_constant_s
    }

    pub fn reset_to_state(&mut self, state: &FlightState) {
        let mut clean = state.clone();
        if !clean.is_finite() {
            self.invalid_inputs = self.invalid_inputs.saturating_add(1);
            clean = FlightState::hover(0.1);
        }
        normalize_quaternion(&mut clean.quaternion);
        if clean.position[2] < 0.0 { clean.position[2] = 0.0; }
        if let Some(body) = self.world.body_mut(self.body) {
            body.transform.translation = Point::new(clean.position);
            body.transform.rotation = rotor_from_quaternion(clean.quaternion);
            body.linear_velocity = SVector::from(clean.linear_velocity);
            body.angular_velocity = angular_bivector(clean.angular_velocity);
            body.clear_accumulators();
            body.wake();
        }
        self.state = clean;
        self.elapsed_s = 0.0;
        self.external_force_n = [0.0; 3];
        self.ground_contacts = 0;
        self.actuator_state = [
            QuadrotorCommand::HOVER_THRUST as f64,
            0.0,
            0.0,
            0.0,
        ];
        self.refresh_state();
    }

    fn refresh_state(&mut self) {
        let Some(body) = self.world.body(self.body) else {
            self.invalid_inputs = self.invalid_inputs.saturating_add(1);
            return;
        };
        let position = body.transform.translation.0;
        let rotation = body.transform.rotation.to_matrix();
        let linear = body.linear_velocity;
        self.state = FlightState {
            position: [position[0], position[1], position[2]],
            quaternion: quaternion_from_matrix(rotation),
            linear_velocity: [linear[0], linear[1], linear[2]],
            angular_velocity: [
                body.angular_velocity.get(1, 2),
                -body.angular_velocity.get(0, 2),
                body.angular_velocity.get(0, 1),
            ],
            timestamp: self.elapsed_s,
        };
        normalize_quaternion(&mut self.state.quaternion);
    }

    fn reject_non_finite_state(&mut self) {
        if self.state.is_finite() { return; }
        self.invalid_inputs = self.invalid_inputs.saturating_add(1);
        self.reset_to_state(&FlightState::hover(0.1));
    }
}

impl Default for SymtropyFlightSimulator {
    fn default() -> Self { Self::new() }
}

impl PhysicsSimulator for SymtropyFlightSimulator {
    fn step(&mut self, cmd: &QuadrotorCommand, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            self.invalid_inputs = self.invalid_inputs.saturating_add(1);
            self.external_force_n = [0.0; 3];
            return;
        }
        // Reject a malformed action before it can be applied to the physics state.
        if !cmd.to_ctrl().iter().all(|value| value.is_finite()) {
            self.invalid_inputs = self.invalid_inputs.saturating_add(1);
            self.external_force_n = [0.0; 3];
            return;
        }
        let requested = cmd.clamped();
        let command = if let Some(tau_s) = self.actuator_lag_time_constant_s {
            let alpha = 1.0 - (-dt / tau_s).exp();
            let requested_values = requested.to_ctrl();
            for index in 0..4 {
                self.actuator_state[index] +=
                    alpha * (requested_values[index] - self.actuator_state[index]);
            }
            QuadrotorCommand {
                thrust: self.actuator_state[0] as f32,
                roll_moment: self.actuator_state[1] as f32,
                pitch_moment: self.actuator_state[2] as f32,
                yaw_moment: self.actuator_state[3] as f32,
            }
            .clamped()
        } else {
            requested
        };
        let external_force = self.external_force_n;
        self.external_force_n = [0.0; 3];
        if !external_force.iter().all(|value| value.is_finite()) {
            self.invalid_inputs = self.invalid_inputs.saturating_add(1);
            return;
        }
        if let Some(body) = self.world.body_mut(self.body) {
            let body_z = SVector::from([0.0, 0.0, 1.0]);
            let thrust_world = body.transform.rotation.rotate_vector(&body_z) * (command.thrust as f64);
            let force = thrust_world + SVector::from(external_force);
            if force.iter().all(|value| value.is_finite()) {
                body.apply_force(force);
            } else {
                self.invalid_inputs = self.invalid_inputs.saturating_add(1);
            }
            let mut torque = Bivector::<3>::zero();
            torque.set(1, 2, command.roll_moment as f64);
            torque.set(0, 2, -(command.pitch_moment as f64));
            torque.set(0, 1, command.yaw_moment as f64);
            body.apply_torque(torque);
        }

        self.world.step(dt);
        self.elapsed_s += dt;
        // Explicit ground clamp, not a contact-complete landing model.
        if let Some(body) = self.world.body_mut(self.body) {
            if body.transform.translation.0[2] < 0.0 {
                body.transform.translation.0[2] = 0.0;
                if body.linear_velocity[2] < 0.0 { body.linear_velocity[2] = 0.0; }
                self.ground_contacts = self.ground_contacts.saturating_add(1);
                body.wake();
            }
        }
        self.refresh_state();
        self.reject_non_finite_state();
    }

    fn state(&self) -> &FlightState { &self.state }
    fn reset(&mut self, altitude: f64) {
        self.reset_to_state(&FlightState::hover(altitude.max(0.0)));
    }

    fn reset_with_perturbation(&mut self, altitude: f64, perturbation: f64, seed: u64) {
        if !altitude.is_finite() || !perturbation.is_finite() || perturbation < 0.0 {
            self.invalid_inputs = self.invalid_inputs.saturating_add(1);
            self.reset(0.1);
            return;
        }
        let mut rng = seed;
        let mut next_f64 = || -> f64 {
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            (rng as f64 / u64::MAX as f64) * 2.0 - 1.0
        };
        let mut state = FlightState::hover(altitude.max(0.0));
        state.position[0] += perturbation * next_f64() * 0.1;
        state.position[1] += perturbation * next_f64() * 0.1;
        state.position[2] += perturbation * next_f64() * 0.05;
        let tilt = perturbation * next_f64() * 0.1;
        state.quaternion = normalize_quaternion_value([1.0, tilt, tilt * 0.5, 0.0]);
        self.reset_to_state(&state);
    }

    fn apply_external_force(&mut self, force: [f64; 3]) {
        if !force.iter().all(|value| value.is_finite()) {
            self.invalid_inputs = self.invalid_inputs.saturating_add(1);
            return;
        }
        for index in 0..3 { self.external_force_n[index] += force[index]; }
    }
}

fn rotor_from_quaternion(q: [f64; 4]) -> Rotor<3> {
    let [w, x, y, z] = normalize_quaternion_value(q);
    let matrix = SMatrix::<f64, 3, 3>::from_row_slice(&[
        1.0 - 2.0 * (y*y + z*z), 2.0 * (x*y - z*w), 2.0 * (x*z + y*w),
        2.0 * (x*y + z*w), 1.0 - 2.0 * (x*x + z*z), 2.0 * (y*z - x*w),
        2.0 * (x*z - y*w), 2.0 * (y*z + x*w), 1.0 - 2.0 * (x*x + y*y),
    ]);
    Rotor::from_matrix(matrix)
}

fn quaternion_from_matrix(m: &SMatrix<f64, 3, 3>) -> [f64; 4] {
    let trace = m[(0,0)] + m[(1,1)] + m[(2,2)];
    let q = if trace > 0.0 {
        let s = (trace + 1.0).sqrt().max(1e-15) * 2.0;
        [0.25*s, (m[(2,1)] - m[(1,2)]) / s, (m[(0,2)] - m[(2,0)]) / s, (m[(1,0)] - m[(0,1)]) / s]
    } else if m[(0,0)] > m[(1,1)] && m[(0,0)] > m[(2,2)] {
        let s = (1.0 + m[(0,0)] - m[(1,1)] - m[(2,2)]).sqrt().max(1e-15) * 2.0;
        [(m[(2,1)] - m[(1,2)]) / s, 0.25*s, (m[(0,1)] + m[(1,0)]) / s, (m[(0,2)] + m[(2,0)]) / s]
    } else if m[(1,1)] > m[(2,2)] {
        let s = (1.0 + m[(1,1)] - m[(0,0)] - m[(2,2)]).sqrt().max(1e-15) * 2.0;
        [(m[(0,2)] - m[(2,0)]) / s, (m[(0,1)] + m[(1,0)]) / s, 0.25*s, (m[(1,2)] + m[(2,1)]) / s]
    } else {
        let s = (1.0 + m[(2,2)] - m[(0,0)] - m[(1,1)]).sqrt().max(1e-15) * 2.0;
        [(m[(1,0)] - m[(0,1)]) / s, (m[(0,2)] + m[(2,0)]) / s, (m[(1,2)] + m[(2,1)]) / s, 0.25*s]
    };
    normalize_quaternion_value(q)
}

fn angular_bivector(omega: [f64; 3]) -> Bivector<3> {
    let mut b = Bivector::zero();
    b.set(1, 2, omega[0]);
    b.set(0, 2, -omega[1]);
    b.set(0, 1, omega[2]);
    b
}
fn normalize_quaternion(q: &mut [f64; 4]) { *q = normalize_quaternion_value(*q); }
fn normalize_quaternion_value(q: [f64; 4]) -> [f64; 4] {
    let n = q.iter().map(|v| v*v).sum::<f64>().sqrt();
    if !n.is_finite() || n < 1e-10 { [1.0, 0.0, 0.0, 0.0] }
    else { q.map(|v| v/n) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_perturbation_is_repeatable() {
        let mut a=SymtropyFlightSimulator::new(); let mut b=SymtropyFlightSimulator::new();
        a.reset_with_perturbation(0.1,0.08,17); b.reset_with_perturbation(0.1,0.08,17);
        for (x,y) in a.state().position.iter().zip(b.state().position.iter()) { assert!((x-y).abs()<1e-12); }
        for (x,y) in a.state().quaternion.iter().zip(b.state().quaternion.iter()) { assert!((x-y).abs()<1e-12); }
    }
    #[test]
    fn hover_command_keeps_state_finite_and_bounded() {
        let mut sim=SymtropyFlightSimulator::new(); sim.reset(0.1);
        for _ in 0..1000 { sim.step(&QuadrotorCommand::hover(),1.0/500.0); }
        assert!(sim.state().is_bounded()); assert!(sim.state().position[2]>0.0);
        assert!((sim.state().position[2]-0.1).abs()<0.05); assert_eq!(sim.invalid_input_count(),0);
    }
    #[test]
    fn one_tick_external_force_changes_lateral_state_and_is_consumed() {
        let mut sim=SymtropyFlightSimulator::new(); sim.reset(0.1);
        sim.apply_external_force([0.002,0.0,0.0]); sim.step(&QuadrotorCommand::hover(),0.01);
        let first=sim.state().position[0]; assert!(first>0.0);
        sim.step(&QuadrotorCommand::hover(),0.01); assert!(sim.state().position[0]>=first);
        assert_eq!(sim.invalid_input_count(),0);
    }
    #[test]
    fn actuator_lag_is_opt_in_and_changes_first_step_response() {
        let mut direct = SymtropyFlightSimulator::new();
        let mut lagged = SymtropyFlightSimulator::new();
        direct.reset(0.1);
        lagged.reset(0.1);
        lagged
            .set_actuator_lag_time_constant_s(Some(0.020))
            .expect("positive finite lag is valid");
        let command = QuadrotorCommand::zero();
        direct.step(&command, 1.0 / 500.0);
        lagged.step(&command, 1.0 / 500.0);
        assert!(direct.state().linear_velocity[2] < lagged.state().linear_velocity[2]);
        assert_eq!(direct.actuator_lag_time_constant_s(), None);
        assert_eq!(lagged.actuator_lag_time_constant_s(), Some(0.020));
    }

    #[test]
    fn actuator_lag_rejects_invalid_constants_and_reset_reinitializes_filter() {
        let mut sim = SymtropyFlightSimulator::new();
        assert!(sim.set_actuator_lag_time_constant_s(Some(0.0)).is_err());
        assert!(sim.set_actuator_lag_time_constant_s(Some(f64::NAN)).is_err());
        assert!(sim.set_actuator_lag_time_constant_s(Some(-0.01)).is_err());
        sim.set_actuator_lag_time_constant_s(Some(0.020)).expect("valid lag");
        sim.step(&QuadrotorCommand::zero(), 1.0 / 500.0);
        sim.reset(0.1);
        assert_eq!(sim.actuator_lag_time_constant_s(), Some(0.020));
        let before = sim.state().clone();
        sim.step(&QuadrotorCommand::zero(), 1.0 / 500.0);
        assert!(sim.state().linear_velocity[2] > before.linear_velocity[2] - 0.02);
    }

    #[test]
    fn non_finite_command_is_rejected_before_mutating_physics() {
        let mut sim = SymtropyFlightSimulator::new();
        sim.reset(0.4);
        let before = sim.state().clone();
        let command = QuadrotorCommand {
            thrust: f32::NAN,
            ..QuadrotorCommand::hover()
        };
        sim.step(&command, 1.0 / 500.0);
        assert!(sim.state().is_finite());
        assert_eq!(sim.state().position, before.position);
        assert_eq!(sim.invalid_input_count(), 1);
    }

    #[test]
    fn rotation_round_trips_through_symtropy_matrix() {
        let input=normalize_quaternion_value([0.91,0.12,-0.24,0.31]);
        let rotor=rotor_from_quaternion(input); let recovered=quaternion_from_matrix(rotor.to_matrix());
        let dot: f64=input.iter().zip(recovered.iter()).map(|(a,b)| a*b).sum();
        assert!((dot.abs()-1.0).abs()<1e-8,"quaternion dot={dot}");
    }
}
