// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixed-step RK4 integration of timelike and null geodesics on stationary
//! Schwarzschild/Kerr backgrounds.
//!
//! Coordinates are measured in metres/radians: (ct,r,theta,phi) for exterior
//! charts and (v,r,theta,phi) for ingoing charts. For timelike curves the
//! affine parameter is proper length s=c*tau, so g(u,u)=-1. For null curves it
//! is an affine parameter with user-chosen tangent normalization, so g(u,u)=0.
//!
//! This first implementation is intentionally fixed-step and bounded. It
//! reports normalization and Killing-constant drift; it does not silently
//! project trajectories back onto a constraint surface or cross a singular
//! coordinate chart. Use ingoing charts for future-horizon crossing.

use core::fmt;

use super::{
    autodiff::{DifferentiatedMetric4, JetError},
    connection::{ConnectionError, LeviCivitaConnection4},
    KerrBlackHole, SchwarzschildBlackHole,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeodesicKind {
    TimelikeProperLength,
    NullAffine,
}

impl GeodesicKind {
    fn normalization_target(self) -> f64 {
        match self {
            Self::TimelikeProperLength => -1.0,
            Self::NullAffine => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchwarzschildChart {
    Exterior,
    IngoingEddingtonFinkelstein,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerrChart {
    BoyerLindquistExterior,
    Ingoing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeodesicBackground {
    Schwarzschild {
        black_hole: SchwarzschildBlackHole,
        chart: SchwarzschildChart,
    },
    Kerr {
        black_hole: KerrBlackHole,
        chart: KerrChart,
    },
}

impl GeodesicBackground {
    fn differentiated_metric(
        self,
        coordinates: [f64; 4],
    ) -> Result<DifferentiatedMetric4, GeodesicError> {
        let radius = coordinates[1];
        let theta = coordinates[2];
        match self {
            Self::Schwarzschild { black_hole, chart: SchwarzschildChart::Exterior } =>
                Ok(DifferentiatedMetric4::schwarzschild_exterior(
                    black_hole, radius, theta,
                )?),
            Self::Schwarzschild {
                black_hole,
                chart: SchwarzschildChart::IngoingEddingtonFinkelstein,
            } => Ok(DifferentiatedMetric4::schwarzschild_ingoing_ef(
                black_hole, radius, theta,
            )?),
            Self::Kerr {
                black_hole,
                chart: KerrChart::BoyerLindquistExterior,
            } => Ok(DifferentiatedMetric4::kerr_exterior(
                black_hole, radius, theta,
            )?),
            Self::Kerr { black_hole, chart: KerrChart::Ingoing } =>
                Ok(DifferentiatedMetric4::kerr_ingoing(
                    black_hole, radius, theta,
                )?),
        }
    }
}

/// A point and tangent on a worldline. Coordinates and affine parameter use
/// length units for the time coordinate; angles remain dimensionless radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodesicState {
    coordinates: [f64; 4],
    tangent: [f64; 4],
    affine_parameter_m: f64,
}

impl GeodesicState {
    pub fn new(
        coordinates: [f64; 4],
        tangent: [f64; 4],
        affine_parameter_m: f64,
    ) -> Result<Self, GeodesicError> {
        if !affine_parameter_m.is_finite()
            || coordinates.iter().any(|x| !x.is_finite())
            || tangent.iter().any(|x| !x.is_finite())
        {
            return Err(GeodesicError::NonFiniteState);
        }
        Ok(Self {
            coordinates,
            tangent,
            affine_parameter_m,
        })
    }

    pub const fn coordinates(self) -> [f64; 4] {
        self.coordinates
    }

    pub const fn tangent(self) -> [f64; 4] {
        self.tangent
    }

    pub const fn affine_parameter_m(self) -> f64 {
        self.affine_parameter_m
    }
}

/// Conserved quantities and normalization at one state.
///
/// Energy is per unit rest mass for timelike trajectories, and inherits the
/// arbitrary affine normalization for null trajectories. Angular momentum
/// is per unit rest mass (or the corresponding null affine scale) in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodesicDiagnostics {
    pub normalization: f64,
    pub normalization_residual: f64,
    pub killing_energy: f64,
    pub axial_angular_momentum_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodesicIntegrator {
    background: GeodesicBackground,
    kind: GeodesicKind,
    step_size_m: f64,
}

impl GeodesicIntegrator {
    /// Create a fixed-positive-step RK4 integrator.
    ///
    /// A negative integration direction can be represented by reversing the
    /// initial tangent, rather than supplying a negative step size.
    pub fn new(
        background: GeodesicBackground,
        kind: GeodesicKind,
        step_size_m: f64,
    ) -> Result<Self, GeodesicError> {
        if !step_size_m.is_finite() || step_size_m <= 0.0 {
            return Err(GeodesicError::InvalidStepSize(step_size_m));
        }
        Ok(Self {
            background,
            kind,
            step_size_m,
        })
    }

    pub const fn step_size_m(self) -> f64 {
        self.step_size_m
    }

    pub const fn kind(self) -> GeodesicKind {
        self.kind
    }

    /// Evaluate g(u,u), Killing energy, and axial angular momentum.
    ///
    /// Stationarity and axisymmetry make p_0 and p_phi conserved for all
    /// supported Schwarzschild/Kerr charts, including the ingoing charts.
    pub fn diagnostics(
        &self,
        state: GeodesicState,
    ) -> Result<GeodesicDiagnostics, GeodesicError> {
        let differentiated = self.background.differentiated_metric(state.coordinates)?;
        let g = differentiated.metric.covariant();
        let mut normalization = 0.0;
        let mut covector = [0.0; 4];
        for mu in 0..4 {
            for nu in 0..4 {
                normalization += g[mu][nu] * state.tangent[mu] * state.tangent[nu];
                covector[mu] += g[mu][nu] * state.tangent[nu];
            }
        }
        if !normalization.is_finite() || covector.iter().any(|x| !x.is_finite()) {
            return Err(GeodesicError::NonFiniteDiagnostics);
        }
        Ok(GeodesicDiagnostics {
            normalization,
            normalization_residual: normalization - self.kind.normalization_target(),
            killing_energy: -covector[0],
            axial_angular_momentum_m: covector[3],
        })
    }

    /// Advance exactly one RK4 step. A chart-domain error aborts the step;
    /// no coordinate singularity is interpreted as a physical singularity.
    pub fn advance(
        &self,
        state: GeodesicState,
    ) -> Result<GeodesicState, GeodesicError> {
        let y0 = pack_state(state);
        let h = self.step_size_m;
        let k1 = self.rhs(y0)?;
        let k2 = self.rhs(add_scaled(y0, k1, 0.5 * h))?;
        let k3 = self.rhs(add_scaled(y0, k2, 0.5 * h))?;
        let k4 = self.rhs(add_scaled(y0, k3, h))?;

        let mut next = y0;
        for i in 0..8 {
            next[i] += h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        if next.iter().any(|x| !x.is_finite()) {
            return Err(GeodesicError::NonFiniteEvolution);
        }

        GeodesicState::new(
            [next[0], next[1], next[2], next[3]],
            [next[4], next[5], next[6], next[7]],
            state.affine_parameter_m + h,
        )
    }

    /// Integrate a fixed number of steps and retain every state, including
    /// the initial state. The caller chooses the step count and records it
    /// with the fixture manifest.
    pub fn integrate(
        &self,
        initial: GeodesicState,
        steps: usize,
    ) -> Result<Vec<GeodesicState>, GeodesicError> {
        let mut states = Vec::with_capacity(steps.saturating_add(1));
        let mut current = initial;
        states.push(current);
        for _ in 0..steps {
            current = self.advance(current)?;
            states.push(current);
        }
        Ok(states)
    }

    fn rhs(&self, packed: [f64; 8]) -> Result<[f64; 8], GeodesicError> {
        let coordinates = [packed[0], packed[1], packed[2], packed[3]];
        let tangent = [packed[4], packed[5], packed[6], packed[7]];
        if packed.iter().any(|x| !x.is_finite()) {
            return Err(GeodesicError::NonFiniteState);
        }

        let differentiated = self.background.differentiated_metric(coordinates)?;
        let connection = LeviCivitaConnection4::from_metric_and_derivatives(
            &differentiated.metric,
            &differentiated.first,
        )?;
        let gamma = connection.symbols();

        let mut derivative = [0.0; 8];
        for mu in 0..4 {
            derivative[mu] = tangent[mu];
        }
        for rho in 0..4 {
            let mut acceleration = 0.0;
            for mu in 0..4 {
                for nu in 0..4 {
                    acceleration -= gamma[rho][mu][nu] * tangent[mu] * tangent[nu];
                }
            }
            if !acceleration.is_finite() {
                return Err(GeodesicError::NonFiniteEvolution);
            }
            derivative[rho + 4] = acceleration;
        }
        Ok(derivative)
    }
}

fn pack_state(state: GeodesicState) -> [f64; 8] {
    [
        state.coordinates[0], state.coordinates[1], state.coordinates[2], state.coordinates[3],
        state.tangent[0], state.tangent[1], state.tangent[2], state.tangent[3],
    ]
}

fn add_scaled(mut base: [f64; 8], delta: [f64; 8], scale: f64) -> [f64; 8] {
    for i in 0..8 {
        base[i] += scale * delta[i];
    }
    base
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeodesicError {
    Jet(JetError),
    Connection(ConnectionError),
    InvalidStepSize(f64),
    NonFiniteState,
    NonFiniteEvolution,
    NonFiniteDiagnostics,
}

impl From<JetError> for GeodesicError {
    fn from(value: JetError) -> Self { Self::Jet(value) }
}
impl From<ConnectionError> for GeodesicError {
    fn from(value: ConnectionError) -> Self { Self::Connection(value) }
}

impl fmt::Display for GeodesicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jet(error) => write!(f, "{error}"),
            Self::Connection(error) => write!(f, "{error}"),
            Self::InvalidStepSize(value) =>
                write!(f, "geodesic step size must be finite and positive, got {value} m"),
            Self::NonFiniteState => write!(f, "geodesic state contains a non-finite value"),
            Self::NonFiniteEvolution => write!(f, "geodesic integration produced a non-finite value"),
            Self::NonFiniteDiagnostics => write!(f, "geodesic diagnostics are non-finite"),
        }
    }
}
impl std::error::Error for GeodesicError {}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{DimensionlessSpin, MassKg};

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn schwarzschild() -> SchwarzschildBlackHole {
        SchwarzschildBlackHole::new(MassKg::new(SOLAR_MASS_KG).unwrap())
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        let allowed = if expected == 0.0 { tolerance } else { tolerance * expected.abs() };
        assert!(
            (actual - expected).abs() <= allowed,
            "actual={actual:.16e}, expected={expected:.16e}, allowed={allowed:.3e}"
        );
    }

    #[test]
    fn ingoing_schwarzschild_radial_null_geodesic_crosses_horizon_with_conserved_energy() {
        let hole = schwarzschild();
        let rg = hole.gravitational_radius_m();
        let integrator = GeodesicIntegrator::new(
            GeodesicBackground::Schwarzschild {
                black_hole: hole,
                chart: SchwarzschildChart::IngoingEddingtonFinkelstein,
            },
            GeodesicKind::NullAffine,
            0.05 * rg,
        ).unwrap();

        // In ingoing EF coordinates, v=const and decreasing r is an exact
        // radial null geodesic. Its affine tangent is (0,-1,0,0).
        let initial = GeodesicState::new(
            [0.0, 2.1 * rg, core::f64::consts::FRAC_PI_2, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            0.0,
        ).unwrap();
        let initial_diagnostics = integrator.diagnostics(initial).unwrap();
        close(initial_diagnostics.normalization, 0.0, 1e-13);
        close(initial_diagnostics.killing_energy, 1.0, 1e-13);
        close(initial_diagnostics.axial_angular_momentum_m, 0.0, 1e-13);

        let states = integrator.integrate(initial, 4).unwrap();
        let final_state = *states.last().unwrap();
        let final_diagnostics = integrator.diagnostics(final_state).unwrap();
        close(final_state.coordinates()[0], 0.0, 1e-13);
        close(final_state.coordinates()[1], 1.9 * rg, 2e-12);
        close(final_diagnostics.normalization, 0.0, 1e-12);
        close(final_diagnostics.killing_energy, initial_diagnostics.killing_energy, 1e-12);
        close(
            final_diagnostics.axial_angular_momentum_m,
            initial_diagnostics.axial_angular_momentum_m,
            1e-12,
        );
        assert_eq!(states.len(), 5);
    }

    #[test]
    fn schwarzschild_timelike_circular_orbit_preserves_normalization_and_killing_constants() {
        let hole = schwarzschild();
        let rg = hole.gravitational_radius_m();
        let radius = 10.0 * rg;
        let ut = 1.0 / (1.0 - 3.0 * rg / radius).sqrt();
        let uphi = (rg / radius.powi(3)).sqrt() * ut;
        let initial = GeodesicState::new(
            [0.0, radius, core::f64::consts::FRAC_PI_2, 0.0],
            [ut, 0.0, 0.0, uphi],
            0.0,
        ).unwrap();
        let integrator = GeodesicIntegrator::new(
            GeodesicBackground::Schwarzschild {
                black_hole: hole,
                chart: SchwarzschildChart::Exterior,
            },
            GeodesicKind::TimelikeProperLength,
            0.01 * rg,
        ).unwrap();

        let before = integrator.diagnostics(initial).unwrap();
        close(before.normalization, -1.0, 2e-12);
        let states = integrator.integrate(initial, 20).unwrap();
        let after = integrator.diagnostics(*states.last().unwrap()).unwrap();
        assert!(after.normalization_residual.abs() < 1e-9);
        assert!((after.killing_energy - before.killing_energy).abs() < 1e-9);
        assert!(
            (after.axial_angular_momentum_m - before.axial_angular_momentum_m).abs()
                < 1e-9
        );
        close(states.last().unwrap().coordinates()[1], radius, 1e-10);
    }

    #[test]
    fn zero_spin_kerr_geodesic_matches_schwarzschild_in_same_boyer_lindquist_chart() {
        let mass = MassKg::new(SOLAR_MASS_KG).unwrap();
        let spin = DimensionlessSpin::new(0.0).unwrap();
        let schwarzschild = SchwarzschildBlackHole::new(mass);
        let kerr = KerrBlackHole::new(mass, spin);
        let rg = schwarzschild.gravitational_radius_m();
        let radius = 9.0 * rg;
        let ut = 1.0 / (1.0 - 3.0 * rg / radius).sqrt();
        let uphi = (rg / radius.powi(3)).sqrt() * ut;
        let state = GeodesicState::new(
            [0.0, radius, core::f64::consts::FRAC_PI_2, 0.0],
            [ut, 0.0, 0.0, uphi],
            0.0,
        ).unwrap();
        let schwarzschild_integrator = GeodesicIntegrator::new(
            GeodesicBackground::Schwarzschild {
                black_hole: schwarzschild,
                chart: SchwarzschildChart::Exterior,
            },
            GeodesicKind::TimelikeProperLength,
            0.005 * rg,
        ).unwrap();
        let kerr_integrator = GeodesicIntegrator::new(
            GeodesicBackground::Kerr {
                black_hole: kerr,
                chart: KerrChart::BoyerLindquistExterior,
            },
            GeodesicKind::TimelikeProperLength,
            0.005 * rg,
        ).unwrap();
        let s = schwarzschild_integrator.integrate(state, 10).unwrap();
        let k = kerr_integrator.integrate(state, 10).unwrap();
        for (left, right) in s.iter().zip(k.iter()) {
            for i in 0..4 {
                close(left.coordinates()[i], right.coordinates()[i], 2e-12);
                close(left.tangent()[i], right.tangent()[i], 2e-12);
            }
        }
    }

    #[test]
    fn invalid_step_and_nonfinite_state_are_rejected() {
        let hole = schwarzschild();
        assert!(matches!(
            GeodesicIntegrator::new(
                GeodesicBackground::Schwarzschild {
                    black_hole: hole,
                    chart: SchwarzschildChart::Exterior,
                },
                GeodesicKind::TimelikeProperLength,
                0.0,
            ),
            Err(GeodesicError::InvalidStepSize(_))
        ));
        assert!(matches!(
            GeodesicState::new([0.0, f64::NAN, 1.0, 0.0], [1.0, 0.0, 0.0, 0.0], 0.0),
            Err(GeodesicError::NonFiniteState)
        ));
    }

    #[test]
    fn boyer_lindquist_chart_fails_closed_at_horizon_boundary() {
        let mass = MassKg::new(SOLAR_MASS_KG).unwrap();
        let hole = KerrBlackHole::new(mass, DimensionlessSpin::new(0.8).unwrap());
        let state = GeodesicState::new(
            [0.0, hole.outer_horizon_radius_m(), 1.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            0.0,
        ).unwrap();
        let integrator = GeodesicIntegrator::new(
            GeodesicBackground::Kerr {
                black_hole: hole,
                chart: KerrChart::BoyerLindquistExterior,
            },
            GeodesicKind::TimelikeProperLength,
            0.001 * hole.gravitational_radius_m(),
        ).unwrap();
        assert!(matches!(
            integrator.diagnostics(state),
            Err(GeodesicError::Jet(JetError::Metric(_)))
        ));
    }
}
